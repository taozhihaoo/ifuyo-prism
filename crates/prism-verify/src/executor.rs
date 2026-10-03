//! CheckExecutor + 有界并发调度（规格 M8 §29–§30/§90–§99/§172–§176/§225–§228）。
//!
//! 只记录执行事实（§98），不做质量判断。命令经 M0 CommandRunner 执行（§2）。
//! 输出/命令脱敏（§77/§173）+ 有界采集（M0 max_output_bytes）+ TimedOut/
//! Unavailable/Error 分类（§95）。
//!
//! 进程树边界（§92/§229 如实记录）：M0 `terminate` 仅终止**直接子进程**；
//! 孙进程孤儿（npm→node→vitest worker 链）是已文档化的平台限制——
//! Windows Job Object / POSIX process group 清理在 M8b 完善。
//! timeout/cancel 路径已测试直接子进程被终止且状态正确（TimedOut/Cancelled）。

use crate::model::{CheckRun, CheckSpec, CheckState, WritePolicy};
use crate::redact::command_display;
use prism_core::runtime::command::{CommandSpec, CommandRunner};
use prism_core::{PrismError, PrismResult};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

/// 单条 CheckSpec → CheckRun 的执行（§98：CheckSpec → CommandRunner → CheckRun）。
pub fn execute_check(
    spec: &CheckSpec,
    runner: &dyn CommandRunner,
    project_root: &Path,
    cancel: &prism_core::task::CancellationToken,
) -> PrismResult<CheckRun> {
    execute_check_with_prereqs(spec, runner, project_root, true, cancel)
}

/// `enforce_prerequisites=false` 供 FakeCommandRunner 测试基座使用：
/// fake program 不在真实 PATH 上，前置检查会误判 Unavailable（§180）。
pub fn execute_check_with_prereqs(
    spec: &CheckSpec,
    runner: &dyn CommandRunner,
    project_root: &Path,
    enforce_prerequisites: bool,
    cancel: &prism_core::task::CancellationToken,
) -> PrismResult<CheckRun> {
    let started_at = prism_core::timestamp::Timestamp::now_utc().to_string();
    let instant = Instant::now();
    let working_directory = spec
        .working_directory
        .clone()
        .unwrap_or_else(|| ".".to_string());
    let command_display = command_display(&spec.program, &spec.args);

    let mut run = CheckRun {
        run_id: uuid::Uuid::new_v4().to_string(),
        check_id: spec.id.clone(),
        kind: spec.kind,
        name: spec.name.clone(),
        required: spec.required,
        state: CheckState::Running,
        command_display: command_display.clone(),
        working_directory: working_directory.clone(),
        started_at: Some(started_at),
        finished_at: None,
        duration_ms: 0,
        exit_code: None,
        stdout: String::new(),
        stderr: String::new(),
        output_truncated: false,
        environment_metadata: Vec::new(),
        diagnostics: Vec::new(),
        reason: None,
        side_effects: Vec::new(),
    };

    // ---- WritePolicy（§85–§87）：ArbitraryWrite 默认拒绝执行 ----
    if spec.write_policy == WritePolicy::ArbitraryWrite {
        run.state = CheckState::Skipped;
        run.reason = Some(
            "skipped: command may modify source (arbitrary_write); configure write_policy explicitly to allow"
                .to_string(),
        );
        run.diagnostics.push(verify_diagnostic(
            "write_policy_refused",
            run.reason.clone().unwrap_or_default(),
        ));
        return Ok(run);
    }

    // ---- Prerequisite：ExecutableExists（§20/§159–§161）----
    let workdir = project_root.join(&working_directory);
    if enforce_prerequisites && !crate::tool::executable_exists(&spec.program, &workdir) {
        run.state = CheckState::Unavailable;
        run.reason =
            Some(format!("Required executable '{}' was not found.", spec.program));
        run.diagnostics.push(verify_diagnostic(
            "program_not_found",
            run.reason.clone().unwrap_or_default(),
        ));
        return Ok(run);
    }

    // ---- 真实执行（M0 CommandRunner，§2）----
    let mut command_spec = CommandSpec::new(spec.program.clone(), spec.args.iter().cloned())
        .with_working_directory(workdir)
        .with_timeout_ms(spec.timeout_ms);
    if let Some(max_bytes) = spec.max_output_bytes {
        command_spec.max_output_bytes = Some(max_bytes);
    }

    match runner.run(&command_spec, Some(cancel)) {
        Ok(output) => {
            run.exit_code = output.exit_code;
            run.stdout = crate::redact::redact_text(&output.stdout);
            run.stderr = crate::redact::redact_text(&output.stderr);
            run.output_truncated = output.output_truncated;
            run.duration_ms = instant.elapsed().as_millis() as u64;
            run.finished_at = Some(prism_core::timestamp::Timestamp::now_utc().to_string());
            run.state = match output.exit_code {
                Some(0) => CheckState::Passed,
                Some(_) => CheckState::Failed,
                None => CheckState::Error, // 信号终止等异常（§187）
            };
            if output.output_truncated {
                run.diagnostics.push(verify_diagnostic(
                    "output_truncated",
                    "stdout/stderr exceeded capture limit; head retained".to_string(),
                ));
            }
            Ok(run)
        }
        Err(PrismError::Cancelled) => {
            run.state = CheckState::Cancelled;
            run.duration_ms = instant.elapsed().as_millis() as u64;
            run.finished_at = Some(prism_core::timestamp::Timestamp::now_utc().to_string());
            run.reason = Some("cancelled by user/system".to_string());
            Ok(run)
        }
        Err(PrismError::Command { message }) if message.contains("timed out") => {
            run.state = CheckState::TimedOut;
            run.duration_ms = instant.elapsed().as_millis() as u64;
            run.finished_at = Some(prism_core::timestamp::Timestamp::now_utc().to_string());
            run.diagnostics.push(verify_diagnostic(
                "timeout",
                format!(
                    "timed out after {}ms; process termination attempted (§91)",
                    spec.timeout_ms
                ),
            ));
            Ok(run)
        }
        Err(PrismError::Command { message }) => {
            run.duration_ms = instant.elapsed().as_millis() as u64;
            run.finished_at = Some(prism_core::timestamp::Timestamp::now_utc().to_string());
            let kind = if message.contains("os error 2") || message.contains("not found") {
                // §20：工具不存在 ⇒ Unavailable，不是 Failed。
                run.state = CheckState::Unavailable;
                run.reason =
                    Some(format!("Required executable '{}' was not found.", spec.program));
                "program_not_found"
            } else if message.contains("os error 5") || message.contains("permission") {
                run.state = CheckState::Error;
                "permission_denied"
            } else if message.contains("directory") {
                run.state = CheckState::Error;
                "working_directory_missing"
            } else {
                run.state = CheckState::Error;
                "spawn_failed"
            };
            run.diagnostics.push(verify_diagnostic(kind, message));
            Ok(run)
        }
        Err(err) => Err(err),
    }
}

