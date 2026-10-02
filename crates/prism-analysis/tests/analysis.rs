//! CodeAnalysisService 集成测试：选择/并发/取消/确定性/索引。

use prism_analysis::{CodeAnalysisOptions, CodeAnalysisService, SymbolIndex};
use prism_core::task::{CancellationToken, TaskState};
use prism_parser::ParseStatus;
use prism_project::{DiscoveryService, FileKind};
use std::path::PathBuf;

fn temp_root(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "prism-analysis-{name}-{}-{}",
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

fn discovery_of(root: &std::path::Path) -> prism_project::ProjectDiscovery {
    DiscoveryService::new()
        .discover(
            root,
            prism_project::ScanOptions::default(),
            &CancellationToken::new(),
            |_| {},
        )
        .unwrap()
}

#[test]
fn full_pipeline_selects_parses_and_skips_correctly() {
    let root = temp_root("pipeline");
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::create_dir_all(root.join("src/generated")).unwrap();
    std::fs::create_dir_all(root.join("vendor")).unwrap();
    std::fs::write(
        root.join("src/app.ts"),
        "export class App {\n    render(): string { return \"x\"; }\n}\n",
    )
    .unwrap();
    std::fs::write(
        root.join("src/service.py"),
        "class Service:\n    def run(self):\n        return 1\n",
    )
    .unwrap();
    std::fs::write(root.join("src/generated/old.ts"), "export class Old {}\n").unwrap();
    std::fs::write(root.join("vendor/lib.rs"), "struct RubyLike;\n").unwrap();
    std::fs::write(root.join("README.md"), "docs").unwrap();

    let discovery = discovery_of(&root);
    let model = CodeAnalysisService::new()
        .analyze(
            &discovery,
            CodeAnalysisOptions::default(),
            &CancellationToken::new(),
            |_| {},
        )
        .unwrap();

    // 管线：app.ts + service.py 解析；generated/old.ts 跳过；lib.rs unsupported。
    assert_eq!(model.parse_results.len(), 4);
    assert_eq!(model.summary.files_parsed, 2);
    assert_eq!(model.summary.files_unsupported, 1);
    assert_eq!(model.summary.files_skipped, 1);
    assert_eq!(model.summary.status, TaskState::Completed);

    let by_path = |p: &str| {
        model
            .parse_results
            .iter()
            .find(|r| r.relative_path == p)
            .unwrap_or_else(|| panic!("missing {p}"))
    };
    assert_eq!(by_path("src/app.ts").status, ParseStatus::Successful);
    assert!(by_path("src/app.ts")
        .symbols
        .iter()
        .any(|s| s.kind == prism_parser::SymbolKind::Class));
    assert_eq!(by_path("src/service.py").status, ParseStatus::Successful);
    assert_eq!(by_path("vendor/lib.rs").status, ParseStatus::Unsupported);
    assert_eq!(
        by_path("vendor/lib.rs").reason.as_deref(),
        Some("no_parser")
    );
    assert_eq!(by_path("src/generated/old.ts").status, ParseStatus::Skipped);
    assert_eq!(
        by_path("src/generated/old.ts").reason.as_deref(),
        Some("generated")
    );

    // 只包含管线文件（Source + Generated 跳过声明）；Documentation 等不进入。
    assert!(model.files.iter().all(|f| matches!(
        f.classification.kind,
        FileKind::Source | FileKind::Generated
    )));
    assert!(model.files.iter().all(|f| f.relative_path != "README.md"));

    // 结果按路径排序（确定性）。
    let paths: Vec<&str> = model
        .parse_results
        .iter()
        .map(|r| r.relative_path.as_str())
        .collect();
    let mut sorted = paths.clone();
    sorted.sort();
    assert_eq!(paths, sorted);

    // 符号模型总数一致。
    assert_eq!(model.summary.symbols_total, model.symbols().count() as u64);
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn repeated_analysis_is_deterministic() {
    let root = temp_root("determinism");
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(root.join("src/a.ts"), "export class A { go(): void {} }\n").unwrap();
    std::fs::write(
        root.join("src/b.py"),
        "class B:\n    def go(self):\n        pass\n",
    )
    .unwrap();

    let service = CodeAnalysisService::new();
    let first = service
        .analyze(
            &discovery_of(&root),
            CodeAnalysisOptions::default(),
            &CancellationToken::new(),
            |_| {},
        )
        .unwrap();
    let second = service
        .analyze(
            &discovery_of(&root),
            CodeAnalysisOptions::default(),
            &CancellationToken::new(),
            |_| {},
        )
        .unwrap();

    let strip = |model: &prism_analysis::ProjectCodeModel| {
        let mut value = serde_json::to_value(model).unwrap();
        value["summary"]["duration_ms"] = serde_json::json!(0);
        value["project"]["id"] = serde_json::json!("<id>");
        value["project"]["discovered_at"] = serde_json::json!("<t>");
        for result in value["parse_results"].as_array_mut().unwrap() {
            result["duration_ms"] = serde_json::json!(0);
        }
        value
    };
    assert_eq!(strip(&first), strip(&second));
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn cancellation_preserves_partial_model_with_status() {
    let root = temp_root("cancel");
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(root.join("src/main.py"), "def main():\n    pass\n").unwrap();

    let token = CancellationToken::new();
    token.cancel();
    let model = CodeAnalysisService::new()
        .analyze(
            &discovery_of(&root),
            CodeAnalysisOptions::default(),
            &token,
            |_| {},
        )
        .unwrap();
    assert_eq!(model.summary.status, TaskState::Cancelled);
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn symbol_index_resolves_extracted_symbols() {
    let root = temp_root("index");
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(
        root.join("src/app.ts"),
        "export class App {\n    run(): void {}\n}\n",
    )
    .unwrap();

    let discovery = discovery_of(&root);
    let model = CodeAnalysisService::new()
        .analyze(
            &discovery,
            CodeAnalysisOptions::default(),
            &CancellationToken::new(),
            |_| {},
        )
        .unwrap();
    let index = SymbolIndex::build(&model);

    let app_ids = index.by_name("App");
    assert_eq!(app_ids.len(), 1);
    let app = index.get(&model, &app_ids[0]).unwrap();
    assert_eq!(app.qualified_name, "src/app.App");

    let children = index.children_ids(&app.id);
    assert_eq!(children.len(), 1);
    let run = index.get(&model, &children[0]).unwrap();
    assert_eq!(run.kind, prism_parser::SymbolKind::Method);
    assert_eq!(run.parent_id.as_ref(), Some(&app.id));
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn bounded_concurrency_processes_all_files() {
    let root = temp_root("concurrency");
    std::fs::create_dir_all(root.join("src")).unwrap();
    for i in 0..12 {
        std::fs::write(
            root.join(format!("src/m{i}.py")),
            format!("def main_{i}():\n    pass\n"),
        )
        .unwrap();
    }
    let model = CodeAnalysisService::new()
        .analyze(
            &discovery_of(&root),
            CodeAnalysisOptions { max_workers: 3 },
            &CancellationToken::new(),
            |_| {},
        )
        .unwrap();
    assert_eq!(model.summary.files_parsed, 12);
    assert_eq!(model.summary.symbols_total, 24); // 每文件 File + Function
    std::fs::remove_dir_all(&root).ok();
}
