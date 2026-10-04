//! 统一 Fact 模型（2.0-M21 §11）。
//!
//! Core 不负责产生事实（那是 analyzer 的职责），只定义事实如何被表达；
//! Value 允许结构化（不强迫 string）。

use crate::evidence::EvidenceId;
use crate::identity::EntityId;
use crate::meta::{Confidence, ConfidenceReason, Freshness, Origin, Provenance};
use serde::{Deserialize, Serialize};
use serde_json::Value as Json;
use std::fmt;

/// 事实种类：开放注册 + 通用常量（M21 §11 清单为方向，不强制全实现）。
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct FactType(String);

impl FactType {
    pub fn file_exists() -> Self {
        FactType("file_exists".into())
    }
    pub fn file_hash() -> Self {
        FactType("file_hash".into())
    }
    pub fn dependency_declared() -> Self {
        FactType("dependency_declared".into())
    }
    pub fn modules_total() -> Self {
        FactType("modules_total".into())
    }
    pub fn dependencies_total() -> Self {
        FactType("dependencies_total".into())
    }
    pub fn unresolved_total() -> Self {
        FactType("unresolved_total".into())
    }
    pub fn cycles_total() -> Self {
        FactType("cycles_total".into())
    }

    pub fn other(name: impl Into<String>) -> Self {
        FactType(name.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for FactType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// 结构化事实值（§11：不强迫 string）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FactValue {
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    Structured(Json),
}

/// 统一事实。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Fact {
    /// 内容寻址稳定 id（`fact:{hash16}`）。
    pub id: String,
    pub fact_type: FactType,
    /// 事实主体；项目级聚合事实为 None（显式缺省，不伪造主体）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject: Option<EntityId>,
    pub value: FactValue,
    pub origin: Origin,
    pub confidence: Confidence,
    pub confidence_reason: ConfidenceReason,
    pub freshness: Freshness,
    /// 支撑证据（§58：重要推断必须可关联证据或显式 unavailable）。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence: Vec<EvidenceId>,
    pub observed_at: String,
    pub provenance: Provenance,
}

impl Fact {
    #[allow(clippy::too_many_arguments)] // 事实构造的完整契约字段（§11）
    pub fn new(
        fact_type: FactType,
        subject: Option<EntityId>,
        value: FactValue,
        origin: Origin,
        confidence: Confidence,
        confidence_reason: ConfidenceReason,
        freshness: Freshness,
        evidence: Vec<EvidenceId>,
        observed_at: impl Into<String>,
        provenance: Provenance,
    ) -> Self {
        let fingerprint = format!(
            "{fact_type}|{}|{value_json}",
            subject.as_ref().map(|s| s.as_str()).unwrap_or(""),
            value_json = serde_json::to_string(&fact_value_fingerprint(&value)).unwrap_or_default(),
        );
        Fact {
            id: format!("fact:{}", crate::identity::fnv1a_hex16(&fingerprint)),
            fact_type,
            subject,
            value,
            origin,
            confidence,
            confidence_reason,
            freshness,
            evidence,
            observed_at: observed_at.into(),
            provenance,
        }
    }
}

/// 指纹辅助：值 → JSON（内容寻址 id 的输入）。
fn fact_value_fingerprint(value: &FactValue) -> Json {
    serde_json::to_value(value).unwrap_or(Json::Null)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn provenance() -> Provenance {
        Provenance {
            producer: "test".into(),
            producer_version: "1".into(),
            observed_at: "2026-10-05T00:00:00Z".into(),
        }
    }

    #[test]
    fn same_content_same_id_different_value_different_id() {
        let a = Fact::new(
            FactType::modules_total(),
            None,
            FactValue::Int(7),
            Origin::Fact,
            Confidence::High,
            ConfidenceReason::direct_observation(),
            Freshness::Fresh,
            vec![],
            "2026-10-05T00:00:00Z",
            provenance(),
        );
        let b = Fact::new(
            FactType::modules_total(),
            None,
            FactValue::Int(7),
            Origin::Fact,
            Confidence::High,
            ConfidenceReason::direct_observation(),
            Freshness::Fresh,
            vec![],
            "2026-10-05T00:00:00Z",
            provenance(),
        );
        assert_eq!(a.id, b.id);
        let changed = Fact::new(
            FactType::modules_total(),
            None,
            FactValue::Int(8),
            Origin::Fact,
            Confidence::High,
            ConfidenceReason::direct_observation(),
            Freshness::Fresh,
            vec![],
            "2026-10-05T00:00:00Z",
            provenance(),
        );
        assert_ne!(a.id, changed.id);
    }

    #[test]
    fn structured_value_round_trips() {
        let fact = Fact::new(
            FactType::other("toolchain"),
            None,
            FactValue::Structured(serde_json::json!({ "name": "node", "version": "20.x" })),
            Origin::Fact,
            Confidence::High,
            ConfidenceReason::direct_observation(),
            Freshness::Fresh,
            vec![],
            "t",
            provenance(),
        );
        let back: Fact = serde_json::from_str(&serde_json::to_string(&fact).unwrap()).unwrap();
        assert_eq!(back, fact);
    }
}
