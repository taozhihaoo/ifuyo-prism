//! 反向索引与有界反向遍历（规格 M6 §58–§64、§78、§80–§84、§92–§113）。
//!
//! 方向契约（§11/§12，依据 M3 真实实现而非字段名）：
//! - `Dependency{source_id → target_id}`：source **依赖** target（import 语义）
//!   ⇒ 反向遍历 = 谁的 target 是被改模块（谁依赖它）。
//! - `CallEdge{caller_id → callee_id}`：caller **调用** callee
//!   ⇒ 反向遍历 = 谁的 callee 是被改符号（调用者）。
//!
//! 索引只读 M3 事实建一次（§62/§63），不复制图模型（§64）。
//! 只遍历 Resolved 边；Unresolved/Ambiguous 计数与诊断，不伪造目标
//! （§5/§102/§103）——唯一例外：指向已删除符号名字的未解析调用
//! （§20/§96，按精确限定名优先匹配，置信降级）。

use crate::model::{
    ImpactConfidence, ImpactKind, ImpactOptions, ImpactPath, ImpactReason, NodeKind, PathHop,
    RelationKind,
};
use prism_changes::{ChangeSet, FileChangeType, MappingStatus, MatchConfidence, SymbolChangeType};
use prism_core::task::CancellationToken;
use prism_graph::{DependencyGraphResult, DependencyKind, ResolutionStatus};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// M3 Confidence → 影响置信度（图边不产生 Exact）。
fn map_confidence(confidence: prism_core::Confidence) -> ImpactConfidence {
    match confidence {
        prism_core::Confidence::High => ImpactConfidence::HighConfidence,
        prism_core::Confidence::Medium | prism_core::Confidence::Low => ImpactConfidence::Probable,
    }
}

/// SymbolId 内嵌路径段：`symbol:{path}#{qualified}#{kind}` ⇒ 取 path。
fn path_of_symbol_id(symbol_id: &str) -> &str {
    let rest = symbol_id.strip_prefix("symbol:").unwrap_or(symbol_id);
    rest.split('#').next().unwrap_or(rest)
}

fn name_of_qualified(qualified: &str) -> &str {
    qualified.rsplit('.').next().unwrap_or(qualified)
}

/// 一条反向边：from（依赖方/调用方）→ 被改节点。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReverseEdge {
    pub from: String,
    pub relation: RelationKind,
    /// "resolved" / "unresolved" / "ambiguous"（M3 ResolutionStatus 原样语义）。
    pub resolution: String,
    pub confidence: ImpactConfidence,
    /// 展示路径（调用方符号所在文件 / 依赖方模块路径）。
    pub from_path: String,
    /// 调用方符号限定名（符号边专用；模块边为空）。
    pub from_qualified: String,
}

/// 反向邻接索引（§62；§63 不复制 M3 模型，只建索引）。
#[derive(Debug, Default, Clone)]
pub struct ReverseIndex {
    /// target 模块 → 依赖它的模块边（仅 Resolved）。
    pub module_dependents: BTreeMap<String, Vec<ReverseEdge>>,
    /// callee 符号 → 调用它的符号边（仅 Resolved）。
    pub callers_by_symbol: BTreeMap<String, Vec<ReverseEdge>>,
    /// 名字/限定名 → 指向该名字的未解析调用边（§20 deleted-symbol 专用）。
    pub unresolved_callers_by_name: BTreeMap<String, Vec<ReverseEdge>>,
    /// raw_target → 未解析/歧义模块依赖边（仅计数/诊断用，不遍历，§5）。
    pub unresolved_dependents_by_target: BTreeMap<String, Vec<ReverseEdge>>,
    /// 模块 id → 展示路径。
    pub module_paths: BTreeMap<String, String>,
}

