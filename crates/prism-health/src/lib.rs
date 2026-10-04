//! prism-health — Health Facts 层（M7）。
//!
//! ```text
//! M1-M6 事实（借用，不复制）→ HealthRule（确定性、只读上下文）
//!   → RuleResult{findings, diagnostics} → collect-sort-dedup → HealthAnalysis
//! ```
//!
//! 边界（规格 M7 §0/§4/§125–§128/§182–§188）：
//!
//! - **Findings first, score second—or never**：不产生 0-100 健康分
//! - 无 AI/网络/遥测/auto-fix/远程规则更新；完全离线可复现
//! - 不建持久化数据库；复用 M1-M6 已计算事实（§188）
//! - Severity 复用 M0 三级（Info/Warning/Critical），与 Confidence 独立（§11/§12）
//! - FindingId 确定性派生（§6/§198：M11 Snapshot 依赖稳定身份）

pub mod derived;
pub mod engine;
pub mod model;
pub mod registry;
pub mod render;
pub mod rules;

pub use derived::{DerivedFacts, SymbolMetrics};
pub use engine::{
    analyze, HealthAnalysis, HealthContext, HealthEngineStats, HealthOptions, HealthScope,
};
pub use model::{
    ChangeRelation, Evidence, EvidenceKind, Finding, FindingCategory, FindingExplanation,
    FindingId, FindingLocation, FindingStatus, HealthDiagnostic, HealthSummary, RuleResult,
    HEALTH_SCHEMA_VERSION,
};
pub use registry::{HealthRule, HealthRuleMeta, HealthRuleRegistry};
pub use render::{render_html, render_markdown, render_text};
pub use rules::maintenance::MaintenanceMarkerRule;
