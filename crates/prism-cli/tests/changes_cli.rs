//! `prism changes` CLI 契约测试（规格 M5 §92）。
//!
//! 真实 git 仓库 + 真实二进制；验证文本/JSON/markdown、scope 参数与退出码。

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
        .env("GIT_AUTHOR_DATE", "2026-01-01T00:00:00+00:00")
        .env("GIT_COMMITTER_DATE", "2026-01-01T00:00:00+00:00")
        .output()
        .expect("git must be available");
    assert!(
        output.status.success(),
        "git {:?} failed: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
}

/// 一个带一次提交 + 一处工作区修改的 TS 仓库。
fn temp_git_project(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "prism-cli-changes-{name}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(
        dir.join("src/app.ts"),
        "export function app() {\n  return 1;\n}\n",
    )
    .unwrap();
    git(&dir, &["init", "-b", "main"]);
    git(&dir, &["config", "user.name", "Prism Test"]);
    git(&dir, &["config", "user.email", "test@prism.local"]);
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-m", "base"]);
    std::fs::write(
        dir.join("src/app.ts"),
        "export function app() {\n  return 42;\n}\n",
    )
    .unwrap();
    dir
}

#[test]
fn changes_reports_text_summary_on_dirty_repo() {
    let dir = temp_git_project("text");
    let output = prism(&["changes", dir.to_str().unwrap()]);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = stdout_text(&output);
    assert!(text.contains("Scope: working_tree"));
    assert!(text.contains("src/app.ts modified"));
    assert!(
        text.contains("~ app modified"),
        "symbol line expected: {text}"
    );
    assert!(text.contains("module:file:src/app.ts"));
}

#[test]
fn changes_json_outputs_stable_changeset_contract() {
    let dir = temp_git_project("json");
    let output = prism(&["changes", dir.to_str().unwrap(), "--format", "json"]);
    assert!(output.status.success());
    let value: serde_json::Value =
        serde_json::from_str(&stdout_text(&output)).expect("valid JSON expected");
    assert!(value["id"].as_str().unwrap().starts_with("changeset:"));
    assert_eq!(value["scope"], "working_tree");
    assert_eq!(value["summary"]["files_modified"], 1);
    assert_eq!(value["files"][0]["new_path"], "src/app.ts");
    let symbol = &value["symbols"][0];
    assert_eq!(symbol["name"], "app");
    assert_eq!(symbol["change_type"], "modified");
    assert_eq!(symbol["evidence"]["source"], "symbol_range_overlap");
}

#[test]
fn changes_markdown_renders_sections() {
    let dir = temp_git_project("markdown");
    let output = prism(&["changes", dir.to_str().unwrap(), "--format", "markdown"]);
    assert!(output.status.success());
    let text = stdout_text(&output);
    for section in [
        "# ChangeSet",
        "## Change Summary",
        "## Files",
        "## Symbols",
        "## Modules",
        "## Diagnostics",
    ] {
        assert!(text.contains(section), "markdown must contain {section}");
    }
}

#[test]
fn changes_from_to_revision_range_and_determinism() {
    let dir = temp_git_project("range");
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-m", "change"]);
    let args = [
        "changes",
        dir.to_str().unwrap(),
        "--from",
        "HEAD~1",
        "--to",
        "HEAD",
        "--format",
        "json",
    ];
    let first = prism(&args);
    assert!(first.status.success());
    let second = prism(&args);
    // 确定性（§81）：同一状态连续两次输出一致。
    assert_eq!(stdout_text(&first), stdout_text(&second));
    let value: serde_json::Value = serde_json::from_str(&stdout_text(&first)).unwrap();
    assert_eq!(value["scope"], "revision_range");
    assert_eq!(value["base"].as_str().unwrap().len(), 40);
}

#[test]
fn changes_non_git_exit_6_and_missing_path_exit_3() {
    let empty = std::env::temp_dir().join(format!("prism-cli-nogit-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&empty);
    std::fs::create_dir_all(&empty).unwrap();
    let output = prism(&["changes", empty.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(6), "non-git must exit 6");
    assert!(String::from_utf8_lossy(&output.stderr).contains("not a git repository"));

    let missing = empty.join("definitely-missing");
    let output = prism(&["changes", missing.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(3), "missing path must exit 3");
    let _ = std::fs::remove_dir_all(&empty);
}

#[test]
fn changes_invalid_revision_exit_3() {
    let dir = temp_git_project("badrev");
    let output = prism(&[
        "changes",
        dir.to_str().unwrap(),
        "--from",
        "no-such-rev",
        "--to",
        "HEAD",
    ]);
    assert_eq!(
        output.status.code(),
        Some(3),
        "invalid revision must exit 3"
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("invalid base revision"));
}

#[test]
fn changes_staged_scope_reads_index() {
    let dir = temp_git_project("staged");
    git(&dir, &["add", "-A"]);
    let output = prism(&["changes", dir.to_str().unwrap(), "--staged"]);
    assert!(output.status.success());
    let text = stdout_text(&output);
    assert!(text.contains("Scope: staged"));
    assert!(text.contains("Target: index"));
}