impl ReverseIndex {
    /// 从 M3 图事实构建（一次，§62/§77）。
    pub fn build(graph: &DependencyGraphResult) -> Self {
        let mut index = ReverseIndex::default();
        for module in &graph.modules {
            if let Some(path) = &module.path {
                index
                    .module_paths
                    .insert(module.id.as_str().to_string(), path.clone());
            }
        }
        for dependency in &graph.dependencies {
            if dependency.resolution != ResolutionStatus::Resolved {
                // 未解析/歧义模块依赖：仅进计数索引（§44/§102/§103），
                // 绝不作为遍历边（否则伪造影响目标）。
                let relation = if dependency.kind == DependencyKind::ProjectReference {
                    RelationKind::ProjectReferences
                } else {
                    RelationKind::DependsOn
                };
                index
                    .unresolved_dependents_by_target
                    .entry(dependency.raw_target.clone())
                    .or_default()
                    .push(ReverseEdge {
                        from: dependency.source_id.as_str().to_string(),
                        relation,
                        resolution: match dependency.resolution {
                            ResolutionStatus::Ambiguous => "ambiguous".to_string(),
                            _ => "unresolved".to_string(),
                        },
                        confidence: ImpactConfidence::Probable,
                        from_path: dependency.source_path.clone(),
                        from_qualified: String::new(),
                    });
                continue;
            }
            let Some(target_id) = &dependency.target_id else {
                continue;
            };
            let relation = if dependency.kind == DependencyKind::ProjectReference {
                RelationKind::ProjectReferences
            } else {
                RelationKind::DependsOn
            };
            index
                .module_dependents
                .entry(target_id.as_str().to_string())
                .or_default()
                .push(ReverseEdge {
                    from: dependency.source_id.as_str().to_string(),
                    relation,
                    resolution: "resolved".to_string(),
                    confidence: map_confidence(dependency.confidence),
                    from_path: dependency.source_path.clone(),
                    from_qualified: String::new(),
                });
        }
        for call in &graph.calls {
            let caller = ReverseEdge {
                from: call.caller_id.as_str().to_string(),
                relation: RelationKind::Calls,
                resolution: match call.resolution {
                    ResolutionStatus::Ambiguous => "ambiguous".to_string(),
                    _ => "unresolved".to_string(),
                },
                confidence: ImpactConfidence::Probable,
                from_path: path_of_symbol_id(call.caller_id.as_str()).to_string(),
                from_qualified: call.caller_qualified.clone(),
            };
            let Some(callee_id) = &call.callee_id else {
                // 未解析调用：进名字索引（deleted-symbol 证据），不当常规遍历边。
                index
                    .unresolved_callers_by_name
                    .entry(call.callee_name.clone())
                    .or_default()
                    .push(caller.clone());
                let qualified_key = call
                    .callee_qualified
                    .clone()
                    .unwrap_or_else(|| call.callee_name.clone());
                index
                    .unresolved_callers_by_name
                    .entry(qualified_key)
                    .or_default()
                    .push(caller);
                continue;
            };
            index
                .callers_by_symbol
                .entry(callee_id.as_str().to_string())
                .or_default()
                .push(ReverseEdge {
                    from: call.caller_id.as_str().to_string(),
                    relation: RelationKind::Calls,
                    resolution: "resolved".to_string(),
                    confidence: map_confidence(call.confidence),
                    from_path: path_of_symbol_id(call.caller_id.as_str()).to_string(),
                    from_qualified: call.caller_qualified.clone(),
                });
        }
        index
    }
}

/// 遍历节点键。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeKey {
    pub kind: NodeKind,
    pub id: String,
}

impl NodeKey {
    pub fn module(id: &str) -> Self {
        NodeKey {
            kind: NodeKind::FileModule,
            id: id.to_string(),
        }
    }

    pub fn symbol(id: &str) -> Self {
        NodeKey {
            kind: NodeKind::Symbol,
            id: id.to_string(),
        }
    }

    pub fn state_key(&self) -> String {
        format!("{}:{}", self.kind.as_str(), self.id)
    }
}

/// 一个遍历种子（§59/§60：已归一化）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Seed {
    pub key: NodeKey,
    pub path: String,
    pub name: String,
    /// M5 变更映射置信（传播起点，§46/§47）。
    pub change_confidence: ImpactConfidence,
    /// 被改符号已删除：用 before 身份 + 名字索引（§20）。
    pub deleted_symbol: bool,
    pub before_qualified: Option<String>,
    pub before_name: Option<String>,
}

/// 受影响节点累积器（§83 合并、§34 多路径计数）。
#[derive(Debug, Clone)]
pub struct ImpactedAccumulator {
    pub key: NodeKey,
    pub distance: u32,
    pub confidence: ImpactConfidence,
    pub reasons: BTreeSet<ImpactReason>,
    pub kinds: BTreeSet<ImpactKind>,
    pub path_count: u64,
    pub path: String,
    pub name: String,
}

