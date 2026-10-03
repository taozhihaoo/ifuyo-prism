//! ContextService 编排（规格 M10a §87/§252–§254/§258–§264/§474–§475）。
//!
//! CLI 与 GUI 共用同一 service（§475）；输入全部来自调用方的 M1–M9 既有
//! 结果（§263，引擎不重算）；取消/预算/失败隔离贯穿（§259–§261）。

use crate::collectors::{collect_candidates, FactsInputs};
use crate::model::{
    ContextDiagnostic, ContextGroup, ContextItem, ContextManifest, ContextPack, ContextRequest,
    ContextSummary, PackStatus, SourceType, TargetResolution,
    CONTEXT_SCHEMA_VERSION,
};
use crate::selector::{select, InvalidBudget};
use prism_core::task::CancellationToken;
use prism_core::PrismResult;

pub struct ContextService;

impl ContextService {
    /// 生成 ContextPack（§7/§474）。
    #[allow(clippy::too_many_arguments)]
    pub fn generate(
        request: ContextRequest,
        discovery: Option<&prism_project::ProjectDiscovery>,
        code_model: Option<&prism_analysis::ProjectCodeModel>,
        changeset: Option<&prism_changes::ChangeSet>,
        impact: Option<&prism_impact::ImpactAnalysis>,
        graph: Option<&prism_graph::DependencyGraphResult>,
        health: Option<&prism_health::HealthAnalysis>,
        verification: Option<&prism_verify::VerificationReport>,
        environment: Option<&prism_env::EnvironmentSnapshot>,
        cancel: &CancellationToken,
    ) -> PrismResult<ContextPack> {
        // ---- 1) 任务分类（§12–§18）----
        let classification = crate::task::classify(request.task.as_deref());

        // ---- 2) 目标解析（§15–§16）：显式 target 优先，其次从 task 提取 ----
        let explicit_target = request.target.clone().or_else(|| {
            crate::task::extract_target(request.task.as_deref())
        });
        let target =
            crate::task::resolve_target(explicit_target.as_deref(), discovery, code_model);

        // ---- 3) 候选收集（§87–§89；collector 隔离，§260）----
        let mut diagnostics = Vec::new();
        let inputs = FactsInputs {
            discovery,
            code_model,
            changeset,
            impact,
            graph,
            health,
            verification,
            environment,
            classification: &classification,
            target: &target,
            scope: request.scope,
            cancel,
        };
        let candidates = collect_candidates(&inputs);

        // ---- 4) Scope 过滤 + 选择 + 预算（§82–§90/§197–§201）----
        let (scoped, scope_excluded) =
            crate::selector::apply_scope(candidates, request.scope, &target);
        let selection = match select(scoped, &request.budget, &classification) {
            Ok(output) => output,
            Err(InvalidBudget(message)) => {
                // §200：零预算 ⇒ 明确 validation error。
                return Err(prism_core::PrismError::InvalidInput {
                    message: message.to_string(),
                });
            }
        };
        let mut trace = selection.trace;
        trace.excluded.extend(scope_excluded);

        // ---- 5) 懒物化（§202/§254）----
        let mut items = selection.selected;
        let materialize_report = crate::materializer::materialize(
            &mut items,
            std::path::Path::new(&request.project_root),
            code_model,
            &request.budget,
            cancel,
        );

        // ---- 6) 诊断（§258）----
        match &target.resolution {
            TargetResolution::Ambiguous { candidates } => diagnostics.push(ContextDiagnostic {
                kind: "target_ambiguous".to_string(),
                subject: Some(target.raw.clone()),
                detail: format!(
                    "Target ambiguous — candidates: {}",
                    candidates.join(", ")
                ),
            }),
            TargetResolution::NotFound => diagnostics.push(ContextDiagnostic {
                kind: "target_not_found".to_string(),
                subject: Some(target.raw.clone()),
                detail: "Target mentioned in task but not found in the project.".to_string(),
            }),
            _ => {}
        }
        if materialize_report.unavailable > 0 {
            diagnostics.push(ContextDiagnostic {
                kind: "items_unavailable".to_string(),
                subject: None,
                detail: format!(
                    "{} item(s) could not be materialized (missing/unreadable files).",
                    materialize_report.unavailable
                ),
            });
        }
        if cancel.is_cancelled() {
            diagnostics.push(ContextDiagnostic {
                kind: "cancelled".to_string(),
                subject: None,
                detail: "Context generation cancelled — remaining items left metadata-only."
                    .to_string(),
            });
        }

        // ---- 7) 分组排序（§114–§116）：组间 default_rank，组内 band + id ----
        items.sort_by(|a, b| {
            a.group
                .default_rank()
                .cmp(&b.group.default_rank())
                .then_with(|| a.priority.cmp(&b.priority))
                .then_with(|| a.id.cmp(&b.id))
        });

        // ---- 8) 汇总 + 状态（§259/§385–§388）----
        let summary = summarize(&items);
        let critical_truncated = items
            .iter()
            .any(|item| item.priority == crate::model::Priority::Critical && item.truncated);
        let status = if cancel.is_cancelled() {
            PackStatus::Partial
        } else if critical_truncated || target.resolution == TargetResolution::NotFound {
            PackStatus::Incomplete
        } else if materialize_report.unavailable > 0 || !diagnostics.is_empty() {
            PackStatus::Partial
        } else {
            PackStatus::Complete
        };

        let mut limitations = Vec::new();
        if critical_truncated {
            limitations.push("Critical item truncated by budget (§195).".to_string());
        }
        if target.resolution == TargetResolution::NotFound {
            limitations.push("Task target not found in project; context is generic.".to_string());
        }
        if classification.specificity == crate::model::TaskSpecificity::Generic {
            // §18：明确声明任务具体性有限。
            limitations.push(
                "Task specificity limited — generic project overview context generated."
                    .to_string(),
            );
        }
        if materialize_report.truncated > 0 {
            limitations.push(format!(
                "{} item(s) truncated by budget; see per-item truncation facts.",
                materialize_report.truncated
            ));
        }

        let manifest = ContextManifest {
            schema_version: CONTEXT_SCHEMA_VERSION,
            id: uuid::Uuid::new_v4().to_string(),
            project_root: request.project_root.clone(),
            task: request.task.clone(),
            classification,
            target,
            scope: request.scope,
            generated_at: prism_core::Timestamp::now_utc().to_string(),
            status,
            summary,
            budget: selection.budget,
            limitations,
        };

        Ok(ContextPack {
            manifest,
            items,
            selection: trace,
            diagnostics,
        })
    }
}

