//! 统一图构建（规格 §21–§25）。
//!
//! 多级图从同一底层关系派生（规格 §22）：file-level Imports → module-level
//! DependsOn（按父子层级上溯）；环检测用 SCC（Tarjan，仅 Resolved 边）；
//! 指标全部为客观计数（§24），不做评价。

use crate::model::{
    Cycle, Dependency, GraphEdge, GraphEdgeKind, GraphMetrics, GraphNode, GraphNodeKind, Module,
    ResolutionStatus,
};
use prism_core::Confidence;
use std::collections::{BTreeMap, BTreeSet};

/// 图装配输入输出。
pub struct GraphAssembly {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub cycles: Vec<Cycle>,
    pub metrics: GraphMetrics,
}

/// 从模块 + 依赖构建统一图。
pub fn build_graph(modules: &[Module], dependencies: &[Dependency]) -> GraphAssembly {
    let mut nodes: BTreeMap<String, GraphNode> = BTreeMap::new();
    let mut edges: BTreeMap<String, GraphEdge> = BTreeMap::new();

    // ---- 节点：模块层级 ----
    for module in modules {
        let kind = match module.kind {
            crate::model::ModuleKind::Project => GraphNodeKind::Project,
            crate::model::ModuleKind::Package => GraphNodeKind::Package,
            crate::model::ModuleKind::Namespace => GraphNodeKind::Namespace,
            crate::model::ModuleKind::FileModule => GraphNodeKind::FileModule,
            crate::model::ModuleKind::ExternalPackage => GraphNodeKind::ExternalPackage,
            crate::model::ModuleKind::Builtin => GraphNodeKind::BuiltinModule,
            crate::model::ModuleKind::Unknown => GraphNodeKind::FileModule,
        };
        nodes.insert(
            node_id(kind, &module.id.to_string()),
            GraphNode {
                id: node_id(kind, &module.id.to_string()),
                kind,
                label: module.name.clone(),
                entity: module.id.to_string(),
                language: module.language,
            },
        );
    }

    // ---- 边：Contains（父子层级）----
    for module in modules {
        if let Some(parent) = &module.parent_id {
            let parent_kind = modules.iter().find(|m| &m.id == parent).map_or(
                GraphNodeKind::Project,
                |m| match m.kind {
                    crate::model::ModuleKind::Project => GraphNodeKind::Project,
                    crate::model::ModuleKind::Package => GraphNodeKind::Package,
                    crate::model::ModuleKind::Namespace => GraphNodeKind::Namespace,
                    _ => GraphNodeKind::Project,
                },
            );
            let edge = GraphEdge {
                id: format!(
                    "edge:{}#{}#contains",
                    node_id(parent_kind, &parent.to_string()),
                    node_id(GraphNodeKind::FileModule, &module.id.to_string())
                ),
                source: node_id(parent_kind, &parent.to_string()),
                target: node_id(GraphNodeKind::FileModule, &module.id.to_string()),
                kind: GraphEdgeKind::Contains,
                resolution: ResolutionStatus::NotApplicable,
                confidence: Confidence::High,
                evidence: module.evidence.clone(),
            };
            edges.insert(edge.id.clone(), edge);
        }
    }

    // ---- 边：Imports（文件级，Resolved/Ambiguous 才入图）+ External/Builtin ----
    for dependency in dependencies {
        match dependency.resolution {
            ResolutionStatus::Resolved => {
                if dependency.kind == crate::model::DependencyKind::ProjectReference {
                    continue; // ProjectReference 在模块层表达（见下）
                }
                let source = node_id(GraphNodeKind::FileModule, &dependency.source_id.to_string());
                let target = node_id(
                    GraphNodeKind::FileModule,
                    dependency
                        .target_id
                        .as_ref()
                        .map(|id| id.to_string())
                        .unwrap_or_default()
                        .as_str(),
                );
                if source == target {
                    continue; // 自环无信息量
                }
                let kind = match dependency.kind {
                    crate::model::DependencyKind::ReExport => GraphEdgeKind::ReExports,
                    _ => GraphEdgeKind::Imports,
                };
                let edge = GraphEdge {
                    id: format!("edge:{source}#{target}#{}", edge_kind_str(kind)),
                    source,
                    target,
                    kind,
                    resolution: ResolutionStatus::Resolved,
                    confidence: dependency.confidence,
                    evidence: dependency.evidence.clone(),
                };
                edges.insert(edge.id.clone(), edge);
            }
            ResolutionStatus::External => {
                let entity = format!("external:{}", dependency.raw_target);
                nodes.entry(entity.clone()).or_insert_with(|| GraphNode {
                    id: node_id(GraphNodeKind::ExternalPackage, &entity),
                    kind: GraphNodeKind::ExternalPackage,
                    label: dependency.raw_target.clone(),
                    entity: entity.clone(),
                    language: None,
                });
                let source = node_id(GraphNodeKind::FileModule, &dependency.source_id.to_string());
                let target = node_id(GraphNodeKind::ExternalPackage, &entity);
                let edge = GraphEdge {
                    id: format!("edge:{source}#{target}#imports"),
                    source,
                    target,
                    kind: GraphEdgeKind::Imports,
                    resolution: ResolutionStatus::External,
                    confidence: dependency.confidence,
                    evidence: dependency.evidence.clone(),
                };
                edges.insert(edge.id.clone(), edge);
            }
            ResolutionStatus::Builtin => {
                let entity = format!("builtin:{}", dependency.raw_target);
                nodes.entry(entity.clone()).or_insert_with(|| GraphNode {
                    id: node_id(GraphNodeKind::BuiltinModule, &entity),
                    kind: GraphNodeKind::BuiltinModule,
                    label: dependency.raw_target.clone(),
                    entity: entity.clone(),
                    language: None,
                });
                let source = node_id(GraphNodeKind::FileModule, &dependency.source_id.to_string());
                let target = node_id(GraphNodeKind::BuiltinModule, &entity);
                let edge = GraphEdge {
                    id: format!("edge:{source}#{target}#imports"),
                    source,
                    target,
                    kind: GraphEdgeKind::Imports,
                    resolution: ResolutionStatus::Builtin,
                    confidence: dependency.confidence,
                    evidence: dependency.evidence.clone(),
                };
                edges.insert(edge.id.clone(), edge);
            }
            _ => {
                // Unresolved / Ambiguous / Ignored / NotApplicable：不建边，
                // 以 diagnostics 呈现（由调用方生成）。
            }
        }
    }

    // ---- 边：ProjectReferences（模块层 Project→Project）----
    for dependency in dependencies {
        if dependency.kind != crate::model::DependencyKind::ProjectReference {
            continue;
        }
        if dependency.resolution != ResolutionStatus::Resolved {
            continue;
        }
        let Some(target_path) = &dependency.target_path else {
            continue;
        };
        let source_node = node_id(GraphNodeKind::Project, &dependency.source_path);
        let target_node = node_id(GraphNodeKind::Project, target_path);
        nodes
            .entry(source_node.clone())
            .or_insert_with(|| GraphNode {
                id: source_node.clone(),
                kind: GraphNodeKind::Project,
                label: dependency.source_path.clone(),
                entity: dependency.source_path.clone(),
                language: None,
            });
        nodes
            .entry(target_node.clone())
            .or_insert_with(|| GraphNode {
                id: target_node.clone(),
                kind: GraphNodeKind::Project,
                label: target_path.clone(),
                entity: target_path.clone(),
                language: None,
            });
        let edge = GraphEdge {
            id: format!("edge:{source_node}#{target_node}#project_reference"),
            source: source_node,
            target: target_node,
            kind: GraphEdgeKind::ProjectReferences,
            resolution: ResolutionStatus::Resolved,
            confidence: Confidence::High,
            evidence: dependency.evidence.clone(),
        };
        edges.insert(edge.id.clone(), edge);
    }

    // ---- 模块级 DependsOn：由 Imports 按模块归属上溯派生 ----
    let file_to_module: BTreeMap<String, &Module> = modules
        .iter()
        .filter(|m| m.kind == crate::model::ModuleKind::FileModule)
        .map(|m| (m.path.clone().unwrap_or_default(), m))
        .collect();
    let module_node_of_file = |path: &str| -> Option<String> {
        file_to_module
            .get(path)
            .map(|m| node_id(GraphNodeKind::FileModule, &m.id.to_string()))
    };
    for dependency in dependencies {
        if dependency.resolution != ResolutionStatus::Resolved
            || dependency.kind == crate::model::DependencyKind::ProjectReference
        {
            continue;
        }
        let (Some(source_file), Some(target_file)) = (
            module_node_of_file(&dependency.source_path),
            dependency
                .target_path
                .as_deref()
                .and_then(module_node_of_file),
        ) else {
            continue;
        };
        if source_file == target_file {
            continue;
        }
        let edge = GraphEdge {
            id: format!("edge:{source_file}#{target_file}#depends_on"),
            source: source_file,
            target: target_file,
            kind: GraphEdgeKind::DependsOn,
            resolution: ResolutionStatus::Resolved,
            confidence: dependency.confidence,
            evidence: dependency.evidence.clone(),
        };
        edges.insert(edge.id.clone(), edge);
    }

    // ---- 环检测：Resolved Imports 文件级图上的 SCC（Tarjan，迭代式）----
    let cycles = detect_cycles(&nodes, &edges);

    // ---- 指标 ----
    let node_list: Vec<&GraphNode> = nodes.values().collect();
    let edge_list: Vec<&GraphEdge> = edges.values().collect();
    let metrics = compute_metrics(&node_list, &edge_list, &cycles);

    GraphAssembly {
        nodes: nodes.into_values().collect(),
        edges: edges.into_values().collect(),
        cycles,
        metrics,
    }
}

