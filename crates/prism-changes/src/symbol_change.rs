//! SymbolChange：M5 第二层实体（规格 M5 §15–§27、§49、§53）。
//!
//! 主类型 + 特征标志（primary_change_type + characteristics，§17），
//! 不强迫单一 enum；绝不把 ChangedSymbol 解释成 ImpactedSymbol（§0）。

use crate::evidence::ChangeEvidence;
use crate::model::{LineRange, MappingStatus, MatchConfidence};
use prism_core::ids::SymbolId;
use prism_parser::SymbolKind;
use serde::{Deserialize, Serialize};

/// 符号变更主类型（规格 M5 §16）。细分差异进 characteristics，不堆 enum。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum SymbolChangeType {
    Added,
    Deleted,
    Modified,
    Renamed,
    Moved,
    Unknown,
}

/// 变更特征（规格 M5 §58 可靠子集）。每项必须有证据来源；
/// Breaking/风险类判断被明确排除（§58/§59）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum SymbolCharacteristic {
    /// 声明行本身在变更行内。
    DeclarationChanged,
    /// 结构化 signature 字符串发生变化。
    SignatureChanged,
    /// 变更行落在符号体内。
    BodyChanged,
    /// visibility 发生变化。
    VisibilityChanged,
    /// import/export 符号变更（依赖影响留给 M6，§27）。
    ImportChanged,
    ExportChanged,
    /// documentation_present 发生变化。
    DocumentationChanged,
    /// 变更落在符号范围内但无法进一步归类。
    MetadataChanged,
}

/// 一个符号的归一化变更。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SymbolChange {
    /// 所属 FileChange id（回溯文件）。
    pub file_change_id: String,
    /// 展示路径（rename 后路径）。
    pub path: String,
    pub before_symbol_id: Option<SymbolId>,
    pub after_symbol_id: Option<SymbolId>,
    pub before_qualified_name: Option<String>,
    pub after_qualified_name: Option<String>,
    /// 展示名（after 优先）。
    pub name: String,
    pub kind: SymbolKind,
    pub change_type: SymbolChangeType,
    /// 排序去重后的特征标志（确定性）。
    pub characteristics: Vec<SymbolCharacteristic>,
    pub location_before: Option<LineRange>,
    pub location_after: Option<LineRange>,
    /// 支撑本结论的 hunk 行区间（§53）。
    pub changed_ranges: Vec<crate::model::ChangedRange>,
    pub mapping_status: MappingStatus,
    pub confidence: MatchConfidence,
    pub evidence: ChangeEvidence,
}

impl SymbolChange {
    /// 对外排序键：(path, name, change_type) —— 确定性输出（§63）。
    pub fn sort_key(&self) -> (&str, &str, SymbolChangeType) {
        (self.path.as_str(), self.name.as_str(), self.change_type)
    }

    /// LineRange 从 SourceRange（end 独占）取展示区间：两端含入。
    /// end.column == 1 表示最后一行不被包含（坐标契约，M2 model.rs）。
    pub fn line_range(start_line: u32, end_line: u32, end_column: u32) -> LineRange {
        let end = if end_column <= 1 && end_line > start_line {
            end_line - 1
        } else {
            end_line
        };
        LineRange {
            start_line,
            end_line: end,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enums_serialize_snake_case() {
        assert_eq!(
            serde_json::to_string(&SymbolChangeType::Renamed).unwrap(),
            "\"renamed\""
        );
        assert_eq!(
            serde_json::to_string(&SymbolCharacteristic::SignatureChanged).unwrap(),
            "\"signature_changed\""
        );
    }

    #[test]
    fn line_range_respects_exclusive_end() {
        // end 指向下一行第 1 列 ⇒ 最后一行不含。
        let r = SymbolChange::line_range(20, 46, 1);
        assert_eq!(
            r,
            LineRange {
                start_line: 20,
                end_line: 45
            }
        );
        // end 与 start 同行（单行符号）不收缩。
        let single = SymbolChange::line_range(7, 8, 1);
        assert_eq!(
            single,
            LineRange {
                start_line: 7,
                end_line: 7
            }
        );
        // end.column > 1 ⇒ end 行包含。
        let wide = SymbolChange::line_range(20, 45, 12);
        assert_eq!(
            wide,
            LineRange {
                start_line: 20,
                end_line: 45
            }
        );
    }
}
