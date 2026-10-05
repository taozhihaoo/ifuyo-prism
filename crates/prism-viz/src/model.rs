//! Visualization View Model（M212 §5-§7/§43-§47）。
//!
//! 投影模型——**不是**第二套事实模型：`VizNode.entity_id` 携带原 Core
//! 身份；颜色/图例语义由 `kind`/`status` 驱动（§45 统一 Visual Token）。

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// M212 投影契约版本（§86 Data Contract）。
pub const VIZ_SCHEMA_VERSION: u32 = 1;

/// 节点种类（§45：颜色/形状按 kind 有明确语义，跨 View 一致）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VizNodeKind {
    Project,
    Directory,
    File,
    Symbol,
    Package,
    Scene,
    GodotNode,
    Script,
    Resource,
    Unknown,
}

impl VizNodeKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            VizNodeKind::Project => "project",
            VizNodeKind::Directory => "directory",
            VizNodeKind::File => "file",
            VizNodeKind::Symbol => "symbol",
            VizNodeKind::Package => "package",
            VizNodeKind::Scene => "scene",
            VizNodeKind::GodotNode => "godot_node",
            VizNodeKind::Script => "script",
            VizNodeKind::Resource => "resource",
            VizNodeKind::Unknown => "unknown",
        }
    }
}

/// 节点/图状态（§46：Unknown/Partial 不得画成正常）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VizStatus {
    Complete,
    Partial,
    Unknown,
    Unavailable,
}

/// 图节点（§6）：`entity_id` = 原 Core 稳定身份，非随机 GUID。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VizNode {
    /// 投影内稳定 id（`viz:{kind}:{entity}`）。
    pub id: String,
    /// 原 Core 身份（entity/路径/包名——可回溯，§4 Evidence-first）。
    pub entity_id: String,
    pub kind: VizNodeKind,
    pub label: String,
    /// 层级父节点 id（Architecture 层级聚合 §9；Godot scene→node §19）。
    pub parent: Option<String>,
    pub status: VizStatus,
    /// 附加事实（原样透传，不渲染端分析）。
    pub metadata: BTreeMap<String, String>,
}

/// 图边（§7）：必须来自真实 Relation/Dependency/Signal，不猜测。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VizEdge {
    /// `edge:{relation}:{source}#{target}`。
    pub id: String,
    /// 关系种类（§47 视觉区分 + §48 图例）。
    pub relation: String,
    pub source: String,
    pub target: String,
    pub status: VizStatus,
    pub metadata: BTreeMap<String, String>,
}

/// 图例（§48：线型/颜色语义的解释数据，随投影输出）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VizLegendEntry {
    pub key: String,
    pub description: String,
}

/// 投影上限（§28/§49 爆炸防护；超出 ⇒ truncated + 诊断）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VizLimits {
    pub max_nodes: usize,
    pub max_edges: usize,
}

impl Default for VizLimits {
    fn default() -> Self {
        VizLimits {
            max_nodes: 500,
            max_edges: 2000,
        }
    }
}

/// 投影结果（§5 View Model）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VizGraph {
    pub schema_version: u32,
    /// 投影视图 id（godot/architecture/dependency/impact）。
    pub view: String,
    pub status: VizStatus,
    /// 按 (kind, label, id) 稳定排序（§44）。
    pub nodes: Vec<VizNode>,
    /// 按 (relation, source, target) 稳定排序。
    pub edges: Vec<VizEdge>,
    pub legend: Vec<VizLegendEntry>,
    /// 上限触发了截断（§28 明确提示，不静默）。
    pub truncated: bool,
    pub diagnostics: Vec<String>,
}

impl VizGraph {
    pub fn new(view: &str) -> Self {
        VizGraph {
            schema_version: VIZ_SCHEMA_VERSION,
            view: view.to_string(),
            status: VizStatus::Complete,
            nodes: Vec::new(),
            edges: Vec::new(),
            legend: Vec::new(),
            truncated: false,
            diagnostics: Vec::new(),
        }
    }

