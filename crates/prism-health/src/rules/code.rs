//! 代码规则（规格 M7 §36–§41/§115/§152/§153/§212/§216/§218）。
//!
//! 全部消费共享 CodeMetrics（§93 一次计算）；阈值为产品默认值 + 配置
//! 覆盖（evidence 携带 value 与 threshold，§218）；度量未定义（如 Python
//! 嵌套）⇒ 安静跳过（§115 NotApplicable），不猜测。

use crate::derived::SymbolMetrics;
use crate::model::{
    Evidence, EvidenceKind, Finding, FindingCategory, FindingExplanation, FindingLocation,
    FindingStatus, RuleResult,
};
use crate::registry::{HealthRule, HealthRuleMeta};
use prism_core::severity::{Confidence, Severity};
use prism_core::task::CancellationToken;
use prism_core::PrismError;
use prism_parser::SymbolKind;
use std::sync::LazyLock;

pub const DEFAULT_COMPLEXITY_THRESHOLD: u64 = 10;
pub const DEFAULT_LARGE_FUNCTION_LINES: u64 = 200;
pub const DEFAULT_LARGE_TYPE_LINES: u64 = 400;
pub const DEFAULT_NESTING_DEPTH: u64 = 5;

struct CodeRuleSpec {
    meta: &'static HealthRuleMeta,
    default_threshold: u64,
    /// None = 使用配置/默认；Some(v) = 配置覆盖（§218）。
    applies_to: fn(SymbolKind) -> bool,
    pick: fn(&SymbolMetrics) -> Option<u64>,
    value_label: &'static str,
}

fn complexity_pick(metric: &SymbolMetrics) -> Option<u64> {
    // 明确定义（§37）：complexity = 1 + 分支关键词/操作符计数（词法近似）。
    Some(metric.branch_count + 1)
}

fn line_count_pick(metric: &SymbolMetrics) -> Option<u64> {
    Some(metric.line_count)
}

fn nesting_pick(metric: &SymbolMetrics) -> Option<u64> {
    metric.max_brace_nesting
}

fn is_callable(kind: SymbolKind) -> bool {
    matches!(kind, SymbolKind::Function | SymbolKind::Method | SymbolKind::Constructor)
}

fn is_type(kind: SymbolKind) -> bool {
    matches!(
        kind,
        SymbolKind::Class | SymbolKind::Struct | SymbolKind::Interface | SymbolKind::Enum
    )
}

fn evaluate_code_rule(
    context: &crate::engine::HealthContext,
    configured: Option<u64>,
    cancel: &CancellationToken,
    spec: &CodeRuleSpec,
) -> prism_core::PrismResult<RuleResult> {
    let mut result = RuleResult::default();
    let Some(code_model) = context.code_model else {
        return Ok(result);
    };
    let Some(derived) = context.derived else {
        return Ok(result);
    };
    let threshold = configured.unwrap_or(spec.default_threshold);

    for parse_result in &code_model.parse_results {
        for symbol in &parse_result.symbols {
            if cancel.is_cancelled() {
                return Err(PrismError::Cancelled);
            }
            if !(spec.applies_to)(symbol.kind) {
                continue;
            }
            let Some(metric) = derived.symbol_metrics.get(symbol.id.as_str()) else {
                continue;
            };
            let Some(value) = (spec.pick)(metric) else {
                continue;
            };
            if value <= threshold {
                continue;
            }
            let path = &parse_result.relative_path;
            result.findings.push(Finding {
                id: Finding::derive_id(
                    spec.meta.id,
                    &format!("{path}#{}#{}", symbol.name, symbol.range.start.line),
                ),
                rule_id: spec.meta.id.to_string(),
                category: FindingCategory::Code,
                severity: spec.meta.default_severity,
                confidence: Confidence::High,
                title: format!("Detected {} exceeding threshold", spec.meta.name),
                message: format!(
                    "{}: {value_label} = {value} (threshold {threshold}) at {path}:{}.",
                    spec.meta.name,
                    symbol.range.start.line,
                    value_label = spec.value_label
                ),
                explanation: FindingExplanation {
                    what: format!(
                        "{} of {} = {value}, exceeding the configured threshold of {threshold}.",
                        spec.value_label,
                        symbol.name
                    ),
                    evidence_summary: format!(
                        "{} = {value}; threshold = {threshold}",
                        spec.value_label
                    ),
                    why_it_matters: Some(
                        "Metrics beyond a configured threshold often mark a candidate for review; this is a factual signal, not a quality verdict.".to_string(),
                    ),
                },
                location: FindingLocation::new(path.clone(), symbol.range.start.line, 1),
                related_locations: vec![],
                related_symbols: vec![symbol.id.as_str().to_string()],
                related_modules: vec![],
                related_files: vec![path.clone()],
                evidence: vec![Evidence::new(
                    EvidenceKind::Symbol,
                    symbol.id.as_str().to_string(),
                    format!("{} = {value}; threshold = {threshold}", spec.value_label),
                )
                .with_location(FindingLocation::new(
                    path.clone(),
                    symbol.range.start.line,
                    1,
                ))],
                suggested_action: Some(
                    "Review the symbol against the configured threshold.".to_string(),
                ),
                status: FindingStatus::Active,
            change_relation: None,
                metadata: [
                    ("value".to_string(), value.to_string()),
                    ("threshold".to_string(), threshold.to_string()),
                ]
                .into_iter()
                .collect(),
            });
        }
    }
    Ok(result)
}

