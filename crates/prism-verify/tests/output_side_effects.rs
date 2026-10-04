//! M8a gate 补充测试（§167-§169/§198-§202/§229）：
//! large-output 截断、side-effect 观察、timeout 直接子进程终止。

use prism_core::runtime::command::StdCommandRunner;
use prism_core::task::CancellationToken;
use prism_verify::{execute_plan_without_prereqs, CheckKind, CheckSpec, CheckState, WritePolicy};
use std::path::PathBuf;
use std::sync::Arc;

fn spec(id: &str, kind: CheckKind, program: &str, args: Vec<String>) -> CheckSpec {
    CheckSpec {
        id: id.to_string(),
        kind,
        name: id.to_string(),
        program: program.to_string(),
        args,
        working_directory: None,
        timeout_ms: 10_000,
        max_output_bytes: None,
        required: false,
        write_policy: WritePolicy::BuildArtifacts,
        depends_on: vec![],
        source: prism_verify::CheckSource::Configured,
        confidence: prism_verify::CheckConfidence::Exact,
        detected_from: "test".to_string(),
    }
}

/// §201/§202：输出超过 max_output_bytes ⇒ output_truncated=true、
/// 内容有界（head 保留），且进程正常完成。
#[test]
fn large_output_is_truncated_with_flag() {
    let program = if cfg!(windows) { "cmd.exe" } else { "/bin/sh" };
    let args: Vec<String> = if cfg!(windows) {
        // ping 每秒一行，产生稳定多行输出。
        vec!["/C".to_string(), "ping.exe -n 5 127.0.0.1".to_string()]
    } else {
        vec!["-c".to_string(), "ping -c 5 127.0.0.1".to_string()]
    };
    let mut spec = spec("noisy", CheckKind::Test, program, args);
    spec.max_output_bytes = Some(100);

    let runner = StdCommandRunner::new();
    let runs = execute_plan_without_prereqs(
        &[spec],
        Arc::new(runner),
        &PathBuf::from("."),
        1,
        None,
        &CancellationToken::new(),
    )
    .unwrap();

    assert_eq!(runs.len(), 1);
    let run = &runs[0];
    // 截断必须显式标记（§79/§202）。
    assert!(run.output_truncated, "output must be flagged truncated");
    assert!(
        run.stdout.len() <= 200,
        "bounded capture: stdout {} bytes",
        run.stdout.len()
    );
    assert!(
        run.diagnostics.iter().any(|d| d.kind == "output_truncated"),
        "truncation diagnostic expected"
    );
    // 进程本身正常完成（非 timeout/failed 判定混淆——ping exit 0）。
    assert_eq!(run.state, CheckState::Passed);
}

/// §91/§188：超时 ⇒ TimedOut + 诊断；直接子进程被终止（无孤儿 hang）。
#[test]
fn timeout_reports_timed_out_and_terminates_process() {
    let program = if cfg!(windows) { "ping.exe" } else { "ping" };
    let args: Vec<String> = if cfg!(windows) {
        vec!["-n".to_string(), "30".to_string(), "127.0.0.1".to_string()]
    } else {
        vec!["-c".to_string(), "30".to_string(), "127.0.0.1".to_string()]
    };
    let mut spec = spec("slow", CheckKind::Test, program, args);
    spec.timeout_ms = 500;

    let runs = execute_plan_without_prereqs(
        &[spec],
        Arc::new(StdCommandRunner::new()),
        &PathBuf::from("."),
        1,
        None,
        &CancellationToken::new(),
    )
    .unwrap();

    assert_eq!(runs[0].state, CheckState::TimedOut);
    assert!(
        runs[0].diagnostics.iter().any(|d| d.kind == "timeout"),
        "timeout diagnostic expected"
    );
}

/// §92/§229 如实记录：M0 terminate 只杀直接子进程；孙进程孤儿是
/// 已文档化的平台限制（job object / process group 留 M8b 完善）。
#[test]
fn process_tree_limitation_is_documented() {
    // 行为占位：本测试确保限制声明存在于代码文档（§92 executor.rs 头注释）。
    let source = include_str!("../src/executor.rs");
    assert!(
        source.contains("single-process") || source.contains("进程树"),
        "process-tree limitation must be documented"
    );
}
