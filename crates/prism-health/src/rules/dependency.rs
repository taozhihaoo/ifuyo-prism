//! 依赖规则（规格 M7 §31–§35/§209/§210）。
//!
//! 直接消费 M3 依赖事实（§34 不重写 resolver）：
//! - DEP001 Unresolved Dependency
//! - DEP002 Ambiguous Dependency
// DEP003 与 ARCH001 同源、DEP005 需要生态依赖模型：不在第一版目录，见审计。

use crate::model::{
    Evidence, EvidenceKind, Finding, FindingCategory, FindingExplanation, FindingLocation,
    FindingStatus, RuleResult,
};
use crate::registry::{HealthRule, HealthRuleMeta};
use prism_core::severity::{Confidence, Severity};
use prism_core::task::CancellationToken;
use std::sync::LazyLock;

#[allow(clippy::too_many_arguments)] // finding 字段直传；参数即 finding 形状
fn dependency_finding(
    rule_id: &str,
    title: &str,
    what: &str,
    why: &str,
    source_path: &str,
    raw_target: &str,
    line: Option<u32>,
    column: Option<u32>,
    dependency_id: &str,
    severity: Severity,
    confidence: Confidence,
) -> Finding {
    let location = FindingLocation::new(
        source_path.to_string(),
        line.unwrap_or(1),
        column.unwrap_or(1),
    );
    Finding {
        id: Finding::derive_id(rule_id, dependency_id),
        rule_id: rule_id.to_string(),
        category: FindingCategory::Dependency,
        severity,
        confidence,
        title: title.to_string(),
        message: format!(
            "Could not uniquely resolve \"{raw_target}\" referenced by {source_path}."
        ),
        explanation: FindingExplanation {
            what: what.to_string(),
            evidence_summary: format!(
                "resolution status = {}; requested target = \"{raw_target}\"",
                if rule_id == "DEP002" {
                    "Ambiguous"
                } else {
                    "Unresolved"
                }
            ),
            why_it_matters: Some(why.to_string()),
        },
        location,
        related_locations: vec![],
        related_symbols: vec![],
        related_modules: vec![],
        related_files: vec![source_path.to_string()],
        evidence: vec![Evidence::new(
            EvidenceKind::Relationship,
            dependency_id.to_string(),
            format!("requested target = \"{raw_target}\""),
        )
        .with_location(FindingLocation::new(
            source_path.to_string(),
            line.unwrap_or(1),
            column.unwrap_or(1),
        ))],
        suggested_action: Some(
            "Verify the dependency specifier and the availability of the target.".to_string(),
        ),
        status: FindingStatus::Active,
        change_relation: None,
        metadata: [("raw_target".to_string(), raw_target.to_string())]
            .into_iter()
            .collect(),
    }
}

fn evaluate_dependencies(
    context: &crate::engine::HealthContext,
    cancel: &CancellationToken,
    rule_id: &'static str,
    ambiguous: bool,
) -> prism_core::PrismResult<RuleResult> {
    let mut result = RuleResult::default();
    let Some(graph) = context.graph else {
        return Ok(result);
    };
    for dependency in &graph.dependencies {
        if cancel.is_cancelled() {
            return Err(prism_core::PrismError::Cancelled);
        }
        let matches = if ambiguous {
            dependency.resolution == prism_graph::ResolutionStatus::Ambiguous
        } else {
            dependency.resolution == prism_graph::ResolutionStatus::Unresolved
        };
        if !matches {
            continue;
        }
        result.findings.push(dependency_finding(
            rule_id,
            if ambiguous {
                "Ambiguous Dependency"
            } else {
                "Unresolved Dependency"
            },
            if ambiguous {
                "A dependency could not be uniquely resolved; multiple candidates were found."
            } else {
                "A referenced dependency could not be resolved to a local target."
            },
            "Unresolved dependencies can prevent reliable static analysis and may indicate a missing or misnamed target.",
            &dependency.source_path,
            &dependency.raw_target,
            dependency.location.map(|l| l.line),
            dependency.location.map(|l| l.column),
            &dependency.id,
            if ambiguous { Severity::Info } else { Severity::Warning },
            if ambiguous { Confidence::Medium } else { Confidence::High },
        ));
    }
    Ok(result)
}

static DEP001: LazyLock<HealthRuleMeta> = LazyLock::new(|| HealthRuleMeta {
    id: "DEP001",
    name: "Unresolved Dependency",
    category: FindingCategory::Dependency,
    description:
        "A dependency reference could not be resolved to a local target (M3 resolution fact).",
    default_severity: Severity::Warning,
    default_enabled: true,
    threshold_meaning: None,
    needs_derived: false,
});

static DEP002: LazyLock<HealthRuleMeta> = LazyLock::new(|| HealthRuleMeta {
    id: "DEP002",
    name: "Ambiguous Dependency",
    category: FindingCategory::Dependency,
    description:
        "A dependency reference matched multiple candidates and could not be uniquely resolved.",
    default_severity: Severity::Info,
    default_enabled: true,
    threshold_meaning: None,
    needs_derived: false,
});

pub struct UnresolvedDependencyRule;

impl HealthRule for UnresolvedDependencyRule {
    fn meta(&self) -> &'static HealthRuleMeta {
        &DEP001
    }

    fn evaluate(
        &self,
        context: &crate::engine::HealthContext,
        _config: &prism_core::config::HealthRuleConfig,
        cancel: &CancellationToken,
    ) -> prism_core::PrismResult<RuleResult> {
        evaluate_dependencies(context, cancel, "DEP001", false)
    }
}

pub struct AmbiguousDependencyRule;

impl HealthRule for AmbiguousDependencyRule {
    fn meta(&self) -> &'static HealthRuleMeta {
        &DEP002
    }

    fn evaluate(
        &self,
        context: &crate::engine::HealthContext,
        _config: &prism_core::config::HealthRuleConfig,
        cancel: &CancellationToken,
    ) -> prism_core::PrismResult<RuleResult> {
        evaluate_dependencies(context, cancel, "DEP002", true)
    }
}
