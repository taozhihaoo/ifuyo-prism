//! M6 遍历语义测试：合成 M3 图 + 合成 ChangeSet（规格 §92–§113）。
//!
//! 全部拓扑手工构造，方向契约来自 M3 真实语义：
//! `Dependency{source→target}` = source 依赖 target；`CallEdge{caller→callee}`。
//! 测试约定：qualified = "lib.foo" ⇒ 所在文件 = "lib.ts"（M2 路径根式）。

use prism_changes::{
    ChangeEvidence, ChangeScope, ChangeSet, ChangeSource, ChangeSummary, FileChange,
    FileChangeType, MappingStatus, MatchConfidence, SymbolChange, SymbolChangeType,
    SymbolCharacteristic,
};
use prism_core::ids::{ModuleId, SymbolId};
use prism_core::task::CancellationToken;
use prism_graph::{
    CallEdge, Dependency, DependencyGraphResult, DependencyKind, GraphMetrics, Module, ModuleKind,
    ResolutionStatus,
};
use prism_impact::{
    build_seeds, traverse, ImpactConfidence, ImpactKind, ImpactOptions, ImpactReason, NodeKind,
};
use prism_parser::SymbolKind;

// ---------- 合成图构造 ----------

fn file_module(path: &str) -> Module {
    Module {
        id: ModuleId::from_string(format!("module:file:{path}")),
        name: path.to_string(),
        kind: ModuleKind::FileModule,
        file_ids: vec![],
        parent_id: None,
        language: None,
        path: Some(path.to_string()),
        evidence: vec![],
    }
}

/// source 依赖 target（M3 语义：import 方向）。
fn dependency(source: &str, target: &str) -> Dependency {
    Dependency {
        id: format!("dep:{source}#{target}#import"),
        source_id: ModuleId::from_string(format!("module:file:{source}")),
        source_path: source.to_string(),
        raw_target: target.to_string(),
        kind: DependencyKind::Import,
        resolution: ResolutionStatus::Resolved,
        confidence: prism_core::Confidence::High,
        target_id: Some(ModuleId::from_string(format!("module:file:{target}"))),
        target_path: Some(target.to_string()),
        imported_names: vec![],
        location: None,
        evidence: vec![],
    }
}

/// 未解析依赖（无 target_id；§102/§103）。
fn unresolved_dependency(source: &str, raw_target: &str) -> Dependency {
    Dependency {
        id: format!("dep:{source}#{raw_target}#import"),
        source_id: ModuleId::from_string(format!("module:file:{source}")),
        source_path: source.to_string(),
        raw_target: raw_target.to_string(),
        kind: DependencyKind::Import,
        resolution: ResolutionStatus::Unresolved,
        confidence: prism_core::Confidence::Low,
        target_id: None,
        target_path: None,
        imported_names: vec![],
        location: None,
        evidence: vec![],
    }
}

fn symbol_id(path: &str, qualified: &str) -> SymbolId {
    SymbolId::from_string(format!("symbol:{path}#{qualified}#function"))
}

/// qualified = "a.b" ⇒ 文件路径 "a.ts"（M2 路径根式测试约定）。
fn file_of(qualified: &str) -> String {
    let prefix = qualified
        .rsplit_once('.')
        .map(|(p, _)| p)
        .unwrap_or(qualified);
    format!("{prefix}.ts")
}

/// caller 调用 callee（callee None = 未解析，进名字索引）。
fn call(caller_qualified: &str, callee_qualified: &str, callee_resolved: bool) -> CallEdge {
    let caller_path = file_of(caller_qualified);
    let callee_name = callee_qualified
        .rsplit('.')
        .next()
        .unwrap_or(callee_qualified)
        .to_string();
    CallEdge {
        id: format!("call:{caller_qualified}#{callee_name}#1"),
        caller_id: symbol_id(&caller_path, caller_qualified),
        caller_qualified: caller_qualified.to_string(),
        callee_text: callee_name.clone(),
        callee_name,
        callee_id: callee_resolved.then(|| symbol_id(&file_of(callee_qualified), callee_qualified)),
        callee_qualified: Some(callee_qualified.to_string()),
        resolution: if callee_resolved {
            ResolutionStatus::Resolved
        } else {
            ResolutionStatus::Unresolved
        },
        confidence: prism_core::Confidence::High,
        location: None,
    }
}

