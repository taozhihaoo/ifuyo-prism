//! Godot 跨层投影（M212 §18-§20 第一优先 Vertical Slice）。
//!
//! 链路：`Project → Scene → GodotNode → Script → Symbol`（经 M24
//! GodotIndex attached_script 证据 + M23 by_file 类型）；signal 连接为边
//! （§19）；缺符号索引/无类型 ⇒ Unknown 节点（§46）。全部来自 M24/M23
//! 既有数据——不重新解析场景（§18）。

use crate::model::{
    edge_id, node_id, VizEdge, VizGraph, VizLegendEntry, VizLimits, VizNode, VizNodeKind, VizStatus,
};
use prism_csharp::SymbolIndex;
use prism_godot::{GodotAnalysis, GodotIndex};
use std::collections::BTreeMap;

/// Godot 跨层投影。
///
/// * `analysis` — M24 Adapter 产物（scenes/nodes/scripts）。
/// * `symbols`  — M23 索引（Some 时挂 script→type 层；None ⇒ 该层 Unknown，
///   如实，不伪造）。
pub fn godot_graph(
    analysis: &GodotAnalysis,
    symbols: Option<&SymbolIndex>,
    limits: &VizLimits,
) -> VizGraph {
    let mut graph = VizGraph::new("godot");
    let index = GodotIndex::build(analysis);
    let project_id = node_id(VizNodeKind::Project, "godot-project");
    if index.scenes.is_empty() {
        graph
            .diagnostics
            .push("no godot scenes in injected analysis".to_string());
        return graph.finalize(limits);
    }
    graph.nodes.push(VizNode {
        id: project_id.clone(),
        entity_id: "godot-project".to_string(),
        kind: VizNodeKind::Project,
        label: "Godot Project".to_string(),
        parent: None,
        status: VizStatus::Complete,
        metadata: Default::default(),
    });

    // script 路径 → C# 类型层（一次性构建，跨场景复用）。
    let mut script_types: BTreeMap<String, Vec<(String, String, String)>> = BTreeMap::new();
    if let Some(index) = symbols {
        for (script_relative, ids) in &index.by_file {
            for symbol_id in ids.iter().take(3) {
                if let Some(symbol) = index.by_id.get(symbol_id) {
                    script_types
                        .entry(script_relative.clone())
                        .or_default()
                        .push((
                            symbol.id.clone(),
                            symbol.qualified_name.clone(),
                            symbol.display_name.clone(),
                        ));
                }
            }
        }
    }

    let mut links: Vec<(String, String)> = Vec::new();
    // node_scripts: NodePath → script 相对路径；node_paths: NodePath → scene。
    let mut node_scripts_sorted: Vec<(&String, &String)> = index.node_scripts.iter().collect();
    node_scripts_sorted.sort_by(|a, b| a.0.cmp(b.0));

    for (scene_path, document) in &index.scenes {
        let scene_id = node_id(VizNodeKind::Scene, scene_path);
        links.push((project_id.clone(), scene_id.clone()));
        graph.nodes.push(VizNode {
            id: scene_id.clone(),
            entity_id: scene_path.clone(),
            kind: VizNodeKind::Scene,
            label: scene_path.clone(),
            parent: Some(project_id.clone()),
            status: if document.well_formed {
                VizStatus::Complete
            } else {
                VizStatus::Partial
            },
            metadata: Default::default(),
        });
        // ext_resource（script/resource）节点；悬挂引用 ⇒ Unknown（§46）。
        for resource in &document.ext_resources {
            let is_script = resource.resource_type.eq_ignore_ascii_case("Script");
            let kind = if is_script {
                VizNodeKind::Script
            } else {
                VizNodeKind::Resource
            };
            let res_relative = prism_godot::res_to_relative(&resource.path);
            let resource_id = node_id(kind, &res_relative);
            graph.nodes.push(VizNode {
                id: resource_id.clone(),
                entity_id: res_relative.clone(),
                kind,
                label: res_relative.clone(),
                parent: Some(scene_id.clone()),
                status: VizStatus::Complete,
                metadata: Default::default(),
            });
            links.push((scene_id.clone(), resource_id.clone()));
        }
        // signal 连接（§19：Node → Target Node）。
        for connection in &document.connections {
            let from_path = resolve_connection_path(&connection.from, scene_path, &index);
            let to_path = resolve_connection_path(&connection.to, scene_path, &index);
            graph.edges.push(VizEdge {
                id: edge_id("signal", &from_path, &to_path),
                relation: "signal".to_string(),
                source: node_id(VizNodeKind::GodotNode, &from_path),
                target: node_id(VizNodeKind::GodotNode, &to_path),
                status: VizStatus::Complete,
                metadata: [
                    ("signal".to_string(), connection.signal.clone()),
                    (
                        "method".to_string(),
                        connection.method.clone().unwrap_or_default(),
                    ),
                ]
                .into_iter()
                .collect(),
            });
        }
    }
    // node 层 + script 层 + C# 类型层（跨层 §20）。
    let mut scripted_nodes: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (node_path, script_relative) in &index.node_scripts {
        scripted_nodes
            .entry(script_relative.clone())
            .or_default()
            .push(node_path.clone());
        let node_id_value = node_id(VizNodeKind::GodotNode, node_path);
        let scene = index.node_paths.get(node_path).cloned().unwrap_or_default();
        graph.nodes.push(VizNode {
            id: node_id_value.clone(),
            entity_id: node_path.clone(),
            kind: VizNodeKind::GodotNode,
            label: node_path.clone(),
            parent: Some(node_id(VizNodeKind::Scene, &scene)),
            status: VizStatus::Complete,
            metadata: Default::default(),
        });
        let script_id = node_id(VizNodeKind::Script, script_relative);
        links.push((node_id_value, script_id.clone()));
        // script → C# type。
        match script_types.get(script_relative) {
            Some(types) if !types.is_empty() => {
                for (symbol_id, qualified_name, display_name) in types {
                    let type_id = node_id(VizNodeKind::Symbol, qualified_name);
                    graph.nodes.push(VizNode {
                        id: type_id.clone(),
                        entity_id: symbol_id.clone(),
                        kind: VizNodeKind::Symbol,
                        label: display_name.clone(),
                        parent: Some(script_id.clone()),
                        status: VizStatus::Complete,
                        metadata: Default::default(),
                    });
                    links.push((script_id.clone(), type_id));
                }
            }
            _ => {
                if symbols.is_some() {
                    graph.nodes.push(VizNode {
                        id: node_id(VizNodeKind::Unknown, &format!("{script_relative}#types")),
                        entity_id: script_relative.clone(),
                        kind: VizNodeKind::Unknown,
                        label: format!("{script_relative} (no M23 symbols)"),
                        parent: Some(script_id),
                        status: VizStatus::Unknown,
                        metadata: Default::default(),
                    });
                } else {
                    graph.nodes.push(VizNode {
                        id: node_id(VizNodeKind::Unknown, &format!("{script_relative}#symbols")),
                        entity_id: script_relative.clone(),
                        kind: VizNodeKind::Unknown,
                        label: format!("{script_relative} (symbol index unavailable)"),
                        parent: Some(script_id),
                        status: VizStatus::Unknown,
                        metadata: Default::default(),
                    });
                }
                graph.status = VizStatus::Partial;
            }
        }
    }
    for (source, target) in links {
        graph.edges.push(VizEdge {
            id: edge_id("contains", &source, &target),
            relation: "contains".to_string(),
            source,
            target,
            status: VizStatus::Complete,
            metadata: Default::default(),
        });
    }
    graph.legend = [
        ("scene", "Godot .tscn scene file"),
        ("godot_node", "node inside a scene (NodePath)"),
        ("script", "attached script resource"),
        ("symbol", "C# type from M23 index"),
        ("resource", "non-script external resource"),
        ("signal", "signal connection between nodes"),
        ("contains", "hierarchy containment"),
    ]
    .iter()
    .map(|(key, description)| VizLegendEntry {
        key: key.to_string(),
        description: description.to_string(),
    })
    .collect();
    graph.finalize(limits)
}

