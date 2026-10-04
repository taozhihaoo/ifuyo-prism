//! `prism intel <path>` — Intelligence Core 消费面：
//! M21 依赖域 + M22 L0 结构域 + M23 C# 符号域（`--domain`），三面同源的 CLI 面。
//!
//! 旧 `prism graph` / `prism scan` JSON 契约保持不变（M21 §32 additive first）。

use crate::cli::IntelDomain;
use prism_core::task::CancellationToken;
use prism_graph::DependencyService;
use prism_intel::{AnalysisContext, AnalysisScope, IntelBundle, IntelOutcome, ProjectIdentity};
use prism_project::Project;
use std::path::Path;

/// 单文件读取上限（§45 资源限额；与 M22 manifest 上限同量级）。
const MAX_SOURCE_BYTES: u64 = 4 * 1_048_576;

/// `prism intel <path> [--domain dependency|structural|symbols|all]
/// [--search name] [--json]`。
pub fn run(path: &Path, domain: IntelDomain, search: Option<&str>, json: bool) -> i32 {
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
        absorb(
            &mut merged,
            &mut domain_label,
            outcome,
            "dependency",
            "dependency+l0_structural+symbols",
        );
    }

    // ---- M22 L0 结构域 ----
    // symbols 域也依赖 L0 的文件清单 + 语言标注（§三十四：不重扫文件系统）。
    let l0_needed = matches!(
        domain,
        IntelDomain::Structural | IntelDomain::Symbols | IntelDomain::All
    );
    let mut l0_bundle: Option<IntelBundle> = None;
    if l0_needed {
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
        if matches!(domain, IntelDomain::Structural | IntelDomain::All) {
            absorb(
                &mut merged,
                &mut domain_label,
                outcome,
                "l0_structural",
                "dependency+l0_structural+symbols",
            );
        } else {
            l0_bundle = outcome.data;
        }
    }

    // ---- M23 C# 符号域（消费 M22 文件清单；§三十四）----
    let mut symbol_index: Option<prism_csharp::SymbolIndex> = None;
    if matches!(domain, IntelDomain::Symbols | IntelDomain::All) {
        let Some(l0) =
            l0_bundle.or_else(|| merged.as_ref().and_then(|outcome| outcome.data.clone()))
        else {
            eprintln!("prism: intel symbols domain requires the structural inventory");
            return crate::EXIT_ERROR;
        };
        let csharp_files = collect_csharp_files(&l0);
        if csharp_files.is_empty() {
            warnings.push("no C# files found in inventory; symbols domain empty".to_string());
        }
        let mut engine = prism_csharp::SymbolEngine::new();
        let mut parse_error_files = 0usize;
        for (relative_path, absolute) in &csharp_files {
            let Ok(metadata) = std::fs::metadata(absolute) else {
                continue;
            };
            if metadata.len() > MAX_SOURCE_BYTES {
                warnings.push(format!("source {relative_path} exceeds size cap; skipped"));
                continue;
            }
            let Ok(source) = std::fs::read_to_string(absolute) else {
                warnings.push(format!("source {relative_path} unreadable; skipped"));
                continue;
            };
            let had_errors = {
                let (_tree, has_error) = prism_csharp::extract::parse(&source);
                has_error
            };
            if had_errors {
                parse_error_files += 1;
            }
            engine.update_file(relative_path, &source);
        }
        let index = engine.build_index();
        if let Some(first_error) = csharp_files.first() {
            let _ = first_error;
        }
        let outcome = prism_csharp::intel::symbols_to_intel(
            &index,
            &identity,
            &prism_core::Timestamp::now_utc().to_string(),
            parse_error_files,
        );
        // 搜索输出（§35 symbol search；只查声明，非全文搜索）。
        if let Some(needle) = search {
            let hits = index.search(needle, None);
            if json {
                match serde_json::to_string_pretty(&hits) {
                    Ok(text) => println!("{text}"),
                    Err(err) => {
                        eprintln!("prism: failed to serialize search results: {err}");
                        return crate::EXIT_ERROR;
                    }
                }
            } else {
                println!("Symbol search \"{}\": {} hit(s)", needle, hits.len());
                for symbol in hits {
                    let first = symbol.declarations.first();
                    let location = first
                        .map(|d| format!("{}:{}", d.file, d.range.start_line))
                        .unwrap_or_default();
                    println!("  {:?} {} @ {}", symbol.kind, symbol.display_name, location);
                }
            }
            return crate::EXIT_OK;
        }
        absorb(
            &mut merged,
            &mut domain_label,
            outcome,
            "symbols",
            "dependency+l0_structural+symbols",
        );
        symbol_index = Some(index);
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
        if let Some(index) = symbol_index.as_ref().filter(|i| !i.by_id.is_empty()) {
            println!();
            println!("Symbols (top-level sample; --search <name> to query):");
            let mut shown = 0;
            for symbol in index.by_id.values() {
                if symbol.parent_id.is_some() || shown >= 10 {
                    continue;
                }
                let location = symbol
                    .declarations
                    .first()
                    .map(|d| format!("{}:{}", d.file, d.range.start_line))
                    .unwrap_or_default();
                println!("  {:?} {} @ {}", symbol.kind, symbol.display_name, location);
                shown += 1;
            }
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

/// 从 L0 bundle 收集 C# 文件（language metadata basis=extension，§三十四）。
fn collect_csharp_files(l0: &IntelBundle) -> Vec<(String, String)> {
    let root = l0.project.root.clone();
    let mut out = Vec::new();
    for entity in l0.entities.values() {
        if entity.kind != prism_intel::EntityKind::file() {
            continue;
        }
        let is_csharp = entity
            .metadata
            .get("language")
            .map(|value| value == "csharp")
            .unwrap_or(false);
        if !is_csharp {
            continue;
        }
        // 路径在 location（M22 口径），不在 metadata。
        let prism_intel::EntityLocation::FileOnly { path } = &entity.location else {
            continue;
        };
        if path.is_empty() {
            continue;
        }
        let absolute = format!("{}/{}", root.trim_end_matches(['/', '\\']), path);
        out.push((path.clone(), absolute.clone()));
    }
    out
}

fn absorb(
    merged: &mut Option<IntelOutcome<IntelBundle>>,
    domain_label: &mut &'static str,
    outcome: IntelOutcome<IntelBundle>,
    single: &'static str,
    combined: &'static str,
) {
    match merged {
        Some(existing) => {
            if let Some(bundle) = outcome.data {
                existing.data.as_mut().unwrap().merge(bundle);
            }
            *domain_label = combined;
        }
        None => {
            *merged = Some(outcome);
            *domain_label = single;
        }
    }
}

/// Failed/Cancelled 的统一提前返回。
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
