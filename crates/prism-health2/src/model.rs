//! Health 2.0 模型（2.0-M28 §5-§7/§20-§23）。
//!
//! 职责分离（§3/§21）：**Finding 是分析结果；Metric 是测量结果；Health
//! 聚合是视图；Quality Gate 是策略判断**——四者不同一模型。Finding 沿
//! M21 `prism_intel::Finding`（唯一 SoT，§4）；生命周期由 Baseline 对比
//! 推导（§23-§26/§31）。

use prism_intel::Finding;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Metric：测量结果（§20/§12）——有值、单位、可解释定义与来源，
/// 独立于 Finding（Metric 是测量，不是分析结论）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Metric {
    /// 稳定键（如 "file_lines:src/main.rs"）。
    pub key: String,
    pub value: u64,
    /// 单位（lines / count / bytes）。
    pub unit: String,
    /// 可解释定义（§12：公式可解释，无黑盒）。
    pub definition: String,
    /// 产生该测量的规则/分析器（§50 Provenance）。
    pub producer: String,
}

/// Health 2.0 报告：一次分析的完整产物（§34 Snapshot 可承载）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Health2Report {
    pub schema_version: u32,
    /// Metric 键 → 测量（BTreeMap 确定性遍历，§41）。
    pub metrics: BTreeMap<String, Metric>,
    /// Evidence-backed Findings（M21 Finding SoT，§4）。
    pub findings: Vec<Finding>,
    /// 证据库：findings[].evidence 引用的 Evidence 全集（§37 可追溯）。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence: Vec<prism_intel::evidence::Evidence>,
    /// 规则执行诊断（prerequisite 失败/跳过，§37）。
    pub diagnostics: Vec<String>,
}

impl Health2Report {
    pub const SCHEMA_VERSION: u32 = 1;

    /// Finding 指纹集 = M21 内容寻址 id（无行号先例）——生命周期键（§24）。
    pub fn fingerprints(&self) -> std::collections::BTreeSet<String> {
        self.findings.iter().map(|f| f.id.clone()).collect()
    }
}

/// 生命周期对比（§26/§31/§33）：当前报告 vs 基线报告。
///
/// Suppression 语义（M21 FindingStatus::Suppressed）：当前侧被用户显式
/// 忽略的 finding 归入 `suppressed_findings`，**不**计入 New/Persistent，
/// 也不会因消失被记为 Resolved（不可自动复活/转正）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct LifecycleDiff {
    /// 基线没有、当前有（New；不含 Suppressed）。
    pub new_findings: Vec<String>,
    /// 两边都有（Persistent，§23；不含 Suppressed）。
    pub persistent_findings: Vec<String>,
    /// 基线有、当前没有且非 Suppressed（Resolved）。
    pub resolved_findings: Vec<String>,
    /// 基线为 New、当前仍存在（Regressed = 未修复确认，§33）。
    pub regressed_findings: Vec<String>,
    /// 当前侧用户显式忽略（Suppressed）。
    pub suppressed_findings: Vec<String>,
}

/// 按 fingerprint 集对比（增量 = 对比两次报告，§42/§84）。
pub fn diff_against_baseline(current: &Health2Report, baseline: &Health2Report) -> LifecycleDiff {
    let suppressed: std::collections::BTreeSet<String> = current
        .findings
        .iter()
        .filter(|f| f.status == prism_intel::finding::FindingStatus::Suppressed)
        .map(|f| f.id.clone())
        .collect();
    let mut current_ids = current.fingerprints();
    for id in &suppressed {
        current_ids.remove(id);
    }
    let baseline_ids = baseline.fingerprints();
    LifecycleDiff {
        new_findings: current_ids.difference(&baseline_ids).cloned().collect(),
        persistent_findings: current_ids.intersection(&baseline_ids).cloned().collect(),
        resolved_findings: baseline_ids.difference(&current_ids).cloned().collect(),
        regressed_findings: Vec::new(),
        suppressed_findings: suppressed.into_iter().collect(),
    }
}

