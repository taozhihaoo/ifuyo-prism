//! Impact 模型与遍历（2.0-M26 §5-§9/§33-§41/§59/§65-§68）。
//!
//! - Impact 四类：Direct / Transitive / Potential / Unknown（§5）；
//!   与 Confidence 分离（§6）；无百分比伪装（§38）。
//! - 遍历：反向边、cycle-safe（visited）、有界（max_hops/max_entities，
//!   §33/§34/§59）、确定性（BTreeMap + 固定排序，§58）。
//! - Change-aware 规则（§16）：ChangeType × RelationKind ⇒ 类别与传播。
//! - 去重（§67/§68）：实体唯一；Direct 优先 Transitive；保留主路径。

use crate::graph::EvidenceGraph;
use crate::Confidence;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// 变更种类（§12 首批；由 Change Mapping 判定）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeType {
    /// 声明移除/签名不兼容变化（§20 Contract changed）。
    ContractRemoved,
    /// 声明签名变化（参数/返回/约束）。
    SignatureChanged,
    /// 仅实现体变化（§19 Body-only ≠ callers broken）。
    BodyChanged,
    /// 基类型/接口面变化（§14）。
    BaseOrInterfaceChanged,
    /// 新增声明（传播面 = 新实体的反向邻居）。
    Added,
    /// 文件级变化（无法映射到符号——scope Unknown，§44 诚实降级）。
    FileLevel,
}

/// Impact 类型（§5）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImpactType {
    Direct,
    Transitive,
    Potential,
    Unknown,
}

/// Impact 类别（§17）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImpactCategory {
    Breaking,
    Behavioral,
    Structural,
    Test,
    Runtime,
    Unknown,
}

/// 一条受影响实体记录（§67：实体唯一 + 主路径 + 备选路径数）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AffectedEntity {
    pub entity_id: String,
    pub impact: ImpactType,
    pub category: ImpactCategory,
    pub confidence: Confidence,
    /// 最短主路径（entity id 链，含起点变更实体）。
    pub primary_path: Vec<String>,
    /// 其余更长的解释路径数（§67 保留计数，不展开）。
    pub alternative_paths: usize,
}

/// 遍历/结果上限（§33/§57；默认值适合普通开发者，§65）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImpactPolicy {
    pub max_hops: usize,
    pub max_entities: usize,
}

impl Default for ImpactPolicy {
    fn default() -> Self {
        ImpactPolicy {
            max_hops: 3,
            max_entities: 500,
        }
    }
}

/// 遍历结果统计（§59 cycle / §66 截断的诚实元数据）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TraversalMetadata {
    pub cycles_detected: usize,
    pub truncated_by_hop_limit: usize,
    pub truncated_by_entity_limit: usize,
}

/// Impact 分析结果（§41：Affected Set；Risk 排序归 M28 §42）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ImpactResult {
    /// 按 (impact, category, 路径长, id) 确定序排列。
    pub affected: Vec<AffectedEntity>,
    pub metadata: TraversalMetadata,
}

/// 单条变更（Change Mapping 产物）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntityChange {
    pub entity_id: String,
    pub change_type: ChangeType,
}

