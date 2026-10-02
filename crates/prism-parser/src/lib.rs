//! prism-parser — Code Understanding 层（M2）。
//!
//! 职责：
//!
//! - [`model`]：语言无关的 Symbol 归一化模型与 SourceRange 坐标契约
//! - [`parse`]：ParseRequest / ParseResult / ParseDiagnostic（部分成功可表达）
//! - [`registry`]：LanguageParser trait 与 ParserRegistry（语言分派集中于此）
//! - [`languages`]：具体语言 adapter（tree-sitter 实现，语言私有 AST 不外泄）
//!
//! 边界（纲领/规格 §66）：不做依赖图、不做架构分析、不做 AI；
//! 解析失败的文件产生 diagnostics，不击穿整个项目。

pub mod model;
pub mod parse;
pub mod registry;

pub use model::{
    SourceLocation, SourceRange, Symbol, SymbolKind, SymbolVisibility, COORDINATE_CONTRACT,
};
pub use parse::{ParseDiagnostic, ParseDiagnosticSeverity, ParseRequest, ParseResult, ParseStatus};
pub use registry::{LanguageParser, ParserRegistry};
