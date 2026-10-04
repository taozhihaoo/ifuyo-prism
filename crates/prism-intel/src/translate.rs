//! 第一条垂直切片：M3 依赖域 → Intelligence Core（2.0-M21 §34–§35/§63）。
//!
//! 真实管线：`DependencyService::inspect_graph`（既有 analyzer，不重算）
//! → Core Entities/Relations/Evidence/Facts → CLI/GUI 消费。
//! 旧 JSON 契约（`prism graph`）保持不变——Core 是增量视图，非替换（§32/§33）。

use crate::bundle::IntelBundle;
use crate::context::{AnalysisContext, IntelOutcome, IntelStatus};
use crate::entity::{Entity, EntityLocation, EntityScope};
use crate::evidence::{Evidence, EvidenceKind};
use crate::fact::{Fact, FactType, FactValue};
use crate::identity::{EntityId, EntityKind};
use crate::meta::{Confidence, ConfidenceReason, Freshness, Origin, Provenance};
use crate::relation::{Relation, RelationTarget, RelationType, UnresolvedReference};
use prism_graph::{DependencyGraphResult, ModuleKind, ResolutionStatus};

/// 把既有依赖图翻译为 Core 集合（§63：Core 真正被生产代码使用）。
pub fn dependencies_to_intel(
    graph: &DependencyGraphResult,
    ctx: &AnalysisContext,
) -> IntelOutcome<IntelBundle> {
    if ctx.cancel.is_cancelled() {
        return IntelOutcome::cancelled(None, vec!["cancelled before translation".into()]);
    }

    let mut warnings: Vec<String> = Vec::new();
    let provenance = Provenance {
        producer: "prism-graph::DependencyService".to_string(),
        producer_version: graph.schema_version.to_string(),
        observed_at: ctx.observed_at.clone(),
    };
    let mut bundle = IntelBundle::new(ctx.project.clone(), provenance);

    // ---- 模块实体（scope：ExternalPackage/Builtin ⇒ External）----
    for module in &graph.modules {
        let (kind, external) = match module.kind {
            ModuleKind::Project => (EntityKind::project(), false),
            ModuleKind::Package => (EntityKind::package(), false),
            ModuleKind::Namespace => (EntityKind::namespace(), false),
            ModuleKind::FileModule => (EntityKind::module(), false),
            ModuleKind::ExternalPackage => (EntityKind::package(), true),
            ModuleKind::Builtin => (EntityKind::package(), true),
            ModuleKind::Unknown => (EntityKind::other("unknown"), false),
        };
        let id = bundle.entity_id(&kind, module.id.as_str());
        let location = match &module.path {
            Some(path) => EntityLocation::FileOnly { path: path.clone() },
            None if external => EntityLocation::External {
                reference: module.name.clone(),
            },
            None => EntityLocation::Unknown,
        };
        let mut entity = Entity::new(
            id,
            kind,
            module.name.clone(),
            location,
            if external {
                EntityScope::External
            } else {
                EntityScope::Internal
            },
        );
        if let Some(language) = module.language {
            entity.metadata.insert(
                "language".to_string(),
                format!("{language:?}").to_lowercase(),
            );
        }
        for item in &module.evidence {
            let evidence = Evidence::new(
                EvidenceKind::Analyzer,
                Some(item.path.clone()),
                None,
                Some(item.source.clone()),
                item.reason.clone(),
            );
            bundle.insert_evidence(evidence);
        }
        bundle.insert_entity(entity);
    }

    if ctx.cancel.is_cancelled() {
        return IntelOutcome::cancelled(Some(bundle), vec!["cancelled after modules".into()]);
    }

    // ---- 关系（方向：source 模块 → target）----
    let module_entity = |bundle: &IntelBundle, id: &prism_graph::ModuleId| -> Option<EntityId> {
        let key = id.as_str();
        let module = graph.modules.iter().find(|m| m.id.as_str() == key)?;
        let kind = match module.kind {
            ModuleKind::Project => EntityKind::project(),
            ModuleKind::Package => EntityKind::package(),
            ModuleKind::Namespace => EntityKind::namespace(),
            ModuleKind::FileModule => EntityKind::module(),
            ModuleKind::ExternalPackage | ModuleKind::Builtin => EntityKind::package(),
            ModuleKind::Unknown => EntityKind::other("unknown"),
        };
        Some(bundle.entity_id(&kind, key))
    };

    for dependency in &graph.dependencies {
        let relation_type = match dependency.kind {
            prism_graph::DependencyKind::Import | prism_graph::DependencyKind::ModuleReference => {
                RelationType::imports()
            }
            prism_graph::DependencyKind::ReExport => RelationType::re_exports(),
            prism_graph::DependencyKind::NamespaceReference => RelationType::references(),
            prism_graph::DependencyKind::ProjectReference
            | prism_graph::DependencyKind::PackageReference => RelationType::depends_on(),
            prism_graph::DependencyKind::Unknown => RelationType::references(),
        };
        let Some(source_id) = module_entity(&bundle, &dependency.source_id) else {
            continue;
        };
        let target = match &dependency.target_id {
            Some(target_id) => match module_entity(&bundle, target_id) {
                Some(entity) => RelationTarget::Resolved { entity },
                None => RelationTarget::Unresolved {
                    reference: UnresolvedReference {
                        specifier: dependency.raw_target.clone(),
                        reason: Some("target module missing from graph".into()),
                    },
                },
            },
            None => RelationTarget::Unresolved {
                reference: UnresolvedReference {
                    specifier: dependency.raw_target.clone(),
                    reason: None,
                },
            },
        };
        let mut evidence_ids = Vec::new();
        for item in &dependency.evidence {
            let evidence = Evidence::new(
                EvidenceKind::Analyzer,
                Some(item.path.clone()),
                None,
                Some(item.source.clone()),
                item.reason.clone(),
            );
            evidence_ids.push(evidence.id.clone());
            bundle.insert_evidence(evidence);
        }
        let mut relation = Relation::new(
            relation_type,
            source_id,
            target,
            Origin::Fact,
            Confidence::from_core(dependency.confidence),
            evidence_ids,
        );
        relation.metadata.insert(
            "resolution".to_string(),
            resolution_label(dependency.resolution).to_string(),
        );
        bundle.insert_relation(relation);
    }

    // ---- 聚合事实（§11：Core 表达事实；粒度随 M22/M23 分析器扩展）----
    let unresolved = graph
        .dependencies
        .iter()
        .filter(|dependency| dependency.target_id.is_none())
        .count();
    let aggregate: [(&FactType, i64); 4] = [
        (&FactType::modules_total(), graph.modules.len() as i64),
        (
            &FactType::dependencies_total(),
            graph.dependencies.len() as i64,
        ),
        (&FactType::unresolved_total(), unresolved as i64),
        (&FactType::cycles_total(), graph.cycles.len() as i64),
    ];
    for (fact_type, value) in aggregate {
        bundle.insert_fact(Fact::new(
            fact_type.clone(),
            None,
            FactValue::Int(value),
            Origin::Fact,
            Confidence::High,
            ConfidenceReason::direct_observation(),
            Freshness::Fresh,
            vec![],
            ctx.observed_at.clone(),
            bundle.provenance.clone(),
        ));
    }

    // ---- 解析诊断 → warnings（诚实降级；cap 防噪）----
    const MAX_DIAGNOSTIC_WARNINGS: usize = 20;
    for diagnostic in graph.diagnostics.iter().take(MAX_DIAGNOSTIC_WARNINGS) {
        warnings.push(format!(
            "unresolved import {} -> {} ({})",
            diagnostic.source_path, diagnostic.raw_target, diagnostic.reason
        ));
    }
    if graph.diagnostics.len() > MAX_DIAGNOSTIC_WARNINGS {
        warnings.push(format!(
            "…and {} more resolution diagnostics",
            graph.diagnostics.len() - MAX_DIAGNOSTIC_WARNINGS
        ));
    }

    // 诊断是项目的问题（§28 Diagnostic），不是分析失败——分析已完整完成，
    // 状态保持 Complete，诊断进 warnings 供呈现（§26：warnings 与 status 独立）。
    IntelOutcome {
        status: IntelStatus::Complete,
        data: Some(bundle),
        warnings,
        errors: Vec::new(),
    }
}

