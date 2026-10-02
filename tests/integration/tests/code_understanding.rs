//! M2 Code Understanding 集成测试（规格 §47–§52）。
//!
//! 固定 fixtures：`tests/fixtures/{basic-*,nested-symbols,syntax-errors,
//! unicode,unsupported-language,generated,empty}`；large-file 在运行期构造
//! （避免提交大文件）。

mod support;

use prism_analysis::{CodeAnalysisOptions, CodeAnalysisService, SymbolIndex};
use prism_core::task::{CancellationToken, TaskState};
use prism_parser::{ParseStatus, SymbolKind};
use prism_project::DiscoveryService;
use std::path::Path;
use support::temp_dir;

fn fixture(name: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(name)
        .canonicalize()
        .expect("fixture must exist")
}

fn inspect(path: &std::path::Path) -> prism_analysis::ProjectCodeModel {
    let discovery = DiscoveryService::new()
        .discover(
            path,
            prism_project::ScanOptions::default(),
            &CancellationToken::new(),
            |_| {},
        )
        .unwrap();
    CodeAnalysisService::new()
        .analyze(
            &discovery,
            CodeAnalysisOptions::default(),
            &CancellationToken::new(),
            |_| {},
        )
        .unwrap()
}

fn result_of<'a>(
    model: &'a prism_analysis::ProjectCodeModel,
    path: &str,
) -> &'a prism_parser::ParseResult {
    model
        .parse_results
        .iter()
        .find(|r| r.relative_path.ends_with(path))
        .unwrap_or_else(|| panic!("parse result for {path} missing"))
}

#[test]
fn basic_typescript_fixture_extracts_expected_symbols() {
    let model = inspect(&fixture("basic-typescript"));
    let index = SymbolIndex::build(&model);
    let index_ts = result_of(&model, "index.ts");
    assert_eq!(index_ts.status, ParseStatus::Successful);

    for name in ["Widget", "Named", "build"] {
        assert!(
            index_ts.symbols.iter().any(|s| s.name == name),
            "missing {name}: {:?}",
            index_ts.symbols.iter().map(|s| &s.name).collect::<Vec<_>>()
        );
    }
    // 层级：render 方法属于 Widget 类。
    let widget = index.by_name("Widget")[0].clone();
    let render = index.children_ids(&widget);
    assert!(render
        .iter()
        .filter_map(|id| index.get(&model, id))
        .any(|s| s.kind == SymbolKind::Method && s.name == "render"));
}

#[test]
fn basic_javascript_python_csharp_fixtures_parse() {
    let model = inspect(&fixture("basic-javascript"));
    let js = result_of(&model, "index.js");
    assert_eq!(js.status, ParseStatus::Successful);
    assert!(js
        .symbols
        .iter()
        .any(|s| s.kind == SymbolKind::Class && s.name == "Store"));
    assert!(js
        .symbols
        .iter()
        .any(|s| s.kind == SymbolKind::Method && s.name == "add"));

    let model = inspect(&fixture("basic-python"));
    let py = result_of(&model, "main.py");
    assert_eq!(py.status, ParseStatus::Successful);
    assert!(py
        .symbols
        .iter()
        .any(|s| s.kind == SymbolKind::Class && s.name == "Service"));
    assert!(py
        .symbols
        .iter()
        .any(|s| s.kind == SymbolKind::Method && s.name == "run"));
    assert!(py
        .symbols
        .iter()
        .any(|s| s.kind == SymbolKind::Function && s.name == "build_service"));

    let model = inspect(&fixture("basic-csharp"));
    let cs = result_of(&model, "Program.cs");
    assert_eq!(cs.status, ParseStatus::Successful);
    assert!(cs
        .symbols
        .iter()
        .any(|s| s.kind == SymbolKind::Namespace && s.qualified_name.ends_with("Basic.CSharp")));
    assert!(cs
        .symbols
        .iter()
        .any(|s| s.kind == SymbolKind::Class && s.name == "Greeter"));
    assert!(cs
        .symbols
        .iter()
        .any(|s| s.kind == SymbolKind::Property && s.name == "Message"));
    assert!(cs
        .symbols
        .iter()
        .any(|s| s.kind == SymbolKind::Method && s.name == "Greet"));
}

#[test]
fn nested_symbols_have_correct_hierarchy_and_qualified_names() {
    let model = inspect(&fixture("nested-symbols"));
    let index = SymbolIndex::build(&model);

    // TS: namespace Deep > class Outer > method inner
    let ts_nested = result_of(&model, "nested.ts");
    let outer = ts_nested
        .symbols
        .iter()
        .find(|s| s.kind == SymbolKind::Class && s.name == "Outer")
        .expect("Outer class");
    let outer_parent = index
        .get(&model, outer.parent_id.as_ref().unwrap())
        .unwrap();
    assert_eq!(outer_parent.kind, SymbolKind::Namespace);
    assert_eq!(outer_parent.name, "Deep");
    let inner = ts_nested
        .symbols
        .iter()
        .find(|s| s.name == "inner")
        .unwrap();
    assert_eq!(inner.parent_id.as_ref(), Some(&outer.id));
    assert!(inner
        .qualified_name
        .starts_with("src/nested.Deep.Outer.inner"));

    // C#: namespace Deep.Space > class Container > method Answer
    let cs_nested = result_of(&model, "Nested.cs");
    let container = cs_nested
        .symbols
        .iter()
        .find(|s| s.kind == SymbolKind::Class && s.name == "Container")
        .expect("Container class");
    let container_parent = index
        .get(&model, container.parent_id.as_ref().unwrap())
        .unwrap();
    assert_eq!(container_parent.kind, SymbolKind::Namespace);
    assert!(container_parent.qualified_name.ends_with("Deep.Space"));
}

