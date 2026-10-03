//! M8 GUI：verify 计划/执行 IPC（§112/§117/§118/§270-§277）。
//!
//! 生命周期遵守 §268/§297/§298：先 discover+plan（dry-run 语义），
//! 用户显式 Run 才执行；取消传播；结果存 session（§129/§273，无数据库）。
//! verified_state 记录 CleanAtCommit/DirtyWorkspace（§276/§277）。

use crate::state::lock_session;
use prism_core::error::PrismErrorDto;
use crate::state::AppState;
use prism_core::config::PrismConfig;
use prism_core::runtime::command::StdCommandRunner;
use prism_core::task::TaskState;
use prism_verify::discover_checks;
use serde::Serialize;
use std::sync::Arc;
use tauri::State;

/// verify 会话状态 DTO（前端轮询）。
#[derive(Debug, Clone, Serialize)]
pub struct VerifyStatusDto {
    /// "idle" | "planning" | "running" | "completed" | "failed" | "cancelled"
    pub status: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct VerifyPlanDto {
    pub checks: Vec<prism_verify::PlannedCheck>,
}

fn verify_root(state: &State<'_, AppState>) -> Result<std::path::PathBuf, PrismErrorDto> {
    let session = lock_session(&state.session);
    session
        .project
        .as_ref()
        .map(|project| project.root_path.clone())
        .ok_or_else(|| {
            prism_core::error::PrismErrorDto::new("invalid_input", "no project opened; call open_project first")
        })
}

/// 发现验证入口并生成计划（不执行；§268 第一阶段）。
#[tauri::command]
pub fn verify_plan(state: State<'_, AppState>) -> Result<VerifyPlanDto, PrismErrorDto> {
    let root = verify_root(&state)?;
    let config_path = root.join(PrismConfig::config_file_name());
    let (config, _loaded) = PrismConfig::load(&config_path).map_err(|err| {
        PrismErrorDto::new("config", format!("config invalid: {err}"))
    })?;
    let specs = discover_checks(&root, &config.verify);
    let plan = prism_verify::plan_checks(&specs);
    let mut session = lock_session(&state.session);
    session.verify_plan = Some(plan.clone());
    Ok(VerifyPlanDto { checks: plan.checks })
}

/// 显式执行验证（§118 Run all / Run selected；§297 用户可见执行意图）。
#[tauri::command]
pub fn start_verify(
    state: State<'_, AppState>,
    checks: Option<Vec<String>>,
) -> Result<String, PrismErrorDto> {
    let root = verify_root(&state)?;
    let config_path = root.join(PrismConfig::config_file_name());
    let (config, _loaded) = PrismConfig::load(&config_path).map_err(|err| {
        PrismErrorDto::new("config", format!("config invalid: {err}"))
    })?;
    let (root, token, session_arc) = {
        let mut session = lock_session(&state.session);
        if session.verify_status == Some(TaskState::Running) {
            return Err(PrismErrorDto::new(
                "invalid_input",
                "a verification is already running",
            ));
        }
        session.begin_verify();
        let token = session.verify_token.clone();
        (root, token, state.session.clone())
    };

    std::thread::spawn(move || {
        let execute = || -> prism_core::PrismResult<prism_verify::VerificationReport> {
            let specs = discover_checks(&root, &config.verify);
            let options = prism_verify::VerifyOptions {
                selected: checks.clone(),
                ..Default::default()
            };
            let runner: Arc<dyn prism_core::CommandRunner> = Arc::new(StdCommandRunner::new());
            let git = prism_git::GitService::new(runner.clone());
            prism_verify::run_verification(
                &specs,
                &root,
                &root
                    .file_name()
                    .map(|name| name.to_string_lossy().to_string())
                    .unwrap_or_else(|| "project".to_string()),
                &options,
                runner,
                2,
                true,
                Some(&git),
                &[],
                &token,
            )
        };
        let outcome = execute();

        let mut session = lock_session(&session_arc);
        match outcome {
            Ok(report) => {
                session.verify_status = Some(TaskState::Completed);
                session.verify_report = Some(report);
            }
            Err(prism_core::PrismError::Cancelled) => {
                session.verify_status = Some(TaskState::Cancelled);
            }
            Err(err) => {
                session.verify_status = Some(TaskState::Failed);
                session.verify_error = Some(err.to_string());
            }
        }
    });

    Ok("started".to_string())
}

/// 查询 verify 会话状态。
#[tauri::command]
pub fn get_verify_status(state: State<'_, AppState>) -> VerifyStatusDto {
    let session = lock_session(&state.session);
    let status = match session.verify_status {
        None => "idle".to_string(),
        Some(status) => serde_json::to_value(status)
            .ok()
            .and_then(|v| v.as_str().map(str::to_string))
            .unwrap_or_else(|| "unknown".to_string()),
    };
    VerifyStatusDto {
        status,
        error: session.verify_error.clone(),
    }
}

/// 获取最近一次完成的验证报告与计划。
#[tauri::command]
pub fn get_verify_results(
    state: State<'_, AppState>,
) -> Option<(VerifyPlanDto, Option<prism_verify::VerificationReport>)> {
    let session = lock_session(&state.session);
    session
        .verify_plan
        .clone()
        .map(|plan| (VerifyPlanDto { checks: plan.checks }, session.verify_report.clone()))
}

/// 取消当前验证（§93：传播/终止/清理/标记 Cancelled）。
#[tauri::command]
pub fn cancel_verify(state: State<'_, AppState>) -> Result<(), PrismErrorDto> {
    let session = lock_session(&state.session);
    session.verify_token.cancel();
    Ok(())
}
