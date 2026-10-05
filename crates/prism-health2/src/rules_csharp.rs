//! C# 规则（2.0-M28 §B Symbol/Complexity + §E Semantic findings）。
//!
//! 输入 = 注入的 M23 抽取（FileExtraction.symbols 带 1-based range）与
//! 可选 M25 语义诊断；规则不重新解析源码（§2）。Finding id 内容寻址、
//! 无行号先例（同一符号变大变小不换 id；符号重命名 ⇒ 新 id，如实）。

use crate::model::Metric;
use crate::registry::{RuleContext, RuleOutcome};
use prism_csharp::SymbolKindCs;
use prism_intel::evidence::{Evidence, EvidenceKind, SourceRange};
use prism_intel::finding::{Finding, FindingStatus};
use prism_intel::identity::{EntityId, EntityKind};
use prism_intel::{Confidence, Origin};

fn metric(key: &str, value: u64, definition: &str) -> Metric {
    Metric {
        key: key.to_string(),
        value,
        unit: "count".to_string(),
        definition: definition.to_string(),
        producer: "prism-health2::rules_csharp".to_string(),
    }
}

fn is_type_kind(kind: SymbolKindCs) -> bool {
    matches!(
        kind,
        SymbolKindCs::Class
            | SymbolKindCs::Struct
            | SymbolKindCs::Interface
            | SymbolKindCs::Enum
            | SymbolKindCs::Record
            | SymbolKindCs::RecordStruct
    )
}

fn is_method_kind(kind: SymbolKindCs) -> bool {
    matches!(
        kind,
        SymbolKindCs::Method
            | SymbolKindCs::Constructor
            | SymbolKindCs::Destructor
            | SymbolKindCs::Operator
            | SymbolKindCs::LocalFunction
    )
}

fn cs_range(path: &str, range: &prism_csharp::SourceRangeCs) -> SourceRange {
    SourceRange {
        path: path.to_string(),
        start_line: range.start_line,
        start_column: range.start_column,
        end_line: range.end_line,
        end_column: range.end_column,
    }
}

fn line_span(range: &prism_csharp::SourceRangeCs) -> u64 {
    // 1-based 含首尾（空声明至少 1 行）。
    (range.end_line.saturating_sub(range.start_line) + 1) as u64
}

/// Complexity 规则：超大类型 / 超大方法（§B 文件级之上的符号级 size；
/// 阈值可配置）。度量 max_type_lines / max_method_lines 同步产出。
pub fn large_type_rule(ctx: &RuleContext) -> RuleOutcome {
    let mut outcome = RuleOutcome::default();
    let Some(facts) = &ctx.csharp else {
        outcome
            .diagnostics
            .push("no C# facts injected; rule skipped".to_string());
        return outcome;
    };
    let mut types_total = 0u64;
    let mut methods_total = 0u64;
    let mut max_type_lines = 0u64;
    let mut max_method_lines = 0u64;
    for extraction in facts.extractions.values() {
        if ctx.cancel.is_cancelled() {
            outcome.diagnostics.push("cancelled mid-rule".to_string());
            break;
        }
        for symbol in &extraction.symbols {
            if is_type_kind(symbol.kind) {
                types_total += 1;
            } else if is_method_kind(symbol.kind) {
                methods_total += 1;
            } else {
                continue;
            }
            let lines = line_span(&symbol.range);
            let is_type = is_type_kind(symbol.kind);
            if is_type {
                max_type_lines = max_type_lines.max(lines);
            } else {
                max_method_lines = max_method_lines.max(lines);
            }
            let threshold = if is_type {
                ctx.thresholds.large_type_lines
            } else {
                ctx.thresholds.large_method_lines
            };
            if lines <= threshold {
                continue;
            }
            let kind_label = if is_type { "type" } else { "method" };
            let finding_type = if is_type {
                "COMPLEX/LARGE_TYPE"
            } else {
                "COMPLEX/LARGE_METHOD"
            };
            let entity_id = EntityId::new("health2", &EntityKind::symbol(), &symbol.qualified_name);
            let evidence = Evidence::new(
                EvidenceKind::SourceRange,
                Some(symbol.file.clone()),
                Some(cs_range(&symbol.file, &symbol.range)),
                None,
                format!("{kind_label} spans {lines} lines (M23 extraction range)"),
            );
            let mut finding = Finding::new(
                finding_type,
                prism_core::Severity::Warning,
                FindingStatus::Open,
                format!(
                    "Large {kind_label} {} ({lines} lines)",
                    symbol.qualified_name
                ),
                format!(
                    "{} `{}` spans {lines} lines (threshold {threshold})",
                    symbol.file, symbol.qualified_name
                ),
                Origin::Fact,
                Confidence::High,
                Some(entity_id),
                vec![evidence.id.clone()],
            );
            finding
                .metadata
                .insert("path".to_string(), symbol.file.clone());
            finding
                .metadata
                .insert("lines".to_string(), lines.to_string());
            finding
                .metadata
                .insert("observed_at".to_string(), ctx.observed_at.to_string());
            outcome.evidence.push(evidence);
            outcome.findings.push(finding);
        }
    }
    outcome.metrics.push(metric(
        "csharp_types_total",
        types_total,
        "type declarations across injected extractions",
    ));
    outcome.metrics.push(metric(
        "csharp_methods_total",
        methods_total,
        "method-like declarations across injected extractions",
    ));
    outcome.metrics.push(metric(
        "max_type_lines",
        max_type_lines,
        "largest type declaration span in lines",
    ));
    outcome.metrics.push(metric(
        "max_method_lines",
        max_method_lines,
        "largest method-like declaration span in lines",
    ));
    outcome
}

