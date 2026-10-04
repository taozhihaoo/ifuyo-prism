//! Context Health（规格 M10b §61–§73/§126–§130/§274–§282/§338–§346）。
//!
//! 与 M7 的边界（§62/§274）：M7 = 项目工程健康；Context Health 只回答
//! "本次生成的 context 是否 current/reachable/coherent/useful"。
//! 复用 M7 architecture（core Severity/Confidence + CTX rule id + 证据），
//! 不造第二套 severity（§341）。全部规则保守、evidence-first（§339），
//! 不做语义猜测（§73/§340）。

use crate::model::{ContextFreshness, ContextPack, TargetResolution};
use prism_core::{Confidence, Severity};
use serde::{Deserialize, Serialize};

/// Context Health 总体状态（§127/§259）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextHealthStatus {
    #[default]
    Healthy,
    Attention,
    Incomplete,
    Conflict,
    Unavailable,
}

impl ContextHealthStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            ContextHealthStatus::Healthy => "healthy",
            ContextHealthStatus::Attention => "attention",
            ContextHealthStatus::Incomplete => "incomplete",
            ContextHealthStatus::Conflict => "conflict",
            ContextHealthStatus::Unavailable => "unavailable",
        }
    }
}

/// Context Health finding（§64：统一 Finding architecture——core Severity/
/// Confidence + CTX rule id，§341/§342；suppression 沿用 M7 系统 §343）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextHealthFinding {
    /// 稳定规则 id（CTX001–CTX012，§338）。
    pub rule_id: String,
    pub severity: Severity,
    pub confidence: Confidence,
    pub title: String,
    pub message: String,
    /// 证据（§339 evidence-first：两侧事实都列出，§65/§69/§72）。
    pub evidence: Vec<String>,
}

/// 覆盖信号（§129/§277：分项覆盖，无总分）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextCoverage {
    pub target: String,
    pub impact: String,
    pub verification: String,
    pub docs: String,
    pub environment: String,
}

/// Context Health（§126：无单一数字 score，§462）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextHealth {
    pub status: ContextHealthStatus,
    pub findings: Vec<ContextHealthFinding>,
    pub coverage: ContextCoverage,
    /// §130：limitations（同 manifest.limitations，冗余呈现层 convenience）。
    pub limitations: Vec<String>,
}

impl Default for ContextHealth {
    fn default() -> Self {
        ContextHealth {
            status: ContextHealthStatus::Healthy,
            findings: Vec::new(),
            coverage: ContextCoverage {
                target: "unknown".to_string(),
                impact: "unknown".to_string(),
                verification: "unavailable".to_string(),
                docs: "unknown".to_string(),
                environment: "unavailable".to_string(),
            },
            limitations: Vec::new(),
        }
    }
}

/// Context Health 评估输入（从已生成的 pack + 原始事实派生，无新探测）。
pub struct HealthInputs {
    pub verification_stale: ContextFreshness,
    pub environment_present: bool,
    pub docs_count: usize,
}

