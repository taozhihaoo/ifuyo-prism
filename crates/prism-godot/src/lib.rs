//! # Prism Godot Project Adapter（2.0-M24）
//!
//! Prism 第一个项目语义 Adapter：project.godot / .tscn / .tres 静态解析 →
//! Godot Project / Scene / Node / Script / Resource / Autoload 实体 +
//! attached_script / backed_by / signal_connects / loads 关系 → M21 IntelBundle。
//!
//! 边界（§一/§79）：
//! - Core 零 Godot 依赖——Godot 种类全部 `EntityKind::other("godot.*")`；
//! - 不复制 M23 符号——Script `backed_by` C# Symbol（Source of Truth 在 M23）；
//! - res:// ↔ 项目相对路径映射只在 Adapter 层（§40/§41）；
//! - 静态分析：不启动 Godot Editor、无 shell/网络（§52/§70）；
//! - NodePath 三态 Resolved/Unresolved/Ambiguous——不强行猜目标（§28）；
//! - 坏文件 ⇒ 诊断 + 继续（§46）；场景环 ⇒ finding + visited-set 防递归（§44）。

pub mod adapter;
pub mod intel;
pub mod ir;
pub mod parse;

pub use adapter::{
    analyze, normalize_node_path, relative_to_res, res_to_relative, GodotAnalysis, GodotIndex,
    MAX_GODOT_FILES, MAX_GODOT_FILE_BYTES,
};
pub use intel::godot_to_intel;
pub use ir::{
    AutoloadEntry, ConnectionRecord, ExtResourceRecord, GodotProjectFile, NodeRecord,
    SubResourceRecord, TscnDocument,
};

/// M24 输出契约版本（§61 分版本演进）。
pub const GODOT_ADAPTER_VERSION: u32 = 1;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::GodotIndex;
    use prism_core::ids::ProjectId;
    use prism_core::task::CancellationToken;
    use prism_intel::{AnalysisContext, ProjectIdentity};
    use std::path::Path;

    const PROJECT_GODOT: &str = r#"config_version=5

[application]
config/name="Demo"
config/features=PackedStringArray("4.3", "C#")

[autoload]
GameState="*res://Scripts/GameState.cs"
"#;

    const BATTLE_TSCN: &str = r#"[gd_scene load_steps=2 format=3]

[ext_resource type="Script" path="res://Scripts/Player.cs" id="1_p"]

[node name="Battle" type="Node2D"]
[node name="Player" type="CharacterBody2D" parent="."]
script = ExtResource("1_p")
[node name="UI" type="CanvasLayer" parent="."]
[node name="Button" type="Button" parent="UI"]

[connection signal="pressed" from="UI/Button" to="." method="_on_pressed"]
"#;

    const PLAYER_CS: &str = r#"namespace Game;

public partial class Player
{
    [Export]
    public int Speed;

