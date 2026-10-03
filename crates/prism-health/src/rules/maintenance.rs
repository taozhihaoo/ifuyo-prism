//! MAINT001 — Maintenance Marker 规则（规格 M7 §42/§43/§111/§154；
//! 引擎验证载体：词法级、确定性、文件级）。
//!
//! 误报防线（§43/§154，以词法事实为准，无 parser 依赖）：
//! - 大写整词匹配：提取连续大写 run 并要求精确等于 marker
//!   （`todo` 不算、`TODOS`/`TODOs` 不算）
//! - 前导字符不是 `/`（排除 URL `http://x/TODO` 与路径）
//! - 行首必须是该语言已知的注释引导符（按扩展名映射）；
//!   未知扩展名整行跳过（保守：宁漏报不误报）
//!
//! Generated/过大文件不在管线内（M2 策略性跳过），天然不扫描；
//! §111 噪音控制由引擎 per-rule cap + GUI 过滤承担。

use crate::model::{
    Evidence, EvidenceKind, Finding, FindingCategory, FindingExplanation, FindingLocation,
    FindingStatus, RuleResult,
};
use crate::registry::{HealthRule, HealthRuleMeta};
use prism_core::severity::{Confidence, Severity};
use prism_core::task::CancellationToken;
use std::sync::LazyLock;

pub struct MaintenanceMarkerRule;

static META: LazyLock<HealthRuleMeta> = LazyLock::new(|| HealthRuleMeta {
    id: "MAINT001",
    name: "Maintenance Marker",
    category: FindingCategory::Maintenance,
    description:
        "Detects TODO / FIXME / HACK / XXX markers inside comments (lexical, false-positive averse).",
    default_severity: Severity::Info,
    default_enabled: true,
    threshold_meaning: None,
});

const MARKERS: [&str; 4] = ["TODO", "FIXME", "HACK", "XXX"];

/// 扩展名 → 该语言注释行引导符（小写扩展名）。未知扩展名 ⇒ 空集（跳过）。
fn comment_introducers(extension: Option<&str>) -> &'static [&'static str] {
    match extension {
        Some("ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" | "rs" | "cs" | "java" | "c" | "h"
        | "cpp" | "cc" | "hpp" | "go" | "scss" | "css") => &["//", "/*", "*"],
        Some("py" | "pyi" | "sh" | "bash" | "zsh" | "yaml" | "yml" | "toml") => &["#"],
        Some("sql") => &["--"],
        Some("html" | "htm" | "xml" | "vue") => &["<!--"],
        _ => &[],
    }
}

fn is_word_char(character: char) -> bool {
    character.is_alphanumeric() || character == '_'
}

/// 一行中第一个合格 marker 出现位置（整词 + 非路径 + 行首注释引导符）。
/// 返回 (marker, 1-based column)。
fn marker_occurrence<'a>(line: &'a str, introducers: &[&str]) -> Option<(&'a str, u32)> {
    let trimmed = line.trim_start();
    let introducer = introducers.iter().find(|intro| trimmed.starts_with(**intro))?;
    let mut cursor = line.len() - trimmed.len() + introducer.len();

    while cursor < line.len() {
        let Some(relative) = MARKERS
            .iter()
            .filter_map(|marker| line[cursor..].find(marker))
            .min()
        else {
            break;
        };
        let start = cursor + relative;
        let run_len = line[start..]
            .chars()
            .take_while(|character| character.is_ascii_uppercase())
            .map(char::len_utf8)
            .sum::<usize>();
        let run = &line[start..start + run_len];
        let before_ok = line[..start]
            .chars()
            .next_back()
            .is_none_or(|character| !is_word_char(character));
        let after_ok = line[start + run_len..]
            .chars()
            .next()
            .is_none_or(|character| !is_word_char(character));
        let not_url = start == 0 || !line[..start].ends_with('/');
        if MARKERS.contains(&run) && before_ok && after_ok && not_url {
            let column = line[..start].chars().count() as u32 + 1;
            return Some((run, column));
        }
        cursor = start + run_len.max(1);
    }
    None
}

