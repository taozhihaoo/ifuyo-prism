//! CI 配置静态检视（规格 M8 §64-§69/§131-§135/§265-§266）。
//!
//! **CI inspection ≠ CI execution**（§65/§133/§242）：只读取声明，
//! 绝不执行 CI 中的命令，也绝不远程触发（§65/§178）。
//!
//! 第一版只支持 GitHub Actions 的最小静态提取器（§265）：扫描
//! `.github/workflows/*.yml` 中 `jobs.<job>.steps[].run:` 命令，
//! 记录 file:line 证据（§266）。非完整 YAML 引擎；无法解析的行被
//! 忽略（§289：解析失败不影响本地 Build/Test）。

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// 一条 CI 检查（§131/§134）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CICheck {
    /// 所在 job 名。
    pub job: String,
    /// 提取的命令文本（原样；只记录不执行）。
    pub command: String,
    /// 证据位置：`path:line`（§266）。
    pub location: String,
}

/// CI 配置检视结果（§64/§67/§69）。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CIInspection {
    pub provider: String,
    pub config_files: Vec<String>,
    pub checks: Vec<CICheck>,
    /// 解析诊断（malformed YAML 等；不影响本地验证，§289）。
    pub diagnostics: Vec<String>,
}

/// 扫描项目根的 GitHub Actions workflows（§265）。
pub fn inspect_github_actions(root: &Path) -> CIInspection {
    let mut inspection = CIInspection {
        provider: "github_actions".to_string(),
        ..Default::default()
    };
    let workflows = root.join(".github").join("workflows");
    let Ok(entries) = std::fs::read_dir(&workflows) else {
        return inspection;
    };
    let mut files: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            matches!(
                path.extension().and_then(|e| e.to_str()),
                Some("yml") | Some("yaml")
            )
        })
        .collect();
    files.sort();
    for path in files {
        let Ok(content) = std::fs::read_to_string(&path) else {
            continue;
        };
        let relative = path
            .strip_prefix(root)
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_default();
        inspection.config_files.push(relative.clone());
        extract_run_commands(&content, &relative, &mut inspection);
    }
    inspection
}

/// 保守的缩进式提取：`- run: <command>` 行 + 所属 job 名。
/// job 名 = 2 空格缩进 + 非空 + 以 ':' 结尾且位于 `jobs:` 之后的行。
fn extract_run_commands(content: &str, file: &str, inspection: &mut CIInspection) {
    let mut current_job = String::new();
    let mut in_jobs = false;
    for (index, raw_line) in content.lines().enumerate() {
        let line_number = index + 1;
        let trimmed = raw_line.trim_end();
        if trimmed == "jobs:" {
            in_jobs = true;
            continue;
        }
        if in_jobs && !trimmed.is_empty() {
            let indent = trimmed.len() - trimmed.trim_start().len();
            let text = trimmed.trim();
            // 顶层 job 名：2 空格缩进、`name:` 形式、非 step 键。
            if indent == 2 && text.ends_with(':') && !text.starts_with('-') {
                current_job = text.trim_end_matches(':').to_string();
            }
            // step run 命令：`- run: <cmd>` 或 `- name: ...` 后的 `run: <cmd>`。
            if let Some(command) = text.strip_prefix("- run:") {
                inspection.checks.push(CICheck {
                    job: current_job.clone(),
                    command: command.trim().to_string(),
                    location: format!("{file}:{line_number}"),
                });
            } else if let Some(command) = text.strip_prefix("run:") {
                inspection.checks.push(CICheck {
                    job: current_job.clone(),
                    command: command.trim().to_string(),
                    location: format!("{file}:{line_number}"),
                });
            }
        }
    }
}

/// 本地 vs CI 对照（§67/§69）：事实列表，不做 CI-is-wrong 判定（§66）。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationCoverage {
    /// 本地发现的 check id。
    pub local: Vec<String>,
    /// CI 中出现的命令。
    pub ci_commands: Vec<String>,
    /// 双侧都有（按 check id 与 CI 命令尾词粗对齐，事实性对照）。
    pub shared: Vec<String>,
    /// CI 独有命令（§67 标记 CI-only check，informational）。
    pub ci_only: Vec<String>,
}

pub fn compare_with_local(inspection: &CIInspection, local_ids: &[String]) -> VerificationCoverage {
    let local: Vec<String> = local_ids.to_vec();
    let mut shared = Vec::new();
    let mut ci_only = Vec::new();
    for check in &inspection.checks {
        let last = check
            .command
            .split_whitespace()
            .last()
            .unwrap_or("")
            .to_string();
        if local.iter().any(|id| id == &last || check.command.contains(last.as_str())) && !shared.contains(&last) {
            shared.push(last.clone());
        } else if !shared.contains(&last) && !ci_only.contains(&last) {
            ci_only.push(last);
        }
    }
    VerificationCoverage {
        local: local.to_vec(),
        ci_commands: inspection
            .checks
            .iter()
            .map(|check| check.command.clone())
            .collect(),
        shared,
        ci_only,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture_root(content: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "prism-ci-inspect-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.subsec_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(dir.join(".github/workflows")).unwrap();
        std::fs::write(dir.join(".github/workflows/ci.yml"), content).unwrap();
        dir
    }

    /// §196/§265/§266：job/step/command/位置 全部正确提取。
    #[test]
    fn github_actions_commands_extracted_with_evidence() {
        let dir = fixture_root(
            "name: ci\njobs:\n  test:\n    steps:\n      - run: npm test\n      - run: npm run lint\n  build:\n    steps:\n      - run: npm run build\n",
        );
        let inspection = inspect_github_actions(&dir);
        assert_eq!(inspection.provider, "github_actions");
        assert_eq!(inspection.checks.len(), 3);
        assert!(inspection
            .checks
            .iter()
            .any(|c| c.job == "test" && c.command == "npm run lint" && c.location.ends_with("ci.yml:6")));
        assert!(inspection
            .checks
            .iter()
            .any(|c| c.job == "build" && c.command == "npm run build"));
    }

    /// §289：malformed YAML ⇒ 诊断/空结果，不影响本地验证（这里只验证不 panic）。
    #[test]
    fn malformed_yaml_does_not_panic() {
        let dir = fixture_root("jobs: [\n  broken\n");
        let inspection = inspect_github_actions(&dir);
        // 保守提取器按行扫描，不 panic；结果可为空。
        let _ = inspection.checks.len();
    }
}
