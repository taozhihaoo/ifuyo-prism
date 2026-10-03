//! VerifyEngine 编排（规格 M8 §97/§141–§143/§268–§269）：
//! discover → plan → [prerequisites → schedule → execute] → normalize → summarize → report。

use crate::discovery::discover_checks;
use crate::executor::{execute_plan_with, execute_plan_without_prereqs};
use crate::model::{
    CheckSpec, OverallConclusion, VerificationReport, VerifyCounts, VerifyOptions, VerifyPlan,
    VERIFY_SCHEMA_VERSION,
};
use prism_core::task::CancellationToken;
use prism_core::{PrismError, PrismResult};
use std::path::Path;
use std::sync::Arc;

/// 只发现 + 计划（§144 dry-run 基础；§268 discover ≠ execute）。
pub fn build_plan(
    root: &Path,
    verify_config: &prism_core::config::VerifyConfig,
    options: &VerifyOptions,
) -> PrismResult<VerifyPlan> {
    let specs = discover_checks(root, verify_config);
    let selected = select_specs(&specs, options)?;
    Ok(crate::planner::plan_checks(&selected))
}

/// --check 选择（§147/§148）：unknown id 显式报错。
fn select_specs(
    specs: &[CheckSpec],
    options: &VerifyOptions,
) -> PrismResult<Vec<CheckSpec>> {
    match &options.selected {
        None => Ok(specs.to_vec()),
        Some(selected) => {
            for id in selected {
                if !specs.iter().any(|spec| &spec.id == id) {
                    return Err(PrismError::InvalidInput {
                        message: format!("unknown check id: {id}"),
                    });
                }
            }
            Ok(specs
                .iter()
                .filter(|spec| selected.contains(&spec.id))
                .cloned()
                .collect())
        }
    }
}

/// 完整运行（§143）：plan → execute → aggregate → report。
#[allow(clippy::too_many_arguments)]
pub fn run_verification(
    specs: &[CheckSpec],
    root: &Path,
    project_name: &str,
    options: &VerifyOptions,
    runner: Arc<dyn prism_core::CommandRunner>,
    max_concurrency: usize,
    enforce_prerequisites: bool,
    git: Option<&prism_git::GitService>,
    impacted_test_candidates: &[String],
    cancel: &CancellationToken,
) -> PrismResult<VerificationReport> {
    let selected_specs = select_specs(specs, options)?;
    let plan = crate::planner::plan_checks(&selected_specs);
    if options.dry_run {
        // §144–§146：dry-run 只发现 + 展示计划，零命令执行。
        return Ok(VerificationReport {
            schema_version: VERIFY_SCHEMA_VERSION,
            project_name: project_name.to_string(),
            plan: plan.clone(),
            runs: Vec::new(),
            conclusion: OverallConclusion::NotRun,
            completeness: prism_core::PrismCompleteness::Complete,
            counts: VerifyCounts::default(),
            side_effects: Vec::new(),
            impacted_test_candidates: impacted_test_candidates.to_vec(),
        });
    }
    if cancel.is_cancelled() {
        return Err(PrismError::Cancelled);
    }

    let runs = match (enforce_prerequisites, git) {
        (true, None) => {
            execute_plan_with(&selected_specs, runner, root, max_concurrency, cancel)?
        }
        (true, Some(git)) => crate::executor::execute_plan_observed(
            &selected_specs,
            runner,
            root,
            max_concurrency,
            git,
            cancel,
        )?,
        (false, git) => execute_plan_without_prereqs(
            &selected_specs,
            runner,
            root,
            max_concurrency,
            git,
            cancel,
        )?,
    };

    let mut counts = VerifyCounts::default();
    for run in &runs {
        match run.state {
            crate::model::CheckState::Passed => counts.passed += 1,
            crate::model::CheckState::Failed => counts.failed += 1,
            crate::model::CheckState::Unavailable => counts.unavailable += 1,
            crate::model::CheckState::Blocked => counts.blocked += 1,
            crate::model::CheckState::Skipped => counts.skipped += 1,
            crate::model::CheckState::Cancelled => counts.cancelled += 1,
            crate::model::CheckState::TimedOut => counts.timed_out += 1,
            crate::model::CheckState::Error => counts.error += 1,
            crate::model::CheckState::Queued | crate::model::CheckState::Running => {}
        }
    }

    // §71：required 全过 ⇒ Pass；required failed/unavailable/blocked ⇒ Fail；
    // 混合或全部 optional ⇒ Partial；无运行 ⇒ NotRun。
    let required_runs: Vec<_> = runs.iter().filter(|run| run.required).collect();
    let conclusion = if runs.is_empty() {
        OverallConclusion::NotRun
    } else if required_runs
        .iter()
        .any(|run| run.state != crate::model::CheckState::Passed)
    {
        OverallConclusion::Fail
    } else if !required_runs.is_empty() {
        OverallConclusion::Pass
    } else if counts.failed > 0 || counts.error > 0 {
        OverallConclusion::Fail
    } else if counts.passed > 0 {
        OverallConclusion::Partial
    } else {
        OverallConclusion::NotRun
    };

    let completeness = if counts.cancelled > 0 || counts.error > 0 {
        prism_core::PrismCompleteness::Partial
    } else {
        prism_core::PrismCompleteness::Complete
    };

    // §167/§198：执行期间观测到的验证副作用（去重排序）。
    let mut report_side_effects: Vec<String> = runs
        .iter()
        .flat_map(|run| run.side_effects.iter().cloned())
        .collect();
    report_side_effects.sort();
    report_side_effects.dedup();

    Ok(VerificationReport {
        schema_version: VERIFY_SCHEMA_VERSION,
        project_name: project_name.to_string(),
        plan,
        runs,
        conclusion,
        completeness,
        counts,
        side_effects: report_side_effects,
        impacted_test_candidates: impacted_test_candidates.to_vec(),
    })
}