impl HealthRule for MaintenanceMarkerRule {
    fn meta(&self) -> &'static HealthRuleMeta {
        &META
    }

    fn evaluate(
        &self,
        context: &crate::engine::HealthContext,
        _config: &prism_core::config::HealthRuleConfig,
        cancel: &CancellationToken,
    ) -> prism_core::PrismResult<RuleResult> {
        let mut result = RuleResult::default();
        // 不适用即安静跳过（§207/§208）：无代码模型 ⇒ 无词法扫描对象。
        let Some(code_model) = context.code_model else {
            return Ok(result);
        };

        for parse_result in &code_model.parse_results {
            if cancel.is_cancelled() {
                return Err(prism_core::PrismError::Cancelled);
            }
            // Generated / 策略性跳过文件不扫描（与 M2 管线语义一致）。
            if parse_result.status == prism_parser::ParseStatus::Skipped {
                continue;
            }
            let path = &parse_result.relative_path;
            let extension = std::path::Path::new(path)
                .extension()
                .map(|value| value.to_string_lossy().to_ascii_lowercase());
            let introducers = comment_introducers(extension.as_deref());
            if introducers.is_empty() {
                continue;
            }
            let Ok(content) = std::fs::read_to_string(context.root.join(path)) else {
                // 文件不可读（被删除/非 UTF-8）：词法扫描不适用，安静跳过（§208）。
                continue;
            };

            for (index, line) in content.lines().enumerate() {
                if cancel.is_cancelled() {
                    return Err(prism_core::PrismError::Cancelled);
                }
                if !(line.contains("TODO")
                    || line.contains("FIXME")
                    || line.contains("HACK")
                    || line.contains("XXX"))
                {
                    continue;
                }
                let Some((marker, column)) = marker_occurrence(line, introducers) else {
                    continue;
                };
                let line_number = index as u32 + 1;
                let excerpt: String = line.trim().chars().take(120).collect();
                let stable_key = format!("{path}:{line_number}:{column}:{marker}");
                result.findings.push(Finding {
                    id: Finding::derive_id("MAINT001", &stable_key),
                    rule_id: "MAINT001".to_string(),
                    category: FindingCategory::Maintenance,
                    severity: Severity::Info,
                    confidence: Confidence::High,
                    title: format!("Detected {marker} marker"),
                    message: format!("{marker} marker at {path}:{line_number}"),
                    explanation: FindingExplanation {
                        what: format!(
                            "A {marker} marker was detected in a comment on line {line_number}."
                        ),
                        evidence_summary: excerpt.clone(),
                        why_it_matters: Some(
                            "Marker comments often track unfinished or known-fragile work."
                                .to_string(),
                        ),
                    },
                    location: FindingLocation::new(path.clone(), line_number, column),
                    related_locations: vec![],
                    related_symbols: vec![],
                    related_modules: vec![],
                    related_files: vec![path.clone()],
                    evidence: vec![Evidence::new(
                        EvidenceKind::SourceLocation,
                        format!("{path}:{line_number}:{column}"),
                        excerpt,
                    )
                    .with_location(FindingLocation::new(path.clone(), line_number, column))],
                    suggested_action: Some("Resolve or schedule the marked work.".to_string()),
                    status: FindingStatus::Active,
                    metadata: [("marker".to_string(), marker.to_string())]
                        .into_iter()
                        .collect(),
                });
            }
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn introducers_are_language_scoped() {
        assert_eq!(comment_introducers(Some("ts")), &["//", "/*", "*"]);
        assert_eq!(comment_introducers(Some("py")), &["#"]);
        assert!(comment_introducers(Some("xyz")).is_empty());
        assert!(comment_introducers(None).is_empty());
    }

    #[test]
    fn marker_requires_comment_line_whole_word_and_non_url() {
        let ts = comment_introducers(Some("ts"));
        // 正例：注释行整词 TODO。
        assert_eq!(marker_occurrence("// TODO: extract helper", ts), Some(("TODO", 4)));
        assert_eq!(marker_occurrence("/* FIXME later */", ts), Some(("FIXME", 4)));
        // 负例（§43/§154）：
        assert_eq!(
            marker_occurrence("// see http://example.com/TODO here", ts),
            None,
            "URL path must not match"
        );
        assert_eq!(
            marker_occurrence("const label = \"TODO: fix\";", ts),
            None,
            "string literal line is not a comment line"
        );
        assert_eq!(
            marker_occurrence("// TODOS are tracked elsewhere", ts),
            None,
            "longer uppercase token must not match"
        );
        assert_eq!(
            marker_occurrence("// TODOs pending", ts),
            None,
            "trailing lowercase breaks the word boundary"
        );
        assert_eq!(
            marker_occurrence("export const TODO = 1;", ts),
            None,
            "non-comment line must not match even with a whole word"
        );
        // 行中间第二个 TODO 仍可命中（cursor 推进逻辑）。
        assert_eq!(
            marker_occurrence("// note TODO and FIXME", ts),
            Some(("TODO", 9))
        );
    }
}
