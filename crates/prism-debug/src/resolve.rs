//! StackFrame → Symbol / Scene 关联（M210 §十一/§十二）。
//!
//! 优先级（§十一）：项目相对路径 + 方法名 + 行号落在声明范围 ⇒ Exact
//! (FACT)；仅名称全局唯一命中 ⇒ Probable (INFERENCE)；多个候选 ⇒
//! Ambiguous；引擎未注入 ⇒ Unavailable。Godot：脚本文件 → 挂接节点 →
//! 场景（**仅** M24 attached_script 证据，§十二 禁止文件名/类名猜测）。

use crate::model::{FrameResolution, MatchState, ResolutionStatus, StackFrame};
use prism_csharp::SymbolIndex;
use prism_godot::GodotIndex;
use std::path::Path;

/// 帧路径 → 项目相对路径（绝对路径剥根、反斜杠转斜杠）。
pub fn to_project_relative(file_path: &str, project_root: &Path) -> String {
    // Godot 资源路径（res://）即项目相对路径（§12 M24 口径）。
    if let Some(rest) = file_path.strip_prefix("res://") {
        return rest.to_string();
    }
    let normalized = file_path.replace('\\', "/");
    let root = project_root.to_string_lossy().replace('\\', "/");
    let root = root.trim_end_matches('/');
    if let Some(rest) = normalized.strip_prefix(&format!("{root}/")) {
        return rest.to_string();
    }
    // Windows 大小写差异的根前缀（E:/ vs e:/）。
    if normalized.len() > root.len() && normalized[..root.len()].eq_ignore_ascii_case(root) {
        if let Some(rest) = normalized
            .get(root.len()..)
            .and_then(|s| s.strip_prefix('/'))
        {
            return rest.to_string();
        }
    }
    normalized
}

/// 解析全部帧（index 未注入 ⇒ Unavailable，不伪造）。
pub fn resolve_frames(
    frames: &mut [StackFrame],
    index: Option<&SymbolIndex>,
    godot: Option<&GodotIndex>,
    project_root: &Path,
) -> usize {
    let mut resolved = 0usize;
    for frame in frames {
        frame.resolution = resolve_one(frame, index, godot, project_root);
        if frame.resolution.status == ResolutionStatus::Resolved {
            resolved += 1;
        }
    }
    resolved
}