fn verify_diagnostic(kind: &str, detail: String) -> crate::model::VerifyDiagnostic {
    crate::model::VerifyDiagnostic {
        kind: kind.to_string(),
        detail,
    }
}

fn cancelled_run(spec: &CheckSpec, reason: &str) -> CheckRun {
    CheckRun {
        run_id: uuid::Uuid::new_v4().to_string(),
        check_id: spec.id.clone(),
        kind: spec.kind,
        name: spec.name.clone(),
        required: spec.required,
        state: CheckState::Cancelled,
        command_display: command_display(&spec.program, &spec.args),
        working_directory: spec
            .working_directory
            .clone()
            .unwrap_or_else(|| ".".to_string()),
        started_at: None,
        finished_at: None,
        duration_ms: 0,
        exit_code: None,
        stdout: String::new(),
        stderr: String::new(),
        output_truncated: false,
        environment_metadata: Vec::new(),
        diagnostics: Vec::new(),
        reason: Some(reason.to_string()),
        side_effects: Vec::new(),
    }
}

/// `enforce_prerequisites=false`：FakeCommandRunner 测试基座（§180）。
pub fn execute_plan_without_prereqs(
    plan: &[CheckSpec],
    runner: Arc<dyn CommandRunner>,
    project_root: &Path,
    max_concurrency: usize,
    cancel: &prism_core::task::CancellationToken,
) -> PrismResult<Vec<CheckRun>> {
    execute_plan_inner(plan, runner, project_root, max_concurrency, false, None, cancel)
}

