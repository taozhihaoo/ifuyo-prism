//! M210 Debug 规则（§二十四/§二十六/§四十六）。
//!
//! 与 M28/M29 同一挂载点；只消费注入的 `DebugReport` 事实。重复事件
//! 抑制在签名层已完成（1000 次异常 = 1 签名 ⇒ 1 finding + 次数，§四十六
//! 不逐条产 finding）。前置缺失 ⇒ 诊断（§二十六 prerequisite）。

use crate::model::Metric;
use crate::registry::{RuleContext, RuleOutcome};
use prism_intel::evidence::{Evidence, EvidenceKind};
use prism_intel::finding::{Finding, FindingStatus};
use prism_intel::{Confidence, Origin};

fn metric(key: &str, value: u64, definition: &str) -> Metric {
    Metric {
        key: key.to_string(),
        value,
        unit: "count".to_string(),
        definition: definition.to_string(),
        producer: "prism-health2::rules_debug".to_string(),
    }
}

fn evidence_for(error: &prism_debug::DebugError) -> Evidence {
    // 证据链（§二十七）：源 + 行号例证 + 归一签名（redacted 表示）。
    let locations = error
        .occurrences_detail
        .iter()
        .map(|(source_id, line)| format!("{source_id}:{line}"))
        .collect::<Vec<_>>()
        .join(", ");
    Evidence::new(
        EvidenceKind::Other("debug-log".to_string()),
        None,
        None,
        Some(locations),
        format!(
            "normalized signature `{}` ×{} occurrence(s)",
            error.normalized_message, error.occurrences
        ),
    )
}

/// Runtime 规则（§二十四）：RepeatedRuntimeError（次数 ≥ 阈值）与
/// CrashSignature（未处理异常/严重级别）。
pub fn runtime_error_rule(ctx: &RuleContext) -> RuleOutcome {
    let mut outcome = RuleOutcome::default();
    let Some(report) = ctx.debug else {
        outcome
            .diagnostics
            .push("no debug report injected; rule skipped".to_string());
        return outcome;
    };
    if ctx.cancel.is_cancelled() {
        outcome.diagnostics.push("cancelled mid-rule".to_string());
        return outcome;
    }
    if report.events_total == 0 && report.sources.is_empty() {
        // §二.7：没有日志 ≠ 没有错误——诊断表达数据缺失。
        outcome.diagnostics.push(
            "debug sources empty; runtime analysis unavailable (not zero errors)".to_string(),
        );
    }
    for error in &report.errors {
        if error.domain != prism_debug::EventDomain::Runtime {
            continue;
        }
        if outcome.findings.len() >= ctx.thresholds.max_supply_findings {
            outcome
                .diagnostics
                .push("runtime error findings exceed cap; truncated".to_string());
            break;
        }
        // CrashSignature 判据 = 日志原文含 Unhandled exception（§24）。
        let unhandled = error.example_message.contains("Unhandled exception.");
        let repeated = error.occurrences >= ctx.thresholds.repeated_error_occurrences;
        if !unhandled && !repeated {
            continue;
        }
        let (finding_type, severity) = if unhandled {
            ("DEBUG/CRASH_SIGNATURE", prism_core::Severity::Critical)
        } else {
            (
                "DEBUG/REPEATED_RUNTIME_ERROR",
                prism_core::Severity::Warning,
            )
        };
        let evidence = evidence_for(error);
        let mut finding = Finding::new(
            finding_type,
            severity,
            FindingStatus::Open,
            format!(
                "{} ({occurrences} occurrence(s))",
                error.exception_type.as_deref().unwrap_or("Runtime error"),
                occurrences = error.occurrences
            ),
            format!(
                "normalized: {} — first/last observed in this analysis scope",
                error.normalized_message
            ),
            Origin::Fact,
            Confidence::High,
            None,
            vec![evidence.id.clone()],
        );
        // Observed-scope 表述（§二十一：第一次观察到 ≠ 第一次发生）。
        finding.metadata.insert(
            "first_observed".to_string(),
            error.first_observed.clone().unwrap_or_default(),
        );
        finding.metadata.insert(
            "last_observed".to_string(),
            error.last_observed.clone().unwrap_or_default(),
        );
        finding
            .metadata
            .insert("occurrences".to_string(), error.occurrences.to_string());
        if let Some(trace) = &error.stack_trace {
            let top = trace.frames.first();
            if let Some(frame) = top {
                finding.metadata.insert(
                    "top_frame".to_string(),
                    format!(
                        "{}:{}",
                        frame.file_path.clone().unwrap_or_default(),
                        frame.line.map(|l| l.to_string()).unwrap_or_default()
                    ),
                );
                if let Some(scene) = &frame.resolution.scene {
                    finding.metadata.insert("scene".to_string(), scene.clone());
                }
                if let Some(node) = &frame.resolution.node_path {
                    finding
                        .metadata
                        .insert("node_path".to_string(), node.clone());
                }
                if let Some(symbol) = &frame.resolution.symbol_id {
                    finding
                        .metadata
                        .insert("symbol_id".to_string(), symbol.clone());
                }
            }
        }
        outcome.evidence.push(evidence);
        outcome.findings.push(finding);
    }
    let runtime_errors = report
        .errors
        .iter()
        .filter(|error| error.domain == prism_debug::EventDomain::Runtime)
        .count() as u64;
    outcome.metrics.push(metric(
        "debug.unique_runtime_error.count",
        runtime_errors,
        "distinct runtime error signatures in analyzed sources (scope-bound)",
    ));
    outcome
}