fn graph(
    modules: Vec<Module>,
    deps: Vec<Dependency>,
    calls: Vec<CallEdge>,
) -> DependencyGraphResult {
    DependencyGraphResult {
        schema_version: 1,
        project_name: "test".to_string(),
        modules,
        dependencies: deps,
        nodes: vec![],
        edges: vec![],
        calls,
        cycles: vec![],
        metrics: GraphMetrics::default(),
        diagnostics: vec![],
    }
}

// ---------- 合成 ChangeSet 构造 ----------

fn symbol_change(path: &str, qualified: &str, change_type: SymbolChangeType) -> SymbolChange {
    SymbolChange {
        file_change_id: format!("fc:{path}"),
        path: path.to_string(),
        before_symbol_id: (change_type != SymbolChangeType::Added)
            .then(|| symbol_id(path, qualified)),
        after_symbol_id: (change_type != SymbolChangeType::Deleted)
            .then(|| symbol_id(path, qualified)),
        before_qualified_name: (change_type != SymbolChangeType::Added)
            .then(|| qualified.to_string()),
        after_qualified_name: (change_type != SymbolChangeType::Deleted)
            .then(|| qualified.to_string()),
        name: qualified
            .rsplit('.')
            .next()
            .unwrap_or(qualified)
            .to_string(),
        kind: SymbolKind::Function,
        change_type,
        characteristics: vec![SymbolCharacteristic::BodyChanged],
        location_before: None,
        location_after: None,
        changed_ranges: vec![],
        mapping_status: MappingStatus::Mapped,
        confidence: MatchConfidence::Exact,
        evidence: ChangeEvidence::new(
            prism_changes::evidence::EvidenceSource::SymbolRangeOverlap,
            "test",
        ),
    }
}

fn base_changeset(symbols: Vec<SymbolChange>) -> ChangeSet {
    ChangeSet {
        schema_version: 1,
        id: "changeset:test#1#2#working_tree".to_string(),
        source: ChangeSource::Git,
        scope: ChangeScope::WorkingTree,
        base: Some("1".to_string()),
        target: Some("worktree".to_string()),
        repository: Some("test".to_string()),
        files: vec![],
        symbols,
        modules: vec![],
        summary: ChangeSummary::default(),
        diagnostics: vec![],
        truncated: false,
    }
}

fn module_seed(module_id: &str, path: &str) -> prism_impact::Seed {
    prism_impact::Seed {
        key: prism_impact::NodeKey::module(module_id),
        path: path.to_string(),
        name: path.to_string(),
        change_confidence: ImpactConfidence::Exact,
        deleted_symbol: false,
        before_qualified: None,
        before_name: None,
    }
}

fn impacted<'a>(
    output: &'a prism_impact::TraversalOutput,
    kind: NodeKind,
    id: &str,
) -> &'a prism_impact::ImpactedAccumulator {
    output
        .impacted
        .get(&format!("{}:{id}", kind.as_str()))
        .unwrap_or_else(|| {
            panic!(
                "impacted {kind:?}:{id} expected, got {:?}",
                output.impacted.keys()
            )
        })
}

// ---------- §92/§93：模块反向依赖链 ----------

#[test]
fn reverse_dependency_chain_distances() {
    // Service 依赖 Utility；Controller 依赖 Service。改 Utility ⇒
    // Service = direct(1)；Controller = transitive(2)。
    let g = graph(
        vec![
            file_module("util.ts"),
            file_module("service.ts"),
            file_module("ctl.ts"),
        ],
        vec![
            dependency("service.ts", "util.ts"),
            dependency("ctl.ts", "service.ts"),
        ],
        vec![],
    );
    let index = prism_impact::ReverseIndex::build(&g);
    let seeds = vec![module_seed("module:file:util.ts", "util.ts")];
    let output = traverse(
        &seeds,
        &index,
        &ImpactOptions::default(),
        &CancellationToken::new(),
    );

    let service = impacted(&output, NodeKind::FileModule, "module:file:service.ts");
    assert_eq!(service.distance, 1);
    assert!(service.kinds.contains(&ImpactKind::DirectDependent));
    assert!(service
        .reasons
        .contains(&ImpactReason::DependsOnChangedModule));

    let ctl = impacted(&output, NodeKind::FileModule, "module:file:ctl.ts");
    assert_eq!(ctl.distance, 2);
    assert!(ctl.kinds.contains(&ImpactKind::TransitiveDependent));
    assert!(ctl.reasons.contains(&ImpactReason::TransitiveDependency));
    // 方向正确性（§12）：只有 util 的依赖者受影响。
    assert_eq!(output.impacted.len(), 2);
    assert!(!output.depth_exceeded);
    assert!(!output.node_limit_reached);
    assert_eq!(output.unresolved_edges, 0);
}

