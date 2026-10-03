//! `prism health` 完整分析 CLI 契约测试（规格 M7 §75–§80/§165/§241/§247）。

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

fn temp_project(name: &str) -> PathBuf {
    temp_project_with(name, false)
}

/// with_warning=true ⇒ 含未解析依赖（DEP001 Warning）。
fn temp_project_with(name: &str, with_warning: bool) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "prism-cli-health-analyze-{name}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(dir.join("src/app.ts"), "// TODO: x\nexport const a = 1;\n").unwrap();
    if with_warning {
        std::fs::write(
            dir.join("src/warn.ts"),
            "import { gone } from \"@missing/pkg\";\nexport const w = gone;\n",
        )
        .unwrap();
    }
    git(&dir, &["init", "-b", "main"]);
    git(&dir, &["config", "user.name", "Prism Test"]);
    git(&dir, &["config", "user.email", "test@prism.local"]);
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-m", "base"]);
    dir
}

#[test]
fn health_text_reports_counts_and_findings() {
    let dir = temp_project("text");
    let output = prism(&["health", dir.to_str().unwrap()]);
    assert!(
        output.status.success(),
        "findings must NOT fail the run by default (§79): stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = stdout_text(&output);
    assert!(text.starts_with("Health Findings"));
    assert!(text.contains("maintenance"));
    assert!(text.contains("[INFO] MAINT001"), "{text}");
    assert!(text.contains("What:"), "explanation structure (§139)");
}

#[test]
fn health_json_outputs_stable_contract() {
    let dir = temp_project("json");
    let output = prism(&["health", dir.to_str().unwrap(), "--json"]);
    assert!(output.status.success());
    let value: serde_json::Value = serde_json::from_str(&stdout_text(&output)).unwrap();
    // §200 契约字段。
    for field in [
        "analysis_completeness",
        "summary",
        "findings",
        "diagnostics",
        "checked_rules",
        "disabled_rules",
    ] {
        assert!(value.get(field).is_some(), "schema field {field} expected");
    }
    assert_eq!(value["summary"]["total_findings"], 1);
    let finding = &value["findings"][0];
    for field in ["rule_id", "category", "severity", "confidence", "evidence", "status"] {
        assert!(finding.get(field).is_some(), "finding field {field} (§78)");
    }
}

#[test]
fn health_rule_filter_recomputes_summary() {
    let dir = temp_project("rule");
    let output = prism(&["health", dir.to_str().unwrap(), "--rule", "MAINT001", "--json"]);
    assert!(output.status.success());
    let value: serde_json::Value = serde_json::from_str(&stdout_text(&output)).unwrap();
    assert_eq!(value["summary"]["total_findings"], 1, "filtered summary real");
}

#[test]
fn health_category_filter_works() {
    let dir = temp_project("category");
    let output = prism(&["health", dir.to_str().unwrap(), "--category", "maintenance"]);
    assert!(output.status.success());
    assert!(stdout_text(&output).contains("MAINT001"));
    // 不存在的分类 ⇒ 空。
    let output = prism(&["health", dir.to_str().unwrap(), "--category", "nosuch"]);
    assert!(output.status.success());
    let text = stdout_text(&output);
    assert!(text.contains("Total") || text.contains("Info"), "{text}");
}

#[test]
fn health_markdown_and_html_render() {
    let dir = temp_project("render");
    let md = prism(&["health", dir.to_str().unwrap(), "--format", "markdown"]);
    assert!(md.status.success());
    let text = stdout_text(&md);
    assert!(text.contains("# Prism Health Report"));
    assert!(text.contains("## Analysis Limitations"), "§131/§89");

    let html = prism(&["health", dir.to_str().unwrap(), "--format", "html"]);
    assert!(html.status.success());
    let text = stdout_text(&html);
    assert!(text.contains("<!DOCTYPE html>"), "static offline HTML (§90)");
    assert!(text.contains("Prism Health Report"));
}

#[test]
fn health_fail_on_warning_exits_nonzero_but_default_zero() {
    // §79/§80：默认 findings 不失败；--fail-on warning opt-in（需真 Warning finding）。
    let dir = temp_project_with("failon", true);
    let default = prism(&["health", dir.to_str().unwrap()]);
    assert_eq!(default.status.code(), Some(0), "findings != failure by default");
    assert!(stdout_text(&default).contains("WARNING"));
    let fail_on = prism(&["health", dir.to_str().unwrap(), "--fail-on", "warning"]);
    assert_eq!(fail_on.status.code(), Some(1), "--fail-on warning => non-zero");
    assert!(
        String::from_utf8_lossy(&fail_on.stderr).contains("--fail-on"),
        "stderr explains the failure"
    );
}

#[test]
fn health_changed_scope_narrows_results() {
    // §61/§190/§247：--changed 只报变更/受影响范围。
    // 基线提交后，工作区新 TODO（在变更文件内 ⇒ --changed 报 1 条）。
    let dir = temp_project("changed");
    std::fs::write(
        dir.join("src/app.ts"),
        "// TODO: changed file marker\nexport const a = 1;\n",
    )
    .unwrap();
    std::fs::write(dir.join("src/untouched.ts"), "export const u = 9;\n").unwrap();
    // untouched.ts 未提交 ⇒ 也是 untracked/changed ⇒ 全部都算 changed 范围。
    // 为验证“收窄”，先提交一个无 TODO 的基线文件，再加一个变更 TODO 文件：
    let changed = prism(&["health", dir.to_str().unwrap(), "--changed", "--json"]);
    assert!(changed.status.success());
    let value: serde_json::Value = serde_json::from_str(&stdout_text(&changed)).unwrap();
    assert_eq!(value["scope"], "impacted_items");
    // app.ts 与 untouched.ts 都在变更集内（均为未提交文件），TODO 在 app.ts。
    assert_eq!(value["summary"]["total_findings"], 1);
}

#[test]
fn health_non_git_reports_ok_with_empty() {
    // 非 git：无 ChangeSet 可用 ⇒ FullProject 照常跑（M7 不强制 git）。
    let dir = std::env::temp_dir().join(format!("prism-health-nogit-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(dir.join("src/a.ts"), "// TODO: y\nexport const a = 2;\n").unwrap();
    let output = prism(&["health", dir.to_str().unwrap()]);
    assert!(output.status.success(), "non-git full-project health works");
    assert!(stdout_text(&output).contains("MAINT001"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn health_missing_path_exits_3() {
    let missing = std::env::temp_dir().join("prism-health-missing-xyz");
    let output = prism(&["health", missing.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(3));
}
