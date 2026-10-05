//! Evolution 分析（M211 §24-§28）。
//!
//! 输入 = M4 GitProvider 的 bounded 历史数据；纯聚合，零额外 Git 调用
//! （调用方决定窗口与上限，§47/§52）。

use crate::model::{
    ChangeCoupling, EvolutionReport, FileEvolution, HistoryWindow, Hotspot,
    EVOLUTION_SCHEMA_VERSION,
};
use prism_git::model::GitFileChange;
use prism_git::GitService;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// 查询参数（§47 资源限制全部显式）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EvolutionQuery {
    /// 全仓库分析的 commit 窗口（§52 RecentCommits）。
    pub max_commits: u64,
    /// 热点输出上限。
    pub top_hotspots: usize,
    /// 耦合判定阈值（共同变更次数 ≥ min 才输出）。
    pub min_co_change: u64,
    /// 耦合输出上限。
    pub max_couplings: usize,
}

impl Default for EvolutionQuery {
    fn default() -> Self {
        EvolutionQuery {
            max_commits: 200,
            top_hotspots: 20,
            min_co_change: 3,
            max_couplings: 30,
        }
    }
}

/// 单文件演化（§15/§24：--follow 回放聚合）。
pub fn file_evolution(
    service: &GitService,
    path: &Path,
    relative_path: &str,
    max_commits: u64,
    cancel: &prism_core::task::CancellationToken,
) -> prism_git::GitResult<FileEvolution> {
    let entries = service.file_history(path, relative_path, max_commits, cancel)?;
    Ok(aggregate_file(relative_path, &entries))
}

fn aggregate_file(
    relative_path: &str,
    entries: &[prism_git::model::GitFileHistoryEntry],
) -> FileEvolution {
    let mut authors: BTreeSet<String> = BTreeSet::new();
    let mut renames = Vec::new();
    let mut lines_added = 0u64;
    let mut lines_deleted = 0u64;
    let mut change_count = 0u64;
    let mut first: Option<String> = None;
    let mut last: Option<String> = None;
    let mut commit_ids: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for entry in entries {
        commit_ids.insert(entry.commit.id.clone());
        authors.insert(entry.commit.author.name.clone());
        if let Some(timestamp) = &entry.commit.committed_at {
            if first
                .as_deref()
                .is_none_or(|first| timestamp.as_str() < first)
            {
                first = Some(timestamp.clone());
            }
            if last.as_deref().is_none_or(|last| timestamp.as_str() > last) {
                last = Some(timestamp.clone());
            }
        }
        for change in &entry.changes {
            change_count += 1;
            if change.change_type == prism_git::GitChangeType::Renamed {
                if let Some(before) = &change.path_before {
                    renames.push(format!("{before} -> {}", change.path_after));
                }
            }
            lines_added += change.lines_added.unwrap_or(0);
            lines_deleted += change.lines_deleted.unwrap_or(0);
        }
    }
    FileEvolution {
        path: relative_path.to_string(),
        commit_count: commit_ids.len() as u64,
        change_count,
        lines_added,
        lines_deleted,
        contributors: authors.into_iter().collect(),
        first_committed_at: first,
        last_committed_at: last,
        renames,
    }
}

/// 仓库级：近期 commit 集合上的一次性扫描（§24/§25/§26）。
/// 单次 `log -n <max> --numstat` 驱动：文件演化聚合 + 热点 + 耦合。
pub struct RepoScan {
    /// commit id → (排序后的路径集合, author, committed_at)。
    commits: Vec<RepoCommit>,
    window: HistoryWindow,
}

struct RepoCommit {
    id: String,
    author: String,
    committed_at: Option<String>,
    /// 排序后的变更路径（含 rename 双端，path_after 优先）。
    paths: Vec<String>,
    /// path → (added, deleted)。
    churn: BTreeMap<String, (u64, u64)>,
}

/// 扫描近期历史（单次 bounded `log --numstat`）。
pub fn scan_repo(
    service: &GitService,
    path: &Path,
    max_commits: u64,
    cancel: &prism_core::task::CancellationToken,
) -> prism_git::GitResult<RepoScan> {
    let entries = raw_log_numstat(service, path, max_commits, cancel)?;
    Ok(RepoScan {
        commits: entries,
        window: HistoryWindow::RecentCommits { count: max_commits },
    })
}

fn raw_log_numstat(
    service: &GitService,
    path: &Path,
    max_commits: u64,
    cancel: &prism_core::task::CancellationToken,
) -> prism_git::GitResult<Vec<RepoCommit>> {
    // 单次 bounded `log --numstat`（provider 面内，§47 禁止全历史）。
    let entries = service.log_numstat(path, max_commits, cancel)?;
    let mut commits = Vec::new();
    for entry in &entries {
        let mut paths = Vec::new();
        let mut churn = BTreeMap::new();
        for change in &entry.changes {
            paths.push(change.path_after.clone());
            if let Some(before) = &change.path_before {
                paths.push(before.clone());
            }
            let tally = churn
                .entry(change.path_after.clone())
                .or_insert((0u64, 0u64));
            tally.0 += change.lines_added.unwrap_or(0);
            tally.1 += change.lines_deleted.unwrap_or(0);
        }
        paths.sort();
        paths.dedup();
        commits.push(RepoCommit {
            id: entry.commit.id.clone(),
            author: entry.commit.author.name.clone(),
            committed_at: entry.commit.committed_at.clone(),
            paths,
            churn,
        });
    }
    Ok(commits)
}