fn node_id(kind: GraphNodeKind, entity: &str) -> String {
    format!("node:{}:{entity}", kind_str(kind))
}

fn kind_str(kind: GraphNodeKind) -> &'static str {
    match kind {
        GraphNodeKind::Project => "project",
        GraphNodeKind::Package => "package",
        GraphNodeKind::Namespace => "namespace",
        GraphNodeKind::FileModule => "file",
        GraphNodeKind::ExternalPackage => "external",
        GraphNodeKind::BuiltinModule => "builtin",
    }
}

fn edge_kind_str(kind: GraphEdgeKind) -> &'static str {
    match kind {
        GraphEdgeKind::Contains => "contains",
        GraphEdgeKind::Imports => "imports",
        GraphEdgeKind::DependsOn => "depends_on",
        GraphEdgeKind::ReExports => "re_exports",
        GraphEdgeKind::ProjectReferences => "project_reference",
        GraphEdgeKind::PackageDependsOn => "package_depends_on",
    }
}

/// SCC 环检测（Tarjan，迭代实现；仅计 >1 节点的 SCC）。
fn detect_cycles(
    nodes: &BTreeMap<String, GraphNode>,
    edges: &BTreeMap<String, GraphEdge>,
) -> Vec<Cycle> {
    let resolved_imports: Vec<(&String, &String)> = edges
        .values()
        .filter(|e| {
            e.resolution == ResolutionStatus::Resolved
                && matches!(e.kind, GraphEdgeKind::Imports | GraphEdgeKind::DependsOn)
        })
        .map(|e| (&e.source, &e.target))
        .collect();

    let mut adjacency: BTreeMap<&String, Vec<&String>> = BTreeMap::new();
    for (source, target) in &resolved_imports {
        adjacency.entry(source).or_default().push(target);
    }

    // 迭代 Tarjan。
    let mut index_of: BTreeMap<&String, u64> = BTreeMap::new();
    let mut lowlink: BTreeMap<&String, u64> = BTreeMap::new();
    let mut on_stack: BTreeSet<&String> = BTreeSet::new();
    let mut stack: Vec<&String> = Vec::new();
    let mut next_index: u64 = 0;
    let mut sccs: Vec<Vec<&String>> = Vec::new();

    for start in nodes.keys() {
        if index_of.contains_key(start) {
            continue;
        }
        // (node, child iterator position)
        let mut call_stack: Vec<(&String, usize)> = vec![(start, 0)];
        while let Some((node, mut child_pos)) = call_stack.pop() {
            if child_pos == 0 {
                index_of.insert(node, next_index);
                lowlink.insert(node, next_index);
                next_index += 1;
                stack.push(node);
                on_stack.insert(node);
            }
            let children = adjacency.get(node).cloned().unwrap_or_default();
            let mut advanced = false;
            while child_pos < children.len() {
                let child = children[child_pos];
                if !index_of.contains_key(child) {
                    call_stack.push((node, child_pos + 1));
                    call_stack.push((child, 0));
                    advanced = true;
                    break;
                } else if on_stack.contains(child) {
                    let child_low = lowlink[child];
                    let current = lowlink[node];
                    if child_low < current {
                        lowlink.insert(node, child_low);
                    }
                }
                child_pos += 1;
            }
            if advanced {
                continue;
            }
            // 弹出 SCC
            if lowlink[node] == index_of[node] {
                let mut component: Vec<&String> = Vec::new();
                while let Some(top) = stack.pop() {
                    on_stack.remove(top);
                    component.push(top);
                    if top == node {
                        break;
                    }
                }
                if component.len() > 1 {
                    sccs.push(component);
                }
            }
            // 把 lowlink 传播给父帧
            if let Some(&(parent, _)) = call_stack.last() {
                let node_low = lowlink[node];
                let parent_low = lowlink[parent];
                if node_low < parent_low {
                    lowlink.insert(parent, node_low);
                }
            }
        }
    }

    let mut cycles: Vec<Cycle> = sccs
        .into_iter()
        .map(|component| {
            let mut ids: Vec<String> = component.iter().map(|node| (*node).clone()).collect();
            ids.sort();
            Cycle {
                id: format!("cycle:{}", ids.join("->")),
                nodes: ids,
            }
        })
        .collect();
    cycles.sort_by(|a, b| a.id.cmp(&b.id));
    cycles
}

