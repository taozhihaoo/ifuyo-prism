//! `prism viz`（M212 §60）：CLI 消费与 GUI/MCP 同一投影
//! （prism-viz，§71 数据一致性）。输出 JSON 或文本树。

use prism_core::task::CancellationToken;
use prism_viz::{VizGraph, VizLimits};

/// 按 view 组装输入并投影（与 GUI get_visualization_graph 同一数据面）。
pub fn run(path: &std::path::Path, view: &str, json: bool) -> i32 {
    let cancel = CancellationToken::new();
    let result: Result<VizGraph, String> = match view {
        "architecture" => {
            let discovery = match prism_project::DiscoveryService::new().discover(
                path,
                prism_project::ScanOptions::default(),
                &cancel,
                |_| {},
            ) {
                Ok(discovery) => discovery,
                Err(err) => return fail("viz architecture", &err.to_string()),
            };
            let files: Vec<String> = discovery
                .files
                .iter()
                .filter(|file| file.analyzable)
                .map(|file| file.relative_path.clone())
                .collect();
            // M3 依赖图（bounded 解析；失败 ⇒ 无边投影 + 诊断）。
            let deps = match build_dependency_graph(path, &cancel) {
                Ok(deps) => deps,
                Err(err) => {
                    eprintln!("prism: viz dependency graph unavailable: {err}");
                    prism_graph::DependencyGraphResult {
                        schema_version: 1,
                        project_name: path
                            .file_name()
                            .map(|name| name.to_string_lossy().to_string())
                            .unwrap_or_default(),
                        modules: vec![],
                        dependencies: vec![],
                        nodes: vec![],
                        edges: vec![],
                        metrics: Default::default(),
                        calls: vec![],
                        cycles: vec![],
                        diagnostics: vec![],
                    }
                }
            };
            let input = prism_viz::architecture::ArchitectureInput {
                files: &files,
                dependencies: &deps,
            };
            Ok(prism_viz::architecture_graph(&input, &VizLimits::default()))
        }
        "godot" => (|| {
            let discovery = prism_project::DiscoveryService::new()
                .discover(path, prism_project::ScanOptions::default(), &cancel, |_| {})
                .map_err(|err| err.to_string())?;
            let inventory: Vec<(String, String)> = discovery
                .files
                .iter()
                .filter(|file| {
                    file.relative_path == "project.godot"
                        || file.relative_path.ends_with(".tscn")
                        || file.relative_path.ends_with(".tres")
                })
                .map(|file| (file.relative_path.clone(), String::new()))
                .collect();
            let project = prism_project::Project::from_root(path).map_err(|err| err.to_string())?;
            let context = prism_intel::AnalysisContext::new(
                prism_intel::ProjectIdentity::from_project(&project),
                prism_intel::AnalysisScope::Domain("godot".to_string()),
                vec!["godot".to_string()],
                cancel.clone(),
            );
            let analysis = prism_godot::analyze(&inventory, &project.root_path, &context)
                .data
                .ok_or_else(|| "godot analysis unavailable".to_string())?;
            let mut engine = prism_csharp::SymbolEngine::new();
            for file in &discovery.files {
                let is_cs = file
                    .relative_path
                    .rsplit('/')
                    .next()
                    .map(|name| name.ends_with(".cs"))
                    .unwrap_or(false);
                if !is_cs || file.size_bytes > 4 * 1_048_576 {
                    continue;
                }
                if let Ok(source) = std::fs::read_to_string(path.join(&file.relative_path)) {
                    engine.update_file(&file.relative_path, &source);
                }
            }
            let index = engine.build_index();
            Ok(prism_viz::godot_graph(
                &analysis,
                Some(&index),
                &VizLimits::default(),
            ))
        })(),
        "dependency" => (|| {
            let discovery = prism_project::DiscoveryService::new()
                .discover(path, prism_project::ScanOptions::default(), &cancel, |_| {})
                .map_err(|err| err.to_string())?;
            let mut manifests = std::collections::BTreeMap::new();
            for file in &discovery.files {
                let lower = file.relative_path.to_ascii_lowercase();
                let is_manifest = file.relative_path == "project.godot"
                    || lower.ends_with(".csproj")
                    || lower.ends_with("directory.packages.props")
                    || lower == "global.json"
                    || lower.ends_with("packages.lock.json")
                    || lower.ends_with("cargo.lock")
                    || lower.ends_with("cargo.toml")
                    || lower.ends_with("package.json");
                if !is_manifest || file.size_bytes > 1024 * 1024 {
                    continue;
                }
                if let Ok(content) = std::fs::read_to_string(path.join(&file.relative_path)) {
                    manifests.insert(file.relative_path.clone(), content);
                }
            }
            let inputs = prism_supply::SupplyInputs {
                files: &manifests,
                text_files: &[],
                observed_tools: &std::collections::BTreeMap::new(),
                advisory_provider: None,
                license_policy: None,
                secret_limits: Box::leak(Box::new(
                    prism_supply::secrets::SecretScanLimits::default(),
                )),
            };
            let supply = prism_supply::collect(&inputs);
            Ok(prism_viz::dependency_graph(&supply, &VizLimits::default()))
        })(),
        other => {
            eprintln!("prism: unknown viz view '{other}' (architecture|godot|dependency)");
            return crate::EXIT_INVALID_INPUT;
        }
    };
    match result {
        Ok(graph) => {
            if json {
                match serde_json::to_string_pretty(&graph) {
                    Ok(text) => println!("{text}"),
                    Err(err) => {
                        eprintln!("prism: failed to serialize viz graph: {err}");
                        return crate::EXIT_ERROR;
                    }
                }
            } else {
                print_text(&graph);
            }
            crate::EXIT_OK
        }
        Err(err) => fail("viz", &err.to_string()),
    }
}

