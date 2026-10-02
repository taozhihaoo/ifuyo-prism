//! 图构建与全管线集成测试（规格 §37/§38）。

use prism_analysis::{CodeAnalysisOptions, CodeAnalysisService};
use prism_core::task::CancellationToken;
use prism_graph::{build_graph, build_modules, DependencyService, ResolutionStatus};
use prism_project::{DiscoveryService, ScanOptions};
use std::path::{Path, PathBuf};

fn temp_root(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "prism-graph-full-{name}-{}-{}",
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

fn graph_of(root: &Path) -> prism_graph::DependencyGraphResult {
    DependencyService::new()
        .inspect_graph(root, &CancellationToken::new())
        .unwrap()
}

#[test]
fn chain_project_builds_resolved_import_edges() {
    let root = temp_root("chain");
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(
        root.join("src/a.ts"),
        "import { b } from \"./b\";\nexport const a = b;\n",
    )
    .unwrap();
    std::fs::write(root.join("src/b.ts"), "export const b = 1;\n").unwrap();

    let result = graph_of(&root);
    let import_edges: Vec<_> = result
        .edges
        .iter()
        .filter(|e| {
            e.kind == prism_graph::GraphEdgeKind::Imports
                && e.resolution == ResolutionStatus::Resolved
        })
        .collect();
    assert_eq!(import_edges.len(), 1);
    assert!(import_edges[0].target.contains("src/b.ts"));
    assert!(result.cycles.is_empty());
    assert!(result.metrics.resolved_edges >= 1);
}

#[test]
fn cycle_is_detected_and_deterministic() {
    let root = temp_root("cycle");
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(
        root.join("src/a.ts"),
        "import { x } from \"./b\";\nexport const a = x;\n",
    )
    .unwrap();
    std::fs::write(
        root.join("src/b.ts"),
        "import { y } from \"./c\";\nexport const x = y;\n",
    )
    .unwrap();
    std::fs::write(
        root.join("src/c.ts"),
        "import { z } from \"./a\";\nexport const y = z;\n",
    )
    .unwrap();

    let first = graph_of(&root);
    assert_eq!(first.metrics.cycle_count, 1);
    let cycle = &first.cycles[0];
    assert_eq!(cycle.nodes.len(), 3);

    let second = graph_of(&root);
    assert_eq!(first.cycles, second.cycles);
    assert_eq!(first.edges, second.edges);
}

#[test]
fn external_and_builtin_and_unresolved_are_distinguished() {
    let root = temp_root("kinds");
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(
        root.join("package.json"),
        "{\"dependencies\": {\"react\": \"18.0.0\"}}",
    )
    .unwrap();
    std::fs::write(
        root.join("src/app.ts"),
        "import react from \"react\";\nimport fs from \"fs\";\nimport missing from \"./missing\";\nimport os from \"os\";\nexport const all = [react, fs, missing, os];\n",
    )
    .unwrap();

    let result = graph_of(&root);
    assert!(result.metrics.external_edges >= 1, "react → external");
    assert!(result.metrics.builtin_edges >= 2, "fs + os → builtin");
    // 诊断：./missing 未命中项目文件。
    assert!(result
        .diagnostics
        .iter()
        .any(|d| d.raw_target == "./missing" && d.status == ResolutionStatus::Unresolved));
    // External/Builtin 不产生诊断（它们是明确状态）。
    assert!(!result.diagnostics.iter().any(|d| d.raw_target == "react"));
}

#[test]
fn full_pipeline_on_self_scan() {
    // Prism 自扫描（规格 §39）。
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let result = graph_of(&root);
    assert!(result.metrics.node_count > 10);
    assert!(result
        .nodes
        .iter()
        .any(|n| n.kind == prism_graph::GraphNodeKind::ExternalPackage));
    assert!(result.metrics.resolved_edges > 0);
}

#[test]
fn graph_builder_builds_contains_and_metrics() {
    let root = temp_root("builder");
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(root.join("src/a.ts"), "export const a = 1;\n").unwrap();

    let discovery = DiscoveryService::new()
        .discover(
            &root,
            ScanOptions::default(),
            &CancellationToken::new(),
            |_| {},
        )
        .unwrap();
    let model = CodeAnalysisService::new()
        .analyze(
            &discovery,
            CodeAnalysisOptions::default(),
            &CancellationToken::new(),
            |_| {},
        )
        .unwrap();
    let modules = build_modules(&discovery, &model);
    let deps = DependencyService::new();
    let result = deps.analyze(&discovery, &model).unwrap();
    let assembly = build_graph(&modules, &result.dependencies);

    assert!(assembly
        .nodes
        .iter()
        .any(|n| n.kind == prism_graph::GraphNodeKind::Project));
    assert!(assembly
        .edges
        .iter()
        .any(|e| e.kind == prism_graph::GraphEdgeKind::Contains));
    assert!(assembly.metrics.node_count >= 2);
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn call_graph_resolves_same_file_import_and_dynamic() {
    let root = temp_root("calls");
    std::fs::create_dir_all(root.join("src")).unwrap();
    // same.ts: main() 调用同文件 helper() → Resolved（同文件）
    // imported.ts: main() 调用 useUtil()（来自 resolved import ./util）→ Resolved
    // util.ts: useUtil 定义
    // dynamic.ts: handler = compute(); handler() → Unresolved
    std::fs::write(
        root.join("src/same.ts"),
        "function helper(): number {\n    return 1;\n}\nexport function main(): number {\n    return helper();\n}\n",
    )
    .unwrap();
    std::fs::write(
        root.join("src/util.ts"),
        "export function useUtil(): number {\n    return 2;\n}\n",
    )
    .unwrap();
    std::fs::write(
        root.join("src/imported.ts"),
        "import { useUtil } from \"./util\";\nexport function main(): number {\n    return useUtil();\n}\n",
    )
    .unwrap();
    std::fs::write(
        root.join("src/dynamic.ts"),
        "function run(): void {\n    const handler = getHandler();\n    handler();\n}\n",
    )
    .unwrap();

    let result = graph_of(&root);
    let calls = &result.calls;
    assert!(calls.len() >= 3, "expected >= 3 calls, got {}", calls.len());

    // 同文件解析（High）
    let same = calls
        .iter()
        .find(|c| c.callee_name == "helper")
        .expect("helper call");
    assert_eq!(same.resolution, ResolutionStatus::Resolved);
    assert_eq!(same.confidence, prism_core::Confidence::High);
    assert!(same
        .callee_qualified
        .as_deref()
        .unwrap()
        .ends_with("helper"));

    // import 解析（Medium）：callee 符号位于 ./util
    let imported = calls
        .iter()
        .find(|c| c.callee_name == "useUtil")
        .expect("useUtil call");
    assert_eq!(imported.resolution, ResolutionStatus::Resolved);
    assert_eq!(imported.confidence, prism_core::Confidence::Medium);
    assert!(imported
        .callee_qualified
        .as_deref()
        .unwrap()
        .ends_with("util.useUtil"));

    // 动态名：handler → Unresolved，不指向任何符号
    let dynamic = calls
        .iter()
        .find(|c| c.callee_name == "handler" && c.callee_text == "handler")
        .expect("handler call");
    assert_eq!(dynamic.resolution, ResolutionStatus::Unresolved);
    assert!(dynamic.callee_id.is_none());

    // 确定性
    let again = graph_of(&root);
    assert_eq!(result.calls, again.calls);
}
