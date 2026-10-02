//! prism — ifuyo Prism 命令行入口。
//!
//! CLI 与 GUI（Tauri）共享同一个 Core / Application 层；
//! CLI 不实现自己的业务逻辑。
//!
//! ## 退出码契约（M0 起保持稳定）
//!
//! - `0` 成功
//! - `1` 运行错误或命令尚未实现
//! - `2` 用法错误（clap 产生）

mod cli;

use clap::{CommandFactory, Parser};
use prism_core::application::AppInfoService;
use prism_core::config::OutputFormat;
use prism_core::logging;

fn main() {
    let exit_code = run();
    std::process::exit(exit_code);
}

fn run() -> i32 {
    let cli = cli::Cli::parse();

    // CLI 日志只写 stderr，不在调用时创建文件（文件日志是 GUI/长任务场景）。
    let level = if cli.verbose {
        log::Level::Debug
    } else {
        log::Level::Info
    };
    if let Err(err) = logging::init(level, None) {
        eprintln!("warning: failed to initialize logging: {err}");
    }

    match cli.command {
        None => {
            let _ = cli::Cli::command().print_help();
            0
        }
        Some(cli::Command::Info { format }) => run_info(format.into()),
        Some(command) => not_implemented_yet(command.name()),
    }
}

fn run_info(format: OutputFormat) -> i32 {
    let service = AppInfoService::new();
    let info = service.app_info();
    match format {
        OutputFormat::Json => match serde_json::to_string_pretty(&info) {
            Ok(text) => {
                println!("{text}");
                0
            }
            Err(err) => {
                eprintln!("prism: failed to serialize app info: {err}");
                1
            }
        },
        OutputFormat::Text => {
            println!("name:           {}", info.name);
            println!("version:        {}", info.version);
            println!("os:             {}", info.os);
            println!("arch:           {}", info.arch);
            println!("family:         {}", info.family);
            println!("build profile:  {}", info.build_profile);
            0
        }
    }
}

fn not_implemented_yet(name: &str) -> i32 {
    log::debug!("subcommand {name} invoked but not implemented");
    eprintln!("{name}: Not implemented in current milestone.");
    1
}
