//! File → Symbol Change Mapping（2.0-M26 §11-§13）。
//!
//! Git 变更（文件级）→ 符号级变更：对比变更前后的 M23 抽取
//! （变更前 = `git show HEAD:path` / 变更后 = 工作区），以 SymbolId
//! （qualified+kind+signature，行号无关）划分 Added/Removed/Modified。
//! **Modified 判定**：同 id 存在于两侧且 modifiers/base_types 不同 ⇒
//! BaseOrInterfaceChanged/SignatureChanged；否则 BodyChanged。
//! 无法精确映射 ⇒ FileLevel（§44 诚实降级，不伪造符号级变更）。

use crate::model::FileExtraction;
use prism_intel::ImpactChangeType as ChangeType;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// 符号级变更记录（§12）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangedSymbol {
    pub symbol_id: String,
    pub qualified_name: String,
    pub change_type: ChangeType,
}

/// 文件级变更（输入；来自 Git status/diff 映射）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileChangeKind {
    Added,
    Removed,
    Modified,
}

/// 从变更前后抽取对生成符号级变更（§11）。
///
/// - `old`: 变更前抽取（Added 文件 ⇒ None）
/// - `new`: 变更后抽取（Deleted 文件 ⇒ None）
fn symbol_id_key(symbol: &crate::model::RawSymbol) -> String {
    crate::identity::symbol_id(symbol.kind, &symbol.qualified_name, &symbol.parameters)
}

pub fn map_file_change(
    old: Option<&FileExtraction>,
    new: Option<&FileExtraction>,
) -> Vec<ChangedSymbol> {
    // 以 **symbol_id**（含签名分量）为 diff 键——qualified 会把 overload
    // 签名变化折叠成同键，丢失 ContractRemoved/Added 对（§13 身份契约）。
    let old_ids: BTreeMap<String, &crate::model::RawSymbol> = old
        .map(|extraction| {
            extraction
                .symbols
                .iter()
                .map(|s| (symbol_id_key(s), s))
                .collect()
        })
        .unwrap_or_default();
    let new_ids: BTreeMap<String, &crate::model::RawSymbol> = new
        .map(|extraction| {
            extraction
                .symbols
                .iter()
                .map(|s| (symbol_id_key(s), s))
                .collect()
        })
        .unwrap_or_default();

    let mut out = Vec::new();
    // Removed：旧有新无（含 rename 的旧名——rename 检测 = 已记录限制，
    // git rename 证据沿 M22；此处按 Removed+Added 对呈现，不伪造同符号 §13）。
    for (key, symbol) in &old_ids {
        if !new_ids.contains_key(key) {
            out.push(ChangedSymbol {
                symbol_id: key.clone(),
                qualified_name: symbol.qualified_name.clone(),
                change_type: ChangeType::ContractRemoved,
            });
        }
    }
    // Added / Modified。
    for (key, symbol) in &new_ids {
        match old_ids.get(key) {
            None => out.push(ChangedSymbol {
                symbol_id: key.clone(),
                qualified_name: symbol.qualified_name.clone(),
                change_type: ChangeType::Added,
            }),
            Some(previous) => {
                // 同 id：签名（已含于 id）未变。修饰/基类型/接口面变化 ⇒
                // BaseOrInterfaceChanged；否则 BodyChanged。
                let face_changed = previous.modifiers != symbol.modifiers
                    || previous.base_types != symbol.base_types
                    || previous.constraints != symbol.constraints;
                // RawSymbol 无 body hash——同 id（签名同）即视为 Body 级，
                // 除非面变化；文档明示该近似（§19 Code vs Contract 边界）。
                out.push(ChangedSymbol {
                    symbol_id: key.clone(),
                    qualified_name: symbol.qualified_name.clone(),
                    change_type: if face_changed {
                        ChangeType::BaseOrInterfaceChanged
                    } else {
                        ChangeType::BodyChanged
                    },
                });
            }
        }
    }
    out.sort_by(|a, b| a.symbol_id.cmp(&b.symbol_id));
    out.dedup_by(|a, b| a.symbol_id == b.symbol_id);
    out
}

/// 一个文件的完整变更包（供 Impact 输入）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileChangeMapping {
    pub file: String,
    pub kind: FileChangeKind,
    pub changed_symbols: Vec<ChangedSymbol>,
}

/// 语言判断：仅 C# 文件做符号级映射，其余 FileLevel。
pub fn is_csharp(path: &str) -> bool {
    path.ends_with(".cs")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extract::extract;

    fn extraction(file: &str, source: &str) -> FileExtraction {
        extract(file, source)
    }

    #[test]
    fn added_removed_modified_classified() {
        let old = extraction(
            "A.cs",
            "namespace G; public class Player { public void Tick() { } public void Gone() { } }",
        );
        let new = extraction(
            "A.cs",
            "namespace G; public class Player { public void Tick() { } public void Added() { } }",
        );
        let changed = map_file_change(Some(&old), Some(&new));
        // Gone: Removed（contract）；Added: Added；Tick: BodyChanged（同 id）。
        assert!(changed
            .iter()
            .any(|c| c.qualified_name.ends_with("Gone")
                && c.change_type == ChangeType::ContractRemoved));
        assert!(changed
            .iter()
            .any(|c| c.qualified_name.ends_with("Added") && c.change_type == ChangeType::Added));
        assert!(changed.iter().any(
            |c| c.qualified_name.ends_with("Tick") && c.change_type == ChangeType::BodyChanged
        ));
    }

    #[test]
    fn signature_change_is_not_body() {
        let old = extraction("A.cs", "namespace G; class C { void M(int a) { } }");
        let new = extraction("A.cs", "namespace G; class C { void M(int a, int b) { } }");
        let changed = map_file_change(Some(&old), Some(&new));
        // 签名含于 id：旧 id 消失（ContractRemoved）+ 新 id 出现（Added）
        // ——对 Impact 而言等价于 contract break（调用方按旧签名失效）。
        assert!(changed.iter().any(
            |c| c.qualified_name.ends_with("M") && c.change_type == ChangeType::ContractRemoved
        ));
        assert!(changed
            .iter()
            .any(|c| c.qualified_name.ends_with("M") && c.change_type == ChangeType::Added));
    }

    #[test]
    fn deleted_file_maps_all_symbols_removed() {
        let old = extraction("A.cs", "namespace G; class C { void M() { } }");
        let changed = map_file_change(Some(&old), None);
        assert!(changed
            .iter()
            .all(|c| c.change_type == ChangeType::ContractRemoved));
        assert!(!changed.is_empty());
    }

    #[test]
    fn modifiers_change_is_base_or_interface_level() {
        let old = extraction("A.cs", "namespace G; class C : Base { }");
        let new = extraction("A.cs", "namespace G; class C : Other { }");
        let changed = map_file_change(Some(&old), Some(&new));
        let player = changed.iter().find(|c| c.qualified_name == "G.C").unwrap();
        assert_eq!(player.change_type, ChangeType::BaseOrInterfaceChanged);
    }
}
