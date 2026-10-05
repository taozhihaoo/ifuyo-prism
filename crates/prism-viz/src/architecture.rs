//! Architecture 层级投影（M212 §8/§9/§50）：Directory → File 层级 +
//! M3 文件依赖边（原样 depends_on，两端在投影内才画）。

use crate::model::{
    edge_id, node_id, VizEdge, VizGraph, VizLegendEntry, VizLimits, VizNode, VizNodeKind, VizStatus,
};
use prism_graph::DependencyGraphResult;
use std::collections::BTreeMap;

/// Architecture 投影输入（M1/M3 既有产物，零 IO 注入）。
pub struct ArchitectureInput<'a> {
    /// 项目相对文件路径（M1 discovery.files 的 Source/Test 集）。
    pub files: &'a [String],
    /// M3 依赖图结果（file 级 depends_on 边）。
    pub dependencies: &'a DependencyGraphResult,
}

/// Architecture 层级投影（§9：先目录后文件；不做全量符号图）。
pub fn architecture_graph(input: &ArchitectureInput, limits: &VizLimits) -> VizGraph {
    let mut graph = VizGraph::new("architecture");
    let project_id = node_id(VizNodeKind::Project, "project");
    graph.nodes.push(VizNode {
        id: project_id.clone(),
        entity_id: input.dependencies.project_name.clone(),
        kind: VizNodeKind::Project,
        label: input.dependencies.project_name.clone(),
        parent: None,
        status: VizStatus::Complete,
        metadata: Default::default(),
    });
    // 目录树（"src/battle" → dir 节点链；文件挂最深目录，根文件挂 project）。
    let mut dir_nodes: BTreeMap<String, String> = BTreeMap::new();
    let mut file_links: Vec<(String, String)> = Vec::new();
    for file in input.files {
        let (dir, name) = match file.rsplit_once('/') {
            Some((dir, name)) => (dir.to_string(), name.to_string()),
            None => (String::new(), file.clone()),
        };
        let mut prefix = String::new();
        let mut parent_id = project_id.clone();
        for segment in dir.split('/').filter(|s| !s.is_empty()) {
            prefix = if prefix.is_empty() {
                segment.to_string()
            } else {
                format!("{prefix}/{segment}")
            };
            let dir_id = dir_nodes.entry(prefix.clone()).or_insert_with(|| {
                let id = node_id(VizNodeKind::Directory, &prefix);
                graph.nodes.push(VizNode {
                    id: id.clone(),
                    entity_id: prefix.clone(),
                    kind: VizNodeKind::Directory,
                    label: segment.to_string(),
                    parent: Some(parent_id.clone()),
                    status: VizStatus::Complete,
                    metadata: Default::default(),
                });
                id
            });
            parent_id = dir_id.clone();
        }
        let file_id = node_id(VizNodeKind::File, file);
        graph.nodes.push(VizNode {
            id: file_id.clone(),
            entity_id: file.clone(),
            kind: VizNodeKind::File,
            label: name,
            parent: Some(parent_id.clone()),
            status: VizStatus::Complete,
            metadata: Default::default(),
        });
        file_links.push((parent_id, file_id));
    }
    for (parent, file) in file_links {
        graph.edges.push(VizEdge {
            id: edge_id("contains", &parent, &file),
            relation: "contains".to_string(),
            source: parent,
            target: file,
            status: VizStatus::Complete,
            metadata: Default::default(),
        });
    }
    // M3 file 依赖 → depends_on 边（两端均为投影内文件才画；external/
    // builtin 目标属 Dependency View 的 External 边界，§73）。
    for dependency in &input.dependencies.dependencies {
        let Some(target_path) = dependency.target_path.as_deref() else {
            continue;
        };
        let source = node_id(VizNodeKind::File, &dependency.source_path);
        let target = node_id(VizNodeKind::File, target_path);
        let exists = |id: &str| graph.nodes.iter().any(|node| node.id == id);
        if !exists(&source) || !exists(&target) || source == target {
            continue;
        }
        graph.edges.push(VizEdge {
            id: edge_id("depends_on", &dependency.source_path, target_path),
            relation: "depends_on".to_string(),
            source,
            target,
            status: if dependency.resolution == prism_graph::ResolutionStatus::Resolved {
                VizStatus::Complete
            } else {
                VizStatus::Unknown
            },
            metadata: [
                ("raw_target".to_string(), dependency.raw_target.clone()),
                (
                    "resolution".to_string(),
                    format!("{:?}", dependency.resolution).to_lowercase(),
                ),
            ]
            .into_iter()
            .collect(),
        });
    }
    graph.legend = [
        ("directory", "directory/module group"),
        ("file", "source file"),
        ("depends_on", "file-level dependency (M3)"),
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

#[cfg(test)]
mod tests {
    use super::*;
    use prism_graph::model::{Dependency, Module};

    fn dependency(source: &str, target: &str) -> Dependency {
        Dependency {
            id: format!("dep:{source}#{target}#imports"),
            source_id: prism_graph::ModuleId::from_string(format!("file:{source}")),
            source_path: source.to_string(),
            raw_target: target.to_string(),
            kind: prism_graph::DependencyKind::Import,
            resolution: prism_graph::ResolutionStatus::Resolved,
            confidence: prism_core::Confidence::High,
            target_id: Some(prism_graph::ModuleId::from_string(format!("file:{target}"))),
            target_path: Some(target.to_string()),
            imported_names: Vec::new(),
            location: None,
            evidence: vec![],
        }
    }

    fn module(id: &str) -> Module {
        Module {
            id: prism_graph::ModuleId::from_string(format!("file:{id}")),
            name: id.to_string(),
            kind: prism_graph::ModuleKind::FileModule,
            file_ids: vec![],
            parent_id: None,
            language: None,
            path: Some(id.to_string()),
            evidence: vec![],
        }
    }

    #[test]
    fn hierarchy_and_dependency_edges() {
        let files = vec![
            "src/battle/core.rs".to_string(),
            "src/battle/manager.rs".to_string(),
            "src/main.rs".to_string(),
        ];
        let deps = DependencyGraphResult {
            schema_version: 1,
            project_name: "demo".to_string(),
            modules: vec![
                module("src/battle/core.rs"),
                module("src/battle/manager.rs"),
                module("src/main.rs"),
            ],
            dependencies: vec![
                dependency("src/battle/manager.rs", "src/battle/core.rs"),
                dependency("src/main.rs", "src/battle/manager.rs"),
            ],
            nodes: vec![],
            edges: vec![],
            metrics: Default::default(),
            calls: vec![],
            cycles: vec![],
            diagnostics: vec![],
        };
        let graph = architecture_graph(
            &ArchitectureInput {
                files: &files,
                dependencies: &deps,
            },
            &VizLimits::default(),
        );
        let labels: Vec<&str> = graph.nodes.iter().map(|node| node.label.as_str()).collect();
        assert!(labels.contains(&"demo"));
        assert!(labels.contains(&"src"), "一级目录");
        assert!(labels.contains(&"battle"), "二级目录（§9 层级聚合）");
        assert!(labels.contains(&"core.rs"));
        assert_eq!(
            graph
                .edges
                .iter()
                .filter(|edge| edge.relation == "depends_on")
                .count(),
            2
        );
        assert_eq!(graph.status, VizStatus::Complete);
    }

    #[test]
    fn external_targets_stay_out_of_file_layer() {
        // §73：external 目标在 Dependency View 表达，Architecture 文件层
        // 不画到不存在的节点。
        let files = vec!["src/main.rs".to_string()];
        let deps = DependencyGraphResult {
            schema_version: 1,
            project_name: "demo".to_string(),
            modules: vec![module("src/main.rs")],
            dependencies: vec![Dependency {
                id: "dep:ext".to_string(),
                source_id: prism_graph::ModuleId::from_string("file:src/main.rs"),
                source_path: "src/main.rs".to_string(),
                raw_target: "lodash".to_string(),
                kind: prism_graph::DependencyKind::Import,
                resolution: prism_graph::ResolutionStatus::External,
                confidence: prism_core::Confidence::High,
                target_id: None,
                target_path: Some("node:external:external:lodash".to_string()),
                imported_names: Vec::new(),
                location: None,
                evidence: vec![],
            }],
            nodes: vec![],
            edges: vec![],
            metrics: Default::default(),
            calls: vec![],
            cycles: vec![],
            diagnostics: vec![],
        };
        let graph = architecture_graph(
            &ArchitectureInput {
                files: &files,
                dependencies: &deps,
            },
            &VizLimits::default(),
        );
        assert!(graph.edges.iter().all(|edge| edge.relation != "depends_on"));
        assert_eq!(graph.status, VizStatus::Complete);
    }
}
