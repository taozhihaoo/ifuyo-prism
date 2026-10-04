//! Godot Adapter 分析与查询（2.0-M24 §41-§49/§61-§63/§四十八）。
//!
//! 输入 = M22 L0 清单 + 文件内容；输出 = IR → IntelBundle。
//! res:// ↔ 项目相对路径双向映射在本层完成，res:// 不进 Core 身份（§40/§41）。

use crate::ir::{GodotProjectFile, TscnDocument};
use prism_intel::{AnalysisContext, IntelStatus};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// 资源限额（§45/§72）。
pub const MAX_GODOT_FILES: usize = 5_000;
pub const MAX_GODOT_FILE_BYTES: u64 = 1_048_576;

/// Godot 域发现结果（IR 层聚合；transient，不持久化 §60）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct GodotAnalysis {
    /// 找到 project.godot 才为 Some（§7：不做 exists==Godot 的唯一依据——
    /// 但当前 1.0 支持面下 project.godot 是唯一可靠信号，诚实记录）。
    pub project: Option<GodotProjectFile>,
    /// 场景：项目相对路径（去 res:// 形态）→ 文档。
    pub scenes: BTreeMap<String, TscnDocument>,
    /// .tres 资源文档。
    pub resources: BTreeMap<String, TscnDocument>,
    /// 诊断（§46/§75：坏文件不拖垮项目）。
    pub diagnostics: Vec<String>,
}

impl GodotAnalysis {
    pub fn is_godot_project(&self) -> bool {
        self.project.is_some()
    }
}

/// 扫描 L0 清单中的 Godot 文件并解析（失败隔离 §46；取消 §68）。
pub fn analyze(
    inventory: &[(String, String)],
    project_root: &Path,
    ctx: &AnalysisContext,
) -> IntelOutcome2<GodotAnalysis> {
    let mut warnings = Vec::new();
    let mut analysis = GodotAnalysis::default();
    let mut visited: BTreeMap<String, ()> = BTreeMap::new();
    let mut count = 0usize;

    for (relative_path, _kind) in inventory {
        if ctx.cancel.is_cancelled() {
            return IntelOutcome2::cancelled(Some(analysis), warnings);
        }
        let is_godot_file = relative_path == "project.godot"
            || relative_path.ends_with(".tscn")
            || relative_path.ends_with(".tres")
            || relative_path.ends_with(".res");
        if !is_godot_file || visited.contains_key(relative_path) {
            continue;
        }
        if count >= MAX_GODOT_FILES {
            warnings.push("godot file cap reached; remaining files skipped".to_string());
            break;
        }
        visited.insert(relative_path.clone(), ());
        count += 1;
        let absolute = project_root.join(relative_path);
        let Ok(metadata) = std::fs::metadata(&absolute) else {
            continue;
        };
        if metadata.len() > MAX_GODOT_FILE_BYTES {
            warnings.push(format!("{relative_path} exceeds size cap; skipped"));
            continue;
        }
        let Ok(content) = std::fs::read_to_string(&absolute) else {
            warnings.push(format!("{relative_path} unreadable; skipped"));
            continue;
        };
        if relative_path == "project.godot" {
            let project = crate::parse::parse_project_godot(&content);
            if !project.well_formed {
                warnings.push("project.godot has unparseable lines (MalformedProject)".to_string());
            }
            analysis.project = Some(project);
        } else if relative_path.ends_with(".tscn") {
            let document = crate::parse::parse_tscn(&content, false);
            if !document.well_formed {
                warnings.push(format!("{relative_path}: malformed section header"));
            }
            analysis.scenes.insert(relative_path.clone(), document);
        } else {
            let document = crate::parse::parse_tscn(&content, true);
            analysis.resources.insert(relative_path.clone(), document);
        }
    }

    if analysis.project.is_none() {
        return IntelOutcome2::complete(analysis);
    }

    // 场景引用环检测（§44）：visited-set DFS，不无限递归。
    for cycle in detect_scene_cycles(&analysis) {
        warnings.push(format!("SceneReferenceCycle: {cycle}"));
    }
    if warnings.is_empty() {
        IntelOutcome2::complete(analysis)
    } else {
        IntelOutcome2 {
            status: IntelStatus::Complete,
            data: Some(analysis),
            warnings,
            errors: Vec::new(),
        }
    }
}

