//! M211 Evolution 规则（§63：向 M28 提供 Historical Change Evidence）。
//!
//! 与 M28/M29/M210 同一挂载点；只消费注入的 `EvolutionReport`。
//! **不产 Health Score**（§63）；热点/耦合以原始指标表达，Co-change
//! 明确标注为历史关系而非语义依赖（§26/§27）。

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
        producer: "prism-health2::rules_evolution".to_string(),
    }
}

/// Evolution 规则：热点（窗口内 commit 数 ≥ 阈值）以 Info 级原始指标
/// 表达（§68：High-change 表述，不宣布"最差模块"）。
pub fn hotspot_rule(ctx: &RuleContext) -> RuleOutcome {
    let mut outcome = RuleOutcome::default();
    let Some(report) = ctx.evolution else {
        outcome
            .diagnostics
            .push("no evolution report injected; rule skipped".to_string());
        return outcome;
    };
    if ctx.cancel.is_cancelled() {
        outcome.diagnostics.push("cancelled mid-rule".to_string());
        return outcome;
    }
    for hotspot in report
        .hotspots
        .iter()
        .take(ctx.thresholds.max_supply_findings)
    {
        if hotspot.commit_count < ctx.thresholds.evolution_hotspot_min_commits {
            continue;
        }
        let evidence = Evidence::new(
            EvidenceKind::Git,
            Some(hotspot.path.clone()),
            None,
            Some(hotspot.last_committed_at.clone().unwrap_or_default()),
            format!(
                "window `{}`: {} commits, churn +{}/-{}, {} authors",
                report
                    .window
                    .as_ref()
                    .map(|window| window.as_label())
                    .unwrap_or_else(|| "unbounded".to_string()),
                hotspot.commit_count,
                hotspot.lines_added,
                hotspot.lines_deleted,
                hotspot.author_count
            ),
        );
        let mut finding = Finding::new(
            "EVOLUTION/HOTSPOT",
            prism_core::Severity::Info,
            FindingStatus::Open,
            format!("High-change file {} ({} commits)", hotspot.path, hotspot.commit_count),
            "raw change-frequency metrics; this is a change-frequency observation, not a quality verdict".to_string(),
            Origin::Inference,
            Confidence::High,
            None,
            vec![evidence.id.clone()],
        );
        finding
            .metadata
            .insert("commit_count".to_string(), hotspot.commit_count.to_string());
        finding
            .metadata
            .insert("churn".to_string(), hotspot.churn.to_string());
        finding
            .metadata
            .insert("author_count".to_string(), hotspot.author_count.to_string());
        finding.metadata.insert(
            "window".to_string(),
            report
                .window
                .as_ref()
                .map(|window| window.as_label())
                .unwrap_or_default(),
        );
        outcome.evidence.push(evidence);
        outcome.findings.push(finding);
    }
    outcome.metrics.push(metric(
        "evolution.hotspot.count",
        report
            .hotspots
            .iter()
            .filter(|hotspot| hotspot.commit_count >= ctx.thresholds.evolution_hotspot_min_commits)
            .count() as u64,
        "files exceeding the hotspot commit threshold in the analysis window",
    ));
    outcome
}

