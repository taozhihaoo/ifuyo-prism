//! `prism intel <path>` — Intelligence Core 消费面：
//! M21 依赖域切片 + M22 L0 结构域（`--domain`），三面同源的 CLI 面。
//!
//! 旧 `prism graph` / `prism scan` JSON 契约保持不变（M21 §32 additive first）。

use crate::cli::IntelDomain;
use prism_core::task::CancellationToken;
use prism_graph::DependencyService;
use prism_intel::{AnalysisContext, AnalysisScope, IntelBundle, IntelOutcome, ProjectIdentity};
use prism_project::Project;
use std::path::Path;

/// `prism intel <path> [--domain dependency|structural|all] [--json]`。
pub fn run(path: &Path, domain: IntelDomain, json: bool) -> i32 {
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
    let identity = ProjectIdentity::from_project(&project);

    let mut warnings: Vec<String> = Vec::new();
    let mut merged: Option<IntelOutcome<IntelBundle>> = None;
    let mut domain_label = "";

    // ---- M21 依赖域 ----
    if matches!(domain, IntelDomain::Dependency | IntelDomain::All) {
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
        let context = AnalysisContext::new(
            identity.clone(),
            AnalysisScope::Domain("dependency".to_string()),
            vec!["dependency_graph".to_string()],
            cancel.clone(),
        );
        let outcome = prism_intel::translate::dependencies_to_intel(&graph, &context);
        if guard_status(&outcome) {
            return exit_for(&outcome);
        }
        warnings.extend(outcome.warnings.clone());
        match &mut merged {
            Some(existing) => {
                if let Some(bundle) = outcome.data {
                    existing.data.as_mut().unwrap().merge(bundle);
                }
                domain_label = "dependency+l0_structural";
            }
            None => {
                merged = Some(outcome);
                domain_label = "dependency";
            }
        }
    }

    // ---- M22 L0 结构域 ----
    if matches!(domain, IntelDomain::Structural | IntelDomain::All) {
        let context = AnalysisContext::new(
            identity.clone(),
            AnalysisScope::Domain("l0_structural".to_string()),
            vec!["l0_structural".to_string()],
            cancel.clone(),
        );
        let outcome =
            prism_l0::structural_analysis(&project.root_path, &context, &Default::default());
        if guard_status(&outcome) {
            return exit_for(&outcome);
        }
        warnings.extend(outcome.warnings.clone());
        match &mut merged {
            Some(existing) => {
                if let Some(bundle) = outcome.data {
                    existing.data.as_mut().unwrap().merge(bundle);
                }
                domain_label = "dependency+l0_structural";
            }
            None => {
                merged = Some(outcome);
                domain_label = "l0_structural";
            }
        }
    }

    let outcome = merged.expect("at least one domain selected");
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
        println!("Intelligence Core (domains: {domain_label})");
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
        const MAX_FACT_LINES: usize = 10;
        for fact in bundle.facts.values().take(MAX_FACT_LINES) {
            println!("  fact {}: {:?}", fact.fact_type, fact.value);
        }
        if bundle.facts.len() > MAX_FACT_LINES {
            println!(
                "  …and {} more facts (--json for full bundle)",
                bundle.facts.len() - MAX_FACT_LINES
            );
        }
        if !warnings.is_empty() {
            println!();
            println!("Diagnostics (project issues, not analysis errors):");
            for warning in &warnings {
                println!("  {warning}");
            }
        }
    }
    crate::EXIT_OK
}

/// Failed/Cancelled 的统一提前返回（打印已由调用方处理数据面）。
fn guard_status<T>(outcome: &IntelOutcome<T>) -> bool {
    matches!(
        outcome.status,
        prism_intel::IntelStatus::Failed | prism_intel::IntelStatus::Cancelled
    )
}

fn exit_for<T>(outcome: &IntelOutcome<T>) -> i32 {
    match outcome.status {
        prism_intel::IntelStatus::Failed => {
            for error in &outcome.errors {
                eprintln!("prism: intel: {error}");
            }
            crate::EXIT_ERROR
        }
        _ => crate::EXIT_CANCELLED,
    }
}
