//! M1 Project Discovery 集成测试（纲领 §65 最低测试矩阵）。
//!
//! 固定 fixture：`tests/fixtures/fixture-multi-language`；
//! 忽略 / .gitignore / 大文件 / 空项目 / 取消等文件系统场景在运行期
//! 于临时目录构造（git 无法提交 node_modules/target 等被忽略目录）。

mod support;

use prism_core::task::{CancellationToken, TaskState};
use prism_project::{DiscoveryService, ProjectDiscovery, ScanOptions};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use support::temp_dir;

fn fixture_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(name)
        .canonicalize()
        .expect("fixture must exist")
}

fn scan_path(path: &Path) -> ProjectDiscovery {
    let service = DiscoveryService::new();
    service
        .discover(
            path,
            ScanOptions::default(),
            &CancellationToken::new(),
            |_| {},
        )
        .expect("scan must succeed")
}

#[test]
fn multi_language_fixture_full_discovery() {
    let discovery = scan_path(&fixture_path("fixture-multi-language"));

    // 项目身份（§87）。
    assert_eq!(discovery.project.name, "fixture-multi-language");
    assert!(discovery.project.root_path.is_absolute());

    // 文件与目录计数：9 个文件、5 个目录（src, src-tauri, src-tauri/src, tests）。
    assert_eq!(discovery.files.len(), 9);
    assert_eq!(discovery.directories.len(), 4);

    // 项目类型（§87：evidence 必须存在）。
    let types: Vec<&str> = discovery
        .project_types
        .iter()
        .map(|candidate| candidate.name.as_str())
        .collect();
    for expected in ["Tauri", "Rust", "Node.js", "TypeScript"] {
        assert!(types.contains(&expected), "missing {expected} in {types:?}");
    }
    let tauri = discovery
        .project_types
        .iter()
        .find(|candidate| candidate.name == "Tauri")
        .unwrap();
    assert_eq!(tauri.evidence.len(), 2);

    // 入口点（带 confidence，§87）。
    let entries: Vec<&str> = discovery
        .entry_points
        .iter()
        .map(|entry| entry.path.as_str())
        .collect();
    assert!(entries.contains(&"src/main.ts"), "entries: {entries:?}");
    assert!(
        entries.contains(&"src-tauri/src/main.rs"),
        "entries: {entries:?}"
    );

    // 配置识别。
    let configs: Vec<&str> = discovery
        .configurations
        .iter()
        .map(|config| config.path.as_str())
        .collect();
    for expected in [
        "README.md",
        "package.json",
        "tsconfig.json",
        "src-tauri/Cargo.toml",
        "src-tauri/tauri.conf.json",
    ] {
        assert!(
            configs.contains(&expected),
            "missing {expected} in {configs:?}"
        );
    }

    // 语言统计（数量降序中的稳定性）。
    let rust = discovery
        .languages
        .iter()
        .find(|entry| entry.language == prism_project::Language::Rust)
        .expect("rust must be counted");
    assert_eq!(rust.count, 1);
    let python = discovery
        .languages
        .iter()
        .find(|entry| entry.language == prism_project::Language::Python)
        .expect("python must be counted");
    assert_eq!(python.count, 1);

    // 分类：测试文件与文档。
    assert_eq!(discovery.statistics.test_files, 1);
    assert_eq!(discovery.statistics.documentation_files, 1);
    // package.json / tsconfig.json / Cargo.toml / tauri.conf.json（README 归 Documentation）。
    assert_eq!(discovery.statistics.config_files, 4);
    assert_eq!(discovery.scan.status, TaskState::Completed);
    assert_eq!(discovery.schema_version, prism_project::SCHEMA_VERSION);
}

