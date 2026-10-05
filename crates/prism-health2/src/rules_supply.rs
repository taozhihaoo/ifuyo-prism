//! M29 供给规则（Dependency/Security/License/Environment，§44-§46）。
//!
//! 与 M28 规则同一挂载点：`fn(&RuleContext) -> RuleOutcome`，只消费注入
//! 的 `SupplyReport` 事实；不自扫描、不重跑解析（§47）。前置缺失
//! （supply 未注入 / advisory 不可用）⇒ 诊断如实记录，不产生"0 风险"
//! 假结论（§48/§105）。

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
        producer: "prism-health2::rules_supply".to_string(),
    }
}

/// Dependency 规则（§44）：Unresolved 声明（声明了但锁里没有）+ 版本
/// Mismatch/Conflict（Drift/NormalResolution 是事实不是 finding，§11）。
pub fn dependency_resolution_rule(ctx: &RuleContext) -> RuleOutcome {
    let mut outcome = RuleOutcome::default();
    let Some(supply) = ctx.supply else {
        outcome
            .diagnostics
            .push("no supply facts injected; rule skipped".to_string());
        return outcome;
    };
    if ctx.cancel.is_cancelled() {
        outcome.diagnostics.push("cancelled mid-rule".to_string());
        return outcome;
    }
    let mut unresolved = 0u64;
    for comparison in &supply.version_comparisons {
        match comparison.state {
            prism_supply::DriftState::Unresolved => {
                unresolved += 1;
                if outcome.findings.len() >= ctx.thresholds.max_supply_findings {
                    continue;
                }
                let evidence = Evidence::new(
                    EvidenceKind::Manifest,
                    None,
                    None,
                    Some(comparison.declaration_id.clone()),
                    "declared dependency absent from all parsed lockfiles".to_string(),
                );
                let finding = Finding::new(
                    "DEP/UNRESOLVED",
                    prism_core::Severity::Warning,
                    FindingStatus::Open,
                    format!("Unresolved dependency {}", comparison.package_id),
                    comparison.detail.clone(),
                    Origin::Fact,
                    Confidence::High,
                    None,
                    vec![evidence.id.clone()],
                );
                outcome.evidence.push(evidence);
                outcome.findings.push(finding);
            }
            prism_supply::DriftState::Mismatch | prism_supply::DriftState::Conflict => {
                if outcome.findings.len() >= ctx.thresholds.max_supply_findings {
                    continue;
                }
                let evidence = Evidence::new(
                    EvidenceKind::Manifest,
                    None,
                    None,
                    Some(comparison.declaration_id.clone()),
                    comparison.detail.clone(),
                );
                let finding = Finding::new(
                    "DEP/VERSION_MISMATCH",
                    prism_core::Severity::Warning,
                    FindingStatus::Open,
                    format!("Dependency version mismatch {}", comparison.package_id),
                    comparison.detail.clone(),
                    Origin::Fact,
                    Confidence::High,
                    None,
                    vec![evidence.id.clone()],
                );
                outcome.evidence.push(evidence);
                outcome.findings.push(finding);
            }
            _ => {}
        }
    }
    outcome.metrics.push(metric(
        "dependency.declared.count",
        supply.declarations.len() as u64,
        "dependency declarations parsed from manifests",
    ));
    outcome.metrics.push(metric(
        "dependency.resolved.count",
        supply.resolved.len() as u64,
        "resolved dependencies parsed from lockfiles",
    ));
    outcome.metrics.push(metric(
        "dependency.unresolved.count",
        unresolved,
        "declarations absent from lockfiles (state Unresolved)",
    ));
    outcome
}

/// Dependency 规则（§16）：项目引用环（证据 = 环路径原样）。
pub fn dependency_cycle_rule(ctx: &RuleContext) -> RuleOutcome {
    let mut outcome = RuleOutcome::default();
    let Some(supply) = ctx.supply else {
        outcome
            .diagnostics
            .push("no supply facts injected; rule skipped".to_string());
        return outcome;
    };
    for cycle in &supply.cycles {
        if ctx.cancel.is_cancelled() {
            outcome.diagnostics.push("cancelled mid-rule".to_string());
            break;
        }
        if outcome.findings.len() >= ctx.thresholds.max_supply_findings {
            outcome
                .diagnostics
                .push("cycle findings exceed cap; truncated".to_string());
            break;
        }
        let path_text = cycle.path.join(" → ");
        let evidence = Evidence::new(
            EvidenceKind::Manifest,
            None,
            None,
            Some(path_text.clone()),
            "project reference cycle path".to_string(),
        );
        let finding = Finding::new(
            "DEP/CYCLE",
            prism_core::Severity::Warning,
            FindingStatus::Open,
            format!(
                "Project reference cycle {}",
                cycle.path.first().cloned().unwrap_or_default()
            ),
            format!("cycle path: {path_text}"),
            Origin::Fact,
            Confidence::High,
            None,
            vec![evidence.id.clone()],
        );
        outcome.evidence.push(evidence);
        outcome.findings.push(finding);
    }
    outcome.metrics.push(metric(
        "dependency.cycle.count",
        supply.cycles.len() as u64,
        "project reference cycles detected",
    ));
    outcome
}