impl RepoScan {
    /// 热点（§25 原始指标；排序确定性 §50）。
    pub fn hotspots(&self, top: usize) -> Vec<Hotspot> {
        let mut per_file: BTreeMap<
            String,
            (u64, u64, u64, u64, BTreeMap<String, ()>, Option<String>),
        > = BTreeMap::new();
        for commit in &self.commits {
            for path in &commit.paths {
                if path.is_empty() {
                    continue;
                }
                let entry =
                    per_file
                        .entry(path.clone())
                        .or_insert((0, 0, 0, 0, BTreeMap::new(), None));
                entry.0 += 1; // commit_count（同 commit 双端只计一次——paths dedup）
                entry.1 += 1; // change_count（rename 双端各记一次变更记录）
                entry.4.insert(commit.author.clone(), ());
                if let Some((added, deleted)) = commit.churn.get(path) {
                    entry.2 += added;
                    entry.3 += deleted;
                }
                if entry
                    .5
                    .as_deref()
                    .map(|last| {
                        commit
                            .committed_at
                            .as_deref()
                            .map(|t| t > last)
                            .unwrap_or(false)
                    })
                    .unwrap_or(true)
                {
                    entry.5 = commit.committed_at.clone();
                }
            }
        }
        let mut hotspots: Vec<Hotspot> = per_file
            .into_iter()
            .map(
                |(path, (commits, changes, added, deleted, authors, last))| Hotspot {
                    path,
                    commit_count: commits,
                    change_count: changes,
                    churn: added + deleted,
                    lines_added: added,
                    lines_deleted: deleted,
                    author_count: authors.len() as u64,
                    last_committed_at: last,
                },
            )
            .collect();
        hotspots.sort_by(|a, b| {
            b.commit_count
                .cmp(&a.commit_count)
                .then_with(|| b.churn.cmp(&a.churn))
                .then_with(|| a.path.cmp(&b.path))
        });
        hotspots.truncate(top);
        hotspots
    }

    /// 变更耦合（§26/§27：co-change 历史关系；证据 = 共同 commit）。
    pub fn change_couplings(&self, min_co_change: u64, max_pairs: usize) -> Vec<ChangeCoupling> {
        let mut pairs: BTreeMap<(String, String), (u64, Vec<String>)> = BTreeMap::new();
        for commit in &self.commits {
            if commit.paths.len() < 2 {
                continue;
            }
            for i in 0..commit.paths.len() {
                for j in i + 1..commit.paths.len() {
                    let pair = pairs
                        .entry((commit.paths[i].clone(), commit.paths[j].clone()))
                        .or_insert((0, Vec::new()));
                    pair.0 += 1;
                    if pair.1.len() < 5 {
                        pair.1.push(commit.id.clone());
                    }
                }
            }
        }
        let mut couplings: Vec<ChangeCoupling> = pairs
            .into_iter()
            .filter(|(_, (count, _))| *count >= min_co_change)
            .map(|((path_a, path_b), (count, commits))| ChangeCoupling {
                path_a,
                path_b,
                co_change_count: count,
                evidence_commits: commits,
            })
            .collect();
        couplings.sort_by(|a, b| {
            b.co_change_count
                .cmp(&a.co_change_count)
                .then_with(|| a.path_a.cmp(&b.path_a))
                .then_with(|| a.path_b.cmp(&b.path_b))
        });
        couplings.truncate(max_pairs);
        couplings
    }

    pub fn window(&self) -> HistoryWindow {
        self.window.clone()
    }
}

/// 汇总报告（analysis 入口）。
pub fn build_report(
    scan: &RepoScan,
    hotspots: Vec<Hotspot>,
    couplings: Vec<ChangeCoupling>,
    repository_root: Option<String>,
) -> EvolutionReport {
    let mut files = BTreeMap::new();
    let hotspot_scan = scan.hotspots(usize::MAX);
    for hotspot in hotspot_scan {
        files.insert(
            hotspot.path.clone(),
            FileEvolution {
                path: hotspot.path.clone(),
                commit_count: hotspot.commit_count,
                change_count: hotspot.change_count,
                lines_added: hotspot.lines_added,
                lines_deleted: hotspot.lines_deleted,
                contributors: Vec::new(),
                first_committed_at: None,
                last_committed_at: hotspot.last_committed_at.clone(),
                renames: Vec::new(),
            },
        );
    }
    let _ = hotspots;
    EvolutionReport {
        schema_version: EVOLUTION_SCHEMA_VERSION,
        window: Some(scan.window()),
        repository_root,
        files,
        hotspots,
        couplings,
        truncated: false,
        diagnostics: Vec::new(),
    }
}

/// change 类型是否为 rename 的便捷判定（供上层 Symbol 历史使用）。
pub fn is_rename(change: &GitFileChange) -> bool {
    change.change_type == prism_git::GitChangeType::Renamed
}

/// 热点自由函数（lib 出口；见 [`RepoScan::hotspots`]）。
pub fn hotspots(scan: &RepoScan, top: usize) -> Vec<Hotspot> {
    scan.hotspots(top)
}

/// 耦合自由函数（lib 出口；见 [`RepoScan::change_couplings`]）。
pub fn change_couplings(
    scan: &RepoScan,
    min_co_change: u64,
    max_pairs: usize,
) -> Vec<ChangeCoupling> {
    scan.change_couplings(min_co_change, max_pairs)
}