/// Build/Test 规则（§十四/§十六）：BuildFailure / TestFailure —— 一次
/// 运行的失败事实；RepeatedTestFailure 需要历史对比（v1 经基线链）。
pub fn build_test_failure_rule(ctx: &RuleContext) -> RuleOutcome {
    let mut outcome = RuleOutcome::default();
    let Some(report) = ctx.debug else {
        outcome
            .diagnostics
            .push("no debug report injected; rule skipped".to_string());
        return outcome;
    };
    let mut builds = 0u64;
    let mut tests = 0u64;
    for error in &report.errors {
        if ctx.cancel.is_cancelled() {
            outcome.diagnostics.push("cancelled mid-rule".to_string());
            break;
        }
        if outcome.findings.len() >= ctx.thresholds.max_supply_findings {
            outcome
                .diagnostics
                .push("build/test findings exceed cap; truncated".to_string());
            break;
        }
        let (finding_type, severity) = match error.domain {
            prism_debug::EventDomain::Build => {
                builds += 1;
                ("DEBUG/BUILD_FAILURE", prism_core::Severity::Warning)
            }
            prism_debug::EventDomain::Test => {
                tests += 1;
                ("DEBUG/TEST_FAILURE", prism_core::Severity::Warning)
            }
            _ => continue,
        };
        let evidence = evidence_for(error);
        let mut finding = Finding::new(
            finding_type,
            severity,
            FindingStatus::Open,
            format!(
                "{}: {}",
                match error.domain {
                    prism_debug::EventDomain::Build => "Build failure",
                    _ => "Test failure",
                },
                error
                    .normalized_message
                    .chars()
                    .take(80)
                    .collect::<String>()
            ),
            format!("normalized: {}", error.normalized_message),
            Origin::Fact,
            Confidence::High,
            None,
            vec![evidence.id.clone()],
        );
        if let Some(code) = &error.code {
            finding.metadata.insert("code".to_string(), code.clone());
        }
        if let Some(test_name) = report
            .events
            .iter()
            .find_map(|event| event.test_name.clone())
        {
            finding.metadata.insert("test".to_string(), test_name);
        }
        outcome.evidence.push(evidence);
        outcome.findings.push(finding);
    }
    outcome.metrics.push(metric(
        "debug.build_failure.count",
        builds,
        "distinct build failure signatures (scope-bound)",
    ));
    outcome.metrics.push(metric(
        "debug.test_failure.count",
        tests,
        "distinct test failure signatures (scope-bound)",
    ));
    outcome
}