// ---------- §94：调用链 ----------

#[test]
fn call_chain_caller_and_transitive_caller() {
    // a.driver() 调 b.worker()；b.worker() 调 c.leaf()。改 c ⇒
    // b = Caller(1)；a = TransitiveCaller(2)。
    let g = graph(
        vec![
            file_module("a.ts"),
            file_module("b.ts"),
            file_module("c.ts"),
        ],
        vec![],
        vec![
            call("a.driver", "b.worker", true),
            call("b.worker", "c.leaf", true),
        ],
    );
    let index = prism_impact::ReverseIndex::build(&g);
    let changeset = base_changeset(vec![symbol_change(
        "c.ts",
        "c.leaf",
        SymbolChangeType::Modified,
    )]);
    let seeds = build_seeds(&changeset);
    assert_eq!(seeds.len(), 1, "symbol seed expected");
    let output = traverse(
        &seeds,
        &index,
        &ImpactOptions::default(),
        &CancellationToken::new(),
    );

    let b = impacted(
        &output,
        NodeKind::Symbol,
        &symbol_id("b.ts", "b.worker").to_string(),
    );
    assert_eq!(b.distance, 1);
    assert!(b.kinds.contains(&ImpactKind::Caller));
    assert!(b.reasons.contains(&ImpactReason::CallsChangedSymbol));

    let a = impacted(
        &output,
        NodeKind::Symbol,
        &symbol_id("a.ts", "a.driver").to_string(),
    );
    assert_eq!(a.distance, 2);
    assert!(a.kinds.contains(&ImpactKind::TransitiveCaller));
    assert!(a.reasons.contains(&ImpactReason::TransitiveCall));

    // 证据路径（§30）：a ← calls ← b ← calls ← c（种子）。
    assert_eq!(output.paths.len(), 2);
    let a_path = output
        .paths
        .iter()
        .find(|path| path.target.contains("a.driver"))
        .expect("path to a");
    assert_eq!(a_path.distance, 2);
    assert_eq!(a_path.hops.len(), 2);
    assert_eq!(a_path.hops[0].relation, prism_impact::RelationKind::Calls);
}

// ---------- §20/§96：已删除符号 ----------

#[test]
fn deleted_symbol_finds_callers_via_name_index() {
    // after 树中 foo 已删；调用点解析失败（Unresolved 边，名字=foo）。
    let g = graph(
        vec![file_module("lib.ts"), file_module("user.ts")],
        vec![],
        vec![call("user.caller", "lib.foo", false)],
    );
    let index = prism_impact::ReverseIndex::build(&g);
    // M5：Deleted 符号只有 before 身份。
    let changeset = base_changeset(vec![symbol_change(
        "lib.ts",
        "lib.foo",
        SymbolChangeType::Deleted,
    )]);
    let seeds = build_seeds(&changeset);
    assert_eq!(seeds.len(), 1);
    assert!(seeds[0].deleted_symbol);

    let output = traverse(
        &seeds,
        &index,
        &ImpactOptions::default(),
        &CancellationToken::new(),
    );
    let caller = impacted(
        &output,
        NodeKind::Symbol,
        &symbol_id("user.ts", "user.caller").to_string(),
    );
    assert_eq!(caller.distance, 1);
    assert!(caller.reasons.contains(&ImpactReason::CallsDeletedSymbol));
    // 精确限定名匹配 = HighConfidence；变更映射 Exact × 边 High ⇒ 不升 Exact（§46）。
    assert_eq!(caller.confidence, ImpactConfidence::HighConfidence);
}

// ---------- §50/§80/§101：环终止与自影响 ----------