/// 遍历输出（service 再汇总/出报告字段）。
#[derive(Debug, Default)]
pub struct TraversalOutput {
    /// distance ≥ 1 的受影响节点（按 state_key 排序 ⇒ 确定性，§79）。
    pub impacted: BTreeMap<String, ImpactedAccumulator>,
    /// 每个受影响节点一条最短证据路径（§30/§82）。
    pub paths: Vec<ImpactPath>,
    pub depth_exceeded: bool,
    pub node_limit_reached: bool,
    pub cancelled: bool,
    /// 被改节点的反向边中 Unresolved/Ambiguous 计数（§44）。
    pub unresolved_edges: u64,
    pub ambiguous_edges: u64,
}

/// 从 ChangeSet 构建归一化种子（§59/§60）。
///
/// - 符号：Modified/Renamed/Moved ⇒ after 身份；Deleted ⇒ before 身份 +
///   名字索引；**Added 不做消费者遍历**（§21：没有既有消费者）。
/// - 模块：ModuleChange 的模块 id。
/// - 文件：无符号映射的 Modified/Renamed 文件 ⇒ 其 FileModule 种子（§18）。
pub fn build_seeds(change_set: &ChangeSet) -> Vec<Seed> {
    let mut seeds: Vec<Seed> = Vec::new();
    let mut seen: BTreeSet<String> = BTreeSet::new();

    let mut push = |seed: Seed| {
        if seen.insert(seed.key.state_key()) {
            seeds.push(seed);
        }
    };

    for symbol in &change_set.symbols {
        match symbol.change_type {
            SymbolChangeType::Added => continue,
            SymbolChangeType::Deleted => {
                let Some(before_id) = &symbol.before_symbol_id else {
                    continue;
                };
                push(Seed {
                    key: NodeKey::symbol(before_id.as_str()),
                    path: symbol.path.clone(),
                    name: symbol.name.clone(),
                    change_confidence: ImpactConfidence::Exact,
                    deleted_symbol: true,
                    before_qualified: symbol.before_qualified_name.clone(),
                    before_name: Some(symbol.name.clone()),
                });
            }
            _ => {
                let Some(after_id) = &symbol.after_symbol_id else {
                    continue;
                };
                let change_confidence = match symbol.confidence {
                    MatchConfidence::Exact => ImpactConfidence::Exact,
                    MatchConfidence::HighConfidence => ImpactConfidence::HighConfidence,
                    _ => ImpactConfidence::Probable,
                };
                push(Seed {
                    key: NodeKey::symbol(after_id.as_str()),
                    path: symbol.path.clone(),
                    name: symbol.name.clone(),
                    change_confidence,
                    deleted_symbol: false,
                    before_qualified: None,
                    before_name: None,
                });
            }
        }
    }

    for module in &change_set.modules {
        push(Seed {
            key: NodeKey::module(module.module_id.as_str()),
            path: module.changed_files.first().cloned().unwrap_or_default(),
            name: module.module_name.clone(),
            change_confidence: ImpactConfidence::Exact,
            deleted_symbol: false,
            before_qualified: None,
            before_name: None,
        });
    }

    // 文件级兜底（§18）：无符号覆盖的 Modified/Renamed 文件 ⇒ FileModule 种子。
    for file in &change_set.files {
        let symbol_covered = matches!(
            file.mapping_status,
            MappingStatus::Mapped | MappingStatus::Ambiguous
        ) && change_set
            .symbols
            .iter()
            .any(|symbol| symbol.file_change_id == file.id);
        if symbol_covered {
            continue;
        }
        // 纯新增（非二进制）文件没有既有消费者事实（§21）。
        if file.change_type == FileChangeType::Added && !file.is_binary {
            continue;
        }
        let Some(path) = file.new_path.as_deref().or(file.old_path.as_deref()) else {
            continue;
        };
        push(Seed {
            key: NodeKey::module(prism_graph::file_module_id(path).as_str()),
            path: path.to_string(),
            name: path.to_string(),
            change_confidence: ImpactConfidence::Exact,
            deleted_symbol: false,
            before_qualified: None,
            before_name: None,
        });
    }

    seeds
}

/// 每节点保留的前驱上限（§34/§78 有界；主路径=最短先到）。
const MAX_PREDECESSORS: usize = 4;

/// 内部 BFS 状态。
#[derive(Debug, Clone)]
struct NodeState {
    distance: u32,
    confidence: ImpactConfidence,
    reasons: BTreeSet<ImpactReason>,
    /// 证据前驱（首项 = 最短路径主前驱；余项 = 保留的替代路径，§109）。
    predecessors: Vec<(NodeKey, PathHop)>,
}