fn compute_metrics(nodes: &[&GraphNode], edges: &[&GraphEdge], cycles: &[Cycle]) -> GraphMetrics {
    let mut in_degree: BTreeMap<&str, u64> = BTreeMap::new();
    let mut out_degree: BTreeMap<&str, u64> = BTreeMap::new();
    let mut resolved = 0u64;
    let mut ambiguous = 0u64;
    let mut unresolved = 0u64;
    let mut external = 0u64;
    let mut builtin = 0u64;

    for edge in edges {
        *out_degree.entry(edge.source.as_str()).or_insert(0) += 1;
        *in_degree.entry(edge.target.as_str()).or_insert(0) += 1;
        match edge.resolution {
            ResolutionStatus::Resolved => resolved += 1,
            ResolutionStatus::Ambiguous => ambiguous += 1,
            ResolutionStatus::Unresolved => unresolved += 1,
            ResolutionStatus::External => external += 1,
            ResolutionStatus::Builtin => builtin += 1,
            _ => {}
        }
    }

    let mut most_referenced: Vec<(String, u64)> = in_degree
        .iter()
        .map(|(id, count)| ((*id).to_string(), *count))
        .collect();
    most_referenced.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    most_referenced.truncate(10);

    let mut most_dependent: Vec<(String, u64)> = out_degree
        .iter()
        .map(|(id, count)| ((*id).to_string(), *count))
        .collect();
    most_dependent.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    most_dependent.truncate(10);

    // 连通分量：Resolved 依赖子图的弱连通分量数。
    let mut parent: BTreeMap<&str, &str> = BTreeMap::new();
    for node in nodes {
        parent.entry(node.id.as_str()).or_insert(node.id.as_str());
    }
    for edge in edges {
        if edge.resolution == ResolutionStatus::Resolved {
            // union-find find（路径不压缩也行，规模小；两次查找）
            let mut a = edge.source.as_str();
            let mut b = edge.target.as_str();
            // 防御：边端点若不在节点表（不应发生），跳过而非 panic。
            while let Some(next) = parent.get(a) {
                if *next == a {
                    break;
                }
                a = next;
            }
            while let Some(next) = parent.get(b) {
                if *next == b {
                    break;
                }
                b = next;
            }
            if a == b {
                continue;
            }
            if let Some(entry_a) = parent.get_mut(a) {
                *entry_a = b;
            }
        }
    }
    let mut components = 0u64;
    for node in nodes {
        let mut root = node.id.as_str();
        loop {
            match parent.get(root) {
                Some(next) if *next != root => root = next,
                _ => break,
            }
        }
        if root == node.id.as_str() {
            components += 1;
        }
    }

    GraphMetrics {
        node_count: nodes.len() as u64,
        edge_count: edges.len() as u64,
        resolved_edges: resolved,
        ambiguous_edges: ambiguous,
        unresolved_edges: unresolved,
        external_edges: external,
        builtin_edges: builtin,
        cycle_count: cycles.len() as u64,
        connected_components: components,
        most_referenced,
        most_dependent,
    }
}
