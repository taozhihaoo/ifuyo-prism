//! Origin / Confidence / Freshness / Provenance（2.0-M21 §14–§16/§24–§25）。
//!
//! 不变量（§58）：
//! - FACT / INFERENCE / HYPOTHESIS 不得语义混用；
//! - Confidence ≠ Severity ≠ Priority ≠ Score（§20 四意分离）；
//! - Confidence 必须携带可指认来源（禁止纯主观赋值）；
//! - Freshness 四态显式，"有缓存" ≠ "缓存有效"。

use serde::{Deserialize, Serialize};
use std::fmt;

/// 知识层级（M21 §14）：结论从何而来。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Origin {
    /// 直接观察/解析产出，来源可复跑。
    Fact,
    /// 由 ≥2 个事实按确定规则推导，前提可追溯。
    Inference,
    /// 证据不足的合理推测；必须显式标注，禁止伪装成事实。
    Hypothesis,
}

/// 置信度四级（M21 §16）；映射 prism_core::Confidence：
/// High/Medium/Low 直映射，Unknown 为 core 未覆盖的显式第四态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    High,
    Medium,
    Low,
    Unknown,
}

impl Confidence {
    pub fn from_core(confidence: prism_core::Confidence) -> Self {
        match confidence {
            prism_core::Confidence::High => Confidence::High,
            prism_core::Confidence::Medium => Confidence::Medium,
            prism_core::Confidence::Low => Confidence::Low,
        }
    }
}

/// 置信度来源说明（§16 ConfidenceReason）：必须说明"为什么是这个级别"。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfidenceReason(String);

impl ConfidenceReason {
    pub fn direct_observation() -> Self {
        ConfidenceReason("direct parser/analyzer observation".to_string())
    }
    pub fn multiple_compatible_facts() -> Self {
        ConfidenceReason("multiple compatible facts".to_string())
    }
    pub fn heuristic_inference() -> Self {
        ConfidenceReason("heuristic inference with known false-positive surface".to_string())
    }
    pub fn insufficient_evidence() -> Self {
        ConfidenceReason("insufficient evidence".to_string())
    }
    pub fn custom(reason: impl Into<String>) -> Self {
        ConfidenceReason(reason.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ConfidenceReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// 数据新鲜度四态（M21 §24，落实 M20 §33 契约）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Freshness {
    /// 数据与当前工程状态一致。
    Fresh,
    /// 工程已变化；数据仍可能有用但需谨慎。
    Stale,
    /// 不能安全使用。
    Invalid,
    /// 无法判断。
    Unknown,
}

/// 出处（M21 §25）：谁在何时如何产出（analyzer 版本化）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Provenance {
    /// 产出者（如 "prism-graph::DependencyService"）。
    pub producer: String,
    /// 产出者版本（analyzer/adapter schema 版本，非 crate 版本混用）。
    pub producer_version: String,
    /// 观测时间（UTC RFC3339）。
    pub observed_at: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn origin_levels_do_not_conflate() {
        // §43.3：三级必须可区分且序列化稳定。
        let fact = serde_json::to_string(&Origin::Fact).unwrap();
        let inference = serde_json::to_string(&Origin::Inference).unwrap();
        let hypothesis = serde_json::to_string(&Origin::Hypothesis).unwrap();
        assert_eq!(fact, "\"fact\"");
        assert_eq!(inference, "\"inference\"");
        assert_eq!(hypothesis, "\"hypothesis\"");
        assert_ne!(Origin::Fact, Origin::Inference);
        assert_ne!(Origin::Inference, Origin::Hypothesis);
    }

    #[test]
    fn confidence_serialization_is_stable() {
        for (value, text) in [
            (Confidence::High, "\"high\""),
            (Confidence::Medium, "\"medium\""),
            (Confidence::Low, "\"low\""),
            (Confidence::Unknown, "\"unknown\""),
        ] {
            assert_eq!(serde_json::to_string(&value).unwrap(), text);
        }
    }

    #[test]
    fn confidence_is_not_severity() {
        // §20：置信度与严重度是两个轴——类型系统层面即不同类型。
        let confidence = Confidence::Low;
        let severity = prism_core::Severity::Critical;
        assert!(matches!(confidence, Confidence::Low));
        assert!(matches!(severity, prism_core::Severity::Critical));
    }

    #[test]
    fn freshness_has_four_states() {
        for state in [
            Freshness::Fresh,
            Freshness::Stale,
            Freshness::Invalid,
            Freshness::Unknown,
        ] {
            let _ = serde_json::to_string(&state).unwrap();
        }
        assert_ne!(Freshness::Fresh, Freshness::Stale);
    }
}