/// 有界并发执行（§29–§30）：按拓扑层分组，层间顺序（依赖语义），
/// 层内最多 `max_concurrency` 个 worker；结果按计划顺序呈现（§225）。
/// 依赖非 Passed（Failed/Unavailable/Skipped）⇒ Blocked（§27/§161）。
pub fn execute_plan_with(
    plan: &[CheckSpec],
    runner: Arc<dyn CommandRunner>,
    project_root: &Path,
    max_concurrency: usize,
    cancel: &prism_core::task::CancellationToken,
) -> PrismResult<Vec<CheckRun>> {
    execute_plan_inner(plan, runner, project_root, max_concurrency, true, None, cancel)
}

/// 带 M4 Git 副作用观察的执行（§167/§198）：每层前后各取一次 GitStatus，
/// 差异路径归因到该层所有 runs（层粒度，§168 只记录不回滚不归类为问题）。
pub fn execute_plan_observed(
    plan: &[CheckSpec],
    runner: Arc<dyn CommandRunner>,
    project_root: &Path,
    max_concurrency: usize,
    git: &prism_git::GitService,
    cancel: &prism_core::task::CancellationToken,
) -> PrismResult<Vec<CheckRun>> {
    execute_plan_inner(
        plan,
        runner,
        project_root,
        max_concurrency,
        true,
        Some(git),
        cancel,
    )
}


pub(crate) fn execute_plan_inner(
    plan: &[CheckSpec],
    runner: Arc<dyn CommandRunner>,
    project_root: &Path,
    max_concurrency: usize,
    enforce_prerequisites: bool,
    git: Option<&prism_git::GitService>,
    cancel: &prism_core::task::CancellationToken,
) -> PrismResult<Vec<CheckRun>> {
    let mut dependencies: std::collections::BTreeMap<String, Vec<String>> = Default::default();
    let ids: std::collections::BTreeSet<String> = plan.iter().map(|spec| spec.id.clone()).collect();
    for spec in plan {
        dependencies.insert(
            spec.id.clone(),
            spec.depends_on
                .iter()
                .filter(|dep| ids.contains(*dep))
                .cloned()
                .collect(),
        );
    }

    let mut remaining: Vec<&CheckSpec> = plan.iter().collect();
    let mut placed: std::collections::BTreeSet<String> = Default::default();
    let mut outcomes: std::collections::BTreeMap<String, CheckState> = Default::default();
    let mut results: Vec<CheckRun> = Vec::new();

    while !remaining.is_empty() {
        if cancel.is_cancelled() {
            for spec in remaining {
                results.push(cancelled_run(spec, "cancelled before execution"));
            }
            return Ok(results);
        }

        let ready: Vec<&CheckSpec> = remaining
            .iter()
            .filter(|spec| {
                dependencies
                    .get(&spec.id)
                    .map(|deps| deps.iter().all(|dep| placed.contains(dep)))
                    .unwrap_or(true)
            })
            .copied()
            .collect();

        let batch: Vec<&CheckSpec> = if ready.is_empty() {
            // 依赖环：按 id 顺序落层并标记 Error（终止保证，§35 思路）。
            let mut rest = remaining.clone();
            rest.sort_by(|a, b| a.id.cmp(&b.id));
            let next = rest.remove(0);
            let mut run = cancelled_run(next, "dependency cycle detected");
            run.state = CheckState::Error;
            run.diagnostics.push(verify_diagnostic(
                "dependency_cycle",
                format!("check '{}' is part of a dependency cycle", next.id),
            ));
            placed.insert(next.id.clone());
            outcomes.insert(next.id.clone(), run.state);
            results.push(run);
            remaining = rest;
            continue;
        } else {
            ready.into_iter().take(max_concurrency.max(1)).collect()
        };

        // 层内有界并发（§30）：batch 已被 take(max_concurrency) 截断，
        // channel 回收结果；本层全部完成才进入下一层。
        let (sender, receiver) = std::sync::mpsc::channel::<CheckRun>();
        let pending: Vec<CheckSpec> = batch.iter().map(|spec| (*spec).clone()).collect();
        let mut remaining_count = pending.len();
        // §167/§198：层前 Git status 快照（仅在提供 GitService 时）。
        let level_before = git.map(|git| snapshot_status(git, project_root, cancel));

        for spec in &pending {
            if cancel.is_cancelled() {
                results.push(cancelled_run(spec, "cancelled before execution"));
                remaining_count -= 1;
                continue;
            }
            // Blocked 传播（§27/§161）：依赖非 Passed ⇒ Blocked（层内顺序判定）。
            let blocked = dependencies
                .get(&spec.id)
                .map(|deps| {
                    deps.iter().any(|dep| {
                        placed.contains(dep)
                            && outcomes.get(dep).is_some_and(|state| *state != CheckState::Passed)
                    })
                })
                .unwrap_or(false);
            if blocked {
                let failed_deps: Vec<String> = dependencies
                    .get(&spec.id)
                    .cloned()
                    .unwrap_or_default();
                let mut run = cancelled_run(spec, "prerequisite failed/unavailable");
                run.state = CheckState::Blocked;
                run.diagnostics.push(verify_diagnostic(
                    "prerequisite_failed",
                    format!("prerequisite not passed: {}", failed_deps.join(", ")),
                ));
                let state = run.state;
                results.push(run);
                placed.insert(spec.id.clone());
                outcomes.insert(spec.id.clone(), state);
                remaining_count -= 1;
                continue;
            }
            let runner = runner.clone();
            let root = PathBuf::from(project_root);
            let cancel = cancel.clone();
            let spec = spec.clone();
            let sender = sender.clone();
            std::thread::spawn(move || {
                let run = execute_check_with_prereqs(
                    &spec,
                    runner.as_ref(),
                    &root,
                    enforce_prerequisites,
                    &cancel,
                )
                .unwrap_or_else(|err| {
                    let mut fallback = cancelled_run(&spec, "internal execution error");
                    fallback.state = CheckState::Error;
                    fallback
                        .diagnostics
                        .push(verify_diagnostic("internal_error", err.to_string()));
                    fallback
                });
                let _ = sender.send(run);
            });
        }
        drop(sender);
        let mut level_runs: Vec<CheckRun> = Vec::new();
        for run in receiver {
            placed.insert(run.check_id.clone());
            outcomes.insert(run.check_id.clone(), run.state);
            remaining_count -= 1;
            level_runs.push(run);
            if remaining_count == 0 {
                break;
            }
        }
        // §269/§35：已落位的 check 从剩余集合移除——缺失此行会导致无限重排。
        remaining.retain(|spec| !placed.contains(&spec.id));

        // §167/§198：层后快照 ⇒ 副作用路径归因到本层全部 runs。
        if let (Some(git), Some(before)) = (git, &level_before) {
            let after = snapshot_status(git, project_root, cancel);
            let side = diff_statuses(before, &after);
            if !side.is_empty() {
                for run in &mut level_runs {
                    run.side_effects = side.clone();
                }
            }
        }
        results.extend(level_runs);
    }

    // —— 按 plan 顺序排序（§225）——
    results.sort_by_key(|run| plan.iter().position(|spec| spec.id == run.check_id));
    Ok(results)
}

