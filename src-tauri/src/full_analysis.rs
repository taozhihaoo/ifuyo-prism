//! 全面分析编排（P2-1）：scan → inspect → graph → health 串行。
//!
//! 每阶段 = 对应单域命令的**同一管线、同一会话字段写入**——编排完成后
//! 各页面轮询自然看到结果，不建第二套结果通道。取消令牌贯穿全部阶段，
//! 中途取消保留已完成阶段（验收："中途取消显示已完成部分"）；取消/失败
//! 之后的剩余阶段标记 skipped（不伪装成"没跑也没事"）。health 阶段复用
//! 前三阶段的 discovery/code model/graph，不重复扫描；完成即写入会话缓存
//! （P2-2）。

use crate::state::{lock_session, AppState};
use prism_core::error::PrismErrorDto;
use prism_core::task::{CancellationToken, TaskState};
use serde::Serialize;
use std::time::Instant;
use tauri::State;

const STAGE_KEYS: [&str; 4] = ["scan", "inspect", "graph", "health"];

/// 单阶段状态（GUI 轮询渲染）。
#[derive(Debug, Clone, Serialize)]
pub struct FullStageState {
    pub key: String,
    /// pending | running | completed | failed | cancelled | skipped
    pub status: String,
    pub duration_ms: Option<u64>,
    pub error: Option<String>,
}

/// 编排状态 DTO（前端轮询入口）。
#[derive(Serialize)]
pub struct FullAnalysisStatusDto {
    /// "idle" | "running" | "completed" | "failed" | "cancelled"
    pub status: String,
    pub stages: Vec<FullStageState>,
    pub error: Option<String>,
}

fn stage_rows() -> Vec<FullStageState> {
    STAGE_KEYS
        .iter()
        .map(|key| FullStageState {
            key: (*key).to_string(),
            status: "pending".to_string(),
            duration_ms: None,
            error: None,
        })
        .collect()
}

fn set_stage(
    stages: &mut [FullStageState],
    index: usize,
    status: &str,
    duration_ms: Option<u64>,
    error: Option<String>,
) {
    if let Some(stage) = stages.get_mut(index) {
        stage.status = status.to_string();
        stage.duration_ms = duration_ms;
        stage.error = error;
    }
}

/// 启动全面分析（后台线程串行执行；拒绝与任何单域运行并发）。
#[tauri::command]
pub fn start_full_analysis(state: State<'_, AppState>) -> Result<String, PrismErrorDto> {
    let (root, token, session_arc, cache) = {
        let mut session = lock_session(&state.session);
        let Some(project) = session.project.as_ref() else {
            return Err(PrismErrorDto::new(
                "invalid_input",
                "no project opened; call open_project first",
            ));
        };
        if session.full_status == Some(TaskState::Running) {
            return Err(PrismErrorDto::new(
                "invalid_input",
                "a full analysis is already running",
            ));
        }
        if session.status == Some(TaskState::Running)
            || session.inspect_status == Some(TaskState::Running)
            || session.graph_status == Some(TaskState::Running)
            || session.health_status == Some(TaskState::Running)
        {
            return Err(PrismErrorDto::new(
                "invalid_input",
                "a domain analysis is already running; cancel it first",
            ));
        }
        let root = project.root_path.clone();
        // 全量体检 = 一次全新扫描；begin_scan 使各域旧结果过期（与单独
        // start_scan 同语义），但不触碰编排自身状态字段。
        session.begin_scan();
        session.full_token = CancellationToken::new();
        session.full_status = Some(TaskState::Running);
        session.full_stages = stage_rows();
        session.full_error = None;
        let token = session.full_token.clone();
        (root, token, state.session.clone(), state.cache.clone())
    };

    std::thread::spawn(move || run_pipeline(root, token, session_arc, cache));
    Ok("started".to_string())
}