/// Regressed（§33）：`baseline_new_ids` = 基线轮中为 New 的指纹集
/// （基线报告自带的 lifecycle 记录）；与当前仍存在的指纹求交。
pub fn mark_regressed(
    mut diff: LifecycleDiff,
    baseline_new_ids: &[String],
    current: &Health2Report,
) -> LifecycleDiff {
    let current_ids = current.fingerprints();
    diff.regressed_findings = baseline_new_ids
        .iter()
        .filter(|id| current_ids.contains(*id) && !diff.suppressed_findings.contains(id))
        .cloned()
        .collect();
    diff
}

/// 指纹集对比（持久化基线形态，优化清单批 A）：基线只有指纹集与
/// `baseline_new_ids`（来自 analysis_runs 行），不必反序列化整个基线
/// 报告。语义与 `diff_against_baseline` + `mark_regressed` 完全一致：
/// Suppressed 不参与 New/Persistent/Resolved/Regressed（§26/§31/§33）。
pub fn diff_against_fingerprints(
    current: &Health2Report,
    baseline_ids: &[String],
    baseline_new_ids: &[String],
) -> LifecycleDiff {
    let baseline_set: std::collections::BTreeSet<String> = baseline_ids.iter().cloned().collect();
    let diff = diff_against_baseline(current, &synthetic_baseline(baseline_set));
    mark_regressed(diff, baseline_new_ids, current)
}

/// 以指纹集合成的空报告基线（复用既有 diff 路径，保证语义单源）。
fn synthetic_baseline(ids: std::collections::BTreeSet<String>) -> Health2Report {
    Health2Report {
        findings: ids
            .into_iter()
            .map(|id| {
                let mut finding = prism_intel::Finding::new(
                    "SYNTHETIC",
                    prism_core::Severity::Info,
                    prism_intel::finding::FindingStatus::Open,
                    id.clone(),
                    "",
                    prism_intel::Origin::Fact,
                    prism_intel::Confidence::Low,
                    None,
                    vec![],
                );
                // 指纹 id 由 Finding::new 派生；直接覆写保证集合恒等。
                finding.id = id;
                finding
            })
            .collect(),
        ..Health2Report::default()
    }
}

#[cfg(test)]
mod fingerprint_diff_tests {
    use super::*;

    fn report_with(ids: &[&str]) -> Health2Report {
        Health2Report {
            findings: ids
                .iter()
                .map(|id| {
                    let mut finding = prism_intel::Finding::new(
                        "T",
                        prism_core::Severity::Info,
                        prism_intel::finding::FindingStatus::Open,
                        *id,
                        "",
                        prism_intel::Origin::Fact,
                        prism_intel::Confidence::Low,
                        None,
                        vec![],
                    );
                    finding.id = (*id).to_string();
                    finding
                })
                .collect(),
            ..Health2Report::default()
        }
    }

    #[test]
    fn fingerprint_diff_matches_report_diff_semantics() {
        let current = report_with(&["A", "B"]);
        // 基线 = {A, C}，其中 C 是基线轮 New ⇒ 与 report diff + regress 完全一致。
        let diff = diff_against_fingerprints(&current, &["A".into(), "C".into()], &["C".into()]);
        assert_eq!(diff.new_findings, vec!["B".to_string()]);
        assert_eq!(diff.persistent_findings, vec!["A".to_string()]);
        assert_eq!(diff.resolved_findings, vec!["C".to_string()]);
        assert!(diff.regressed_findings.is_empty());

        // 基线轮 New 的是 A（本轮仍在）⇒ Regressed。
        let diff = diff_against_fingerprints(&current, &["A".into(), "C".into()], &["A".into()]);
        assert_eq!(diff.regressed_findings, vec!["A".to_string()]);
    }

    #[test]
    fn empty_baseline_means_first_run_all_new() {
        let current = report_with(&["A"]);
        let diff = diff_against_fingerprints(&current, &[], &[]);
        assert_eq!(diff.new_findings, vec!["A".to_string()]);
        assert!(diff.persistent_findings.is_empty());
    }
}
