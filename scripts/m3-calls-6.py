# -*- coding: utf-8 -*-
"""M3 call graph: model + resolution + service wiring."""

import io

# 1) model.rs: CallEdge + result field + metric
p = "crates/prism-graph/src/model.rs"
s = io.open(p, encoding="utf-8", newline="\n").read()

anchor = "/// M3 对外 JSON 契约（规格 §33）。"
call_edge = """/// 一条静态调用边（规格 §19 可靠子集；仅直接调用，且 callee 解析成功时
/// 才为 Resolved；动态/未知名保持 Unresolved，不猜测）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CallEdge {
    /// 稳定 ID：`call:{caller}#{callee_name}#{line}:{column}`。
    pub id: String,
    /// 调用方符号（限定路径所属）。
    pub caller_id: prism_core::ids::SymbolId,
    pub caller_qualified: String,
    /// 原样 callee 文本（"helper" / "obj.method"）。
    pub callee_text: String,
    /// 调用名的最后一段（解析键）。
    pub callee_name: String,
    /// Resolved 时的被调符号。
    pub callee_id: Option<prism_core::ids::SymbolId>,
    pub callee_qualified: Option<String>,
    pub resolution: ResolutionStatus,
    pub confidence: Confidence,
    pub location: Option<SourceLocation>,
}

/// M3 对外 JSON 契约（规格 §33）。"""
assert anchor in s
s = s.replace(anchor, call_edge, 1)

old_result = """    /// 按稳定 ID 排序。
    pub edges: Vec<GraphEdge>,"""
new_result = """    /// 按稳定 ID 排序。
    pub edges: Vec<GraphEdge>,
    /// 静态调用边（符号级，规格 §19/§20），按稳定 ID 排序。
    #[serde(default)]
    pub calls: Vec<CallEdge>,"""
assert old_result in s
s = s.replace(old_result, new_result, 1)
io.open(p, "w", encoding="utf-8", newline="\n").write(s)
print("model ok")

# 2) lib.rs re-exports
p = "crates/prism-graph/src/lib.rs"
s = io.open(p, encoding="utf-8", newline="\n").read()
s = s.replace("""pub mod builder;
pub mod model;
pub mod modules;
pub mod resolve;
pub mod service;""", """pub mod builder;
pub mod calls;
pub mod model;
pub mod modules;
pub mod resolve;
pub mod service;""")
s = s.replace("""pub use model::{
    Cycle, Dependency, DependencyGraphResult, DependencyKind, GraphEdge, GraphEdgeKind,
    GraphMetrics, GraphNode, GraphNodeKind, Module, ModuleKind, ResolutionDiagnostic,
    ResolutionStatus, GRAPH_SCHEMA_VERSION,
};""", """pub use model::{
    CallEdge, Cycle, Dependency, DependencyGraphResult, DependencyKind, GraphEdge,
    GraphEdgeKind, GraphMetrics, GraphNode, GraphNodeKind, Module, ModuleKind,
    ResolutionDiagnostic, ResolutionStatus, GRAPH_SCHEMA_VERSION,
};""")
s = s.replace("pub use resolve::{resolve_import, ResolutionContext};",
              "pub use calls::resolve_calls;\npub use resolve::{resolve_import, ResolutionContext};")
io.open(p, "w", encoding="utf-8", newline="\n").write(s)
print("lib ok")