/// Security 规则（§32-§36）：SecretCandidate —— redaction 全链（finding
/// 标题/描述/证据只含 redacted 上下文与位置，绝无 secret 原文）。
pub fn secret_candidate_rule(ctx: &RuleContext) -> RuleOutcome {
    let mut outcome = RuleOutcome::default();
    let Some(supply) = ctx.supply else {
        outcome
            .diagnostics
            .push("no supply facts injected; rule skipped".to_string());
        return outcome;
    };
    if supply.secret_scan_truncated {
        outcome
            .diagnostics
            .push("secret scan truncated at candidate cap (finding explosion guard)".to_string());
    }
    for candidate in &supply.secret_candidates {
        if ctx.cancel.is_cancelled() {
            outcome.diagnostics.push("cancelled mid-rule".to_string());
            break;
        }
        let severity = match candidate.confidence {
            Confidence::High => prism_core::Severity::Critical,
            Confidence::Medium => prism_core::Severity::Warning,
            _ => prism_core::Severity::Info,
        };
        // 证据引用 = 位置 + 规则（不含内容，§35）。
        let evidence = Evidence::new(
            EvidenceKind::File,
            Some(candidate.path.clone()),
            None,
            Some(format!("{}:{}", candidate.path, candidate.line)),
            format!(
                "secret candidate rule `{}` — {}",
                candidate.rule, candidate.detection_reason
            ),
        );
        let mut finding = Finding::new(
            "SEC/SECRET_CANDIDATE",
            severity,
            FindingStatus::Open,
            format!(
                "Secret candidate ({}) at {}",
                candidate.detection_class, candidate.path
            ),
            format!(
                "{}; redacted context: {}",
                candidate.detection_reason, candidate.redacted_context
            ),
            Origin::Fact,
            candidate.confidence,
            None,
            vec![evidence.id.clone()],
        );
        finding
            .metadata
            .insert("rule".to_string(), candidate.rule.clone());
        finding
            .metadata
            .insert("line".to_string(), candidate.line.to_string());
        finding.metadata.insert(
            "detection_class".to_string(),
            candidate.detection_class.clone(),
        );
        if !candidate.exclusion_notes.is_empty() {
            finding.metadata.insert(
                "exclusion_notes".to_string(),
                candidate.exclusion_notes.join("; "),
            );
        }
        outcome.evidence.push(evidence);
        outcome.findings.push(finding);
    }
    outcome.metrics.push(metric(
        "security.secret_candidate.count",
        supply.secret_candidates.len() as u64,
        "secret candidates detected (redacted; not certainties)",
    ));
    outcome
}

