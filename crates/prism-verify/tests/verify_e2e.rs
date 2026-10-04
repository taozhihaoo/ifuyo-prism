//! M8a 引擎 e2e（规格 §149–§161/§179–§191/§198–§202）：FakeCommandRunner 基座。

use prism_core::config::{VerifyCheckConfig, VerifyConfig};
use prism_core::task::CancellationToken;
use prism_verify::{
    build_plan, discover_checks, execute_plan_with, execute_plan_without_prereqs, run_verification,
    CheckKind, CheckSource, CheckSpec, CheckState, FakeCommandRunner, FakeResponse,
    OverallConclusion, WritePolicy,
};
use std::path::PathBuf;
use std::sync::Arc;

fn node_project_root() -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "prism-verify-node-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("package.json"),
        r#"{
  "name": "demo",
  "scripts": {
    "build": "tsc",
    "test": "vitest",
    "lint": "eslint .",
    "format": "prettier --write .",
    "deploy": "node deploy.js"
  }
}"#,
    )
    .unwrap();
    dir
}

fn spec(
    id: &str,
    kind: CheckKind,
    program: &str,
    required: bool,
    depends_on: &[&str],
) -> CheckSpec {
    CheckSpec {
        id: id.to_string(),
        kind,
        name: id.to_string(),
        program: program.to_string(),
        args: vec![],
        working_directory: None,
        timeout_ms: 5_000,
        max_output_bytes: None,
        required,
        write_policy: WritePolicy::BuildArtifacts,
        depends_on: depends_on.iter().map(|s| s.to_string()).collect(),
        source: CheckSource::Configured,
        confidence: prism_verify::CheckConfidence::Exact,
        detected_from: "test".to_string(),
    }
}

// ---------- §24 discovery ----------

#[test]
fn discovery_finds_script_checks_with_provenance() {
    let dir = node_project_root();
    let specs = discover_checks(&dir, &VerifyConfig::default());
    let ids: Vec<&str> = specs.iter().map(|s| s.id.as_str()).collect();
    assert!(ids.contains(&"build"));
    assert!(ids.contains(&"test"));
    assert!(ids.contains(&"lint"));
    // deploy 不在已知 kind 表 ⇒ 不发明（§23）。
    assert!(!ids.contains(&"deploy"));
    let build = specs.iter().find(|s| s.id == "build").unwrap();
    assert_eq!(build.source, CheckSource::PackageScript);
    assert_eq!(build.detected_from, "package.json scripts.build");
    assert_eq!(build.program, "npm");
    assert_eq!(build.args, vec!["run", "build"]);
}

/// §59：configured wins——同 id 只保留一条且来自配置。
#[test]
fn configured_check_overrides_detected_same_id() {
    let dir = node_project_root();
    let mut config = VerifyConfig::default();
    config.checks.insert(
        "test".to_string(),
        VerifyCheckConfig {
            kind: "test".to_string(),
            program: "custom-runner".to_string(),
            args: vec!["--all".to_string()],
            working_directory: None,
            timeout_ms: None,
            required: Some(true),
            write_policy: None,
            depends_on: vec![],
        },
    );
    let specs = discover_checks(&dir, &config);
    let tests: Vec<_> = specs.iter().filter(|s| s.id == "test").collect();
    assert_eq!(tests.len(), 1, "no duplicate execution (§59)");
    assert_eq!(tests[0].program, "custom-runner");
    assert_eq!(tests[0].source, CheckSource::Configured);
    assert!(tests[0].required);
}

// ---------- §27/§151：Build 失败 ⇒ Test Blocked ----------

