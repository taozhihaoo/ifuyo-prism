//! Tauri IPC commands。
//!
//! M0 基础集：get_app_info / get_prism_version / get_runtime_info。
//! M1 新增：open_project / start_scan / cancel_scan / get_scan_status /
//! get_discovery。全部转发到 prism-project 的 DiscoveryService / 共享会话，
//! GUI 不自建扫描逻辑。扫描在后台线程执行，前端轮询 get_scan_status
//! （纲领 §68 允许轮询而非 streaming）。

use crate::state::{lock_session, status_string, AppState, InspectProgress};
use prism_analysis::{CodeAnalysisOptions, CodeAnalysisService, ProjectCodeModel};
use prism_core::application::{AppInfo, RuntimeInfo};
use prism_core::error::PrismErrorDto;
use prism_core::runtime::command::StdCommandRunner;
use prism_core::task::{CancellationToken, TaskState};
use prism_git::model::{GitCommit, GitRef, GitRepository, GitStatus};
use prism_git::GitService;
use prism_graph::DependencyGraphResult;
use prism_project::{DiscoveryService, Project, ProjectDiscovery, ScanOptions, ScanProgress};
use serde::Serialize;
use std::sync::Arc;
use tauri::State;

/// 应用信息（名称、版本、平台、构建 profile）。
#[tauri::command]
pub fn get_app_info(state: State<'_, AppState>) -> AppInfo {
    state.app_info.app_info()
}

