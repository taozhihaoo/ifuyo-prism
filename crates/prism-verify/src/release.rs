//! Release 检查（规格 M8 §62/§136-§139/§243-§245/§308-§309）。
//!
//! 只基于已有事实（M4 GitStatus + required files + 前置 CheckRuns）判定；
//! 无部署（§63/§245）、永不自动 `git clean`（§139/§171）、不发神秘分数（§61）。

use prism_core::task::CancellationToken;
use prism_git::GitService;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// 一条 release 检查结果（§62：PASS/FAIL/BLOCKED + 证据）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseCheck {
    pub id: String,
    pub title: String,
    /// passed / failed / blocked
    pub state: String,
    pub detail: String,
}

/// Release 验证汇总（§61：PASS/FAIL/BLOCKED + 证据；无分数）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseReport {
    /// PASS / FAIL / BLOCKED
    pub conclusion: String,
    pub checks: Vec<ReleaseCheck>,
}

/// 评估 release 检查（§136：version present / required files / working tree
/// clean / build+test passed——build/test 结论由调用方以 CheckRuns 传入）。
#[allow(clippy::too_many_arguments)]
pub fn evaluate_release(
    root: &Path,
    git: Option<&GitService>,
    required_files: &[String],
    require_clean_tree: bool,
    build_passed: Option<bool>,
    test_passed: Option<bool>,
    cancel: &CancellationToken,
) -> ReleaseReport {
    let mut checks: Vec<ReleaseCheck> = Vec::new();

    // ---- required files（§138）----
    for file in required_files {
        let state = if cancel.is_cancelled() {
            "blocked"
        } else if root.join(file).is_file() {
            "passed"
        } else {
            "failed"
        };
        checks.push(ReleaseCheck {
            id: format!("required_file:{file}"),
            title: format!("Required file present: {file}"),
            state: state.to_string(),
            detail: if state == "passed" {
                format!("{file} exists")
            } else {
                format!("{file} is missing")
            },
        });
    }

    // ---- working tree clean（§139：只报告，永不 git clean）----
    if require_clean_tree {
        let state = match git {
            Some(git) => match git.status(root, false, cancel) {
                Ok(status) if status.entries.is_empty() => "passed".to_string(),
                Ok(status) => {
                    format!("failed ({} working-tree change(s))", status.entries.len())
                }
                Err(err) => format!("blocked ({err})"),
            },
            None => "blocked (git unavailable)".to_string(),
        };
        let passed = state == "passed";
        checks.push(ReleaseCheck {
            id: "git_clean".to_string(),
            title: "Working tree clean".to_string(),
            state: state.clone(),
            detail: if passed {
                "working tree has no changes".to_string()
            } else {
                "working tree is dirty; Prism never runs git clean".to_string()
            },
        });
    }

    // ---- build/test passed（来自 CheckRuns 的事实，§62）----
    if let Some(passed) = build_passed {
        checks.push(ReleaseCheck {
            id: "build_passed".to_string(),
            title: "Build passed".to_string(),
            state: if passed { "passed" } else { "failed" }.to_string(),
            detail: if passed {
                "build check passed".to_string()
            } else {
                "build check failed".to_string()
            },
        });
    }
    if let Some(passed) = test_passed {
        checks.push(ReleaseCheck {
            id: "test_passed".to_string(),
            title: "Tests passed".to_string(),
            state: if passed { "passed" } else { "failed" }.to_string(),
            detail: if passed {
                "test check passed".to_string()
            } else {
                "test check failed".to_string()
            },
        });
    }

    let conclusion = if checks.iter().any(|check| check.state == "failed") {
        "FAIL"
    } else if checks.iter().any(|check| check.state == "blocked") {
        // 同为 BLOCKED 分支曾重复——保留独立分支以明确三种结论。
        "BLOCKED"
    } else if checks.is_empty() {
        "BLOCKED"
    } else {
        "PASS"
    };

    ReleaseReport {
        conclusion: conclusion.to_string(),
        checks,
    }
}

/// 从 CheckRuns 提取 build/test 结论（§62：真实执行事实）。
pub fn build_test_conclusions(runs: &[crate::model::CheckRun]) -> (Option<bool>, Option<bool>) {
    let mut build = None;
    let mut test = None;
    for run in runs {
        match run.kind {
            crate::model::CheckKind::Build => {
                build = Some(run.state == crate::model::CheckState::Passed);
            }
            crate::model::CheckKind::Test => {
                test = Some(run.state == crate::model::CheckState::Passed);
            }
            _ => {}
        }
    }
    (build, test)
}
