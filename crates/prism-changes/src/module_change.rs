//! ModuleChange：M5 第三层实体（规格 M5 §28–§31、§54）。
//!
//! 复用 M3 的 ModuleId（`module:file:{path}` / `module:namespace:{ns}`），
//! 不重新发明 module identity；FileChange = Deleted 不自动等于
//! Module Deleted——由 before/after 模块事实决定（§31）。

use crate::evidence::ChangeEvidence;
use crate::model::MappingStatus;
use prism_graph::{ModuleKind, ModuleId};
use serde::{Deserialize, Serialize};

/// 模块变更类型。模块生命周期只由模块事实决定（§31）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModuleChangeType {
    Added,
    Deleted,
    Modified,
}

/// 一个模块的归一化变更，证据指向具体文件/符号（§54）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleChange {
    /// `mc:{module_id}`。
    pub id: String,
    pub module_id: ModuleId,
    pub module_name: String,
    pub module_kind: ModuleKind,
    pub change_type: ModuleChangeType,
    /// 受影响文件（项目相对路径，排序去重）。
    pub changed_files: Vec<String>,
    /// 受影响符号数（事实计数，不做影响推断）。
    pub changed_symbol_count: u64,
    pub mapping_status: MappingStatus,
    pub evidence: ChangeEvidence,
}

impl ModuleChange {
    pub fn derive_id(module_id: &ModuleId) -> String {
        format!("mc:{}", module_id.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn module_change_serializes_with_m3_module_id() {
        let module_id = prism_graph::file_module_id("src/a.ts");
        let change = ModuleChange {
            id: ModuleChange::derive_id(&module_id),
            module_id,
            module_name: "a.ts".to_string(),
            module_kind: ModuleKind::FileModule,
            change_type: ModuleChangeType::Modified,
            changed_files: vec!["src/a.ts".to_string()],
            changed_symbol_count: 2,
            mapping_status: MappingStatus::Mapped,
            evidence: ChangeEvidence::new(crate::evidence::EvidenceSource::ModuleFacts, "file module of changed file"),
        };
        let json = serde_json::to_value(&change).unwrap();
        assert_eq!(json["id"], "mc:module:file:src/a.ts");
        assert_eq!(json["module_id"], "module:file:src/a.ts");
        assert_eq!(json["change_type"], "modified");
        assert_eq!(json["module_kind"], "file_module");
    }
}