#[test]
fn syntax_errors_yield_partial_success_not_project_failure() {
    let model = inspect(&fixture("syntax-errors"));
    assert_eq!(model.summary.status, TaskState::Completed);

    let broken = result_of(&model, "broken.ts");
    assert!(
        matches!(broken.status, ParseStatus::SuccessfulWithDiagnostics),
        "broken.ts must carry diagnostics"
    );
    assert!(broken.has_errors());
    // 部分解析：Broken 类与 ok 方法仍被保留（规格 §24）。
    assert!(broken
        .symbols
        .iter()
        .any(|s| s.kind == SymbolKind::Class && s.name == "Broken"));
    assert!(broken.symbols.iter().any(|s| s.name == "ok"));

    assert_eq!(
        result_of(&model, "valid.ts").status,
        ParseStatus::Successful
    );
    assert_eq!(
        result_of(&model, "valid.py").status,
        ParseStatus::Successful
    );

    // 项目级退出语义：有解析错误 → warnings（由 CLI 层映射 exit 5）。
    assert!(model.summary.error_count > 0);
}

#[test]
fn unicode_fixture_locations_follow_byte_contract() {
    let model = inspect(&fixture("unicode"));
    let py = result_of(&model, "unicode.py");
    assert_eq!(py.status, ParseStatus::Successful);

    let function = py
        .symbols
        .iter()
        .find(|s| s.kind == SymbolKind::Function)
        .expect("计算价格 must be extracted");
    assert_eq!(function.name, "计算价格");
    assert_eq!(function.range.start.line, 3); // 1 注释, 2 空, 3 空, 4 def
    assert_eq!(function.range.start.column, 1); // UTF-8 字节列，行首

    let class = py
        .symbols
        .iter()
        .find(|s| s.kind == SymbolKind::Class)
        .unwrap();
    assert_eq!(class.name, "订单");

    // CRLF 文件解析正常且行号正确。
    let crlf = result_of(&model, "crlf.ts");
    assert_eq!(crlf.status, ParseStatus::Successful);
    let method = crlf
        .symbols
        .iter()
        .find(|s| s.kind == SymbolKind::Method)
        .unwrap();
    assert_eq!(method.range.start.line, 2);
}

#[test]
fn unsupported_language_files_are_reported_not_fatal() {
    let model = inspect(&fixture("unsupported-language"));
    // example.rb：M1 语言表无 Ruby → language None → 不进入代码模型（如实记录在 discovery）。
    assert_eq!(model.summary.files_total, 0);
    // 已识别但无 parser 的语言（如 Rust）由 pipeline 测试覆盖（见 analysis.rs）。
}

#[test]
fn generated_files_are_skipped_by_policy() {
    let model = inspect(&fixture("generated"));
    let generated = result_of(&model, "out.ts");
    assert_eq!(generated.status, ParseStatus::Skipped);
    assert_eq!(generated.reason.as_deref(), Some("generated"));
    assert!(generated.symbols.is_empty());
}

#[test]
fn empty_project_yields_empty_completed_model() {
    let model = inspect(&fixture("empty"));
    assert_eq!(model.parse_results.len(), 0);
    assert_eq!(model.summary.status, TaskState::Completed);
    assert_eq!(model.summary.symbols_total, 0);
}

#[test]
fn large_files_are_discovered_but_skipped() {
    let root = temp_dir("m2-large");
    std::fs::create_dir_all(root.join("src")).unwrap();
    let big = vec![b'x'; 3 * 1024 * 1024];
    std::fs::write(root.join("src/huge.py"), &big).unwrap();
    std::fs::write(root.join("src/small.py"), "def ok():\n    pass\n").unwrap();

    let model = inspect(&root);
    assert_eq!(model.parse_results.len(), 2);
    let huge = result_of(&model, "huge.py");
    assert_eq!(huge.status, ParseStatus::Skipped);
    assert_eq!(huge.reason.as_deref(), Some("too_large"));
    assert_eq!(
        result_of(&model, "small.py").status,
        ParseStatus::Successful
    );
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn inspect_is_deterministic_across_runs_on_fixture() {
    let fixture = fixture("basic-typescript");
    let first = inspect(&fixture);
    let second = inspect(&fixture);
    let strip = |model: &prism_analysis::ProjectCodeModel| {
        let mut value = serde_json::to_value(model).unwrap();
        value["summary"]["duration_ms"] = serde_json::json!(0);
        value["project"]["id"] = serde_json::json!("<id>");
        value["project"]["discovered_at"] = serde_json::json!("<t>");
        for result in value["parse_results"].as_array_mut().unwrap() {
            result["duration_ms"] = serde_json::json!(0);
            result["file_id"] = serde_json::json!("<id>");
            for symbol in result["symbols"].as_array_mut().unwrap() {
                symbol["file_id"] = serde_json::json!("<id>");
            }
        }
        for file in value["files"].as_array_mut().unwrap() {
            file["id"] = serde_json::json!("<id>");
        }
        value
    };
    assert_eq!(strip(&first), strip(&second));
}