/// 有界反向 BFS（§78 limits；§80 visited 确定性；§112 取消）。
pub fn traverse(
    seeds: &[Seed],
    index: &ReverseIndex,
    options: &ImpactOptions,
    cancel: &CancellationToken,
) -> TraversalOutput {
    let mut output = TraversalOutput::default();
    let mut states: BTreeMap<String, NodeState> = BTreeMap::new();
    let mut queue: VecDeque<(NodeKey, u32)> = VecDeque::new();
    let mut node_count: u64 = 0;
    let mut deleted_symbol_ids: BTreeSet<String> = BTreeSet::new();

    for seed in seeds {
        // 被改节点相邻边的 resolution 计数（§44/§103）。
        // deleted 符号的调用者会同时出现在限定名与名字两个索引里：按
        // (resolution, caller) 去重，只计一次。
        let mut counted: BTreeSet<String> = BTreeSet::new();
        for edge in edges_of_seed(seed, index) {
            let dedup_key = format!("{}:{}", edge.resolution, edge.from);
            if !counted.insert(dedup_key) {
                continue;
            }
            match edge.resolution.as_str() {
                "unresolved" => output.unresolved_edges += 1,
                "ambiguous" => output.ambiguous_edges += 1,
                _ => {}
            }
        }
        if seed.deleted_symbol {
            deleted_symbol_ids.insert(seed.key.id.clone());
        }
        states.insert(
            seed.key.state_key(),
            NodeState {
                distance: 0,
                confidence: seed.change_confidence,
                reasons: BTreeSet::new(),
                predecessors: Vec::new(),
            },
        );
        queue.push_back((seed.key.clone(), 0));
        node_count += 1;
    }

    while let Some((node, distance)) = queue.pop_front() {
        if cancel.is_cancelled() {
            output.cancelled = true;
            return output;
        }
        let new_distance = distance + 1;
        if new_distance > options.max_depth {
            output.depth_exceeded = true;
            continue;
        }
        let Some(state) = states.get(&node.state_key()) else {
            continue;
        };
        let source_confidence = state.confidence;

        // 展开反向边：模块⇒依赖者；符号⇒调用者；已删除符号⇒名字级调用。
        let mut expansions: Vec<ReverseEdge> = Vec::new();
        match node.kind {
            NodeKind::FileModule | NodeKind::Package | NodeKind::Namespace | NodeKind::Project => {
                expansions.extend(
                    index
                        .module_dependents
                        .get(&node.id)
                        .cloned()
                        .unwrap_or_default(),
                );
            }
            NodeKind::Symbol => {
                expansions.extend(
                    index
                        .callers_by_symbol
                        .get(&node.id)
                        .cloned()
                        .unwrap_or_default(),
                );
                if deleted_symbol_ids.contains(&node.id) {
                    expansions.extend(deleted_symbol_callers(&node.id, seeds, index));
                }
            }
            NodeKind::File | NodeKind::Documentation => {}
        }

        for edge in expansions {
            let dependent_key = match node.kind {
                NodeKind::Symbol => NodeKey::symbol(&edge.from),
                _ => NodeKey::module(&edge.from),
            };
            let edge_confidence = source_confidence.weakest(edge.confidence);
            let reason = classify_reason(&node.kind, &edge, new_distance);

            let hop = PathHop {
                from: node.id.clone(),
                relation: edge.relation,
                to: edge.from.clone(),
                resolution: edge.resolution.clone(),
                confidence: edge_confidence,
            };
            let existing = states.get(&dependent_key.state_key()).cloned();
            match existing {
                None => {
                    if node_count >= options.max_nodes {
                        output.node_limit_reached = true;
                        continue;
                    }
                    node_count += 1;
                    let mut reasons = BTreeSet::new();
                    reasons.insert(reason);
                    states.insert(
                        dependent_key.state_key(),
                        NodeState {
                            distance: new_distance,
                            confidence: edge_confidence,
                            reasons,
                            predecessors: vec![(node.clone(), hop)],
                        },
                    );
                    queue.push_back((dependent_key, new_distance));
                }
                Some(existing) => {
                    if let Some(current) = states.get_mut(&dependent_key.state_key()) {
                        if new_distance == existing.distance {
                            // 同距离多路径：合并 reason/confidence，保留前驱（§34/§82）。
                            current.reasons.insert(reason);
                            current.confidence = current.confidence.weakest(edge_confidence);
                            if current.predecessors.len() < MAX_PREDECESSORS
                                && current
                                    .predecessors
                                    .iter()
                                    .all(|(prev, _)| prev.id != node.id)
                            {
                                current.predecessors.push((node.clone(), hop));
                            }
                        } else if new_distance > existing.distance
                            && current.predecessors.len() < MAX_PREDECESSORS
                            && current
                                .predecessors
                                .iter()
                                .all(|(prev, _)| prev.id != node.id)
                        {
                            // 更长的替代路径保留一条证据（§109 multiple evidence paths）。
                            current.reasons.insert(reason);
                            current.predecessors.push((node.clone(), hop));
                        }
                        // 更短距离已被 BFS 先到。
                    }
                }
            }
        }
    }

    // ---- 汇总 impacted（distance ≥ 1），分类确定性 ----
    for (state_key, state) in &states {
        if state.distance == 0 {
            continue;
        }
        let key = decode_state_key(state_key);
        let mut kinds: BTreeSet<ImpactKind> = BTreeSet::new();
        if state.reasons.contains(&ImpactReason::CallsChangedSymbol) {
            kinds.insert(ImpactKind::Caller);
        }
        if state.reasons.contains(&ImpactReason::CallsDeletedSymbol) {
            kinds.insert(ImpactKind::Caller);
        }
        if state.reasons.contains(&ImpactReason::TransitiveCall) {
            kinds.insert(ImpactKind::TransitiveCaller);
        }
        if state
            .reasons
            .contains(&ImpactReason::DependsOnChangedModule)
        {
            kinds.insert(ImpactKind::ModuleConsumer);
        }
        if state.reasons.contains(&ImpactReason::TransitiveDependency) {
            kinds.insert(ImpactKind::TransitiveDependent);
        }
        if state.reasons.contains(&ImpactReason::ProjectReference) {
            kinds.insert(ImpactKind::ProjectConsumer);
        }
        if state.distance == 1 {
            kinds.insert(ImpactKind::DirectDependent);
        } else {
            kinds.insert(ImpactKind::TransitiveDependent);
        }
        let (name, path) = display_for(&key, index);
        output.impacted.insert(
            state_key.clone(),
            ImpactedAccumulator {
                key,
                distance: state.distance,
                confidence: state.confidence,
                reasons: state.reasons.clone(),
                kinds,
                path_count: state.predecessors.len() as u64,
                path,
                name,
            },
        );
    }

    // ---- 证据路径重建：沿 best_prev 回溯到 distance 0（§30，每节点一条最短）----
    for (state_key, accumulator) in output.impacted.clone() {
        // 环安全（§80）：只从 distance ≥ 1 的节点回溯，到达 distance 0 即停；
        // 跳数上限 = max_depth + 1，任何环都不可能走成无限路径。
        let mut hops: Vec<PathHop> = Vec::new();
        let mut walk = Some(accumulator.key.clone());
        while let Some(current) = walk {
            let Some(state) = states.get(&current.state_key()) else {
                break;
            };
            if state.distance == 0 || hops.len() > options.max_depth as usize + 1 {
                break;
            }
            match state.predecessors.first() {
                Some((prev, hop)) => {
                    hops.push(hop.clone());
                    walk = Some(prev.clone());
                }
                None => break,
            }
        }
        hops.reverse();
        if let Some(first_hop) = hops.first() {
            let path = ImpactPath {
                id: format!(
                    "ip:{}#{}#{}#0",
                    first_hop.from, accumulator.key.id, accumulator.distance
                ),
                source_change: first_hop.from.clone(),
                target: accumulator.key.id.clone(),
                distance: accumulator.distance,
                hops,
            };
            output.paths.push(path);
        }
        let _ = state_key;
    }

    output
}