/// Security 规则（§28/§30）：KnownVulnerability —— 只有 advisory 明确
/// 命中才是 Known；advisory 不可用 ⇒ 诊断（绝不"0 漏洞"，§48）。
pub fn known_vulnerability_rule(ctx: &RuleContext) -> RuleOutcome {
    let mut outcome = RuleOutcome::default();
    let Some(supply) = ctx.supply else {
        outcome
            .diagnostics
            .push("no supply facts injected; rule skipped".to_string());
        return outcome;
    };
    match supply.advisory_capability {
        Some(prism_supply::ProviderCapability::Supported) => {}
        Some(prism_supply::ProviderCapability::Unavailable) => {
            outcome
                .diagnostics
                .push("vulnerability analysis unavailable: advisory database not provided (not zero vulnerabilities)".to_string());
            return outcome;
        }
        _ => {
            outcome
                .diagnostics
                .push("vulnerability analysis state unknown (provider not Supported)".to_string());
            return outcome;
        }
    }
    for advisory_match in &supply.advisory_matches {
        if ctx.cancel.is_cancelled() {
            outcome.diagnostics.push("cancelled mid-rule".to_string());
            break;
        }
        if outcome.findings.len() >= ctx.thresholds.max_supply_findings {
            outcome
                .diagnostics
                .push("vulnerability findings exceed cap; truncated".to_string());
            break;
        }
        let severity = match advisory_match.advisory.severity_hint.as_str() {
            "critical" => prism_core::Severity::Critical,
            "high" | "moderate" => prism_core::Severity::Warning,
            _ => prism_core::Severity::Info,
        };
        let evidence = Evidence::new(
            EvidenceKind::Manifest,
            None,
            None,
            Some(format!(
                "advisory:{}@{}",
                advisory_match.advisory.id,
                supply.advisory_database_version.clone().unwrap_or_default()
            )),
            format!(
                "affected range(s) {:?} intersect resolved {} (db {})",
                advisory_match.advisory.affected_ranges,
                advisory_match.resolved_version,
                supply
                    .advisory_database_version
                    .clone()
                    .unwrap_or_else(|| "unknown".to_string())
            ),
        );
        // Remediation（§80）：advisory 给了 safe version 才推荐具体版本。
        let remediation = advisory_match
            .advisory
            .fixed_version
            .as_ref()
            .map(|version| format!("Update to >= {version}"))
            .unwrap_or_else(|| "Update dependency and review advisory".to_string());
        let mut finding = Finding::new(
            "SEC/KNOWN_VULNERABILITY",
            severity,
            FindingStatus::Open,
            format!(
                "Known vulnerability {}: {}@{}",
                advisory_match.advisory.id,
                advisory_match.advisory.package_id,
                advisory_match.resolved_version
            ),
            advisory_match.conclusion.clone(),
            Origin::Fact,
            Confidence::High,
            None,
            vec![evidence.id.clone()],
        );
        finding.remediation = Some(remediation);
        finding.metadata.insert(
            "advisory_source".to_string(),
            advisory_match.advisory.source.clone(),
        );
        finding.metadata.insert(
            "resolved_version".to_string(),
            advisory_match.resolved_version.clone(),
        );
        outcome.evidence.push(evidence);
        outcome.findings.push(finding);
    }
    outcome.metrics.push(metric(
        "security.known_vulnerability.count",
        supply.advisory_matches.len() as u64,
        "advisory-matched vulnerable dependencies (Known, not heuristic)",
    ));
    outcome
}

/// License 规则（§20-§22/§44）：policy 违规（fact×policy 可解释）+
/// UNKNOWN license 计量（policy.require_known 才升 finding）。
pub fn license_rule(ctx: &RuleContext) -> RuleOutcome {
    let mut outcome = RuleOutcome::default();
    let Some(supply) = ctx.supply else {
        outcome
            .diagnostics
            .push("no supply facts injected; rule skipped".to_string());
        return outcome;
    };
    for violation in &supply.license_policy_violations {
        if ctx.cancel.is_cancelled() {
            outcome.diagnostics.push("cancelled mid-rule".to_string());
            break;
        }
        let evidence = Evidence::new(
            EvidenceKind::Manifest,
            None,
            None,
            Some(violation.subject.clone()),
            violation.reasoning.clone(),
        );
        let finding = Finding::new(
            "LIC/POLICY_VIOLATION",
            prism_core::Severity::Warning,
            FindingStatus::Open,
            format!("License policy violation {}", violation.subject),
            violation.reasoning.clone(),
            Origin::Fact,
            Confidence::High,
            None,
            vec![evidence.id.clone()],
        );
        outcome.evidence.push(evidence);
        outcome.findings.push(finding);
    }
    let unknown_licenses = supply
        .licenses
        .iter()
        .filter(|fact| !fact.spdx_known || fact.expression.is_empty())
        .count();
    let require_known = supply
        .license_policy
        .as_ref()
        .map(|policy| policy.require_known)
        .unwrap_or(false);
    if require_known
        && unknown_licenses > 0
        && outcome.findings.len() < ctx.thresholds.max_supply_findings
    {
        let finding = Finding::new(
            "LIC/UNKNOWN_LICENSE",
            prism_core::Severity::Info,
            FindingStatus::Open,
            format!("{unknown_licenses} license fact(s) UNKNOWN"),
            "license metadata missing or not an SPDX-known expression (UNKNOWN ≠ no license)"
                .to_string(),
            Origin::Fact,
            Confidence::High,
            None,
            vec![],
        );
        outcome.findings.push(finding);
    }
    outcome.metrics.push(metric(
        "license.unknown.count",
        unknown_licenses as u64,
        "license facts that are not SPDX-known (policy may require known)",
    ));
    outcome.metrics.push(metric(
        "license.policy_violation.count",
        supply.license_policy_violations.len() as u64,
        "license facts violating the applied license policy",
    ));
    outcome
}

