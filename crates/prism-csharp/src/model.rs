//! L1 C# Symbol 模型（2.0-M23 §6-§16/§59）。
//!
//! 不变量：
//! - SymbolId 稳定且与行号无关（qualified + kind + signature 派生，§7）；
//! - overload 靠 signature 分量区分（§8）；partial 靠 declarations 聚合（§9）；
//! - Range 是证据定位，不是身份（§19）；
//! - DisplayName / QualifiedName / Signature / CanonicalIdentity 分离（§59）。

use serde::{Deserialize, Serialize};

/// 符号种类（§6 清单中语法层可确定者；不伪造 parser 能力）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SymbolKindCs {
    Namespace,
    Class,
    Struct,
    Interface,
    Enum,
    Record,
    RecordStruct,
    Delegate,
    Method,
    Constructor,
    Destructor,
    Property,
    Field,
    Event,
    Indexer,
    Operator,
    EnumMember,
    LocalFunction,
}

impl SymbolKindCs {
    pub fn as_str(&self) -> &'static str {
        match self {
            SymbolKindCs::Namespace => "namespace",
            SymbolKindCs::Class => "class",
            SymbolKindCs::Struct => "struct",
            SymbolKindCs::Interface => "interface",
            SymbolKindCs::Enum => "enum",
            SymbolKindCs::Record => "record",
            SymbolKindCs::RecordStruct => "record_struct",
            SymbolKindCs::Delegate => "delegate",
            SymbolKindCs::Method => "method",
            SymbolKindCs::Constructor => "constructor",
            SymbolKindCs::Destructor => "destructor",
            SymbolKindCs::Property => "property",
            SymbolKindCs::Field => "field",
            SymbolKindCs::Event => "event",
            SymbolKindCs::Indexer => "indexer",
            SymbolKindCs::Operator => "operator",
            SymbolKindCs::EnumMember => "enum_member",
            SymbolKindCs::LocalFunction => "local_function",
        }
    }
}

/// 1-based 源码范围（§19：证据定位，非身份）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceRangeCs {
    pub start_line: u32,
    pub start_column: u32,
    pub end_line: u32,
    pub end_column: u32,
}

/// 参数（§16：TypeSyntax 原样，不做语义解析）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParamInfo {
    pub name: String,
    pub type_syntax: String,
}

/// 特性（§17：只记录名称原文——`[Export]` 是"Symbol has Attribute named
/// Export"，Godot 语义归 M24）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttributeInfo {
    pub name: String,
}

/// 单个声明出现（§9：logical symbol 与 declaration occurrence 分离——
/// partial class 的每个文件片段是一条 declaration）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeclarationOccurrence {
    /// 项目相对路径（`/` 分隔，沿 M22 身份口径）。
    pub file: String,
    pub range: SourceRangeCs,
    pub name_range: SourceRangeCs,
}

/// 语法层原始符号（单文件抽取产物；跨文件聚合由 index 完成）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawSymbol {
    pub kind: SymbolKindCs,
    pub name: String,
    /// 含容器链的限定名（namespace + 外层类型 + 名称；不含签名）。
    pub qualified_name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub namespace: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub modifiers: Vec<String>,
    /// 泛型参数名（arity = len，§14）。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub type_parameters: Vec<String>,
    /// where 约束原文（语法事实，不做类型解析）。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub constraints: Vec<String>,
    /// 语法基类型列表（base_list 原文；解析归 M25）。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub base_types: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attributes: Vec<AttributeInfo>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parameters: Vec<ParamInfo>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub return_type_syntax: Option<String>,
    pub is_partial: bool,
    /// `///` 文档首行摘要（§18；无则 None）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub doc_summary: Option<String>,
    pub file: String,
    pub range: SourceRangeCs,
    pub name_range: SourceRangeCs,
}

/// 解析诊断（§50：Prism 自己的抽象，不泄漏第三方错误结构）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParseDiagnostic {
    pub file: String,
    pub message: String,
    pub range: SourceRangeCs,
    /// 原始信息保留位（debug 用）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw: Option<String>,
}

/// 单文件抽取结果。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileExtraction {
    pub file: String,
    pub content_hash: String,
    pub symbols: Vec<RawSymbol>,
    pub diagnostics: Vec<ParseDiagnostic>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_serialize_stable() {
        assert_eq!(
            serde_json::to_string(&SymbolKindCs::RecordStruct).unwrap(),
            "\"record_struct\""
        );
        assert_eq!(SymbolKindCs::Method.as_str(), "method");
    }

    #[test]
    fn range_is_plain_data_not_identity() {
        let range = SourceRangeCs {
            start_line: 1,
            start_column: 1,
            end_line: 2,
            end_column: 3,
        };
        let moved = SourceRangeCs {
            start_line: 10,
            start_column: 1,
            end_line: 11,
            end_column: 3,
        };
        assert_ne!(
            range, moved,
            "range changes on code movement — that's why it is not identity"
        );
    }
}
