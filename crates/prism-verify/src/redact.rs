//! Secret 脱敏（规格 M8 §77/§173–§176）。
//!
//! 实现位于 prism-core 的中央隐私层（M9 §346：各里程碑复用同一套）；
//! 本模块仅保留兼容再导出，M8 原有调用路径不变。

pub use prism_core::privacy::{command_display, redact_args, redact_text};
