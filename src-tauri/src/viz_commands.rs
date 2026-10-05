//! M212 Visualization 投影命令（§39 VisualizationQueryService 的 Tauri 面）。
//!
//! 只做**投影**：把 M1/M3/M23/M24/M29 既有产物映射为
//! `prism_viz::VizGraph`；不重新分析（deps 解析等均为既有引擎的
//! bounded 复用）。缺数据 ⇒ 图内 Unknown/Partial + 诊断（§46/§67），
//! 不返回错误掩盖能力缺口。

use crate::state::{lock_session, AppState};
use prism_core::task::CancellationToken;
use prism_viz::{VizGraph, VizLimits};
use tauri::State;

/// 按 view 构建投影。同步执行（各投影均为亚秒级 bounded 计算）。
#[tauri::command]
pub fn get_visualization_graph(
    state: State<'_, AppState>,
    view: String,
    path: Option<String>,
) -> Result<VizGraph, String> {
    let cancel = CancellationToken::new();
    match view.as_str() {
        "architecture" => {
            let session = lock_session(&state.session);
            let Some(discovery) = session.discovery.clone() else {
                return Ok(VizGraph::new("architecture").finalize(&VizLimits::default()));
            };
            let files: Vec<String> = discovery
                .files
                .iter()
                .filter(|file| file.analyzable)
                .map(|file| file.relative_path.clone())
                .collect();
            let input = prism_viz::architecture::ArchitectureInput {
                files: &files,
                dependencies: session
                    .graph_model
                    .as_ref()
                    .ok_or_else(|| "dependency analysis has not run yet".to_string())?,
            };
            Ok(prism_viz::architecture_graph(&input, &VizLimits::default()))
        }
        "godot" => {
            let root = resolve_root(&state, path.as_deref())?;
            let analysis = godot_analysis(&root, &cancel)?;
            let index = symbol_index(&root, &cancel);
            Ok(prism_viz::godot_graph(
                &analysis,
                Some(&index),
                &VizLimits::default(),
            ))
        }
        "dependency" => {
            let root = resolve_root(&state, path.as_deref())?;
            let supply = supply_report(&root, &cancel)?;
            Ok(prism_viz::dependency_graph(&supply, &VizLimits::default()))
        }
        other => Err(format!("unknown visualization view: {other}")),
    }
}

fn resolve_root(
    state: &State<'_, AppState>,
    path: Option<&str>,
) -> Result<std::path::PathBuf, String> {
    if let Some(path) = path {
        return Ok(std::path::PathBuf::from(path));
    }
    let session = lock_session(&state.session);
    session
        .project
        .as_ref()
        .map(|project| project.root_path.clone())
        .ok_or_else(|| "no project open".to_string())
}

/// M24 Godot 分析（bounded，复用 intel 域同款 inventory→analyze 管线）。
fn godot_analysis(
    root: &std::path::Path,
    cancel: &CancellationToken,
) -> Result<prism_godot::GodotAnalysis, String> {
    let discovery = prism_project::DiscoveryService::new()
        .discover(root, prism_project::ScanOptions::default(), cancel, |_| {})
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
    let project = prism_project::Project::from_root(root).map_err(|err| err.to_string())?;
    let context = prism_intel::AnalysisContext::new(
        prism_intel::ProjectIdentity::from_project(&project),
        prism_intel::AnalysisScope::Domain("godot".to_string()),
        vec!["godot".to_string()],
        cancel.clone(),
    );
    let outcome = prism_godot::analyze(&inventory, &project.root_path, &context);
    outcome
        .data
        .ok_or_else(|| "godot analysis unavailable".to_string())
}

/// M23 符号索引（bounded .cs 读取，health2/debug 同款口径）。
fn symbol_index(root: &std::path::Path, cancel: &CancellationToken) -> prism_csharp::SymbolIndex {
    let mut engine = prism_csharp::SymbolEngine::new();
    if let Ok(discovery) = prism_project::DiscoveryService::new().discover(
        root,
        prism_project::ScanOptions::default(),
        cancel,
        |_| {},
    ) {
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
            if let Ok(source) = std::fs::read_to_string(root.join(&file.relative_path)) {
                engine.update_file(&file.relative_path, &source);
            }
        }
    }
    engine.build_index()
}

/// M29 供给报告（bounded manifest 读取 + secret 扫描面注入空——GUI 投影
/// 不做 secret 扫描，Dependency View 只需要声明/解析/license）。
fn supply_report(
    root: &std::path::Path,
    cancel: &CancellationToken,
) -> Result<prism_supply::SupplyReport, String> {
        let discovery = prism_project::DiscoveryService::new()
        .discover(root, prism_project::ScanOptions::default(), cancel, |_| {})
        .map_err(|err| err.to_string())?;
    let mut manifests: std::collections::BTreeMap<String, String> = std::collections::BTreeMap::new();
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
        if let Ok(content) = std::fs::read_to_string(root.join(&file.relative_path)) {
            manifests.insert(file.relative_path.clone(), content);
        }
    }
    let inputs = prism_supply::SupplyInputs {
        files: &manifests,
        text_files: &[],
        observed_tools: &std::collections::BTreeMap::new(),
        advisory_provider: None,
        license_policy: None,
        secret_limits: Box::leak(Box::new(prism_supply::secrets::SecretScanLimits::default())),
    };
    Ok(prism_supply::collect(&inputs))
}
