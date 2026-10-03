//! `prism verify checks / ci / release` 子命令（规格 M8 §55/§60-§69/§62/§136-§139）。
//!
//! - checks：列出发现的验证入口 + source/provenance（§55）
//! - ci：静态检视 GitHub Actions + 本地对照（§64-§69；零执行，§65/§242）
//! - release：基于事实的 release 检查（§62/§136-§139；零部署 §245）

use prism_core::config::PrismConfig;
use prism_core::runtime::command::StdCommandRunner;
use prism_core::task::CancellationToken;
use prism_git::GitService;
use prism_verify::{compare_with_local, discover_checks, evaluate_release, CheckKind, inspect_github_actions};
use std::path::Path;
use std::sync::Arc;

fn load_config(path: &Path) -> Result<PrismConfig, i32> {
    let config_path = path.join(PrismConfig::config_file_name());
    PrismConfig::load(&config_path).map(|(config, _)| config).map_err(|err| {
        eprintln!("prism: config invalid: {err}");
        crate::EXIT_ERROR
    })
}

/// `prism verify checks <path>`（§55）：列出验证入口 + source。
pub fn run_checks(path: &Path) -> i32 {
    if !path.exists() {
        eprintln!("prism: path does not exist: {}", path.to_string_lossy());
        return crate::EXIT_INVALID_INPUT;
    }
    let Ok(config) = load_config(path) else {
        return crate::EXIT_ERROR;
    };
    let specs = discover_checks(path, &config.verify);
    if specs.is_empty() {
        println!("No verification entry points detected.");
        println!("Configure custom checks under verify.checks in prism.config.json.");
        return crate::EXIT_OK;
    }
    println!("Detected verification entry points:\n");
    for spec in &specs {
        println!(
            "{:<11} {:?}  {}  [{}]  source: {}",
            spec.id,
            spec.kind,
            spec.name,
            if spec.required { "required" } else { "optional" },
            spec.detected_from
        );
        let _ = spec.program.len();
    }
    crate::EXIT_OK
}
/// `prism verify ci <path>`（§64-§69）：静态检视 + 本地对照。
pub fn run_ci(path: &Path) -> i32 {
    if !path.exists() {
        eprintln!("prism: path does not exist: {}", path.to_string_lossy());
        return crate::EXIT_INVALID_INPUT;
    }
    let Ok(config) = load_config(path) else {
        return crate::EXIT_ERROR;
    };
    let inspection = inspect_github_actions(path);
    if inspection.config_files.is_empty() {
        println!("No CI configuration found (only GitHub Actions workflows are inspected).");
        return crate::EXIT_OK;
    }
    println!(
        "CI provider: {} ({} config file(s))\n",
        inspection.provider,
        inspection.config_files.len()
    );
    println!("CI commands (inspection only — never executed by Prism):\n");
    for check in &inspection.checks {
        println!(
            "  [{}] {}  ({})",
            check.job, check.command, check.location
        );
    }

    let specs = discover_checks(path, &config.verify);
    let local_ids: Vec<String> = specs.iter().map(|spec| spec.id.clone()).collect();
    let coverage = compare_with_local(&inspection, &local_ids);
    println!("\nLocal verification entry points:");
    for id in &coverage.local {
        println!("  {id}");
    }
    println!("\nShared (local check matched in CI):");
    for id in &coverage.shared {
        println!("  {id}");
    }
    if !coverage.ci_only.is_empty() {
        println!("\nCI-only commands (informational, not judged):");
        for command in &coverage.ci_only {
            println!("  {command}");
        }
    }
    crate::EXIT_OK
}

/// `prism verify release <path>`（§62/§136-§139/§308-§309）。
pub fn run_release(path: &Path) -> i32 {
    if !path.exists() {
        eprintln!("prism: path does not exist: {}", path.to_string_lossy());
        return crate::EXIT_INVALID_INPUT;
    }
    let Ok(config) = load_config(path) else {
        return crate::EXIT_ERROR;
    };
    let runner: Arc<dyn prism_core::CommandRunner> = Arc::new(StdCommandRunner::new());
    let cancel = CancellationToken::new();
    let git = GitService::new(runner.clone());

    // 事实收集：build/test 结论来自真实执行的 CheckRuns（§62）。
    let specs = discover_checks(path, &config.verify);
    let selected: Vec<&prism_verify::CheckSpec> = specs
        .iter()
        .filter(|spec| matches!(spec.kind, CheckKind::Build | CheckKind::Test))
        .collect();
    let mut build_passed = None;
    let mut test_passed = None;
    if !selected.is_empty() {
        let runs = prism_verify::execute_plan_with(
            &selected
                .iter()
                .map(|spec| (*spec).clone())
                .collect::<Vec<_>>(),
            runner,
            path,
            2,
            &cancel,
        )
        .unwrap_or_default();
        let (build, test) = prism_verify::build_test_conclusions(&runs);
        build_passed = build;
        test_passed = test;
    }

    let report = evaluate_release(
        path,
        Some(&git),
        &config.verify.release.required_files,
        config.verify.release.require_clean_tree,
        build_passed,
        test_passed,
        &cancel,
    );

    println!("Release verification: {}\n", report.conclusion);
    for check in &report.checks {
        println!(
            "{:<24} {:>8}  {}",
            check.title, check.state, check.detail
        );
    }
    if report.conclusion == "FAIL" {
        crate::EXIT_ERROR
    } else {
        crate::EXIT_OK
    }
}
