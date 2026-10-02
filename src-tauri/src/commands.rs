//! Tauri IPC commands（M0 基础集）。
//!
//! 全部返回真实数据（来自 AppInfoService），禁止伪造。
//! 未来里程碑在此追加 scan/changes/health/... 命令，仍只做转发。

use crate::state::AppState;
use prism_core::application::app_info::prism_version;
use prism_core::application::{AppInfo, RuntimeInfo};
use tauri::State;

/// 应用信息（名称、版本、平台、构建 profile）。
#[tauri::command]
pub fn get_app_info(state: State<'_, AppState>) -> AppInfo {
    state.app_info.app_info()
}

/// Prism 版本号。
#[tauri::command]
pub fn get_prism_version(_state: State<'_, AppState>) -> &'static str {
    prism_version()
}

/// 运行时信息（平台与构建事实）。
#[tauri::command]
pub fn get_runtime_info(state: State<'_, AppState>) -> RuntimeInfo {
    state.app_info.runtime_info()
}
