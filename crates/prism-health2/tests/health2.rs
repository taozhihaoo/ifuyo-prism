//! prism-health2 测试（2.0-M28 §82-§85：规则/生命周期/Gate/确定性）。

use prism_health2::baseline::diff_against_baseline;
use prism_health2::gate::{evaluate_gate, QualityGatePolicy};
use prism_health2::rules::{large_file_findings, RuleThresholds, SourceFileInput};
use prism_health2::Health2Report;

fn file(path: &str, lines: u64) -> SourceFileInput {
    SourceFileInput {
        path: path.into(),
        lines,
    }
}

#[test]
fn large_file_rule_threshold_and_determinism() {
    let files = vec![file("small.rs", 100), file("big.rs", 2000)];
    let findings = large_file_findings(&files, &RuleThresholds::default(), "t");
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].finding_type, "STRUCT/LARGE_FILE");
    // 确定性：同输入两遍 id 一致。
    let again = large_file_findings(&files, &RuleThresholds::default(), "t");
    assert_eq!(findings[0].id, again[0].id);
}

#[test]
fn lifecycle_new_persistent_resolved() {
    // 第一轮：2 个 finding；第二轮：1 修复 + 1 新增。
    let f1 = prism_health2_finding("F1");
    let f2 = prism_health2_finding("F2");
    let baseline = Health2Report {
        schema_version: 1,
        findings: vec![f1.clone(), f2.clone()],
        ..Health2Report::default()
    };

    let f3 = prism_health2_finding("F3");
    let current = Health2Report {
        schema_version: 1,
        findings: vec![f1.clone(), f3],
        ..Health2Report::default()
    };

    let diff = diff_against_baseline(&current, &baseline);
    assert_eq!(diff.persistent_findings.len(), 1, "F1 persistent");
    assert_eq!(diff.new_findings.len(), 1, "F3 new");
    assert_eq!(diff.resolved_findings.len(), 1, "F2 resolved");
}

fn prism_health2_finding(title: &str) -> prism_intel::Finding {
    prism_intel::Finding::new(
        "TEST",
        prism_core::Severity::Warning,
        prism_intel::finding::FindingStatus::Open,
        title,
        "fixture",
        prism_intel::Origin::Fact,
        prism_intel::Confidence::High,
        None,
        vec![],
    )
}

#[test]
fn gate_verdict_ladder_by_policy() {
    let baseline = Health2Report::default();
    let finding = prism_health2_finding("F1");
    let current = Health2Report {
        findings: vec![finding.clone()],
        ..Health2Report::default()
    };
    let diff = diff_against_baseline(&current, &baseline);

    // 默认 policy：max_new 0 ⇒ 1 个 new ⇒ FAIL。
    let result = evaluate_gate(Some(&diff), 0, &QualityGatePolicy::default());
    assert_eq!(result.verdict, prism_health2::GateVerdict::Fail);
    // 预算内非零 ⇒ WARN；无对比依据 ⇒ UNKNOWN。
    let lenient = QualityGatePolicy {
        max_new: 5,
        max_breaking: 0,
    };
    assert_eq!(
        evaluate_gate(Some(&diff), 0, &lenient).verdict,
        prism_health2::GateVerdict::Warn
    );
    assert_eq!(
        evaluate_gate(None, 0, &lenient).verdict,
        prism_health2::GateVerdict::Unknown
    );
}

#[test]
fn lifecycle_regressed_and_suppression() {
    use prism_intel::finding::FindingStatus;

    let f1 = prism_health2_finding("F1");
    let baseline_new_ids = vec![f1.id.clone()];
    // 基线轮把它标记为 New；当前轮仍存在 ⇒ Regressed。
    let baseline = Health2Report {
        schema_version: 1,
        findings: vec![f1.clone()],
        ..Health2Report::default()
    };
    let current = Health2Report {
        schema_version: 1,
        findings: vec![f1.clone()],
        ..Health2Report::default()
    };
    let diff = prism_health2::mark_regressed(
        diff_against_baseline(&current, &baseline),
        &baseline_new_ids,
        &current,
    );
    assert_eq!(diff.regressed_findings.len(), 1);

    // Suppressed：不计 New/Persistent，也不产生 Regressed。
    let mut suppressed = f1.clone();
    suppressed.status = FindingStatus::Suppressed;
    let current_suppressed = Health2Report {
        schema_version: 1,
        findings: vec![suppressed],
        ..Health2Report::default()
    };
    let diff = prism_health2::mark_regressed(
        diff_against_baseline(&current_suppressed, &baseline),
        &baseline_new_ids,
        &current_suppressed,
    );
    assert_eq!(diff.suppressed_findings.len(), 1);
    assert!(diff.new_findings.is_empty() && diff.persistent_findings.is_empty());
    assert!(diff.regressed_findings.is_empty());
}

#[test]
fn serialization_round_trip() {
    let report = Health2Report {
        schema_version: prism_health2::HEALTH2_SCHEMA_VERSION,
        ..Health2Report::default()
    };
    let text = serde_json::to_string(&report).unwrap();
    let back: Health2Report = serde_json::from_str(&text).unwrap();
    assert_eq!(back, report);
}

#[test]
fn analyze_inventory_metrics_and_findings() {
    let files = vec![file("a.rs", 100), file("big.rs", 2000)];
    let report = prism_health2::analyze_inventory(&files, &RuleThresholds::default(), "t");
    assert_eq!(report.metrics.get("files_total").unwrap().value, 2);
    assert_eq!(report.metrics.get("lines_total").unwrap().value, 2100);
    assert_eq!(report.metrics.get("large_files").unwrap().value, 1);
    assert_eq!(report.findings.len(), 1);
    assert_eq!(report.findings[0].finding_type, "STRUCT/LARGE_FILE");
}

#[test]
fn analyze_inventory_deterministic() {
    let files = vec![file("b.rs", 5000)];
    let a = prism_health2::analyze_inventory(&files, &RuleThresholds::default(), "t1");
    let b = prism_health2::analyze_inventory(&files, &RuleThresholds::default(), "t2");
    // 确定性 = 指纹与度量稳定（observed_at 是 Freshness 元数据，不进 id）。
    assert_eq!(
        a.findings[0].id, b.findings[0].id,
        "同输入 ⇒ 同 finding 指纹"
    );
    assert_eq!(a.metrics, b.metrics);
    assert_eq!(a.findings.len(), b.findings.len());
}