/// Prism 版本号。
#[tauri::command]
pub fn get_prism_version(_state: State<'_, AppState>) -> &'static str {
    prism_core::application::app_info::prism_version()
}

/// 运行时信息（平台与构建事实）。
#[tauri::command]
pub fn get_runtime_info(state: State<'_, AppState>) -> RuntimeInfo {
    state.app_info.runtime_info()
}

// ---------- M1：Project Discovery ----------

/// 扫描会话对外状态。
#[derive(Debug, Clone, Serialize)]
pub struct ScanStatusDto {
    pub project: Option<Project>,
    /// "idle" | "queued" | "running" | "completed" | "failed" | "cancelled"
    pub status: String,
    pub directories: u64,
    pub files: u64,
    pub ignored: u64,
    pub errors: u64,
    /// Completed + 非致命错误 > 0 → completed with warnings。
    pub has_warnings: bool,
    /// Failed 时的错误描述。
    pub error: Option<String>,
}

/// 打开项目：验证目录有效性并建立 Project（重置会话）。
#[tauri::command]
pub fn open_project(path: String, state: State<'_, AppState>) -> Result<Project, PrismErrorDto> {
    let project = Project::from_root(&path).map_err(PrismErrorDto::from)?;
    let mut session = lock_session(&state.session);
    session.open(project.clone());
    Ok(project)
}

/// 启动扫描（后台线程执行），返回 task id；进度经 get_scan_status 轮询。
#[tauri::command]
pub fn start_scan(state: State<'_, AppState>) -> Result<String, PrismErrorDto> {
    let (root, token, task_id, session_arc) = {
        let mut session = lock_session(&state.session);
        if session.project.is_none() {
            return Err(PrismErrorDto::new(
                "invalid_input",
                "no project opened; call open_project first",
            ));
        }
        if session.status == Some(TaskState::Running) {
            return Err(PrismErrorDto::new(
                "invalid_input",
                "a scan is already running",
            ));
        }
        let root = session
            .project
            .as_ref()
            .expect("checked above")
            .root_path
            .clone();
        let task_id = session.begin_scan();
        let token = session.token.clone();
        (root, token, task_id, state.session.clone())
    };

    std::thread::spawn(move || {
        let service = DiscoveryService::new();
        let result = service.discover(
            &root,
            ScanOptions::default(),
            &token,
            |progress: ScanProgress| {
                // 实时计数写回共享会话；锁竞争极低（写 8 字节），可接受。
                let mut session = lock_session(&session_arc);
                session.progress = progress;
            },
        );
        let mut session = lock_session(&session_arc);
        match result {
            Ok(discovery) => {
                session.status = Some(discovery.scan.status);
                // 完成时以最终统计为准（与扫描中的实时计数一致）。
                session.progress = ScanProgress {
                    directories: discovery.statistics.total_directories,
                    files: discovery.statistics.total_files,
                    ignored: discovery.statistics.ignored_files,
                    errors: discovery.statistics.error_count,
                };
                session.discovery = Some(discovery);
            }
            Err(err) => {
                session.status = Some(TaskState::Failed);
                session.error = Some(err.to_string());
            }
        }
    });

    Ok(task_id.to_string())
}

/// 请求取消当前扫描。
#[tauri::command]
pub fn cancel_scan(state: State<'_, AppState>) -> Result<(), PrismErrorDto> {
    let session = lock_session(&state.session);
    session.token.cancel();
    Ok(())
}

/// 查询扫描会话状态（前端轮询入口）。
#[tauri::command]
pub fn get_scan_status(state: State<'_, AppState>) -> ScanStatusDto {
    let session = lock_session(&state.session);
    let status = status_string(session.status);
    let has_warnings = session.status == Some(TaskState::Completed)
        && session
            .discovery
            .as_ref()
            .is_some_and(|discovery| discovery.scan.errors_count > 0);
    ScanStatusDto {
        project: session.project.clone(),
        status,
        directories: session.progress.directories,
        files: session.progress.files,
        ignored: session.progress.ignored,
        errors: session.progress.errors,
        has_warnings,
        error: session.error.clone(),
    }
}

/// 获取最近一次扫描的完整 Discovery 结果（未完成/未扫描为 None）。
#[tauri::command]
pub fn get_discovery(state: State<'_, AppState>) -> Option<ProjectDiscovery> {
    let session = lock_session(&state.session);
    session.discovery.clone()
}

// ---------- M2：Code Understanding ----------

/// inspect 会话对外状态。
#[derive(Debug, Clone, Serialize)]
pub struct InspectStatusDto {
    /// "idle" | "running" | "completed" | "failed" | "cancelled"
    pub status: String,
    pub files_total: u64,
    pub files_done: u64,
    pub symbols: u64,
    pub errors: u64,
    /// Completed 且存在解析失败/错误诊断 → completed with warnings。
    pub has_warnings: bool,
    pub error: Option<String>,
}

/// 启动代码理解（discovery + analysis 后台线程），返回 task id。
/// 前端经 get_inspect_status 轮询、get_code_model 取结果。
#[tauri::command]
pub fn start_inspect(state: State<'_, AppState>) -> Result<String, PrismErrorDto> {
    let (root, token, task_id, session_arc) = {
        let mut session = lock_session(&state.session);
        if session.project.is_none() {
            return Err(PrismErrorDto::new(
                "invalid_input",
                "no project opened; call open_project first",
            ));
        }
        if session.inspect_status == Some(TaskState::Running) {
            return Err(PrismErrorDto::new(
                "invalid_input",
                "an inspect is already running",
            ));
        }
        let root = session
            .project
            .as_ref()
            .expect("checked above")
            .root_path
            .clone();
        let task_id = session.begin_inspect();
        let token = session.inspect_token.clone();
        (root, token, task_id, state.session.clone())
    };

    let progress_session = session_arc.clone();
    std::thread::spawn(move || {
        let cancel = &token;
        let result =
            DiscoveryService::new().discover(&root, ScanOptions::default(), cancel, |_| {});
        let outcome = result.and_then(move |discovery| {
            CodeAnalysisService::new().analyze(
                &discovery,
                CodeAnalysisOptions::default(),
                cancel,
                move |progress: prism_analysis::CodeAnalysisProgress| {
                    let mut session = lock_session(&progress_session);
                    session.inspect_progress = InspectProgress {
                        files_total: progress.files_total,
                        files_done: progress.files_done,
                        symbols: progress.symbols,
                        errors: progress.errors,
                    };
                },
            )
        });

        let mut session = lock_session(&session_arc);
        match outcome {
            Ok(model) => {
                session.inspect_status = Some(model.summary.status);
                session.inspect_progress = InspectProgress {
                    files_total: model.summary.files_total,
                    files_done: model.summary.files_total,
                    symbols: model.summary.symbols_total,
                    errors: model.summary.error_count,
                };
                session.code_model = Some(model);
            }
            Err(err) => {
                session.inspect_status = Some(TaskState::Failed);
                session.inspect_error = Some(err.to_string());
            }
        }
    });

    Ok(task_id.to_string())
}

/// 查询 inspect 会话状态（前端轮询入口）。
#[tauri::command]
pub fn get_inspect_status(state: State<'_, AppState>) -> InspectStatusDto {
    let session = lock_session(&state.session);
    let status = match session.inspect_status {
        None => "idle".to_string(),
        Some(state) => serde_json::to_value(state)
            .ok()
            .and_then(|v| v.as_str().map(str::to_string))
            .unwrap_or_else(|| "unknown".to_string()),
    };
    let has_warnings = session.inspect_status == Some(TaskState::Completed)
        && session
            .code_model
            .as_ref()
            .is_some_and(|model| model.summary.files_failed > 0 || model.summary.error_count > 0);
    InspectStatusDto {
        status,
        files_total: session.inspect_progress.files_total,
        files_done: session.inspect_progress.files_done,
        symbols: session.inspect_progress.symbols,
        errors: session.inspect_progress.errors,
        has_warnings,
        error: session.inspect_error.clone(),
    }
}

/// 获取最近一次 inspect 的完整 Code Model（未完成为 None）。
#[tauri::command]
pub fn get_code_model(state: State<'_, AppState>) -> Option<ProjectCodeModel> {
    let session = lock_session(&state.session);
    session.code_model.clone()
}

/// 取消当前 inspect。
#[tauri::command]
pub fn cancel_inspect(state: State<'_, AppState>) -> Result<(), PrismErrorDto> {
    let session = lock_session(&state.session);
    session.inspect_token.cancel();
    Ok(())
}

// ---------- M3：Dependency & Architecture ----------

/// graph 会话对外状态。
#[derive(Debug, Clone, Serialize)]
pub struct GraphStatusDto {
    /// "idle" | "running" | "completed" | "failed" | "cancelled"
    pub status: String,
    pub node_count: u64,
    pub edge_count: u64,
    pub cycle_count: u64,
    pub unresolved_count: u64,
    pub error: Option<String>,
}

/// 启动依赖图分析（后台线程；内部复用 discovery + code model）。
#[tauri::command]
pub fn start_graph(state: State<'_, AppState>) -> Result<String, PrismErrorDto> {
    let (token, task_id, session_arc) = {
        let mut session = lock_session(&state.session);
        if session.project.is_none() {
            return Err(PrismErrorDto::new(
                "invalid_input",
                "no project opened; call open_project first",
            ));
        }
        if session.graph_status == Some(TaskState::Running) {
            return Err(PrismErrorDto::new(
                "invalid_input",
                "a graph analysis is already running",
            ));
        }
        if session.discovery.is_none() || session.code_model.is_none() {
            return Err(PrismErrorDto::new(
                "invalid_input",
                "run a scan first; graph analysis builds on discovery and code model",
            ));
        }
        let task_id = session.begin_graph();
        let token = session.graph_token.clone();
        (token, task_id, state.session.clone())
    };

    std::thread::spawn(move || {
        let discovery = {
            let session = lock_session(&session_arc);
            session.discovery.clone()
        };
        let model = {
            let session = lock_session(&session_arc);
            session.code_model.clone()
        };
        let outcome = match (discovery, model) {
            (Some(discovery), Some(model)) => {
                prism_graph::DependencyService::new().analyze(&discovery, &model)
            }
            _ => Err(prism_core::PrismError::Internal {
                message: "session lost discovery or code model".to_string(),
            }),
        };
        let mut session = lock_session(&session_arc);
        match outcome {
            Ok(graph) => {
                session.graph_status = Some(TaskState::Completed);
                session.graph_model = Some(graph);
            }
            Err(err) => {
                session.graph_status = Some(TaskState::Failed);
                session.graph_error = Some(err.to_string());
            }
        }
        let _ = token;
    });

    Ok(task_id.to_string())
}

/// 查询 graph 会话状态。
#[tauri::command]
pub fn get_graph_status(state: State<'_, AppState>) -> GraphStatusDto {
    let session = lock_session(&state.session);
    let status = match session.graph_status {
        None => "idle".to_string(),
        Some(state) => serde_json::to_value(state)
            .ok()
            .and_then(|v| v.as_str().map(str::to_string))
            .unwrap_or_else(|| "unknown".to_string()),
    };
    let (node_count, edge_count, cycle_count, unresolved_count) = session
        .graph_model
        .as_ref()
        .map(|graph| {
            (
                graph.metrics.node_count,
                graph.metrics.edge_count,
                graph.metrics.cycle_count,
                graph.metrics.unresolved_edges,
            )
        })
        .unwrap_or((0, 0, 0, 0));
    GraphStatusDto {
        status,
        node_count,
        edge_count,
        cycle_count,
        unresolved_count,
        error: session.graph_error.clone(),
    }
}

/// 获取最近一次的依赖图结果。
#[tauri::command]
pub fn get_dependency_graph(state: State<'_, AppState>) -> Option<DependencyGraphResult> {
    let session = lock_session(&state.session);
    session.graph_model.clone()
}

// ---------- M4：Git facts ----------

/// Git facts 快照（GUI Changes 页一次性消费；只读，规格 §32/§33）。
#[derive(Debug, Clone, Serialize)]
pub struct GitFactsDto {
    /// 非 Git 项目为 None（规格 §25）。
    pub repository: Option<GitRepository>,
    pub head: Option<prism_git::GitHead>,
    pub status: GitStatus,
    pub recent_commits: Vec<GitCommit>,
    pub refs: Vec<GitRef>,
}

#[tauri::command]
pub fn get_git_facts(
    state: State<'_, AppState>,
    include_ignored: bool,
) -> Result<GitFactsDto, PrismErrorDto> {
    let root = {
        let session = lock_session(&state.session);
        session
            .project
            .as_ref()
            .map(|project| project.root_path.clone())
            .ok_or_else(|| {
                PrismErrorDto::new(
                    "invalid_input",
                    "no project opened; call open_project first",
                )
            })?
    };
    let cancel = CancellationToken::new();
    let service = GitService::new(Arc::new(StdCommandRunner::new()));
    let status = service
        .status(&root, include_ignored, &cancel)
        .map_err(PrismErrorDto::from)?;
    let (recent_commits, refs) = if status.repository.is_some() {
        (
            service.log(&root, 20, &cancel).unwrap_or_default(),
            service.refs(&root, &cancel).unwrap_or_default(),
        )
    } else {
        (Vec::new(), Vec::new())
    };
    Ok(GitFactsDto {
        repository: status.repository.clone(),
        head: status.head.clone(),
        status,
        recent_commits,
        refs,
    })
}