/// Change-aware 传播规则（§16 表的代码化；返回类别 + 是否继续传播）。
fn rule_for(change: ChangeType, relation: &str) -> Option<(ImpactCategory, bool)> {
    let calls_like = matches!(relation, "calls" | "constructs");
    let reference_like = matches!(
        relation,
        "references_type" | "attached_script" | "references_resource"
    );
    let hierarchy = matches!(
        relation,
        "inherits" | "implements" | "overrides" | "backed_by"
    );
    match change {
        // Contract 移除/不兼容：调用/引用方 Breaking；层级派生 Behavioral。
        ChangeType::ContractRemoved => {
            if calls_like || reference_like {
                Some((ImpactCategory::Breaking, false))
            } else if hierarchy {
                Some((ImpactCategory::Behavioral, true))
            } else {
                Some((ImpactCategory::Unknown, false))
            }
        }
        // 签名变化：调用方 Breaking（binding 失效风险）；引用方 Behavioral。
        ChangeType::SignatureChanged => {
            if calls_like {
                Some((ImpactCategory::Breaking, true))
            } else if reference_like || hierarchy {
                Some((ImpactCategory::Behavioral, true))
            } else {
                Some((ImpactCategory::Unknown, false))
            }
        }
        // 仅实现体：行为可能变化——Potential，不自动 Breaking（§19/§70 Case 2）。
        // §19/§70 Case 2：Body-only ⇒ Behavioral（行为可能变化，非 Breaking）。
        ChangeType::BodyChanged => {
            if calls_like || hierarchy {
                Some((ImpactCategory::Behavioral, true))
            } else {
                Some((ImpactCategory::Unknown, false))
            }
        }
        // 基类型/接口面变化：派生/实现面 Behavioral（§21/§22 按 Change 判断）。
        ChangeType::BaseOrInterfaceChanged => {
            if hierarchy {
                Some((ImpactCategory::Behavioral, true))
            } else {
                Some((ImpactCategory::Structural, true))
            }
        }
        // 新增：反向邻居 Structural（新面可用）。
        ChangeType::Added => Some((ImpactCategory::Structural, false)),
        // 文件级（scope Unknown）：Unknown 类别（§44）。
        ChangeType::FileLevel => Some((ImpactCategory::Unknown, true)),
    }
}

