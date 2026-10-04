//! # Prism Intelligence Core（2.0-M21）
//!
//! 统一工程智能数据层：Entity / Fact / Evidence / Relation / Finding +
//! Origin / Confidence / Provenance / Freshness + AnalysisContext +
//! IntelOutcome（partial 诚实表达）+ IntelBundle（确定性集合）。
//!
//! 边界（M21 Gate）：
//! - Core 不依赖 GUI / Tauri / CLI / MCP / Godot / 具体 adapter；
//! - 事实由 analyzer 产出，Core 只定义表达（M21 不重算 M1–M9）；
//! - 不持久化（§42 transient：SQLite 归 prism-store，snapshot 归 prism-snapshot）；
//! - 无 AI/网络/遥测；secret 沿中央 redaction，Core 不新增执行面。
//!
//! 第一条垂直切片（§63/§34）：`translate::dependencies_to_intel`——
//! 既有 `prism-graph` 依赖图 → Core 集合；`prism intel` CLI 消费。
//! 旧 JSON 契约保持不变（§32 additive first）。

pub mod bundle;
pub mod context;
pub mod entity;
pub mod evidence;
pub mod fact;
pub mod finding;
pub mod identity;
pub mod meta;
pub mod relation;
pub mod translate;

pub use bundle::{IntelBundle, INTEL_SCHEMA_VERSION};
pub use context::{AnalysisContext, AnalysisScope, IntelOutcome, IntelStatus};
pub use entity::{Entity, EntityLocation, EntityScope};
pub use evidence::{Evidence, EvidenceAnchor, EvidenceId, EvidenceKind, SourceRange};
pub use fact::{Fact, FactType, FactValue};
pub use finding::{Finding, FindingStatus};
pub use identity::{EntityId, EntityKind, ProjectIdentity};
pub use meta::{Confidence, ConfidenceReason, Freshness, Origin, Provenance};
pub use relation::{Relation, RelationTarget, RelationType, UnresolvedReference};

#[cfg(test)]
mod integration_tests {
    use crate::bundle::IntelBundle;
    use crate::context::{AnalysisContext, AnalysisScope};
    use crate::evidence::Evidence;
    use crate::evidence::EvidenceKind;
    use crate::fact::{Fact, FactType, FactValue};
    use crate::finding::{Finding, FindingStatus};
    use crate::identity::{EntityKind, ProjectIdentity};
    use crate::meta::{Confidence, ConfidenceReason, Freshness, Origin, Provenance};
    use crate::relation::{Relation, RelationTarget, RelationType};
    use prism_core::ids::ProjectId;
    use prism_core::task::CancellationToken;
    use prism_core::Severity;

    fn ctx() -> AnalysisContext {
        AnalysisContext::new(
            ProjectIdentity::from_root(ProjectId::from_string("proj:e2e"), "C:/e2e"),
            AnalysisScope::FullProject,
            vec!["dependency_graph".into()],
            CancellationToken::new(),
        )
    }

    fn provenance(at: &str) -> Provenance {
        Provenance {
            producer: "e2e-test".into(),
            producer_version: "1".into(),
            observed_at: at.into(),
        }
    }

    /// 端到端：手工组装一条 实体→事实→关系→发现 全链 bundle（M21 §35 数据流验证）。
    #[test]
    fn full_chain_project_to_finding_with_evidence_traceability() {
        let context = ctx();
        let mut bundle =
            IntelBundle::new(context.project.clone(), provenance(&context.observed_at));

        let module_id = bundle.entity_id(&EntityKind::module(), "src/battle.py");
        bundle.insert_entity(crate::entity::Entity::new(
            module_id.clone(),
            EntityKind::module(),
            "battle.py",
            crate::entity::EntityLocation::FileOnly {
                path: "src/battle.py".into(),
            },
            crate::entity::EntityScope::Internal,
        ));

        let evidence = Evidence::new(
            EvidenceKind::SourceRange,
            Some("src/battle.py".into()),
            Some(crate::evidence::SourceRange {
                path: "src/battle.py".into(),
                start_line: 12,
                start_column: 4,
                end_line: 12,
                end_column: 32,
            }),
            None,
            "explicit import statement",
        );
        bundle.insert_evidence(evidence.clone());

        let fact = Fact::new(
            FactType::dependency_declared(),
            Some(module_id.clone()),
            FactValue::Str("left-pad".into()),
            Origin::Fact,
            Confidence::High,
            ConfidenceReason::direct_observation(),
            Freshness::Fresh,
            vec![evidence.id.clone()],
            context.observed_at.clone(),
            provenance(&context.observed_at),
        );
        bundle.insert_fact(fact);

        let _external = bundle.entity_id(&EntityKind::package(), "pkg:left-pad");
        let relation = Relation::new(
            RelationType::depends_on(),
            module_id.clone(),
            RelationTarget::Unresolved {
                reference: crate::relation::UnresolvedReference {
                    specifier: "left-pad".into(),
                    reason: Some("external".into()),
                },
            },
            Origin::Inference,
            Confidence::Medium,
            vec![evidence.id.clone()],
        );
        bundle.insert_relation(relation);

        // INFERENCE 关系的前置 FACT 在 bundle 中可追溯（§58 invariant）。
        let finding = Finding::new(
            "DEP001",
            Severity::Warning,
            FindingStatus::Open,
            "unresolved dependency declared",
            "module declares a dependency with no resolvable target",
            Origin::Inference,
            Confidence::Medium,
            Some(module_id.clone()),
            vec![evidence.id.clone()],
        );
        bundle.insert_finding(finding);

        // —— 不变量逐条验证 ——
        // Evidence 可追溯：fact/relation/finding 的证据都在 bundle 中。
        for fact in bundle.facts.values() {
            for id in &fact.evidence {
                assert!(bundle.evidence.contains_key(id));
            }
        }
        for relation in bundle.relations.values() {
            for id in &relation.evidence {
                assert!(bundle.evidence.contains_key(id));
            }
        }
        for finding in bundle.findings.values() {
            for id in &finding.evidence {
                assert!(bundle.evidence.contains_key(id));
            }
        }
        // FACT（事实）与 INFERENCE（关系/发现）不混淆。
        let facts: Vec<&crate::fact::Fact> = bundle.facts.values().collect();
        assert!(facts.iter().all(|f| f.origin == Origin::Fact));
        assert!(bundle
            .relations
            .values()
            .all(|r| r.origin == Origin::Inference));
        // 序列化 round-trip + 确定性。
        let text = serde_json::to_string_pretty(&bundle).unwrap();
        let back: IntelBundle = serde_json::from_str(&text).unwrap();
        assert_eq!(back, bundle);
        assert_eq!(serde_json::to_string_pretty(&back).unwrap(), text);
    }

    #[test]
    fn same_display_name_is_independent_entity() {
        let context = ctx();
        let bundle = IntelBundle::new(context.project.clone(), provenance(&context.observed_at));
        let a = bundle.entity_id(&EntityKind::module(), "src/a/util.py");
        let b = bundle.entity_id(&EntityKind::module(), "src/b/util.py");
        assert_ne!(a, b, "same display name in different files stays distinct");
    }
}
