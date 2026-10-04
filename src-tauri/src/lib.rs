//! Prism 桌面应用壳（Tauri 2）。
//!
//! 本 crate 只是 IPC 边界：commands 接收前端调用、调用 Application 层、
//! 返回 Core 数据。业务逻辑一律在 `prism-core`，GUI 不自建逻辑。

mod commands;
mod context_commands;
mod env_commands;
mod snapshot_commands;
mod state;
mod verify_commands;

use prism_core::application::app_info::prism_version;
use prism_core::config::{self, PrismConfig};
use prism_core::logging;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let (config, loaded_from_disk) =
        match config::default_config_path().and_then(|path| PrismConfig::load(&path)) {
            Ok(pair) => pair,
            Err(err) => {
                // 配置损坏时退回默认值并继续启动；让用户看到日志而不是白屏。
                eprintln!("warning: failed to load config, using defaults: {err}");
                (PrismConfig::default(), false)
            }
        };

    let level = config.logging.level.to_log_level();
    let log_file = if config.logging.file_enabled {
        config::resolve_data_dir(&config)
            .ok()
            .map(|dir| logging::default_log_file(&dir))
    } else {
        None
    };
    if let Err(err) = logging::init(level, log_file.as_deref()) {
        eprintln!("warning: failed to initialize logging: {err}");
    }
    log::info!(
        "prism {} starting (config loaded from disk: {loaded_from_disk})",
        prism_version()
    );

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(state::AppState::new(config))
        .invoke_handler(tauri::generate_handler![
            commands::get_app_info,
            commands::get_prism_version,
            commands::get_runtime_info,
            commands::open_project,
            commands::start_scan,
            commands::cancel_scan,
            commands::get_scan_status,
            commands::get_discovery,
            commands::start_inspect,
            commands::get_inspect_status,
            commands::get_code_model,
            commands::cancel_inspect,
            commands::start_graph,
            commands::get_graph_status,
            commands::get_dependency_graph,
            commands::get_git_facts,
            commands::start_changeset,
            commands::get_changeset_status,
            commands::get_changeset,
            commands::cancel_changeset,
            commands::start_impact,
            commands::get_impact_status,
            commands::get_impact,
            commands::cancel_impact,
            commands::start_health,
            commands::get_health_status,
            commands::get_health_analysis,
            commands::cancel_health,
            verify_commands::verify_plan,
            verify_commands::start_verify,
            verify_commands::get_verify_status,
            verify_commands::get_verify_results,
            verify_commands::cancel_verify,
            env_commands::start_environment_scan,
            env_commands::get_environment_status,
            env_commands::get_environment_results,
            env_commands::cancel_environment_scan,
            context_commands::start_context_generation,
            context_commands::get_context_status,
            context_commands::get_context_results,
            context_commands::cancel_context_generation,
            context_commands::export_context,
            snapshot_commands::snapshot_capture,
            snapshot_commands::snapshot_history,
            snapshot_commands::compare_snapshots,
            snapshot_commands::snapshot_timeline,
            snapshot_commands::delete_snapshot,
        ])
        .run(tauri::generate_context!())
        .unwrap_or_else(|err| {
            eprintln!("fatal: tauri runtime failed to start: {err}");
            std::process::exit(1);
        });
}