// ---------- 具体规则 ----------

static CODE001: LazyLock<HealthRuleMeta> = LazyLock::new(|| HealthRuleMeta {
    id: "CODE001",
    name: "High Complexity",
    category: FindingCategory::Code,
    description: "Function complexity (1 + branch keyword/operator occurrences) exceeds the configured threshold.",
    default_severity: Severity::Warning,
    default_enabled: true,
    threshold_meaning: Some("complexity threshold (1 + branch keywords/operators in function body)"),
    needs_derived: true,
});

static CODE002: LazyLock<HealthRuleMeta> = LazyLock::new(|| HealthRuleMeta {
    id: "CODE002",
    name: "Very Large Function",
    category: FindingCategory::Code,
    description: "Function source-line count exceeds the configured threshold.",
    default_severity: Severity::Warning,
    default_enabled: true,
    threshold_meaning: Some("function source-line threshold"),
    needs_derived: true,
});

static CODE003: LazyLock<HealthRuleMeta> = LazyLock::new(|| HealthRuleMeta {
    id: "CODE003",
    name: "Very Large Type",
    category: FindingCategory::Code,
    description: "Type (class/struct/interface/enum) source-line count exceeds the configured threshold.",
    default_severity: Severity::Info,
    default_enabled: true,
    threshold_meaning: Some("type source-line threshold"),
    needs_derived: true,
});

static CODE004: LazyLock<HealthRuleMeta> = LazyLock::new(|| HealthRuleMeta {
    id: "CODE004",
    name: "Excessive Nesting",
    category: FindingCategory::Code,
    description: "Maximum brace nesting depth inside a function exceeds the configured threshold (brace languages).",
    default_severity: Severity::Warning,
    default_enabled: true,
    threshold_meaning: Some("maximum brace nesting depth threshold"),
    needs_derived: true,
});

pub struct HighComplexityRule;

impl HealthRule for HighComplexityRule {
    fn meta(&self) -> &'static HealthRuleMeta {
        &CODE001
    }

    fn evaluate(
        &self,
        context: &crate::engine::HealthContext,
        config: &prism_core::config::HealthRuleConfig,
        cancel: &CancellationToken,
    ) -> prism_core::PrismResult<RuleResult> {
        evaluate_code_rule(
            context,
            config.threshold,
            cancel,
            &CodeRuleSpec {
                meta: &CODE001,
                default_threshold: DEFAULT_COMPLEXITY_THRESHOLD,
                applies_to: is_callable,
                pick: complexity_pick,
                value_label: "complexity",
            },
        )
    }
}

pub struct LargeFunctionRule;

impl HealthRule for LargeFunctionRule {
    fn meta(&self) -> &'static HealthRuleMeta {
        &CODE002
    }

    fn evaluate(
        &self,
        context: &crate::engine::HealthContext,
        config: &prism_core::config::HealthRuleConfig,
        cancel: &CancellationToken,
    ) -> prism_core::PrismResult<RuleResult> {
        evaluate_code_rule(
            context,
            config.threshold,
            cancel,
            &CodeRuleSpec {
                meta: &CODE002,
                default_threshold: DEFAULT_LARGE_FUNCTION_LINES,
                applies_to: is_callable,
                pick: line_count_pick,
                value_label: "source lines",
            },
        )
    }
}

pub struct LargeTypeRule;

impl HealthRule for LargeTypeRule {
    fn meta(&self) -> &'static HealthRuleMeta {
        &CODE003
    }

    fn evaluate(
        &self,
        context: &crate::engine::HealthContext,
        config: &prism_core::config::HealthRuleConfig,
        cancel: &CancellationToken,
    ) -> prism_core::PrismResult<RuleResult> {
        evaluate_code_rule(
            context,
            config.threshold,
            cancel,
            &CodeRuleSpec {
                meta: &CODE003,
                default_threshold: DEFAULT_LARGE_TYPE_LINES,
                applies_to: is_type,
                pick: line_count_pick,
                value_label: "source lines",
            },
        )
    }
}

pub struct ExcessiveNestingRule;

impl HealthRule for ExcessiveNestingRule {
    fn meta(&self) -> &'static HealthRuleMeta {
        &CODE004
    }

    fn evaluate(
        &self,
        context: &crate::engine::HealthContext,
        config: &prism_core::config::HealthRuleConfig,
        cancel: &CancellationToken,
    ) -> prism_core::PrismResult<RuleResult> {
        evaluate_code_rule(
            context,
            config.threshold,
            cancel,
            &CodeRuleSpec {
                meta: &CODE004,
                default_threshold: DEFAULT_NESTING_DEPTH,
                applies_to: is_callable,
                pick: nesting_pick,
                value_label: "maximum brace nesting depth",
            },
        )
    }
}