/// Impact 遍历：反向传播（§33-§36）——cycle-safe（§59）、有界、确定性（§58）。
///
/// BFS 分层：hop 1 = Direct，hop ≥ 2 = Transitive；类别由
/// `rule_for(change_type, relation)` 决定（§16 Change-aware 传播）；
/// unresolved 边降 Confidence（§63）；路径全程记录（§8 可解释）。
pub fn analyze(
    graph: &EvidenceGraph,
    changes: &[EntityChange],
    policy: &ImpactPolicy,
) -> ImpactResult {
    use std::collections::VecDeque;

    let mut metadata = TraversalMetadata::default();
    let mut affected: BTreeMap<String, AffectedEntity> = BTreeMap::new();
    // 已访问实体（含变更起点）——cycle-safe 与去重的统一屏障。
    let mut visited: BTreeMap<String, ()> = BTreeMap::new();
    // 队列：(当前节点, 携带的变更类型, 从变更起点到当前节点的路径, hop)。
    let mut queue: VecDeque<(String, ChangeType, Vec<String>, usize)> = VecDeque::new();

    // 变更起点入队（起点本身不是受影响记录；change_type 随边传播 §16）。
    let mut seeds: BTreeMap<String, ChangeType> = BTreeMap::new();
    for change in changes {
        // §13/§72 签名变化种子扩展：旧签名 id 在当前图中无节点（签名含于
        // id）时，回退到 **qualified 前缀** 匹配图内节点——新签名节点以同
        // qualified 承载旧契约，其 callers 即受影响方。
        if graph
            .edges
            .iter()
            .any(|e| e.source == change.entity_id || e.target == change.entity_id)
        {
            seeds
                .entry(change.entity_id.clone())
                .or_insert(change.change_type);
            continue;
        }
        let stripped = change.entity_id.trim_start_matches("cs:");
        let qualified = stripped.split('#').next().unwrap_or(stripped);
        let prefix = format!("cs:{qualified}#");
        let mut expanded = false;
        let mut endpoints: std::collections::BTreeSet<&String> = std::collections::BTreeSet::new();
        for edge in &graph.edges {
            for endpoint in [&edge.source, &edge.target] {
                if endpoint.starts_with(&prefix) {
                    endpoints.insert(endpoint);
                }
            }
        }
        for endpoint in endpoints {
            expanded = true;
            seeds.entry(endpoint.clone()).or_insert(change.change_type);
        }
        if !expanded {
            seeds
                .entry(change.entity_id.clone())
                .or_insert(change.change_type);
        }
    }
    for (entity, change_type) in &seeds {
        visited.insert(entity.clone(), ());
        queue.push_back((entity.clone(), *change_type, vec![entity.clone()], 0));
    }

    // §63 名字面候选：ContractRemoved ⇒ unresolved 调用点（伪目标
    // `unresolved:{成员名}`）为 Potential/Low 候选——单跳直查，不进
    // visited 主遍历（名字面 = 候选级证据，不与图遍历混淆）。
    let mut name_seeds: BTreeMap<String, ChangeType> = BTreeMap::new();
    for change in changes {
        if change.change_type == ChangeType::ContractRemoved {
            let stripped = change.entity_id.trim_start_matches("cs:");
            if let Some(member) = stripped
                .split('#')
                .next()
                .and_then(|q| q.rsplit('.').next())
            {
                name_seeds
                    .entry(format!("unresolved:{member}"))
                    .or_insert(change.change_type);
            }
        }
    }

    if std::env::var("PRISM_DBG_IMPACT").is_ok() {
        eprintln!(
            "[analyze] seeds={:?} visited={:?}",
            seeds.keys().collect::<Vec<_>>(),
            visited.keys().collect::<Vec<_>>()
        );
    }
    if std::env::var("PRISM_DBG_IMPACT").is_ok() {
        eprintln!(
            "[analyze] seeds={:?} visited={:?} queue={}",
            seeds.keys().collect::<Vec<_>>(),
            visited.keys().collect::<Vec<_>>(),
            queue.len()
        );
    }
    while let Some((current, change_type, path, hop)) = queue.pop_front() {
        // 出队即标记：防环 + 防同实体重复扩展（§59）。
        visited.insert(current.clone(), ());
        if hop >= policy.max_hops {
            metadata.truncated_by_hop_limit += 1;
            continue;
        }
        for edge in graph.edges_into(&current) {
            if std::env::var("PRISM_DBG_IMPACT").is_ok() {
                eprintln!(
                    "[impact] hop={hop} current={current} edge {} {}->{} visited_src={}",
                    edge.relation,
                    edge.source,
                    edge.target,
                    visited.contains_key(&edge.source)
                );
            }
            let source = &edge.source;
            if visited.contains_key(source) {
                // cycle / 重复到达：不重复入队；起点回到自身即环。
                if *source == current || path.iter().any(|p| p == source) {
                    metadata.cycles_detected += 1;
                }
                continue;
            }
            if affected.contains_key(source) {
                // 已按更短 hop 记录（§68 Direct 优先）——保留主路径。
                continue;
            }
            let Some((category, propagate)) = rule_for(change_type, &edge.relation) else {
                continue;
            };
            let impact = if hop + 1 == 1 {
                ImpactType::Direct
            } else {
                ImpactType::Transitive
            };
            // §63：unresolved 边不作高可信传播；hop ≥ 2 降 Medium（§64 证据质量）。
            let confidence = if edge.resolution.as_deref() == Some("unresolved") {
                Confidence::Low
            } else if hop + 1 == 1 {
                Confidence::High
            } else {
                Confidence::Medium
            };
            let mut new_path = path.clone();
            new_path.push(source.clone());
            affected.insert(
                source.clone(),
                AffectedEntity {
                    entity_id: source.clone(),
                    impact,
                    category,
                    confidence,
                    primary_path: new_path.clone(),
                    alternative_paths: 0,
                },
            );
            visited.insert(source.clone(), ());
            if propagate {
                queue.push_back((source.clone(), change_type, new_path, hop + 1));
            }
            if affected.len() >= policy.max_entities {
                metadata.truncated_by_entity_limit += 1;
                break;
            }
        }
    }

    // 名字面候选（§63）：主遍历后补录——unresolved 调用点为 Potential/Low。
    for pseudo in name_seeds.keys() {
        for edge in graph.edges_into(pseudo) {
            if affected.contains_key(&edge.source) {
                continue;
            }
            let entry = AffectedEntity {
                entity_id: edge.source.clone(),
                impact: ImpactType::Potential,
                category: rule_for(ChangeType::ContractRemoved, &edge.relation)
                    .map(|(category, _)| category)
                    .unwrap_or(ImpactCategory::Unknown),
                confidence: Confidence::Low,
                primary_path: vec![pseudo.clone(), edge.relation.clone(), edge.source.clone()],
                alternative_paths: 0,
            };
            affected.insert(edge.source.clone(), entry);
        }
    }

    let mut result_affected: Vec<AffectedEntity> = affected.into_values().collect();
    result_affected.sort_by(|a, b| {
        (
            a.impact,
            a.category,
            a.primary_path.len(),
            a.entity_id.as_str(),
        )
            .cmp(&(
                b.impact,
                b.category,
                b.primary_path.len(),
                b.entity_id.as_str(),
            ))
    });
    ImpactResult {
        affected: result_affected,
        metadata,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{EvidenceGraph, GraphEdge};

    fn graph_from(edges: &[(&str, &str, &str)]) -> EvidenceGraph {
        EvidenceGraph::from_edges(edges.iter().map(|(s, r, t)| GraphEdge {
            source: (*s).into(),
            target: (*t).into(),
            relation: (*r).into(),
            file: format!("{s}.cs"),
            line: 10,
            resolution: Some("resolved".into()),
        }))
    }

    fn ids(result: &ImpactResult) -> Vec<String> {
        result
            .affected
            .iter()
            .map(|a| a.entity_id.clone())
            .collect()
    }

    /// §70 Case 1：A CALLS B，B changed ⇒ A Direct affected。
    #[test]
    fn case1_caller_directly_affected() {
        let graph = graph_from(&[("A", "calls", "B")]);
        let changes = [EntityChange {
            entity_id: "B".into(),
            change_type: ChangeType::SignatureChanged,
        }];
        let result = analyze(&graph, &changes, &ImpactPolicy::default());
        assert_eq!(ids(&result), vec!["A".to_string()]);
        assert_eq!(result.affected[0].impact, ImpactType::Direct);
        assert_eq!(result.affected[0].category, ImpactCategory::Breaking);
    }

    /// §70 Case 2：B 仅实现体变化 ⇒ A Behavioral，不自动 Breaking。
    #[test]
    fn case2_body_only_change_is_behavioral() {
        let graph = graph_from(&[("A", "calls", "B")]);
        let changes = [EntityChange {
            entity_id: "B".into(),
            change_type: ChangeType::BodyChanged,
        }];
        let result = analyze(&graph, &changes, &ImpactPolicy::default());
        assert_eq!(result.affected[0].category, ImpactCategory::Behavioral);
        assert_ne!(result.affected[0].category, ImpactCategory::Breaking);
    }

    /// §70 Case 3：B removed ⇒ A Breaking。
    #[test]
    fn case3_removal_is_breaking() {
        let graph = graph_from(&[("A", "calls", "B")]);
        let changes = [EntityChange {
            entity_id: "B".into(),
            change_type: ChangeType::ContractRemoved,
        }];
        let result = analyze(&graph, &changes, &ImpactPolicy::default());
        assert_eq!(result.affected[0].category, ImpactCategory::Breaking);
    }

    /// §70 Case 4 + §21：Base ← Derived ← Special 传递链，hop 区分
    /// Direct/Transitive，且 Derived 与 Special 都出现。
    #[test]
    fn case4_inheritance_chain_transitive() {
        let graph = graph_from(&[
            ("Player", "inherits", "BaseCharacter"),
            ("SpecialPlayer", "inherits", "Player"),
        ]);
        let changes = [EntityChange {
            entity_id: "BaseCharacter".into(),
            change_type: ChangeType::BaseOrInterfaceChanged,
        }];
        let result = analyze(&graph, &changes, &ImpactPolicy::default());
        assert_eq!(
            ids(&result),
            vec!["Player".to_string(), "SpecialPlayer".to_string()]
        );
        assert_eq!(result.affected[0].impact, ImpactType::Direct);
        assert_eq!(result.affected[1].impact, ImpactType::Transitive);
    }

    /// §70 Case 5：Interface 签名变化 ⇒ 实现者 Behavioral affected。
    #[test]
    fn case5_interface_change_reaches_implementers() {
        let graph = graph_from(&[("Player", "implements", "ICombatant")]);
        let changes = [EntityChange {
            entity_id: "ICombatant".into(),
            change_type: ChangeType::SignatureChanged,
        }];
        let result = analyze(&graph, &changes, &ImpactPolicy::default());
        assert_eq!(ids(&result), vec!["Player".to_string()]);
        assert_eq!(result.affected[0].category, ImpactCategory::Behavioral);
    }

    /// §59/§70 Case 10：A → B → C → A 环——遍历终止，不无限循环。
    #[test]
    fn cycle_terminates_and_reports() {
        let graph = graph_from(&[
            ("A", "calls", "B"),
            ("B", "calls", "C"),
            ("C", "calls", "A"),
        ]);
        let changes = [EntityChange {
            entity_id: "B".into(),
            change_type: ChangeType::SignatureChanged,
        }];
        let result = analyze(&graph, &changes, &ImpactPolicy::default());
        assert!(
            result.metadata.cycles_detected > 0,
            "cycle must be recorded"
        );
        // B→C→A 且 A→B：A/C 都出现一次（去重）。
        let ids_now = ids(&result);
        assert!(ids_now.contains(&"A".to_string()));
        assert!(ids_now.contains(&"C".to_string()));
        assert_eq!(
            ids_now.len(),
            ids_now
                .iter()
                .collect::<std::collections::BTreeSet<_>>()
                .len()
        );
    }

    /// §33/§34：max_hops 有界 + 传递链 Medium（§64）。
    #[test]
    fn bounded_traversal_and_transitive_confidence() {
        // A calls B；C calls A ⇒ B changed：A Direct(1 hop)，C Transitive(2 hops)。
        let graph = graph_from(&[("A", "calls", "B"), ("C", "calls", "A")]);
        let policy = ImpactPolicy {
            max_hops: 2,
            ..ImpactPolicy::default()
        };
        let changes = [EntityChange {
            entity_id: "B".into(),
            change_type: ChangeType::SignatureChanged,
        }];
        let result = analyze(&graph, &changes, &policy);
        assert_eq!(ids(&result), vec!["A".to_string(), "C".to_string()]);
        let a = result.affected.iter().find(|x| x.entity_id == "A").unwrap();
        let c = result.affected.iter().find(|x| x.entity_id == "C").unwrap();
        assert_eq!(a.impact, ImpactType::Direct);
        assert_eq!(a.confidence, Confidence::High);
        assert_eq!(c.impact, ImpactType::Transitive);
        assert_eq!(c.confidence, Confidence::Medium, "多跳 ⇒ Medium（§64）");
    }

    /// §63：unresolved 边不作高可信传播 ⇒ Low Confidence。
    #[test]
    fn unresolved_edge_lowers_confidence() {
        let mut graph = graph_from(&[("C", "calls", "D")]);
        graph.edges[0].resolution = Some("unresolved".into());
        let changes = [EntityChange {
            entity_id: "D".into(),
            change_type: ChangeType::SignatureChanged,
        }];
        let result = analyze(&graph, &changes, &ImpactPolicy::default());
        let c = result.affected.iter().find(|x| x.entity_id == "C").unwrap();
        assert_eq!(c.confidence, Confidence::Low, "unresolved 边 ⇒ Low（§63）");
    }

    /// §65：max_entities 截断 + 元数据记录（§57 资源耗尽防护）。
    #[test]
    fn entity_limit_truncates_and_records() {
        // 3 个直接 caller + 上限 2 ⇒ 截断 1。
        let graph = graph_from(&[
            ("X1", "calls", "T"),
            ("X2", "calls", "T"),
            ("X3", "calls", "T"),
        ]);
        let policy = ImpactPolicy {
            max_entities: 2,
            ..ImpactPolicy::default()
        };
        let changes = [EntityChange {
            entity_id: "T".into(),
            change_type: ChangeType::SignatureChanged,
        }];
        let result = analyze(&graph, &changes, &policy);
        assert_eq!(result.affected.len(), 2);
        assert_eq!(result.metadata.truncated_by_entity_limit, 1);
    }

    /// §58：同输入两遍 ⇒ 相同结果（确定性）。
    #[test]
    fn deterministic_output() {
        let graph = graph_from(&[("A", "calls", "B"), ("B", "calls", "C")]);
        let changes = [EntityChange {
            entity_id: "B".into(),
            change_type: ChangeType::BodyChanged,
        }];
        let r1 = analyze(&graph, &changes, &ImpactPolicy::default());
        let r2 = analyze(&graph, &changes, &ImpactPolicy::default());
        assert_eq!(r1, r2);
    }
}