/// Semantic 规则（§E Diagnostics → Findings）：M25 引擎诊断按
/// (kind, file) 聚合——避免 finding 爆炸（§Finding Explosion），同时
/// 每组保留至多 5 条 SourceRange 证据。组数上限可配置，超出记诊断。
pub fn semantic_diagnostic_rule(ctx: &RuleContext) -> RuleOutcome {
    let mut outcome = RuleOutcome::default();
    let Some(facts) = &ctx.csharp else {
        outcome
            .diagnostics
            .push("no C# facts injected; rule skipped".to_string());
        return outcome;
    };
    let Some(engine) = facts.semantics else {
        outcome
            .diagnostics
            .push("no semantic engine injected; rule skipped".to_string());
        return outcome;
    };
    // (kind, file) → (count, ranges)（BTreeMap ⇒ 确定性组序）。
    let mut groups: std::collections::BTreeMap<
        (String, String),
        (u64, Vec<prism_csharp::SemanticDiagnostic>),
    > = std::collections::BTreeMap::new();
    for diagnostic in engine.diagnostics() {
        let entry = groups
            .entry((diagnostic.kind.clone(), diagnostic.file.clone()))
            .or_insert((0, Vec::new()));
        entry.0 += 1;
        if entry.1.len() < 5 {
            entry.1.push(diagnostic.clone());
        }
    }
    let mut total = 0u64;
    for ((kind, file), (count, samples)) in &groups {
        total += count;
        if outcome.findings.len() >= ctx.thresholds.max_semantic_groups {
            outcome.diagnostics.push(format!(
                "semantic diagnostic groups exceed cap {}; remaining groups suppressed (finding explosion guard)",
                ctx.thresholds.max_semantic_groups
            ));
            break;
        }
        if ctx.cancel.is_cancelled() {
            outcome.diagnostics.push("cancelled mid-rule".to_string());
            break;
        }
        let severity = match kind.as_str() {
            "ambiguous_reference" => prism_core::Severity::Warning,
            _ => prism_core::Severity::Info,
        };
        let evidence_list: Vec<Evidence> = samples
            .iter()
            .map(|sample| {
                Evidence::new(
                    EvidenceKind::SourceRange,
                    Some(file.clone()),
                    Some(cs_range(file, &sample.range)),
                    None,
                    format!("M25 semantic diagnostic `{kind}`"),
                )
            })
            .collect();
        let entity_id = EntityId::new("health2", &EntityKind::file(), file);
        let mut finding = Finding::new(
            format!("SEMANTIC/{kind}"),
            severity,
            FindingStatus::Open,
            format!("{kind} ×{count} in {file}"),
            samples
                .first()
                .map(|sample| sample.message.clone())
                .unwrap_or_default(),
            Origin::Fact,
            Confidence::High,
            Some(entity_id),
            evidence_list.iter().map(|e| e.id.clone()).collect(),
        );
        finding.metadata.insert("file".to_string(), file.clone());
        finding
            .metadata
            .insert("occurrences".to_string(), count.to_string());
        finding
            .metadata
            .insert("observed_at".to_string(), ctx.observed_at.to_string());
        outcome.evidence.extend(evidence_list);
        outcome.findings.push(finding);
    }
    outcome.metrics.push(metric(
        "semantic_diagnostics_total",
        total,
        "M25 semantic diagnostics across injected engine",
    ));
    outcome
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::CSharpFacts;

    use crate::RuleThresholds;
    use prism_core::task::CancellationToken;
    use prism_csharp::{SemanticEngine, SymbolEngine};
    use std::collections::BTreeMap;

    fn context<'a>(
        extractions: &'a BTreeMap<String, prism_csharp::FileExtraction>,
        semantics: Option<&'a SemanticEngine>,
        thresholds: &'a RuleThresholds,
        cancel: &'a CancellationToken,
    ) -> RuleContext<'a> {
        RuleContext {
            files: &[],
            thresholds,
            observed_at: "t",
            csharp: Some(CSharpFacts {
                extractions,
                semantics,
            }),
            godot: None,
            supply: None,
            debug: None,
            evolution: None,
            api: None,
            cancel,
        }
    }

    fn type_extraction(file: &str, total_lines: u32) -> prism_csharp::FileExtraction {
        let mut engine = SymbolEngine::new();
        let body: String = (1..=total_lines)
            .map(|i| match i {
                1 => "class Big {".to_string(),
                i if i == total_lines => "}".to_string(),
                _ => "// filler".to_string(),
            })
            .collect::<Vec<_>>()
            .join("\n");
        engine.update_file(file, &body);
        engine.extraction(file).unwrap().clone()
    }

    #[test]
    fn large_type_produces_finding_with_evidence_and_metrics() {
        let cancel = CancellationToken::new();
        let mut extractions = BTreeMap::new();
        extractions.insert(
            "Assets/Big.cs".to_string(),
            type_extraction("Assets/Big.cs", 500),
        );
        let thresholds = RuleThresholds::default();
        let ctx = context(&extractions, None, &thresholds, &cancel);
        let outcome = large_type_rule(&ctx);
        assert_eq!(outcome.findings.len(), 1, "500 行 > 默认 400 阈值");
        assert_eq!(outcome.findings[0].finding_type, "COMPLEX/LARGE_TYPE");
        assert_eq!(outcome.findings[0].evidence.len(), 1);
        assert_eq!(outcome.evidence.len(), 1);
        assert_eq!(
            outcome
                .metrics
                .iter()
                .find(|m| m.key == "max_type_lines")
                .unwrap()
                .value,
            500
        );
    }

    #[test]
    fn small_type_produces_metrics_only() {
        let cancel = CancellationToken::new();
        let mut extractions = BTreeMap::new();
        extractions.insert(
            "Assets/Small.cs".to_string(),
            type_extraction("Assets/Small.cs", 30),
        );
        let thresholds = RuleThresholds::default();
        let ctx = context(&extractions, None, &thresholds, &cancel);
        let outcome = large_type_rule(&ctx);
        assert!(outcome.findings.is_empty());
        assert_eq!(
            outcome
                .metrics
                .iter()
                .find(|m| m.key == "csharp_types_total")
                .unwrap()
                .value,
            1
        );
    }

    #[test]
    fn semantic_diagnostics_aggregate_per_kind_and_file() {
        let cancel = CancellationToken::new();
        // dynamic 参数接收者 ⇒ M25 诚实降级诊断（局部 dynamic 不在
        // M25 检测面内——参数/字段才检测，沿 M25 语义）。
        let source = "class A { void M(dynamic x) { x.Foo(); x.Bar(); } }";
        let mut engine = SymbolEngine::new();
        engine.update_file("Assets/A.cs", source);
        let index = engine.build_index();
        let mut semantics = SemanticEngine::new();
        semantics.update_source(&index, "Assets/A.cs", source);
        semantics.rebuild(&index);
        let extractions = BTreeMap::new();
        let thresholds = RuleThresholds::default();
        let ctx = context(&extractions, Some(&semantics), &thresholds, &cancel);
        let outcome = semantic_diagnostic_rule(&ctx);
        assert!(
            !outcome.findings.is_empty(),
            "dynamic 调用应产出 SEMANTIC finding"
        );
        assert_eq!(
            outcome.findings.len(),
            1,
            "同 (kind,file) 聚合为一条 finding"
        );
        assert!(outcome.findings[0].finding_type.starts_with("SEMANTIC/"));
        assert!(!outcome.findings[0].evidence.is_empty());
    }

    #[test]
    fn empty_input_is_no_findings() {
        let cancel = CancellationToken::new();
        let extractions = BTreeMap::new();
        let thresholds = RuleThresholds::default();
        let ctx = context(&extractions, None, &thresholds, &cancel);
        assert!(large_type_rule(&ctx).findings.is_empty());
        assert!(semantic_diagnostic_rule(&ctx).findings.is_empty());
    }
}
