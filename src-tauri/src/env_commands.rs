//! M9 GUI：environment 扫描 IPC（§298–§304 GUI/CLI 同源、复用 M0 Task/取消）。
//!
//! GUI 不做任何前端检测（§299/§300）：全部事实来自 Rust EnvironmentService；
//! 扫描异步执行（§302 task 状态轮询），取消复用 CancellationToken（§303）。

use crate::state::{lock_session, AppState};
use prism_core::error::PrismErrorDto;
use prism_core::runtime::command::StdCommandRunner;
use prism_core::task::TaskState;
use prism_core::config::PrismConfig;
use std::sync::Arc;
use tauri::State;

fn env_root(state: &State<'_, AppState>) -> Result<std::path::PathBuf, PrismErrorDto> {
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
pub struct EnvironmentStatusDto {
    /// "idle" | "running" | "completed" | "failed" | "cancelled"
    pub status: String,
    pub error: Option<String>,
}

use serde::Serialize;

/// 启动环境扫描（§274/§278 Refresh：重新 discovery + probe，非 cache 回放）。
#[tauri::command]
pub fn start_environment_scan(
    state: State<'_, AppState>,
    all_tools: Option<bool>,
) -> Result<String, PrismErrorDto> {
    let root = env_root(&state)?;
    let config_path = root.join(PrismConfig::config_file_name());
    let (config, _loaded) = PrismConfig::load(&config_path).map_err(|err| {
        PrismErrorDto::new("config", format!("config invalid: {err}"))
    })?;
    let _ = config; // env 配置节留给 M9b+ 扩展；加载保证非法配置显式失败。
    let (root, token, session_arc) = {
        let mut session = lock_session(&state.session);
        if session.env_status == Some(TaskState::Running) {
            return Err(PrismErrorDto::new(
                "invalid_input",
                "an environment scan is already running",
            ));
        }
        session.env_token = prism_core::task::CancellationToken::new();
        session.env_status = Some(TaskState::Running);
        session.env_snapshot = None;
        session.env_error = None;
        let token = session.env_token.clone();
        (root, token, state.session.clone())
    };

    std::thread::spawn(move || {
        let execute = || -> prism_core::PrismResult<prism_env::EnvironmentSnapshot> {
            let cancel = token.clone();
            let discovery = prism_project::DiscoveryService::new()
                .discover(&root, prism_project::ScanOptions::default(), &cancel, |_| {})
                .ok();
            let project = prism_project::Project::from_root(&root)?;
            let runner: Arc<dyn prism_core::CommandRunner> = Arc::new(StdCommandRunner::new());
            let options = prism_env::EnvironmentScanOptions {
                include_all_tools: all_tools.unwrap_or(false),
                ..Default::default()
            };
            let mut snapshot = prism_env::EnvironmentService::scan(
                &root,
                discovery.as_ref(),
                &options,
                runner,
                &cancel,
            )?;
            snapshot.project_id = project.id.as_str().to_string();
            Ok(snapshot)
        };
        let outcome = execute();

        let mut session = lock_session(&session_arc);
        match outcome {
            Ok(snapshot) => {
                session.env_status = Some(TaskState::Completed);
                session.env_snapshot = Some(snapshot);
            }
            Err(prism_core::PrismError::Cancelled) => {
                session.env_status = Some(TaskState::Cancelled);
            }
            Err(err) => {
                session.env_status = Some(TaskState::Failed);
                session.env_error = Some(err.to_string());
            }
        }
    });

    Ok("started".to_string())
}

/// 查询环境扫描状态。
#[tauri::command]
pub fn get_environment_status(state: State<'_, AppState>) -> EnvironmentStatusDto {
    let session = lock_session(&state.session);
    let status = match session.env_status {
        None => "idle".to_string(),
        Some(status) => serde_json::to_value(status)
            .ok()
            .and_then(|value| value.as_str().map(str::to_string))
            .unwrap_or_else(|| "unknown".to_string()),
    };
    EnvironmentStatusDto {
        status,
        error: session.env_error.clone(),
    }
}

/// 最近一次完成的环境快照（GUI/CLI 同源 §298：来自同一 EnvironmentService）。
#[tauri::command]
pub fn get_environment_results(
    state: State<'_, AppState>,
) -> Option<prism_env::EnvironmentSnapshot> {
    let session = lock_session(&state.session);
    session.env_snapshot.clone()
}

/// 取消进行中的环境扫描（§303）。
#[tauri::command]
pub fn cancel_environment_scan(state: State<'_, AppState>) -> Result<(), PrismErrorDto> {
    let session = lock_session(&state.session);
    session.env_token.cancel();
    Ok(())
}
