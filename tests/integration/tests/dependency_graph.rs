//! M3 Dependency & Architecture 集成测试（规格 §36–§39）。
//!
//! fixture `dependency-graph` 覆盖：链式依赖、环（a↔c 经 b/y 与 y2/x 双环
//! 结构）、tsconfig paths、external（react）、builtin（fs）、unresolved
//! （./ghost 与 ghost）。

mod support;

use prism_analysis::{CodeAnalysisOptions, CodeAnalysisService};
use prism_core::task::CancellationToken;
use prism_graph::{DependencyService, ResolutionStatus};
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

fn graph_of(path: &Path) -> prism_graph::DependencyGraphResult {
    let discovery = DiscoveryService::new()
        .discover(
            path,
            prism_project::ScanOptions::default(),
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
    DependencyService::new()
        .analyze(&discovery, &model)
        .unwrap()
}

#[test]
fn dependency_graph_fixture_full_resolution() {
    let result = graph_of(&fixture("dependency-graph"));

    // 各解析状态都有代表（规格 §10）。
    let resolved = result
        .dependencies
        .iter()
        .filter(|d| d.resolution == ResolutionStatus::Resolved)
        .count();
    let external = result
        .dependencies
        .iter()
        .filter(|d| d.resolution == ResolutionStatus::External)
        .count();
    let builtin = result
        .dependencies
        .iter()
        .filter(|d| d.resolution == ResolutionStatus::Builtin)
        .count();
    let unresolved = result
        .dependencies
        .iter()
        .filter(|d| d.resolution == ResolutionStatus::Unresolved)
        .count();

    assert!(resolved >= 5, "chain + paths + cycle resolved: {resolved}");
    assert_eq!(external, 1, "react must be external");
    assert_eq!(builtin, 1, "fs must be builtin");
    // ./ghost 未命中项目文件（ghost 裸名在本 fixture 无独立导入）。
    assert!(unresolved >= 1, "./ghost must be unresolved: {unresolved}");

    // tsconfig paths：@/helpers/helper → src/helpers/helper.ts。
    assert!(result.dependencies.iter().any(|d| {
        d.raw_target == "@/helpers/helper"
            && d.resolution == ResolutionStatus::Resolved
            && d.target_path.as_deref() == Some("src/helpers/helper.ts")
    }));

    // 链式：a.ts → b.ts（Resolved，目标正确）。
    assert!(result.dependencies.iter().any(|d| {
        d.source_path.ends_with("a.ts")
            && d.raw_target == "./b"
            && d.target_path.as_deref() == Some("src/b.ts")
    }));

    // unresolved 关系必须产生诊断（规格 §26）。
    assert!(result
        .diagnostics
        .iter()
        .any(|d| d.raw_target.contains("ghost")));
}

#[test]
fn cycles_detected_with_stable_ids() {
    let result = graph_of(&fixture("dependency-graph"));
    assert!(result.metrics.cycle_count >= 1, "fixture contains cycles");
    // x.ts -> y2.ts -> x.ts 是一个最小环。
    assert!(result.cycles.iter().any(|cycle| cycle.nodes.len() == 2));

    let again = graph_of(&fixture("dependency-graph"));
    assert_eq!(
        result.cycles, again.cycles,
        "cycle ids must be deterministic"
    );
}

#[test]
fn metrics_are_consistent_with_graph() {
    let result = graph_of(&fixture("dependency-graph"));
    assert_eq!(result.metrics.node_count, result.nodes.len() as u64);
    assert_eq!(result.metrics.edge_count, result.edges.len() as u64);
    assert_eq!(result.metrics.cycle_count, result.cycles.len() as u64);
    // 排序契约：节点/边/依赖/诊断全部按稳定 ID / 路径排序。
    let node_ids: Vec<&str> = result.nodes.iter().map(|n| n.id.as_str()).collect();
    let mut sorted_nodes = node_ids.clone();
    sorted_nodes.sort();
    assert_eq!(node_ids, sorted_nodes);
    let dep_ids: Vec<&str> = result.dependencies.iter().map(|d| d.id.as_str()).collect();
    let mut sorted_deps = dep_ids.clone();
    sorted_deps.sort();
    assert_eq!(dep_ids, sorted_deps);
}

#[test]
fn graph_json_roundtrip_preserves_schema() {
    let result = graph_of(&fixture("dependency-graph"));
    let json = serde_json::to_string(&result).unwrap();
    let back: prism_graph::DependencyGraphResult = serde_json::from_str(&json).unwrap();
    assert_eq!(back, result);
    assert_eq!(back.schema_version, prism_graph::GRAPH_SCHEMA_VERSION);
}

#[test]
fn empty_project_graph_is_valid() {
    let root = temp_dir("m3-empty");
    let result = graph_of(&root);
    assert_eq!(result.nodes.len(), 1, "project node always exists");
    assert_eq!(result.edges.len(), 0);
    assert_eq!(result.metrics.connected_components, 1);
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn full_pipeline_chain_chain_reaction() {
    // prism graph CLI 用的同一条链（discover -> analyze -> resolve -> graph）。
    let result = graph_of(&fixture("dependency-graph"));
    let a_to_b = result
        .dependencies
        .iter()
        .find(|d| d.source_path.ends_with("a.ts") && d.raw_target == "./b")
        .expect("a -> b dependency");
    assert_eq!(a_to_b.resolution, ResolutionStatus::Resolved);
    assert_eq!(a_to_b.confidence, prism_core::Confidence::High);
}
