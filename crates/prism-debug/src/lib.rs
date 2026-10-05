//! # Prism Debug Intelligence（2.0-M210）
//!
//! Debug 数据的事实供给层：`Raw Log → Parser → DebugEvent → DebugError
//! （稳定指纹）→ ErrorCluster → StackFrame → M23 Symbol / M24 Scene →
//! M28 Finding`。
//!
//! 边界（§二/§三/§二十三/§六十七）：
//! - **不建第二套模型**：Finding 走 M28 RuleRegistry（`rules` 挂
//!   health2 侧）；Evidence/Entity 沿 M21；脱敏复用 M29
//!   `secrets::redact_line`；不重新实现 Secret Scanner。
//! - **Unknown 一等公民**：没有日志 ≠ 没有错误（§二.7）；Unsupported
//!   parser ⇒ 如实标记，绝不产出"无错误"（§三十二）。
//! - **证据可追溯**（§五 原则 2）：每个事件保留 source/line 指回原始
//!   日志；原始文本本身不入库（redacted 表示 + 位置引用）。
//! - **Root Cause 边界**（§六十八）：只产 Candidate/Hypothesis，时间
//!   接近 ≠ 因果（§十八）。
//! - **不可信输入**（§二十八/§五十五）：日志文本一律为 DATA，非指令；
//!   手写解析器 + 有界读取 + malformed ⇒ 诊断。

pub mod model;
pub mod normalize;
pub mod parsers;
pub mod redact;
pub mod report;
pub mod resolve;

pub use model::{
    ClusterExplanation, DebugError, DebugEvent, DebugReport, DebugRun, DebugSource, ErrorCluster,
    EventDomain, FrameResolution, MatchState, NormalizedLevel, ParseState, ResolutionStatus,
    StackFrame, StackTrace, DEBUG_SCHEMA_VERSION,
};
pub use normalize::normalize_message;
pub use parsers::{parse_source, ParserKind};
pub use redact::redact_line;
pub use report::{analyze, DebugAnalysisInput};
pub use resolve::resolve_frames;
