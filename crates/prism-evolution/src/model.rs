//! Evolution 事实模型（M211 §15/§24/§25/§26）。
//!
//! 全部为**原始指标**（§25 推荐：changes/commits/churn/last_changed），
//! 无综合评分；窗口语义显式记录（§28/§52）。

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// M211 输出契约版本。
pub const EVOLUTION_SCHEMA_VERSION: u32 = 1;

/// 时间窗口声明（§28/§52：统计必须显式窗口，无"最近经常修改"）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum HistoryWindow {
    /// 最近 N 个 commit（bounded 历史的基础形态，§47）。
    RecentCommits { count: u64 },
}

impl HistoryWindow {
    pub fn as_label(&self) -> String {
        match self {
            HistoryWindow::RecentCommits { count } => format!("recent:{count}-commits"),
        }
    }
}

/// 单文件演化事实（§24 EvolutionFact 的文件级形态）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileEvolution {
    pub path: String,
    /// 触达该路径的 commit 数（--follow 回放，含 rename 前）。
    pub commit_count: u64,
    /// 变更记录数（rename 也计为一次变更）。
    pub change_count: u64,
    pub lines_added: u64,
    pub lines_deleted: u64,
    /// 去重排序贡献者（§23：只叫 contributor，不宣称 owner）。
    pub contributors: Vec<String>,
    pub first_committed_at: Option<String>,
    pub last_committed_at: Option<String>,
    /// 历史上的 rename 链（old→new，按时间序）。
    pub renames: Vec<String>,
}

/// 变更热点（§25：原始指标；无 Score）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hotspot {
    pub path: String,
    /// 窗口内触达的 commit 数。
    pub commit_count: u64,
    /// 变更记录数（含 rename）。
    pub change_count: u64,
    /// added + deleted 合计（churn）。
    pub churn: u64,
    pub lines_added: u64,
    pub lines_deleted: u64,
    pub author_count: u64,
    pub last_committed_at: Option<String>,
}

/// 变更耦合（§26/§27）：历史关系，**不是**语义依赖。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangeCoupling {
    /// 排序在前（字典序）的路径。
    pub path_a: String,
    pub path_b: String,
    /// 窗口内共同变更的 commit 数。
    pub co_change_count: u64,
    /// 证据 commit id（cap 5，§81 证据链）。
    pub evidence_commits: Vec<String>,
}

/// 分析报告（M28 规则层/CLI 的注入对象）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct EvolutionReport {
    pub schema_version: u32,
    pub window: Option<HistoryWindow>,
    /// 分析范围声明（§45/§55：不假设 git root == project root 时如实记录）。
    pub repository_root: Option<String>,
    pub files: BTreeMap<String, FileEvolution>,
    /// 热点按 (commit_count desc, churn desc, path asc) 排序（§50 确定性）。
    pub hotspots: Vec<Hotspot>,
    /// 耦合按 (co_change_count desc, path_a, path_b) 排序。
    pub couplings: Vec<ChangeCoupling>,
    /// 被截断标记（§70：不能静默丢数据）。
    pub truncated: bool,
    pub diagnostics: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_labels_are_explicit() {
        assert_eq!(
            HistoryWindow::RecentCommits { count: 200 }.as_label(),
            "recent:200-commits"
        );
    }

    #[test]
    fn report_defaults_deterministic() {
        let a = EvolutionReport::default();
        let b = EvolutionReport::default();
        assert_eq!(a, b);
    }
}
