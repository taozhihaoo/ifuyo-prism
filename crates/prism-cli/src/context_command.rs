//! `prism context <path>` — 确定性 Context 生成基础命令（规格 M10a §283/§347/§348）。
//!
//! stdout 只输出数据（§347）；M1–M6 事实由本命令现场构建（无 session 时
//! M7–M9 事实缺省为 None——collectors 如实降级，§260）；GUI/CLI 同源
//! ContextService（§475）。

use prism_analysis::{CodeAnalysisOptions, CodeAnalysisService};
use prism_changes::{ChangeScope, ChangeSetOptions, ChangeSetService};
use prism_context::model::{ContextBudget, ContextRequest, ContextScope};
use prism_core::runtime::command::{CommandRunner, StdCommandRunner};
use prism_core::task::CancellationToken;
use prism_graph::DependencyService;
use std::path::Path;
use std::sync::Arc;

#[allow(clippy::too_many_arguments)]
pub fn run(
    path: &Path,
    task: Option<String>,
    json: bool,
    scope: Option<String>,
    target: Option<String>,
    max_tokens: Option<u64>,
    markdown: bool,
    dry_run: bool,
    explain: bool,
    check_health: bool,
    profile: Option<String>,
    max_items: Option<usize>,
    max_bytes: Option<u64>,
    impact_aware: bool,
) -> i32 {
    if let Some(value) = profile.as_deref() {
        if prism_context::ContextRecipe::parse(value).is_none() {
            eprintln!("prism: unknown profile '{value}' (auto|review|debug)");
            return crate::EXIT_INVALID_INPUT;
        }
    }
    if !path.exists() {
        eprintln!("prism: path does not exist: {}", path.to_string_lossy());
        return crate::EXIT_INVALID_INPUT;
    }
    let runner: Arc<dyn CommandRunner> = Arc::new(StdCommandRunner::new());
    let cancel = CancellationToken::new();

    // ---- M1/M2/M3 现场事实（§263：调用方负责提供；CLI 无 session）----
    let discovery = match prism_project::DiscoveryService::new().discover(
        path,
        prism_project::ScanOptions::default(),
        &cancel,
        |_| {},
    ) {
        Ok(discovery) => discovery,
        Err(err) => {
            eprintln!("prism: context failed building discovery: {err}");
            return crate::EXIT_ERROR;
        }
    };
    let code_model = match CodeAnalysisService::new().analyze(
        &discovery,
        CodeAnalysisOptions::default(),
        &cancel,
        |_| {},
    ) {
        Ok(model) => model,
        Err(err) => {
            eprintln!("prism: context failed building code model: {err}");
            return crate::EXIT_ERROR;
        }
    };
    let graph = DependencyService::new()
        .analyze(&discovery, &code_model)
        .ok();
    // M5/M6（git 仓库才有；失败 ⇒ None，collectors 降级 §260）。
    let changeset = ChangeSetService::new(runner.clone())
        .build(
            path,
            ChangeScope::WorkingTree,
            None,
            None,
            &ChangeSetOptions::default(),
            &cancel,
        )
        .ok()
        .flatten();
    let impact = changeset
        .as_ref()
        .zip(graph.as_ref())
        .and_then(|(changeset, graph)| {
            prism_impact::ImpactAnalysisService
                .analyze(
                    changeset,
                    &code_model,
                    graph,
                    &prism_impact::ImpactOptions::default(),
                    &cancel,
                )
                .ok()
        });

    let parsed_scope = match scope.as_deref() {
        None => ContextScope::Auto,
        Some("auto") => ContextScope::Auto,
        Some("current_change") | Some("current-change") => ContextScope::CurrentChange,
        Some("target") => ContextScope::Target,
        Some("full") => ContextScope::Full,
        Some(other) => {
            eprintln!("prism: unknown scope '{other}' (auto|current_change|target|full)");
            return crate::EXIT_INVALID_INPUT;
        }
    };

    let mut budget = ContextBudget::default();
    if let Some(max_tokens) = max_tokens {
        budget.max_tokens_estimate = max_tokens;
    }
    if let Some(max_items) = max_items {
        budget.max_items = max_items;
    }
    if let Some(max_bytes) = max_bytes {
        budget.max_bytes = max_bytes;
    }

    let request = ContextRequest {
        project_root: path.to_string_lossy().to_string(),
        task,
        scope: parsed_scope,
        target,
        budget,
    };

    // ---- M27 Impact/Change-aware（§19/§20）：--impact-aware 时组装
    // M26 Evidence 输入（工作区变更 + Evidence 关系）。None ⇒ 与 1.0 等价。
    let (evidence_relations, changed_entities) = if impact_aware {
        build_impact_evidence(path, &discovery, &cancel)
    } else {
        (Vec::new(), std::collections::BTreeSet::new())
    };

    let pack = match prism_context::ContextService::generate_with_evidence(
        request,
        Some(&discovery),
        Some(&code_model),
        changeset.as_ref(),
        impact.as_ref(),
        graph.as_ref(),
        None, // M7/M8/M9 session 事实 CLI 未持有 ⇒ None（collectors 降级）
        None,
        None,
        Some(&evidence_relations),
        Some(&changed_entities),
        &cancel,
    ) {
        Ok(pack) => pack,
        Err(prism_core::PrismError::InvalidInput { message }) => {
            // §200：零预算等请求错误 ⇒ exit 3。
            eprintln!("prism: {message}");
            return crate::EXIT_INVALID_INPUT;
        }
        Err(err) => {
            eprintln!("prism: context generation failed: {err}");
            return crate::EXIT_ERROR;
        }
    };

    // §284：--check-health 只输出 Context Health。
    if check_health {
        let value = serde_json::json!({
            "status": pack.health.status,
            "findings": pack.health.findings,
            "coverage": pack.health.coverage,
            "risk_signals": pack.risk_signals,
        });
        match serde_json::to_string_pretty(&value) {
            Ok(text) => println!("{text}"),
            Err(err) => {
                eprintln!("prism: failed to serialize context health: {err}");
                return crate::EXIT_ERROR;
            }
        }
        return crate::EXIT_OK;
    }

    // §285/§350：dry-run 只输出 selection plan（不物化内容——pack 的
    // content 在 dry-run 分支不打印）。
    if dry_run {
        println!(
            "Selection plan: {} selected · {} excluded · est tokens {}/{}",
            manifest_selected(&pack),
            pack.selection.excluded.len(),
            pack.manifest.budget.used_tokens_estimate,
            pack.manifest.budget.limit_tokens_estimate
        );
        for (index, item) in pack.items.iter().enumerate() {
            println!(
                "{}. [{:>9}] {} — why: {}",
                index + 1,
                item.priority.as_str(),
                item.title,
                item.reasons
                    .iter()
                    .map(|reason| reason.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
        for excluded in &pack.selection.excluded {
            println!(
                "excluded: {} ({})",
                excluded.title,
                excluded.reason.as_str()
            );
        }
        return crate::EXIT_OK;
    }

    // §287：explain = selection trace（含排除理由）。
    if explain {
        println!(
            "Selection trace: {} candidates → {} selected, {} excluded",
            pack.selection.candidates,
            pack.selection.selected.len(),
            pack.selection.excluded.len()
        );
        for excluded in &pack.selection.excluded {
            println!(
                "  EXCLUDED [{:?}] {} — {}",
                excluded.reason, excluded.title, excluded.detail
            );
        }
        return crate::EXIT_OK;
    }

    if json {
        match serde_json::to_string_pretty(&pack) {
            Ok(text) => println!("{text}"),
            Err(err) => {
                eprintln!("prism: failed to serialize context pack: {err}");
                return crate::EXIT_ERROR;
            }
        }
    } else if markdown {
        print!("{}", prism_context::render_markdown(&pack));
    } else {
        print!("{}", prism_context::render_text(&pack));
    }
    crate::EXIT_OK
}

fn manifest_selected(pack: &prism_context::ContextPack) -> usize {
    pack.items.len()
}

/// M27 Impact/Change-aware 输入组装（§19/§20）：工作区 git 变更 ⇒ 变更
/// 符号集；M23/M25 引擎跑全项目 .cs ⇒ Evidence Graph 关系；取 **target ∈
/// 变更集** 的边。全部复用既有引擎（§79 消费者不自行 BFS/解析）。
fn build_impact_evidence(
    root: &Path,
    discovery: &prism_project::ProjectDiscovery,
    cancel: &CancellationToken,
) -> (
    Vec<prism_intel::GraphEdge>,
    std::collections::BTreeSet<String>,
) {
    let mut changed_files: Vec<String> = Vec::new();
    let runner: Arc<dyn prism_core::runtime::command::CommandRunner> =
        Arc::new(StdCommandRunner::new());
    let git = prism_git::GitService::new(runner.clone());
    let Ok(status) = git.status(root, false, cancel) else {
        return (Vec::new(), std::collections::BTreeSet::new());
    };
    for entry in &status.entries {
        if entry.is_ignored {
            continue;
        }
        if entry.path.ends_with(".cs") && !changed_files.contains(&entry.path) {
            changed_files.push(entry.path.clone());
        }
    }
    if changed_files.is_empty() {
        return (Vec::new(), std::collections::BTreeSet::new());
    }

    // 全项目 .cs（M1 discovery 语言标注 + 变更清单兜底）。
    let mut all_cs: Vec<String> = discovery
        .files
        .iter()
        .filter(|f| {
            f.extension
                .as_deref()
                .map(|e| e.eq_ignore_ascii_case("cs"))
                .unwrap_or(false)
        })
        .map(|f| f.relative_path.clone())
        .collect();
    for path in &changed_files {
        if !all_cs.contains(path) {
            all_cs.push(path.clone());
        }
    }
    all_cs.sort();

    let mut engine = prism_csharp::SymbolEngine::new();
    let mut semantic = prism_csharp::SemanticEngine::new();
    for relative_path in &all_cs {
        let absolute = root.join(relative_path);
        let Ok(metadata) = std::fs::metadata(&absolute) else {
            continue;
        };
        if metadata.len() > 4 * 1_048_576 {
            continue;
        }
        let Ok(source) = std::fs::read_to_string(&absolute) else {
            continue;
        };
        engine.update_file(relative_path, &source);
        semantic.update_source(&engine.build_index(), relative_path, &source);
    }
    semantic.rebuild(&engine.build_index());

    // 变更符号集：HEAD 内容 vs 工作区内容的抽取 diff（symbol_id 键）。
    let mut changed: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for relative_path in &changed_files {
        let old_source = git
            .read_blob(root, "HEAD", relative_path, 4 * 1_048_576, cancel)
            .ok()
            .and_then(|blob| match blob {
                prism_git::RevisionBlob::Text { content } => Some(content),
                _ => None,
            });
        let new_source = std::fs::read_to_string(root.join(relative_path)).ok();
        let old_extraction = old_source
            .as_deref()
            .map(|c| prism_csharp::extract::extract(relative_path, c));
        let new_extraction = new_source
            .as_deref()
            .map(|c| prism_csharp::extract::extract(relative_path, c));
        for changed_symbol in
            prism_csharp::map_file_change(old_extraction.as_ref(), new_extraction.as_ref())
        {
            changed.insert(changed_symbol.symbol_id);
        }
    }

    // Evidence 边：target ∈ 变更集的关系（B changed ⇒ source 受影响）。
    let mut edges = Vec::new();
    for relation in semantic.relations() {
        let prism_csharp::SemanticTarget::Resolved { symbol_id, .. } = &relation.target else {
            continue;
        };
        if !changed.contains(symbol_id) {
            continue;
        }
        edges.push(prism_intel::GraphEdge {
            source: relation.source_symbol_id.clone(),
            target: symbol_id.clone(),
            relation: relation.kind.as_str().to_string(),
            file: relation.file.clone(),
            line: relation.range.start_line,
            resolution: Some(format!("{:?}", relation.resolution).to_lowercase()),
        });
    }
    (edges, changed)
}
