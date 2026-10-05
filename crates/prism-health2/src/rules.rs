//! Health 2.0 规则（2.0-M28 §11A：结构面第一批，阈值可配置）。
//!
//! 输入 = L0 文件清单（相对路径 + 行数），全部来自 M22/M23 既有事实——
//! 不重新扫描文件系统（§2 边界）。规则产出 M21 `prism_intel::Finding`
//! （id 内容寻址、无行号；Evidence/Origin/Confidence 齐全）。

use prism_intel::finding::{Finding, FindingStatus};
use prism_intel::identity::{EntityId, EntityKind};
use prism_intel::{Confidence, Origin};

/// 规则输入：一个源文件的行数与路径（来自 M22 清单）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceFileInput {
    pub path: String,
    pub lines: u64,
}

/// 规则阈值（§11A：可配置，不写死裸数字策略）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuleThresholds {
    pub large_file_lines: u64,
    /// 类型声明行数上限（§B Symbol/Complexity）。
    pub large_type_lines: u64,
    /// 方法类声明行数上限（§B）。
    pub large_method_lines: u64,
    /// 语义诊断聚合组上限（finding 爆炸防护）。
    pub max_semantic_groups: usize,
    /// M29 供给规则单条 finding 上限（§69 爆炸防护）。
    pub max_supply_findings: usize,
    /// M210：运行时错误重复次数阈值（RepeatedRuntimeError）。
    pub repeated_error_occurrences: u64,
    /// M210：是否上报未解析栈帧（DEBUG/UNRESOLVED_STACK_FRAME）。
    pub report_unresolved_frames: bool,
    /// M211：热点判定阈值（窗口内 commit 数）。
    pub evolution_hotspot_min_commits: u64,
    /// M211：耦合判定阈值（共同变更次数）。
    pub evolution_min_co_change: u64,
    /// M213：HTTP 状态阈值（≥ 该值视为失败观测，默认 500）。
    pub api_failure_status_min: u16,
    /// M213：同一端点重复失败次数阈值。
    pub api_repeated_failures: u64,
}

impl Default for RuleThresholds {
    fn default() -> Self {
        RuleThresholds {
            large_file_lines: 1000,
            large_type_lines: 400,
            large_method_lines: 100,
            max_semantic_groups: 200,
            max_supply_findings: 200,
            repeated_error_occurrences: 10,
            report_unresolved_frames: true,
            evolution_hotspot_min_commits: 10,
            evolution_min_co_change: 5,
            api_failure_status_min: 500,
            api_repeated_failures: 3,
        }
    }
}

/// 结构规则：大文件（§11A 文件级；阈值可配置）。
/// Finding id 内容寻址（无行号）；生命周期由 Baseline 对比（§26）。
pub fn large_file_findings(
    files: &[SourceFileInput],
    thresholds: &RuleThresholds,
    observed_at: &str,
) -> Vec<Finding> {
    let mut out = Vec::new();
    for file in files {
        if file.lines <= thresholds.large_file_lines {
            continue;
        }
        let entity_id = EntityId::new("health2", &EntityKind::file(), &file.path);
        let mut finding = Finding::new(
            "STRUCT/LARGE_FILE",
            prism_core::Severity::Warning,
            FindingStatus::Open,
            format!("Large source file ({} lines)", file.lines),
            format!(
                "{} has {} lines (threshold {})",
                file.path, file.lines, thresholds.large_file_lines
            ),
            Origin::Fact,
            Confidence::High,
            Some(entity_id),
            vec![],
        );
        finding
            .metadata
            .insert("path".to_string(), file.path.clone());
        finding
            .metadata
            .insert("lines".to_string(), file.lines.to_string());
        finding
            .metadata
            .insert("observed_at".to_string(), observed_at.to_string());
        out.push(finding);
    }
    out
}

/// 注册表形态的结构规则（§C）：在 `large_file_findings` 之上补
/// Metric（files/lines/large_files）与每条 Finding 的 File 证据。
pub fn large_file_rule(ctx: &crate::registry::RuleContext) -> crate::registry::RuleOutcome {
    let mut outcome = crate::registry::RuleOutcome::default();
    if ctx.files.is_empty() {
        outcome
            .diagnostics
            .push("no file inventory injected; rule skipped".to_string());
        return outcome;
    }
    let total_lines: u64 = ctx.files.iter().map(|f| f.lines).sum();
    let findings = large_file_findings(ctx.files, ctx.thresholds, ctx.observed_at);
    let large_files = findings.len() as u64;
    for mut finding in findings {
        let path = finding.metadata.get("path").cloned().unwrap_or_default();
        let lines = finding.metadata.get("lines").cloned().unwrap_or_default();
        let evidence = prism_intel::evidence::Evidence::new(
            prism_intel::evidence::EvidenceKind::File,
            Some(path.clone()),
            None,
            None,
            format!("L0 inventory: {lines} lines"),
        );
        finding.evidence = vec![evidence.id.clone()];
        outcome.evidence.push(evidence);
        outcome.findings.push(finding);
    }
    for (key, value, definition) in [
        (
            "files_total",
            ctx.files.len() as u64,
            "number of inventoried source files",
        ),
        (
            "lines_total",
            total_lines,
            "total lines across inventoried files",
        ),
        (
            "large_files",
            large_files,
            "files over the large-file threshold",
        ),
    ] {
        outcome.metrics.push(crate::model::Metric {
            key: key.to_string(),
            value,
            unit: "count".to_string(),
            definition: definition.to_string(),
            producer: "prism-health2::rules".to_string(),
        });
    }
    outcome
}

#[cfg(test)]
mod tests {
    use super::*;

    fn files(specs: &[(&str, u64)]) -> Vec<SourceFileInput> {
        specs
            .iter()
            .map(|(path, lines)| SourceFileInput {
                path: (*path).into(),
                lines: *lines,
            })
            .collect()
    }

    #[test]
    fn under_threshold_produces_no_findings() {
        let files = files(&[("a.rs", 100), ("b.rs", 999)]);
        assert!(large_file_findings(&files, &RuleThresholds::default(), "t").is_empty());
    }

    #[test]
    fn over_threshold_produces_open_warning_with_metadata() {
        let files = files(&[("big.rs", 2000)]);
        let findings = large_file_findings(&files, &RuleThresholds::default(), "t");
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].finding_type, "STRUCT/LARGE_FILE");
        assert_eq!(findings[0].severity, prism_core::Severity::Warning);
        assert_eq!(findings[0].status, FindingStatus::Open);
        assert_eq!(
            findings[0].metadata.get("lines").map(String::as_str),
            Some("2000")
        );
    }

    #[test]
    fn deterministic_ids_for_same_input() {
        let files = files(&[("big.rs", 2000)]);
        let a = large_file_findings(&files, &RuleThresholds::default(), "t1");
        let b = large_file_findings(&files, &RuleThresholds::default(), "t2");
        assert_eq!(a[0].id, b[0].id, "同一输入 ⇒ 同一 finding id");
    }
}