fn resolve_one(
    frame: &StackFrame,
    index: Option<&SymbolIndex>,
    godot: Option<&GodotIndex>,
    project_root: &Path,
) -> FrameResolution {
    let Some(index) = index else {
        return FrameResolution {
            status: ResolutionStatus::Unavailable,
            match_state: MatchState::Unsupported,
            symbol_id: None,
            scene: None,
            node_path: None,
            reasoning: "no symbol index injected; resolution unavailable (still valid debug data)"
                .to_string(),
        };
    };
    let relative = frame
        .file_path
        .as_deref()
        .map(|path| to_project_relative(path, project_root));
    // 1) 文件 + 方法（+行号范围）⇒ Exact/Strong。
    if let Some(relative) = &relative {
        let candidates = index.by_file.get(relative).cloned().unwrap_or_default();
        let by_method: Vec<&String> = candidates
            .iter()
            .filter(|symbol_id| {
                frame
                    .method
                    .as_deref()
                    .map(|method| {
                        index
                            .by_id
                            .get(*symbol_id)
                            .map(|symbol| {
                                symbol.name == method || symbol.qualified_name.ends_with(method)
                            })
                            .unwrap_or(false)
                    })
                    .unwrap_or(false)
            })
            .collect();
        if by_method.len() == 1 {
            let symbol = index
                .by_id
                .get(by_method[0].as_str())
                .expect("id from index");
            let line_in_range = frame.line.map(|line| {
                symbol.declarations.iter().any(|declaration| {
                    (declaration.range.start_line..=declaration.range.end_line).contains(&{ line })
                })
            });
            let (status, match_state) = match line_in_range {
                Some(true) => (ResolutionStatus::Resolved, MatchState::Exact),
                Some(false) | None => (ResolutionStatus::Resolved, MatchState::Strong),
            };
            let (scene, node_path) = godot_scene_for(relative, godot);
            return FrameResolution {
                status,
                match_state,
                symbol_id: Some(symbol.id.clone()),
                scene,
                node_path,
                reasoning: format!(
                    "file `{relative}` + method `{}` matched single M23 symbol{}",
                    frame.method.as_deref().unwrap_or(""),
                    if line_in_range == Some(true) {
                        " with line inside declaration range"
                    } else {
                        ""
                    }
                ),
            };
        }
        if by_method.len() > 1 {
            return FrameResolution {
                status: ResolutionStatus::Ambiguous,
                match_state: MatchState::Ambiguous,
                symbol_id: None,
                scene: None,
                node_path: None,
                reasoning: format!(
                    "{} same-name symbols in `{relative}` (overload/partial); not guessing",
                    by_method.len()
                ),
            };
        }
    }
    // 2) 仅方法名全局唯一 ⇒ Probable（INFERENCE）。
    if let Some(method) = &frame.method {
        // M23 by_name 键为小写（§41 索引口径）。
        let by_name = index
            .by_name
            .get(&method.to_ascii_lowercase())
            .cloned()
            .unwrap_or_default();
        if by_name.len() == 1 {
            let symbol = index.by_id.get(by_name[0].as_str()).expect("id from index");
            return FrameResolution {
                status: ResolutionStatus::Resolved,
                match_state: MatchState::Probable,
                symbol_id: Some(symbol.id.clone()),
                scene: None,
                node_path: None,
                reasoning: format!(
                    "method name `{method}` unique in project (INFERENCE: no file evidence)"
                ),
            };
        }
        if by_name.len() > 1 {
            return FrameResolution {
                status: ResolutionStatus::Ambiguous,
                match_state: MatchState::Ambiguous,
                symbol_id: None,
                scene: None,
                node_path: None,
                reasoning: format!("{} candidates named `{method}` project-wide", by_name.len()),
            };
        }
    }
    // 3) Godot 脚本关联（有文件证据但无符号命中时仍可挂场景）。
    if let Some(relative) = relative.as_deref() {
        let (scene, node_path) = godot_scene_for(relative, godot);
        if scene.is_some() {
            return FrameResolution {
                status: ResolutionStatus::Unresolved,
                match_state: MatchState::Unresolved,
                symbol_id: None,
                scene,
                node_path,
                reasoning: "no M23 symbol matched, but M24 attached_script evidence maps file to scene/node".to_string(),
            };
        }
    }
    FrameResolution {
        status: ResolutionStatus::Unresolved,
        match_state: MatchState::Unresolved,
        symbol_id: None,
        scene: None,
        node_path: None,
        reasoning: "no file/method evidence matched local symbols".to_string(),
    }
}

