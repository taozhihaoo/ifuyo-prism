//! ChangeSet 根模型：身份、scope、summary（规格 M5 §5–§7、§32、§63）。
//!
//! 身份契约（§6）：`changeset:{repository}#{base}#{target}#{scope}`。
//! repository 用仓库目录名（不含绝对路径，§63；稳定项目身份属 M11）。
//! base/target 用解析后的完整 SHA（或 worktree/index/unborn），
//! 因此 `HEAD~1` 与直接写 SHA 得到同一逻辑身份。

use crate::diagnostic::ChangeDiagnostic;
use crate::file_change::FileChange;
use crate::module_change::ModuleChange;
use crate::symbol_change::SymbolChange;
use serde::{Deserialize, Serialize};

/// 对外 JSON 契约版本。
pub const CHANGESET_SCHEMA_VERSION: u32 = 1;

/// 变更来源（规格 M5 §104）。M5 只实现 Git；其余为前向兼容的显式占位。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeSource {
    Git,
    Snapshot,
    Patch,
    Unknown,
}

/// 变更范围（规格 M5 §7）。与 M4 GitDiffScope 对齐；单提交 scope 留待需要时加入。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeScope {
    /// HEAD → 工作区。
    WorkingTree,
    /// HEAD → index。
    Staged,
    /// 两个 revision 之间。
    RevisionRange,
}

impl ChangeScope {
    pub const fn as_str(self) -> &'static str {
        match self {
            ChangeScope::WorkingTree => "working_tree",
            ChangeScope::Staged => "staged",
            ChangeScope::RevisionRange => "revision_range",
        }
    }
}

/// diff→symbol 归属状态（规格 M5 §21）。不强迫所有变更都映射到符号。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MappingStatus {
    /// 已归属到明确实体并保留证据。
    Mapped,
    /// 存在多个同等候选，不做硬选择（§83）。
    Ambiguous,
    /// 变更存在但无法归属（如 changed lines 在任何符号之外、解析失败侧）。
    Unmapped,
    /// 该维度不适用（二进制/不支持语言/新增文件无 before 侧）。
    NotApplicable,
}

/// 符号匹配置信度（规格 M5 §20）。分类置信，不制造伪精确分数。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchConfidence {
    /// 稳定 ID 完全一致。
    Exact,
    /// 结构证据充分（如 rename 证据 + 主体一致）。
    HighConfidence,
    /// 弱证据（保留给未来策略；当前实现不产生）。
    Probable,
    /// 无匹配对象（added/deleted 侧）。
    Unmatched,
}

/// 汇总统计：只统计事实，不含任何风险/影响判断（规格 M5 §32/§59）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ChangeSummary {
    pub files_total: u64,
    pub files_added: u64,
    pub files_modified: u64,
    pub files_deleted: u64,
    pub files_renamed: u64,
    pub files_copied: u64,
    pub files_type_changed: u64,
    pub files_other: u64,
    pub symbols_total: u64,
    pub symbols_added: u64,
    pub symbols_deleted: u64,
    pub symbols_modified: u64,
    pub symbols_renamed: u64,
    pub symbols_moved: u64,
    pub modules_changed: u64,
    /// 有符号处于 Ambiguous 归属的文件数。
    pub files_with_ambiguous_symbols: u64,
    /// Test 类文件变更数（§60，只计数不断言影响）。
    pub test_files_changed: u64,
    /// Documentation 类文件变更数（§61）。
    pub documentation_files_changed: u64,
    pub diagnostics_total: u64,
}

/// 一行/一段行号区间（两端含入；对外展示用）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LineRange {
    pub start_line: u32,
    pub end_line: u32,
}

/// 一个受影响 hunk 的行区间摘要（evidence 的组成部分）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChangedRange {
    pub old_start: Option<u64>,
    pub old_lines: u64,
    pub new_start: Option<u64>,
    pub new_lines: u64,
}

/// ChangeSet（规格 M5 §5/§63）：稳定字段、确定性排序、可 JSON 序列化。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangeSet {
    pub schema_version: u32,
    /// 确定性身份：`changeset:{repository}#{base}#{target}#{scope}`。
    pub id: String,
    pub source: ChangeSource,
    pub scope: ChangeScope,
    /// 解析后的基线标识（完整 SHA / "worktree" / "index" / "unborn"）。
    pub base: Option<String>,
    /// 解析后的目标标识（同上）。
    pub target: Option<String>,
    /// 仓库目录名（不含绝对路径，§63）；非 Git 场景为 None。
    pub repository: Option<String>,
    /// 按 new_path（无则 old_path）字典序。
    pub files: Vec<FileChange>,
    /// 按 (path, name, change_type) 字典序。
    pub symbols: Vec<SymbolChange>,
    /// 按 module_id 字典序。
    pub modules: Vec<ModuleChange>,
    pub summary: ChangeSummary,
    /// 按 (kind, path, detail) 字典序；映射失败不是 ChangeSet 失败（§33/§34）。
    pub diagnostics: Vec<ChangeDiagnostic>,
    /// 输入 diff 被文件数上限截断时为 true（如实标注，§35）。
    pub truncated: bool,
}

impl ChangeSet {
    /// 确定性身份（§6）。同一 Git 状态 + 同一 scope ⇒ 同一 id。
    pub fn derive_id(repository: Option<&str>, base: &str, target: &str, scope: ChangeScope) -> String {
        format!(
            "changeset:{}#{}#{}#{}",
            repository.unwrap_or("unknown"),
            base,
            target,
            scope.as_str()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_is_deterministic_and_path_free() {
        let a = ChangeSet::derive_id(Some("prism"), "abc123", "worktree", ChangeScope::WorkingTree);
        let b = ChangeSet::derive_id(Some("prism"), "abc123", "worktree", ChangeScope::WorkingTree);
        assert_eq!(a, b);
        assert!(a.starts_with("changeset:prism#abc123#worktree#working_tree"));
        assert!(!a.contains('\\'), "no absolute paths in identity");
    }

    #[test]
    fn scope_and_enums_serialize_snake_case() {
        assert_eq!(
            serde_json::to_string(&ChangeScope::RevisionRange).unwrap(),
            "\"revision_range\""
        );
        assert_eq!(
            serde_json::to_string(&MappingStatus::NotApplicable).unwrap(),
            "\"not_applicable\""
        );
        assert_eq!(
            serde_json::to_string(&MatchConfidence::HighConfidence).unwrap(),
            "\"high_confidence\""
        );
        assert_eq!(ChangeScope::Staged.as_str(), "staged");
    }

    #[test]
    fn summary_defaults_to_zero_facts() {
        let summary = ChangeSummary::default();
        assert_eq!(summary.files_total, 0);
        assert_eq!(summary.symbols_total, 0);
        assert_eq!(summary.modules_changed, 0);
    }
}
