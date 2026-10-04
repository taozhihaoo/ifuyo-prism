//! Godot 文本格式解析（§13/§31）：project.godot（最小 INI 段解析）与
//! .tscn/.tres（节头 + 属性行）。纯文本、静态、零执行（§52）。
//! 坏文件 ⇒ well_formed=false / 诊断，绝不让单文件失败拖垮项目（§46）。

use crate::ir::{
    AutoloadEntry, ConnectionRecord, ExtResourceRecord, GodotProjectFile, NodeRecord,
    SubResourceRecord, TscnDocument, TscnSection,
};

/// 解析 project.godot（§8：config_version/name/main_scene/features 版本线索/
/// autoload；其余键忽略——不建巨大模型）。
pub fn parse_project_godot(content: &str) -> GodotProjectFile {
    let mut project = GodotProjectFile {
        well_formed: true,
        ..GodotProjectFile::default()
    };
    let mut section = String::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with(';') {
            continue;
        }
        if let Some(name) = line
            .strip_prefix('[')
            .and_then(|rest| rest.strip_suffix(']'))
        {
            section = name.to_string();
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            // 无法解析的行（§78 MalformedProject 诊断来源）。
            project.well_formed = false;
            continue;
        };
        let key = key.trim();
        let value = unquote(value.trim());
        match (section.as_str(), key) {
            ("application", "config/name") => project.project_name = Some(value),
            ("application", "run/main_scene") => project.main_scene = Some(value),
            ("application", "config/features") => {
                project.engine_version_hint = extract_feature_version(&value);
            }
            ("", "config_version") => project.config_version = Some(value),
            ("autoload", _) => {
                if let Some(entry) = autoload_entry(key, &value) {
                    project.autoloads.push(entry);
                }
            }
            _ => {}
        }
    }
    project
}

fn autoload_entry(name: &str, value: &str) -> Option<AutoloadEntry> {
    let cleaned = value.trim_start_matches('*');
    let relative = cleaned.strip_prefix("res://")?;
    if relative.is_empty() {
        return None;
    }
    Some(AutoloadEntry {
        name: name.to_string(),
        value: value.to_string(),
        project_relative_path: relative.to_string(),
    })
}

/// config/features 值形如 PackedStringArray("4.3", "C#")——取首个 4.x/3.x 线索。
fn extract_feature_version(value: &str) -> Option<String> {
    // 值形如 PackedStringArray("4.3", "C#")——按引号切分找首个版本号 token。
    for token in value.split('"') {
        if token.starts_with("4.") || token.starts_with("3.") {
            return Some(token.to_string());
        }
    }
    None
}

fn unquote(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.len() >= 2 && trimmed.starts_with('"') && trimmed.ends_with('"') {
        trimmed[1..trimmed.len() - 1].to_string()
    } else {
        trimmed.to_string()
    }
}

/// 解析 .tscn/.tres（§13/§21）。行驱动：`[节头]` 单行 + 属性行；
/// 属性值为行内原文（多行数组按首行记录——限制记录于文档）。
pub fn parse_tscn(content: &str, is_resource: bool) -> TscnDocument {
    let mut document = TscnDocument {
        well_formed: true,
        is_resource,
        ..TscnDocument::default()
    };
    let mut current_node: Option<NodeRecord> = None;
    for (index, raw_line) in content.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with(';') {
            continue;
        }
        if line.starts_with('[') {
            // 新节头开始：冲刷上一个 node（其属性行已并入）。
            flush_node(&mut document, &mut current_node);
            match parse_section_header(line, index + 1) {
                Some(section) => match section {
                    TscnSection::Header => {
                        document.format = Some(line.to_string());
                    }
                    TscnSection::ExtResource(record) => document.ext_resources.push(record),
                    TscnSection::SubResource(record) => document.sub_resources.push(record),
                    TscnSection::Node(record) => current_node = Some(record),
                    TscnSection::Connection(record) => document.connections.push(record),
                },
                None => {
                    document.well_formed = false;
                }
            }
            continue;
        }
        // 属性行：归属当前 node（其余节的属性不进 IR——§32 控制规模）。
        if let (Some(node), Some((key, value))) = (current_node.as_mut(), split_property(line)) {
            match key.as_str() {
                "script" => {
                    node.script_ext_resource_id = ext_resource_ref(&value);
                }
                "instance" => {
                    node.instance_ext_resource_id = ext_resource_ref(&value);
                }
                _ => {}
            }
        }
    }
    flush_node(&mut document, &mut current_node);
    document
}

fn flush_node(document: &mut TscnDocument, current_node: &mut Option<NodeRecord>) {
    if let Some(node) = current_node.take() {
        document.nodes.push(node);
    }
}

fn split_property(line: &str) -> Option<(String, String)> {
    let (key, value) = line.split_once('=')?;
    Some((key.trim().to_string(), value.trim().to_string()))
}

