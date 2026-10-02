//! 语言 adapter（tree-sitter 实现）。
//!
//! 语言私有 AST 只在本模块内出现，绝不外泄（规格 §9：不做万能 AST）；
//! 每个 adapter 把语法结构归一化为 Prism Symbol 模型。

pub mod csharp;
pub mod python;
pub mod web;

pub use csharp::CSharpParser;
pub use python::PythonParser;
pub use web::WebParser;