fn run_pipeline(
    root: std::path::PathBuf,
    token: CancellationToken,
    session_arc: std::sync::Arc<std::sync::Mutex<crate::state::ScanSession>>,
    cache: std::sync::Arc<crate::session_cache::SessionCache>,
) {
    let finished = |session_arc: &std::sync::Arc<std::sync::Mutex<crate::state::ScanSession>>,
                    status: TaskState,
                    from: usize,
                    stage_status: &str,
                    error: Option<String>| {
        let mut session = lock_session(session_arc);
        session.full_status = Some(status);
        for index in from..STAGE_KEYS.len() {
            set_stage(&mut session.full_stages, index, stage_status, None, None);
        }
        session.full_error = error;
    };

    let mut carried = Carried::default();
    for (index, key) in STAGE_KEYS.iter().enumerate() {
        {
            let mut session = lock_session(&session_arc);
            set_stage(&mut session.full_stages, index, "running", None, None);
        }
        let started = Instant::now();
        let outcome = run_stage(key, &root, &token, &session_arc, &mut carried, &cache);
        let duration = started.elapsed().as_millis() as u64;
        match outcome {
            StageOutcome::Completed => {
                let mut session = lock_session(&session_arc);
                set_stage(
                    &mut session.full_stages,
                    index,
                    "completed",
                    Some(duration),
                    None,
                );
            }
            StageOutcome::Cancelled => {
                finished(&session_arc, TaskState::Cancelled, index, "cancelled", None);
                // 当前阶段是"进行中被取消"，剩余才是 skipped。
                let mut session = lock_session(&session_arc);
                set_stage(
                    &mut session.full_stages,
                    index,
                    "cancelled",
                    Some(duration),
                    None,
                );
                for rest in index + 1..STAGE_KEYS.len() {
                    set_stage(&mut session.full_stages, rest, "skipped", None, None);
                }
                return;
            }
            StageOutcome::Failed(err) => {
                finished(
                    &session_arc,
                    TaskState::Failed,
                    index + 1,
                    "skipped",
                    Some(format!("stage {} failed: {err}", STAGE_KEYS[index])),
                );
                let mut session = lock_session(&session_arc);
                set_stage(
                    &mut session.full_stages,
                    index,
                    "failed",
                    Some(duration),
                    Some(err.to_string()),
                );
                return;
            }
        }
    }
    let mut session = lock_session(&session_arc);
    session.full_status = Some(TaskState::Completed);
    session.full_error = None;
}

/// 阶段间携带的产物（避免二次扫描/分析）。
#[derive(Default)]
struct Carried {
    discovery: Option<prism_project::ProjectDiscovery>,
    model: Option<prism_analysis::ProjectCodeModel>,
    graph: Option<prism_graph::DependencyGraphResult>,
}

enum StageOutcome {
    Completed,
    Cancelled,
    Failed(prism_core::PrismError),
}

