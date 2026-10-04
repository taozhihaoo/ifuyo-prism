//! M10 GUI：context 生成 IPC（§292–§303/§354–§363；§475 GUI/CLI 同源）。
//!
//! 扫描异步执行复用 session task 语义（§302/§303）；导出返回内容字符串
//! 由前端 Copy/下载（§302/§311：不做自动 AI 上传，§308–§310）。

use crate::state::{lock_session, AppState};
use prism_analysis::{CodeAnalysisOptions, CodeAnalysisService};
use prism_changes::{ChangeScope, ChangeSetOptions, ChangeSetService};
use prism_context::model::{ContextBudget, ContextRequest, ContextScope};
use prism_context::ContextService;
use prism_core::config::PrismConfig;
use prism_core::error::PrismErrorDto;
use prism_core::runtime::command::StdCommandRunner;
use prism_core::task::TaskState;
use prism_graph::DependencyService;
use serde::Serialize;
use std::sync::Arc;
use tauri::State;

fn context_root(state: &State<'_, AppState>) -> Result<std::path::PathBuf, PrismErrorDto> {
    let session = lock_session(&state.session);
    session
        .project
        .as_ref()
        .map(|project| project.root_path.clone())
        .ok_or_else(|| {
            PrismErrorDto::new("invalid_input", "no project opened; call open_project first")
        })
}

#[derive(Debug, Clone, Serialize)]
pub struct ContextStatusDto {
    /// "idle" | "running" | "completed" | "failed" | "cancelled"
    pub status: String,
    pub error: Option<String>,
}

/// 启动 context 生成（§274 Refresh 语义：全新生成，非 cache 回放）。
#[tauri::command]
pub fn start_context_generation(
    state: State<'_, AppState>,
    task: Option<String>,
    scope: Option<String>,
    max_tokens: Option<u64>,
) -> Result<String, PrismErrorDto> {
    let root = context_root(&state)?;
    let config_path = root.join(PrismConfig::config_file_name());
    let (config, _loaded) = PrismConfig::load(&config_path)
        .map_err(|err| PrismErrorDto::new("config", format!("config invalid: {err}")))?;
    let _ = config;
    let parsed_scope = match scope.as_deref() {
        None | Some("auto") => ContextScope::Auto,
        Some("current_change") => ContextScope::CurrentChange,
        Some("target") => ContextScope::Target,
        Some("full") => ContextScope::Full,
        Some(other) => {
            return Err(PrismErrorDto::new(
                "invalid_input",
                format!("unknown scope '{other}'"),
            ))
        }
    };
    let mut budget = ContextBudget::default();
    if let Some(max_tokens) = max_tokens {
        budget.max_tokens_estimate = max_tokens;
    }
    let request = ContextRequest {
        project_root: root.to_string_lossy().to_string(),
        task: task.clone(),
        scope: parsed_scope,
        target: None,
        budget,
    };
    let (root, cancel, session_arc) = {
        let mut session = lock_session(&state.session);
        if session.context_status == Some(TaskState::Running) {
            return Err(PrismErrorDto::new(
                "invalid_input",
                "a context generation is already running",
            ));
        }
        session.context_token = prism_core::task::CancellationToken::new();
        session.context_status = Some(TaskState::Running);
        session.context_pack = None;
        session.context_error = None;
        let token = session.context_token.clone();
        (root, token, state.session.clone())
    };

    std::thread::spawn(move || {
        let execute = || -> prism_core::PrismResult<prism_context::ContextPack> {
            let discovery = prism_project::DiscoveryService::new()
                .discover(&root, prism_project::ScanOptions::default(), &cancel, |_| {})?;
            let code_model = CodeAnalysisService::new().analyze(
                &discovery,
                CodeAnalysisOptions::default(),
                &cancel,
                |_| {},
            )?;
            let graph = DependencyService::new()
                .analyze(&discovery, &code_model)
                .ok();
            let runner: Arc<dyn prism_core::CommandRunner> = Arc::new(StdCommandRunner::new());
            let changeset = ChangeSetService::new(runner)
                .build(
                    &root,
                    ChangeScope::WorkingTree,
                    None,
                    None,
                    &ChangeSetOptions::default(),
                    &cancel,
                )
                .ok()
                .flatten();
            let impact = changeset
                .as_ref()
                .zip(graph.as_ref())
                .and_then(|(changeset, graph)| {
                    prism_impact::ImpactAnalysisService
                        .analyze(
                            changeset,
                            &code_model,
                            graph,
                            &prism_impact::ImpactOptions::default(),
                            &cancel,
                        )
                        .ok()
                });
            // M7–M9 session 事实可在此传入；v1 GUI 不持有 => None（§260 降级）。
            ContextService::generate(
                request,
                Some(&discovery),
                Some(&code_model),
                changeset.as_ref(),
                impact.as_ref(),
                graph.as_ref(),
                None,
                None,
                None,
                &cancel,
            )
        };
        let outcome = execute();

        let mut session = lock_session(&session_arc);
        match outcome {
            Ok(pack) => {
                session.context_status = Some(TaskState::Completed);
                session.context_pack = Some(pack);
            }
            Err(prism_core::PrismError::Cancelled) => {
                session.context_status = Some(TaskState::Cancelled);
            }
            Err(err) => {
                session.context_status = Some(TaskState::Failed);
                session.context_error = Some(err.to_string());
            }
        }
    });

    Ok("started".to_string())
}

/// 查询生成状态。
#[tauri::command]
pub fn get_context_status(state: State<'_, AppState>) -> ContextStatusDto {
    let session = lock_session(&state.session);
    let status = match session.context_status {
        None => "idle".to_string(),
        Some(status) => serde_json::to_value(status)
            .ok()
            .and_then(|value| value.as_str().map(str::to_string))
            .unwrap_or_else(|| "unknown".to_string()),
    };
    ContextStatusDto {
        status,
        error: session.context_error.clone(),
    }
}

/// 最近一次生成的 pack。
#[tauri::command]
pub fn get_context_results(
    state: State<'_, AppState>,
) -> Option<prism_context::ContextPack> {
    let session = lock_session(&state.session);
    session.context_pack.clone()
}

/// 取消生成（§303）。
#[tauri::command]
pub fn cancel_context_generation(state: State<'_, AppState>) -> Result<(), PrismErrorDto> {
    let session = lock_session(&state.session);
    session.context_token.cancel();
    Ok(())
}

/// 导出（§302/§311：用户显式操作；返回内容字符串，前端 Copy/下载）。
#[tauri::command]
pub fn export_context(
    state: State<'_, AppState>,
    format: String,
) -> Result<String, PrismErrorDto> {
    let session = lock_session(&state.session);
    let pack = session.context_pack.as_ref().ok_or_else(|| {
        PrismErrorDto::new("invalid_input", "no context pack generated yet")
    })?;
    match format.as_str() {
        "json" => serde_json::to_string_pretty(pack)
            .map_err(|err| PrismErrorDto::new("internal", format!("serialize failed: {err}"))),
        "markdown" => Ok(prism_context::render_markdown(pack)),
        "text" => Ok(prism_context::render_plain_text(pack)),
        other => Err(PrismErrorDto::new(
            "invalid_input",
            format!("unknown export format '{other}' (json|markdown|text)"),
        )),
    }
}
