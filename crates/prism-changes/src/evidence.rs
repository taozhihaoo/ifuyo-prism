//! 变更证据链（规格 M5 §53/§54）：每个结论可追溯 file → hunk → line range。
//!
//! 证据自包含（不要求持有原始 GitDiff 即可理解），但不含源码内容与
//! diff 正文（§95：日志/报告不携带源码）。

use serde::{Deserialize, Serialize};

/// 证据来源标识（稳定字符串，机器可读）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceSource {
    /// 来自 Git diff 的 hunk 行区间。
    GitDiff,
    /// 来自 before/after 符号 range 与变更行的重叠。
    SymbolRangeOverlap,
    /// 来自 Git rename 元数据（相似度/路径对）。
    GitRename,
    /// 来自 before/after 解析结果的存在性对比。
    ParseResult,
    /// 来自 M3 模块事实（FileModule/namespace 派生）。
    ModuleFacts,
}

/// 一条变更的证据。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangeEvidence {
    pub source: EvidenceSource,
    /// 一句话解释这条证据为何支持该结论（不含源码内容）。
    pub detail: String,
    /// 关联的 FileChange id（符号/模块级证据回溯到文件）。
    pub file_change_id: Option<String>,
    /// 受影响的 hunk 行区间（old/new 两侧）。
    pub ranges: Vec<crate::model::ChangedRange>,
}

impl ChangeEvidence {
    pub fn new(source: EvidenceSource, detail: impl Into<String>) -> Self {
        ChangeEvidence {
            source,
            detail: detail.into(),
            file_change_id: None,
            ranges: Vec::new(),
        }
    }

    pub fn with_file(mut self, file_change_id: impl Into<String>) -> Self {
        self.file_change_id = Some(file_change_id.into());
        self
    }

    pub fn with_ranges(mut self, ranges: Vec<crate::model::ChangedRange>) -> Self {
        self.ranges = ranges;
        self
    }
}

/// ChangeEvidence 序列化形态稳定（§63 stable field names）。
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evidence_keeps_file_link_and_ranges() {
        let evidence = ChangeEvidence::new(EvidenceSource::SymbolRangeOverlap, "hunk lines 31-34 overlap Foo.bar")
            .with_file("fc:src/a.ts")
            .with_ranges(vec![crate::model::ChangedRange {
                old_start: Some(31),
                old_lines: 4,
                new_start: Some(31),
                new_lines: 4,
            }]);
        let json = serde_json::to_value(&evidence).unwrap();
        assert_eq!(json["source"], "symbol_range_overlap");
        assert_eq!(json["file_change_id"], "fc:src/a.ts");
        assert!(json["ranges"].as_array().unwrap().len() == 1);
    }
}
