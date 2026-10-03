//! ChangeDiagnostic：可解释的部分失败（规格 M5 §33/§34/§80）。
//!
//! 映射失败不是 ChangeSet 失败：95 mapped + 5 unmapped 仍是有效结果，
//! 5 条 diagnostic 如实记录。

use serde::{Deserialize, Serialize};

/// 诊断种类（规格 M5 §33 全集 + 实现所需的补充项）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum ChangeDiagnosticKind {
    /// 同一变更存在多个同等符号候选（§83）。
    SymbolMappingAmbiguous,
    /// 变更行存在但某一侧符号事实不可用（解析失败/缺失基线）。
    SymbolMappingUnavailable,
    /// 历史 revision 内容不可用（过大/编码/读取失败）。
    HistoricalModelUnavailable,
    UnsupportedLanguage,
    ParseFailed,
    BinaryFile,
    GeneratedFile,
    /// 删除文件且 before 源不可得 ⇒ 不伪造符号（§40）。
    DeletedFileWithoutBaseline,
    /// rename 匹配不确定（保守降级为 delete+add，§49）。
    RenameMappingUncertain,
    /// 内容超过大小上限（§78）。
    FileTooLarge,
    /// 非 UTF-8 内容：显式不可解析，不假装空文件（§42）。
    EncodingUnsupported,
    /// git 查询失败但未击穿整个 ChangeSet（§80）。
    GitQueryFailed,
}

impl ChangeDiagnosticKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            ChangeDiagnosticKind::SymbolMappingAmbiguous => "symbol_mapping_ambiguous",
            ChangeDiagnosticKind::SymbolMappingUnavailable => "symbol_mapping_unavailable",
            ChangeDiagnosticKind::HistoricalModelUnavailable => "historical_model_unavailable",
            ChangeDiagnosticKind::UnsupportedLanguage => "unsupported_language",
            ChangeDiagnosticKind::ParseFailed => "parse_failed",
            ChangeDiagnosticKind::BinaryFile => "binary_file",
            ChangeDiagnosticKind::GeneratedFile => "generated_file",
            ChangeDiagnosticKind::DeletedFileWithoutBaseline => "deleted_file_without_baseline",
            ChangeDiagnosticKind::RenameMappingUncertain => "rename_mapping_uncertain",
            ChangeDiagnosticKind::FileTooLarge => "file_too_large",
            ChangeDiagnosticKind::EncodingUnsupported => "encoding_unsupported",
            ChangeDiagnosticKind::GitQueryFailed => "git_query_failed",
        }
    }
}

/// 一条变更诊断。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangeDiagnostic {
    pub kind: ChangeDiagnosticKind,
    /// 项目相对路径；全局性诊断为 None。
    pub path: Option<String>,
    /// 一句话解释（不含源码/diff 内容，§95）。
    pub detail: String,
}

/// 确定性排序键（§63 deterministic ordering）。
pub fn diagnostic_sort_key(d: &ChangeDiagnostic) -> (&'static str, &str, &str) {
    (
        d.kind.as_str(),
        d.path.as_deref().unwrap_or(""),
        d.detail.as_str(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostic_serializes_snake_case_and_sorts() {
        let d = ChangeDiagnostic {
            kind: ChangeDiagnosticKind::ParseFailed,
            path: Some("src/broken.ts".to_string()),
            detail: "after side failed to parse".to_string(),
        };
        let json = serde_json::to_value(&d).unwrap();
        assert_eq!(json["kind"], "parse_failed");
        assert_eq!(diagnostic_sort_key(&d).0, "parse_failed");
    }
}