/// Environment 规则（§41-§42/§46）：声明 × 观测对照 ⇒ 四态 finding
/// （Mismatch/Missing = Warning；Unknown = Info；Match 不产出）。
pub fn toolchain_rule(ctx: &RuleContext) -> RuleOutcome {
    let mut outcome = RuleOutcome::default();
    let Some(supply) = ctx.supply else {
        outcome
            .diagnostics
            .push("no supply facts injected; rule skipped".to_string());
        return outcome;
    };
    let mut mismatch = 0u64;
    let mut missing = 0u64;
    for comparison in &supply.toolchain_comparisons {
        if ctx.cancel.is_cancelled() {
            outcome.diagnostics.push("cancelled mid-rule".to_string());
            break;
        }
        let (finding_type, severity, state_label) = match comparison.state {
            prism_supply::ToolchainState::Mismatch => (
                "ENV/TOOLCHAIN_MISMATCH",
                prism_core::Severity::Warning,
                "mismatch",
            ),
            prism_supply::ToolchainState::Missing => {
                ("ENV/TOOL_MISSING", prism_core::Severity::Warning, "missing")
            }
            prism_supply::ToolchainState::Unknown => (
                "ENV/VERSION_UNKNOWN",
                prism_core::Severity::Info,
                "version-unknown",
            ),
            _ => continue,
        };
        match comparison.state {
            prism_supply::ToolchainState::Mismatch => mismatch += 1,
            prism_supply::ToolchainState::Missing => missing += 1,
            _ => {}
        }
        if outcome.findings.len() >= ctx.thresholds.max_supply_findings {
            outcome
                .diagnostics
                .push("environment findings exceed cap; truncated".to_string());
            break;
        }
        let evidence = Evidence::new(
            EvidenceKind::Manifest,
            None,
            None,
            Some(comparison.tool.clone()),
            comparison.detail.clone(),
        );
        // 标题含约束原样：多条声明（global.json vs csproj）不折叠成同指纹。
        let finding = Finding::new(
            finding_type,
            severity,
            FindingStatus::Open,
            format!(
                "Toolchain {} {} ({})",
                comparison.tool,
                state_label,
                comparison.declared.clone().unwrap_or_default()
            ),
            comparison.detail.clone(),
            Origin::Fact,
            Confidence::High,
            None,
            vec![evidence.id.clone()],
        );
        outcome.evidence.push(evidence);
        outcome.findings.push(finding);
    }
    outcome.metrics.push(metric(
        "environment.mismatch.count",
        mismatch,
        "declared toolchain vs observed version mismatches",
    ));
    outcome.metrics.push(metric(
        "environment.missing_tool.count",
        missing,
        "declared tools not observed on this machine",
    ));
    outcome
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::RuleContext;
    use crate::RuleThresholds;
    use prism_core::task::CancellationToken;
    use prism_supply::SupplyReport;

    fn context<'a>(
        supply: Option<&'a SupplyReport>,
        thresholds: &'a RuleThresholds,
        cancel: &'a CancellationToken,
    ) -> RuleContext<'a> {
        RuleContext {
            files: &[],
            thresholds,
            observed_at: "t",
            csharp: None,
            godot: None,
            supply,
            debug: None,
            cancel,
        }
    }

    #[test]
    fn no_supply_facts_is_diagnostic_not_finding() {
        let cancel = CancellationToken::new();
        let thresholds = RuleThresholds::default();
        let ctx = context(None, &thresholds, &cancel);
        for rule in [
            dependency_resolution_rule,
            dependency_cycle_rule,
            secret_candidate_rule,
            known_vulnerability_rule,
            license_rule,
            toolchain_rule,
        ] {
            let outcome = rule(&ctx);
            assert!(outcome.findings.is_empty());
            assert!(!outcome.diagnostics.is_empty(), "§48 prerequisite 诊断");
        }
    }

    #[test]
    fn vulnerability_unavailable_is_not_zero() {
        let cancel = CancellationToken::new();
        let thresholds = RuleThresholds::default();
        let supply = SupplyReport {
            schema_version: 1,
            advisory_capability: Some(prism_supply::ProviderCapability::Unavailable),
            ..SupplyReport::default()
        };
        let ctx = context(Some(&supply), &thresholds, &cancel);
        let outcome = known_vulnerability_rule(&ctx);
        assert!(outcome.findings.is_empty());
        assert!(outcome
            .diagnostics
            .iter()
            .any(|d| d.contains("unavailable") && d.contains("not zero")));
    }

    #[test]
    fn secret_candidate_finding_contains_no_secret_value() {
        let cancel = CancellationToken::new();
        let thresholds = RuleThresholds::default();
        let supply = SupplyReport {
            schema_version: 1,
            secret_candidates: vec![prism_supply::SecretCandidate {
                id: "find:sec:abc".to_string(),
                rule: "aws_access_key_id".to_string(),
                path: "src/a.cs".to_string(),
                line: 3,
                detection_class: "cloud_credential".to_string(),
                detection_reason: "known credential prefix `AKIA` followed by 16 chars".to_string(),
                exclusion_notes: vec![],
                confidence: Confidence::Medium,
                redacted_context: "AKIA…(20 chars)".to_string(),
            }],
            ..SupplyReport::default()
        };
        let ctx = context(Some(&supply), &thresholds, &cancel);
        let outcome = secret_candidate_rule(&ctx);
        assert_eq!(outcome.findings.len(), 1);
        assert_eq!(outcome.findings[0].finding_type, "SEC/SECRET_CANDIDATE");
        let rendered = format!("{:?}", outcome.findings[0]);
        assert!(!rendered.contains("AKIAABCDEFGHIJKLMNOP"));
        assert_eq!(
            outcome
                .metrics
                .iter()
                .find(|m| m.key == "security.secret_candidate.count")
                .unwrap()
                .value,
            1
        );
    }

    #[test]
    fn known_vulnerability_severity_and_remediation() {
        let cancel = CancellationToken::new();
        let thresholds = RuleThresholds::default();
        let supply = SupplyReport {
            schema_version: 1,
            advisory_capability: Some(prism_supply::ProviderCapability::Supported),
            advisory_database_version: Some("db-1".to_string()),
            advisory_matches: vec![prism_supply::AdvisoryMatch {
                advisory: prism_supply::Advisory {
                    id: "ADV-1".to_string(),
                    source: "test".to_string(),
                    ecosystem: "nuget".to_string(),
                    package_id: "P".to_string(),
                    affected_ranges: vec![">=1.0,<1.2.5".to_string()],
                    fixed_version: Some("1.2.5".to_string()),
                    severity_hint: "critical".to_string(),
                },
                resolved_version: "1.2.3".to_string(),
                conclusion: "Potentially affected".to_string(),
            }],
            ..SupplyReport::default()
        };
        let ctx = context(Some(&supply), &thresholds, &cancel);
        let outcome = known_vulnerability_rule(&ctx);
        assert_eq!(outcome.findings.len(), 1);
        assert_eq!(outcome.findings[0].severity, prism_core::Severity::Critical);
        assert_eq!(
            outcome.findings[0].remediation.as_deref(),
            Some("Update to >= 1.2.5"),
            "§80：advisory 有 safe version 才推荐具体版本"
        );
    }

    #[test]
    fn drift_states_do_not_become_findings() {
        let cancel = CancellationToken::new();
        let thresholds = RuleThresholds::default();
        let supply = SupplyReport {
            schema_version: 1,
            version_comparisons: vec![
                prism_supply::VersionComparison {
                    declaration_id: "nuget:A".into(),
                    package_id: "A".into(),
                    state: prism_supply::DriftState::NormalResolution,
                    detail: "8.0.* vs 8.0.11".into(),
                },
                prism_supply::VersionComparison {
                    declaration_id: "nuget:B".into(),
                    package_id: "B".into(),
                    state: prism_supply::DriftState::Unresolved,
                    detail: "declared but absent".into(),
                },
            ],
            ..SupplyReport::default()
        };
        let ctx = context(Some(&supply), &thresholds, &cancel);
        let outcome = dependency_resolution_rule(&ctx);
        assert_eq!(
            outcome.findings.len(),
            1,
            "NormalResolution 不产出 finding（§11）"
        );
        assert_eq!(outcome.findings[0].finding_type, "DEP/UNRESOLVED");
    }
}
