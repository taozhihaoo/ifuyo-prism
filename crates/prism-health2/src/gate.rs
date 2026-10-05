//! Quality Gate（2.0-M28 §29-§30：策略判断，引擎不自行分析）。
//!
//! 四态结论（§H）：PASS / WARN / FAIL / UNKNOWN。
//! - FAIL：New 或 Breaking 超出 policy 预算；
//! - WARN：在预算内但非零（未达 PASS 的干净标准）；
//! - PASS：New = Breaking = 0；
//! - UNKNOWN：无生命周期对比可用（无基线/schema 不兼容）——不伪造结论。

use crate::baseline::LifecycleDiff;
use serde::{Deserialize, Serialize};

/// Gate 策略（§65：简单可配置；默认 0 容忍）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct QualityGatePolicy {
    /// 允许的最大 New 数（超出 ⇒ FAIL）。
    pub max_new: usize,
    /// 允许的最大 Breaking 类别数（超出 ⇒ FAIL）。
    pub max_breaking: usize,
}

/// Gate 结论（§H 四态）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GateVerdict {
    Pass,
    Warn,
    Fail,
    Unknown,
}

/// Gate 结论。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QualityGateResult {
    pub verdict: GateVerdict,
    pub new_count: usize,
    pub breaking_count: usize,
    pub reasons: Vec<String>,
}

/// 按 Policy 评估：`diff = None` ⇒ UNKNOWN（无对比依据，不伪造）；
/// breaking_count 由调用方按语义诊断/类别统计（§30 策略判断与引擎分析
/// 分离）。Suppressed 不参与计数（model 层已从 new 中排除）。
pub fn evaluate_gate(
    diff: Option<&LifecycleDiff>,
    breaking_count: usize,
    policy: &QualityGatePolicy,
) -> QualityGateResult {
    let Some(diff) = diff else {
        return QualityGateResult {
            verdict: GateVerdict::Unknown,
            new_count: 0,
            breaking_count,
            reasons: vec!["no lifecycle diff available (no comparable baseline)".to_string()],
        };
    };
    let new_count = diff.new_findings.len();
    let mut reasons = Vec::new();
    let mut failed = false;
    if new_count > policy.max_new {
        failed = true;
        reasons.push(format!(
            "new findings {} > allowed {}",
            new_count, policy.max_new
        ));
    }
    if breaking_count > policy.max_breaking {
        failed = true;
        reasons.push(format!(
            "breaking findings {} > allowed {}",
            breaking_count, policy.max_breaking
        ));
    }
    let verdict = if failed {
        GateVerdict::Fail
    } else if new_count == 0 && breaking_count == 0 {
        GateVerdict::Pass
    } else {
        GateVerdict::Warn
    };
    QualityGateResult {
        verdict,
        new_count,
        breaking_count,
        reasons,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Health2Report;
    use prism_core::Severity;
    use prism_intel::finding::FindingStatus;
    use prism_intel::{Confidence, Finding, Origin};

    fn finding(title: &str, status: FindingStatus) -> Finding {
        Finding::new(
            "TEST",
            Severity::Warning,
            status,
            title,
            "",
            Origin::Fact,
            Confidence::High,
            None,
            vec![],
        )
    }

    fn diff_with(current: Vec<Finding>, baseline: Vec<Finding>) -> LifecycleDiff {
        let to_report = |findings: Vec<Finding>| Health2Report {
            schema_version: 1,
            findings,
            ..Health2Report::default()
        };
        crate::diff_against_baseline(&to_report(current), &to_report(baseline))
    }

    #[test]
    fn no_diff_is_unknown() {
        let result = evaluate_gate(None, 0, &QualityGatePolicy::default());
        assert_eq!(result.verdict, GateVerdict::Unknown);
    }

    #[test]
    fn ladder_pass_warn_fail() {
        let policy = QualityGatePolicy {
            max_new: 5,
            max_breaking: 0,
        };
        let clean = diff_with(vec![], vec![]);
        assert_eq!(
            evaluate_gate(Some(&clean), 0, &policy).verdict,
            GateVerdict::Pass
        );
        let warn = diff_with(vec![finding("F1", FindingStatus::Open)], vec![]);
        assert_eq!(
            evaluate_gate(Some(&warn), 0, &policy).verdict,
            GateVerdict::Warn
        );
        let fail = diff_with(
            (1..=6)
                .map(|i| finding(&format!("F{i}"), FindingStatus::Open))
                .collect(),
            vec![],
        );
        assert_eq!(
            evaluate_gate(Some(&fail), 0, &policy).verdict,
            GateVerdict::Fail
        );
    }

    #[test]
    fn suppressed_does_not_fail_gate() {
        let policy = QualityGatePolicy::default();
        let suppressed = vec![finding("S1", FindingStatus::Suppressed)];
        let diff = diff_with(suppressed, vec![]);
        assert_eq!(
            evaluate_gate(Some(&diff), 0, &policy).verdict,
            GateVerdict::Pass
        );
    }
}
