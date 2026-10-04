//! 证据模型（2.0-M21 §12–§13）。
//!
//! 不变量：任何非平凡结论（Relation/Finding/Inference 类 Fact）都能
//! 关联证据，或显式标记 evidence 不可用；证据必须能定位来源。

use crate::identity::EntityId;
use serde::{Deserialize, Serialize};
use std::fmt;

/// 证据种类（M21 §12 清单）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    File,
    SourceRange,
    Git,
    Manifest,
    Config,
    Database,
    Command,
    Parser,
    Analyzer,
    Test,
    Runtime,
    UserProvided,
    /// 开放种类（adapter 贡献；点分/短横线命名约定）。
    Other(String),
}

/// 源码范围（行/列；允许只有部分信息）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceRange {
    pub path: String,
    pub start_line: u32,
    pub start_column: u32,
    pub end_line: u32,
    pub end_column: u32,
}

/// 证据：可定位、可序列化、可被机器消费（§37）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Evidence {
    pub id: EvidenceId,
    pub kind: EvidenceKind,
    /// 项目相对路径（`/` 分隔）；无文件来源时为 None。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// 精确范围（kind = SourceRange 时通常存在）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub range: Option<SourceRange>,
    /// 非文件来源的引用原样保留（commit id / 命令 / analyzer 名 / URI）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
    /// 为什么这条证据支持结论。
    pub note: String,
}

impl Evidence {
    /// 内容寻址 id（跨运行稳定；同证据去重的依据）。
    pub fn new(
        kind: EvidenceKind,
        path: Option<String>,
        range: Option<SourceRange>,
        reference: Option<String>,
        note: impl Into<String>,
    ) -> Self {
        let note = note.into();
        let fingerprint = format!(
            "{kind:?}|{}|{}|{}|{note}",
            path.as_deref().unwrap_or(""),
            range
                .as_ref()
                .map(|r| format!(
                    "{}:{}:{}:{}:{}",
                    r.path, r.start_line, r.start_column, r.end_line, r.end_column
                ))
                .unwrap_or_default(),
            reference.as_deref().unwrap_or(""),
        );
        Evidence {
            id: EvidenceId(format!("ev:{}", crate::identity::fnv1a_hex16(&fingerprint))),
            kind,
            path,
            range,
            reference,
            note,
        }
    }

    /// 无证据的显式表达（§58：允许 unavailable，禁止假装有证据）。
    pub fn unavailable(reason: impl Into<String>) -> Self {
        let reason = reason.into();
        Evidence {
            id: EvidenceId(format!(
                "ev:unavailable:{}",
                crate::identity::fnv1a_hex16(&reason)
            )),
            kind: EvidenceKind::Other("unavailable".to_string()),
            path: None,
            range: None,
            reference: None,
            note: reason,
        }
    }
}

/// 证据 ID（内容寻址）。
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct EvidenceId(String);

impl EvidenceId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for EvidenceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// 证据锚：模型中引用证据的方式（id + 可选实体 target）。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct EvidenceAnchor {
    pub evidence_id: EvidenceId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject: Option<EntityId>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evidence_id_is_content_addressed_and_deduplicable() {
        let a = Evidence::new(
            EvidenceKind::SourceRange,
            Some("src/a.py".into()),
            Some(SourceRange {
                path: "src/a.py".into(),
                start_line: 1,
                start_column: 0,
                end_line: 1,
                end_column: 9,
            }),
            None,
            "import statement",
        );
        let b = Evidence::new(
            EvidenceKind::SourceRange,
            Some("src/a.py".into()),
            Some(SourceRange {
                path: "src/a.py".into(),
                start_line: 1,
                start_column: 0,
                end_line: 1,
                end_column: 9,
            }),
            None,
            "import statement",
        );
        assert_eq!(a.id, b.id, "identical evidence must share one id");
        let c = Evidence::new(
            EvidenceKind::Git,
            None,
            None,
            Some("abc123".into()),
            "changed in commit",
        );
        assert_ne!(a.id, c.id);
    }

    #[test]
    fn round_trip_preserves_kind_and_note() {
        let e = Evidence::new(
            EvidenceKind::Analyzer,
            None,
            None,
            Some("dep_resolver".into()),
            "resolved via ts resolver",
        );
        let text = serde_json::to_string(&e).unwrap();
        let back: Evidence = serde_json::from_str(&text).unwrap();
        assert_eq!(back, e);
    }
}
