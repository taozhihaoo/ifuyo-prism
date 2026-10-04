//! 统一 Finding 模型（2.0-M21 §19–§21）。
//!
//! Severity（严重度）/ Confidence（可信度）/ Status（生命周期）三轴分离；
//! 1.0 prism-core::Severity 直接复用，不另造一套。

use crate::evidence::EvidenceId;
use crate::identity::EntityId;
use crate::meta::{Confidence, Origin};
use prism_core::Severity;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Finding 生命周期（M21 §21：按当前真实需要最小集，兼容 1.0 语义）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingStatus {
    /// 新发现，未处理。
    Open,
    /// 已确认/修复（compare 语义下的 resolved）。
    Resolved,
    /// 用户显式忽略（不可自动复活为 resolved）。
    Suppressed,
    /// 目标实体已变化，结论过期。
    Stale,
    Unknown,
}

/// 可消费的工程结论（Health/Security/Dependency risk/Architecture/…共用容器）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Finding {
    /// 内容寻址稳定 id（`find:{hash16}`；沿 1.0 finding_fingerprint 的
    /// 无行号先例——同一问题在行内移动不产生新 id）。
    pub id: String,
    /// 规则/种类 id（如 "MAINT001"；开放注册）。
    pub finding_type: String,
    pub severity: Severity,
    pub status: FindingStatus,
    pub title: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    pub origin: Origin,
    pub confidence: Confidence,
    /// 结论目标；项目级结论可缺省。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<EntityId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence: Vec<EvidenceId>,
    /// 建议动作；**不强迫所有 Finding 都有 remediation**（§19）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remediation: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub metadata: BTreeMap<String, String>,
}

impl Finding {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        finding_type: impl Into<String>,
        severity: Severity,
        status: FindingStatus,
        title: impl Into<String>,
        description: impl Into<String>,
        origin: Origin,
        confidence: Confidence,
        target: Option<EntityId>,
        evidence: Vec<EvidenceId>,
    ) -> Self {
        let finding_type = finding_type.into();
        let title = title.into();
        // 无行号指纹：type|title|target（行内移动不换 id，§37 先例）。
        let target_key = target.as_ref().map(|t| t.as_str()).unwrap_or("");
        let fingerprint = format!("{finding_type}|{title}|{target_key}");
        Finding {
            id: format!("find:{}", crate::identity::fnv1a_hex16(&fingerprint)),
            finding_type,
            severity,
            status,
            title,
            description: description.into(),
            origin,
            confidence,
            target,
            evidence,
            remediation: None,
            metadata: BTreeMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn severity_confidence_status_are_separate_axes() {
        let f = Finding::new(
            "SEC001",
            Severity::Warning,
            FindingStatus::Open,
            "hard-coded secret",
            "credential-like assignment",
            Origin::Fact,
            Confidence::High,
            None,
            vec![],
        );
        // 三个轴各自独立取值，互不替代（§20）。
        assert_eq!(f.severity, Severity::Warning);
        assert_eq!(f.confidence, Confidence::High);
        assert_eq!(f.status, FindingStatus::Open);
        assert!(f.remediation.is_none(), "remediation is optional");
    }

    #[test]
    fn id_excludes_line_numbers_and_description() {
        let a = Finding::new(
            "R1",
            Severity::Warning,
            FindingStatus::Open,
            "t",
            "d1",
            Origin::Fact,
            Confidence::High,
            None,
            vec![],
        );
        let b = Finding::new(
            "R1",
            Severity::Warning,
            FindingStatus::Open,
            "t",
            "d2",
            Origin::Fact,
            Confidence::High,
            None,
            vec![],
        );
        assert_eq!(
            a.id, b.id,
            "description-only edits must not change identity"
        );
    }
}