/// 评估 Context Health（§128 signals → CTX findings → status）。
pub fn evaluate(pack: &ContextPack, inputs: &HealthInputs) -> ContextHealth {
    let mut findings: Vec<ContextHealthFinding> = Vec::new();

    // CTX005 Verification result stale（§131–§132：旧 pass 不当当前 pass）。
    if inputs.verification_stale == ContextFreshness::Stale {
        findings.push(ContextHealthFinding {
            rule_id: "CTX005".to_string(),
            severity: Severity::Warning,
            confidence: Confidence::High,
            title: "Verification result stale".to_string(),
            message: "Verification facts were captured on a different workspace state \
                      (StaleForCurrentWorkspace); they are included as recorded facts, \
                      not as the current pass/fail."
                .to_string(),
            evidence: vec!["M8 verified_state=DirtyWorkspace vs current state".to_string()],
        });
    }

    // CTX007 Target ambiguous（§16/§258）。
    if pack
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.kind == "target_ambiguous")
    {
        findings.push(ContextHealthFinding {
            rule_id: "CTX007".to_string(),
            severity: Severity::Warning,
            confidence: Confidence::High,
            title: "Target ambiguous".to_string(),
            message: "Task target matched multiple locations; context includes all \
                      candidates rather than guessing."
                .to_string(),
            evidence: pack
                .diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.kind == "target_ambiguous")
                .map(|diagnostic| diagnostic.detail.clone())
                .collect(),
        });
    }

    // CTX008 Context candidate unavailable（§326）。
    let unavailable = pack
        .items
        .iter()
        .filter(|item| item.unavailable_reason.is_some())
        .count();
    if unavailable > 0 {
        findings.push(ContextHealthFinding {
            rule_id: "CTX008".to_string(),
            severity: Severity::Warning,
            confidence: Confidence::High,
            title: "Context candidate unavailable".to_string(),
            message: format!(
                "{unavailable} selected item(s) could not be materialized (missing/unreadable)."
            ),
            evidence: pack
                .items
                .iter()
                .filter_map(|item| {
                    item.unavailable_reason
                        .as_ref()
                        .map(|reason| format!("{}: {reason}", item.title))
                })
                .collect(),
        });
    }

    // CTX009 Critical item truncated（§195/§281）。
    let critical_truncated = pack
        .items
        .iter()
        .any(|item| item.priority == crate::model::Priority::Critical && item.truncated);
    if critical_truncated {
        findings.push(ContextHealthFinding {
            rule_id: "CTX009".to_string(),
            severity: Severity::Warning,
            confidence: Confidence::High,
            title: "Critical item truncated".to_string(),
            message: "A Critical item exceeded the remaining budget and was truncated \
                      at a content boundary."
                .to_string(),
            evidence: pack
                .items
                .iter()
                .filter(|item| item.truncated)
                .map(|item| {
                    format!(
                        "{}: {} -> {} bytes",
                        item.title,
                        item.original_size.unwrap_or_default(),
                        item.included_size.unwrap_or_default()
                    )
                })
                .collect(),
        });
    }

    // CTX011 Instruction scope conflict（§57/§60/§103：同名 instruction
    // 出现在多层目录——全部保留，报告事实）。
    let mut instruction_roots: std::collections::BTreeMap<String, Vec<String>> = Default::default();
    for item in &pack.items {
        if item
            .reasons
            .contains(&crate::model::SelectionReason::ProjectInstruction)
        {
            if let Some(path) = &item.source.path {
                let name = path.rsplit('/').next().unwrap_or(path).to_string();
                instruction_roots.entry(name).or_default().push(path.clone());
            }
        }
    }
    for (name, paths) in instruction_roots {
        if paths.len() > 1 {
            findings.push(ContextHealthFinding {
                rule_id: "CTX011".to_string(),
                severity: Severity::Info,
                confidence: Confidence::High,
                title: "Instruction scope overlap".to_string(),
                message: format!(
                    "{name} present at multiple path scopes; all retained \
                     (nearer path is more specific, §60 — root instructions NOT dropped)."
                ),
                evidence: paths,
            });
        }
    }

    // CTX012 Documentation reference ambiguous：需要 docs 提及路径与 M1 交叉
    // 验证的候选集——v1 由 collector 的 path 命中天然覆盖（未命中即不产生
    // doc 候选）；显式规则留 M11 与文档健康深化。

    // ---- 覆盖信号（§129/§277）----
    let target_str = match &pack.manifest.target.resolution {
        TargetResolution::Resolved { .. } => "covered",
        TargetResolution::Ambiguous { .. } => "ambiguous",
        TargetResolution::NotFound => "not_found",
        TargetResolution::None => "none",
    };
    let impact_str = if pack
        .items
        .iter()
        .any(|item| item.group == crate::model::ContextGroup::Impact)
    {
        "available"
    } else {
        "partial"
    };
    let verification_str = if inputs.verification_stale == ContextFreshness::Unknown {
        "unavailable"
    } else {
        "available"
    };
    let docs_str = if inputs.docs_count > 0 {
        if findings
            .iter()
            .any(|finding| finding.rule_id == "CTX011")
        {
            "conflicts"
        } else {
            "covered"
        }
    } else {
        "partial"
    };
    let environment_str = if inputs.environment_present {
        "available"
    } else {
        "unavailable"
    };
    let coverage = ContextCoverage {
        target: target_str.to_string(),
        impact: impact_str.to_string(),
        verification: verification_str.to_string(),
        docs: docs_str.to_string(),
        environment: environment_str.to_string(),
    };

    // ---- 总体状态（§127/§355–§358 同思路；无 score）----
    let has_conflict = pack
        .risk_signals
        .contains(&crate::model::RiskSignalKind::DocumentationConflict);
    let status = if has_conflict {
        ContextHealthStatus::Conflict
    } else if critical_truncated
        || pack.manifest.status == crate::model::PackStatus::Incomplete
    {
        ContextHealthStatus::Incomplete
    } else if findings
        .iter()
        .any(|finding| finding.severity == Severity::Warning)
        || pack.manifest.status == crate::model::PackStatus::Partial
    {
        ContextHealthStatus::Attention
    } else {
        ContextHealthStatus::Healthy
    };

    ContextHealth {
        status,
        findings,
        coverage,
        limitations: Vec::new(),
    }
}