    protected void _OnReady() { }
}
"#;

    fn make_project() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "prism-godot-m24-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.subsec_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("Scenes")).unwrap();
        std::fs::create_dir_all(dir.join("Scripts")).unwrap();
        std::fs::write(dir.join("project.godot"), PROJECT_GODOT).unwrap();
        std::fs::write(dir.join("Scenes/Battle.tscn"), BATTLE_TSCN).unwrap();
        std::fs::write(dir.join("Scripts/Player.cs"), PLAYER_CS).unwrap();
        dir
    }

    fn inventory(dir: &Path) -> Vec<(String, String)> {
        let mut out = Vec::new();
        for entry in walk(dir, dir) {
            out.push((entry.clone(), "godot".to_string()));
        }
        out
    }

    fn walk(root: &Path, dir: &Path) -> Vec<String> {
        let mut out = Vec::new();
        let Ok(entries) = std::fs::read_dir(dir) else {
            return out;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                out.extend(walk(root, &path));
            } else if let Ok(relative) = path.strip_prefix(root) {
                out.push(relative.to_string_lossy().replace('\\', "/"));
            }
        }
        out
    }

    /// §61 第一个 Vertical Slice：project.godot → Autoload → Scene → Node
    /// → Script → C# Symbol 全链进入 IntelBundle。
    #[test]
    fn vertical_slice_scene_node_script_symbol() {
        let dir = make_project();
        let identity = ProjectIdentity::from_project(&prism_project_like(&dir));
        let context = AnalysisContext::new(
            identity.clone(),
            prism_intel::AnalysisScope::FullProject,
            vec!["godot".into()],
            CancellationToken::new(),
        );
        let files = inventory(&dir);
        let analysis = analyze(&files, &dir, &context).data.unwrap();
        assert!(analysis.is_godot_project());
        assert!(analysis.scenes.contains_key("Scenes/Battle.tscn"));

        let index = GodotIndex::build(&analysis);
        assert!(index.node_by_path("/Battle/Player").is_some());
        assert_eq!(
            index.script_for_node("/Battle/Player").map(String::as_str),
            Some("Scripts/Player.cs")
        );

        // M23 符号（真实链路：脚本文件 → class Player）。
        let mut engine = prism_csharp::SymbolEngine::new();
        engine.update_file("Scripts/Player.cs", PLAYER_CS);
        let csharp = engine.build_index();

        let outcome = godot_to_intel(&analysis, &index, Some(&csharp), &identity, "t");
        let bundle = outcome.data.unwrap();

        // Autoload 链：project → autoload → script。
        assert!(bundle
            .relations
            .values()
            .any(|r| r.relation_type.as_str() == "loads"));
        assert!(bundle
            .entities
            .values()
            .any(|e| e.kind.as_str() == "godot.autoload"));
        // Scene → Node 树。
        assert!(bundle
            .relations
            .values()
            .any(|r| r.relation_type == prism_intel::RelationType::contains()));
        // attached_script：node → script。
        assert!(bundle
            .relations
            .values()
            .any(|r| r.relation_type.as_str() == "attached_script"));
        // backed_by：script → C# class symbol（INFERENCE）。
        let backed = bundle
            .relations
            .values()
            .find(|r| r.relation_type.as_str() == "backed_by")
            .expect("script must be backed by C# class");
        assert_eq!(backed.origin, prism_intel::Origin::Inference);
        // signal connection：from/to 均可解析（to="." = root）。
        assert!(bundle
            .relations
            .values()
            .any(|r| r.relation_type.as_str() == "signal_connects"));
        // godot 事实 + 检测事实。
        assert!(bundle
            .facts
            .values()
            .any(|f| f.fact_type.as_str() == "godot_project_detected"));
        // 序列化确定性（§47）。
        let text = serde_json::to_string(&bundle).unwrap();
        let back: prism_intel::IntelBundle = serde_json::from_str(&text).unwrap();
        assert_eq!(back, bundle);
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn prism_project_like(dir: &Path) -> prism_project::Project {
        prism_project::Project::from_root(dir).unwrap()
    }

    #[test]
    fn non_godot_project_is_honest_empty() {
        let identity = ProjectIdentity::from_root(ProjectId::from_string("proj:x"), "C:/x");
        let context = AnalysisContext::new(
            identity.clone(),
            prism_intel::AnalysisScope::FullProject,
            vec!["godot".into()],
            CancellationToken::new(),
        );
        let analysis = analyze(&[], Path::new("C:/x"), &context).data.unwrap();
        let index = GodotIndex::build(&analysis);
        let outcome = godot_to_intel(&analysis, &index, None, &identity, "t");
        let bundle = outcome.data.unwrap();
        assert!(
            !bundle
                .facts
                .values()
                .any(|f| f.fact_type.as_str() == "godot_project_detected"),
            "非 Godot 项目不得伪造检测事实"
        );
    }

    #[test]
    fn broken_tscn_is_diagnostic_not_failure() {
        let dir = make_project();
        std::fs::write(dir.join("Scenes/Broken.tscn"), "[node name=\"X\" type=\n").unwrap();
        let context = AnalysisContext::new(
            ProjectIdentity::from_project(&prism_project_like(&dir)),
            prism_intel::AnalysisScope::FullProject,
            vec!["godot".into()],
            CancellationToken::new(),
        );
        let files = inventory(&dir);
        let outcome = analyze(&files, &dir, &context);
        // 失败隔离（§46）：坏场景是诊断，项目分析仍 Complete。
        assert_eq!(outcome.status, prism_intel::IntelStatus::Complete);
        assert!(outcome.warnings.iter().any(|w| w.contains("Broken.tscn")));
        assert!(outcome
            .data
            .unwrap()
            .scenes
            .contains_key("Scenes/Battle.tscn"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn incremental_result_equals_clean_full_result() {
        // §66 最后一条 invariant：Incremental == Fresh Full。
        let dir = make_project();
        let files_a = inventory(&dir);
        let context = AnalysisContext::new(
            ProjectIdentity::from_project(&prism_project_like(&dir)),
            prism_intel::AnalysisScope::FullProject,
            vec!["godot".into()],
            CancellationToken::new(),
        );
        let full = analyze(&files_a, &dir, &context).data.unwrap();

        // “增量”：逐文件喂同一 adapter（等价于只重析变化文件）。
        let mut incremental = GodotAnalysis::default();
        for (relative_path, _) in &files_a {
            let absolute = dir.join(relative_path);
            let content = std::fs::read_to_string(&absolute).unwrap();
            if relative_path == "project.godot" {
                incremental.project = Some(crate::parse::parse_project_godot(&content));
            } else if relative_path.ends_with(".tscn") {
                incremental.scenes.insert(
                    relative_path.clone(),
                    crate::parse::parse_tscn(&content, false),
                );
            } else if relative_path.ends_with(".tres") {
                incremental.resources.insert(
                    relative_path.clone(),
                    crate::parse::parse_tscn(&content, true),
                );
            }
        }
        assert_eq!(full, incremental, "增量结果必须等于等价的全量结果");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
