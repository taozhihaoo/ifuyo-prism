# -*- coding: utf-8 -*-
"""M3 CLI main.rs: run_graph implementation."""

import io

p = "crates/prism-cli/src/main.rs"
s = io.open(p, encoding="utf-8", newline="\n").read()

s = s.replace(
    "use prism_analysis::{CodeAnalysisOptions, CodeAnalysisService, ProjectCodeModel};",
    "use prism_analysis::{CodeAnalysisOptions, CodeAnalysisService, ProjectCodeModel};\nuse prism_graph::{DependencyGraphResult, DependencyService, GraphNodeKind, ResolutionStatus};",
)

s = s.replace(
    "        Some(cli::Command::Inspect { path, format }) => run_inspect(&path, format.into()),",
    "        Some(cli::Command::Inspect { path, format }) => run_inspect(&path, format.into()),\n        Some(cli::Command::Graph { path, kind, format }) => {\n            run_graph(&path, kind.as_str(), format.into())\n        }",
)

s = s.replace(
    "fn not_implemented_yet(name: &str) -> i32 {",
    '''fn run_graph(path: &Path, kind: &str, format: OutputFormat) -> i32 {
    let cancel = CancellationToken::new();
    let result = DependencyService::new().inspect_graph(path, &cancel);
    match result {
        Ok(graph) => {
            match format {
                OutputFormat::Json => match serde_json::to_string_pretty(&graph) {
                    Ok(text) => println!("{text}"),
                    Err(err) => {
                        eprintln!("prism: failed to serialize dependency graph: {err}");
                        return EXIT_ERROR;
                    }
                },
                OutputFormat::Text => print_graph_text(&graph, kind),
            }
            let _ = kind;
            EXIT_OK
        }
        Err(err) => {
            eprintln!("prism: graph failed: {err}");
            if matches!(
                err.code(),
                PrismErrorCode::InvalidInput | PrismErrorCode::NotFound
            ) {
                EXIT_INVALID_INPUT
            } else {
                EXIT_ERROR
            }
        }
    }
}

fn print_graph_text(graph: &DependencyGraphResult, kind: &str) {
    let metrics = &graph.metrics;
    println!("ifuyo-prism");
    println!("Dependency & Architecture ({kind} view)");
    println!();
    println!("Project: {}", graph.project_name);
    println!();
    println!("Graph");
    println!("  Nodes: {}", metrics.node_count);
    println!("  Edges: {}", metrics.edge_count);
    println!("  Resolved: {}", metrics.resolved_edges);
    println!("  External: {}", metrics.external_edges);
    println!("  Builtin: {}", metrics.builtin_edges);
    println!("  Unresolved: {}", metrics.unresolved_edges);
    println!("  Ambiguous: {}", metrics.ambiguous_edges);
    println!("  Cycles: {}", metrics.cycle_count);
    println!("  Components: {}", metrics.connected_components);
    println!();
    if !metrics.most_referenced.is_empty() {
        println!("Most referenced (fan-in)");
        for (id, count) in metrics.most_referenced.iter().take(5) {
            println!("  {count:>4}  {id}");
        }
        println!();
    }
    if !metrics.most_dependent.is_empty() {
        println!("Most dependent (fan-out)");
        for (id, count) in metrics.most_dependent.iter().take(5) {
            println!("  {count:>4}  {id}");
        }
        println!();
    }
    if !graph.cycles.is_empty() {
        println!("Cycles");
        for cycle in graph.cycles.iter().take(10) {
            println!("  {} ({})", cycle.id, cycle.nodes.len());
        }
        println!();
    }
    if !graph.diagnostics.is_empty() {
        println!("Unresolved dependencies");
        for diagnostic in graph.diagnostics.iter().take(20) {
            println!(
                "  {}: {} — {}",
                diagnostic.source_path, diagnostic.raw_target, diagnostic.reason
            );
        }
        println!();
    }
    // 按层级视图展示节点（file/module/project 视图过滤）。
    println!("Nodes ({kind})");
    let wanted = match kind {
        "module" => "namespace",
        "project" => "project",
        _ => "file",
    };
    for node in graph.nodes.iter().take(30) {
        let matches = match kind {
            "module" => {
                node.kind == GraphNodeKind::Package || node.kind == GraphNodeKind::Namespace
            }
            "project" => node.kind == GraphNodeKind::Project,
            _ => node.kind == GraphNodeKind::FileModule,
        };
        if matches {
            println!("  {}", node.label);
        }
        if wanted == "never" {
            break;
        }
    }
    let total: usize = graph
        .nodes
        .iter()
        .filter(|node| match kind {
            "module" => {
                node.kind == GraphNodeKind::Package || node.kind == GraphNodeKind::Namespace
            }
            "project" => node.kind == GraphNodeKind::Project,
            _ => node.kind == GraphNodeKind::FileModule,
        })
        .count();
    if total > 30 {
        println!("  … and {} more", total - 30);
    }
    if graph.diagnostics.is_empty() && graph.metrics.unresolved_edges == 0 {
        println!();
        println!("No unresolved dependencies.");
    }
}

fn not_implemented_yet(name: &str) -> i32 {'''
)

io.open(p, "w", encoding="utf-8", newline="\n").write(s)
print("main.rs done")