/// 种子可见的反向边（含 deleted 名字索引；unresolved 计数用）。
fn edges_of_seed<'a>(seed: &Seed, index: &'a ReverseIndex) -> Vec<&'a ReverseEdge> {
    let mut edges = Vec::new();
    let collect =
        |key: &Option<String>, index: &'a ReverseIndex, edges: &mut Vec<&'a ReverseEdge>| {
            if let Some(key) = key {
                if let Some(list) = index.unresolved_callers_by_name.get(key) {
                    edges.extend(list.iter());
                }
            }
        };
    match seed.key.kind {
        NodeKind::FileModule | NodeKind::Package | NodeKind::Namespace | NodeKind::Project => {
            if let Some(list) = index.module_dependents.get(&seed.key.id) {
                edges.extend(list.iter());
            }
            // 未解析模块依赖按 raw_target 对齐（仅当恰好等于项目相对路径，
            // 保守：不对 specifier 做猜测解析）。
            if let Some(list) = index.unresolved_dependents_by_target.get(&seed.path) {
                edges.extend(list.iter());
            }
        }
        NodeKind::Symbol => {
            if let Some(list) = index.callers_by_symbol.get(&seed.key.id) {
                edges.extend(list.iter());
            }
            if seed.deleted_symbol {
                collect(&seed.before_qualified, index, &mut edges);
                collect(&seed.before_name, index, &mut edges);
            }
        }
        _ => {}
    }
    edges
}

