//! 架构规则（规格 M7 §22–§30/§132/§133/§149–§151）。
//!
//! ARCH001 复用 M3 预计算的 SCC（§132 不重造）；ARCH002/003 消费共享
//! fan 统计（§94），阈值为**产品默认值并可通过配置覆盖**（§26/§216/§217：
//! 不冒充通用工程法则）。方向语义来自 M3：`Dependency.source → target`
//! = source 依赖 target（§135，有 fixture 验证）。

use crate::model::{
    Evidence, EvidenceKind, Finding, FindingCategory, FindingExplanation, FindingLocation,
    FindingStatus, RuleResult,
};
use crate::registry::{HealthRule, HealthRuleMeta};
use prism_core::severity::{Confidence, Severity};
use prism_core::task::CancellationToken;
use prism_graph::ModuleKind;
use std::collections::BTreeMap;
use std::sync::LazyLock;

// ---------- 共享工具 ----------

/// ModuleId 字符串 → 展示路径（模块树里找 path；找不到退回 id）。
fn module_paths(graph: &prism_graph::DependencyGraphResult) -> BTreeMap<String, String> {
    graph
        .modules
        .iter()
        .map(|module| {
            (
                module.id.as_str().to_string(),
                module.path.clone().unwrap_or_else(|| module.id.as_str().to_string()),
            )
        })
        .collect()
}

fn module_name(module_id: &str) -> String {
    // module:file:src/a.ts ⇒ src/a.ts；module:namespace:App ⇒ App
    module_id
        .splitn(3, ':')
        .nth(2)
        .unwrap_or(module_id)
        .to_string()
}

fn fan_finding(
    rule_id: &str,
    direction: &str,
    module_id: &str,
    count: u64,
    threshold: u64,
    paths: &BTreeMap<String, String>,
    threshold_meaning: &str,
) -> Finding {
    let path = paths.get(module_id).cloned().unwrap_or_else(|| module_id.to_string());
    Finding {
        id: Finding::derive_id(rule_id, module_id),
        rule_id: rule_id.to_string(),
        category: FindingCategory::Architecture,
        severity: Severity::Info,
        confidence: Confidence::High,
        title: format!("Detected {direction} exceeding configured threshold"),
        message: format!(
            "{direction} = {count} for module {module_id} (threshold {threshold})."
        ),
        explanation: FindingExplanation {
            what: format!(
                "Module {module_id} has {direction} of {count} resolved dependencies, exceeding the configured threshold of {threshold}."
            ),
            evidence_summary: format!("{direction} = {count}; threshold = {threshold}"),
            why_it_matters: Some(
                "High connectivity can make change impact harder to reason about; this is a factual signal, not a design verdict.".to_string(),
            ),
        },
        location: FindingLocation::new(path.clone(), 1, 1),
        related_locations: vec![],
        related_symbols: vec![],
        related_modules: vec![module_id.to_string()],
        related_files: vec![path],
        evidence: vec![Evidence::new(
            EvidenceKind::Relationship,
            module_id.to_string(),
            format!("{threshold_meaning}: count = {count}, threshold = {threshold}"),
        )],
        suggested_action: Some("Review the module's responsibilities if the connectivity is unexpected.".to_string()),
        status: FindingStatus::Active,
        metadata: [
            ("count".to_string(), count.to_string()),
            ("threshold".to_string(), threshold.to_string()),
        ]
        .into_iter()
        .collect(),
    }
}

// ---------- ARCH001 Circular Module Dependency ----------

static ARCH001: LazyLock<HealthRuleMeta> = LazyLock::new(|| HealthRuleMeta {
    id: "ARCH001",
    name: "Circular Module Dependency",
    category: FindingCategory::Architecture,
    description: "Detected static dependency cycles between modules (SCC from the M3 graph).",
    default_severity: Severity::Warning,
    default_enabled: true,
    threshold_meaning: None,
    needs_derived: false,
});

pub struct CircularDependencyRule;

impl HealthRule for CircularDependencyRule {
    fn meta(&self) -> &'static HealthRuleMeta {
        &ARCH001
    }

    fn evaluate(
        &self,
        context: &crate::engine::HealthContext,
        _config: &prism_core::config::HealthRuleConfig,
        cancel: &CancellationToken,
    ) -> prism_core::PrismResult<RuleResult> {
        let mut result = RuleResult::default();
        let Some(graph) = context.graph else {
            return Ok(result); // 不适用即安静跳过（§207/§208）
        };
        let paths = module_paths(graph);
        for cycle in &graph.cycles {
            if cancel.is_cancelled() {
                return Err(prism_core::PrismError::Cancelled);
            }
            if cycle.nodes.len() < 2 {
                continue;
            }
            let members: Vec<String> = cycle
                .nodes
                .iter()
                .map(|id| module_name(id))
                .collect();
            let path = paths
                .get(&cycle.nodes[0])
                .cloned()
                .unwrap_or_else(|| "project".to_string());
            let evidence: Vec<Evidence> = cycle
                .nodes
                .iter()
                .map(|node| {
                    Evidence::new(
                        EvidenceKind::Relationship,
                        node.clone(),
                        format!("{} participates in the detected dependency cycle", module_name(node)),
                    )
                })
                .collect();
            result.findings.push(Finding {
                id: Finding::derive_id("ARCH001", &cycle.id),
                rule_id: "ARCH001".to_string(),
                category: FindingCategory::Architecture,
                severity: Severity::Warning,
                confidence: Confidence::High,
                title: "Detected circular module dependency".to_string(),
                message: format!(
                    "Detected static dependency cycle involving {} module(s): {}",
                    members.len(),
                    members.join(" -> ")
                ),
                explanation: FindingExplanation {
                    what: format!(
                        "A static dependency cycle was detected between {} modules.",
                        members.len()
                    ),
                    evidence_summary: format!("cycle members: {}", members.join(" -> ")),
                    why_it_matters: Some(
                        "Dependency cycles can make build order and change impact harder to reason about."
                            .to_string(),
                    ),
                },
                location: FindingLocation::new(path, 1, 1),
                related_locations: vec![],
                related_symbols: vec![],
                related_modules: cycle.nodes.clone(),
                related_files: vec![],
                evidence,
                suggested_action: Some("Review the dependency direction between these modules.".to_string()),
                status: FindingStatus::Active,
                metadata: [
                    ("cycle_id".to_string(), cycle.id.clone()),
                    ("member_count".to_string(), cycle.nodes.len().to_string()),
                ]
                .into_iter()
                .collect(),
            });
        }
        Ok(result)
    }
}

