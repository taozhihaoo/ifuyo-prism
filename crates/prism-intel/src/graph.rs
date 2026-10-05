//! Evidence Graph（2.0-M26 §七）：统一反向关系索引。
//!
//! 不是第二套图模型——把 IntelBundle/M25/M24 的关系**摄入**为
//! `(target → edges)` 反向索引 + 正向索引，Impact 遍历只消费本索引。
//! 来源关系（M25 CALLS/INHERITS...、M24 attached_script/references_scene...
//! ）经 `push` 以字符串 relation 注册；BTreeMap 保证确定性。

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// 图边（来源无关——M25 语义关系、M24 Godot 关系、M22 结构关系同形）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphEdge {
    /// Entity id（各里程碑 EntityId 原样——同口径 id 跨域链接）。
    pub source: String,
    pub target: String,
    /// 关系名（calls/inherits/attached_script/references_scene/...）。
    pub relation: String,
    /// 证据定位（file:line 原文）。
    pub file: String,
    pub line: u32,
    /// 解析状态标注（resolved/unresolved/external；§63 unresolved 不作
    /// 高可信传播——遍历仍经过，但 ImpactType 降级为 Potential）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolution: Option<String>,
}

/// Evidence Graph：正向 + 反向索引（确定性遍历）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EvidenceGraph {
    pub edges: Vec<GraphEdge>,
    /// target → 边下标（Impact 反向传播方向：B changed ⇒ 谁⇒B 的边）。
    reverse: BTreeMap<String, Vec<usize>>,
    /// source → 边下标（正向查询 callers-of 方向的补集场景）。
    forward: BTreeMap<String, Vec<usize>>,
}

impl EvidenceGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, edge: GraphEdge) {
        self.edges.push(edge);
        self.reindex_last();
    }

    fn reindex_last(&mut self) {
        let index = self.edges.len() - 1;
        let edge = &self.edges[index];
        self.reverse
            .entry(edge.target.clone())
            .or_default()
            .push(index);
        self.forward
            .entry(edge.source.clone())
            .or_default()
            .push(index);
    }

    /// 从既有边集合构建（一次性摄入）。
    pub fn from_edges(edges: impl IntoIterator<Item = GraphEdge>) -> Self {
        let mut graph = Self::new();
        for edge in edges {
            graph.push(edge);
        }
        graph
    }

    /// 反向查询：所有 **指向** `entity` 的边（B changed ⇒ 查这些边）。
    pub fn edges_into(&self, entity: &str) -> Vec<&GraphEdge> {
        self.reverse
            .get(entity)
            .map(|indexes| indexes.iter().map(|i| &self.edges[*i]).collect())
            .unwrap_or_default()
    }

    /// 正向查询：`entity` 发出的边。
    pub fn edges_from(&self, entity: &str) -> Vec<&GraphEdge> {
        self.forward
            .get(entity)
            .map(|indexes| indexes.iter().map(|i| &self.edges[*i]).collect())
            .unwrap_or_default()
    }

    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn edge(source: &str, relation: &str, target: &str) -> GraphEdge {
        GraphEdge {
            source: source.into(),
            target: target.into(),
            relation: relation.into(),
            file: "f.cs".into(),
            line: 1,
            resolution: Some("resolved".into()),
        }
    }

    #[test]
    fn reverse_lookup_finds_callers() {
        let mut graph = EvidenceGraph::new();
        graph.push(edge("A", "calls", "B"));
        graph.push(edge("C", "calls", "B"));
        graph.push(edge("B", "calls", "D"));
        let into_b = graph.edges_into("B");
        assert_eq!(into_b.len(), 2, "A→B 与 C→B 均为反向命中");
        assert!(graph.edges_into("D").iter().all(|e| e.source == "B"));
        assert!(graph.edges_into("A").is_empty());
    }

    #[test]
    fn iteration_is_deterministic() {
        let mut graph = EvidenceGraph::new();
        for name in ["Z", "A", "M"] {
            graph.push(edge(name, "calls", "T"));
        }
        let sources: Vec<&str> = graph
            .edges_into("T")
            .iter()
            .map(|e| e.source.as_str())
            .collect();
        // 确定性 = 同输入序列 ⇒ 同输出（§58）；本索引保持插入序。
        assert_eq!(sources, vec!["Z", "A", "M"], "插入序稳定可复现");
    }
}
