//! `prism context` CLI 契约（规格 M10a §283/§347/§348/§138/§200）。

use std::process::Command;

fn prism(args: &[&str]) -> std::process::Output {
    let bin = env!("CARGO_BIN_EXE_prism");
    Command::new(bin).args(args).output().expect("prism binary")
}

fn stdout_text(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn temp_project(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "prism-cli-ctx-{name}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(dir.join("package.json"), r#"{ "name": "demo" }"#).unwrap();
    std::fs::write(
        dir.join("src/BattleManager.ts"),
        "class BattleManager {\n  resolve(i) { return i; }\n}\n",
    )
    .unwrap();
    std::fs::write(dir.join("README.md"), "# demo\n\n## Build\nnpm run build\n").unwrap();
    dir
}

#[test]
fn context_json_is_machine_readable_with_manifest() {
    let dir = temp_project("json");
    // §81：任务式请求。
    let output = prism(&[
        "context",
        dir.to_str().unwrap(),
        "--task",
        "重构 BattleManager",
        "--json",
    ]);
    assert!(output.status.success());
    // §348：stdout 只有 JSON。
    let value: serde_json::Value =
        serde_json::from_str(&stdout_text(&output)).expect("pure JSON pack");
    let manifest = &value["manifest"];
    assert_eq!(manifest["schema_version"], 1);
    assert!(manifest["task"].is_string());
    assert!(manifest["summary"].is_object());
    assert!(manifest["budget"]["limit_tokens_estimate"].is_number());
    // §16/§22：目标解析为 Resolved 且 item 带理由。
    assert_eq!(manifest["target"]["resolution"]["kind"], "resolved");
    let items = value["items"].as_array().unwrap();
    assert!(!items.is_empty());
    let target = items
        .iter()
        .find(|item| {
            item["reasons"]
                .as_array()
                .unwrap()
                .iter()
                .any(|reason| reason == "explicit_task_target")
        })
        .expect("target item present");
    assert_eq!(target["priority"], "critical");
    assert!(target["content"].is_string(), "materialized content");
    // §218：content hash（secret-safe hashing 存在）。
    assert!(target["content_hash"].is_string());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn context_selection_is_reproducible() {
    // §138/§220：同状态同任务 ⇒ 相同 selected ids 与预算消耗
    // （manifest.id/captured 时间戳天然不同，不计入）。
    let dir = temp_project("repro");
    let run = || {
        let output = prism(&[
            "context",
            dir.to_str().unwrap(),
            "--task",
            "重构 BattleManager",
            "--json",
        ]);
        let value: serde_json::Value = serde_json::from_str(&stdout_text(&output)).unwrap();
        let ids: Vec<String> = value["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|item| item["id"].as_str().unwrap().to_string())
            .collect();
        let bytes = value["manifest"]["budget"]["used_bytes"].as_u64().unwrap();
        (ids, bytes)
    };
    let first = run();
    let second = run();
    assert_eq!(first, second, "§139 deterministic selection must reproduce");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn context_text_mode_lists_groups_and_limitations() {
    let dir = temp_project("text");
    let output = prism(&["context", dir.to_str().unwrap(), "--task", "做得更好"]);
    assert!(output.status.success());
    let stdout = stdout_text(&output);
    assert!(stdout.contains("Context Pack"));
    assert!(stdout.contains("[project]"));
    // §18：Unknown 任务 ⇒ 明确声明 task specificity limited。
    assert!(stdout.contains("Task specificity limited"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn context_zero_budget_is_invalid_input() {
    // §200：零预算 ⇒ validation error（exit 3），不是随机行为。
    let dir = temp_project("zero");
    let output = prism(&["context", dir.to_str().unwrap(), "--max-tokens", "0"]);
    assert_eq!(output.status.code(), Some(3));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn context_missing_path_is_invalid_input() {
    let output = prism(&["context", "Z:/definitely/missing", "--json"]);
    assert_eq!(output.status.code(), Some(3));
}

// §285/§350：dry-run 输出 selection plan（不物化全文）。
#[test]
fn context_dry_run_shows_plan_without_content() {
    let dir = temp_project("dryrun");
    let output = prism(&[
        "context",
        dir.to_str().unwrap(),
        "--task",
        "重构 BattleManager",
        "--dry-run",
    ]);
    assert!(output.status.success());
    let stdout = stdout_text(&output);
    assert!(stdout.contains("Selection plan"));
    assert!(stdout.contains("explicit_task_target"));
    // plan 不含物化正文（正文以 class 开头）。
    assert!(
        !stdout.contains("class BattleManager"),
        "dry-run must not materialize"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

// §287：explain 显示排除理由。
#[test]
fn context_explain_shows_trace() {
    let dir = temp_project("explain");
    let output = prism(&["context", dir.to_str().unwrap(), "--explain"]);
    assert!(output.status.success());
    assert!(stdout_text(&output).contains("Selection trace"));
    let _ = std::fs::remove_dir_all(&dir);
}

// §284：--check-health 只输出 health。
#[test]
fn context_check_health_outputs_health_json() {
    let dir = temp_project("health");
    let output = prism(&["context", dir.to_str().unwrap(), "--check-health"]);
    assert!(output.status.success());
    let value: serde_json::Value =
        serde_json::from_str(&stdout_text(&output)).expect("health JSON");
    assert!(value["status"].is_string());
    assert!(value["findings"].is_array());
    let _ = std::fs::remove_dir_all(&dir);
}

// §349：markdown 输出结构。
#[test]
fn context_markdown_renders_pack() {
    let dir = temp_project("md");
    let output = prism(&[
        "context",
        dir.to_str().unwrap(),
        "--markdown",
        "--task",
        "重构 BattleManager",
    ]);
    assert!(output.status.success());
    let stdout = stdout_text(&output);
    assert!(stdout.starts_with("# Prism Context Pack"));
    assert!(stdout.contains("## Task"));
    assert!(stdout.contains("## Context Health"));
    let _ = std::fs::remove_dir_all(&dir);
}

// §290/§291：profile 校验。
#[test]
fn context_unknown_profile_rejected() {
    let dir = temp_project("profile");
    let output = prism(&["context", dir.to_str().unwrap(), "--profile", "everything"]);
    assert_eq!(output.status.code(), Some(3));
    let _ = std::fs::remove_dir_all(&dir);
}

// §205-§207 选择/截断集成：小预算 ⇒ 截断事实。
#[test]
fn context_small_budget_truncates_with_facts() {
    let dir = temp_project("budget");
    let output = prism(&[
        "context",
        dir.to_str().unwrap(),
        "--task",
        "重构 BattleManager",
        "--json",
        "--max-tokens",
        "128",
    ]);
    assert!(
        output.status.success(),
        "small budget must still generate (§199)"
    );
    let value: serde_json::Value = serde_json::from_str(&stdout_text(&output)).unwrap();
    // §199：极小预算 ⇒ Critical 保留 + limitations 声明，不崩溃。
    assert!(!value["manifest"]["limitations"]
        .as_array()
        .unwrap()
        .is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}