/// 兼容别名（engine 使用）。
pub fn execute_plan(
    plan: &[CheckSpec],
    runner: Arc<dyn CommandRunner>,
    project_root: &Path,
    max_concurrency: usize,
    cancel: &prism_core::task::CancellationToken,
    _config: &prism_core::config::HealthConfig,
) -> PrismResult<Vec<CheckRun>> {
    execute_plan_with(plan, runner, project_root, max_concurrency, cancel)
}

/// GitStatus 快照：path → (index_state, working_tree_state, is_untracked)。
type StatusSnapshot = std::collections::BTreeMap<String, (String, String, bool)>;

fn snapshot_status(
    git: &prism_git::GitService,
    root: &Path,
    cancel: &prism_core::task::CancellationToken,
) -> StatusSnapshot {
    let mut snapshot = StatusSnapshot::new();
    if let Ok(status) = git.status(root, false, cancel) {
        for entry in &status.entries {
            snapshot.insert(
                entry.path.clone(),
                (
                    entry.index_state.as_str().to_string(),
                    entry.working_tree_state.as_str().to_string(),
                    entry.is_untracked,
                ),
            );
        }
    }
    snapshot
}

/// before/after 差异：新增、删除或状态变化的路径（§167 verification side effect）。
fn diff_statuses(before: &StatusSnapshot, after: &StatusSnapshot) -> Vec<String> {
    let mut paths: Vec<String> = Vec::new();
    for (path, state) in after {
        if before.get(path) != Some(state) {
            paths.push(path.clone());
        }
    }
    for path in before.keys() {
        if !after.contains_key(path) {
            paths.push(path.clone());
        }
    }
    paths.sort();
    paths.dedup();
    paths
}
