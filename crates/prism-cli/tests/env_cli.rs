//! `prism env` CLI 契约（规格 M9a §240–§242/§295–§297 + §205 fixtures）。

use std::process::Command;

fn prism(args: &[&str]) -> std::process::Output {
    let bin = env!("CARGO_BIN_EXE_prism");
    Command::new(bin).args(args).output().expect("prism binary")
}

fn stdout_text(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn stderr_text(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_string()
}

fn temp_project(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "prism-cli-env-{name}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn env_json_is_machine_readable_and_complete() {
    // §205 Node fixture。
    let dir = temp_project("node");
    std::fs::write(dir.join("package.json"), r#"{ "name": "demo" }"#).unwrap();

    let output = prism(&["env", dir.to_str().unwrap(), "--json"]);
    assert!(output.status.success(), "satisfied/unknown must exit 0 (§242)");
    // §295：机器模式 stdout 只有数据。
    let value: serde_json::Value =
        serde_json::from_str(&stdout_text(&output)).expect("stdout must be pure JSON");
    assert_eq!(value["schema_version"], 1);
    assert!(value["host"]["os"].is_string());
    assert!(value["host"]["architecture"].is_string());
    assert!(value["host"]["username"].is_null(), "§14 minimal identity");
    let tools = value["tools"].as_array().unwrap();
    let names: Vec<&str> = tools
        .iter()
        .map(|tool| tool["name"].as_str().unwrap())
        .collect();
    assert!(names.contains(&"git") && names.contains(&"node"), "node profile: {names:?}");
    // §54：变量无值字段。
    for variable in value["variables"].as_array().unwrap() {
        assert!(variable.get("value").is_none());
        assert!(variable["possibly_sensitive"].is_boolean());
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn env_text_mode_matches_cli_contract() {
    // §296：人类模式带状态标签；stderr 无进度噪音。
    let dir = temp_project("text");
    std::fs::write(dir.join("package.json"), "{}").unwrap();

    let output = prism(&["env", dir.to_str().unwrap()]);
    assert!(output.status.success());
    let stdout = stdout_text(&output);
    assert!(stdout.contains("Environment"));
    assert!(stdout.contains("OS"));
    assert!(stdout.contains("Tools"));
    assert!(stdout.contains("Snapshot"), "snapshot id + captured_at 末尾呈现");
    assert!(stdout.contains("does not guarantee reproducibility"), "§291");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn env_all_tools_probes_beyond_profile() {
    // §270 heavy：显式开启后覆盖 allowlist 生态（dotnet/python 出现在列表）。
    let dir = temp_project("all");
    std::fs::write(dir.join("package.json"), "{}").unwrap();

    let output = prism(&["env", dir.to_str().unwrap(), "--json", "--all-tools"]);
    assert!(output.status.success());
    let value: serde_json::Value = serde_json::from_str(&stdout_text(&output)).unwrap();
    let names: Vec<&str> = value["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|tool| tool["name"].as_str().unwrap())
        .collect();
    assert!(names.contains(&"dotnet") && names.contains(&"python"), "{names:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn env_missing_path_is_invalid_input() {
    let output = prism(&["env", "Z:/definitely/missing/path", "--json"]);
    assert_eq!(output.status.code(), Some(3), "路径不存在 ⇒ 3");
    assert!(stderr_text(&output).contains("path does not exist"));
}

#[test]
fn env_unsatisfied_requirement_fails_and_strict_treats_unknown_as_failure() {
    // §240：required requirement Unsatisfied ⇒ 非零。
    // PATH 清空 ⇒ packageManager pin 的 pnpm 经 lookup 确认缺失（§105：
    // "Required package manager unavailable"，不是 npm is good enough）。
    let dir = temp_project("unsatisfied");
    std::fs::write(
        dir.join("package.json"),
        r#"{ "packageManager": "pnpm@10.17.0" }"#,
    )
    .unwrap();

    let bin = env!("CARGO_BIN_EXE_prism");
    let output = Command::new(bin)
        .args(["env", dir.to_str().unwrap(), "--json"])
        .env("PATH", "")
        .output()
        .expect("run prism");
    assert_eq!(output.status.code(), Some(1), "§240 unsatisfied ⇒ 1");
    let value: serde_json::Value = serde_json::from_str(&stdout_text(&output)).unwrap();
    let matches = value["matches"].as_array().unwrap();
    assert!(!matches.is_empty(), "packageManager requirement must be matched");
    assert_eq!(matches[0]["result"], "unsatisfied", "lookup confirmed absence (§105)");

    // §241：--strict 下 Unknown 也算失败（unit 级 Unknown 语义见 requirement.rs）。
    let strict = Command::new(bin)
        .args(["env", dir.to_str().unwrap(), "--strict"])
        .env("PATH", "")
        .output()
        .expect("run prism");
    assert_eq!(strict.status.code(), Some(1));
    let _ = std::fs::remove_dir_all(&dir);
}
