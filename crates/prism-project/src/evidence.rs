//! Evidence 模型：让检测结果回答“为什么”。
//!
//! M1 的项目类型 / 配置 / 入口点检测都必须附带证据；
//! 未来 M2/M3/Health 复用同一结构（以本定义为准，允许后续扩展字段）。

use serde::{Deserialize, Serialize};

/// 一条可定位的证据。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Evidence {
    /// 证据来源（如 "project_type_detector" / "entry_point_detector"）。
    pub source: String,
    /// 证据指向的项目相对路径（`/` 分隔）。
    pub path: String,
    /// 为什么这条路径支持该结论。
    pub reason: String,
}

impl Evidence {
    pub fn new(
        source: impl Into<String>,
        path: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Evidence {
            source: source.into(),
            path: path.into(),
            reason: reason.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_with_snake_case_fields() {
        let evidence = Evidence::new("project_type_detector", "Cargo.toml", "rust manifest found");
        let json = serde_json::to_value(&evidence).unwrap();
        assert_eq!(json["source"], "project_type_detector");
        assert_eq!(json["path"], "Cargo.toml");
        assert_eq!(json["reason"], "rust manifest found");
    }
}