/// connection endpoint（`.`/相对 NodePath）→ 绝对 NodePath（/Root/...）。
fn resolve_connection_path(endpoint: &str, scene_path: &str, index: &GodotIndex) -> String {
    let candidates: Vec<String> = index
        .node_paths
        .keys()
        .filter(|node_path| {
            index
                .node_paths
                .get(*node_path)
                .map(|scene| scene == scene_path)
                .unwrap_or(false)
        })
        .cloned()
        .collect();
    if endpoint == "." || endpoint.is_empty() {
        // 根节点：该场景中 parent 为 None 的节点路径。
        return candidates
            .iter()
            .find(|path| path.matches('/').count() == 1)
            .cloned()
            .or_else(|| candidates.first().cloned())
            .unwrap_or_else(|| endpoint.to_string());
    }
    // 相对路径 ⇒ 前缀匹配（/Root/xxx 以 /endpoint 或 endpoint 结尾段）。
    candidates
        .iter()
        .find(|path| path.ends_with(&format!("/{endpoint}")))
        .cloned()
        .unwrap_or_else(|| endpoint.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use prism_csharp::SymbolEngine;
    use prism_godot::ir::{ConnectionRecord, ExtResourceRecord, NodeRecord, TscnDocument};
    use prism_godot::GodotAnalysis;
    use std::collections::BTreeMap;

    fn analysis(scenes: BTreeMap<String, TscnDocument>) -> GodotAnalysis {
        GodotAnalysis {
            project: None,
            scenes,
            resources: Default::default(),
            diagnostics: vec![],
        }
    }

    fn battle_scene() -> TscnDocument {
        TscnDocument {
            format: Some("3".into()),
            ext_resources: vec![ExtResourceRecord {
                resource_type: "Script".into(),
                path: "res://Scripts/Battle.cs".into(),
                id: "1".into(),
            }],
            sub_resources: vec![],
            nodes: vec![
                NodeRecord {
                    name: "BattleRoot".into(),
                    node_type: Some("Node".into()),
                    parent: None,
                    instance_ext_resource_id: None,
                    script_ext_resource_id: Some("1".into()),
                    line: 1,
                },
                NodeRecord {
                    name: "HUD".into(),
                    node_type: Some("CanvasLayer".into()),
                    parent: Some(".".into()),
                    instance_ext_resource_id: None,
                    script_ext_resource_id: None,
                    line: 2,
                },
            ],
            connections: vec![],
            well_formed: true,
            is_resource: false,
        }
    }

    fn battle_index() -> SymbolIndex {
        let mut engine = SymbolEngine::new();
        engine.update_file(
            "Scripts/Battle.cs",
            "namespace App {\npublic class Battle {\n}\n}\n",
        );
        engine.build_index()
    }

    #[test]
    fn cross_layer_projection_scene_node_script_symbol() {
        let mut scenes = BTreeMap::new();
        scenes.insert("Scenes/Battle.tscn".to_string(), battle_scene());
        let graph = godot_graph(
            &analysis(scenes),
            Some(&battle_index()),
            &VizLimits::default(),
        );
        assert_eq!(graph.view, "godot");
        let labels: Vec<&str> = graph.nodes.iter().map(|node| node.label.as_str()).collect();
        assert!(labels.contains(&"Scenes/Battle.tscn"));
        assert!(labels.contains(&"/BattleRoot"), "node 层（NodePath）");
        assert!(labels.contains(&"Scripts/Battle.cs"));
        assert!(labels.contains(&"App.Battle"), "M23 type 层（跨层 §20）");
        // 跨层链边：script → symbol。
        assert!(graph.edges.iter().any(|edge| edge.relation == "contains"
            && edge.source.ends_with("Scripts/Battle.cs")
            && edge.target.ends_with("Battle")));
        assert_eq!(graph.status, VizStatus::Complete);
    }

    #[test]
    fn missing_symbol_index_is_partial_not_fake() {
        let mut scenes = BTreeMap::new();
        scenes.insert("Scenes/Battle.tscn".to_string(), battle_scene());
        let graph = godot_graph(&analysis(scenes), None, &VizLimits::default());
        assert_eq!(graph.status, VizStatus::Partial);
        assert!(graph
            .nodes
            .iter()
            .any(|node| node.status == VizStatus::Unknown && node.label.contains("unavailable")));
    }

    #[test]
    fn signal_connection_becomes_edge_between_nodes() {
        let mut scene = battle_scene();
        scene.connections = vec![ConnectionRecord {
            signal: "pressed".to_string(),
            from: ".".to_string(),
            to: "HUD".to_string(),
            method: Some("_on_pressed".to_string()),
            line: 3,
        }];
        let mut scenes = BTreeMap::new();
        scenes.insert("Scenes/Battle.tscn".to_string(), scene);
        let graph = godot_graph(&analysis(scenes), None, &VizLimits::default());
        let signal = graph
            .edges
            .iter()
            .find(|edge| edge.relation == "signal")
            .expect("signal edge");
        assert_eq!(
            signal.metadata.get("signal").map(String::as_str),
            Some("pressed")
        );
        assert!(signal.source.ends_with("BattleRoot"), "from=. ⇒ 根节点");
        assert!(signal.target.ends_with("HUD"));
    }

    #[test]
    fn empty_analysis_is_unknown() {
        let graph = godot_graph(&analysis(BTreeMap::new()), None, &VizLimits::default());
        assert_eq!(graph.status, VizStatus::Unknown);
    }
}
