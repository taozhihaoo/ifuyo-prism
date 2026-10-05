//! 环境声明与观测对照（M29 §37-§42/§84）。
//!
//! Declared ≠ Observed（§38）：`global.json: 8.0.x` 与 `dotnet 9.x` 同时
//! 为真时是声明/观测两种事实；不直接判 Build Broken（结合能力验证）。
//! Godot：project.godot 的 features 只是**声明提示**（§84）——机器版本
//! 必须来自真实观测。

use serde::{Deserialize, Serialize};

/// 项目声明的工具链要求（来源均为项目文件，Origin=Fact）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnvironmentDeclaration {
    /// 工具逻辑名（与 prism-env allowlist 同词表：dotnet/godot/...）。
    pub tool: String,
    /// 原样约束（`8.0.100` / `net8.0` / `4.3`）。
    pub constraint: String,
    pub source_path: String,
    pub line: Option<usize>,
}

/// 对照状态（§41：区分 Drift/Mismatch/Missing/Unknown，不统一 Error）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolchainState {
    /// 观测版本满足声明（精确或声明为提示性版本）。
    Match,
    /// 声明是范围/前瞻（rollForward），观测不违背。
    RangeSatisfied,
    /// 主版本不一致等明确不匹配。
    Mismatch,
    /// 声明了但机器上未观测到。
    Missing,
    /// 无法观测（工具探测失败/不在 allowlist）。
    Unknown,
}

/// 一条对照结论（§41/§38：可解释，非单一 EnvironmentError）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolchainComparison {
    pub tool: String,
    pub declared: Option<String>,
    pub observed: Option<String>,
    pub state: ToolchainState,
    pub detail: String,
}

/// 声明 × 观测 对照（§38/§41）。
///
/// `observed`：来自真实探测的工具事实（name → raw version 字符串；
/// 探测不到 ⇒ 不在 map 中）。
pub fn compare_declared_observed(
    declarations: &[EnvironmentDeclaration],
    observed: &std::collections::BTreeMap<String, String>,
) -> Vec<ToolchainComparison> {
    let mut out = Vec::new();
    for declaration in declarations {
        let observed_version = observed.get(&declaration.tool);
        let (state, detail) = match observed_version {
            None => (
                ToolchainState::Missing,
                format!(
                    "declared `{}` in {} but tool not observed on PATH",
                    declaration.constraint, declaration.source_path
                ),
            ),
            Some(version) => {
                let satisfies = crate::version::version_satisfies(version, &declaration.constraint);
                let prefix_satisfies =
                    version_satisfies_as_prefix(version, &declaration.constraint);
                if satisfies || prefix_satisfies {
                    (
                        ToolchainState::Match,
                        format!(
                            "declared `{}` vs observed `{version}`",
                            declaration.constraint
                        ),
                    )
                } else {
                    // 主版本不同 ⇒ Mismatch；其余 ⇒ 保守 Unknown（不误报）。
                    let declared_major = major_of(&declaration.constraint);
                    let observed_major = major_of(version);
                    if let (Some(d), Some(o)) = (declared_major, observed_major) {
                        if d != o {
                            (
                                ToolchainState::Mismatch,
                                format!(
                                    "declared `{}` vs observed `{version}` (major differs)",
                                    declaration.constraint
                                ),
                            )
                        } else {
                            (
                                ToolchainState::RangeSatisfied,
                                format!(
                                    "declared `{}` vs observed `{version}` (same major, minor drift)",
                                    declaration.constraint
                                ),
                            )
                        }
                    } else {
                        (
                            ToolchainState::Unknown,
                            format!(
                                "declared `{}` vs observed `{version}` — constraint not comparable",
                                declaration.constraint
                            ),
                        )
                    }
                }
            }
        };
        out.push(ToolchainComparison {
            tool: declaration.tool.clone(),
            declared: Some(declaration.constraint.clone()),
            observed: observed_version.cloned(),
            state,
            detail,
        });
    }
    out.sort_by(|a, b| a.tool.cmp(&b.tool));
    out
}

/// `net8.0` vs `9.x` 这类前缀形态：net 主版本号提取。
fn major_of(version: &str) -> Option<u64> {
    version
        .trim_start_matches(|c: char| c.is_ascii_alphabetic())
        .split('.')
        .next()
        .and_then(|part| part.parse().ok())
}

/// `net8.0` vs `8.0.404`：把 netN.M 视作 N.M 前缀匹配。
fn version_satisfies_as_prefix(observed: &str, declared: &str) -> bool {
    let Some(declared_rest) = declared.strip_prefix("net") else {
        return false;
    };
    let declared_parts: Vec<&str> = declared_rest.split('.').collect();
    if declared_parts.is_empty() {
        return false;
    }
    let observed_rest = observed.trim_start_matches(|c: char| c.is_ascii_alphabetic());
    let observed_parts: Vec<&str> = observed_rest.split('.').collect();
    declared_parts
        .iter()
        .zip(observed_parts.iter())
        .all(|(d, o)| d == o)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn declaration(tool: &str, constraint: &str) -> EnvironmentDeclaration {
        EnvironmentDeclaration {
            tool: tool.to_string(),
            constraint: constraint.to_string(),
            source_path: "global.json".to_string(),
            line: Some(2),
        }
    }

    #[test]
    fn declared_8_observed_9_is_mismatch_not_build_broken() {
        // §38 示例：global.json 8.0.x + SDK 9.x = Declared ≠ Observed，
        // 是 Mismatch 事实，不是"Build Broken"判决。
        let mut observed = std::collections::BTreeMap::new();
        observed.insert("dotnet".to_string(), "9.0.100".to_string());
        let comparisons = compare_declared_observed(&[declaration("dotnet", "8.0.100")], &observed);
        assert_eq!(comparisons[0].state, ToolchainState::Mismatch);
        assert!(comparisons[0].detail.contains("major differs"));
    }

    #[test]
    fn matching_and_missing_states() {
        let mut observed = std::collections::BTreeMap::new();
        observed.insert("dotnet".to_string(), "8.0.100".to_string());
        let comparisons = compare_declared_observed(
            &[
                declaration("dotnet", "8.0.100"),
                declaration("godot", "4.3"),
            ],
            &observed,
        );
        assert_eq!(comparisons[0].state, ToolchainState::Match);
        assert_eq!(
            comparisons[1].state,
            ToolchainState::Missing,
            "声明了但未观测 ⇒ Missing"
        );
    }

    #[test]
    fn net_framework_prefix_matches_runtime_version() {
        // csproj TargetFramework net8.0 vs dotnet 8.0.404 ⇒ 前缀满足。
        let mut observed = std::collections::BTreeMap::new();
        observed.insert("dotnet".to_string(), "8.0.404".to_string());
        let comparisons = compare_declared_observed(
            &[EnvironmentDeclaration {
                tool: "dotnet".to_string(),
                constraint: "net8.0".to_string(),
                source_path: "src/App.csproj".to_string(),
                line: Some(3),
            }],
            &observed,
        );
        assert_eq!(comparisons[0].state, ToolchainState::Match);
    }

    #[test]
    fn unobserved_tool_is_unknown_state_not_error() {
        let comparisons = compare_declared_observed(
            &[declaration("dotnet", "8.0.100")],
            &std::collections::BTreeMap::new(),
        );
        assert_eq!(comparisons[0].state, ToolchainState::Missing);
        assert!(
            comparisons[0].observed.is_none(),
            "观测侧如实为 None（§17）"
        );
    }
}
