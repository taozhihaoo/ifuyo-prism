//! FileChange：M5 第一层实体（规格 M5 §8–§14）。
//!
//! 把 Git-specific 的 GitDiffFile 转化为 Prism-level change facts：
//! 保留 before/after 存在性、语言与文件种类（M1 分类器语义），
//! rename 事实原样保留（不拆成 delete+add，§13）。

use crate::evidence::ChangeEvidence;
use crate::model::MappingStatus;
use prism_project::{FileKind, Language};
use serde::{Deserialize, Serialize};

/// 文件变更类型（规格 M5 §9）。rename 事实由 Git 明确给出时保留。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileChangeType {
    Added,
    Modified,
    Deleted,
    Renamed,
    Copied,
    TypeChanged,
    Unmerged,
    Unknown,
}

/// 一个文件的归一化变更。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileChange {
    /// `fc:{new_path|old_path}`；一个 ChangeSet 内路径唯一 ⇒ id 唯一且确定。
    pub id: String,
    pub old_path: Option<String>,
    pub new_path: Option<String>,
    pub change_type: FileChangeType,
    pub additions: Option<u64>,
    pub deletions: Option<u64>,
    /// before 侧是否存在（Added ⇒ false，Deleted ⇒ true）。
    pub old_exists: bool,
    /// after 侧是否存在。
    pub new_exists: bool,
    pub old_language: Option<Language>,
    pub new_language: Option<Language>,
    pub old_file_kind: Option<FileKind>,
    pub new_file_kind: Option<FileKind>,
    /// rename 且扩展名语言不同 ⇒ true（§14；只记录事实，不评风险）。
    pub language_changed: bool,
    pub is_binary: bool,
    /// Git 报告的 rename/copy 相似度（百分比整数），未提供为 None。
    pub similarity: Option<u64>,
    /// 符号映射维度是否成功运行（§21/§33；失败不击穿 ChangeSet）。
    pub mapping_status: MappingStatus,
    pub evidence: ChangeEvidence,
}

impl FileChange {
    /// 展示路径（rename 后优先）。
    pub fn path(&self) -> &str {
        self.new_path
            .as_deref()
            .or(self.old_path.as_deref())
            .unwrap_or("")
    }

    /// 确定性 id：`fc:{path}`。
    pub fn derive_id(path: &str) -> String {
        format!("fc:{path}")
    }

    /// 两侧语言是否都已知且不同。
    pub fn compute_language_changed(old: Option<Language>, new: Option<Language>) -> bool {
        matches!((old, new), (Some(a), Some(b)) if a != b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_and_path_are_stable() {
        assert_eq!(FileChange::derive_id("src/a.ts"), "fc:src/a.ts");
        let change = FileChange {
            id: FileChange::derive_id("src/b.ts"),
            old_path: Some("src/a.ts".to_string()),
            new_path: Some("src/b.ts".to_string()),
            change_type: FileChangeType::Renamed,
            additions: Some(0),
            deletions: Some(0),
            old_exists: true,
            new_exists: true,
            old_language: Some(Language::TypeScript),
            new_language: Some(Language::TypeScript),
            old_file_kind: Some(FileKind::Source),
            new_file_kind: Some(FileKind::Source),
            language_changed: false,
            is_binary: false,
            similarity: Some(100),
            mapping_status: MappingStatus::Mapped,
            evidence: ChangeEvidence::new(crate::evidence::EvidenceSource::GitRename, "git -z raw"),
        };
        assert_eq!(change.path(), "src/b.ts");
        let json = serde_json::to_value(&change).unwrap();
        assert_eq!(json["change_type"], "renamed");
        assert_eq!(json["mapping_status"], "mapped");
        assert_eq!(json["old_file_kind"], "source");
    }

    #[test]
    fn language_change_detected_only_when_both_known() {
        assert!(FileChange::compute_language_changed(
            Some(Language::Python),
            Some(Language::TypeScript)
        ));
        assert!(!FileChange::compute_language_changed(
            Some(Language::Python),
            None
        ));
        assert!(!FileChange::compute_language_changed(
            Some(Language::Rust),
            Some(Language::Rust)
        ));
    }
}
