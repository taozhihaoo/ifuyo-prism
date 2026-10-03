//! `prism env <path>` — 环境扫描基础命令（规格 M9a §240–§242/§268/§295–§297）。
//!
//! stdout 只输出数据（JSON/文本报告，§295）；人类进度不打印（本命令足够快）。
//! exit：0 = 无 Unsatisfied；1 = 有 Unsatisfied（或 --strict 下有 Unknown）；
//! 3 = 路径不存在等输入错误。Unknown ≠ Unsatisfied（§242 默认宽松）。

use prism_core::runtime::command::{CommandRunner, StdCommandRunner};
use prism_core::task::CancellationToken;
use prism_env::{EnvironmentScanOptions, EnvironmentService};
use std::path::Path;
use std::sync::Arc;

pub fn run(
    path: &Path,
    json: bool,
    strict: bool,
    all_tools: bool,
    refresh: bool,
    format: Option<crate::cli::HealthFormatArg>,
) -> i32 {
    let _ = refresh; // §378：扫描本就每次全新执行；标志仅声明语义。
    scan_and_report(path, json, strict, all_tools, format, Section::Full)
}

/// §237/§238 子命令：按节输出。
pub fn run_section(
    section: Section,
    path: &Path,
    json: bool,
) -> i32 {
    scan_and_report(path, json, false, false, None, section)
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Full,
    Requirements,
    Tools,
    Ports,
    Processes,
}

fn scan_and_report(
    path: &Path,
    json: bool,
    strict: bool,
    all_tools: bool,
    format: Option<crate::cli::HealthFormatArg>,
    section: Section,
) -> i32 {
    if !path.exists() {
        eprintln!("prism: path does not exist: {}", path.to_string_lossy());
        return crate::EXIT_INVALID_INPUT;
    }
    let runner: Arc<dyn CommandRunner> = Arc::new(StdCommandRunner::new());
    let cancel = CancellationToken::new();

    // §152/§334：复用一次 M1 discovery（失败 ⇒ generic profile，§122 不失败）。
    let discovery = prism_project::DiscoveryService::new()
        .discover(path, prism_project::ScanOptions::default(), &cancel, |_| {})
        .ok();
    let project = match prism_project::Project::from_root(path) {
        Ok(project) => project,
        Err(err) => {
            eprintln!("prism: cannot open project: {err}");
            return crate::EXIT_INVALID_INPUT;
        }
    };

    let options = EnvironmentScanOptions {
        include_all_tools: all_tools,
        ..Default::default()
    };
    let mut snapshot = match EnvironmentService::scan(
        path,
        discovery.as_ref(),
        &options,
        runner.clone(),
        &cancel,
    ) {
        Ok(snapshot) => snapshot,
        Err(err) => {
            eprintln!("prism: environment scan failed: {err}");
            return crate::EXIT_ERROR;
        }
    };
    snapshot.project_id = project.id.as_str().to_string();

    let wants_json = json || format == Some(crate::cli::HealthFormatArg::Json);
    let exit_code = report(&snapshot, section, wants_json, format);
    if exit_code != 0 {
        return exit_code;
    }

    // §240/§242：Unsatisfied ⇒ 非零；Unknown 默认不算失败；--strict 才算（§241）。
    // （Full 视图才参与 exit code；分节视图只读。）
    let mut unsatisfied = 0usize;
    let mut unknown = 0usize;
    for matched in &snapshot.matches {
        match matched.result {
            prism_env::MatchResult::Unsatisfied => unsatisfied += 1,
            prism_env::MatchResult::Unknown => unknown += 1,
            _ => {}
        }
    }
    if section == Section::Full && unsatisfied > 0 {
        eprintln!(
            "prism: {unsatisfied} environment requirement(s) unsatisfied"
        );
        return crate::EXIT_ERROR;
    }
    if section == Section::Full && strict && unknown > 0 {
        eprintln!(
            "prism: --strict: {unknown} environment requirement(s) unknown (treated as failure)"
        );
        return crate::EXIT_ERROR;
    }
    crate::EXIT_OK
}


/// 分节/全量输出（§237/§239：stdout 只有数据）。
fn report(
    snapshot: &prism_env::EnvironmentSnapshot,
    section: Section,
    json: bool,
    format: Option<crate::cli::HealthFormatArg>,
) -> i32 {
    if json {
        let value = match section {
            Section::Full => serde_json::json!(snapshot),
            Section::Requirements => serde_json::json!({
                "requirements": snapshot.requirements,
                "conflicts": snapshot.conflicts,
                "matches": snapshot.matches,
            }),
            Section::Tools => serde_json::json!({
                "tools": snapshot.tools,
                "runtimes": snapshot.runtimes,
                "sdks": snapshot.sdks,
                "package_managers": snapshot.package_managers,
                "virtualenvs": snapshot.virtualenvs,
            }),
            Section::Ports => serde_json::json!({
                "ports": snapshot.ports,
                "expected_ports": snapshot.expected_ports,
            }),
            Section::Processes => serde_json::json!({
                "processes": snapshot.processes,
            }),
        };
        match serde_json::to_string_pretty(&value) {
            Ok(text) => println!("{text}"),
            Err(err) => {
                eprintln!("prism: failed to serialize environment data: {err}");
                return crate::EXIT_ERROR;
            }
        }
        return crate::EXIT_OK;
    }
    match section {
        Section::Full => match format {
            Some(crate::cli::HealthFormatArg::Markdown) => {
                print!("{}", prism_env::render_markdown(snapshot))
            }
            Some(crate::cli::HealthFormatArg::Html) => {
                print!("{}", prism_env::render_html(snapshot))
            }
            _ => print!("{}", prism_env::render_text(snapshot)),
        },
        Section::Requirements => {
            println!("Requirements ({} total)", snapshot.requirements.len());
            for (requirement, matched) in snapshot
                .requirements
                .iter()
                .zip(snapshot.matches.iter())
            {
                println!(
                    "  {:<16} {} {}  =>  {:<12} [{}]",
                    requirement.subject,
                    requirement.operator.as_str(),
                    requirement.expected,
                    matched.result.as_str(),
                    requirement.source
                );
            }
            for conflict in &snapshot.conflicts {
                println!(
                    "  CONFLICT {} ({} sources):",
                    conflict.subject,
                    conflict.entries.len()
                );
                for entry in &conflict.entries {
                    println!(
                        "    {} ({}): {} {}",
                        entry.source,
                        entry.source_kind.as_str(),
                        entry.operator.as_str(),
                        entry.expected
                    );
                }
            }
        }
        Section::Tools => {
            for tool in &snapshot.tools {
                println!(
                    "  {:<10} {:<36} {:<12} {}",
                    tool.name,
                    tool.raw_version.as_deref().unwrap_or("version unknown"),
                    format!("{:?}", tool.probe),
                    tool.path.as_deref().unwrap_or("-")
                );
            }
        }
        Section::Ports => {
            for expected in &snapshot.expected_ports {
                println!("  expected {} (from {})", expected.port, expected.source);
            }
            for port in &snapshot.ports {
                println!(
                    "  {:<6} LISTENING {} pid={:?} process={:?} observed={}",
                    port.port, port.address, port.pid, port.process_name, port.observed_at
                );
            }
        }
        Section::Processes => {
            for process in &snapshot.processes {
                println!(
                    "  pid {:<7} {:<24} ports={:?} class={} observed={}",
                    process.pid,
                    process.name,
                    process.ports,
                    process.classification.as_str(),
                    process.observed_at
                );
            }
        }
    }
    crate::EXIT_OK
}
