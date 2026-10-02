//! ParseRequest / ParseResult / ParseDiagnostic。
//!
//! 单文件解析失败是文件级 non-fatal 事件：状态落在 [`ParseStatus`]，
//! 不产生项目级错误（规格 §22/§23）。

use crate::model::{SourceLocation, Symbol};
use prism_core::ids::FileId;
use prism_project::Language;
use serde::{Deserialize, Serialize};

/// 一条 import 语法事实（M3 关系解析的输入；不做任何解析判定）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportInfo {
    /// 原样 specifier（如 "./utils" / "os" / "System.Collections"）。
    pub specifier: String,
    pub kind: ImportKind,
    /// 命名导入（如 `{ a, b }` / `from x import y`）；不确定时为空。
    pub names: Vec<String>,
    /// 语句起始位置（诊断/evidence 用）。
    pub location: Option<crate::model::SourceLocation>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportKind {
    /// 普通 import / using。
    Plain,
    /// `export ... from "./x"`（re-export 语法）。
    ReExport,
    /// `import type` / `import type {..}`。
    TypeOnly,
}

/// 一条直接调用语法事实（规格 §19 可靠子集：仅直接函数/方法调用）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CallInfo {
    /// 调用方所在函数/方法的限定路径；顶层调用无宿主时不捕获。
    pub caller_qualified: String,
    /// 被调者的原样文本（如 "helper" / "obj.method" / "svc.fetch"）。
    pub callee_text: String,
    /// 调用表达式起始位置。
    pub location: Option<crate::model::SourceLocation>,
}

/// 解析请求：按需读取的单文件内容（解析后即丢弃，不驻留）。
#[derive(Debug, Clone)]
pub struct ParseRequest {
    pub file_id: FileId,
    /// 项目相对路径（`/` 分隔；SymbolId 派生的一部分）。
    pub relative_path: String,
    pub language: Language,
    pub content: String,
}

/// 解析结果状态（规格 §22 的五态）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParseStatus {
    /// 完全成功，无 diagnostics。
    Successful,
    /// 有诊断但已尽可能恢复符号（部分解析，规格 §24）。
    SuccessfulWithDiagnostics,
    /// 解析失败（如编码不支持、parser 内部错误）。
    Failed,
    /// 无对应 parser（如 Ruby；规格 §35）。
    Unsupported,
    /// 按策略跳过（如 Generated 文件）。
    Skipped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParseDiagnosticSeverity {
    Info,
    Warning,
    Error,
}

/// 一条可定位的解析诊断。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParseDiagnostic {
    pub severity: ParseDiagnosticSeverity,
    pub message: String,
    /// 尽力而为：崩溃点不可定位时为 None。
    pub location: Option<SourceLocation>,
    /// 稳定诊断码（如 "syntax_error" / "unsupported_encoding"）。
    pub code: String,
}

impl ParseDiagnostic {
    pub fn error(code: &str, message: impl Into<String>, location: Option<SourceLocation>) -> Self {
        ParseDiagnostic {
            severity: ParseDiagnosticSeverity::Error,
            message: message.into(),
            location,
            code: code.to_string(),
        }
    }

    pub fn warning(
        code: &str,
        message: impl Into<String>,
        location: Option<SourceLocation>,
    ) -> Self {
        ParseDiagnostic {
            severity: ParseDiagnosticSeverity::Warning,
            message: message.into(),
            location,
            code: code.to_string(),
        }
    }
}

/// 一个文件的完整解析事实。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParseResult {
    pub file_id: FileId,
    pub relative_path: String,
    /// Unsupported 状态下可能没有 parser，语言仍记录 M1 的识别结果。
    pub language: Option<Language>,
    pub status: ParseStatus,
    pub symbols: Vec<Symbol>,
    pub diagnostics: Vec<ParseDiagnostic>,
    pub duration_ms: u64,
    /// parser 实现版本（未来 AnalysisCacheKey 的一部分，规格 §32）。
    pub parser_version: String,
    /// Skipped/Unsupported 的原因（如 "generated" / "no_parser"）。
    pub reason: Option<String>,
    /// 文件内全部 import 语法事实（M3 起；M2 已构造处补空 Vec）。
    #[serde(default)]
    pub imports: Vec<ImportInfo>,
    /// 文件内全部直接调用语法事实（M3 起）。
    #[serde(default)]
    pub calls: Vec<CallInfo>,
}

impl ParseResult {
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == ParseDiagnosticSeverity::Error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn statuses_serialize_snake_case() {
        assert_eq!(
            serde_json::to_string(&ParseStatus::SuccessfulWithDiagnostics).unwrap(),
            "\"successful_with_diagnostics\""
        );
        assert_eq!(
            serde_json::to_string(&ParseStatus::Skipped).unwrap(),
            "\"skipped\""
        );
    }

    #[test]
    fn has_errors_detects_error_severity_only() {
        let result = ParseResult {
            file_id: FileId::from_string("file:a.ts"),
            relative_path: "a.ts".to_string(),
            language: Some(Language::TypeScript),
            status: ParseStatus::SuccessfulWithDiagnostics,
            symbols: vec![],
            diagnostics: vec![ParseDiagnostic::warning("trailing", "extra token", None)],
            duration_ms: 1,
            parser_version: "test".to_string(),
            reason: None,
            imports: Vec::new(),
            calls: Vec::new(),
        };
        assert!(!result.has_errors());

        let with_error = ParseResult {
            diagnostics: vec![ParseDiagnostic::error("syntax_error", "broke", None)],
            ..result
        };
        assert!(with_error.has_errors());
    }

    #[test]
    fn result_serializes_with_contract_fields() {
        let result = ParseResult {
            file_id: FileId::from_string("file:a.ts"),
            relative_path: "a.ts".to_string(),
            language: Some(Language::TypeScript),
            status: ParseStatus::Successful,
            symbols: vec![],
            diagnostics: vec![],
            duration_ms: 3,
            parser_version: "tree-sitter-test/1".to_string(),
            reason: None,
            imports: Vec::new(),
            calls: Vec::new(),
        };
        let json = serde_json::to_value(&result).unwrap();
        for field in [
            "file_id",
            "relative_path",
            "language",
            "status",
            "symbols",
            "diagnostics",
            "duration_ms",
            "parser_version",
        ] {
            assert!(json.get(field).is_some(), "missing {field}");
        }
    }
}
