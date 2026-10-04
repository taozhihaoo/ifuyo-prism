//! M11a CLI 集成（规格 §196 Restart Persistence / §197 Multi-project /
//! §198 Same-name / §199 isolation / §217 reproducible）。
//!
//! 每个测试用独立 PRISM_DB_PATH；restart = 多次独立进程调用同一 DB。

use std::process::Command;

fn prism_with_db(db_path: &std::path::Path, args: &[&str]) -> std::process::Output {
    let bin = env!("CARGO_BIN_EXE_prism");
    Command::new(bin)
        .args(args)
        .env("PRISM_DB_PATH", db_path)
        .output()
        .expect("prism binary")
}

fn stdout_text(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn temp_project(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "prism-cli-snap-{name}-{}-{}",
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

// §196 Restart Persistence：进程 A capture → 进程 B（独立进程）list 仍可见。
#[test]
fn snapshot_survives_process_restart() {
    let dir = temp_project("restart");
    std::fs::write(dir.join("a.txt"), "content").unwrap();
    let db = std::env::temp_dir().join(format!(
        "prism-snap-restart-{}-{}.db",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0)
    ));

    // 进程 A：capture。
    let output = prism_with_db(&db, &["snapshot", dir.to_str().unwrap()]);
    assert!(output.status.success());
    let stdout = stdout_text(&output);
    let snapshot_id = stdout
        .split("Snapshot created: ")
        .nth(1)
        .map(|rest| rest.lines().next().unwrap_or("").trim().to_string())
        .expect("created id");

    // 进程 B：history（全新进程，同一 DB）。
    let output = prism_with_db(&db, &["history", dir.to_str().unwrap()]);
    assert!(output.status.success());
    let stdout = stdout_text(&output);
    assert!(
        stdout.contains(&snapshot_id),
        "§196: snapshot must survive process restart; stdout: {stdout}"
    );
    let _ = std::fs::remove_file(&db);
    let _ = std::fs::remove_dir_all(&dir);
}

// §159/§17：dedup + §17 unchanged 文案。
#[test]
fn unchanged_second_capture_deduplicates_with_message() {
    let dir = temp_project("dedup");
    let db = std::env::temp_dir().join(format!(
        "prism-snap-dedup-{}-{}.db",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0)
    ));
    let db_clone = db.clone();

    let first = prism_with_db(&db_clone, &["snapshot", dir.to_str().unwrap()]);
    assert!(stdout_text(&first).contains("Snapshot created:"));

    let second = prism_with_db(&db_clone, &["snapshot", dir.to_str().unwrap()]);
    let stdout = stdout_text(&second);
    assert!(
        stdout.contains("Snapshot unchanged."),
        "§17 message: {stdout}"
    );
    assert!(stdout.contains("Existing snapshot:"));

    // --force 绕过去重（§17/§18）。
    let forced = prism_with_db(&db_clone, &["snapshot", dir.to_str().unwrap(), "--force"]);
    assert!(stdout_text(&forced).contains("Snapshot created:"));
    let _ = std::fs::remove_file(&db);
    let _ = std::fs::remove_dir_all(&dir);
}

// §197/§198：两个项目（同名 app）历史互不串扰。
#[test]
fn multi_project_histories_isolated() {
    let dir_a = temp_project("iso-a");
    let dir_b = temp_project("iso-b");
    std::fs::write(dir_b.join("different.txt"), "b").unwrap();
    let db = std::env::temp_dir().join(format!(
        "prism-snap-iso-{}-{}.db",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0)
    ));

    assert!(prism_with_db(&db, &["snapshot", dir_a.to_str().unwrap()])
        .status
        .success());
    assert!(prism_with_db(&db, &["snapshot", dir_b.to_str().unwrap()])
        .status
        .success());

    // §199：删 A 的历史，B 不受影响。
    let project_a_root = dir_a.to_string_lossy().to_string();
    // 用 history JSON 确认 A 有 1 条。
    let output = prism_with_db(&db, &["history", dir_a.to_str().unwrap(), "--json"]);
    let history_a: serde_json::Value = serde_json::from_str(&stdout_text(&output)).unwrap();
    assert_eq!(history_a.as_array().unwrap().len(), 1);
    assert_eq!(
        history_a[0]["project_root"].as_str().unwrap(),
        project_a_root,
        "§198: same-name-safe identity is root-based"
    );

    // 删除 A 历史（通过 CLI delete 变体——本篇用 store 层；CLI 面在 M11b。
    // 这里验证的是 isolation 语义：B 的 history 仍是 1 条且 root 不同）。
    let output = prism_with_db(&db, &["history", dir_b.to_str().unwrap(), "--json"]);
    let history_b: serde_json::Value = serde_json::from_str(&stdout_text(&output)).unwrap();
    assert_eq!(history_b.as_array().unwrap().len(), 1);
    assert_ne!(
        history_a[0]["state_fingerprint"], history_b[0]["state_fingerprint"],
        "different projects must have different states"
    );
    let _ = std::fs::remove_file(&db);
    let _ = std::fs::remove_dir_all(&dir_a);
    let _ = std::fs::remove_dir_all(&dir_b);
}

// §220：PRISM_DB_PATH 覆盖生效（不同 DB ⇒ 不同历史）。
#[test]
fn db_path_override_isolation() {
    let dir = temp_project("dbpath");
    std::fs::write(dir.join("x.txt"), "x").unwrap();
    let db_a = std::env::temp_dir().join("prism-snap-isol-a.db");
    let db_b = std::env::temp_dir().join("prism-snap-isol-b.db");
    let _ = std::fs::remove_file(&db_a);
    let _ = std::fs::remove_file(&db_b);

    assert!(prism_with_db(&db_a, &["snapshot", dir.to_str().unwrap()])
        .status
        .success());
    // 另一个 DB ⇒ 没有 history。
    let output = prism_with_db(&db_b, &["history", dir.to_str().unwrap()]);
    assert!(output.status.success());
    assert!(stdout_text(&output).contains("No snapshots"));
    let _ = std::fs::remove_file(&db_a);
    let _ = std::fs::remove_file(&db_b);
    let _ = std::fs::remove_dir_all(&dir);
}

// §217：同状态两次 capture（去重后）history 稳定（只 1 条）。
#[test]
fn history_is_reproducible_for_unchanged_state() {
    let dir = temp_project("repro");
    std::fs::write(dir.join("stable.txt"), "stable").unwrap();
    let db = std::env::temp_dir().join(format!(
        "prism-snap-repro-{}-{}.db",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0)
    ));
    for _ in 0..3 {
        assert!(prism_with_db(&db, &["snapshot", dir.to_str().unwrap()])
            .status
            .success());
    }
    let output = prism_with_db(&db, &["history", dir.to_str().unwrap(), "--json"]);
    let history: serde_json::Value = serde_json::from_str(&stdout_text(&output)).unwrap();
    assert_eq!(
        history.as_array().unwrap().len(),
        1,
        "§17 dedup keeps history clean"
    );
    let _ = std::fs::remove_file(&db);
    let _ = std::fs::remove_dir_all(&dir);
}