fn build_dependency_graph(
    path: &std::path::Path,
    cancel: &CancellationToken,
) -> Result<prism_graph::DependencyGraphResult, String> {
    let discovery = prism_project::DiscoveryService::new()
        .discover(path, prism_project::ScanOptions::default(), cancel, |_| {})
        .map_err(|err| err.to_string())?;
    let code_model = prism_analysis::CodeAnalysisService::new()
        .analyze(
            &discovery,
            prism_analysis::CodeAnalysisOptions::default(),
            cancel,
            |_| {},
        )
        .map_err(|err| err.to_string())?;
    prism_graph::DependencyService::new()
        .analyze(&discovery, &code_model)
        .map_err(|err| err.to_string())
}

fn print_text(graph: &VizGraph) {
    println!(
        "viz {} [{:?}] nodes={} edges={}",
        graph.view,
        graph.status,
        graph.nodes.len(),
        graph.edges.len()
    );
    if graph.truncated {
        println!("truncated by resource limits");
    }
    // 层级树（parent 链；根 = None）。
    let children: std::collections::BTreeMap<Option<&String>, Vec<&prism_viz::VizNode>> = graph
        .nodes
        .iter()
        .fold(Default::default(), |mut map, node| {
            map.entry(node.parent.as_ref()).or_default().push(node);
            map
        });
    fn walk<'a>(
        parent: Option<&'a String>,
        children: &std::collections::BTreeMap<Option<&'a String>, Vec<&'a prism_viz::VizNode>>,
        depth: usize,
    ) {
        if depth > 6 {
            return;
        }
        if let Some(nodes) = children.get(&parent) {
            for node in nodes {
                println!(
                    "{}{} [{}]",
                    "  ".repeat(depth),
                    node.label,
                    node.kind.as_str()
                );
                walk(Some(&node.id), children, depth + 1);
            }
        }
    }
    walk(None, &children, 0);
    for diagnostic in &graph.diagnostics {
        println!("diag: {diagnostic}");
    }
}

fn fail(command: &str, err: &str) -> i32 {
    eprintln!("prism: {command} failed: {err}");
    crate::EXIT_ERROR
}
