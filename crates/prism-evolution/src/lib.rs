//! # Prism Git Intelligence & Codebase Evolution（2.0-M211）
//!
//! 把 Git 历史转换成统一工程事实：`Git → Commit → FileChange →
//! EvolutionFact / Hotspot / ChangeCoupling`。
//!
//! 边界（§1.2/§4.1/§25/§26/§42）：
//! - **Git 是证据来源之一，不是真理本身**：churn/频率是 INFERENCE 级
//!   工程事实，原始指标优先，不造 Hotspot Score 魔法数字（§25）。
//! - **Co-change ≠ 语义依赖**（§26/§27）：CO_CHANGES_WITH 是历史关系，
//!   与 REFERENCES/CALLS/DEPENDS_ON 分离。
//! - **Read-only**（§42）：无 push/pull/checkout 等任何写操作；全部经
//!   M4 GitProvider 的安全命令面（无任意 shell，§41）。
//! - **Bounded**（§47）：一切历史分析有 max_commits/top/pair 上限，
//!   窗口语义 = `recent:<n>-commits`（§52 显式声明，不用模糊的"最近"）。
//! - **不重建模型**（§0/§78）：复用 M4 GitProvider/GitCommit；Change→
//!   M26 语义模型经 M26 change_mapping（batch 2）。

pub mod analysis;
pub mod model;
pub mod symbol_history;

pub use analysis::{
    build_report, change_couplings, file_evolution, hotspots, scan_repo, EvolutionQuery,
};
pub use model::{
    ChangeCoupling, EvolutionReport, FileEvolution, HistoryWindow, Hotspot,
    EVOLUTION_SCHEMA_VERSION,
};
pub use symbol_history::{
    symbol_history, SymbolChange, SymbolChangeKind, SymbolHistory, SymbolHistoryEntry,
};
