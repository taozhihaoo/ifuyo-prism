//! Impact 投影（M212 §21/§22）：M26 ImpactResult → 节点/路径边。
//! Direct/Transitive/Potential 分层状态由 ImpactType 驱动；
//! primary_path 链成为逐跳边（§22 Path Detail 可回溯）。

use crate::model::{
    edge_id, node_id, VizEdge, VizGraph, VizLegendEntry, VizLimits, VizNode, VizNodeKind, VizStatus,
};
use prism_intel::{ImpactResult, ImpactType};

/// Impact 投影（§21：保留 ImpactType/Path hops——不只显示 Affected: N）。
pub fn impact_graph(analysis: &ImpactResult, limits: &VizLimits) -> VizGraph {
    let mut graph = VizGraph::new("impact");
    for affected in &analysis.affected {
        let kind = VizNodeKind::Symbol;
        let status = match affected.impact {
            ImpactType::Direct | ImpactType::Transitive => VizStatus::Complete,
            ImpactType::Potential => VizStatus::Partial,
            _ => VizStatus::Unknown,
        };
        graph.nodes.push(VizNode {
            id: node_id(kind, &affected.entity_id),
            entity_id: affected.entity_id.clone(),
            kind,
            label: affected.entity_id.clone(),
            parent: None,
            status,
            metadata: [
                (
                    "impact".to_string(),
                    format!("{:?}", affected.impact).to_lowercase(),
                ),
                (
                    "category".to_string(),
                    format!("{:?}", affected.category).to_lowercase(),
                ),
                (
                    "confidence".to_string(),
                    format!("{:?}", affected.confidence).to_lowercase(),
                ),
                (
                    "path_hops".to_string(),
                    affected.primary_path.len().saturating_sub(1).to_string(),
                ),
            ]
            .into_iter()
            .collect(),
        });
        // 路径边（§22：primary_path 链 → 逐跳边，target_entity 记归属）。
        for hop in affected.primary_path.windows(2) {
            graph.edges.push(VizEdge {
                id: edge_id("impact_path", &hop[0], &hop[1]),
                relation: "impact_path".to_string(),
                source: node_id(kind, &hop[0]),
                target: node_id(kind, &hop[1]),
                status: VizStatus::Complete,
                metadata: [("target_entity".to_string(), affected.entity_id.clone())]
                    .into_iter()
                    .collect(),
            });
        }
    }
    graph.legend = [
        ("direct", "directly affected (1 hop)"),
        ("transitive", "transitively affected"),
        ("potential", "potential (unresolved evidence)"),
        ("impact_path", "primary path hop"),
    ]
    .iter()
    .map(|(key, description)| VizLegendEntry {
        key: key.to_string(),
        description: description.to_string(),
    })
    .collect();
    graph.finalize(limits)
}

#[cfg(test)]
mod tests {
    use super::*;
    use prism_intel::{AffectedEntity, Confidence, ImpactCategory, TraversalMetadata};

    fn affected(entity: &str, impact: ImpactType) -> AffectedEntity {
        AffectedEntity {
            entity_id: entity.to_string(),
            impact,
            category: ImpactCategory::Behavioral,
            confidence: Confidence::High,
            primary_path: vec!["cs:Changed".to_string(), entity.to_string()],
            alternative_paths: 0,
        }
    }

    #[test]
    fn impact_types_drive_status_and_paths_become_edges() {
        let analysis = ImpactResult {
            affected: vec![
                affected("cs:Caller", ImpactType::Direct),
                affected("cs:Far", ImpactType::Potential),
            ],
            metadata: TraversalMetadata::default(),
        };
        let graph = impact_graph(&analysis, &VizLimits::default());
        let far = graph
            .nodes
            .iter()
            .find(|node| node.entity_id == "cs:Far")
            .unwrap();
        assert_eq!(far.status, VizStatus::Partial, "Potential ⇒ Partial（§46）");
        assert!(graph
            .edges
            .iter()
            .any(|edge| edge.relation == "impact_path"
                && edge.metadata.contains_key("target_entity")));
    }

    #[test]
    fn empty_impact_is_unknown() {
        let analysis = ImpactResult::default();
        let graph = impact_graph(&analysis, &VizLimits::default());
        assert_eq!(graph.status, VizStatus::Unknown);
    }
}