/// §385/§386：files/symbols/docs/findings/checks/items/ranges 分开计数。
fn summarize(items: &[ContextItem]) -> ContextSummary {
    let mut summary = ContextSummary::default();
    let mut files = std::collections::BTreeSet::new();
    for item in items {
        summary.items += 1;
        summary.ranges += item.ranges.len();
        match item.source.source_type {
            SourceType::File | SourceType::Symbol => {
                if let Some(path) = &item.source.path {
                    files.insert(path.clone());
                }
                if item.source.source_type == SourceType::Symbol {
                    summary.symbols += 1;
                }
            }
            SourceType::Documentation => summary.docs += 1,
            SourceType::HealthFinding => summary.findings += 1,
            SourceType::CheckRun => summary.checks += 1,
            _ => {}
        }
    }
    summary.files = files.len();
    summary
}

/// 文本呈现（M10a 基础形态；完整 Renderer 归 M10b §349+）。
pub fn render_text(pack: &ContextPack) -> String {
    let mut out = String::new();
    let manifest = &pack.manifest;
    out.push_str(&format!(
        "Context Pack {} [{}] status={:?} task={}\n",
        manifest.id,
        manifest.scope_str(),
        manifest.status,
        manifest.task.as_deref().unwrap_or("(none)")
    ));
    out.push_str(&format!(
        "Items: {} selected · {} excluded · est tokens {}/{}\n",
        manifest.budget.selected_items,
        manifest.budget.excluded_items,
        manifest.budget.used_tokens_estimate,
        manifest.budget.limit_tokens_estimate
    ));
    let mut current_group: Option<ContextGroup> = None;
    for item in &pack.items {
        if current_group != Some(item.group) {
            current_group = Some(item.group);
            out.push_str(&format!("\n[{}]\n", item.group.as_str()));
        }
        out.push_str(&format!(
            "  {:<9} {} — {}\n",
            item.priority.as_str(),
            item.title,
            item.reasons
                .iter()
                .map(|reason| reason.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    for limitation in &manifest.limitations {
        out.push_str(&format!("limitation: {limitation}\n"));
    }
    out
}