fn resolution_label(status: ResolutionStatus) -> &'static str {
    match status {
        ResolutionStatus::Resolved => "resolved",
        ResolutionStatus::Ambiguous => "ambiguous",
        ResolutionStatus::Unresolved => "unresolved",
        ResolutionStatus::External => "external",
        ResolutionStatus::Builtin => "builtin",
        ResolutionStatus::Ignored => "ignored",
        ResolutionStatus::NotApplicable => "not_applicable",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::AnalysisScope;
    use crate::context::IntelStatus;
    use crate::evidence::EvidenceKind;
    use crate::fact::FactValue;
    use crate::relation::RelationTarget;
    use prism_core::ids::ModuleId;
    use prism_core::task::CancellationToken;
    use prism_core::Confidence;
    use prism_graph::{Dependency, Module};

    fn module(id: &str, name: &str, path: Option<&str>, kind: ModuleKind) -> Module {
        Module {
            id: ModuleId::from_string(id),
            name: name.to_string(),
            kind,
            file_ids: vec![],
            parent_id: None,
            language: None,
            path: path.map(|p| p.to_string()),
            evidence: vec![],
        }
    }

    fn dependency(
        id: &str,
        source: &str,
        raw_target: &str,
        target_id: Option<&str>,
        kind: prism_graph::DependencyKind,
    ) -> Dependency {
        Dependency {
            id: id.to_string(),
            source_id: ModuleId::from_string(source),
            source_path: "src/a.py".to_string(),
            raw_target: raw_target.to_string(),
            kind,
            resolution: if target_id.is_some() {
                ResolutionStatus::Resolved
            } else {
                ResolutionStatus::Unresolved
            },
            confidence: Confidence::High,
            target_id: target_id.map(ModuleId::from_string),
            target_path: None,
            imported_names: vec![],
            location: None,
            evidence: vec![prism_project::Evidence::new(
                "test_resolver",
                "src/a.py",
                "import statement",
            )],
        }
    }

    fn fixture_graph() -> DependencyGraphResult {
        DependencyGraphResult {
            schema_version: 1,
            project_name: "demo".into(),
            modules: vec![
                module("mod:a", "a", Some("src/a.py"), ModuleKind::FileModule),
                module("mod:b", "b", Some("src/b.py"), ModuleKind::FileModule),
                module("mod:ext", "left-pad", None, ModuleKind::ExternalPackage),
            ],
            dependencies: vec![
                dependency(
                    "dep:1",
                    "mod:a",
                    "./b.py",
                    Some("mod:b"),
                    prism_graph::DependencyKind::Import,
                ),
                dependency(
                    "dep:2",
                    "mod:a",
                    "left-pad",
                    Some("mod:ext"),
                    prism_graph::DependencyKind::PackageReference,
                ),
                dependency(
                    "dep:3",
                    "mod:a",
                    "ghost",
                    None,
                    prism_graph::DependencyKind::Import,
                ),
            ],
            nodes: vec![],
            edges: vec![],
            calls: vec![],
            cycles: vec![],
            metrics: prism_graph::GraphMetrics {
                node_count: 3,
                edge_count: 3,
                resolved_edges: 2,
                ambiguous_edges: 0,
                unresolved_edges: 1,
                external_edges: 1,
                builtin_edges: 0,
                cycle_count: 0,
                connected_components: 1,
                most_referenced: vec![],
                most_dependent: vec![],
            },
            diagnostics: vec![prism_graph::ResolutionDiagnostic {
                source_path: "src/a.py".into(),
                raw_target: "ghost".into(),
                resolver: "python".into(),
                status: ResolutionStatus::Unresolved,
                reason: "no match".into(),
                location: None,
            }],
        }
    }

    fn context() -> AnalysisContext {
        AnalysisContext::new(
            crate::identity::ProjectIdentity::from_root(
                prism_core::ids::ProjectId::from_string("proj:t"),
                "C:/t",
            ),
            AnalysisScope::Domain("dependency".into()),
            vec!["dependency_graph".into()],
            CancellationToken::new(),
        )
    }

    #[test]
    fn slice_maps_modules_relations_evidence_and_facts() {
        let outcome = dependencies_to_intel(&fixture_graph(), &context());
        assert_eq!(outcome.status, IntelStatus::Complete);
        let bundle = outcome.data.expect("complete outcome carries data");

        assert_eq!(bundle.entities.len(), 3);
        assert_eq!(bundle.relations.len(), 3);
        // resolved：2 条（a→b imports、a→ext depends_on）；unresolved：1 条。
        let resolved = bundle
            .relations
            .values()
            .filter(|r| matches!(r.target, RelationTarget::Resolved { .. }))
            .count();
        let unresolved = bundle
            .relations
            .values()
            .filter(|r| matches!(r.target, RelationTarget::Unresolved { .. }))
            .count();
        assert_eq!(resolved, 2);
        assert_eq!(unresolved, 1);
        // 证据进入 bundle 且被关系引用（可追溯 §43.2）。
        assert!(!bundle.evidence.is_empty());
        for relation in bundle.relations.values() {
            for evidence_id in &relation.evidence {
                assert!(
                    bundle.evidence.contains_key(evidence_id),
                    "relation evidence must resolve in bundle"
                );
            }
        }
        // 聚合事实。
        let total = bundle
            .facts
            .values()
            .find(|f| f.fact_type == FactType::dependencies_total())
            .expect("dependencies_total fact");
        assert_eq!(total.value, FactValue::Int(3));
        // 诊断进 warnings。
        assert!(outcome.warnings[0].contains("ghost"));
    }

    #[test]
    fn determinism_same_input_same_output() {
        let a = dependencies_to_intel(&fixture_graph(), &context());
        let b = dependencies_to_intel(&fixture_graph(), &context());
        let ta = serde_json::to_string(&a.data).unwrap();
        let tb = serde_json::to_string(&b.data).unwrap();
        // observed_at 因 provenance/facts 携带时间而不同——比较剥离时间后的结构。
        let strip = |t: &str| t.replace(&a.data.as_ref().unwrap().provenance.observed_at, "");
        assert_eq!(
            strip(&ta),
            strip(&tb),
            "same input must produce same core output"
        );
        let keys_a: Vec<&String> = a.data.as_ref().unwrap().relations.keys().collect();
        let keys_b: Vec<&String> = b.data.as_ref().unwrap().relations.keys().collect();
        assert_eq!(keys_a, keys_b);
    }

    #[test]
    fn cancellation_is_explicit_not_faked_complete() {
        let ctx = context();
        let graph = fixture_graph();
        ctx.cancel.cancel();
        let outcome = dependencies_to_intel(&graph, &ctx);
        assert_eq!(outcome.status, IntelStatus::Cancelled);
        assert!(outcome.data.is_none(), "cancelled before work: no data");
    }

    #[test]
    fn project_isolation_different_roots_different_ids() {
        let graph = fixture_graph();
        let mut ctx_a = context();
        let mut ctx_b = context();
        ctx_a.project = crate::identity::ProjectIdentity::from_root(
            prism_core::ids::ProjectId::from_string("proj:a"),
            "C:/proj/a",
        );
        ctx_b.project = crate::identity::ProjectIdentity::from_root(
            prism_core::ids::ProjectId::from_string("proj:b"),
            "C:/proj/b",
        );
        let a = dependencies_to_intel(&graph, &ctx_a).data.unwrap();
        let b = dependencies_to_intel(&graph, &ctx_b).data.unwrap();
        let id_a: Vec<_> = a.entities.keys().collect();
        assert!(
            id_a.iter().all(|k| !b.entities.contains_key(*k)),
            "same module in different projects must have different ids"
        );
    }

    #[test]
    fn evidence_kinds_round_trip() {
        // §43.9 序列化 + unknown field tolerance：手动加未知字段仍可反序列化。
        let outcome = dependencies_to_intel(&fixture_graph(), &context());
        let bundle = outcome.data.unwrap();
        let mut text = serde_json::to_string(&bundle).unwrap();
        text.insert_str(text.len() - 1, r#","future_unknown_field": 1"#);
        let back: IntelBundle = serde_json::from_str(&text).expect("unknown fields tolerated");
        assert_eq!(back.entities.len(), bundle.entities.len());
        // EvidenceKind 开放种类。
        let e = Evidence::new(
            EvidenceKind::Other("godot.scene".into()),
            None,
            None,
            None,
            "scene ref",
        );
        assert!(matches!(e.kind, EvidenceKind::Other(_)));
    }
}
