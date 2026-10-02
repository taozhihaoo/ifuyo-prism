//! prism-graph — Dependency & Architecture 关系层（M3）。
//!
//! 在 M1 文件事实与 M2 代码事实之上建立**关系事实**：
//!
//! - [`model`]：Module / Dependency / GraphNode / GraphEdge / Cycle / Metrics
//! - [`modules`]：从 discovery + code model 构建模块层级
//!
//! - [`builder`]：统一图构建（节点/边/SCC 环检测/指标）与 DependencyGraphResult
//! - [`service`]：discovery + analysis + graph 的编排入口（CLI/GUI 同源）
//!
//! 原则：Deterministic First / Evidence First / Resolve When Possible /
//! Explicit Uncertainty。指标是事实不是评价；环只是记录，不做健康判断（M7）。

pub mod builder;
pub mod model;
pub mod modules;
pub mod resolve;
pub mod service;

pub use builder::build_graph;
pub use model::{
    Cycle, Dependency, DependencyGraphResult, DependencyKind, GraphEdge, GraphEdgeKind,
    GraphMetrics, GraphNode, GraphNodeKind, Module, ModuleKind, ResolutionDiagnostic,
    ResolutionStatus, GRAPH_SCHEMA_VERSION,
};
pub use modules::build_modules;
pub use prism_core::ids::ModuleId;
pub use resolve::{resolve_import, ResolutionContext};
pub use service::DependencyService;
