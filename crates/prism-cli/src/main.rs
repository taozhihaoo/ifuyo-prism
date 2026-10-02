//! prism — ifuyo Prism 命令行入口。
//!
//! CLI 与 GUI（Tauri）共享同一个 Core / 领域层；CLI 不实现自己的业务逻辑。
//!
//! ## 退出码契约（M0 起 0/1/2，M1 起按纲领 §78 细分并保持稳定）
//!
//! - `0` 成功
//! - `1` 运行错误或命令尚未实现（fatal）
//! - `2` 用法错误（clap 产生）
//! - `3` 输入无效（路径不存在等）
//! - `4` 操作被取消
//! - `5` 成功完成但有警告（scan completed with warnings）

mod cli;

use clap::{CommandFactory, Parser};
use prism_core::application::AppInfoService;
use prism_core::config::OutputFormat;
use prism_core::logging;
use prism_core::task::CancellationToken;
use prism_core::PrismErrorCode;
use prism_project::{DiscoveryService, ProjectDiscovery, ScanOptions};
use std::path::Path;

const EXIT_OK: i32 = 0;
const EXIT_ERROR: i32 = 1;
/// clap 用法错误自身以 2 退出；常量仅为契约文档保持完整。
#[allow(dead_code)]
const EXIT_USAGE: i32 = 2;
const EXIT_INVALID_INPUT: i32 = 3;
const EXIT_CANCELLED: i32 = 4;
const EXIT_WARNINGS: i32 = 5;

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
            EXIT_OK
        }
        Some(cli::Command::Info { format }) => run_info(format.into()),
        Some(cli::Command::Scan { path, format }) => run_scan(&path, format.into()),
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
                EXIT_OK
            }
            Err(err) => {
                eprintln!("prism: failed to serialize app info: {err}");
                EXIT_ERROR
            }
        },
        OutputFormat::Text => {
            println!("name:           {}", info.name);
            println!("version:        {}", info.version);
            println!("os:             {}", info.os);
            println!("arch:           {}", info.arch);
            println!("family:         {}", info.family);
            println!("build profile:  {}", info.build_profile);
            EXIT_OK
        }
    }
}

fn run_scan(path: &Path, format: OutputFormat) -> i32 {
    let service = DiscoveryService::new();
    let cancel = CancellationToken::new();
    let options = ScanOptions::default();
    match service.discover(path, options, &cancel, |_| {}) {
        Ok(discovery) => {
            match format {
                OutputFormat::Json => match serde_json::to_string_pretty(&discovery) {
                    Ok(text) => println!("{text}"),
                    Err(err) => {
                        eprintln!("prism: failed to serialize discovery result: {err}");
                        return EXIT_ERROR;
                    }
                },
                OutputFormat::Text => print_discovery_text(&discovery),
            }
            match discovery.scan.status {
                prism_core::task::TaskState::Cancelled => EXIT_CANCELLED,
                prism_core::task::TaskState::Completed if discovery.scan.errors_count > 0 => {
                    EXIT_WARNINGS
                }
                _ => EXIT_OK,
            }
        }
        Err(err) => {
            eprintln!("prism: scan failed: {err}");
            if matches!(
                err.code(),
                PrismErrorCode::InvalidInput | PrismErrorCode::NotFound
            ) {
                EXIT_INVALID_INPUT
            } else {
                EXIT_ERROR
            }
        }
    }
}

fn print_discovery_text(discovery: &ProjectDiscovery) {
    let stats = &discovery.statistics;
    println!("Project: {}", discovery.project.name);
    println!("Root: {}", discovery.project.root_path.display());
    println!("Files: {}", stats.total_files);
    println!("Directories: {}", stats.total_directories);
    println!("Ignored: {}", stats.ignored_files);
    println!("Errors: {}", stats.error_count);
    for language in &discovery.languages {
        println!(
            "{} files: {}",
            language.language.display_name(),
            language.count
        );
    }
    if !discovery.project_types.is_empty() {
        println!("Project types:");
        for candidate in &discovery.project_types {
            println!(
                "  {} ({}, {} evidence)",
                candidate.name,
                confidence_word(candidate.confidence),
                candidate.evidence.len()
            );
        }
    }
    if !discovery.entry_points.is_empty() {
        println!("Entry points:");
        for entry in &discovery.entry_points {
            println!(
                "  {} ({}, {:?})",
                entry.path,
                confidence_word(entry.confidence),
                entry.kind
            );
        }
    }
    for error in &discovery.errors {
        println!("warning: {}: {}", error.path, error.message);
    }
    if discovery.scan.errors_count > 0 {
        println!("Scan completed with warnings.");
    }
}

fn confidence_word(confidence: prism_core::Confidence) -> &'static str {
    match confidence {
        prism_core::Confidence::High => "high",
        prism_core::Confidence::Medium => "medium",
        prism_core::Confidence::Low => "low",
    }
}

fn not_implemented_yet(name: &str) -> i32 {
    log::debug!("subcommand {name} invoked but not implemented");
    eprintln!("{name}: Not implemented in current milestone.");
    EXIT_ERROR
}