#[test]
fn cycle_terminates_and_keeps_self_changed() {
    // 互相依赖：a↔b。改 a ⇒ b = dist1；a 不经环再次成为影响（§50）。
    let g = graph(
        vec![file_module("a.ts"), file_module("b.ts")],
        vec![dependency("a.ts", "b.ts"), dependency("b.ts", "a.ts")],
        vec![],
    );
    let index = prism_impact::ReverseIndex::build(&g);
    let seeds = vec![module_seed("module:file:a.ts", "a.ts")];
    let output = traverse(
        &seeds,
        &index,
        &ImpactOptions::default(),
        &CancellationToken::new(),
    );
    assert_eq!(output.impacted.len(), 1, "cycle must terminate");
    assert!(output.impacted.contains_key("file_module:module:file:b.ts"));
    assert!(!output.impacted.contains_key("file_module:module:file:a.ts"));
}

// ---------- §102/§103：未解析关系不伪造目标 ----------

#[test]
fn unresolved_dependency_not_traversed() {
    // consumer unresolved 地 import changed ⇒ 不伪造影响，但局限如实计数。
    let g = graph(
        vec![file_module("consumer.ts")],
        vec![unresolved_dependency("consumer.ts", "changed.ts")],
        vec![],
    );
    let index = prism_impact::ReverseIndex::build(&g);
    let seeds = vec![module_seed("module:file:changed.ts", "changed.ts")];
    let output = traverse(
        &seeds,
        &index,
        &ImpactOptions::default(),
        &CancellationToken::new(),
    );
    assert!(
        !output
            .impacted
            .contains_key("file_module:module:file:consumer.ts"),
        "unresolved edge must not fabricate impact (§103)"
    );
    assert_eq!(output.unresolved_edges, 1);
}

// ---------- §34/§82/§109：多路径合并 ----------

#[test]
fn multiple_paths_merge_to_one_node_with_min_distance() {
    // b 依赖 a（直接）且 b 依赖 c、c 依赖 a。改 a ⇒ b 一次，distance=1，paths=2。
    let g = graph(
        vec![
            file_module("a.ts"),
            file_module("b.ts"),
            file_module("c.ts"),
        ],
        vec![
            dependency("b.ts", "a.ts"),
            dependency("c.ts", "a.ts"),
            dependency("b.ts", "c.ts"),
        ],
        vec![],
    );
    let index = prism_impact::ReverseIndex::build(&g);
    let seeds = vec![module_seed("module:file:a.ts", "a.ts")];
    let output = traverse(
        &seeds,
        &index,
        &ImpactOptions::default(),
        &CancellationToken::new(),
    );
    let b = impacted(&output, NodeKind::FileModule, "module:file:b.ts");
    assert_eq!(b.distance, 1, "min distance wins (§82)");
    assert_eq!(b.path_count, 2, "both evidence paths counted (§34)");
    assert_eq!(output.impacted.len(), 2);
    assert_eq!(
        impacted(&output, NodeKind::FileModule, "module:file:c.ts").distance,
        1
    );
}

// ---------- §37/§78/§110/§111：上限 ----------

#[test]
fn large_fan_out_is_bounded_and_deterministic() {
    let mut modules = vec![file_module("util.ts")];
    let mut deps = Vec::new();
    for i in 0..1000 {
        let path = format!("consumer{i}.ts");
        modules.push(file_module(&path));
        deps.push(dependency(&path, "util.ts"));
    }
    let g = graph(modules, deps, vec![]);
    let index = prism_impact::ReverseIndex::build(&g);
    let seeds = vec![module_seed("module:file:util.ts", "util.ts")];

    let output = traverse(
        &seeds,
        &index,
        &ImpactOptions::default(),
        &CancellationToken::new(),
    );
    assert_eq!(output.impacted.len(), 1000);
    // 确定性（§79）：键序稳定。
    let first = output.impacted.keys().next().unwrap().clone();
    let again = traverse(
        &seeds,
        &index,
        &ImpactOptions::default(),
        &CancellationToken::new(),
    );
    assert_eq!(first, *again.impacted.keys().next().unwrap());

    // max_nodes（§37）。
    let options = ImpactOptions {
        max_nodes: 100,
        ..ImpactOptions::default()
    };
    let truncated = traverse(&seeds, &index, &options, &CancellationToken::new());
    assert!(truncated.node_limit_reached);
    assert!(truncated.impacted.len() <= 101, "seeds + capped nodes");
}

