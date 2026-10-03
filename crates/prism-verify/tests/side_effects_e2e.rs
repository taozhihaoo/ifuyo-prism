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