/// 场景引用边（A 引用 B）与环检测（容量有限的确定性 DFS）。
fn detect_scene_cycles(analysis: &GodotAnalysis) -> Vec<String> {
    let mut edges: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (scene_path, document) in &analysis.scenes {
        for resource in &document.ext_resources {
            if resource.resource_type == "PackedScene" {
                let target = res_to_relative(&resource.path);
                edges.entry(scene_path.clone()).or_default().push(target);
            }
        }
    }
    let mut cycles = Vec::new();
    for start in edges.keys() {
        let mut stack = vec![(start.clone(), vec![start.clone()])];
        let mut local_visited: BTreeMap<String, ()> = BTreeMap::new();
        while let Some((current, path)) = stack.pop() {
            for next in edges.get(&current).into_iter().flatten() {
                if *next == *start {
                    cycles.push(path.join(" -> ") + " -> " + start);
                    break;
                }
                if local_visited.contains_key(next) || *next == *current {
                    continue;
                }
                local_visited.insert(next.clone(), ());
                let mut next_path = path.clone();
                next_path.push(next.clone());
                if next_path.len() <= 16 {
                    stack.push((next.clone(), next_path));
                }
            }
        }
        if cycles.len() >= 8 {
            break;
        }
    }
    cycles.sort();
    cycles.dedup();
    cycles
}

/// `res://X` → 项目相对路径 `X`（§41；Windows 分隔符归一化为 `/`）。
pub fn res_to_relative(res_path: &str) -> String {
    res_path
        .strip_prefix("res://")
        .unwrap_or(res_path)
        .replace('\\', "/")
}

/// 项目相对路径 → `res://X`。
pub fn relative_to_res(relative: &str) -> String {
    format!("res://{}", relative.replace('\\', "/"))
}

/// Godot 查询索引（§48；BTreeMap 确定性）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GodotIndex {
    pub project: Option<GodotProjectFile>,
    pub scenes: BTreeMap<String, TscnDocument>,
    pub resources: BTreeMap<String, TscnDocument>,
    /// 场景内 NodePath（`/Root/Parent/Child`）→ 所属场景。
    pub node_paths: BTreeMap<String, String>,
    /// 节点 → 挂接脚本（项目相对路径）。
    pub node_scripts: BTreeMap<String, String>,
    /// 未解析 NodePath 记录（§28：不强行猜目标）。
    pub unresolved_node_paths: Vec<String>,
    pub diagnostics: Vec<String>,
}

impl GodotIndex {
    /// 从分析结果构建（含 NodePath 解析与 script 挂接映射）。
    pub fn build(analysis: &GodotAnalysis) -> Self {
        let mut index = GodotIndex {
            project: analysis.project.clone(),
            scenes: analysis.scenes.clone(),
            resources: analysis.resources.clone(),
            ..GodotIndex::default()
        };
        index.diagnostics = analysis.diagnostics.clone();
        for (scene_path, document) in &analysis.scenes {
            let root_name = document
                .nodes
                .first()
                .filter(|n| n.parent.is_none())
                .map(|n| n.name.clone())
                .unwrap_or_default();
            for node in &document.nodes {
                let path = if node.parent.is_none() {
                    format!("/{root_name}")
                } else {
                    let parent_path = normalize_node_path(
                        &format!("/{root_name}"),
                        node.parent.as_deref().unwrap_or("."),
                    );
                    format!("{parent_path}/{}", node.name)
                };
                index.node_paths.insert(path.clone(), scene_path.clone());
                if let Some(script_id) = &node.script_ext_resource_id {
                    if let Some(resource) =
                        document.ext_resources.iter().find(|r| r.id == *script_id)
                    {
                        index
                            .node_scripts
                            .insert(path.clone(), res_to_relative(&resource.path));
                    }
                }
            }
        }
        index
    }

    pub fn scenes(&self) -> impl Iterator<Item = (&String, &TscnDocument)> {
        self.scenes.iter()
    }

    pub fn nodes_in_scene(&self, scene: &str) -> Vec<String> {
        self.node_paths
            .iter()
            .filter(|(_, scene_path)| scene_path.as_str() == scene)
            .map(|(path, _)| path.clone())
            .collect()
    }

    /// NodePath 查询（§48 get node by NodePath；跨场景路径需带场景上下文）。
    pub fn node_by_path(&self, path: &str) -> Option<&String> {
        self.node_paths.get(path)
    }

    pub fn script_for_node(&self, node_path: &str) -> Option<&String> {
        self.node_scripts.get(node_path)
    }

    pub fn nodes_using_script(&self, script_relative: &str) -> Vec<String> {
        self.node_scripts
            .iter()
            .filter(|(_, script)| script.as_str() == script_relative)
            .map(|(path, _)| path.clone())
            .collect()
    }

    pub fn connections(&self, scene: &str) -> Vec<&crate::ir::ConnectionRecord> {
        self.scenes
            .get(scene)
            .map(|document| document.connections.iter().collect())
            .unwrap_or_default()
    }
}

/// NodePath 归一化（§18：`.`/`..`/多段相对路径 → 绝对路径）。
pub fn normalize_node_path(base: &str, relative: &str) -> String {
    let mut segments: Vec<String> = base
        .split('/')
        .filter(|s| !s.is_empty() && *s != ".")
        .map(str::to_string)
        .collect();
    for segment in relative.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                segments.pop();
            }
            other => segments.push(other.to_string()),
        }
    }
    format!("/{}", segments.join("/"))
}

