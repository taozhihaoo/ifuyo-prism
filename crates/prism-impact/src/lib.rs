//! prism-impact — Impact Facts 层（M6）。
//!
//! ```text
//! M5 ChangeSet (What changed?)
//!     × M3 relationship graph (facts, M3 owns)
//!     → ImpactAnalysis (What may be affected? Why? How far? How certain?)
//! ```
//!
//! 边界（规格 M6 §0/§132–§135）：
//!
//! - 输出 Potential Impact，绝不是 Guaranteed Impact：
//!   "存在依赖关系" ≠ "必然受到影响"
//! - Changed 集合与 Impacted 候选集合严格分离（§48/§49）
//! - 未解析/歧义关系保持不确定，不伪造目标（§5/§102/§103）
//! - 无 AI、无网络、无运行时 tracing、无数据库、无 git 变更（§117–§120、§142）
//! - 不做健康评分/架构评价/语义 bug 检测（M7 边界，§65/§66/§67）
//!
//! 契约（§126/§79）：serde snake_case、字段稳定、确定性排序、
//! `analysis_completeness` 显式表达 Complete/Partial/Truncated/Failed。

pub mod model;
pub mod render;
pub mod service;
pub mod traversal;

pub use model::{
    ImpactAnalysis, ImpactCompleteness, ImpactConfidence, ImpactDiagnostic, ImpactDiagnosticKind,
    ImpactKind, ImpactOptions, ImpactPath, ImpactReason, ImpactSummary, ImpactableNode, NodeKind,
    PathHop, RelationKind, RiskSignal, RiskSignalKind, IMPACT_SCHEMA_VERSION,
};
pub use render::{render_markdown, render_text};
pub use service::ImpactAnalysisService;
pub use traversal::{build_seeds, traverse, ImpactedAccumulator, NodeKey, ReverseEdge, ReverseIndex, Seed, TraversalOutput};
