//! Verify 报告渲染（规格 M8 §76/§77/§89/§144–§154）。
//! 无 "Verification failed." 空话——每条失败带 command/exit/reason/evidence（§76）。

use crate::model::{CheckState, OverallConclusion, VerificationReport};
use std::fmt::Write as _;

fn state_label(state: CheckState) -> &'static str {
    match state {
        CheckState::Queued => "QUEUED",
        CheckState::Running => "RUNNING",
        CheckState::Passed => "PASSED",
        CheckState::Failed => "FAILED",
        CheckState::Skipped => "SKIPPED",
        CheckState::Cancelled => "CANCELLED",
        CheckState::TimedOut => "TIMED OUT",
        CheckState::Blocked => "BLOCKED",
        CheckState::Unavailable => "UNAVAILABLE",
        CheckState::Error => "ERROR",
    }
}

fn conclusion_label(conclusion: OverallConclusion) -> &'static str {
    match conclusion {
        OverallConclusion::Pass => "PASS",
        OverallConclusion::Fail => "FAIL",
        OverallConclusion::Partial => "PARTIAL",
        OverallConclusion::NotRun => "NOT RUN",
    }
}

/// 文本渲染（§76 失败原因 + §149–§154 UX + §144 dry-run plan）。
pub fn render_text(report: &VerificationReport) -> String {
    let mut out = String::new();
    if report.runs.is_empty() && matches!(report.conclusion, OverallConclusion::NotRun) {
        // dry-run / 空 plan：展示计划（§145）。
        let _ = writeln!(out, "Verify Plan\n");
        for (index, check) in report.plan.checks.iter().enumerate() {
            let _ = writeln!(
                out,
                "[{}] {} — {}",
                index + 1,
                check.kind.as_str(),
                check.name
            );
            let _ = writeln!(out, "    {} {}", check.program, check.args.join(" "));
            let _ = writeln!(out, "    source: {}", check.detected_from);
            if check.required {
                let _ = writeln!(out, "    required");
            }
        }
        let _ = writeln!(out, "\nNo commands executed.");
        return out;
    }

    let counts = &report.counts;
    let _ = writeln!(
        out,
        "Verification: {} (passed {} · failed {} · unavailable {} · blocked {} · skipped {} · cancelled {} · timed out {} · error {})",
        conclusion_label(report.conclusion),
        counts.passed,
        counts.failed,
        counts.unavailable,
        counts.blocked,
        counts.skipped,
        counts.cancelled,
        counts.timed_out,
        counts.error
    );
    let _ = writeln!(out, "Completeness: {:?}\n", report.completeness);
    match &report.verified_state {
        Some(crate::model::VerifiedState::DirtyWorkspace { changed_files }) => {
            // §277 的用户可见文案。
            let _ = writeln!(
                out,
                "{changed_files} working-tree changes present at verification time."
            );
        }
        Some(crate::model::VerifiedState::CleanAtCommit {
            commit: Some(commit),
        }) => {
            let _ = writeln!(out, "Working tree clean at commit {commit}.");
        }
        Some(crate::model::VerifiedState::CleanAtCommit { commit: None }) => {
            let _ = writeln!(out, "Working tree clean (no commits yet).");
        }
        None => {}
    }
    // §307/§34：候选只标注 Potential，不冒充完整 selective verification。
    if !report.impacted_test_candidates.is_empty() {
        let _ = writeln!(out, "\nScope: current changes (working tree)");
        let _ = writeln!(
            out,
            "Impacted test candidates (potential — not a verified selection):"
        );
        for candidate in &report.impacted_test_candidates {
            let _ = writeln!(out, "  - {candidate}");
        }
        out.push('\n');
    }

    for run in &report.runs {
        let _ = writeln!(
            out,
            "[{}] {} {}",
            state_label(run.state),
            run.kind.as_str(),
            run.name
        );
        let _ = writeln!(out, "  Command: {}", run.command_display);
        let _ = writeln!(out, "  Working directory: {}", run.working_directory);
        if let Some(exit_code) = run.exit_code {
            let _ = writeln!(out, "  Exit code: {exit_code}");
        }
        let _ = writeln!(out, "  Duration: {}ms", run.duration_ms);
        for diagnostic in &run.diagnostics {
            let _ = writeln!(out, "  [{}] {}", diagnostic.kind, diagnostic.detail);
        }
        if let Some(reason) = &run.reason {
            let _ = writeln!(out, "  Reason: {reason}");
        }
        if !run.stdout.is_empty() {
            let _ = writeln!(out, "  stdout:\n{}", indent(&run.stdout));
        }
        if !run.stderr.is_empty() {
            let _ = writeln!(out, "  stderr:\n{}", indent(&run.stderr));
        }
        if run.output_truncated {
            let _ = writeln!(out, "  output_truncated = true");
        }
        out.push('\n');
    }
    out
}

fn indent(text: &str) -> String {
    text.lines()
        .map(|line| format!("    {line}\n"))
        .collect::<String>()
}

/// HTML 静态报告（§90）。
pub fn render_html(report: &VerificationReport) -> String {
    let mut rows = String::new();
    for run in &report.runs {
        let _ = writeln!(
            rows,
            "<tr class=\"{state}\"><td>{state}</td><td>{rule}</td><td>{title}</td>\
             <td><code>{command}</code></td><td>{exit}</td><td>{duration}ms</td></tr>",
            state = state_label(run.state),
            rule = run.check_id,
            title = run.name,
            command = run.command_display,
            exit = run
                .exit_code
                .map(|code| code.to_string())
                .unwrap_or_else(|| "-".to_string()),
            duration = run.duration_ms,
        );
    }
    let conclusion = conclusion_label(report.conclusion);
    format!(
        r#"<!DOCTYPE html>
<html lang="en"><head><meta charset="utf-8"><title>Prism Verify Report — {project}</title>
<style>body {{ font-family: system-ui, sans-serif; margin: 2rem; }}
table {{ border-collapse: collapse; width: 100%; }}
td, th {{ border: 1px solid #ddd; padding: .35rem .6rem; text-align: left; }}
tr.FAILED td:first-child {{ color: #b00020; font-weight: 600; }}</style></head><body>
<h1>Prism Verify Report — {project}</h1>
<p>Conclusion: <strong>{conclusion}</strong> · Completeness: {completeness:?}</p>
<table><tr><th>State</th><th>Check</th><th>Title</th><th>Command</th><th>Exit</th><th>Duration</th></tr>
{rows}</table></body></html>"#,
        project = report.project_name,
        completeness = report.completeness,
    )
}