#[test]
fn default_ignores_exclude_node_modules_and_target() {
    let root = temp_dir("ignore-tree");
    std::fs::create_dir_all(root.join("node_modules/left-pad")).unwrap();
    std::fs::create_dir_all(root.join("target/debug")).unwrap();
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(root.join("node_modules/left-pad/index.js"), "x").unwrap();
    std::fs::write(root.join("target/debug/app.exe"), "x").unwrap();
    std::fs::write(root.join("src/main.py"), "x").unwrap();

    let discovery = scan_path(&root);
    assert_eq!(discovery.files.len(), 1);
    assert_eq!(discovery.files[0].relative_path, "src/main.py");
    assert_eq!(discovery.scan.files_ignored, 2);
    assert!(discovery
        .directories
        .iter()
        .all(|d| d.name != "node_modules"));
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn gitignore_respected_across_full_pipeline() {
    let root = temp_dir("gitignore-tree");
    std::fs::write(root.join(".gitignore"), "secrets/\n*.local\n").unwrap();
    std::fs::create_dir_all(root.join("secrets")).unwrap();
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(root.join("secrets/key.py"), "x").unwrap();
    std::fs::write(root.join("debug.local"), "x").unwrap();
    std::fs::write(root.join("src/main.py"), "x").unwrap();

    let discovery = scan_path(&root);
    let paths: Vec<&str> = discovery
        .files
        .iter()
        .map(|file| file.relative_path.as_str())
        .collect();
    assert_eq!(paths, vec![".gitignore", "src/main.py"]);
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn oversized_file_is_discovered_but_marked_not_analyzable() {
    let root = temp_dir("large-file");
    std::fs::create_dir_all(root.join("src")).unwrap();
    let big = vec![b'x'; 3 * 1024 * 1024]; // 3MB > 默认 2MB 上限
    std::fs::write(root.join("src/huge.py"), &big).unwrap();
    std::fs::write(root.join("src/small.py"), "x").unwrap();

    let discovery = scan_path(&root);
    assert_eq!(
        discovery.files.len(),
        2,
        "large file must still be discovered"
    );
    let huge = discovery
        .files
        .iter()
        .find(|file| file.file_name == "huge.py")
        .unwrap();
    assert!(!huge.analyzable);
    assert_eq!(huge.non_analyzable_reason.as_deref(), Some("too_large"));
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn empty_project_scans_to_empty_completed_result() {
    let root = temp_dir("empty-tree");
    let discovery = scan_path(&root);
    assert_eq!(discovery.files.len(), 0);
    assert_eq!(discovery.directories.len(), 0);
    assert_eq!(discovery.scan.status, TaskState::Completed);
    assert_eq!(discovery.languages.len(), 0);
    assert_eq!(discovery.entry_points.len(), 0);
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn pre_cancelled_token_yields_cancelled_status() {
    let root = temp_dir("cancel-tree");
    std::fs::write(root.join("main.py"), "x").unwrap();
    let token = CancellationToken::new();
    token.cancel();
    let service = DiscoveryService::new();
    let discovery = service
        .discover(&root, ScanOptions::default(), &token, |_| {})
        .unwrap();
    assert_eq!(discovery.scan.status, TaskState::Cancelled);
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn progress_callback_receives_monotonic_counts() {
    let root = temp_dir("progress-tree");
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(root.join("src/a.py"), "x").unwrap();
    std::fs::write(root.join("src/b.py"), "x").unwrap();

    let service = DiscoveryService::new();
    let last_files = AtomicUsize::new(0);
    let discovery = service
        .discover(
            &root,
            ScanOptions::default(),
            &CancellationToken::new(),
            |progress| {
                let previous = last_files.load(Ordering::SeqCst);
                assert!(
                    progress.files as usize >= previous,
                    "progress must be monotonic: {progress:?}"
                );
                last_files.store(progress.files as usize, Ordering::SeqCst);
            },
        )
        .unwrap();
    assert_eq!(discovery.files.len(), 2);
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn repeated_scans_are_structurally_deterministic() {
    let fixture = fixture_path("fixture-multi-language");
    let first = scan_path(&fixture);
    let second = scan_path(&fixture);

    let strip = |discovery: &ProjectDiscovery| {
        let mut value = serde_json::to_value(discovery).unwrap();
        // 运行期事实（时间/ID/路径中的机器段）不参与结构比较。
        value["scan"]["scan_id"] = serde_json::json!("<id>");
        value["scan"]["started_at"] = serde_json::json!("<t>");
        value["scan"]["finished_at"] = serde_json::json!("<t>");
        value["scan"]["duration_ms"] = serde_json::json!(0);
        value["project"]["id"] = serde_json::json!("<id>");
        value["project"]["root_path"] = serde_json::json!("<root>");
        value["scan"]["root_path"] = serde_json::json!("<root>");
        value["project"]["discovered_at"] = serde_json::json!("<t>");
        value
    };
    assert_eq!(strip(&first), strip(&second));
}

#[test]
fn discovery_json_roundtrip_preserves_schema() {
    let discovery = scan_path(&fixture_path("fixture-multi-language"));
    let json = serde_json::to_string(&discovery).unwrap();
    let back: ProjectDiscovery = serde_json::from_str(&json).unwrap();
    assert_eq!(back, discovery);
}
