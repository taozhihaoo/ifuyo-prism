//! `prism health rules` CLI 契约测试（规格 M7 §74/§129/§220/§221）。

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

fn temp_project(name: &str, config_json: Option<&str>) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "prism-cli-health-{name}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    if let Some(config) = config_json {
        std::fs::write(dir.join("prism.config.json"), config).unwrap();
    }
    dir
}

#[test]
fn rules_lists_builtin_rules_with_state() {
    let dir = temp_project("plain", None);
    let output = prism(&["health", dir.to_str().unwrap(), "rules"]);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = stdout_text(&output);
    assert!(text.contains("MAINT001"), "{text}");
    assert!(text.contains("Maintenance Marker"));
    assert!(text.contains("enabled"));
}

#[test]
fn rules_reflects_config_disable() {
    // §220/§221：配置禁用 ⇒ rules 显示最终 disabled 状态。
    let dir = temp_project(
        "disabled",
        Some(
            r#"{
  "schema_version": 1,
  "health": {
    "rules": { "MAINT001": { "enabled": false } }
  }
}"#,
        ),
    );
    let output = prism(&["health", dir.to_str().unwrap(), "rules"]);
    assert!(output.status.success());
    let text = stdout_text(&output);
    assert!(
        text.contains("disabled"),
        "MAINT001 must show disabled: {text}"
    );
}

#[test]
fn rules_json_outputs_stable_contract() {
    let dir = temp_project("json", None);
    let output = prism(&["health", dir.to_str().unwrap(), "rules", "--json"]);
    assert!(output.status.success());
    let value: serde_json::Value = serde_json::from_str(&stdout_text(&output)).unwrap();
    let entries = value.as_array().expect("rules array");
    assert!(!entries.is_empty());
    let maint = entries
        .iter()
        .find(|entry| entry["id"] == "MAINT001")
        .expect("MAINT001 expected");
    assert_eq!(maint["name"], "Maintenance Marker");
    assert_eq!(maint["category"], "maintenance");
    assert_eq!(maint["enabled"], true);
}

#[test]
fn invalid_config_is_a_hard_error_not_defaults() {
    // §72：配置解析失败必须显式报错（不能静默回落默认——那会掩盖拼写错误）。
    let dir = temp_project(
        "invalid",
        Some(
            r#"{
  "schema_version": 1,
  "health": { "rules": { "MAINT001": { "threshhold": 5 } } }
}"#,
        ),
    );
    let output = prism(&["health", dir.to_str().unwrap(), "rules"]);
    assert_eq!(output.status.code(), Some(1), "invalid config must fail");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("config invalid"),
        "stderr must explain: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn missing_path_exits_3() {
    let missing = std::env::temp_dir().join("prism-health-missing-path-xyz");
    let _ = std::fs::remove_dir_all(&missing);
    let output = prism(&["health", missing.to_str().unwrap(), "rules"]);
    assert_eq!(output.status.code(), Some(3));
}