/// IntelOutcome 别名（避免直接依赖 M21 Result 泛型全路径）。
pub type IntelOutcome2<T> = prism_intel::IntelOutcome<T>;

#[allow(unused)]
fn unused_path_helper(_path: &Path) {}

#[cfg(test)]
mod tests {
    use super::*;
    use prism_core::ids::ProjectId;
    use prism_intel::AnalysisContext;
    use prism_intel::ProjectIdentity;

    fn ctx() -> AnalysisContext {
        AnalysisContext::new(
            ProjectIdentity::from_root(ProjectId::from_string("proj:g"), "C:/g"),
            prism_intel::AnalysisScope::FullProject,
            vec!["godot".into()],
            prism_core::task::CancellationToken::new(),
        )
    }

    const TSCN: &str = r#"[gd_scene load_steps=3 format=3]

[ext_resource type="Script" path="res://Scripts/Player.cs" id="1_p"]

[node name="Battle" type="Node2D"]
[node name="Player" type="CharacterBody2D" parent="."]
script = ExtResource("1_p")
[node name="UI" type="CanvasLayer" parent="."]
[node name="HealthBar" type="ProgressBar" parent="UI"]
"#;

    #[test]
    fn non_godot_project_recorded_honestly() {
        let outcome = analyze(&[], Path::new("C:/nowhere"), &ctx());
        let analysis = outcome.data.unwrap();
        assert!(!analysis.is_godot_project());
        assert!(analysis.scenes.is_empty());
    }

    #[test]
    fn node_paths_resolve_through_parent_chain() {
        let mut analysis = GodotAnalysis {
            project: Some(crate::parse::parse_project_godot(
                "config_version=5\n[application]\nconfig/name=\"G\"\n",
            )),
            ..GodotAnalysis::default()
        };
        analysis.scenes.insert(
            "Scenes/Battle.tscn".into(),
            crate::parse::parse_tscn(TSCN, false),
        );
        let index = GodotIndex::build(&analysis);
        assert!(index.node_by_path("/Battle/Player").is_some());
        assert!(index.node_by_path("/Battle/UI/HealthBar").is_some());
        assert_eq!(
            index.script_for_node("/Battle/Player").map(String::as_str),
            Some("Scripts/Player.cs")
        );
        assert!(index.script_for_node("/Battle/UI").is_none());
        // §48 查询：场景内节点 + 反查脚本使用者。
        assert_eq!(index.nodes_in_scene("Scenes/Battle.tscn").len(), 4);
        assert_eq!(
            index.nodes_using_script("Scripts/Player.cs"),
            vec!["/Battle/Player".to_string()]
        );
    }

    #[test]
    fn res_mapping_round_trip() {
        let relative = res_to_relative("res://Scripts/Player.cs");
        assert_eq!(relative, "Scripts/Player.cs");
        assert_eq!(relative_to_res(&relative), "res://Scripts/Player.cs");
        // Windows 分隔符归一化。
        assert_eq!(res_to_relative("res://A\\B.cs"), "A/B.cs");
    }

    #[test]
    fn normalize_handles_dot_dot_and_nested() {
        assert_eq!(
            normalize_node_path("/Battle/UI", "../Player"),
            "/Battle/Player"
        );
        assert_eq!(
            normalize_node_path("/Battle", "UI/HealthBar"),
            "/Battle/UI/HealthBar"
        );
        assert_eq!(normalize_node_path("/Battle", "."), "/Battle");
    }

    #[test]
    fn scene_reference_cycle_detected() {
        let mut analysis = GodotAnalysis {
            project: Some(crate::parse::parse_project_godot("config_version=5\n")),
            ..GodotAnalysis::default()
        };
        let scene_a = "[gd_scene format=3]\n[ext_resource type=\"PackedScene\" path=\"res://B.tscn\" id=\"1\"]\n[node name=\"A\"]\n[node name=\"B\" parent=\".\" instance=ExtResource(\"1\")]\n";
        let scene_b = "[gd_scene format=3]\n[ext_resource type=\"PackedScene\" path=\"res://A.tscn\" id=\"1\"]\n[node name=\"B\"]\n[node name=\"A\" parent=\".\" instance=ExtResource(\"1\")]\n";
        analysis
            .scenes
            .insert("A.tscn".into(), crate::parse::parse_tscn(scene_a, false));
        analysis
            .scenes
            .insert("B.tscn".into(), crate::parse::parse_tscn(scene_b, false));
        let cycles = detect_scene_cycles(&analysis);
        assert!(
            cycles.iter().any(|w| w.contains("A.tscn")),
            "cycle must be detected with evidence: {cycles:?}"
        );
    }

    #[test]
    fn cancelled_analysis_reports_cancelled() {
        let context = ctx();
        context.cancel.cancel();
        let inventory = vec![("Scenes/A.tscn".to_string(), "godot".to_string())];
        let outcome = analyze(&inventory, Path::new("C:/g"), &context);
        assert_eq!(outcome.status, prism_intel::IntelStatus::Cancelled);
    }
}