/// Stack 规则（§十/§七十）：解析缺口如实上报（Unresolved 帧是有效
/// Debug 数据，但覆盖率作为 Metric + 诊断可见）。
pub fn stack_resolution_rule(ctx: &RuleContext) -> RuleOutcome {
    let mut outcome = RuleOutcome::default();
    let Some(report) = ctx.debug else {
        outcome
            .diagnostics
            .push("no debug report injected; rule skipped".to_string());
        return outcome;
    };
    let mut total_frames = 0u64;
    let mut resolved_frames = 0u64;
    let mut unresolved_with_symbol_engine = 0u64;
    for error in &report.errors {
        if let Some(trace) = &error.stack_trace {
            for frame in &trace.frames {
                total_frames += 1;
                if frame.resolution.status == prism_debug::ResolutionStatus::Resolved {
                    resolved_frames += 1;
                } else if report.symbol_resolution_available
                    && frame.resolution.status == prism_debug::ResolutionStatus::Unresolved
                {
                    unresolved_with_symbol_engine += 1;
                }
            }
        }
    }
    if unresolved_with_symbol_engine > 0 && ctx.thresholds.report_unresolved_frames {
        let finding = Finding::new(
            "DEBUG/UNRESOLVED_STACK_FRAME",
            prism_core::Severity::Info,
            FindingStatus::Open,
            format!("{unresolved_with_symbol_engine} stack frame(s) unresolved despite symbol index"),
            "frames could not be matched to M23 symbols (external code/overload/name drift); still valid debug data".to_string(),
            Origin::Fact,
            Confidence::High,
            None,
            vec![],
        );
        outcome.findings.push(finding);
    }
    outcome.metrics.push(metric(
        "debug.stack_frame.total",
        total_frames,
        "stack frames in parsed traces (numerator/denominator basis §70)",
    ));
    outcome.metrics.push(metric(
        "debug.stack_frame.resolved",
        resolved_frames,
        "frames resolved to M23 symbols (Exact/Strong/Probable)",
    ));
    outcome
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::RuleContext;
    use crate::RuleThresholds;
    use prism_core::task::CancellationToken;
    use prism_debug::DebugReport;

    fn context<'a>(
        report: Option<&'a DebugReport>,
        thresholds: &'a RuleThresholds,
        cancel: &'a CancellationToken,
    ) -> RuleContext<'a> {
        RuleContext {
            files: &[],
            thresholds,
            observed_at: "t",
            csharp: None,
            godot: None,
            supply: None,
            debug: report,
            evolution: None,
            cancel,
        }
    }

    fn runtime_error(occurrences: u64, unhandled: bool) -> prism_debug::DebugError {
        let message = if unhandled {
            "Unhandled exception. System.NullReferenceException: Object ref"
        } else {
            "connection refused"
        };
        prism_debug::DebugError {
            error_id: "err:test".to_string(),
            fingerprint: "fp:test".to_string(),
            domain: prism_debug::EventDomain::Runtime,
            exception_type: if unhandled {
                Some("System.NullReferenceException".to_string())
            } else {
                None
            },
            normalized_message: message.to_string(),
            example_message: message.to_string(),
            code: None,
            top_frames: vec![],
            stack_trace: None,
            occurrences,
            occurrences_detail: vec![("src:godot".to_string(), 5)],
            first_observed: None,
            last_observed: None,
            redaction_count: 0,
        }
    }

    #[test]
    fn no_debug_report_is_diagnostic() {
        let cancel = CancellationToken::new();
        let thresholds = RuleThresholds::default();
        let ctx = context(None, &thresholds, &cancel);
        for rule in [
            runtime_error_rule,
            build_test_failure_rule,
            stack_resolution_rule,
        ] {
            let outcome = rule(&ctx);
            assert!(outcome.findings.is_empty());
            assert!(!outcome.diagnostics.is_empty());
        }
    }

    #[test]
    fn repeated_runtime_error_produces_one_finding_with_occurrences() {
        let cancel = CancellationToken::new();
        let thresholds = RuleThresholds::default();
        let report = DebugReport {
            schema_version: 1,
            errors: vec![runtime_error(50, false)],
            ..DebugReport::default()
        };
        let ctx = context(Some(&report), &thresholds, &cancel);
        let outcome = runtime_error_rule(&ctx);
        assert_eq!(outcome.findings.len(), 1, "50 次 = 1 finding（§46 抑制）");
        assert_eq!(
            outcome.findings[0].finding_type,
            "DEBUG/REPEATED_RUNTIME_ERROR"
        );
        assert_eq!(outcome.findings[0].severity, prism_core::Severity::Warning);
        assert!(outcome.findings[0].title.contains("50 occurrence(s)"));
    }

    #[test]
    fn unhandled_exception_is_crash_signature_critical() {
        let cancel = CancellationToken::new();
        let thresholds = RuleThresholds::default();
        let report = DebugReport {
            schema_version: 1,
            errors: vec![runtime_error(1, true)],
            ..DebugReport::default()
        };
        let ctx = context(Some(&report), &thresholds, &cancel);
        let outcome = runtime_error_rule(&ctx);
        assert_eq!(outcome.findings[0].finding_type, "DEBUG/CRASH_SIGNATURE");
        assert_eq!(outcome.findings[0].severity, prism_core::Severity::Critical);
    }

    #[test]
    fn below_threshold_runtime_error_is_not_a_finding() {
        let cancel = CancellationToken::new();
        let thresholds = RuleThresholds::default();
        let report = DebugReport {
            schema_version: 1,
            errors: vec![runtime_error(2, false)],
            ..DebugReport::default()
        };
        let ctx = context(Some(&report), &thresholds, &cancel);
        let outcome = runtime_error_rule(&ctx);
        assert!(outcome.findings.is_empty(), "偶发错误不产 finding");
        assert_eq!(
            outcome
                .metrics
                .iter()
                .find(|m| m.key == "debug.unique_runtime_error.count")
                .unwrap()
                .value,
            1
        );
    }

    #[test]
    fn empty_sources_is_unavailable_not_zero() {
        let cancel = CancellationToken::new();
        let thresholds = RuleThresholds::default();
        let report = DebugReport::default();
        let ctx = context(Some(&report), &thresholds, &cancel);
        let outcome = runtime_error_rule(&ctx);
        assert!(outcome.findings.is_empty());
        assert!(outcome
            .diagnostics
            .iter()
            .any(|d| d.contains("not zero errors")));
    }

    #[test]
    fn build_and_test_failures() {
        let cancel = CancellationToken::new();
        let thresholds = RuleThresholds::default();
        let mut build_error = runtime_error(1, false);
        build_error.domain = prism_debug::EventDomain::Build;
        build_error.code = Some("CS0246".to_string());
        let mut test_error = runtime_error(1, false);
        test_error.domain = prism_debug::EventDomain::Test;
        let report = DebugReport {
            schema_version: 1,
            errors: vec![build_error, test_error],
            ..DebugReport::default()
        };
        let ctx = context(Some(&report), &thresholds, &cancel);
        let outcome = build_test_failure_rule(&ctx);
        assert_eq!(outcome.findings.len(), 2);
        assert!(outcome
            .findings
            .iter()
            .any(|f| f.finding_type == "DEBUG/BUILD_FAILURE"));
        assert!(outcome
            .findings
            .iter()
            .any(|f| f.finding_type == "DEBUG/TEST_FAILURE"));
        assert!(outcome
            .findings
            .iter()
            .any(|f| f.metadata.get("code").map(String::as_str) == Some("CS0246")));
    }
}