/// 已删除符号的名字级调用者（§20）：精确限定名 HighConfidence；同名 Probable。
fn deleted_symbol_callers(node_id: &str, seeds: &[Seed], index: &ReverseIndex) -> Vec<ReverseEdge> {
    let Some(seed) = seeds
        .iter()
        .find(|seed| seed.deleted_symbol && seed.key.id == node_id)
    else {
        return Vec::new();
    };
    let mut edges: Vec<ReverseEdge> = Vec::new();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    // 分类规则（§45/§46）：精确限定名匹配 = HighConfidence（身份唯一）；
    // 仅同名 = Probable。不与名字索引边的泛化 Probable 再取弱。
    let mut collect = |key: Option<&String>, confidence: ImpactConfidence| {
        if let Some(key) = key {
            if let Some(list) = index.unresolved_callers_by_name.get(key) {
                for mut edge in list.clone() {
                    if seen.insert(edge.from.clone()) {
                        edge.confidence = confidence;
                        edges.push(edge);
                    }
                }
            }
        }
    };
    collect(
        seed.before_qualified.as_ref(),
        ImpactConfidence::HighConfidence,
    );
    collect(seed.before_name.as_ref(), ImpactConfidence::Probable);
    edges
}

/// 关系 → 影响理由（§61：定义 impact 含义；跳数>1 用传递语义）。
fn classify_reason(changed_kind: &NodeKind, edge: &ReverseEdge, distance: u32) -> ImpactReason {
    match changed_kind {
        NodeKind::Symbol => {
            if edge.resolution == "unresolved" {
                // 名字级未解析调用仅出现在 deleted 符号展开（§20）。
                ImpactReason::CallsDeletedSymbol
            } else if distance > 1 {
                ImpactReason::TransitiveCall
            } else {
                ImpactReason::CallsChangedSymbol
            }
        }
        _ => match edge.relation {
            RelationKind::ProjectReferences => ImpactReason::ProjectReference,
            _ => {
                if distance > 1 {
                    ImpactReason::TransitiveDependency
                } else {
                    ImpactReason::DependsOnChangedModule
                }
            }
        },
    }
}

/// 节点展示名与路径。
fn display_for(key: &NodeKey, index: &ReverseIndex) -> (String, String) {
    match key.kind {
        NodeKind::FileModule | NodeKind::Package | NodeKind::Namespace | NodeKind::Project => {
            let path = index.module_paths.get(&key.id).cloned().unwrap_or_default();
            let name = path.rsplit('/').next().unwrap_or(&path).to_string();
            (name, path)
        }
        NodeKind::Symbol => {
            let rest = key.id.strip_prefix("symbol:").unwrap_or(&key.id);
            let mut segments = rest.splitn(3, '#');
            let path = segments.next().unwrap_or("").to_string();
            let qualified = segments.next().unwrap_or("");
            (name_of_qualified(qualified).to_string(), path)
        }
        _ => (key.id.clone(), String::new()),
    }
}

/// state_key（"kind:id"）解码回 NodeKey。
fn decode_state_key(state_key: &str) -> NodeKey {
    let (kind, id) = state_key
        .split_once(':')
        .unwrap_or(("file_module", state_key));
    let kind = match kind {
        "symbol" => NodeKind::Symbol,
        "package" => NodeKind::Package,
        "namespace" => NodeKind::Namespace,
        "project" => NodeKind::Project,
        "file" => NodeKind::File,
        "documentation" => NodeKind::Documentation,
        _ => NodeKind::FileModule,
    };
    NodeKey {
        kind,
        id: id.to_string(),
    }
}