/// Evolution 规则：变更耦合（co-change ≥ 阈值 ⇒ Info；历史关系，
/// 非语义依赖——§26/§27 边界写进 finding 描述）。
pub fn coupling_rule(ctx: &RuleContext) -> RuleOutcome {
    let mut outcome = RuleOutcome::default();
    let Some(report) = ctx.evolution else {
        outcome
            .diagnostics
            .push("no evolution report injected; rule skipped".to_string());
        return outcome;
    };
    for coupling in &report.couplings {
        if ctx.cancel.is_cancelled() {
            outcome.diagnostics.push("cancelled mid-rule".to_string());
            break;
        }
        if coupling.co_change_count < ctx.thresholds.evolution_min_co_change {
            continue;
        }
        if outcome.findings.len() >= ctx.thresholds.max_supply_findings {
            outcome
                .diagnostics
                .push("coupling findings exceed cap; truncated".to_string());
            break;
        }
        let evidence = Evidence::new(
            EvidenceKind::Git,
            None,
            None,
            Some(coupling.evidence_commits.join(", ")),
            format!(
                "{} common commits in window `{}`",
                coupling.co_change_count,
                report
                    .window
                    .as_ref()
                    .map(|window| window.as_label())
                    .unwrap_or_else(|| "unbounded".to_string())
            ),
        );
        let mut finding = Finding::new(
            "EVOLUTION/CHANGE_COUPLING",
            prism_core::Severity::Info,
            FindingStatus::Open,
            format!(
                "Co-changed files {} <-> {} (×{})",
                coupling.path_a, coupling.path_b, coupling.co_change_count
            ),
            "historical co-change relation — NOT a semantic dependency (references/calls are separate evidence)".to_string(),
            Origin::Inference,
            Confidence::Medium,
            None,
            vec![evidence.id.clone()],
        );
        finding.metadata.insert(
            "co_change_count".to_string(),
            coupling.co_change_count.to_string(),
        );
        outcome.evidence.push(evidence);
        outcome.findings.push(finding);
    }
    outcome.metrics.push(metric(
        "evolution.coupling.count",
        report.couplings.len() as u64,
        "co-change pairs meeting the coupling threshold in the analysis window",
    ));
    outcome
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::RuleContext;
    use crate::RuleThresholds;
    use prism_core::task::CancellationToken;
    use prism_evolution::{ChangeCoupling, EvolutionReport, HistoryWindow, Hotspot};

    fn context<'a>(
        report: Option<&'a EvolutionReport>,
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
            evolution: report,
            cancel,
        }
    }

    fn report(min_commits: u64) -> EvolutionReport {
        EvolutionReport {
            schema_version: 1,
            window: Some(HistoryWindow::RecentCommits { count: 200 }),
            hotspots: vec![Hotspot {
                path: "src/hot.rs".to_string(),
                commit_count: min_commits,
                change_count: min_commits,
                churn: 500,
                lines_added: 300,
                lines_deleted: 200,
                author_count: 2,
                last_committed_at: Some("2026-10-06T00:00:00+00:00".to_string()),
            }],
            couplings: vec![ChangeCoupling {
                path_a: "src/a.rs".to_string(),
                path_b: "src/b.rs".to_string(),
                co_change_count: 6,
                evidence_commits: vec!["c1".to_string(), "c2".to_string()],
            }],
            ..EvolutionReport::default()
        }
    }

    #[test]
    fn no_evolution_report_is_diagnostic() {
        let cancel = CancellationToken::new();
        let thresholds = RuleThresholds::default();
        let ctx = context(None, &thresholds, &cancel);
        for rule in [hotspot_rule, coupling_rule] {
            let outcome = rule(&ctx);
            assert!(outcome.findings.is_empty());
            assert!(!outcome.diagnostics.is_empty());
        }
    }

    #[test]
    fn hotspot_and_coupling_raw_metrics_findings() {
        let cancel = CancellationToken::new();
        let thresholds = RuleThresholds::default();
        let report = report(15);
        let ctx = context(Some(&report), &thresholds, &cancel);
        let hotspots = hotspot_rule(&ctx);
        assert_eq!(hotspots.findings.len(), 1);
        assert_eq!(hotspots.findings[0].finding_type, "EVOLUTION/HOTSPOT");
        assert_eq!(hotspots.findings[0].severity, prism_core::Severity::Info);
        assert_eq!(
            hotspots.findings[0]
                .metadata
                .get("commit_count")
                .map(String::as_str),
            Some("15")
        );
        assert_eq!(
            hotspots.findings[0].origin,
            prism_intel::Origin::Inference,
            "频率观察是 INFERENCE 级工程事实（§5）"
        );
        let couplings = coupling_rule(&ctx);
        assert_eq!(couplings.findings.len(), 1);
        assert!(
            couplings.findings[0]
                .description
                .contains("NOT a semantic dependency"),
            "§27 边界写进 finding"
        );
    }

    #[test]
    fn below_threshold_stays_metric_only() {
        let cancel = CancellationToken::new();
        let thresholds = RuleThresholds::default();
        let report = report(5);
        let ctx = context(Some(&report), &thresholds, &cancel);
        assert!(hotspot_rule(&ctx).findings.is_empty(), "5 < 10 阈值");
        let quiet = EvolutionReport {
            couplings: Vec::new(),
            ..report
        };
        let ctx = context(Some(&quiet), &thresholds, &cancel);
        assert!(coupling_rule(&ctx).findings.is_empty());
    }
}
