//! # Prism Visualization（2.0-M212）
//!
//! Visualization **投影层**：把 M21-M211 既有工程事实投影为确定、有界的
//! View Model（`VisualizationGraph`），供 CLI / Tauri / GUI 共同消费。
//!
//! 边界（§2/§3/§86/§119）：
//! - **投影不是第二事实源**：node.id 携带原 Core 身份；edge 必须来自真实
//!   Relation/Dependency/Signal 数据；renderer 不得自行推断。
//! - **Unknown/Partial 一等公民**（§46/§67）：数据缺失 ⇒ 节点/图状态如实
//!   标注，绝不画成正常。
//! - **有界**（§28/§49）：一切投影有 max_nodes/depth 上限；超出 ⇒
//!   truncated + 诊断，不静默丢数据。
//! - **确定性**（§43/§44）：节点/边按 (kind, label, id) 稳定排序；同输入
//!   同投影。
//! - **零 IO**：输入全部由调用方注入（Tauri/CLI 组装既有引擎产物），
//!   本 crate 不扫描、不解析、不访问网络。

pub mod architecture;
pub mod dependency;
pub mod godot;
pub mod impact;
pub mod model;

pub use architecture::architecture_graph;
pub use dependency::dependency_graph;
pub use godot::godot_graph;
pub use impact::impact_graph;
pub use model::{
    VizEdge, VizGraph, VizLegendEntry, VizLimits, VizNode, VizNodeKind, VizStatus,
    VIZ_SCHEMA_VERSION,
};
