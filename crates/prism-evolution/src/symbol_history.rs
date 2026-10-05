//! Symbol History（M211 §15-§17）：把文件历史回放成符号级变更。
//!
//! 证据链：`Commit → FileChange → blob(old^/new) → M23 抽取 → M26
//! map_file_change → SymbolChange`。签名变化按 M26 口径：同 qualified 的
//! ContractRemoved+Added 对 ⇒ SignatureChanged（含签名分量 id，§13 身份契约）。
//! blob 不可得（非 UTF-8/超大/缺失）⇒ 该条目如实记诊断，不猜测。

use prism_csharp::{extract, map_file_change, ChangedSymbol};
use prism_git::model::RevisionBlob;
use prism_git::GitService;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// 符号变更种类（§16 第一优先集；Godot/C# 优先）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SymbolChangeKind {
    Added,
    Deleted,
    Modified,
    /// 同 qualified 的移除+新增对（含签名/基类/接口变化，§17）。
    SignatureChanged,
    Unknown,
}

/// 单个符号变更。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SymbolChange {
    /// M23 逻辑符号 id（内容寻址）。
    pub symbol_id: String,
    pub qualified_name: String,
    pub kind: SymbolChangeKind,
}

/// 一个 commit 对该文件产生的符号级变更条目。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SymbolHistoryEntry {
    pub commit_id: String,
    pub committed_at: Option<String>,
    pub author: String,
    /// 该条目解析所用的路径（rename 时为 old→new 中的对应端）。
    pub file_path: String,
    pub changes: Vec<SymbolChange>,
    /// M23 抽取是否可用（blob 缺失/非文本 ⇒ false，符号层 Unknown）。
    pub extraction_available: bool,
    pub diagnostics: Vec<String>,
}

/// 回放结果。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SymbolHistory {
    pub entries: Vec<SymbolHistoryEntry>,
    pub diagnostics: Vec<String>,
}

const BLOB_MAX_BYTES: u64 = 4 * 1024 * 1024;

/// 文件 → 符号历史（§15：创建/修改/签名变化/删除由哪些 commit 造成）。
pub fn symbol_history(
    service: &GitService,
    root: &Path,
    relative_path: &str,
    max_commits: u64,
    cancel: &prism_core::task::CancellationToken,
) -> prism_git::GitResult<SymbolHistory> {
    let is_csharp = relative_path.ends_with(".cs");
    let entries = service.file_history(root, relative_path, max_commits, cancel)?;
    let mut out = Vec::new();
    let diagnostics = Vec::new();
    for entry in &entries {
        let Some(change) = entry
            .changes
            .iter()
            .find(|change| {
                change.path_after == relative_path
                    || Some(&relative_path.to_string()) == change.path_before.as_ref()
            })
            .or_else(|| entry.changes.first())
        else {
            continue;
        };
        let new_path = change.path_after.as_str();
        let old_path = change.path_before.as_deref().unwrap_or(new_path);
        let mut entry_diagnostics = Vec::new();
        let mut old_extraction = None;
        let mut new_extraction = None;
        if is_csharp {
            // new：本提交中的内容。
            match service.read_blob(
                root,
                entry.commit.id.as_str(),
                new_path,
                BLOB_MAX_BYTES,
                cancel,
            ) {
                Ok(RevisionBlob::Text { content }) => {
                    new_extraction = Some(extract(new_path, &content));
                }
                Ok(RevisionBlob::Missing) => entry_diagnostics.push(format!(
                    "{new_path} missing at {} (deleted)",
                    entry.commit.id
                )),
                Ok(RevisionBlob::TooLarge { size_bytes }) => {
                    entry_diagnostics.push(format!(
                        "{new_path} too large at {} ({size_bytes} bytes); symbol change Unknown",
                        entry.commit.id
                    ));
                }
                Ok(RevisionBlob::EncodingUnsupported) => entry_diagnostics.push(format!(
                    "{new_path} non-UTF-8 at {}; symbol change Unknown",
                    entry.commit.id
                )),
                Err(err) => entry_diagnostics.push(format!("read new blob failed: {err}")),
            }
            // old：父提交中的内容（rename 用 path_before）。
            let parent_revision = format!("{}^", entry.commit.id);
            match service.read_blob(root, &parent_revision, old_path, BLOB_MAX_BYTES, cancel) {
                Ok(RevisionBlob::Text { content }) => {
                    old_extraction = Some(extract(old_path, &content));
                }
                Ok(RevisionBlob::Missing) => {}
                Ok(RevisionBlob::TooLarge { size_bytes }) => entry_diagnostics.push(format!(
                    "{old_path} too large at {parent_revision} ({size_bytes} bytes)"
                )),
                Ok(RevisionBlob::EncodingUnsupported) => {
                    entry_diagnostics.push(format!("{old_path} non-UTF-8 at {parent_revision}"))
                }
                Err(err) => entry_diagnostics.push(format!("read old blob failed: {err}")),
            }
        } else {
            entry_diagnostics.push(
                "non-C# path; symbol layer unsupported (still valid file history)".to_string(),
            );
        }
        let changes = match (&old_extraction, &new_extraction) {
            (old, new) if old.is_some() || new.is_some() => {
                merge_pairs(map_file_change(old.as_ref(), new.as_ref()))
            }
            _ => Vec::new(),
        };
        out.push(SymbolHistoryEntry {
            commit_id: entry.commit.id.clone(),
            committed_at: entry.commit.committed_at.clone(),
            author: entry.commit.author.name.clone(),
            file_path: new_path.to_string(),
            extraction_available: old_extraction.is_some() || new_extraction.is_some(),
            changes,
            diagnostics: entry_diagnostics,
        });
    }
    Ok(SymbolHistory {
        entries: out,
        diagnostics,
    })
}

