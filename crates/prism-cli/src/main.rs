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
//! - `6` 非 Git 项目（`prism git <cmd>` 明确请求 Git 时）

mod changes_command;
mod cli;
mod health_command;
mod verify_command;
mod verify_subcommands;
mod impact_command;
mod git_command;

use clap::{CommandFactory, Parser};
use prism_analysis::{CodeAnalysisOptions, CodeAnalysisService, ProjectCodeModel};
use prism_core::application::AppInfoService;
use prism_core::config::OutputFormat;
use prism_core::logging;
use prism_core::runtime::command::{CommandRunner, StdCommandRunner};
use prism_core::task::CancellationToken;
use prism_core::PrismErrorCode;
use prism_graph::{DependencyGraphResult, DependencyService, GraphNodeKind};
use prism_project::{DiscoveryService, ProjectDiscovery, ScanOptions};
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

const EXIT_OK: i32 = 0;
const EXIT_ERROR: i32 = 1;
/// clap 用法错误自身以 2 退出；常量仅为契约文档保持完整。
#[allow(dead_code)]
const EXIT_USAGE: i32 = 2;
const EXIT_INVALID_INPUT: i32 = 3;
const EXIT_CANCELLED: i32 = 4;
const EXIT_WARNINGS: i32 = 5;
/// M4：`prism git <cmd>` 在非 Git 项目上的专用退出码（规格 §39）。
const EXIT_NOT_GIT: i32 = 6;

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
        Some(cli::Command::Inspect { path, format }) => run_inspect(&path, format.into()),
        Some(cli::Command::Graph { path, kind, format }) => {
            run_graph(&path, kind.as_str(), format.into())
        }
        Some(cli::Command::Git { path, command }) => {
            let runner: Arc<dyn CommandRunner> = Arc::new(StdCommandRunner::new());
            git_command::run(&path, &command, runner)
        }
        Some(cli::Command::Impact {
            path,
            staged,
            from,
            to,
            format,
            max_files,
            max_depth,
            max_nodes,
            include_tests,
            include_docs,
        }) => impact_command::run(
            &path,
            staged,
            from.as_deref(),
            to.as_deref(),
            format,
            max_files,
            max_depth,
            max_nodes,
            include_tests,
            include_docs,
        ),
        Some(cli::Command::Changes {
            path,
            staged,
            from,
            to,
            format,
            max_files,
        }) => changes_command::run(&path, staged, from.as_deref(), to.as_deref(), format, max_files),
        Some(cli::Command::Health {
            path,
            json,
            format,
            changed,
            rule,
            category,
            fail_on,
            max_files,
            command,
        }) => match command {
            Some(cli::HealthCommand::Rules { json }) => {
                health_command::run_rules(&path, json)
            }
            None => health_command::run_analyze(
                &path,
                if json {
                    crate::cli::HealthFormatArg::Json
                } else {
                    format
                },
                changed,
                rule.as_deref(),
                category.as_deref(),
                fail_on,
                max_files,
            ),
        },
        Some(cli::Command::Verify {
            path,
            json,
            format,
            checks,
            changed,
            dry_run,
            fail_on_unavailable,
            command,
        }) => match command {
            Some(cli::VerifyCommand::Checks { path: sub_path, .. }) => {
                verify_subcommands::run_checks(&sub_path)
            }
            Some(cli::VerifyCommand::Ci { path: sub_path, json: _ }) => {
                verify_subcommands::run_ci(&sub_path)
            }
            Some(cli::VerifyCommand::Release { path: sub_path, json: _ }) => {
                verify_subcommands::run_release(&sub_path)
            }
            None => verify_command::run(
                &path,
                if json {
                    crate::cli::HealthFormatArg::Json
                } else {
                    format
                },
                checks.as_slice(),
                dry_run,
                changed,
                fail_on_unavailable,
            ),
        },
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

