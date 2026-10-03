//! §167/§198 side-effect e2e：真实 git 仓库中 check 写文件 ⇒
//! before/after Git status 差异被记录为 verification side effect。

use prism_core::config::{VerifyCheckConfig, VerifyConfig};
use prism_core::runtime::command::{CommandRunner, StdCommandRunner};
use prism_core::task::CancellationToken;
use prism_git::GitService;
use prism_verify::{discover_checks, execute_plan_observed, CheckState};
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new(name: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "prism-verify-side-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.subsec_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let git = |args: &[&str]| {
            let output = Command::new("git")
                .args(args)
                .current_dir(&root)
                .output()
                .expect("git available");
            assert!(output.status.success());
        };
        git(&["init", "-b", "main"]);
        git(&["config", "user.name", "Prism Test"]);
        git(&["config", "user.email", "test@prism.local"]);
        std::fs::write(root.join("base.txt"), "base
").unwrap();
        git(&["add", "-A"]);
        git(&["commit", "-m", "base"]);
        Fixture { root }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

#[test]
fn side_effects_recorded_from_git_status_diff() {
    let fixture = Fixture::new("side");
    // 自定义 check：写一个新文件（BuildArtifacts 允许）。
    let mut config = VerifyConfig::default();
    config.checks.insert(
        "generate".to_string(),
        VerifyCheckConfig {
            kind: "build".to_string(),
            program: if cfg!(windows) { "cmd.exe" } else { "sh" }.to_string(),
            args: if cfg!(windows) {
                vec!["/C".to_string(), "echo generated > out.txt".to_string()]
            } else {
                vec!["-c".to_string(), "echo generated > out.txt".to_string()]
            },
            working_directory: None,
            timeout_ms: None,
            required: None,
            write_policy: Some("build_artifacts".to_string()),
            depends_on: vec![],
        },
    );

    let specs = discover_checks(&fixture.root, &config);
    let cancel = CancellationToken::new();
    let runner: Arc<dyn CommandRunner> = Arc::new(StdCommandRunner::new());
    let git = GitService::new(runner.clone());

    let runs = execute_plan_observed(
        &specs,
        runner.clone(),
        &fixture.root,
        2,
        &git,
        &cancel,
    )
    .unwrap();

    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].state, CheckState::Passed);
    assert!(
        runs[0].side_effects.iter().any(|p| p.ends_with("out.txt")),
        "generated file observed as side effect: {:?}",
        runs[0].side_effects
    );
}

// ---------- §276/§277：verified_state（CleanAtCommit/DirtyWorkspace） ----------

fn run_with_git(root: &std::path::Path) -> prism_verify::VerificationReport {
    let runner: Arc<dyn CommandRunner> = Arc::new(StdCommandRunner::new());
    let git = GitService::new(runner.clone());
    let config = VerifyConfig::default();
    let specs = discover_checks(root, &config);
    prism_verify::run_verification(
        &specs,
        root,
        "demo",
        &prism_verify::VerifyOptions::default(),
        runner,
        2,
        true,
        Some(&git),
        &[],
        &CancellationToken::new(),
    )
    .unwrap()
}

#[test]
fn clean_worktree_reports_clean_at_commit() {
    let fixture = Fixture::new("clean");
    let report = run_with_git(&fixture.root);
    match report.verified_state {
        Some(prism_verify::VerifiedState::CleanAtCommit { commit }) => {
            let head = Command::new("git")
                .args(["rev-parse", "HEAD"])
                .current_dir(&fixture.root)
                .output()
                .unwrap();
            let expected = String::from_utf8_lossy(&head.stdout).trim().to_string();
            assert_eq!(commit.as_deref(), Some(expected.as_str()));
        }
        other => panic!("expected CleanAtCommit, got {other:?}"),
    }
}

#[test]
fn dirty_worktree_reports_dirty_with_count() {
    let fixture = Fixture::new("dirty");
    // §277：验证时有未提交变更（含 untracked）⇒ DirtyWorkspace + 数量。
    std::fs::write(fixture.root.join("draft.txt"), "wip\n").unwrap();
    let report = run_with_git(&fixture.root);
    match report.verified_state {
        Some(prism_verify::VerifiedState::DirtyWorkspace { changed_files }) => {
            assert_eq!(changed_files, 1);
        }
        other => panic!("expected DirtyWorkspace, got {other:?}"),
    }
}

#[test]
fn non_git_directory_reports_unknown_verified_state() {
    let root = std::env::temp_dir().join(format!(
        "prism-verify-nogit-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(&root).unwrap();
    let report = run_with_git(&root);
    assert!(
        report.verified_state.is_none(),
        "non-git dir must not fake a verified_state: {:?}",
        report.verified_state
    );
    let _ = std::fs::remove_dir_all(&root);
}
