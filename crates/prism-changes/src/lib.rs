//! prism-changes — ChangeSet 层（M5，Change Facts）。
//!
//! 把 M4 的 Git diff facts 与 M2/M3 的代码事实连接为归一化变更集：
//!
//! ```text
//! GitDiff → FileChange → SymbolChange → ModuleChange → ChangeSet
//! ```
//!
//! 边界（规格 M5 §0/§56）：
//!
//! - 只回答 "What changed?"；"What may be affected?" 属于 M6
//! - ChangedSymbol ≠ ImpactedSymbol：不做依赖传播、不做风险/影响评分
//! - 证据驱动：每个 SymbolChange/ModuleChange 可追溯 file → hunk → range
//! - 保守映射：无法可靠归属时显式 Ambiguous/Unmapped，不猜
//! - 不依赖 AI/网络/遥测；历史源只读，绝不改写用户工作区
//!
//! 契约（规格 M5 §63）：serde snake_case、字段稳定、确定性排序、
//! 默认不含绝对路径。

pub mod diagnostic;
pub mod evidence;
pub mod file_change;
pub mod matcher;
pub mod model;
pub mod module_change;
pub mod revision;
pub mod service;
pub mod symbol_change;

pub use diagnostic::{ChangeDiagnostic, ChangeDiagnosticKind};
pub use evidence::ChangeEvidence;
pub use matcher::{compute_symbol_changes, ChangedLines, SymbolMatchInput};
pub use model::ChangedRange;
pub use file_change::{FileChange, FileChangeType};
pub use model::{
    ChangeScope, ChangeSet, ChangeSource, ChangeSummary, CHANGESET_SCHEMA_VERSION, MatchConfidence,
    MappingStatus,
};
pub use module_change::{ModuleChange, ModuleChangeType};
pub use revision::{
    build_side_model, CurrentWorkingTreeSource, FileModel, GitRevisionSource, RevisionSource,
    SideModel, SourceState,
};
pub use model::LineRange;
pub use service::{ChangeSetOptions, ChangeSetService};
pub use symbol_change::{SymbolChange, SymbolChangeType, SymbolCharacteristic};
