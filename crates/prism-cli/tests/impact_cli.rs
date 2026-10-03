//! `prism impact` CLI 契约测试（规格 M6 §87–§90、§113）。

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

const LIB: &str = "export function pay(amount: number): number {\n  return amount * 2;\n}\n";
const CHECKOUT: &str = "import { pay } from \"./lib\";\nexport function checkout(amount: number): number {\n  return pay(amount) + 1;\n}\n";
const TEST: &str = "import { checkout } from \"./checkout\";\nexport function testCheckout(): number {\n  return checkout(1);\n}\n";

/// lib→checkout→test 链 + 一处未提交修改。
fn temp_impact_project(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "prism-cli-impact-{name}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("lib.ts"), LIB).unwrap();
    std::fs::write(dir.join("checkout.ts"), CHECKOUT).unwrap();
    std::fs::write(dir.join("checkout.test.ts"), TEST).unwrap();
    git(&dir, &["init", "-b", "main"]);
    git(&dir, &["config", "user.name", "Prism Test"]);
    git(&dir, &["config", "user.email", "test@prism.local"]);
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-m", "base"]);
    std::fs::write(dir.join("lib.ts"), &LIB.replace("amount * 2", "amount * 9")).unwrap();
    dir
}

#[test]
fn impact_text_reports_affected_and_limitations() {
    let dir = temp_impact_project("text");
    let output = prism(&["impact", dir.to_str().unwrap()]);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = stdout_text(&output);
    assert!(text.contains("Changed: 1 files, 1 symbols, 1 modules"));
    assert!(text.contains("Potentially affected"));
    assert!(text.contains("caller"), "caller classification expected: {text}");
    assert!(text.contains("test"), "test candidate expected: {text}");
    // §74/§131：局限不隐藏。
    assert!(text.contains("Analysis limitations"));
}

#[test]
fn impact_json_outputs_stable_contract() {
    let dir = temp_impact_project("json");
    let output = prism(&["impact", dir.to_str().unwrap(), "--format", "json"]);
    assert!(output.status.success());
    let value: serde_json::Value =
        serde_json::from_str(&stdout_text(&output)).expect("valid JSON expected");
    assert!(value["id"].as_str().unwrap().starts_with("impact:"));
    assert_eq!(value["schema_version"], 1);
    assert_eq!(value["analysis_completeness"], "complete");
    assert!(value["changed_items"].as_array().unwrap().len() >= 1);
    assert!(value["impacted_items"].as_array().unwrap().len() >= 1);
    // §126 的 schema 字段齐备。
    for field in ["impact_paths", "summary", "diagnostics", "risk_signals"] {
        assert!(value.get(field).is_some(), "schema field {field} expected");
    }
}

#[test]
fn impact_is_deterministic_across_runs() {
    let dir = temp_impact_project("determinism");
    let args = ["impact", dir.to_str().unwrap(), "--format", "json"];
    let first = prism(&args);
    let second = prism(&args);
    assert!(first.status.success() && second.status.success());
    assert_eq!(stdout_text(&first), stdout_text(&second), "§113 determinism");
}

#[test]
fn impact_truncated_is_still_success() {
    // §90：valid but truncated ≠ failed。
    let dir = temp_impact_project("truncated");
    let output = prism(&[
        "impact",
        dir.to_str().unwrap(),
        "--max-depth",
        "1",
        "--format",
        "json",
    ]);
    assert!(
        output.status.success(),
        "truncated analysis must not be a failure exit"
    );
    let value: serde_json::Value = serde_json::from_str(&stdout_text(&output)).unwrap();
    assert_eq!(value["analysis_completeness"], "truncated");
    assert_eq!(value["analysis_incomplete"], true);
}

#[test]
fn impact_non_git_exit_6_and_missing_path_exit_3() {
    let empty = std::env::temp_dir().join(format!("prism-cli-impact-nogit-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&empty);
    std::fs::create_dir_all(&empty).unwrap();
    let output = prism(&["impact", empty.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(6), "non-git must exit 6");

    let missing = empty.join("definitely-missing");
    let output = prism(&["impact", missing.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(3), "missing path must exit 3");
    let _ = std::fs::remove_dir_all(&empty);
}

#[test]
fn impact_revision_range_matches_changes_scopes() {
    let dir = temp_impact_project("range");
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-m", "change"]);
    // 清空工作区后再比较两个提交（§87 --from/--to）。
    let output = prism(&[
        "impact",
        dir.to_str().unwrap(),
        "--from",
        "HEAD~1",
        "--to",
        "HEAD",
    ]);
    assert!(output.status.success());
    let text = stdout_text(&output);
    assert!(text.contains("Scope: revision_range"));
    assert!(text.contains("Changed: 1 files"));
}