/// 脚本文件 → (场景, 节点路径)：仅经 M24 `attached_script` 证据。
fn godot_scene_for(
    script_relative: &str,
    godot: Option<&GodotIndex>,
) -> (Option<String>, Option<String>) {
    let Some(godot) = godot else {
        return (None, None);
    };
    for (node_path, script) in &godot.node_scripts {
        if script == script_relative {
            if let Some(scene) = godot.node_paths.get(node_path) {
                return (Some(scene.clone()), Some(node_path.clone()));
            }
        }
    }
    (None, None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::StackFrame;
    use prism_csharp::SymbolEngine;

    fn frame(method: &str, file: Option<&str>, line: Option<u32>) -> StackFrame {
        StackFrame {
            index: 0,
            module: None,
            type_name: None,
            method: Some(method.to_string()),
            file_path: file.map(str::to_string),
            line,
            column: None,
            resolution: FrameResolution {
                status: ResolutionStatus::Unavailable,
                match_state: MatchState::Unsupported,
                symbol_id: None,
                scene: None,
                node_path: None,
                reasoning: String::new(),
            },
        }
    }

    fn index_with_battle() -> SymbolIndex {
        let source = "namespace App.Battle {\npublic class BattleController {\npublic void ResolveCard(string card) {\n// line 4\n}\npublic void Update() {\n}\n}\n}\n";
        let mut engine = SymbolEngine::new();
        engine.update_file("Scripts/Battle/BattleController.cs", source);
        engine.build_index()
    }

    #[test]
    fn exact_resolution_with_line_in_range() {
        let index = index_with_battle();
        let root = Path::new("E:/proj");
        let mut frames = vec![frame(
            "ResolveCard",
            Some("E:/proj/Scripts/Battle/BattleController.cs"),
            Some(4),
        )];
        let resolved = resolve_frames(&mut frames, Some(&index), None, root);
        assert_eq!(resolved, 1);
        assert_eq!(frames[0].resolution.status, ResolutionStatus::Resolved);
        assert_eq!(frames[0].resolution.match_state, MatchState::Exact);
        assert!(frames[0].resolution.symbol_id.is_some());
        assert!(frames[0].resolution.reasoning.contains("line inside"));
    }

    #[test]
    fn strong_without_line_then_ambiguous_overloads() {
        let index = index_with_battle();
        let root = Path::new("E:/proj");
        let mut frames = vec![frame(
            "Update",
            Some("E:/proj/Scripts/Battle/BattleController.cs"),
            None,
        )];
        resolve_frames(&mut frames, Some(&index), None, root);
        assert_eq!(
            frames[0].resolution.match_state,
            MatchState::Strong,
            "文件+方法命中但无行号"
        );
    }

    #[test]
    fn method_name_unique_is_probable_inference() {
        let probe = index_with_battle();
        eprintln!(
            "[probe] by_name ResolveCard = {:?}",
            probe.by_name.get("ResolveCard")
        );
        eprintln!(
            "[probe] all names = {:?}",
            probe.by_name.keys().collect::<Vec<_>>()
        );
        let index = index_with_battle();
        let root = Path::new("E:/proj");
        let mut frames = vec![frame("ResolveCard", None, None)];
        resolve_frames(&mut frames, Some(&index), None, root);
        assert_eq!(frames[0].resolution.match_state, MatchState::Probable);
        assert!(frames[0].resolution.reasoning.contains("INFERENCE"));
    }

    #[test]
    fn missing_index_is_unavailable_not_fake() {
        let mut frames = vec![frame("ResolveCard", Some("a.cs"), None)];
        let resolved = resolve_frames(&mut frames, None, None, Path::new("."));
        assert_eq!(resolved, 0);
        assert_eq!(frames[0].resolution.status, ResolutionStatus::Unavailable);
        assert!(frames[0].resolution.reasoning.contains("unavailable"));
    }

    #[test]
    fn godot_scene_mapping_requires_attached_script_evidence() {
        use prism_godot::ir::{ExtResourceRecord, NodeRecord, TscnDocument};
        use prism_godot::GodotAnalysis;
        let index = index_with_battle();
        let mut scenes = std::collections::BTreeMap::new();
        scenes.insert(
            "Scenes/Battle.tscn".to_string(),
            TscnDocument {
                format: Some("3".into()),
                ext_resources: vec![ExtResourceRecord {
                    resource_type: "Script".into(),
                    path: "res://Scripts/Battle/BattleController.cs".into(),
                    id: "1".into(),
                }],
                sub_resources: vec![],
                nodes: vec![NodeRecord {
                    name: "BattleController".into(),
                    node_type: Some("Node".into()),
                    parent: None,
                    instance_ext_resource_id: None,
                    script_ext_resource_id: Some("1".into()),
                    line: 1,
                }],
                connections: vec![],
                well_formed: true,
                is_resource: false,
            },
        );
        let analysis = GodotAnalysis {
            project: None,
            scenes,
            resources: Default::default(),
            diagnostics: vec![],
        };
        let godot = GodotIndex::build(&analysis);
        let root = Path::new("E:/proj");
        let mut frames = vec![frame(
            "ResolveCard",
            Some("E:/proj/Scripts/Battle/BattleController.cs"),
            Some(4),
        )];
        resolve_frames(&mut frames, Some(&index), Some(&godot), root);
        assert_eq!(
            frames[0].resolution.scene.as_deref(),
            Some("Scenes/Battle.tscn")
        );
        assert_eq!(
            frames[0].resolution.node_path.as_deref(),
            Some("/BattleController")
        );
    }
}
