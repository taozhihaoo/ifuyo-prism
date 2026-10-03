//! Severity 与 Confidence。
//!
//! 依开发总纲领 §11：对外 Severity 只允许 `Info / Warning / Critical` 三级；
//! 内部实现可另设 P0–P3 优先级，但不得进入对外模型。UI 不制造“项目评分”。
//!
//! Confidence 与 Severity 相互独立；两者都使用分类式枚举，
//! 禁止虚构百分比置信度。

/// 问题严重程度（对外三级，JSON 为小写字符串）。
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Info,
    Warning,
    Critical,
}

/// 证据可信度（分类式，与 Severity 独立）。
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum Confidence {
    Low,
    Medium,
    High,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn severity_serializes_lowercase() {
        assert_eq!(serde_json::to_string(&Severity::Info).unwrap(), "\"info\"");
        assert_eq!(
            serde_json::to_string(&Severity::Warning).unwrap(),
            "\"warning\""
        );
        assert_eq!(
            serde_json::to_string(&Severity::Critical).unwrap(),
            "\"critical\""
        );
        let back: Severity = serde_json::from_str("\"warning\"").unwrap();
        assert_eq!(back, Severity::Warning);
    }

    #[test]
    fn confidence_serializes_lowercase() {
        assert_eq!(
            serde_json::to_string(&Confidence::High).unwrap(),
            "\"high\""
        );
    }

    #[test]
    fn severity_ordering_is_info_lt_warning_lt_critical() {
        assert!(Severity::Info < Severity::Warning);
        assert!(Severity::Warning < Severity::Critical);
    }

    #[test]
    fn confidence_and_severity_are_independent_types() {
        // 编译期即可证明二者不可混淆；此测试只是把该约束固定下来。
        fn takes_severity(_: Severity) {}
        takes_severity(Severity::Critical);
    }
}

/// 分析完整度（§100；M6 ImpactCompleteness 同义，M7 起统一从 core 引用）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrismCompleteness {
    /// 全部请求的分析完成。
    Complete,
    /// 部分源/映射不可用，但结果有效。
    Partial,
    /// 达到上限被截断。
    Truncated,
    /// 无法产生有效结果。
    Failed,
}
