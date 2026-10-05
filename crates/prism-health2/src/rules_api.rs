//! M213 API 规则（§28/§41）：HTTP 观测 → M28 统一 Finding。
//!
//! 规则只消费注入的 `ApiObservation` 切片（§47 有界）；绝不产
//! ApiHealthScore（§28）。失败分类沿 Observation.failure（§9 六类不统一）。

use crate::model::Metric;
use crate::registry::{RuleContext, RuleOutcome};
use prism_api::FailureKind;
use prism_intel::evidence::{Evidence, EvidenceKind};
use prism_intel::finding::{Finding, FindingStatus};
use prism_intel::{Confidence, Origin};
use std::collections::BTreeMap;

fn metric(key: &str, value: u64, definition: &str) -> Metric {
    Metric {
        key: key.to_string(),
        value,
        unit: "count".to_string(),
        definition: definition.to_string(),
        producer: "prism-health2::rules_api".to_string(),
    }
}

fn evidence_for(observation: &prism_api::ApiObservation) -> Evidence {
    // §69 证据链起点：观测身份 + 失败分类（无 secret——redaction 在观测层完成）。
    Evidence::new(
        EvidenceKind::Other("api-observation".to_string()),
        None,
        None,
        Some(observation.observation_id.clone()),
        format!(
            "{} {} => status {:?}, failure {:?}, latency {} ms",
            observation.method.as_str(),
            observation.url,
            observation.status,
            observation.failure,
            observation.latency_ms
        ),
    )
}

/// API 规则（§41 UnexpectedStatusCode/EndpointFailure）：
/// HTTP 失败观测（status ≥ api_failure_status_min 或非状态类失败）。
pub fn unexpected_status_rule(ctx: &RuleContext) -> RuleOutcome {
    let mut outcome = RuleOutcome::default();
    let Some(observations) = ctx.api else {
        outcome
            .diagnostics
            .push("no api observations injected; rule skipped".to_string());
        return outcome;
    };
    if observations.is_empty() {
        outcome.diagnostics.push(
            "api observations empty; status analysis unavailable (not zero failures)".to_string(),
        );
        return outcome;
    }
    for observation in observations {
        if ctx.cancel.is_cancelled() {
            outcome.diagnostics.push("cancelled mid-rule".to_string());
            break;
        }
        if outcome.findings.len() >= ctx.thresholds.max_supply_findings {
            outcome
                .diagnostics
                .push("api status findings exceed cap; truncated".to_string());
            break;
        }
        let failed_status = observation
            .status
            .map(|code| code >= ctx.thresholds.api_failure_status_min)
            .unwrap_or(false);
        let non_status_failure = !matches!(
            observation.failure,
            FailureKind::None | FailureKind::HttpStatus | FailureKind::Cancelled
        );
        if !failed_status && !non_status_failure {
            continue;
        }
        let evidence = evidence_for(observation);
        let mut finding = Finding::new(
            "API/UNEXPECTED_STATUS",
            prism_core::Severity::Warning,
            FindingStatus::Open,
            format!(
                "{} {} => {:?}",
                observation.method.as_str(),
                observation.url,
                observation.failure
            ),
            format!(
                "status {:?}; failure {:?}; latency {} ms (observation, not root cause)",
                observation.status, observation.failure, observation.latency_ms
            ),
            Origin::Fact,
            Confidence::High,
            None,
            vec![evidence.id.clone()],
        );
        if let Some(status) = observation.status {
            finding
                .metadata
                .insert("status".to_string(), status.to_string());
        }
        if let Some(correlation) = &observation.correlation_id {
            finding
                .metadata
                .insert("correlation_id".to_string(), correlation.clone());
        }
        finding.metadata.insert(
            "failure_kind".to_string(),
            format!("{:?}", observation.failure).to_lowercase(),
        );
        outcome.evidence.push(evidence);
        outcome.findings.push(finding);
    }
    let failures = observations
        .iter()
        .filter(|observation| observation.failure != FailureKind::None)
        .count() as u64;
    outcome.metrics.push(metric(
        "api.failure_observation.count",
        failures,
        "failed API observations in injected set (scope-bound)",
    ));
    outcome
}