fn run_stage(
    stage_key: &str,
    root: &std::path::Path,
    token: &CancellationToken,
    session_arc: &std::sync::Arc<std::sync::Mutex<crate::state::ScanSession>>,
    carried: &mut Carried,
    cache: &std::sync::Arc<crate::session_cache::SessionCache>,
) -> StageOutcome {
    match stage_key {
        "scan" => {
            let progress_arc = session_arc.clone();
            let result = prism_project::DiscoveryService::new().discover(
                root,
                prism_project::ScanOptions::default(),
                token,
                |progress: prism_project::ScanProgress| {
                    let mut session = lock_session(&progress_arc);
                    session.progress = progress;
                },
            );
            match result {
                Ok(discovery) => {
                    let mut session = lock_session(session_arc);
                    session.status = Some(discovery.scan.status);
                    session.progress = prism_project::ScanProgress {
                        directories: discovery.statistics.total_directories,
                        files: discovery.statistics.total_files,
                        ignored: discovery.statistics.ignored_files,
                        errors: discovery.statistics.error_count,
                    };
                    carried.discovery = Some(discovery);
                    StageOutcome::Completed
                }
                Err(prism_core::PrismError::Cancelled) => {
                    let mut session = lock_session(session_arc);
                    session.status = Some(TaskState::Cancelled);
                    StageOutcome::Cancelled
                }
                Err(err) => {
                    let mut session = lock_session(session_arc);
                    session.status = Some(TaskState::Failed);
                    session.error = Some(err.to_string());
                    StageOutcome::Failed(err)
                }
            }
        }
        "inspect" => {
            let Some(discovery) = carried.discovery.clone() else {
                return StageOutcome::Failed(prism_core::PrismError::Internal {
                    message: "scan stage produced no discovery".to_string(),
                });
            };
            {
                let mut session = lock_session(session_arc);
                session.begin_inspect();
            }
            let progress_arc = session_arc.clone();
            let outcome = prism_analysis::CodeAnalysisService::new().analyze(
                &discovery,
                prism_analysis::CodeAnalysisOptions::default(),
                token,
                move |progress: prism_analysis::CodeAnalysisProgress| {
                    let mut session = lock_session(&progress_arc);
                    session.inspect_progress = crate::state::InspectProgress {
                        files_total: progress.files_total,
                        files_done: progress.files_done,
                        symbols: progress.symbols,
                        errors: progress.errors,
                    };
                },
            );
            match outcome {
                Ok(model) => {
                    let mut session = lock_session(session_arc);
                    session.inspect_status = Some(model.summary.status);
                    session.inspect_progress = crate::state::InspectProgress {
                        files_total: model.summary.files_total,
                        files_done: model.summary.files_total,
                        symbols: model.summary.symbols_total,
                        errors: model.summary.error_count,
                    };
                    carried.model = Some(model);
                    StageOutcome::Completed
                }
                Err(prism_core::PrismError::Cancelled) => {
                    let mut session = lock_session(session_arc);
                    session.inspect_status = Some(TaskState::Cancelled);
                    StageOutcome::Cancelled
                }
                Err(err) => {
                    let mut session = lock_session(session_arc);
                    session.inspect_status = Some(TaskState::Failed);
                    session.inspect_error = Some(err.to_string());
                    StageOutcome::Failed(err)
                }
            }
        }
        "graph" => {
            let (Some(discovery), Some(model)) = (&carried.discovery, &carried.model) else {
                return StageOutcome::Failed(prism_core::PrismError::Internal {
                    message: "earlier stage produced no discovery/code model".to_string(),
                });
            };
            {
                let mut session = lock_session(session_arc);
                session.begin_graph();
            }
            match prism_graph::DependencyService::new().analyze(discovery, model) {
                Ok(graph) => {
                    let mut session = lock_session(session_arc);
                    session.graph_status = Some(TaskState::Completed);
                    carried.graph = Some(graph);
                    StageOutcome::Completed
                }
                Err(err) => {
                    let mut session = lock_session(session_arc);
                    session.graph_status = Some(TaskState::Failed);
                    session.graph_error = Some(err.to_string());
                    if matches!(err, prism_core::PrismError::Cancelled) {
                        return StageOutcome::Cancelled;
                    }
                    StageOutcome::Failed(err)
                }
            }
        }
        "health" => {
            let (Some(discovery), Some(model), Some(graph)) =
                (&carried.discovery, &carried.model, &carried.graph)
            else {
                return StageOutcome::Failed(prism_core::PrismError::Internal {
                    message: "earlier stage produced no discovery/code model/graph".to_string(),
                });
            };
            {
                let mut session = lock_session(session_arc);
                session.begin_health();
            }
            let health_config = prism_core::config::HealthConfig::default();
            let context = prism_health::HealthContext {
                project: &discovery.project,
                code_model: Some(model),
                discovery: Some(discovery),
                graph: Some(graph),
                git: None,
                change_set: None,
                impact: None,
                root,
                derived: None,
                health_config: &health_config,
            };
            match prism_health::analyze(
                &prism_health::HealthRuleRegistry::with_builtins(),
                &prism_core::config::HealthConfig::default(),
                &context,
                &prism_health::HealthOptions::default(),
                token,
            ) {
                Ok(analysis) => {
                    let mut session = lock_session(session_arc);
                    session.health_status = Some(TaskState::Completed);
                    crate::session_cache::cache_health(cache, root, &analysis);
                    session.health_model = Some(analysis);
                    StageOutcome::Completed
                }
                Err(prism_core::PrismError::Cancelled) => {
                    let mut session = lock_session(session_arc);
                    session.health_status = Some(TaskState::Cancelled);
                    StageOutcome::Cancelled
                }
                Err(err) => {
                    let mut session = lock_session(session_arc);
                    session.health_status = Some(TaskState::Failed);
                    session.health_error = Some(err.to_string());
                    StageOutcome::Failed(err)
                }
            }
        }
        other => StageOutcome::Failed(prism_core::PrismError::Internal {
            message: format!("unknown stage: {other}"),
        }),
    }
}

#[tauri::command]
pub fn get_full_analysis_status(state: State<'_, AppState>) -> FullAnalysisStatusDto {
    let session = lock_session(&state.session);
    let status = match session.full_status {
        None => "idle".to_string(),
        Some(status) => serde_json::to_value(status)
            .ok()
            .and_then(|v| v.as_str().map(str::to_string))
            .unwrap_or_else(|| "unknown".to_string()),
    };
    FullAnalysisStatusDto {
        status,
        stages: session.full_stages.clone(),
        error: session.full_error.clone(),
    }
}

#[tauri::command]
pub fn cancel_full_analysis(state: State<'_, AppState>) -> Result<(), PrismErrorDto> {
    let session = lock_session(&state.session);
    session.full_token.cancel();
    Ok(())
}