#[test]
fn max_depth_stops_traversal() {
    // n2→n1, n3→n2, n4→n3。max_depth=2 ⇒ 到 n3 为止，n4 排除。
    let g = graph(
        vec![
            file_module("n1.ts"),
            file_module("n2.ts"),
            file_module("n3.ts"),
            file_module("n4.ts"),
        ],
        vec![
            dependency("n2.ts", "n1.ts"),
            dependency("n3.ts", "n2.ts"),
            dependency("n4.ts", "n3.ts"),
        ],
        vec![],
    );
    let index = prism_impact::ReverseIndex::build(&g);
    let seeds = vec![module_seed("module:file:n1.ts", "n1.ts")];
    let options = ImpactOptions {
        max_depth: 2,
        ..ImpactOptions::default()
    };
    let output = traverse(&seeds, &index, &options, &CancellationToken::new());
    assert!(output
        .impacted
        .contains_key("file_module:module:file:n2.ts"));
    assert!(output
        .impacted
        .contains_key("file_module:module:file:n3.ts"));
    assert!(!output
        .impacted
        .contains_key("file_module:module:file:n4.ts"));
    assert!(output.depth_exceeded, "§36/§78：截断必须显式");
}

// ---------- §112：取消 ----------

#[test]
fn cancellation_stops_traversal() {
    let g = graph(
        vec![file_module("a.ts"), file_module("b.ts")],
        vec![dependency("b.ts", "a.ts")],
        vec![],
    );
    let index = prism_impact::ReverseIndex::build(&g);
    let seeds = vec![module_seed("module:file:a.ts", "a.ts")];
    let cancel = CancellationToken::new();
    cancel.cancel();
    let output = traverse(&seeds, &index, &ImpactOptions::default(), &cancel);
    assert!(output.cancelled);
    assert!(output.impacted.is_empty());
}

// ---------- §21/§59/§60：种子归一化 ----------

#[test]
fn added_symbols_are_not_seeded_and_files_map_to_modules() {
    let mut changeset = base_changeset(vec![symbol_change(
        "new.ts",
        "new.thing",
        SymbolChangeType::Added,
    )]);
    // 符号映射不可用的 Modified 文件 ⇒ FileModule 种子（§18）。
    changeset.files.push(FileChange {
        id: "fc:plain.ts".to_string(),
        old_path: None,
        new_path: Some("plain.ts".to_string()),
        change_type: FileChangeType::Modified,
        additions: Some(1),
        deletions: Some(1),
        old_exists: true,
        new_exists: true,
        old_language: None,
        new_language: None,
        old_file_kind: None,
        new_file_kind: None,
        language_changed: false,
        is_binary: false,
        similarity: None,
        mapping_status: MappingStatus::NotApplicable,
        evidence: ChangeEvidence::new(
            prism_changes::evidence::EvidenceSource::GitDiff,
            "unsupported",
        ),
    });

    let seeds = build_seeds(&changeset);
    // Added 符号不产生消费者遍历种子（§21）；plain.ts ⇒ module:file:plain.ts。
    assert_eq!(seeds.len(), 1);
    assert_eq!(seeds[0].key.id, "module:file:plain.ts");
}

// ---------- §19/§99：rename 用 after 身份 ----------

#[test]
fn rename_prefers_after_identity() {
    // rename 后的 bar 在当前图中被调用 ⇒ 以 after 身份找到调用者。
    let g = graph(
        vec![file_module("lib.ts"), file_module("user.ts")],
        vec![],
        vec![call("user.site", "lib.bar", true)],
    );
    let index = prism_impact::ReverseIndex::build(&g);
    let changeset = base_changeset(vec![symbol_change(
        "lib.ts",
        "lib.bar",
        SymbolChangeType::Renamed,
    )]);
    let seeds = build_seeds(&changeset);
    assert_eq!(seeds.len(), 1);
    assert!(
        seeds[0].key.id.contains("lib.bar"),
        "after identity preferred"
    );
    assert!(!seeds[0].deleted_symbol);

    let output = traverse(
        &seeds,
        &index,
        &ImpactOptions::default(),
        &CancellationToken::new(),
    );
    let site = impacted(
        &output,
        NodeKind::Symbol,
        &symbol_id("user.ts", "user.site").to_string(),
    );
    assert!(site.reasons.contains(&ImpactReason::CallsChangedSymbol));
    // M5 rename 置信在此测试为 Exact（fixture 简化）；真实链路由 service 层
    // 以 weakest 传播（e2e 测试覆盖）。这里验证关系链不断（§99）。
    assert_eq!(site.confidence, ImpactConfidence::HighConfidence);
}