/// 节头解析：`[kind key="value" ...]`。
fn parse_section_header(line: &str, line_number: usize) -> Option<TscnSection> {
    let inner = line.strip_prefix('[')?.strip_suffix(']')?;
    let mut parts = inner.split_whitespace();
    let kind = parts.next()?;
    let attributes = parse_attributes(inner);
    match kind {
        "gd_scene" | "gd_resource" => Some(TscnSection::Header),
        "ext_resource" => Some(TscnSection::ExtResource(ExtResourceRecord {
            resource_type: attributes.get("type").cloned().unwrap_or_default(),
            path: attributes.get("path").cloned().unwrap_or_default(),
            id: attributes.get("id").cloned().unwrap_or_default(),
        })),
        "sub_resource" => Some(TscnSection::SubResource(SubResourceRecord {
            resource_type: attributes.get("type").cloned().unwrap_or_default(),
            id: attributes.get("id").cloned().unwrap_or_default(),
        })),
        "node" => Some(TscnSection::Node(NodeRecord {
            name: attributes.get("name").cloned().unwrap_or_default(),
            node_type: attributes.get("type").cloned(),
            parent: attributes.get("parent").cloned(),
            // instance 是节头属性（§20 场景实例），非属性行。
            instance_ext_resource_id: attributes
                .get("instance")
                .and_then(|value| ext_resource_ref(value)),
            script_ext_resource_id: None,
            line: line_number,
        })),
        "connection" => Some(TscnSection::Connection(ConnectionRecord {
            signal: attributes.get("signal").cloned().unwrap_or_default(),
            from: attributes.get("from").cloned().unwrap_or_default(),
            to: attributes.get("to").cloned().unwrap_or_default(),
            method: attributes.get("method").cloned(),
            line: line_number,
        })),
        _ => None,
    }
}

/// 节头内 key="value" 属性（值内不含引号的 Godot 文本格式约定）。
fn parse_attributes(inner: &str) -> std::collections::BTreeMap<String, String> {
    let mut out = std::collections::BTreeMap::new();
    let mut rest = inner;
    while let Some(eq) = rest.find('=') {
        let key = rest[..eq]
            .trim()
            .rsplit(' ')
            .next()
            .unwrap_or("")
            .to_string();
        let after = rest[eq + 1..].trim_start();
        if let Some(value) = after.strip_prefix('"') {
            match value.find('"') {
                Some(end) => {
                    out.insert(key, value[..end].to_string());
                    rest = &value[end + 1..];
                }
                None => {
                    // 引号未闭合（§46 截断文件）：记录已得部分。
                    out.insert(key, value.to_string());
                    break;
                }
            }
        } else {
            // 非引号值（如 instance=ExtResource("id") 已在此处理之外的
            // 属性行走 split_property；节头内非引号值取到下一空格）。
            let token_end = after.find(' ').unwrap_or(after.len());
            out.insert(key, after[..token_end].to_string());
            if token_end + 1 > after.len() {
                break;
            }
            rest = &after[token_end + 1..];
        }
    }
    out
}

/// `ExtResource("id")` → id。
fn ext_resource_ref(value: &str) -> Option<String> {
    let inner = value.strip_prefix("ExtResource(")?.strip_suffix(')')?;
    Some(unquote(inner))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_godot_parses_metadata_and_autoloads() {
        let content = r#"; Engine configuration file.
config_version=5

[application]
config/name="SandBook"
run/main_scene="res://Scenes/Main.tscn"
config/features=PackedStringArray("4.3", "C#")

[autoload]
GameManager="*res://Scripts/Core/GameManager.cs"

[display]
window/size/viewport_width=1152
"#;
        let project = parse_project_godot(content);
        assert!(project.well_formed);
        assert_eq!(project.config_version.as_deref(), Some("5"));
        assert_eq!(project.project_name.as_deref(), Some("SandBook"));
        assert_eq!(
            project.main_scene.as_deref(),
            Some("res://Scenes/Main.tscn")
        );
        assert_eq!(project.engine_version_hint.as_deref(), Some("4.3"));
        assert_eq!(project.autoloads.len(), 1);
        assert_eq!(project.autoloads[0].name, "GameManager");
        assert_eq!(
            project.autoloads[0].project_relative_path,
            "Scripts/Core/GameManager.cs"
        );
    }

    #[test]
    fn malformed_project_godot_marks_not_well_formed() {
        let project = parse_project_godot("config_version=5\nthis line has no equals\n");
        assert!(!project.well_formed, "unparseable line must be recorded");
    }

    #[test]
    fn tscn_parses_nodes_resources_connections() {
        let content = r#"[gd_scene load_steps=4 format=3 uid="uid://abc"]

[ext_resource type="Script" path="res://Scripts/Player.cs" id="1_p"]
[ext_resource type="PackedScene" path="res://Scenes/Enemy.tscn" id="2_e"]

[node name="Battle" type="Node2D"]
[node name="Player" type="CharacterBody2D" parent="."]
script = ExtResource("1_p")
[node name="Enemy" parent="." instance=ExtResource("2_e")]
[node name="UI" type="CanvasLayer" parent="."]
[node name="HealthBar" type="ProgressBar" parent="UI"]

[connection signal="pressed" from="UI/Button" to="." method="_on_pressed"]
"#;
        let document = parse_tscn(content, false);
        assert!(document.well_formed);
        assert_eq!(document.ext_resources.len(), 2);
        assert_eq!(document.nodes.len(), 5);
        // script 属性挂到正确节点。
        let player = document.nodes.iter().find(|n| n.name == "Player").unwrap();
        assert_eq!(player.script_ext_resource_id.as_deref(), Some("1_p"));
        assert_eq!(player.parent.as_deref(), Some("."));
        let enemy = document.nodes.iter().find(|n| n.name == "Enemy").unwrap();
        assert_eq!(enemy.instance_ext_resource_id.as_deref(), Some("2_e"));
        assert_eq!(document.connections.len(), 1);
        assert_eq!(document.connections[0].signal, "pressed");
        assert_eq!(
            document.connections[0].method.as_deref(),
            Some("_on_pressed")
        );
    }

    #[test]
    fn truncated_tscn_records_unterminated_attribute() {
        let document = parse_tscn("[node name=\"Battle\" type=\"Node2D", false);
        assert!(!document.well_formed);
    }

    #[test]
    fn tres_document_records_resources_only() {
        let content = r#"[gd_resource type="Resource" format=3]

[resource]
script = ExtResource("1_r")
"#;
        let document = parse_tscn(content, true);
        assert!(document.is_resource);
        assert!(document.nodes.is_empty());
    }
}
