//! CLI 集成测试：直接运行编译出的 `prism` 二进制验证对外契约。
//!
//! 使用 cargo 内建的 `CARGO_BIN_EXE_prism` 机制，不引入额外依赖。

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

fn stderr_text(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_string()
}

#[test]
fn version_reports_real_version() {
    let output = prism(&["--version"]);
    assert!(output.status.success());
    let text = stdout_text(&output);
    assert!(
        text.contains("prism 0.1.0"),
        "--version must print real version, got: {text}"
    );
}

#[test]
fn help_lists_all_milestone_commands() {
    let output = prism(&["--help"]);
    assert!(output.status.success());
    let text = stdout_text(&output);
    for name in [
        "info", "scan", "inspect", "changes", "health", "verify", "env", "context", "report",
    ] {
        assert!(text.contains(name), "--help must mention {name}");
    }
}

#[test]
fn no_command_prints_help_and_succeeds() {
    let output = prism(&[]);
    assert!(output.status.success());
}

#[test]
fn info_outputs_real_app_info_text() {
    let output = prism(&["info"]);
    assert!(output.status.success());
    let text = stdout_text(&output);
    assert!(text.contains("Prism"));
    assert!(text.contains("0.1.0"));
    assert!(text.contains("os:"), "info must carry real platform facts");
}

#[test]
fn info_json_output_is_valid_json() {
    let output = prism(&["info", "--format", "json"]);
    assert!(output.status.success());
    let value: serde_json::Value =
        serde_json::from_str(&stdout_text(&output)).expect("info --format json must be valid JSON");
    assert_eq!(value["name"], "Prism");
    assert_eq!(value["version"], "0.1.0");
    assert!(value.get("os").is_some());
}

#[test]
fn future_commands_clearly_report_not_implemented() {
    for args in [
        vec!["env", "."],
        vec!["context", "."],
        vec!["report", "."],
    ] {
        let output = prism(&args);
        assert!(!output.status.success(), "{args:?} must not fake success");
        assert!(
            stderr_text(&output).contains("Not implemented in current milestone."),
            "{args:?} must state clearly it is not implemented"
        );
    }
}

#[test]
fn usage_error_exits_with_code_2() {
    let output = prism(&["definitely-not-a-command"]);
    assert_eq!(
        output.status.code(),
        Some(2),
        "clap usage errors exit with 2"
    );
}

// ---------- prism scan（M1） ----------

