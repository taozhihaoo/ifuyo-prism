//! 统一 Relation 模型（2.0-M21 §17–§18/§59–§60）。
//!
//! 不变量：
//! - 方向显式（source → target），A DependsOn B ≠ B DependsOn A；
//! - 关系种类语义分明，禁止统一成 related_to；
//! - target 允许显式 Unresolved——禁止 analyzer 伪造假实体（§59）。

use crate::evidence::EvidenceId;
use crate::identity::EntityId;
use crate::meta::{Confidence, Origin};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

/// 关系种类：开放注册 + 通用常量（M21 §17 清单为方向）。
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RelationType(String);

impl RelationType {
    pub fn contains() -> Self {
        RelationType("contains".into())
    }
    pub fn defines() -> Self {
        RelationType("defines".into())
    }
    pub fn references() -> Self {
        RelationType("references".into())
    }
    pub fn calls() -> Self {
        RelationType("calls".into())
    }
    pub fn inherits() -> Self {
        RelationType("inherits".into())
    }
    pub fn implements() -> Self {
        RelationType("implements".into())
    }
    pub fn depends_on() -> Self {
        RelationType("depends_on".into())
    }
    pub fn imports() -> Self {
        RelationType("imports".into())
    }
    pub fn re_exports() -> Self {
        RelationType("re_exports".into())
    }
    pub fn generates() -> Self {
        RelationType("generates".into())
    }
    pub fn tests() -> Self {
        RelationType("tests".into())
    }
    pub fn located_in() -> Self {
        RelationType("located_in".into())
    }

    pub fn other(name: impl Into<String>) -> Self {
        RelationType(name.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for RelationType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// 未解析引用（§59）：specifier 原样保留，不造假实体。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnresolvedReference {
    /// 原样目标（import specifier / 包名 / URI）。
    pub specifier: String,
    /// 可选的未解析原因（来自 resolver 诊断）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// 关系目标：项目内已解析实体，或显式未解析引用。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RelationTarget {
    Resolved { entity: EntityId },
    Unresolved { reference: UnresolvedReference },
}

/// 统一关系（方向：source → target）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Relation {
    /// 内容寻址稳定 id（`rel:{hash16}`，覆盖 source/type/target 方向）。
    pub id: String,
    pub relation_type: RelationType,
    pub source: EntityId,
    pub target: RelationTarget,
    pub origin: Origin,
    pub confidence: Confidence,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence: Vec<EvidenceId>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub metadata: BTreeMap<String, String>,
}

impl Relation {
    pub fn new(
        relation_type: RelationType,
        source: EntityId,
        target: RelationTarget,
        origin: Origin,
        confidence: Confidence,
        evidence: Vec<EvidenceId>,
    ) -> Self {
        let target_key = match &target {
            RelationTarget::Resolved { entity } => format!("resolved:{}", entity.as_str()),
            RelationTarget::Unresolved { reference } => {
                format!("unresolved:{}", reference.specifier)
            }
        };
        let fingerprint = format!("{source}|{relation_type}|{target_key}");
        Relation {
            id: format!("rel:{}", crate::identity::fnv1a_hex16(&fingerprint)),
            relation_type,
            source,
            target,
            origin,
            confidence,
            evidence,
            metadata: BTreeMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::EntityKind;

    fn entity(key: &str) -> EntityId {
        EntityId::new("project-a", &EntityKind::module(), key)
    }

    #[test]
    fn direction_is_significant() {
        let a = entity("a");
        let b = entity("b");
        let ab = Relation::new(
            RelationType::depends_on(),
            a.clone(),
            RelationTarget::Resolved { entity: b.clone() },
            Origin::Fact,
            Confidence::High,
            vec![],
        );
        let ba = Relation::new(
            RelationType::depends_on(),
            b,
            RelationTarget::Resolved { entity: a },
            Origin::Fact,
            Confidence::High,
            vec![],
        );
        assert_ne!(ab.id, ba.id, "A→B must differ from B→A");
    }

    #[test]
    fn unresolved_target_is_legal_without_fake_entity() {
        let a = entity("a");
        let rel = Relation::new(
            RelationType::imports(),
            a,
            RelationTarget::Unresolved {
                reference: UnresolvedReference {
                    specifier: "left-pad".into(),
                    reason: Some("external package".into()),
                },
            },
            Origin::Fact,
            Confidence::High,
            vec![],
        );
        let back: Relation = serde_json::from_str(&serde_json::to_string(&rel).unwrap()).unwrap();
        assert_eq!(back, rel);
        match back.target {
            RelationTarget::Unresolved { reference } => assert_eq!(reference.specifier, "left-pad"),
            other => panic!("expected unresolved, got {other:?}"),
        }
    }

    #[test]
    fn relation_types_do_not_collapse_to_related_to() {
        assert_ne!(RelationType::depends_on(), RelationType::references());
        assert_ne!(RelationType::contains(), RelationType::calls());
    }
}