/// qualified 基座（签名后缀前的部分）。
fn qualified_base(qualified: &str) -> String {
    qualified
        .rsplit_once('#')
        .map(|(prefix, _)| prefix.to_string())
        .unwrap_or_else(|| qualified.to_string())
}

/// ContractRemoved+Added 同 qualified 对 ⇒ 单条 SignatureChanged（§17）。
fn merge_pairs(changed: Vec<ChangedSymbol>) -> Vec<SymbolChange> {
    use prism_intel::ImpactChangeType;
    let mut out: Vec<SymbolChange> = Vec::new();
    let removed: Vec<&ChangedSymbol> = changed
        .iter()
        .filter(|change| change.change_type == ImpactChangeType::ContractRemoved)
        .collect();
    for change in &changed {
        let kind = match change.change_type {
            ImpactChangeType::Added => {
                // qualified 基座相同（签名后缀不同）即视为同符号换签名（§13）。
                let is_signature_pair = removed.iter().any(|old_change| {
                    qualified_base(&old_change.qualified_name)
                        == qualified_base(&change.qualified_name)
                });
                if is_signature_pair {
                    SymbolChangeKind::SignatureChanged
                } else {
                    SymbolChangeKind::Added
                }
            }
            ImpactChangeType::ContractRemoved => {
                // 与 Added 对成对时并入 SignatureChanged；无对（纯删除）⇒ Deleted。
                let paired = changed.iter().any(|new_change| {
                    new_change.change_type == ImpactChangeType::Added
                        && qualified_base(&new_change.qualified_name)
                            == qualified_base(&change.qualified_name)
                });
                if paired {
                    continue;
                }
                SymbolChangeKind::Deleted
            }
            ImpactChangeType::BaseOrInterfaceChanged => SymbolChangeKind::SignatureChanged,
            ImpactChangeType::BodyChanged => SymbolChangeKind::Modified,
            _ => SymbolChangeKind::Unknown,
        };
        out.push(SymbolChange {
            symbol_id: change.symbol_id.clone(),
            qualified_name: qualified_base(&change.qualified_name),
            kind,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signature_pair_merges_to_signature_changed() {
        use prism_intel::ImpactChangeType;
        let changed = vec![
            ChangedSymbol {
                symbol_id: "cs:App.A.Attack#method#old".to_string(),
                qualified_name: "cs:App.A.Attack#method#old".to_string(),
                change_type: ImpactChangeType::ContractRemoved,
            },
            ChangedSymbol {
                symbol_id: "cs:App.A.Attack#method#new".to_string(),
                qualified_name: "cs:App.A.Attack#method#new".to_string(),
                change_type: ImpactChangeType::Added,
            },
        ];
        let merged = merge_pairs(changed);
        assert_eq!(merged.len(), 1, "同 qualified 对合并");
        assert_eq!(merged[0].kind, SymbolChangeKind::SignatureChanged);
        assert_eq!(merged[0].qualified_name, "cs:App.A.Attack#method");
    }

    #[test]
    fn pure_removal_is_deleted_not_paired() {
        use prism_intel::ImpactChangeType;
        let changed = vec![ChangedSymbol {
            symbol_id: "cs:App.A.Gone#method#old".to_string(),
            qualified_name: "cs:App.A.Gone#method#old".to_string(),
            change_type: ImpactChangeType::ContractRemoved,
        }];
        let merged = merge_pairs(changed);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].kind, SymbolChangeKind::Deleted);
    }
}
