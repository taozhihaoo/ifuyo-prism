//! `prism verify` CLI 契约测试（规格 M8 §141–§148/§165/§241/§247）。

use std::path::PathBuf;
use std::process::{Command, Output};

fn prism(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_prism"))
        .args(args)
        .output()
        .expect("failed to spawn prism binary")
}

fn stdout_text(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn git(root: &std::path::Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .expect("git must be available");
    assert!(
        output.status.success(),
        "git {:?} failed: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
}

/// 带 package.json scripts 的 Node fixture（build/test/lint + deploy 不识别）。
fn temp_project(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "prism-cli-verify-{name}-{}-{}",
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
    "deploy": "node deploy.js"
  }
}"#,
    )
    .unwrap();
    git(&dir, &["init", "-b", "main"]);
    git(&dir, &["config", "user.name", "Prism Test"]);
    git(&dir, &["config", "user.email", "test@prism.local"]);
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-m", "base"]);
    dir
}

#[test]
fn verify_dry_run_shows_plan_without_execution() {
    // §144–§146：只发现 + 展示计划；未知 script 不发明（§23）。
    let dir = temp_project("dry-run");
    let output = prism(&["verify", dir.to_str().unwrap(), "--dry-run"]);
    assert!(output.status.success(), "dry-run exits 0");
    let text = stdout_text(&output);
    assert!(text.contains("Verify Plan"));
    assert!(text.contains("npm run build"), "build check planned");
    assert!(text.contains("npm test"), "test check planned");
    assert!(
        text.contains("package.json scripts.build"),
        "§236 provenance"
    );
    assert!(!text.contains("deploy"), "§23: unknown script not invented");
    assert!(text.contains("No commands executed."));
}

#[test]
fn verify_json_outputs_stable_contract() {
    let dir = temp_project("json");
    // npm/tsc 在测试环境可能不存在 ⇒ Unavailable；JSON 契约仍然完整。
    let output = prism(&["verify", dir.to_str().unwrap(), "--dry-run", "--json"]);
    // npm 可能不存在 ⇒ Unavailable；契约仍然完整。
    assert!(
        output.status.success() || output.status.code() == Some(1),
        "exit 0 or 1 acceptable: {}",
        output.status.code().unwrap_or(-1)
    );
    let value: serde_json::Value = serde_json::from_str(&stdout_text(&output)).unwrap();
    for field in ["plan", "runs", "conclusion", "counts", "completeness"] {
        assert!(
            value.get(field).is_some(),
            "schema field {field} (§200-style)"
        );
    }
    assert!(value["plan"]["checks"].as_array().unwrap().len() == 3);
}

#[test]
fn verify_unknown_check_rejected() {
    // §148：unknown check 显式报错（exit 3）。
    let dir = temp_project("unknown");
    let output = prism(&["verify", dir.to_str().unwrap(), "--check", "foobar"]);
    assert_eq!(output.status.code(), Some(3), "unknown check ⇒ 3");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("unknown check"),
        "stderr explains: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn verify_missing_path_exits_3() {
    let missing = std::env::temp_dir().join("prism-verify-missing-xyz");
    let output = prism(&["verify", missing.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(3));
}

#[test]
fn verify_help_documents_semantics() {
    // §142：--help 真实可用，说明 commands/scope/safety。
    let output = prism(&["verify", "--help"]);
    assert!(output.status.success());
    let text = stdout_text(&output);
    assert!(text.contains("dry-run"));
    assert!(text.contains("--check"));
}