fn run_inspect(path: &Path, format: OutputFormat) -> i32 {
    let cancel = CancellationToken::new();
    let model =
        CodeAnalysisService::new().inspect(path, CodeAnalysisOptions::default(), &cancel, |_| {});
    match model {
        Ok(model) => {
            match format {
                OutputFormat::Json => match serde_json::to_string_pretty(&model) {
                    Ok(text) => println!("{text}"),
                    Err(err) => {
                        eprintln!("prism: failed to serialize code model: {err}");
                        return EXIT_ERROR;
                    }
                },
                OutputFormat::Text => print_inspect_text(&model),
            }
            match model.summary.status {
                prism_core::task::TaskState::Cancelled => EXIT_CANCELLED,
                _ if model.summary.files_failed > 0 || model.summary.error_count > 0 => {
                    EXIT_WARNINGS
                }
                _ => EXIT_OK,
            }
        }
        Err(err) => {
            eprintln!("prism: inspect failed: {err}");
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

fn print_inspect_text(model: &ProjectCodeModel) {
    let summary = &model.summary;
    println!("ifuyo-prism");
    println!("Code Understanding");
    println!();
    println!("Project: {}", model.project.name);
    println!("Root: {}", model.project.root_path.display());
    println!();
    println!("Files");
    println!("  Source: {}", summary.files_total);
    println!("  Parsed: {}", summary.files_parsed);
    println!("  With diagnostics: {}", summary.files_with_diagnostics);
    println!("  Unsupported: {}", summary.files_unsupported);
    println!("  Skipped: {}", summary.files_skipped);
    println!("  Failed: {}", summary.files_failed);
    println!();
    println!("Symbols: {}", summary.symbols_total);
    let mut kind_counts: BTreeMap<&str, u64> = BTreeMap::new();
    for symbol in model.symbols() {
        *kind_counts.entry(symbol.kind.as_str()).or_insert(0) += 1;
    }
    for (kind, count) in &kind_counts {
        println!("  {kind}: {count}");
    }
    println!();
    println!("Diagnostics");
    println!("  Errors: {}", summary.error_count);
    println!("  Warnings: {}", summary.warning_count);
    println!();
    println!("Top files");
    for result in model.parse_results.iter().take(5) {
        println!("  {}", result.relative_path);
        // 顶层符号（File 根的直接子级），每个最多展示 5 个。
        let file_symbol = result.symbols.first();
        let Some(root_symbol) = file_symbol else {
            continue;
        };
        let top_level = model
            .symbols()
            .filter(|s| s.parent_id.as_ref() == Some(&root_symbol.id))
            .take(5);
        for symbol in top_level {
            println!("    {} ({})", symbol.name, symbol.kind.as_str());
        }
    }
    if summary.status == prism_core::task::TaskState::Cancelled {
        println!();
        println!("Inspect cancelled — partial results above.");
    }
}

fn run_graph(path: &Path, kind: &str, format: OutputFormat) -> i32 {
    let cancel = CancellationToken::new();
    let result = DependencyService::new().inspect_graph(path, &cancel);
    match result {
        Ok(graph) => {
            match format {
                OutputFormat::Json => match serde_json::to_string_pretty(&graph) {
                    Ok(text) => println!("{text}"),
                    Err(err) => {
                        eprintln!("prism: failed to serialize dependency graph: {err}");
                        return EXIT_ERROR;
                    }
                },
                OutputFormat::Text => print_graph_text(&graph, kind),
            }
            let _ = kind;
            EXIT_OK
        }
        Err(err) => {
            eprintln!("prism: graph failed: {err}");
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

fn print_graph_text(graph: &DependencyGraphResult, kind: &str) {
    let metrics = &graph.metrics;
    println!("ifuyo-prism");
    println!("Dependency & Architecture ({kind} view)");
    println!();
    println!("Project: {}", graph.project_name);
    println!();
    println!("Graph");
    println!("  Nodes: {}", metrics.node_count);
    println!("  Edges: {}", metrics.edge_count);
    println!("  Resolved: {}", metrics.resolved_edges);
    println!("  External: {}", metrics.external_edges);
    println!("  Builtin: {}", metrics.builtin_edges);
    println!("  Unresolved: {}", metrics.unresolved_edges);
    println!("  Ambiguous: {}", metrics.ambiguous_edges);
    println!("  Cycles: {}", metrics.cycle_count);
    println!("  Components: {}", metrics.connected_components);
    println!();
    if !metrics.most_referenced.is_empty() {
        println!("Most referenced (fan-in)");
        for (id, count) in metrics.most_referenced.iter().take(5) {
            println!("  {count:>4}  {id}");
        }
        println!();
    }
    if !metrics.most_dependent.is_empty() {
        println!("Most dependent (fan-out)");
        for (id, count) in metrics.most_dependent.iter().take(5) {
            println!("  {count:>4}  {id}");
        }
        println!();
    }
    if !graph.cycles.is_empty() {
        println!("Cycles");
        for cycle in graph.cycles.iter().take(10) {
            println!("  {} ({})", cycle.id, cycle.nodes.len());
        }
        println!();
    }
    if !graph.diagnostics.is_empty() {
        println!("Unresolved dependencies");
        for diagnostic in graph.diagnostics.iter().take(20) {
            println!(
                "  {}: {} — {}",
                diagnostic.source_path, diagnostic.raw_target, diagnostic.reason
            );
        }
        println!();
    }
    // 按层级视图展示节点（file/module/project 视图过滤）。
    println!("Nodes ({kind})");
    let wanted = match kind {
        "module" => "namespace",
        "project" => "project",
        _ => "file",
    };
    for node in graph.nodes.iter().take(30) {
        let matches = match kind {
            "module" => {
                node.kind == GraphNodeKind::Package || node.kind == GraphNodeKind::Namespace
            }
            "project" => node.kind == GraphNodeKind::Project,
            _ => node.kind == GraphNodeKind::FileModule,
        };
        if matches {
            println!("  {}", node.label);
        }
        if wanted == "never" {
            break;
        }
    }
    let total: usize = graph
        .nodes
        .iter()
        .filter(|node| match kind {
            "module" => {
                node.kind == GraphNodeKind::Package || node.kind == GraphNodeKind::Namespace
            }
            "project" => node.kind == GraphNodeKind::Project,
            _ => node.kind == GraphNodeKind::FileModule,
        })
        .count();
    if total > 30 {
        println!("  … and {} more", total - 30);
    }
    if graph.diagnostics.is_empty() && graph.metrics.unresolved_edges == 0 {
        println!();
        println!("No unresolved dependencies.");
    }
}

fn not_implemented_yet(name: &str) -> i32 {
    log::debug!("subcommand {name} invoked but not implemented");
    eprintln!("{name}: Not implemented in current milestone.");
    EXIT_ERROR
}