/// API 规则（§28 RepeatedRequestFailure）：同 (method,url) 失败 ≥ 阈值 ⇒
/// 单条聚合 finding（§46 重复抑制：N 次失败 = 1 finding + 次数）。
pub fn repeated_failure_rule(ctx: &RuleContext) -> RuleOutcome {
    let mut outcome = RuleOutcome::default();
    let Some(observations) = ctx.api else {
        outcome
            .diagnostics
            .push("no api observations injected; rule skipped".to_string());
        return outcome;
    };
    let mut failures: BTreeMap<String, u64> = BTreeMap::new();
    if observations.is_empty() {
        outcome.diagnostics.push(
            "api observations empty; failure aggregation unavailable (not zero failures)"
                .to_string(),
        );
        return outcome;
    }
    for observation in observations {
        if observation.failure != FailureKind::None && observation.failure != FailureKind::Cancelled
        {
            *failures
                .entry(format!(
                    "{} {}",
                    observation.method.as_str(),
                    observation.url
                ))
                .or_insert(0) += 1;
        }
    }
    for (key, count) in &failures {
        if *count < ctx.thresholds.api_repeated_failures {
            continue;
        }
        if ctx.cancel.is_cancelled() {
            break;
        }
        if outcome.findings.len() >= ctx.thresholds.max_supply_findings {
            outcome
                .diagnostics
                .push("repeated failure findings exceed cap; truncated".to_string());
            break;
        }
        let finding = Finding::new(
            "API/REPEATED_FAILURE",
            prism_core::Severity::Warning,
            FindingStatus::Open,
            format!("Repeated failures: {key} (×{count})"),
            format!(
                "same request failed {count} time(s) in injected observation set — aggregation, not root-cause analysis"
            ),
            Origin::Fact,
            Confidence::High,
            None,
            vec![],
        );
        outcome.findings.push(finding);
    }
    outcome.metrics.push(metric(
        "api.repeated_failure.count",
        failures
            .values()
            .filter(|count| **count >= ctx.thresholds.api_repeated_failures)
            .count() as u64,
        "request keys with >= repeated-failure threshold (scope-bound)",
    ));
    outcome
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::RuleContext;
    use crate::RuleThresholds;
    use prism_api::HttpMethod;
    use prism_core::task::CancellationToken;

    fn context<'a>(
        observations: &'a [prism_api::ApiObservation],
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
            debug: None,
            evolution: None,
            api: Some(observations),
            cancel,
        }
    }

    fn observation(
        url: &str,
        status: Option<u16>,
        failure: FailureKind,
    ) -> prism_api::ApiObservation {
        prism_api::ApiObservation {
            observation_id: format!("obs:{url}:{status:?}"),
            method: HttpMethod::Get,
            url: url.to_string(),
            matched_endpoint: None,
            request_headers: Default::default(),
            request_body: None,
            status,
            response_headers: Default::default(),
            response_body: None,
            response_truncated: false,
            latency_ms: 42,
            failure,
            observed_at: "2026-10-06T00:00:00+00:00".to_string(),
            correlation_id: None,
        }
    }

    #[test]
    fn no_observations_is_diagnostic() {
        let cancel = CancellationToken::new();
        let thresholds = RuleThresholds::default();
        let empty: Vec<prism_api::ApiObservation> = Vec::new();
        let ctx = context(&empty, &thresholds, &cancel);
        for rule in [unexpected_status_rule, repeated_failure_rule] {
            let outcome = rule(&ctx);
            assert!(outcome.findings.is_empty());
            assert!(!outcome.diagnostics.is_empty(), "空观测必须如实诊断");
        }
    }

    #[test]
    fn http_500_produces_unexpected_status_finding() {
        let cancel = CancellationToken::new();
        let thresholds = RuleThresholds::default();
        let observations = vec![observation(
            "/api/cards/draw",
            Some(500),
            FailureKind::HttpStatus,
        )];
        let ctx = context(&observations, &thresholds, &cancel);
        let outcome = unexpected_status_rule(&ctx);
        assert_eq!(outcome.findings.len(), 1);
        assert_eq!(outcome.findings[0].finding_type, "API/UNEXPECTED_STATUS");
        assert_eq!(outcome.findings[0].severity, prism_core::Severity::Warning);
        assert_eq!(
            outcome.findings[0]
                .metadata
                .get("status")
                .map(String::as_str),
            Some("500")
        );
        assert_eq!(
            outcome.findings[0].origin,
            prism_intel::Origin::Fact,
            "HTTP 500 是 Runtime FACT（§5）"
        );
    }

    #[test]
    fn repeated_failures_aggregate_to_one_finding() {
        let cancel = CancellationToken::new();
        let thresholds = RuleThresholds::default();
        // 同一 URL 重复失败（观测 id 各异——事件独立，端点聚合）。
        let observations: Vec<prism_api::ApiObservation> = (0..5)
            .map(|index| prism_api::ApiObservation {
                observation_id: format!("obs:{index}"),
                ..observation("/api/x", Some(500), FailureKind::HttpStatus)
            })
            .collect();
        let ctx = context(&observations, &thresholds, &cancel);
        let outcome = repeated_failure_rule(&ctx);
        assert_eq!(outcome.findings.len(), 1, "5 次 = 1 聚合 finding（§46）");
        assert!(outcome.findings[0].title.contains("×5"));
    }

    #[test]
    fn http_200_is_not_a_failure() {
        let cancel = CancellationToken::new();
        let thresholds = RuleThresholds::default();
        let observations = vec![observation("/api/ok", Some(200), FailureKind::None)];
        let ctx = context(&observations, &thresholds, &cancel);
        assert!(unexpected_status_rule(&ctx).findings.is_empty());
    }
}
