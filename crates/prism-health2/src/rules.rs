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
}

impl Default for RuleThresholds {
    fn default() -> Self {
        RuleThresholds {
            large_file_lines: 1000,
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