fn temp_project(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("prism-cli-scan-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(dir.join("src/main.ts"), "let x = 1;").unwrap();
    std::fs::write(dir.join("README.md"), "doc").unwrap();
    std::fs::write(dir.join("package.json"), "{}").unwrap();
    std::fs::create_dir_all(dir.join("node_modules/pkg")).unwrap();
    std::fs::write(dir.join("node_modules/pkg/index.js"), "x").unwrap();
    dir
}

#[test]
fn scan_outputs_human_readable_summary() {
    let dir = temp_project("text");
    let output = prism(&["scan", dir.to_str().unwrap()]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "clean scan must exit 0, stderr: {}",
        stderr_text(&output)
    );
    let text = stdout_text(&output);
    assert!(text.contains("Project:"), "{text}");
    assert!(text.contains("Files: 3"), "{text}");
    // node_modules 与其子项 pkg 被忽略（walker 不进入被忽略目录）。
    assert!(text.contains("Ignored: 1"), "{text}");
    assert!(text.contains("TypeScript files: 1"), "{text}");
    assert!(text.contains("Node.js (high"), "{text}");
    assert!(text.contains("src/main.ts (high"), "{text}");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn scan_json_outputs_stable_discovery_contract() {
    let dir = temp_project("json");
    let output = prism(&["scan", dir.to_str().unwrap(), "--format", "json"]);
    assert_eq!(output.status.code(), Some(0));
    let value: serde_json::Value =
        serde_json::from_str(&stdout_text(&output)).expect("must be valid JSON");
    assert_eq!(value["schema_version"], 1);
    assert_eq!(value["scan"]["status"], "completed");
    assert_eq!(value["statistics"]["total_files"], 3);
    assert_eq!(value["files"].as_array().unwrap().len(), 3);
    // JSON 中的文件路径必须是项目相对路径（纲领 §11/§51）。
    let paths: Vec<&str> = value["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["relative_path"].as_str().unwrap())
        .collect();
    for path in &paths {
        assert!(
            !path.starts_with('/') && !path.starts_with("E:"),
            "absolute path leaked: {path}"
        );
    }
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn scan_missing_path_is_invalid_input_exit_3() {
    let missing = std::env::temp_dir().join("prism-cli-scan-missing-xyz");
    let output = prism(&["scan", missing.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(3), "invalid input must exit 3");
}

// ---------- prism inspect（M2） ----------

fn temp_code_project(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("prism-cli-inspect-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(
        dir.join("src/app.ts"),
        "export class App {\n    run(): void {}\n}\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("src/svc.py"),
        "class Svc:\n    def go(self):\n        pass\n",
    )
    .unwrap();
    std::fs::write(dir.join("src/legacy.rs"), "struct Old;\n").unwrap();
    dir
}

#[test]
fn inspect_outputs_text_summary_with_symbols() {
    let dir = temp_code_project("text");
    let output = prism(&["inspect", dir.to_str().unwrap()]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "clean inspect must exit 0, stderr: {}",
        stderr_text(&output)
    );
    let text = stdout_text(&output);
    assert!(text.contains("Code Understanding"), "{text}");
    assert!(text.contains("Parsed: 2"), "{text}");
    assert!(text.contains("Unsupported: 1"), "{text}");
    assert!(text.contains("class: 2"), "{text}");
    assert!(text.contains("src/app.ts"), "{text}");
    assert!(text.contains("App (class)"), "{text}");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn inspect_json_outputs_stable_code_model_contract() {
    let dir = temp_code_project("json");
    let output = prism(&["inspect", dir.to_str().unwrap(), "--format", "json"]);
    assert_eq!(output.status.code(), Some(0));
    let value: serde_json::Value =
        serde_json::from_str(&stdout_text(&output)).expect("must be valid JSON");
    assert_eq!(value["schema_version"], 1);
    assert_eq!(value["summary"]["files_parsed"], 2);
    assert_eq!(value["summary"]["files_unsupported"], 1);
    // 符号必须带稳定 ID 与限定名（契约字段）。
    let symbols = value["parse_results"].as_array().unwrap()[0]["symbols"]
        .as_array()
        .unwrap();
    assert!(symbols
        .iter()
        .all(|s| s["id"].as_str().unwrap().starts_with("symbol:")));
    assert!(symbols.iter().all(|s| s.get("qualified_name").is_some()));
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn inspect_missing_path_is_invalid_input_exit_3() {
    let missing = std::env::temp_dir().join("prism-cli-inspect-missing-xyz");
    let output = prism(&["inspect", missing.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(3), "invalid input must exit 3");
}

// ---------- prism graph（M3） ----------

#[test]
fn graph_outputs_text_summary_with_metrics() {
    let dir = temp_code_project("graph-text");
    let output = prism(&["graph", dir.to_str().unwrap()]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "graph must exit 0, stderr: {}",
        stderr_text(&output)
    );
    let text = stdout_text(&output);
    assert!(text.contains("Dependency & Architecture"), "{text}");
    assert!(text.contains("Nodes:"), "{text}");
    assert!(text.contains("Cycles:"), "{text}");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn graph_json_outputs_stable_contract() {
    let dir = temp_code_project("graph-json");
    let output = prism(&["graph", dir.to_str().unwrap(), "--format", "json"]);
    assert_eq!(output.status.code(), Some(0));
    let value: serde_json::Value =
        serde_json::from_str(&stdout_text(&output)).expect("must be valid JSON");
    assert_eq!(value["schema_version"], 1);
    assert!(value["nodes"].as_array().is_some());
    assert!(value["edges"].as_array().is_some());
    assert!(value["cycles"].as_array().is_some());
    assert!(value["metrics"]["node_count"].is_u64());
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn graph_module_kind_filters_view() {
    let dir = temp_code_project("graph-kind");
    let output = prism(&["graph", dir.to_str().unwrap(), "--kind", "module"]);
    assert_eq!(output.status.code(), Some(0));
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn graph_missing_path_is_invalid_input_exit_3() {
    let missing = std::env::temp_dir().join("prism-cli-graph-missing-xyz");
    let output = prism(&["graph", missing.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(3));
}
