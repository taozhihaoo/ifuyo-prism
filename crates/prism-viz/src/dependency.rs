//! Dependency 投影（M212 §10/§11/§25）：Project → Package 层。
//! Direct/Transitive/Unresolved/Unknown-license 状态来自 M29 事实；
//! Declared ≠ Resolved 用节点状态表达（§10 必须可分辨）。

use crate::model::{
    edge_id, node_id, VizEdge, VizGraph, VizLegendEntry, VizLimits, VizNode, VizNodeKind, VizStatus,
};
use prism_supply::SupplyReport;
use std::collections::BTreeMap;

/// Dependency 投影（§10：Project → Direct → Transitive 链）。
pub fn dependency_graph(supply: &SupplyReport, limits: &VizLimits) -> VizGraph {
    let mut graph = VizGraph::new("dependency");
    let project_id = node_id(VizNodeKind::Project, "project");
    graph.nodes.push(VizNode {
        id: project_id.clone(),
        entity_id: "project".to_string(),
        kind: VizNodeKind::Project,
        label: "Project".to_string(),
        parent: None,
        status: VizStatus::Complete,
        metadata: Default::default(),
    });
    let licenses: BTreeMap<String, String> = supply
        .licenses
        .iter()
        .map(|fact| (fact.subject.clone(), fact.expression.clone()))
        .collect();
    for declaration in &supply.declarations {
        let status = if declaration.requested_version.is_some() {
            VizStatus::Complete
        } else {
            VizStatus::Unknown
        };
        let package_id = node_id(VizNodeKind::Package, &declaration.id);
        let mut metadata = BTreeMap::new();
        metadata.insert(
            "requested_version".to_string(),
            declaration
                .requested_version
                .clone()
                .unwrap_or_else(|| "unspecified".to_string()),
        );
        metadata.insert("manifest".to_string(), declaration.manifest_path.clone());
        if let Some(existing) = graph.nodes.iter_mut().find(|node| node.id == package_id) {
            existing.status = status;
            for (key, value) in metadata.clone() {
                existing.metadata.insert(key, value);
            }
        } else {
            graph.nodes.push(VizNode {
                id: package_id.clone(),
                entity_id: declaration.id.clone(),
                kind: VizNodeKind::Package,
                label: declaration.package_id.clone(),
                parent: Some(project_id.clone()),
                status,
                metadata: metadata.clone(),
            });
        }
        graph.edges.push(VizEdge {
            id: edge_id("declares", &project_id, &package_id),
            relation: "declares".to_string(),
            source: project_id.clone(),
            target: package_id.clone(),
            status,
            metadata: metadata.clone(),
        });
    }
    for resolved in &supply.resolved {
        let package_id = node_id(VizNodeKind::Package, &resolved.id);
        let version = resolved
            .resolved_version
            .clone()
            .unwrap_or_else(|| "unknown".to_string());
        let license = licenses
            .get(&resolved.id)
            .cloned()
            .unwrap_or_else(|| "UNKNOWN".to_string());
        let mut metadata = BTreeMap::new();
        metadata.insert("resolved_version".to_string(), version);
        metadata.insert(
            "relationship".to_string(),
            if resolved.direct {
                "direct".to_string()
            } else {
                "transitive".to_string()
            },
        );
        metadata.insert("license".to_string(), license.clone());
        if license == "UNKNOWN" {
            // §21 License Unknown：UNKNOWN ≠ 没有 license。
            metadata.insert(
                "license_state".to_string(),
                "unknown (not no-license)".to_string(),
            );
        }
        let mut node_status = VizStatus::Complete;
        if resolved.resolved_version.is_none() {
            node_status = VizStatus::Unknown;
        }
        if let Some(existing) = graph.nodes.iter_mut().find(|node| node.id == package_id) {
            existing.status = node_status;
            for (key, value) in metadata {
                existing.metadata.insert(key, value);
            }
        } else {
            // Transitive（声明中不存在）⇒ 补节点（§11 Transitive 展开）。
            graph.nodes.push(VizNode {
                id: package_id.clone(),
                entity_id: resolved.id.clone(),
                kind: VizNodeKind::Package,
                label: resolved.package_id.clone(),
                parent: Some(project_id.clone()),
                status: node_status,
                metadata,
            });
        }
    }
    if supply.declarations.is_empty() && supply.resolved.is_empty() {
        // §46：无依赖数据不得画成正常空图。
        graph.status = VizStatus::Unknown;
        graph
            .diagnostics
            .push("no dependency declarations/resolved data injected".to_string());
    }
    graph.legend = vec![
        VizLegendEntry {
            key: "package".to_string(),
            description: "dependency package (declared or resolved)".to_string(),
        },
        VizLegendEntry {
            key: "declares".to_string(),
            description: "project declares package".to_string(),
        },
        VizLegendEntry {
            key: "unknown_state".to_string(),
            description: "unresolved version / unknown license — shown as Unknown, not healthy"
                .to_string(),
        },
    ];
    graph.finalize(limits)
}

#[cfg(test)]
mod tests {
    use super::*;
    use prism_supply::{
        DependencyDeclaration, DependencyKind, DependencySource, ResolvedDependency, SupplyReport,
    };

    fn declaration(id: &str, package: &str, version: Option<&str>) -> DependencyDeclaration {
        DependencyDeclaration {
            id: id.to_string(),
            ecosystem: prism_supply::Ecosystem::NuGet,
            package_id: package.to_string(),
            requested_version: version.map(str::to_string),
            version_is_range: false,
            kind: DependencyKind::PackageReference,
            source: DependencySource::NuGet,
            scope: "direct".to_string(),
            manifest_path: "src/App.csproj".to_string(),
            line: Some(1),
            version_source: prism_supply::model::VersionSource::Manifest,
            origin: prism_intel::Origin::Fact,
            evidence: Vec::new(),
        }
    }

    fn resolved(id: &str, package: &str, version: &str, direct: bool) -> ResolvedDependency {
        ResolvedDependency {
            id: id.to_string(),
            ecosystem: prism_supply::Ecosystem::NuGet,
            package_id: package.to_string(),
            resolved_version: if version.is_empty() {
                None
            } else {
                Some(version.to_string())
            },
            direct,
            from_lockfile: true,
            lockfile_path: "packages.lock.json".to_string(),
            evidence: Vec::new(),
        }
    }

    #[test]
    fn direct_transitive_and_unknown_states() {
        let supply = SupplyReport {
            schema_version: 1,
            declarations: vec![declaration("nuget:A", "A", Some("1.0.0"))],
            resolved: vec![
                resolved("nuget:A", "A", "1.0.0", true),
                resolved("nuget:T", "T", "2.0.0", false),
                resolved("nuget:U", "U", "", true),
            ],
            ..SupplyReport::default()
        };
        let graph = dependency_graph(&supply, &VizLimits::default());
        assert_eq!(graph.nodes.len(), 4, "project + A + T + U");
        let by_label = |label: &str| graph.nodes.iter().find(|node| node.label == label).unwrap();
        assert_eq!(by_label("A").status, VizStatus::Complete);
        assert!(by_label("A").metadata.contains_key("license"));
        assert_eq!(
            by_label("T").status,
            VizStatus::Complete,
            "transitive 同样 Complete"
        );
        assert_eq!(
            by_label("U").status,
            VizStatus::Unknown,
            "无解析版本 ⇒ Unknown（不是 0）"
        );
    }

    #[test]
    fn empty_supply_is_unknown() {
        let graph = dependency_graph(&SupplyReport::default(), &VizLimits::default());
        assert_eq!(graph.status, VizStatus::Unknown);
    }
}
