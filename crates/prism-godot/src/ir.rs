//! Adapter 内部轻量 IR（§32）：解析结果 → 规范化 → 转 Core。
//! 不是持久化模型——Core 只看到 Entity/Fact/Relation/Evidence。

use serde::{Deserialize, Serialize};

/// project.godot 解析产物（§8：最小可靠元数据，不硬编码所有键）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct GodotProjectFile {
    pub config_version: Option<String>,
    pub project_name: Option<String>,
    pub main_scene: Option<String>,
    /// config/features 中的引擎版本线索（如 "4.3"）；无 ⇒ None（§33 不猜测）。
    pub engine_version_hint: Option<String>,
    /// `[autoload]` 段：名字 → 资源路径（`*` 前缀 = singleton，原样保留在 path）。
    pub autoloads: Vec<AutoloadEntry>,
    /// 是否完整解析（截断/损坏 ⇒ false + 诊断）。
    pub well_formed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutoloadEntry {
    pub name: String,
    /// 原样值（含可能的 `*` singleton 前缀）。
    pub value: String,
    /// 去掉 `*` 与 `res://` 后的项目相对路径。
    pub project_relative_path: String,
}

/// .tscn/.tres 段类型。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TscnSection {
    /// `[gd_scene ...]` / `[gd_resource ...]` 头。
    Header,
    ExtResource(ExtResourceRecord),
    SubResource(SubResourceRecord),
    Node(NodeRecord),
    Connection(ConnectionRecord),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtResourceRecord {
    pub resource_type: String,
    /// res:// 原样路径。
    pub path: String,
    pub id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubResourceRecord {
    pub resource_type: String,
    pub id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeRecord {
    pub name: String,
    /// 引擎节点类型（如 CharacterBody2D）；instance 节点可能缺省。
    pub node_type: Option<String>,
    /// parent 原样（"." / "UI" / "UI/HealthBar"）；根节点无 parent。
    pub parent: Option<String>,
    /// `instance=ExtResource("id")`（场景实例，§20）。
    pub instance_ext_resource_id: Option<String>,
    /// `script = ExtResource("id")`。
    pub script_ext_resource_id: Option<String>,
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectionRecord {
    pub signal: String,
    pub from: String,
    pub to: String,
    /// 目标方法名原样（§24：能否连到 C# Method 由映射层决定）。
    pub method: Option<String>,
    pub line: usize,
}

/// 单个 .tscn/.tres 文档 IR（§31：Adapter 内部，不进 Core）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TscnDocument {
    pub format: Option<String>,
    pub ext_resources: Vec<ExtResourceRecord>,
    pub sub_resources: Vec<SubResourceRecord>,
    pub nodes: Vec<NodeRecord>,
    pub connections: Vec<ConnectionRecord>,
    pub well_formed: bool,
    pub is_resource: bool,
}
