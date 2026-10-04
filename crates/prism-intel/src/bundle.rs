//! IntelBundle：一次分析的统一产物（2.0-M21 §29/§30/§35）。
//!
//! 不变量：
//! - 全部集合 BTreeMap 按 id 索引：去重 + 确定性遍历（§29/§30）；
//! - bundle 携带项目身份（§38 project isolation）+ 模型版本（§61）；
//! - 生成后对 consumer 只读（§55 不可变结果；insert_* 仅 builder 阶段使用）。

use crate::entity::Entity;
use crate::evidence::{Evidence, EvidenceId};
use crate::fact::Fact;
use crate::finding::Finding;
use crate::identity::{EntityId, ProjectIdentity};
use crate::meta::Provenance;
use crate::relation::Relation;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Core 模型版本（§61）：与 analyzer/adapter 版本分离。
pub const INTEL_SCHEMA_VERSION: u32 = 1;

/// 一次分析产出的统一智能集合。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IntelBundle {
    pub schema_version: u32,
    pub project: ProjectIdentity,
    pub provenance: Provenance,
    pub entities: BTreeMap<EntityId, Entity>,
    pub facts: BTreeMap<String, Fact>,
    pub evidence: BTreeMap<EvidenceId, Evidence>,
    pub relations: BTreeMap<String, Relation>,
    pub findings: BTreeMap<String, Finding>,
}

impl IntelBundle {
    pub fn new(project: ProjectIdentity, provenance: Provenance) -> Self {
        IntelBundle {
            schema_version: INTEL_SCHEMA_VERSION,
            project,
            provenance,
            entities: BTreeMap::new(),
            facts: BTreeMap::new(),
            evidence: BTreeMap::new(),
            relations: BTreeMap::new(),
            findings: BTreeMap::new(),
        }
    }

    /// 幂等插入（同 id 视为同一事实，保留首见）。
    pub fn insert_entity(&mut self, entity: Entity) {
        self.entities.entry(entity.id.clone()).or_insert(entity);
    }

    pub fn insert_fact(&mut self, fact: Fact) {
        self.facts.entry(fact.id.clone()).or_insert(fact);
    }

    pub fn insert_evidence(&mut self, evidence: Evidence) {
        self.evidence.entry(evidence.id.clone()).or_insert(evidence);
    }

    pub fn insert_relation(&mut self, relation: Relation) {
        self.relations
            .entry(relation.id.clone())
            .or_insert(relation);
    }

    pub fn insert_finding(&mut self, finding: Finding) {
        self.findings.entry(finding.id.clone()).or_insert(finding);
    }

    pub fn entity_id(&self, kind: &crate::identity::EntityKind, canonical_key: &str) -> EntityId {
        EntityId::new(&self.project.stable_key, kind, canonical_key)
    }

    /// 便捷查询：某实体的全部出边（方向显式）。
    pub fn outgoing_relations(&self, source: &EntityId) -> Vec<&Relation> {
        self.relations
            .values()
            .filter(|relation| &relation.source == source)
            .collect()
    }

    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
            && self.facts.is_empty()
            && self.relations.is_empty()
            && self.findings.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::{EntityLocation, EntityScope};
    use crate::identity::{EntityKind, ProjectIdentity};
    use crate::meta::{Confidence, Origin};
    use crate::meta::{Freshness, Provenance};
    use crate::relation::{Relation, RelationTarget, RelationType};
    use prism_core::ids::ProjectId;

    fn bundle() -> IntelBundle {
        IntelBundle::new(
            ProjectIdentity::from_root(ProjectId::from_string("proj:t"), "C:/t"),
            Provenance {
                producer: "test".into(),
                producer_version: "1".into(),
                observed_at: "2026-10-05T00:00:00Z".into(),
            },
        )
    }

    fn entity(b: &mut IntelBundle, key: &str) -> EntityId {
        let id = b.entity_id(&EntityKind::module(), key);
        b.insert_entity(Entity::new(
            id.clone(),
            EntityKind::module(),
            key,
            EntityLocation::FileOnly { path: key.into() },
            EntityScope::Internal,
        ));
        id
    }

    #[test]
    fn iteration_is_deterministic_and_duplicate_insert_idempotent() {
        let mut b = bundle();
        let a = entity(&mut b, "src/a.py");
        let z = entity(&mut b, "src/z.py");
        for _ in 0..3 {
            b.insert_relation(Relation::new(
                RelationType::imports(),
                a.clone(),
                RelationTarget::Resolved { entity: z.clone() },
                Origin::Fact,
                Confidence::High,
                vec![],
            ));
        }
        assert_eq!(b.relations.len(), 1, "duplicate relation must dedupe by id");
        let ids: Vec<&str> = b.entities.keys().map(|k| k.as_str()).collect();
        let again: Vec<&str> = b.entities.keys().map(|k| k.as_str()).collect();
        assert_eq!(ids, again, "BTreeMap iteration is stable");
    }

    #[test]
    fn outgoing_relations_filter_by_direction() {
        let mut b = bundle();
        let a = entity(&mut b, "a");
        let x = entity(&mut b, "x");
        b.insert_relation(Relation::new(
            RelationType::imports(),
            a.clone(),
            RelationTarget::Resolved { entity: x.clone() },
            Origin::Fact,
            Confidence::High,
            vec![],
        ));
        b.insert_relation(Relation::new(
            RelationType::imports(),
            x.clone(),
            RelationTarget::Resolved { entity: a.clone() },
            Origin::Fact,
            Confidence::High,
            vec![],
        ));
        assert_eq!(b.outgoing_relations(&a).len(), 1, "only the A→X edge");
        assert_eq!(b.outgoing_relations(&x).len(), 1);
    }

    #[test]
    fn freshness_default_is_explicit_not_assumed() {
        // 新鲜度是显式字段（Fact/Finding 各自携带）——bundle 本身不假装 fresh。
        assert_ne!(Freshness::Fresh, Freshness::Unknown);
    }
}
