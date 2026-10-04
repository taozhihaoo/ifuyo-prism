//! ARCH004 — Forbidden Dependency（规格 M7 §28/§113/§133/§246）。
//!
//! 仅当用户在 `health.architecture.forbidden_dependencies` 显式配置
//! `[{from, to}]` 时运行（§113：未配置=不运行，不制造默认架构真理）。
//! 消费 M3 resolved 依赖事实（§176 不重实现）；from/to 为项目相对路径
//! 片段匹配（子串，不猜测目录=层语义，§134）。
//!
//! Evidence（§28/§246）：observed relationship（source → target）+
//! configured restriction（from must not depend on to）。

use crate::model::{
    Evidence, EvidenceKind, Finding, FindingCategory, FindingExplanation, FindingLocation,
    FindingStatus, RuleResult,
};
use crate::registry::{HealthRule, HealthRuleMeta};
use prism_core::severity::{Confidence, Severity};
use prism_core::task::CancellationToken;
use std::sync::LazyLock;

pub struct ForbiddenDependencyRule;

static META: LazyLock<HealthRuleMeta> = LazyLock::new(|| HealthRuleMeta {
    id: "ARCH004",
    name: "Forbidden Dependency",
    category: FindingCategory::Architecture,
    description: "A resolved dependency matches a user-configured forbidden from/to pair (§28).",
    default_severity: Severity::Warning,
    default_enabled: true,
    threshold_meaning: None,
    needs_derived: false,
});

/// `needle` 是否匹配路径（路径片段子串，`/` 分隔规范化；§134 保守子串语义）。
fn path_matches(path: &str, needle: &str) -> bool {
    let normalized = path.replace('\\', "/");
    let needle = needle.replace('\\', "/");
    let needle = needle.trim_matches('/');
    if needle.is_empty() {
        return false;
    }
    normalized
        .to_ascii_lowercase()
        .contains(&needle.to_ascii_lowercase())
}

impl HealthRule for ForbiddenDependencyRule {
    fn meta(&self) -> &'static HealthRuleMeta {
        &META
    }

    fn evaluate(
        &self,
        context: &crate::engine::HealthContext,
        config: &prism_core::config::HealthRuleConfig,
        cancel: &CancellationToken,
    ) -> prism_core::PrismResult<RuleResult> {
        let _ = config;
        let mut result = RuleResult::default();
        let Some(graph) = context.graph else {
            return Ok(result);
        };
        let forbidden: Vec<(String, String)> = context
            .health_config
            .architecture
            .forbidden_dependencies
            .iter()
            .map(|pair| (pair.from.clone(), pair.to.clone()))
            .collect();
        if forbidden.is_empty() {
            // §113：未配置 ⇒ 安静跳过（无诊断，§208）。
            return Ok(result);
        }

        for dependency in &graph.dependencies {
            if cancel.is_cancelled() {
                return Err(prism_core::PrismError::Cancelled);
            }
            if dependency.resolution != prism_graph::ResolutionStatus::Resolved {
                continue;
            }
            let Some(target_path) = &dependency.target_path else {
                continue;
            };
            for (from, to) in &forbidden {
                if !path_matches(&dependency.source_path, from) || !path_matches(target_path, to) {
                    continue;
                }
                let stable_key =
                    format!("{}#{}#{}#{}", dependency.source_path, target_path, from, to);
                result.findings.push(Finding {
                    id: Finding::derive_id("ARCH004", &stable_key),
                    rule_id: "ARCH004".to_string(),
                    category: FindingCategory::Architecture,
                    severity: Severity::Warning,
                    confidence: Confidence::High,
                    title: "Violates configured forbidden dependency".to_string(),
                    message: format!(
                        "Observed dependency {} -> {} violates the configured restriction: \"{}\" must not depend on \"{}\".",
                        dependency.source_path, target_path, from, to
                    ),
                    explanation: FindingExplanation {
                        what: format!(
                            "The observed dependency {} -> {} matches a configured forbidden pair ({from} -> {to}).",
                            dependency.source_path, target_path
                        ),
                        evidence_summary: format!(
                            "observed: {} -> {}; configured restriction: {from} must not depend on {to}",
                            dependency.source_path, target_path
                        ),
                        why_it_matters: Some(
                            "This finding comes from the project's own configured rule, not a built-in architecture opinion.".to_string(),
                        ),
                    },
                    location: FindingLocation::new(dependency.source_path.clone(), 1, 1),
                    related_locations: vec![],
                    related_symbols: vec![],
                    related_modules: vec![
                        dependency.source_id.as_str().to_string(),
                        target_id_str(dependency),
                    ],
                    related_files: vec![
                        dependency.source_path.clone(),
                        target_path.clone(),
                    ],
                    evidence: vec![
                        Evidence::new(
                            EvidenceKind::Relationship,
                            dependency.id.clone(),
                            format!(
                                "observed relationship: {} -> {}",
                                dependency.source_path, target_path
                            ),
                        ),
                        Evidence::new(
                            EvidenceKind::ConfigFact,
                            format!("{from} -> {to}"),
                            "configured restriction: forbidden_dependencies".to_string(),
                        ),
                    ],
                    suggested_action: Some(
                        "Review the dependency against the project's configured architecture rule.".to_string(),
                    ),
                    status: FindingStatus::Active,
                    change_relation: None,
                    metadata: [
                        ("from".to_string(), from.clone()),
                        ("to".to_string(), to.clone()),
                    ]
                    .into_iter()
                    .collect(),
                });
            }
        }
        Ok(result)
    }
}

fn target_id_str(dependency: &prism_graph::Dependency) -> String {
    dependency
        .target_id
        .as_ref()
        .map(|id| id.as_str().to_string())
        .unwrap_or_default()
}
