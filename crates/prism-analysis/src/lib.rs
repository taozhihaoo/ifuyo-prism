//! prism-analysis — Code Analysis 编排层（M2）。
//!
//! 职责：
//!
//! - 从 M1 [`ProjectDiscovery`] 选择可分析源文件（`Source && analyzable`，
//!   Generated 默认跳过——规格 §46，不重写识别逻辑）
//! - 有界并发解析（每 worker 一个 ParserRegistry），取消与进度
//! - 组装 [`ProjectCodeModel`]（复用 M1 FileInfo，不造平行文件模型——§27）
//! - [`SymbolIndex`]：file/symbol/parent/children/qualified/name 五类查询
//!
//! 确定性：结果按 relative_path 排序，聚合顺序与并发执行顺序无关。
//! 不做依赖图 / 架构分析 / 健康评分（M3/M7）。

pub mod index;
pub mod model;
pub mod service;

pub use index::SymbolIndex;
pub use model::{ParseSummary, ProjectCodeModel, CODE_MODEL_SCHEMA_VERSION};
pub use service::{CodeAnalysisOptions, CodeAnalysisProgress, CodeAnalysisService};
