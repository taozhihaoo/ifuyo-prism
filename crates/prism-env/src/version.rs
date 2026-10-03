//! 结构化版本（规格 M9a §28–§29/§204）。
//!
//! 不做字符串比较（"10" > "9" 的经典错误）；无法解析的版本保留 raw，
//! 比较返回 Unknown（§158/§209 不猜）。

use crate::model::{MatchResult, RequirementOperator};
use serde::{Deserialize, Serialize};

/// 规范化版本（§28 ToolVersion：raw + major/minor/patch）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolVersion {
    pub raw: String,
    pub major: u64,
    pub minor: Option<u64>,
    pub patch: Option<u64>,
    /// 非语义化尾部（如 "windows.1"、构建哈希），只保存不参与数值比较（§29 保守）。
    pub extra: Option<String>,
}

impl ToolVersion {
    pub fn new(major: u64, minor: Option<u64>, patch: Option<u64>) -> Self {
        let mut raw = major.to_string();
        if let Some(minor) = minor {
            raw.push('.');
            raw.push_str(&minor.to_string());
            if let Some(patch) = patch {
                raw.push('.');
                raw.push_str(&patch.to_string());
            }
        }
        ToolVersion {
            raw,
            major,
            minor,
            patch,
            extra: None,
        }
    }
}

/// 从工具版本输出解析版本（§28：`node --version` → "v22.16.0"，
/// `git version 2.51.0.windows.1` → 2.51.0 + extra "windows.1"）。
/// 找不到数字开头的版本段 ⇒ None（调用方记 version unknown）。
pub fn parse_version_output(output: &str) -> Option<ToolVersion> {
    let trimmed = output.trim();
    for token in trimmed.split_whitespace() {
        if let Some(version) = parse_version_token(token) {
            return Some(version);
        }
    }
    None
}

/// 单 token 解析：容忍前缀 `v`/`V`/`ver.`；`1.2.3.4` 尾部进 extra。
pub fn parse_version_token(token: &str) -> Option<ToolVersion> {
    let body = token
        .strip_prefix('v')
        .or_else(|| token.strip_prefix('V'))
        .unwrap_or(token);
    let mut segments = body.split('.');
    let major = segments.next()?.parse::<u64>().ok()?;
    let mut version = ToolVersion {
        raw: token.to_string(),
        major,
        minor: None,
        patch: None,
        extra: None,
    };
    if let Some(minor_raw) = segments.next() {
        // "10.pre" 这类非数字次版本 ⇒ 只保留 major（§29 保守）。
        version.minor = Some(minor_raw.parse::<u64>().ok()?);
    }
    if let Some(patch_raw) = segments.next() {
        if let Ok(patch) = patch_raw.parse::<u64>() {
            version.patch = Some(patch);
        } else {
            version.extra = Some(patch_raw.to_string());
        }
    }
    // 第四段及以后（如 2.51.0.windows.1）整体进 extra（§29 非语义化尾部）。
    let rest: Vec<&str> = segments.collect();
    if !rest.is_empty() {
        let tail = rest.join(".");
        version.extra = Some(match version.extra.take() {
            Some(existing) => format!("{existing}.{tail}"),
            None => tail,
        });
    }
    Some(version)
}

use std::cmp::Ordering;

/// 数值比较（§204：20.1 < 20.10 < 22；缺省分量按 0 ⇒ 20 == 20.0.0）。
pub fn compare_versions(a: &ToolVersion, b: &ToolVersion) -> Ordering {
    a.major
        .cmp(&b.major)
        .then_with(|| a.minor.unwrap_or(0).cmp(&b.minor.unwrap_or(0)))
        .then_with(|| a.patch.unwrap_or(0).cmp(&b.patch.unwrap_or(0)))
}