#[test]
fn build_failure_blocks_dependent_test() {
    let specs = vec![
        spec("build", CheckKind::Build, "failing", true, &[]),
        spec("test", CheckKind::Test, "passing", true, &["build"]),
    ];
    let mut runner = FakeCommandRunner::new();
    runner.script(
        "failing",
        FakeResponse::Output {
            exit_code: 1,
            stdout: String::new(),
            stderr: "boom".to_string(),
        },
    );
    runner.script(
        "passing",
        FakeResponse::Output {
            exit_code: 0,
            stdout: "ok".to_string(),
            stderr: String::new(),
        },
    );

    let runs = execute_plan_without_prereqs(
        &specs,
        Arc::new(runner),
        &PathBuf::from("."),
        2,
        None,
        &CancellationToken::new(),
    )
    .unwrap();
    assert_eq!(runs.len(), 2);
    assert_eq!(runs[0].state, CheckState::Failed);
    assert_eq!(runs[1].state, CheckState::Blocked);
    assert!(runs[1]
        .diagnostics
        .iter()
        .any(|d| d.kind == "prerequisite_failed"));
}

// ---------- §71/§72/§75：required 语义 ----------

#[test]
fn required_failure_makes_conclusion_fail() {
    let specs = vec![
        spec("lint", CheckKind::Lint, "passing", true, &[]),
        spec("audit", CheckKind::Custom, "failing", true, &[]),
    ];
    let mut runner = FakeCommandRunner::new();
    runner.script(
        "passing",
        FakeResponse::Output {
            exit_code: 0,
            stdout: String::new(),
            stderr: String::new(),
        },
    );
    runner.script(
        "failing",
        FakeResponse::Output {
            exit_code: 2,
            stdout: String::new(),
            stderr: String::new(),
        },
    );

    let dir = PathBuf::from(".");
    let report = run_verification(
        &specs,
        &dir,
        "demo",
        &prism_verify::VerifyOptions::default(),
        Arc::new(runner),
        2,
        false,
        None,
        &[],
        &CancellationToken::new(),
    )
    .unwrap();
    assert_eq!(report.conclusion, OverallConclusion::Fail);
    assert_eq!(report.counts.failed, 1);
    assert_eq!(report.counts.passed, 1);
}

#[test]
fn optional_unavailable_keeps_partial_not_fail() {
    // §73：optional Unavailable ⇒ 不把整体变 Failed。
    let specs = vec![
        spec("lint", CheckKind::Lint, "passing", false, &[]),
        spec(
            "typecheck",
            CheckKind::Typecheck,
            "missing-tool",
            false,
            &[],
        ),
    ];
    let mut runner = FakeCommandRunner::new();
    runner.script(
        "missing-tool",
        FakeResponse::SpawnFailed {
            message: "failed to spawn missing-tool: program not found".to_string(),
        },
    ); // §20/§95：spawn 失败 ⇒ Unavailable
    let dir = PathBuf::from(".");
    let report = run_verification(
        &specs,
        &dir,
        "demo",
        &prism_verify::VerifyOptions::default(),
        Arc::new(runner),
        2,
        false,
        None,
        &[],
        &CancellationToken::new(),
    )
    .unwrap();
    assert_eq!(report.conclusion, OverallConclusion::Partial);
    assert_eq!(report.counts.unavailable, 1);
}

#[test]
fn convention_build_failure_blocks_same_toolchain_only() {
    // §27：cargo-build 失败 ⇒ cargo-test Blocked；npm 生态不受牵连。
    let specs = vec![
        spec("npm-build", CheckKind::Build, "npm", false, &[]),
        spec("npm-test", CheckKind::Test, "npm", false, &[]),
        spec("cargo-build", CheckKind::Build, "cargo", false, &[]),
        spec("cargo-test", CheckKind::Test, "cargo", false, &[]),
    ];
    let mut runner = FakeCommandRunner::new();
    runner.script(
        "npm",
        FakeResponse::Output {
            exit_code: 0,
            stdout: String::new(),
            stderr: String::new(),
        },
    );
    runner.script(
        "cargo",
        FakeResponse::Output {
            exit_code: 101,
            stdout: String::new(),
            stderr: String::new(),
        },
    );
    let dir = PathBuf::from(".");
    let report = run_verification(
        &specs,
        &dir,
        "demo",
        &prism_verify::VerifyOptions::default(),
        Arc::new(runner),
        2,
        false,
        None,
        &[],
        &CancellationToken::new(),
    )
    .unwrap();
    let state_of = |id: &str| {
        report
            .runs
            .iter()
            .find(|run| run.check_id == id)
            .map(|run| run.state)
            .unwrap()
    };
    assert_eq!(state_of("cargo-build"), CheckState::Failed);
    assert_eq!(
        state_of("cargo-test"),
        CheckState::Blocked,
        "same-toolchain build failure must block (§27)"
    );
    assert_eq!(state_of("npm-build"), CheckState::Passed);
    assert_eq!(
        state_of("npm-test"),
        CheckState::Passed,
        "npm test must not gate on cargo build"
    );
}