// ---------- ARCH002 / ARCH003：fan-out / fan-in ----------

static ARCH002: LazyLock<HealthRuleMeta> = LazyLock::new(|| HealthRuleMeta {
    id: "ARCH002",
    name: "Excessive Module Fan-out",
    category: FindingCategory::Architecture,
    description: "Module fan-out (resolved outgoing dependencies) exceeds the configured threshold.",
    default_severity: Severity::Info,
    default_enabled: true,
    threshold_meaning: Some("module fan-out (resolved outgoing dependencies) threshold"),
    needs_derived: true,
});

static ARCH003: LazyLock<HealthRuleMeta> = LazyLock::new(|| HealthRuleMeta {
    id: "ARCH003",
    name: "Excessive Module Fan-in",
    category: FindingCategory::Architecture,
    description: "Module fan-in (resolved incoming dependencies) exceeds the configured threshold.",
    default_severity: Severity::Info,
    default_enabled: true,
    threshold_meaning: Some("module fan-in (resolved incoming dependencies) threshold"),
    needs_derived: true,
});

/// 产品默认阈值（§216：记录来源为 product default，非工程法则）。
pub const DEFAULT_FAN_OUT_THRESHOLD: u64 = 15;
pub const DEFAULT_FAN_IN_THRESHOLD: u64 = 20;

pub struct FanOutRule;

impl HealthRule for FanOutRule {
    fn meta(&self) -> &'static HealthRuleMeta {
        &ARCH002
    }

    fn evaluate(
        &self,
        context: &crate::engine::HealthContext,
        config: &prism_core::config::HealthRuleConfig,
        cancel: &CancellationToken,
    ) -> prism_core::PrismResult<RuleResult> {
        evaluate_fan(
            context,
            config.threshold,
            DEFAULT_FAN_OUT_THRESHOLD,
            "fan-out",
            "ARCH002",
            cancel,
        )
    }
}

pub struct FanInRule;

impl HealthRule for FanInRule {
    fn meta(&self) -> &'static HealthRuleMeta {
        &ARCH003
    }

    fn evaluate(
        &self,
        context: &crate::engine::HealthContext,
        config: &prism_core::config::HealthRuleConfig,
        cancel: &CancellationToken,
    ) -> prism_core::PrismResult<RuleResult> {
        evaluate_fan(
            context,
            config.threshold,
            DEFAULT_FAN_IN_THRESHOLD,
            "fan-in",
            "ARCH003",
            cancel,
        )
    }
}

#[allow(clippy::too_many_arguments)]
fn evaluate_fan(
    context: &crate::engine::HealthContext,
    configured: Option<u64>,
    default_threshold: u64,
    direction: &str,
    rule_id: &str,
    cancel: &CancellationToken,
) -> prism_core::PrismResult<RuleResult> {
    let mut result = RuleResult::default();
    let Some(graph) = context.graph else {
        return Ok(result);
    };
    let Some(derived) = context.derived else {
        return Ok(result);
    };
    let paths = module_paths(graph);
    let fan: &BTreeMap<String, u64> = if direction == "fan-out" {
        &derived.fan_out
    } else {
        &derived.fan_in
    };
    let threshold = configured.unwrap_or(default_threshold);
    for (module_id, count) in fan {
        if cancel.is_cancelled() {
            return Err(prism_core::PrismError::Cancelled);
        }
        if *count > threshold {
            result.findings.push(fan_finding(
                rule_id,
                direction,
                module_id,
                *count,
                threshold,
                &paths,
                META_FAN_MEANING.get(direction).copied().unwrap_or(direction),
            ));
        }
    }
    Ok(result)
}

static META_FAN_MEANING: LazyLock<BTreeMap<&'static str, &'static str>> = LazyLock::new(|| {
    [
        ("fan-out", "module fan-out (resolved outgoing dependencies)"),
        ("fan-in", "module fan-in (resolved incoming dependencies)"),
    ]
    .into_iter()
    .collect()
});

/// 供注册表引用的模块种类提示（避免未使用导入告警）。
#[allow(dead_code)]
fn _kind_is_file(kind: ModuleKind) -> bool {
    matches!(kind, ModuleKind::FileModule)
}