/// 需求判定（§46–§50/§158）：
/// - 实际版本缺失（unknown）⇒ Unknown（不是 Unsatisfied，§158）
/// - 期望值解析失败 ⇒ Unknown（不猜）
pub fn satisfies(actual: Option<&ToolVersion>, operator: RequirementOperator, expected: &str) -> MatchResult {
    let Some(expected_version) = parse_version_token(expected) else {
        return MatchResult::Unknown;
    };
    let Some(actual) = actual else {
        return MatchResult::Unknown;
    };
    let ordering = compare_versions(actual, &expected_version);
    match operator {
        // AtLeast：actual >= expected ⇒ Satisfied（数值比较，§204）。
        RequirementOperator::AtLeast => match ordering {
            Ordering::Less => MatchResult::Unsatisfied,
            Ordering::Equal | Ordering::Greater => MatchResult::Satisfied,
        },
        RequirementOperator::AtMost => match ordering {
            Ordering::Less | Ordering::Equal => MatchResult::Satisfied,
            Ordering::Greater => MatchResult::Unsatisfied,
        },
        RequirementOperator::Equals => {
            if ordering == Ordering::Equal {
                MatchResult::Satisfied
            } else {
                MatchResult::Unsatisfied
            }
        }
        RequirementOperator::NotEquals => {
            if ordering != Ordering::Equal {
                MatchResult::Satisfied
            } else {
                MatchResult::Unsatisfied
            }
        }
        RequirementOperator::Exists | RequirementOperator::InRange => MatchResult::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // §204：避免字符串比较 bug 的四组关键用例。
    #[test]
    fn numeric_comparison_not_lexicographic() {
        let v20_1 = parse_version_token("20.1").unwrap();
        let v20_10 = parse_version_token("20.10").unwrap();
        assert_eq!(compare_versions(&v20_1, &v20_10), Ordering::Less, "20.1 < 20.10");

        let v20 = parse_version_token("20").unwrap();
        let v22 = parse_version_token("22").unwrap();
        assert_eq!(compare_versions(&v20, &v22), Ordering::Less);

        let v20_0_0 = parse_version_token("22.0.0").unwrap();
        assert_eq!(compare_versions(&v20_0_0, &parse_version_token("22").unwrap()), Ordering::Equal);
    }

    #[test]
    fn parses_common_tool_outputs() {
        let node = parse_version_output("v22.16.0").unwrap();
        assert_eq!(node.major, 22);
        assert_eq!(node.minor, Some(16));
        assert_eq!(node.patch, Some(0));

        let python = parse_version_output("Python 3.13.15").unwrap();
        assert_eq!(python.major, 3);
        assert_eq!(python.minor, Some(13));

        let git = parse_version_output("git version 2.51.0.windows.1").unwrap();
        assert_eq!(git.major, 2);
        assert_eq!(git.minor, Some(51));
        assert_eq!(git.patch, Some(0));
        assert_eq!(git.extra.as_deref(), Some("windows.1"));

        let cargo = parse_version_output("cargo 1.90.0 (a0b3c6a6a 2025-07-14)").unwrap();
        assert_eq!(cargo.major, 1);
        assert_eq!(cargo.minor, Some(90));
    }

    #[test]
    fn unparseable_output_is_none() {
        assert!(parse_version_output("").is_none());
        assert!(parse_version_output("no version here").is_none());
        assert!(parse_version_token("beta").is_none());
    }

    #[test]
    fn at_least_semantics_with_unknown() {
        let v20 = parse_version_token("20.1.0").unwrap();
        let v23 = parse_version_token("23.0.0").unwrap();
        assert_eq!(satisfies(Some(&v23), RequirementOperator::AtLeast, "22"), MatchResult::Satisfied);
        assert_eq!(satisfies(Some(&v20), RequirementOperator::AtLeast, "22"), MatchResult::Unsatisfied);
        // §158：exists 但 version unknown ⇒ Unknown，不是 Unsatisfied。
        assert_eq!(satisfies(None, RequirementOperator::AtLeast, "22"), MatchResult::Unknown);
        // 期望值不是版本 ⇒ Unknown（不猜）。
        assert_eq!(satisfies(Some(&v20), RequirementOperator::AtLeast, "beta"), MatchResult::Unknown);
    }
}
