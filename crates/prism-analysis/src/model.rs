//! ProjectCodeModel：M2 的输出契约（GUI / CLI / 报告同源，规格 §41/§25）。
//!
//! Symbol 只存放在 [`ParseResult::symbols`]（每文件一份，按解析顺序 = 确定性）；
//! [`SymbolIndex`] 是派生结构（内存态，不参与序列化）。

use prism_core::task::TaskState;
use prism_parser::ParseResult;
use prism_project::{FileInfo, Project};
use serde::{Deserialize, Serialize};

/// 对外 JSON 契约版本。
pub const CODE_MODEL_SCHEMA_VERSION: u32 = 1;

/// 解析汇总（全部为真实计数）。status 默认取 M0 TaskState 的 Queued。
impl Default for ParseSummary {
    fn default() -> Self {
        ParseSummary {
            files_total: 0,
            files_parsed: 0,
            files_with_diagnostics: 0,
            files_failed: 0,
            files_unsupported: 0,
            files_skipped: 0,
            symbols_total: 0,
            error_count: 0,
            warning_count: 0,
            duration_ms: 0,
            status: TaskState::Queued,
        }
    }
}

/// 解析汇总（全部为真实计数）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParseSummary {
    /// 进入管线的文件数（Source 可分析 + Generated 跳过声明）。
    pub files_total: u64,
    /// Successful。
    pub files_parsed: u64,
    /// SuccessfulWithDiagnostics。
    pub files_with_diagnostics: u64,
    /// Failed。
    pub files_failed: u64,
    /// Unsupported（无 parser）。
    pub files_unsupported: u64,
    /// Skipped（generated / too_large 等策略性跳过）。
    pub files_skipped: u64,
    pub symbols_total: u64,
    pub error_count: u64,
    pub warning_count: u64,
    pub duration_ms: u64,
    /// cancelled 表示用户取消后保留的部分结果。
    pub status: TaskState,
}

/// 项目代码模型。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectCodeModel {
    pub schema_version: u32,
    pub project: Project,
    /// 进入管线（或被策略性跳过）的源文件，复用 M1 FileInfo（规格 §27）。
    pub files: Vec<FileInfo>,
    /// 每文件解析事实（含符号与诊断），按 relative_path 字典序。
    pub parse_results: Vec<ParseResult>,
    pub summary: ParseSummary,
}

impl ProjectCodeModel {
    /// 全部符号的平铺迭代（parse_results 为唯一存储，避免双份簿记）。
    pub fn symbols(&self) -> impl Iterator<Item = &prism_parser::Symbol> {
        self.parse_results
            .iter()
            .flat_map(|result| result.symbols.iter())
    }
}