// ---------- §87/§85：WritePolicy 默认拒绝 ----------

#[test]
fn arbitrary_write_skipped_by_default() {
    let mut spec = spec("format", CheckKind::Format, "passing", false, &[]);
    spec.write_policy = WritePolicy::ArbitraryWrite;
    let specs = vec![spec];
    let runner = FakeCommandRunner::new();
    let runs = execute_plan_without_prereqs(
        &specs,
        Arc::new(runner.clone()),
        &PathBuf::from("."),
        2,
        None,
        &CancellationToken::new(),
    )
    .unwrap();
    assert_eq!(runs[0].state, CheckState::Skipped);
    assert!(
        runner.calls().is_empty(),
        "arbitrary_write command must not be spawned (§87)"
    );
}

// ---------- §144/§199：dry-run 零执行 ----------

#[test]
fn dry_run_never_spawns_commands() {
    let dir = node_project_root();
    let options = prism_verify::VerifyOptions {
        dry_run: true,
        ..Default::default()
    };
    let runner = FakeCommandRunner::new();
    let specs = discover_checks(&dir, &config_default());
    let report = run_verification(
        &specs,
        &dir,
        "demo",
        &options,
        Arc::new(runner.clone()),
        2,
        false,
        None,
        &[],
        &CancellationToken::new(),
    )
    .unwrap();
    assert_eq!(report.runs.len(), 0, "dry-run: no executions");
    assert_eq!(report.conclusion, OverallConclusion::NotRun);
    assert!(
        runner.calls().is_empty(),
        "dry-run must not spawn any process"
    );
}

fn config_default() -> VerifyConfig {
    VerifyConfig::default()
}

// ---------- §128/§260-§264：polyglot 发现 id 不冲突 ----------