    /// 去重 + 确定性排序 + 截断标记（所有投影共用的收尾，§43/§44）。
    pub fn finalize(mut self, limits: &VizLimits) -> Self {
        self.nodes.sort_by(|a, b| {
            a.kind
                .cmp(&b.kind)
                .then_with(|| a.label.cmp(&b.label))
                .then_with(|| a.id.cmp(&b.id))
        });
        self.nodes.dedup_by(|a, b| a.id == b.id);
        self.edges.sort_by(|a, b| {
            a.relation
                .cmp(&b.relation)
                .then_with(|| a.source.cmp(&b.source))
                .then_with(|| a.target.cmp(&b.target))
        });
        self.edges.dedup_by(|a, b| a.id == b.id);
        if self.nodes.len() > limits.max_nodes {
            self.nodes.truncate(limits.max_nodes);
            self.truncated = true;
            self.diagnostics.push(format!(
                "node cap {} reached; view truncated",
                limits.max_nodes
            ));
        }
        if self.edges.len() > limits.max_edges {
            self.edges.truncate(limits.max_edges);
            self.truncated = true;
            self.diagnostics.push(format!(
                "edge cap {} reached; view truncated",
                limits.max_edges
            ));
        }
        if self.nodes.is_empty() && self.status == VizStatus::Complete {
            self.status = VizStatus::Unknown;
            self.diagnostics
                .push("projection produced no nodes; data unavailable".to_string());
        }
        self.legend.sort_by(|a, b| a.key.cmp(&b.key));
        self
    }
}

/// 便捷：节点 id 构造。
pub(crate) fn node_id(kind: VizNodeKind, entity: &str) -> String {
    format!("viz:{}:{}", kind.as_str(), entity)
}

/// 便捷：边 id 构造。
pub(crate) fn edge_id(relation: &str, source: &str, target: &str) -> String {
    format!("edge:{relation}:{source}#{target}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finalize_sorts_dedups_and_flags_truncation() {
        let limits = VizLimits {
            max_nodes: 2,
            max_edges: 1,
        };
        let graph = VizGraph {
            schema_version: 1,
            view: "test".to_string(),
            status: VizStatus::Complete,
            nodes: vec![
                VizNode {
                    id: node_id(VizNodeKind::File, "b.rs"),
                    entity_id: "b.rs".to_string(),
                    kind: VizNodeKind::File,
                    label: "b".to_string(),
                    parent: None,
                    status: VizStatus::Complete,
                    metadata: BTreeMap::new(),
                },
                VizNode {
                    id: node_id(VizNodeKind::File, "a.rs"),
                    entity_id: "a.rs".to_string(),
                    kind: VizNodeKind::File,
                    label: "a".to_string(),
                    parent: None,
                    status: VizStatus::Complete,
                    metadata: BTreeMap::new(),
                },
                VizNode {
                    id: node_id(VizNodeKind::File, "c.rs"),
                    entity_id: "c.rs".to_string(),
                    kind: VizNodeKind::File,
                    label: "c".to_string(),
                    parent: None,
                    status: VizStatus::Complete,
                    metadata: BTreeMap::new(),
                },
            ],
            edges: vec![
                VizEdge {
                    id: edge_id("calls", "a", "b"),
                    relation: "calls".to_string(),
                    source: node_id(VizNodeKind::File, "a.rs"),
                    target: node_id(VizNodeKind::File, "b.rs"),
                    status: VizStatus::Complete,
                    metadata: BTreeMap::new(),
                },
                VizEdge {
                    id: edge_id("calls", "a", "b"),
                    relation: "calls".to_string(),
                    source: node_id(VizNodeKind::File, "a.rs"),
                    target: node_id(VizNodeKind::File, "b.rs"),
                    status: VizStatus::Complete,
                    metadata: BTreeMap::new(),
                },
            ],
            legend: Vec::new(),
            truncated: false,
            diagnostics: Vec::new(),
        }
        .finalize(&limits);
        assert_eq!(graph.nodes.len(), 2, "cap 截断");
        assert!(graph.truncated);
        assert!(graph.diagnostics.iter().any(|d| d.contains("truncated")));
        assert_eq!(graph.edges.len(), 1, "重复边去重");
        assert_eq!(graph.nodes[0].label, "a", "按 label 稳定排序");
    }

    #[test]
    fn empty_projection_is_unknown_not_complete() {
        // §46：空数据不得伪装成正常。
        let graph = VizGraph::new("godot").finalize(&VizLimits::default());
        assert_eq!(graph.status, VizStatus::Unknown);
        assert!(graph.diagnostics.iter().any(|d| d.contains("unavailable")));
    }
}
