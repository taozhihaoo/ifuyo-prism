//! Rule 基建（2.0-M28 Phase C：RuleDefinition / RuleRegistry / RuleContext /
//! RuleEvaluator / RuleResult）。
//!
//! 设计约束（§89/§91 "独立但不过度抽象"）：规则 = 纯函数
//! `fn(&RuleContext) -> RuleOutcome`；Registry 持有 (定义, 函数) 对并按 id
//! 确定性排序评估（§41）。规则只消费注入的既有事实（L0 清单 / M23-M25
//! C# 引擎 / M24 Godot Adapter），不自行扫描文件系统、不重跑分析器（§2）。

use crate::model::{Health2Report, Metric};
use crate::rules::SourceFileInput;
use prism_core::task::CancellationToken;
use prism_intel::evidence::Evidence;
use prism_intel::finding::Finding;
use std::collections::BTreeMap;

/// 规则分类（§A-§D 目录的顶层：Structural / Complexity / Semantic / Godot）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RuleCategory {
    Structural,
    Complexity,
    Semantic,
    Godot,
}

impl RuleCategory {
    pub const fn as_str(self) -> &'static str {
        match self {
            RuleCategory::Structural => "structural",
            RuleCategory::Complexity => "complexity",
            RuleCategory::Semantic => "semantic",
            RuleCategory::Godot => "godot",
        }
    }
}

/// 规则定义（注册表元数据；评估函数见 `RuleFn`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuleDefinition {
    /// 稳定规则 id（如 "STRUCT/LARGE_FILE"）。
    pub id: &'static str,
    pub name: &'static str,
    pub category: RuleCategory,
}

/// 单规则产物：Finding + Metric + Evidence + 诊断（§E：Metrics/Relations/
/// Diagnostics 统一转 Findings 的载体）。
#[derive(Debug, Clone, Default)]
pub struct RuleOutcome {
    pub findings: Vec<Finding>,
    pub metrics: Vec<Metric>,
    pub evidence: Vec<Evidence>,
    pub diagnostics: Vec<String>,
}

/// C# 引擎事实注入（M23 抽取 + 可选 M25 语义；None ⇒ 规则记诊断跳过，
/// 不伪造）。
pub struct CSharpFacts<'a> {
    pub extractions: &'a BTreeMap<String, prism_csharp::FileExtraction>,
    pub semantics: Option<&'a prism_csharp::SemanticEngine>,
}

/// 规则上下文：一次评估的全部注入事实（§C RuleContext）。
pub struct RuleContext<'a> {
    pub files: &'a [SourceFileInput],
    pub thresholds: &'a crate::RuleThresholds,
    pub observed_at: &'a str,
    pub csharp: Option<CSharpFacts<'a>>,
    pub godot: Option<&'a prism_godot::GodotAnalysis>,
    pub cancel: &'a CancellationToken,
}

/// 规则函数签名（纯函数；取消检查在循环内自查）。
pub type RuleFn = fn(&RuleContext) -> RuleOutcome;

/// 规则注册表：确定性顺序（按 id 排序）评估全部已注册规则。
#[derive(Default)]
pub struct RuleRegistry {
    entries: Vec<(RuleDefinition, RuleFn)>,
}

impl RuleRegistry {
    pub fn empty() -> Self {
        RuleRegistry {
            entries: Vec::new(),
        }
    }

    /// 注册一条规则（重复 id 以最后注册者为准；评估按 id 排序）。
    pub fn register(&mut self, definition: RuleDefinition, evaluator: RuleFn) {
        if let Some(slot) = self
            .entries
            .iter_mut()
            .find(|(existing, _)| existing.id == definition.id)
        {
            *slot = (definition, evaluator);
        } else {
            self.entries.push((definition, evaluator));
            self.entries.sort_by_key(|(definition, _)| definition.id);
        }
    }

    /// 内建规则集（§A-§D 第一批：Structural / Complexity / Semantic / Godot）。
    pub fn with_builtins() -> Self {
        let mut registry = Self::empty();
        registry.register(
            RuleDefinition {
                id: "STRUCT/LARGE_FILE",
                name: "Large source file",
                category: RuleCategory::Structural,
            },
            crate::rules::large_file_rule,
        );
        registry.register(
            RuleDefinition {
                id: "SYMBOL/SIZE",
                name: "Type/method declaration size",
                category: RuleCategory::Complexity,
            },
            crate::rules_csharp::large_type_rule,
        );
        registry.register(
            RuleDefinition {
                id: "SEMANTIC/UNRESOLVED",
                name: "Unresolved semantic usage",
                category: RuleCategory::Semantic,
            },
            crate::rules_csharp::semantic_diagnostic_rule,
        );
        registry.register(
            RuleDefinition {
                id: "GODOT/BROKEN_WIRING",
                name: "Godot scene wiring problem",
                category: RuleCategory::Godot,
            },
            crate::rules_godot::scene_wiring_rule,
        );
        registry
    }

    pub fn definitions(&self) -> impl Iterator<Item = &RuleDefinition> {
        self.entries.iter().map(|(definition, _)| definition)
    }

    pub fn rule_count(&self) -> usize {
        self.entries.len()
    }

    /// 评估全部规则（§C RuleEvaluator）：按 id 确定性顺序；取消 ⇒ 保留
    /// 已完成部分并如实记诊断（partial 不伪装 success）。
    pub fn evaluate(&self, ctx: &RuleContext) -> Health2Report {
        let mut report = Health2Report {
            schema_version: crate::HEALTH2_SCHEMA_VERSION,
            ..Health2Report::default()
        };
        for (definition, evaluator) in &self.entries {
            if ctx.cancel.is_cancelled() {
                report
                    .diagnostics
                    .push(format!("rule {} skipped: cancelled", definition.id));
                continue;
            }
            let outcome = evaluator(ctx);
            report.findings.extend(outcome.findings);
            report.evidence.extend(outcome.evidence);
            report.diagnostics.extend(
                outcome
                    .diagnostics
                    .into_iter()
                    .map(|d| format!("{}: {d}", definition.id))
                    .collect::<Vec<_>>(),
            );
            for metric in outcome.metrics {
                report.metrics.insert(metric.key.clone(), metric);
            }
        }
        // Finding 确定性输出（§41：同输入同顺序）。
        report.findings.sort_by(|a, b| a.id.cmp(&b.id));
        report
            .evidence
            .sort_by(|a, b| a.id.as_str().cmp(b.id.as_str()));
        report
    }
}