#[test]
fn polyglot_discovery_assigns_unique_ecosystem_ids() {
    let dir = std::env::temp_dir().join(format!(
        "prism-verify-poly-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("package.json"),
        r#"{ "scripts": { "build": "tsc", "test": "vitest" } }"#,
    )
    .unwrap();
    std::fs::write(dir.join("Cargo.toml"), "[workspace]\nmembers = []\n").unwrap();
    std::fs::write(
        dir.join("pyproject.toml"),
        "[tool.pytest.ini_options]\ntestpaths = [\"tests\"]\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("demo.csproj"),
        "<Project Sdk=\"Microsoft.NET.Sdk\" />",
    )
    .unwrap();

    let specs = discover_checks(&dir, &config_default());
    let ids: Vec<&str> = specs.iter().map(|s| s.id.as_str()).collect();

    // 四个生态都被发现（§260），id 全部唯一（§128）。
    for expected in [
        "build",
        "test",
        "cargo-build",
        "cargo-test",
        "cargo-check",
        "cargo-clippy",
        "pytest",
        "dotnet-build",
        "dotnet-test",
    ] {
        assert!(ids.contains(&expected), "missing id {expected}: {ids:?}");
    }
    let mut sorted = ids.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(ids.len(), sorted.len(), "duplicate check ids: {ids:?}");

    // 约定依赖限同一 toolchain：cargo-test → cargo-build（不是 npm build）。
    let plan = build_plan(
        &dir,
        &config_default(),
        &prism_verify::VerifyOptions::default(),
    )
    .unwrap();
    let cargo_test = plan
        .checks
        .iter()
        .find(|check| check.id == "cargo-test")
        .unwrap();
    assert!(cargo_test.depends_on.contains(&"cargo-build".to_string()));
    assert!(
        !cargo_test.depends_on.contains(&"build".to_string()),
        "cargo-test must not gate on the npm build: {:?}",
        cargo_test.depends_on
    );
    let _ = std::fs::remove_dir_all(&dir);
}

// ---------- §148：unknown check ----------

#[test]
fn unknown_check_selection_is_invalid_input() {
    let dir = node_project_root();
    let options = prism_verify::VerifyOptions {
        selected: Some(vec!["foobar".to_string()]),
        ..Default::default()
    };
    let result = build_plan(&dir, &config_default(), &options);
    assert!(result.is_err(), "§148: unknown check must be rejected");
}

// ---------- §77/§173：脱敏 ----------

#[test]
fn failed_command_output_and_args_are_redacted() {
    let mut spec = spec("audit", CheckKind::Custom, "failing", false, &[]);
    spec.args = vec!["--token".to_string(), "abc123".to_string()];
    let specs = vec![spec];
    let mut runner = FakeCommandRunner::new();
    runner.script(
        "failing",
        FakeResponse::Output {
            exit_code: 1,
            stdout: "MY_TOKEN=supersecret123\n".to_string(),
            stderr: "Authorization: Bearer abc.def.ghi\n".to_string(),
        },
    );

    let runs = execute_plan_without_prereqs(
        &specs,
        Arc::new(runner),
        &PathBuf::from("."),
        2,
        None,
        &CancellationToken::new(),
    )
    .unwrap();
    let run = &runs[0];
    assert_eq!(run.state, CheckState::Failed);
    assert!(run.stdout.contains("[REDACTED]"), "stdout redacted");
    assert!(
        !run.stdout.contains("supersecret123"),
        "secret value absent"
    );
    assert!(
        !run.command_display.contains("abc123"),
        "args redacted (§173)"
    );
}

// ---------- §190：cancel ----------

#[test]
fn pre_cancelled_token_yields_cancelled_runs() {
    let specs = vec![spec("lint", CheckKind::Lint, "passing", false, &[])];
    let cancel = CancellationToken::new();
    cancel.cancel();
    let runs = execute_plan_without_prereqs(
        &specs,
        Arc::new(FakeCommandRunner::new()),
        &PathBuf::from("."),
        2,
        None,
        &cancel,
    )
    .unwrap();
    assert!(runs.iter().all(|run| run.state == CheckState::Cancelled));
}

// ---------- 真实命令 smoke（§182）：跨平台 exit code ----------

#[test]
fn real_command_smoke_exit_zero() {
    // 跨平台 true/exit 0（§183：优先已装 runtime；Windows 用 cmd）。
    let program = if cfg!(windows) { "cmd.exe" } else { "true" };
    let args: Vec<String> = if cfg!(windows) {
        vec!["/C".to_string(), "exit 0".to_string()]
    } else {
        vec![]
    };
    let spec = CheckSpec {
        id: "smoke".to_string(),
        kind: CheckKind::Custom,
        name: "smoke".to_string(),
        program: program.to_string(),
        args,
        working_directory: None,
        timeout_ms: 10_000,
        max_output_bytes: None,
        required: false,
        write_policy: WritePolicy::ReadOnly,
        depends_on: vec![],
        source: CheckSource::BuiltIn,
        confidence: prism_verify::CheckConfidence::Exact,
        detected_from: "real command smoke (§182)".to_string(),
    };
    let runner = prism_core::runtime::command::StdCommandRunner::new();
    let runs = execute_plan_with(
        &[spec],
        Arc::new(runner),
        &PathBuf::from("."),
        1,
        &CancellationToken::new(),
    )
    .unwrap();
    assert_eq!(runs[0].state, CheckState::Passed);
}
