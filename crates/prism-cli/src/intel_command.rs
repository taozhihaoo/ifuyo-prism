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
    let mut semantic_relations: Vec<prism_csharp::SemanticRelation> = Vec::new();

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
        IntelDomain::Structural
            | IntelDomain::Symbols
            | IntelDomain::Semantic
            | IntelDomain::Impact
            | IntelDomain::Godot
            | IntelDomain::All
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
    if matches!(
        domain,
        IntelDomain::Symbols
            | IntelDomain::Semantic
            | IntelDomain::Impact
            | IntelDomain::Godot
            | IntelDomain::All
    ) {
        let Some(l0) = l0_bundle
            .clone()
            .or_else(|| merged.as_ref().and_then(|outcome| outcome.data.clone()))
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
        let mut sources: Vec<(String, String)> = Vec::new();
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
            sources.push((relative_path.clone(), source));
        }
        let index = engine.build_index();
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
        // godot 域只需要符号**索引**（backed_by 查询），不吸收 symbols bundle。
        // impact 域同理——只需要 L2 关系图（§七 Evidence Graph 复用）。
        if matches!(domain, IntelDomain::Symbols) {
            absorb(
                &mut merged,
                &mut domain_label,
                outcome,
                "symbols",
                "dependency+l0_structural+symbols",
            );
        }
        symbol_index = Some(index);
    }

    // ---- M25 L2 语义域（消费 L0 清单 + M23 符号索引；§45 垂直切片）----
    // Impact 域同样依赖 L2 关系图（§七 Evidence Graph 复用 M25 产物）。
    if matches!(
        domain,
        IntelDomain::Semantic | IntelDomain::Impact | IntelDomain::All
    ) {
        let Some(l0) = l0_bundle
            .clone()
            .or_else(|| merged.as_ref().and_then(|outcome| outcome.data.clone()))
        else {
            eprintln!("prism: intel semantic domain requires the structural inventory");
            return crate::EXIT_ERROR;
        };
        let csharp_files = collect_csharp_files(&l0);
        let mut sources: Vec<(String, String)> = Vec::new();
        for (relative_path, absolute) in &csharp_files {
            let Ok(metadata) = std::fs::metadata(absolute) else {
                continue;
            };
            if metadata.len() > MAX_SOURCE_BYTES {
                continue;
            }
            let Ok(source) = std::fs::read_to_string(absolute) else {
                continue;
            };
            sources.push((relative_path.clone(), source));
        }
        // M23 索引一次建全（§67：无每文件重建 Compilation 式浪费）。
        let mut engine = prism_csharp::SymbolEngine::new();
        for (relative_path, source) in &sources {
            engine.update_file(relative_path, source);
        }
        let index = engine.build_index();
        let mut semantic = prism_csharp::SemanticEngine::new();
        for (relative_path, source) in &sources {
            semantic.update_source(&index, relative_path, source);
        }
        semantic.rebuild(&index);
        if std::env::var("PRISM_DBG_SEM").is_ok() {
            eprintln!(
                "[sem] files={} rels={}",
                sources.len(),
                semantic.relations().count()
            );
            for relation in semantic.relations() {
                eprintln!(
                    "[sem-rel] {:?} {} -> {:?}",
                    relation.kind, relation.source_symbol_id, relation.target
                );
            }
            for extraction in engine.extraction_map().values() {
                eprintln!(
                    "[sem-ext] {} symbols={} usings={:?}",
                    extraction.file,
                    extraction.symbols.len(),
                    extraction.usings
                );
                for symbol in &extraction.symbols {
                    eprintln!(
                        "[sym] {} {} params={}",
                        symbol.qualified_name,
                        symbol.kind.as_str(),
                        symbol.parameters.len()
                    );
                }
            }
        }
        // Impact 域复用 L2 关系图（§七 Evidence Graph 复用 M21/M25）。
        semantic_relations = semantic.relations().cloned().collect::<Vec<_>>();
        // 查询输出（§52 callers/callees/references）。
        if let Some(needle) = search {
            let hits: Vec<_> = semantic
                .relations()
                .filter(|relation| {
                    relation
                        .source_symbol_id
                        .to_lowercase()
                        .contains(&needle.to_lowercase())
                        || matches!(
                            &relation.target,
                            prism_csharp::SemanticTarget::Resolved { qualified_name, .. }
                                if qualified_name.to_lowercase().contains(&needle.to_lowercase())
                        )
                        || matches!(
                            &relation.target,
                            prism_csharp::SemanticTarget::Unresolved { name, .. }
                                | prism_csharp::SemanticTarget::External { name, .. }
                                    if name.to_lowercase().contains(&needle.to_lowercase())
                        )
                })
                .collect();
            if json {
                match serde_json::to_string_pretty(&hits) {
                    Ok(text) => println!("{text}"),
                    Err(err) => {
                        eprintln!("prism: failed to serialize semantic relations: {err}");
                        return crate::EXIT_ERROR;
                    }
                }
            } else {
                println!("Semantic relations matching \"{}\": {}", needle, hits.len());
                for relation in hits {
                    let target = match &relation.target {
                        prism_csharp::SemanticTarget::Resolved { qualified_name, .. } => {
                            qualified_name.clone()
                        }
                        prism_csharp::SemanticTarget::Unresolved { name, .. }
                        | prism_csharp::SemanticTarget::External { name, .. }
                        | prism_csharp::SemanticTarget::Ambiguous { name, .. } => name.clone(),
                        prism_csharp::SemanticTarget::Unknown { reason } => reason.clone(),
                    };
                    println!(
                        "  {:?} {} -> {} [{:?}/{:?}] @ {}:{}",
                        relation.kind,
                        relation.source_symbol_id,
                        target,
                        relation.resolution,
                        relation.dispatch,
                        relation.file,
                        relation.range.start_line
                    );
                }
            }
            return crate::EXIT_OK;
        }
        let outcome = prism_csharp::intel::semantic_to_intel(
            semantic.relations().cloned().collect::<Vec<_>>().as_slice(),
            semantic
                .diagnostics()
                .cloned()
                .collect::<Vec<_>>()
                .as_slice(),
            &identity,
            &prism_core::Timestamp::now_utc().to_string(),
        );
        // impact 域只复用 L2 关系（§七），不吸收 semantic bundle。
        if matches!(domain, IntelDomain::Semantic) {
            absorb(
                &mut merged,
                &mut domain_label,
                outcome,
                "semantic",
                "dependency+l0_structural+symbols+semantic+godot",
            );
        }
    }

    // ---- M24 Godot 域（消费 L0 清单 + M23 符号索引；§61-§63）----
    if matches!(domain, IntelDomain::Godot | IntelDomain::All) {
        let Some(l0) = l0_bundle
            .take()
            .or_else(|| merged.as_ref().and_then(|outcome| outcome.data.clone()))
        else {
            eprintln!("prism: intel godot domain requires the structural inventory");
            return crate::EXIT_ERROR;
        };
        let inventory: Vec<(String, String)> = l0
            .entities
            .values()
            .filter_map(|entity| match &entity.location {
                prism_intel::EntityLocation::FileOnly { path } if !path.is_empty() => {
                    Some((path.clone(), String::new()))
                }
                _ => None,
            })
            .collect();
        let godot_context = AnalysisContext::new(
            identity.clone(),
            AnalysisScope::Domain("godot".to_string()),
            vec!["godot".to_string()],
            cancel.clone(),
        );
        let outcome = prism_godot::analyze(&inventory, &project.root_path, &godot_context);
        if guard_status(&outcome) {
            return exit_for(&outcome);
        }
        warnings.extend(outcome.warnings.clone());
        let analysis = outcome.data.unwrap();
        if !analysis.is_godot_project() {
            warnings.push(
                "no project.godot found; godot domain empty (not a Godot project)".to_string(),
            );
        }
        let godot_index = prism_godot::GodotIndex::build(&analysis);
        let outcome = prism_godot::godot_to_intel(
            &analysis,
            &godot_index,
            symbol_index.as_ref(),
            &identity,
            &prism_core::Timestamp::now_utc().to_string(),
        );
        absorb(
            &mut merged,
            &mut domain_label,
            outcome,
            "godot",
            "dependency+l0_structural+symbols+godot",
        );
    }

    // ---- M26 Impact 域（Evidence Graph 反向遍历；§33-§41）----
    if matches!(domain, IntelDomain::Impact) {
        let mut graph = prism_intel::EvidenceGraph::new();
        for relation in &semantic_relations {
            let target = match &relation.target {
                prism_csharp::SemanticTarget::Resolved { symbol_id, .. } => symbol_id.clone(),
                prism_csharp::SemanticTarget::Unresolved { name, .. }
                | prism_csharp::SemanticTarget::External { name, .. }
                | prism_csharp::SemanticTarget::Ambiguous { name, .. } => {
                    format!("unresolved:{name}")
                }
                prism_csharp::SemanticTarget::Unknown { reason } => {
                    format!("unknown:{reason}")
                }
            };
            graph.push(prism_intel::GraphEdge {
                source: relation.source_symbol_id.clone(),
                target,
                relation: relation.kind.as_str().to_string(),
                file: relation.file.clone(),
                line: relation.range.start_line,
                resolution: Some(format!("{:?}", relation.resolution).to_lowercase()),
            });
        }
        // 变更源（§10/§43）：Working Tree——git status 逐文件映射符号。
        let runner: std::sync::Arc<dyn prism_core::runtime::command::CommandRunner> =
            std::sync::Arc::new(prism_core::runtime::command::StdCommandRunner::new());
        let git = prism_git::GitService::new(runner);
        let status = match git.status(&project.root_path, false, &cancel) {
            Ok(status) => status,
            Err(err) => {
                eprintln!("prism: impact git status failed: {err}");
                return crate::EXIT_ERROR;
            }
        };
        let mut entity_changes: Vec<prism_intel::EntityChange> = Vec::new();
        for entry in &status.entries {
            if entry.is_ignored || !entry.path.ends_with(".cs") {
                continue;
            }
            let removed = format!("{:?}", entry.working_tree_state).to_lowercase() == "deleted";
            let old_source = git
                .read_blob(
                    &project.root_path,
                    "HEAD",
                    &entry.path,
                    4 * 1_048_576,
                    &cancel,
                )
                .ok()
                .and_then(|blob| match blob {
                    prism_git::RevisionBlob::Text { content } => Some(content),
                    // TooLarge/EncodingUnsupported/Missing ⇒ 无法做符号级对比
                    //（§26 诚实降级：文件级变更仍进入，符号级跳过）。
                    _ => None,
                });
            let new_source = std::fs::read_to_string(project.root_path.join(&entry.path)).ok();
            let old_extraction = old_source
                .as_deref()
                .map(|c| prism_csharp::extract::extract(&entry.path, c));
            let new_extraction = new_source
                .as_deref()
                .map(|c| prism_csharp::extract::extract(&entry.path, c));
            for changed in
                prism_csharp::map_file_change(old_extraction.as_ref(), new_extraction.as_ref())
            {
                // 删除的文件 ⇒ 全部 ContractRemoved；修改/新增 ⇒ 按 mapping 判定。
                let change_type = if removed {
                    prism_intel::ImpactChangeType::ContractRemoved
                } else {
                    changed.change_type
                };
                entity_changes.push(prism_intel::EntityChange {
                    entity_id: changed.symbol_id.clone(),
                    change_type,
                });
                // §26 ContractRemoved 名字面种子：unresolved 调用点以
                // `unresolved:{成员名}` 为伪目标——移除同名成员时这些
                // 调用点 Potential/Low（名字面 = 候选级证据 §63）。
                if change_type == prism_intel::ImpactChangeType::ContractRemoved {
                    let member = changed
                        .qualified_name
                        .rsplit('.')
                        .next()
                        .unwrap_or(changed.qualified_name.as_str());
                    entity_changes.push(prism_intel::EntityChange {
                        entity_id: format!("unresolved:{member}"),
                        change_type,
                    });
                }
            }
        }
        if entity_changes.is_empty() {
            warnings.push("no working-tree symbol changes detected; impact empty".to_string());
        }
        let policy = prism_intel::ImpactPolicy::default();
        eprintln!(
            "[impact] graph_edges={} changes={} rels={}",
            graph.edge_count(),
            entity_changes.len(),
            semantic_relations.len()
        );
        for edge in &graph.edges {
            eprintln!("[impact-edge] src={} tgt={}", edge.source, edge.target);
        }
        for change in &entity_changes {
            eprintln!("[impact-change] id={}", change.entity_id);
        }
        let result = prism_intel::analyze_impact(&graph, &entity_changes, &policy);

        // 输出（§39 报告形态）。
        if json {
            match serde_json::to_string_pretty(&serde_json::json!({
                "changed": entity_changes,
                "affected": result.affected,
                "metadata": result.metadata,
            })) {
                Ok(text) => println!("{text}"),
                Err(err) => {
                    eprintln!("prism: failed to serialize impact result: {err}");
                    return crate::EXIT_ERROR;
                }
            }
        } else {
            println!("Impact (working tree, {} change(s))", entity_changes.len());
            println!();
            let direct: Vec<_> = result
                .affected
                .iter()
                .filter(|a| a.impact == prism_intel::ImpactType::Direct)
                .collect();
            let transitive: Vec<_> = result
                .affected
                .iter()
                .filter(|a| a.impact == prism_intel::ImpactType::Transitive)
                .collect();
            println!("Direct: {}", direct.len());
            for entity in &direct {
                println!("  {} [{:?}]", entity.entity_id, entity.category);
            }
            println!("Transitive: {}", transitive.len());
            for entity in transitive.iter().take(10) {
                println!(
                    "  {} ({})",
                    entity.entity_id,
                    entity.primary_path.join(" -> ")
                );
            }
            if result.metadata.cycles_detected > 0 {
                println!("Cycles detected: {}", result.metadata.cycles_detected);
            }
            if result.metadata.truncated_by_entity_limit > 0 {
                println!("Truncated at entity limit");
            }
        }
        return crate::EXIT_OK;
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
