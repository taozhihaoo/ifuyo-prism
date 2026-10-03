//! `prism verify <path>` — 发现 → 计划 → 执行 → 汇总（规格 M8 §141–§158）。
//!
//! 退出码（§75/§79）：0 = 全部 required Passed；1 = required failed 或
//! --fail-on-unavailable 命中；4 = 取消；3 = 路径/选择无效。

use prism_core::config::PrismConfig;
use prism_core::runtime::command::{CommandRunner, StdCommandRunner};
use prism_core::task::CancellationToken;
use prism_verify::{discover_checks, render_html, render_text, CheckState};
use prism_git::GitService;
use std::path::Path;
use std::sync::Arc;

pub fn run(
    path: &Path,
    format: crate::cli::HealthFormatArg,
    checks: &[String],
    dry_run: bool,
    changed: bool,
    fail_on_unavailable: bool,
) -> i32 {
    if !path.exists() {
        eprintln!("prism: path does not exist: {}", path.to_string_lossy());
        return crate::EXIT_INVALID_INPUT;
    }
    let runner: Arc<dyn CommandRunner> = Arc::new(StdCommandRunner::new());
    let cancel = CancellationToken::new();

    // 项目配置（verify.checks / verify.required；不存在 ⇒ 默认）。
    let config_path = path.join(PrismConfig::config_file_name());
    let (config, _loaded) = match PrismConfig::load(&config_path) {
        Ok(loaded) => loaded,
        Err(err) => {
            eprintln!("prism: config invalid: {err}");
            return crate::EXIT_ERROR;
        }
    };

    let options = prism_verify::VerifyOptions {
        selected: if checks.is_empty() {
            None
        } else {
            Some(checks.to_vec())
        },
        dry_run,
        fail_on_unavailable,
    };

    // §167/§198：GitService 用于副作用观察（非 git 目录静默跳过）。
    let git = GitService::new(runner.clone());

    let specs = discover_checks(path, &config.verify);

    let project_name = path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "project".to_string());

    // §32-§36：--changed 时展示受影响测试候选（Candidate impacted tests，
    // 绝不冒充 "Verified affected tests"）。npm 等工具不在本地 PATH 时仍可列出。
    let impacted_test_candidates: Vec<String> = if changed {
        let cancel_probe = CancellationToken::new();
        prism_project::DiscoveryService::new()
            .discover(path, prism_project::ScanOptions::default(), &cancel_probe, |_| {})
            .map(|discovery| {
                discovery
                    .files
                    .iter()
                    .filter(|file| file.classification.kind == prism_project::FileKind::Test)
                    .map(|file| file.relative_path.clone())
                    .collect()
            })
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    let report = match prism_verify::run_verification(
        &specs,
        path,
        &project_name,
        &options,
        runner,
        2,
        true,
        Some(&git),
        &impacted_test_candidates,
        &cancel,
    ) {
        Ok(report) => report,
        Err(prism_core::PrismError::Cancelled) => {
            eprintln!("prism: verify cancelled");
            return crate::EXIT_CANCELLED;
        }
        Err(prism_core::PrismError::InvalidInput { message }) => {
            eprintln!("prism: {message}");
            return crate::EXIT_INVALID_INPUT;
        }
        Err(err) => {
            eprintln!("prism: verify failed: {err}");
            return crate::EXIT_ERROR;
        }
    };

    match format {
        crate::cli::HealthFormatArg::Json => match serde_json::to_string_pretty(&report) {
            Ok(text) => println!("{text}"),
            Err(err) => {
                eprintln!("prism: failed to serialize verify report: {err}");
                return crate::EXIT_ERROR;
            }
        },
        crate::cli::HealthFormatArg::Text => print!("{}", render_text(&report)),
        crate::cli::HealthFormatArg::Markdown => print!("{}", render_text(&report)),
        crate::cli::HealthFormatArg::Html => print!("{}", render_html(&report)),
    }

    // §75/§79：required failed ⇒ 1；--fail-on-unavailable 命中 ⇒ 1；其余 0。
    let failed = report.counts.failed > 0
        || report
            .runs
            .iter()
            .any(|run| run.required && run.state != CheckState::Passed);
    if failed {
        return crate::EXIT_ERROR;
    }
    if fail_on_unavailable
        && (report.counts.unavailable > 0 || report.counts.blocked > 0)
    {
        eprintln!(
            "prism: --fail-on-unavailable: {} unavailable / {} blocked",
            report.counts.unavailable, report.counts.blocked
        );
        return crate::EXIT_ERROR;
    }
    crate::EXIT_OK
}
