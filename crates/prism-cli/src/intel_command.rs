//! `prism intel <path>` — 2.0-M21 Intelligence Core 垂直切片：
//! 既有 M3 依赖分析 → 统一 Core 模型（prism-intel）。
//!
//! 旧 `prism graph` JSON 契约保持不变（M21 §32 additive first）；
//! 本命令是 Core 模型的第一个生产消费面（CLI；GUI/MCP 待对应能力落地）。

use prism_core::task::CancellationToken;
use prism_graph::DependencyService;
use prism_intel::{AnalysisContext, AnalysisScope};
use prism_project::Project;
use std::path::Path;

/// `prism intel <path> [--json]`。
pub fn run(path: &Path, json: bool) -> i32 {
    if !path.exists() {
        eprintln!("prism: path does not exist: {}", path.to_string_lossy());
        return crate::EXIT_INVALID_INPUT;
    }
    let project = match Project::from_root(path) {
        Ok(project) => project,
        Err(err) => {
            eprintln!("prism: cannot open project: {err}");
            return crate::EXIT_INVALID_INPUT;
        }
    };
    let cancel = CancellationToken::new();

    // 既有 analyzer（不重算；M21 §34）。
    let graph = match DependencyService::new().inspect_graph(&project.root_path, &cancel) {
        Ok(graph) => graph,
        Err(err) => {
            eprintln!("prism: intel failed building dependency graph: {err}");
            return if matches!(
                err.code(),
                prism_core::PrismErrorCode::InvalidInput | prism_core::PrismErrorCode::NotFound
            ) {
                crate::EXIT_INVALID_INPUT
            } else {
                crate::EXIT_ERROR
            };
        }
    };

    let identity = prism_intel::ProjectIdentity::from_project(&project);
    let context = AnalysisContext::new(
        identity,
        AnalysisScope::Domain("dependency".to_string()),
        vec!["dependency_graph".to_string()],
        cancel.clone(),
    );
    let outcome = prism_intel::translate::dependencies_to_intel(&graph, &context);

    match &outcome.status {
        prism_intel::IntelStatus::Failed => {
            for error in &outcome.errors {
                eprintln!("prism: intel: {error}");
            }
            return crate::EXIT_ERROR;
        }
        prism_intel::IntelStatus::Cancelled => return crate::EXIT_CANCELLED,
        _ => {}
    }

    let bundle = outcome
        .data
        .as_ref()
        .expect("non-failed outcome carries bundle data");
    if json {
        match serde_json::to_string_pretty(&outcome) {
            Ok(text) => println!("{text}"),
            Err(err) => {
                eprintln!("prism: failed to serialize intel bundle: {err}");
                return crate::EXIT_ERROR;
            }
        }
    } else {
        println!("ifuyo-prism");
        println!("Intelligence Core (2.0-M21; dependency domain)");
        println!();
        println!(
            "Project: {} ({})",
            bundle.project.display_name, bundle.project.stable_key
        );
        println!(
            "Schema: v{} | Status: {:?}",
            bundle.schema_version, outcome.status
        );
        println!();
        println!("Core");
        println!("  Entities:  {}", bundle.entities.len());
        println!("  Relations: {}", bundle.relations.len());
        println!("  Facts:     {}", bundle.facts.len());
        println!("  Evidence:  {}", bundle.evidence.len());
        println!("  Findings:  {}", bundle.findings.len());
        for fact in bundle.facts.values() {
            println!("  fact {}: {:?}", fact.fact_type, fact.value);
        }
        if !outcome.warnings.is_empty() {
            println!();
            println!("Diagnostics (project issues, not analysis errors):");
            for warning in &outcome.warnings {
                println!("  {warning}");
            }
        }
    }
    crate::EXIT_OK
}
