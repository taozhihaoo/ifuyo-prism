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
    let output = prism(&["context", dir.to_str().unwrap(), "--task", "重构 BattleManager", "--json"]);
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
        .find(|item| item["reasons"].as_array().unwrap()
            .iter().any(|reason| reason == "explicit_task_target"))
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
        let output = prism(&["context", dir.to_str().unwrap(), "--task", "重构 BattleManager", "--json"]);
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
