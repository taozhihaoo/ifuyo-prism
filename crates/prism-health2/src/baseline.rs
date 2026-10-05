//! 生命周期对比（2.0-M28 §26/§31/§33）：当前报告 vs 基线报告 ⇒
//! New / Persistent / Resolved / Regressed / Suppressed。
//!
//! 实现唯一在 `model::diff_against_baseline`（§33 语义：Regressed =
//! 基线 New 未修复确认，经 `model::mark_regressed` 补录；Suppressed
//! 不参与 New/Persistent/Resolved）。

pub use crate::model::{diff_against_baseline, mark_regressed, LifecycleDiff};
