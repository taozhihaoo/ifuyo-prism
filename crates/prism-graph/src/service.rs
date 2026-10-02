//! DependencyService：discovery + analysis + resolution + graph 编排入口。
//!
//! CLI 与 GUI 共用同一管线（规格 §32）；复用 M2 解析结果（§29），
//! 单文件解析失败不击穿（§34），结果按稳定 ID 排序（§27）。

use crate::builder::build_graph;
use crate::calls::resolve_calls;
use crate::model::{
    Dependency, DependencyGraphResult, DependencyKind, GraphMetrics, ResolutionDiagnostic,
    ResolutionStatus, GRAPH_SCHEMA_VERSION,
};
use crate::modules::build_modules;
use crate::resolve::{resolve_import, ResolutionContext};
use prism_analysis::ProjectCodeModel;
use prism_core::Confidence;
use prism_project::ProjectDiscovery;
use std::path::Path;

#[derive(Debug, Default, Clone, Copy)]
pub struct DependencyService;

impl DependencyService {
    pub fn new() -> Self {
        DependencyService
    }

    /// 在既有 discovery + code model 上执行关系分析（GUI 复用 M2 结果）。
    pub fn analyze(
        &self,
        discovery: &ProjectDiscovery,
        model: &ProjectCodeModel,
    ) -> prism_core::PrismResult<DependencyGraphResult> {
        let modules = build_modules(discovery, model);
        let ctx = ResolutionContext::build(&discovery.project.root_path, discovery, model);

        let mut dependencies: Vec<Dependency> = Vec::new();
        let mut diagnostics: Vec<ResolutionDiagnostic> = Vec::new();

        for result in &model.parse_results {
            for import in &result.imports {
                let dependency = resolve_import(&ctx, &result.relative_path, import);
                if matches!(
                    dependency.resolution,
                    ResolutionStatus::Unresolved | ResolutionStatus::Ambiguous
                ) {
                    let reason = dependency
                        .evidence
                        .last()
                        .map(|evidence| evidence.reason.clone())
                        .unwrap_or_else(|| "unresolved".to_string());
                    diagnostics.push(ResolutionDiagnostic {
                        source_path: dependency.source_path.clone(),
                        raw_target: dependency.raw_target.clone(),
                        resolver: "import_resolver".to_string(),
                        status: dependency.resolution,
                        reason,
                        location: dependency.location,
                    });
                }
                dependencies.push(dependency);
            }
        }

        // ProjectReference（.csproj）：解析为项目级依赖。
        for (csproj_path, include) in &ctx.project_references {
            let include = include.replace('\\', "/");
            let resolved_target = normalize_csproj_include(csproj_path, &include);
            let resolved = ctx.files.contains_key(&resolved_target);
            let source_module = modules
                .iter()
                .find(|m| {
                    m.path
                        .as_deref()
                        .is_some_and(|path| path.ends_with(csproj_path))
                })
                .map(|m| m.id.clone());
            dependencies.push(Dependency {
                id: format!("dep:{csproj_path}#{include}#project_reference"),
                source_id: source_module.unwrap_or_else(|| {
                    prism_core::ids::ModuleId::from_string(format!("module:file:{csproj_path}"))
                }),
                source_path: csproj_path.clone(),
                raw_target: include.clone(),
                kind: DependencyKind::ProjectReference,
                resolution: if resolved {
                    ResolutionStatus::Resolved
                } else {
                    ResolutionStatus::Unresolved
                },
                confidence: Confidence::High,
                target_id: resolved.then(|| {
                    prism_core::ids::ModuleId::from_string(format!("module:file:{resolved_target}"))
                }),
                target_path: resolved.then_some(resolved_target.clone()),
                imported_names: vec![],
                location: None,
                evidence: vec![prism_project::Evidence::new(
                    "project_reference_resolver",
                    csproj_path,
                    "ProjectReference Include found in csproj",
                )],
            });
        }

        dependencies.sort_by(|a, b| a.id.cmp(&b.id));

        let calls = resolve_calls(model, &dependencies);

        let diagnostics: Vec<ResolutionDiagnostic> = {
            let mut list = diagnostics;
            list.sort_by(|a, b| {
                a.source_path
                    .cmp(&b.source_path)
                    .then_with(|| a.raw_target.cmp(&b.raw_target))
            });
            list.dedup_by(|a, b| a.source_path == b.source_path && a.raw_target == b.raw_target);
            list
        };

        let assembly = build_graph(&modules, &dependencies);
        let GraphMetrics {
            node_count,
            edge_count,
            resolved_edges,
            ambiguous_edges,
            unresolved_edges,
            external_edges,
            builtin_edges,
            cycle_count,
            connected_components,
            most_referenced,
            most_dependent,
        } = assembly.metrics;

        Ok(DependencyGraphResult {
            schema_version: GRAPH_SCHEMA_VERSION,
            project_name: discovery.project.name.clone(),
            modules,
            dependencies,
            calls,
            nodes: assembly.nodes,
            edges: assembly.edges,
            cycles: assembly.cycles,
            metrics: GraphMetrics {
                node_count,
                edge_count,
                resolved_edges,
                ambiguous_edges,
                unresolved_edges,
                external_edges,
                builtin_edges,
                cycle_count,
                connected_components,
                most_referenced,
                most_dependent,
            },
            diagnostics,
        })
    }

    /// 便捷入口：从项目根直接执行（CLI 单命令场景）。
    pub fn inspect_graph(
        &self,
        root: &Path,
        cancel: &prism_core::task::CancellationToken,
    ) -> prism_core::PrismResult<DependencyGraphResult> {
        let discovery = prism_project::DiscoveryService::new().discover(
            root,
            prism_project::ScanOptions::default(),
            cancel,
            |_| {},
        )?;
        let model = prism_analysis::CodeAnalysisService::new().analyze(
            &discovery,
            prism_analysis::CodeAnalysisOptions::default(),
            cancel,
            |_| {},
        )?;
        self.analyze(&discovery, &model)
    }
}

/// 解析 csproj ProjectReference 的 Include 相对路径为项目相对路径。
fn normalize_csproj_include(csproj_path: &str, include: &str) -> String {
    let base = std::path::Path::new(csproj_path)
        .parent()
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .unwrap_or_default();
    let mut parts: Vec<&str> = if base.is_empty() {
        Vec::new()
    } else {
        base.split('/').collect()
    };
    for segment in include.split('/') {
        match segment {
            "." => {}
            ".." => {
                parts.pop();
            }
            other => parts.push(other),
        }
    }
    parts.join("/")
}
