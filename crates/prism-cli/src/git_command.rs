//! `prism git` 子命令（M4 Git facts，只读）。
//!
//! 经 GitService（与 GUI 同源管线）读取 facts；JSON 输出直接序列化
//! domain model（规格 §37）。非 Git 项目：明确输出 + 专门 exit code（§39）。

use clap::Subcommand;
use prism_core::runtime::command::CommandRunner;
use prism_core::task::CancellationToken;
use prism_git::{GitDiffScope, GitError, GitService};
use std::path::Path;
use std::sync::Arc;

#[derive(Debug, Subcommand)]
pub enum GitCommand {
    /// 工作区状态（branch/HEAD/staged/unstaged/untracked）
    Status {
        #[arg(default_value = ".")]
        path: std::path::PathBuf,
        /// JSON 输出
        #[arg(long)]
        json: bool,
        /// 同时列出 ignored 文件
        #[arg(long)]
        ignored: bool,
    },
    /// M211：文件演化事实（--follow 回放 + contributors/rename 链）
    Evolution {
        /// 文件（项目相对；父级 git [PATH] 提供仓库路径）
        entity: Option<String>,
        #[arg(long, default_value_t = 50)]
        max: u64,
        #[arg(long)]
        json: bool,
    },
    /// M211：变更热点（原始指标，无 Score；§25）
    Hotspots {
        #[arg(default_value = ".")]
        path: std::path::PathBuf,
        #[arg(long, default_value_t = 200)]
        max_commits: u64,
        #[arg(long, default_value_t = 20)]
        top: usize,
        #[arg(long)]
        json: bool,
    },
    /// M211：变更耦合（co-change 历史关系，非语义依赖；§26）
    Coupling {
        #[arg(default_value = ".")]
        path: std::path::PathBuf,
        #[arg(long, default_value_t = 200)]
        max_commits: u64,
        #[arg(long, default_value_t = 3)]
        min: u64,
        #[arg(long)]
        json: bool,
    },
    /// M211：blame 区段（porcelain；§22 最近贡献者，非 owner）
    Blame {
        /// .cs/任意文本文件（项目相对）
        entity: Option<String>,
        #[arg(long, default_value_t = 2000)]
        max_lines: u32,
        #[arg(long)]
        json: bool,
    },
    /// M211 垂直切片（§82/§34）：commit → FileChange → SymbolChange →
    /// M26 Impact（changed symbols ⇒ callers/tests/tests candidates）
    CommitImpact {
        /// commit hash/revision
        commit: String,
        #[arg(long, default_value_t = 200)]
        max_files: usize,
        #[arg(long)]
        json: bool,
    },
    /// M211：符号历史（blob 回放 + M23 抽取 + M26 change_mapping；§15-§17）
    SymbolHistory {
        /// .cs 文件（项目相对；父级 git [PATH] 提供仓库路径）
        entity: Option<String>,
        #[arg(long, default_value_t = 50)]
        max: u64,
        #[arg(long)]
        json: bool,
    },
    /// 最近提交（bounded，规格 §18）
    Log {
        #[arg(default_value = ".")]
        path: std::path::PathBuf,
        #[arg(long, default_value_t = 20)]
        limit: u64,
        #[arg(long)]
        json: bool,
    },
    /// 文件级 diff（working tree / staged / revision range）
    Diff {
        #[arg(default_value = ".")]
        path: std::path::PathBuf,
        /// staged（index vs HEAD）
        #[arg(long)]
        staged: bool,
        /// 起始 revision（与 --to 搭配使用）
        #[arg(long)]
        from: Option<String>,
        /// 目标 revision（与 --from 搭配使用）
        #[arg(long)]
        to: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// 全部引用（branches/remotes/tags）
    Refs {
        #[arg(default_value = ".")]
        path: std::path::PathBuf,
        #[arg(long)]
        json: bool,
    },
}

pub fn run(path: &Path, command: &GitCommand, runner: Arc<dyn CommandRunner>) -> i32 {
    let cancel = CancellationToken::new();
    let service = GitService::new(runner);

    match command {
        GitCommand::Status { ignored, json, .. } => {
            let status = service.status(path, *ignored, &cancel);
            match status {
                Ok(status) => {
                    if *json {
                        print_json(&status);
                    } else {
                        print_status(&status);
                    }
                    if status.repository.is_none() {
                        eprintln!("not a git repository");
                        // 规格 §39：明确请求 Git 的命令用专门非零 exit code。
                        return crate::EXIT_NOT_GIT;
                    }
                    0
                }
                Err(err) => fail("git status", &err),
            }
        }
        GitCommand::Evolution {
            entity, max, json, ..
        } => {
            let Some(entity) = entity.as_deref() else {
                eprintln!("prism: git evolution requires <entity> (project-relative file)");
                return crate::EXIT_INVALID_INPUT;
            };
            match prism_evolution::file_evolution(&service, path, entity, *max, &cancel) {
                Ok(evolution) => {
                    if *json {
                        print_json(&evolution);
                    } else {
                        println!(
                            "file: {} | commits: {} changes: {} churn: +{}/-{} | contributors: {}",
                            evolution.path,
                            evolution.commit_count,
                            evolution.change_count,
                            evolution.lines_added,
                            evolution.lines_deleted,
                            evolution.contributors.join(", ")
                        );
                        for rename in &evolution.renames {
                            println!("  renamed: {rename}");
                        }
                        println!(
                            "first: {} | last: {}",
                            evolution.first_committed_at.as_deref().unwrap_or("?"),
                            evolution.last_committed_at.as_deref().unwrap_or("?")
                        );
                    }
                    0
                }
                Err(err) => fail("git evolution", &err),
            }
        }
        GitCommand::Hotspots {
            max_commits,
            top,
            json,
            ..
        } => match prism_evolution::scan_repo(&service, path, *max_commits, &cancel) {
            Ok(scan) => {
                let hotspots = prism_evolution::hotspots(&scan, *top);
                if *json {
                    print_json(&hotspots);
                } else {
                    println!(
                        "hotspots (window {}):",
                        prism_evolution::HistoryWindow::RecentCommits {
                            count: *max_commits
                        }
                        .as_label()
                    );
                    for hotspot in &hotspots {
                        println!(
                            "  {:<40} commits={} churn=+{}/-{} authors={} last={}",
                            hotspot.path,
                            hotspot.commit_count,
                            hotspot.lines_added,
                            hotspot.lines_deleted,
                            hotspot.author_count,
                            hotspot.last_committed_at.as_deref().unwrap_or("?")
                        );
                    }
                }
                0
            }
            Err(err) => fail("git hotspots", &err),
        },
        GitCommand::Coupling {
            max_commits,
            min,
            json,
            ..
        } => match prism_evolution::scan_repo(&service, path, *max_commits, &cancel) {
            Ok(scan) => {
                let couplings = prism_evolution::change_couplings(&scan, *min, 30);
                if *json {
                    print_json(&couplings);
                } else {
                    println!(
                        "change coupling (co-change != semantic dependency; window {}):",
                        prism_evolution::HistoryWindow::RecentCommits {
                            count: *max_commits
                        }
                        .as_label()
                    );
                    for coupling in &couplings {
                        println!(
                            "  {} <-> {} (co-change {})",
                            coupling.path_a, coupling.path_b, coupling.co_change_count
                        );
                    }
                }
                0
            }
            Err(err) => fail("git coupling", &err),
        },
        GitCommand::Blame {
            entity,
            max_lines,
            json,
            ..
        } => {
            let Some(entity) = entity.as_deref() else {
                eprintln!("prism: git blame requires <entity> (project-relative file)");
                return crate::EXIT_INVALID_INPUT;
            };
            match service.blame(path, entity, *max_lines, &cancel) {
                Ok(ranges) => {
                    if *json {
                        print_json(&ranges);
                    } else {
                        println!(
                            "blame {}: {} range(s) (recent contributors, not ownership)",
                            entity,
                            ranges.len()
                        );
                        for range in &ranges {
                            println!(
                                "  {} {:<16} lines {}-{}",
                                &range.commit[..range.commit.len().min(10)],
                                range.author,
                                range.line_start,
                                range.line_end
                            );
                        }
                    }
                    0
                }
                Err(err) => fail("git blame", &err),
            }
        }
        GitCommand::CommitImpact {
            commit,
            max_files,
            json,
            ..
        } => match commit_impact(&service, path, commit, *max_files, &cancel) {
            Ok(summary) => {
                if *json {
                    print_json(&summary);
                } else {
                    println!(
                        "commit {} changed files: {} changed symbols: {} affected: {} tests: {}",
                        &summary.commit[..summary.commit.len().min(10)],
                        summary.changed_files,
                        summary.changed_symbols,
                        summary.affected.len(),
                        summary.affected_tests
                    );
                    for affected in summary.affected.iter().take(20) {
                        println!(
                            "  {:?} {} (path hops: {})",
                            affected.impact,
                            affected.entity_id,
                            affected.primary_path.len().saturating_sub(1)
                        );
                    }
                }
                0
            }
            Err(err) => fail("git commit-impact", &err),
        },
        GitCommand::SymbolHistory {
            entity, max, json, ..
        } => {
            let Some(entity) = entity.as_deref() else {
                eprintln!(
                    "prism: git symbol-history requires <entity> (project-relative .cs file)"
                );
                return crate::EXIT_INVALID_INPUT;
            };
            match prism_evolution::symbol_history(&service, path, entity, *max, &cancel) {
                Ok(history) => {
                    if *json {
                        print_json(&history);
                    } else {
                        for entry in &history.entries {
                            println!(
                                "{} {} [{}]",
                                &entry.commit_id[..entry.commit_id.len().min(10)],
                                entry.committed_at.as_deref().unwrap_or("?"),
                                entry.author
                            );
                            for change in &entry.changes {
                                println!("  {:?} {}", change.kind, change.qualified_name);
                            }
                            for diagnostic in &entry.diagnostics {
                                println!("  diag: {diagnostic}");
                            }
                        }
                        for diagnostic in &history.diagnostics {
                            println!("diag: {diagnostic}");
                        }
                    }
                    0
                }
                Err(err) => fail("git symbol-history", &err),
            }
        }
        GitCommand::Log { limit, json, .. } => match service.log(path, *limit, &cancel) {
            Ok(commits) => {
                if *json {
                    print_json(&commits);
                } else {
                    for commit in &commits {
                        println!(
                            "{} {} <{}> {}",
                            &commit.id[..7.min(commit.id.len())],
                            commit.author.name,
                            commit.author.email,
                            commit.subject
                        );
                    }
                }
                0
            }
            Err(err) => fail("git log", &err),
        },
        GitCommand::Diff {
            staged,
            from,
            to,
            json,
            ..
        } => {
            let scope = if *staged {
                GitDiffScope::Staged
            } else if from.is_some() || to.is_some() {
                GitDiffScope::RevisionRange
            } else {
                GitDiffScope::WorkingTree
            };
            match service.diff(path, scope, from.as_deref(), to.as_deref(), 200, &cancel) {
                Ok(diff) => {
                    if *json {
                        print_json(&diff);
                    } else {
                        for file in &diff.files {
                            let path_display = match (&file.old_path, &file.new_path) {
                                (Some(old), Some(new)) if old != new => {
                                    format!("{old} -> {new}")
                                }
                                (Some(old), _) => old.clone(),
                                (_, Some(new)) => new.clone(),
                                _ => "?".to_string(),
                            };
                            println!(
                                "{}\n  change: {:?}\n  +{} -{}{}",
                                path_display,
                                file.change_type,
                                file.additions.unwrap_or(0),
                                file.deletions.unwrap_or(0),
                                if file.is_binary { " (binary)" } else { "" }
                            );
                        }
                        if diff.truncated {
                            println!("(diff truncated at {max_files} files)", max_files = 200);
                        }
                    }
                    0
                }
                Err(err) => fail("git diff", &err),
            }
        }
        GitCommand::Refs { json, .. } => match service.refs(path, &cancel) {
            Ok(refs) => {
                if *json {
                    print_json(&refs);
                } else {
                    for git_ref in refs {
                        println!(
                            "{:>2}  {}  {}",
                            git_ref.kind.as_str(),
                            git_ref.name,
                            git_ref.target
                        );
                    }
                }
                0
            }
            Err(err) => fail("git refs", &err),
        },
    }
}

fn print_status(status: &prism_git::GitStatus) {
    if let Some(head) = &status.head {
        println!("HEAD: {:?}", head.kind);
        if let Some(branch) = &head.branch_name {
            println!("Branch: {branch}");
        }
        if let Some(commit) = &head.commit_id {
            println!("Commit: {}", &commit[..7.min(commit.len())]);
        }
    }
    println!("Entries: {}", status.entries.len());
    for entry in &status.entries {
        println!(
            "  {} index={} worktree={}",
            entry.path,
            entry.index_state.as_str(),
            entry.working_tree_state.as_str()
        );
    }
    if !status.ignored.is_empty() {
        println!("Ignored: {}", status.ignored.len());
    }
}

fn fail(command: &str, err: &GitError) -> i32 {
    eprintln!("prism: {command} failed: {err}");
    // 规格 §39：not_git_repository 专门 exit code。
    if err.kind() == "not_git_repository" {
        crate::EXIT_NOT_GIT
    } else {
        1
    }
}

fn print_json<T: serde::Serialize>(value: &T) {
    match serde_json::to_string_pretty(value) {
        Ok(text) => println!("{text}"),
        Err(err) => eprintln!("prism: failed to serialize: {err}"),
    }
}

/// Commit 影响摘要（结构化输出；§38 原始指标集合，无 Risk Score）。
#[derive(Debug, serde::Serialize)]
struct CommitImpactSummary {
    commit: String,
    changed_files: usize,
    changed_symbols: usize,
    affected: Vec<prism_intel::AffectedEntity>,
    affected_tests: usize,
    truncated: bool,
}

/// M211 §82/§34 垂直切片：Commit → FileChange → SymbolChange → M26 Impact。
/// 只读；变化集 = commit 相对其父的 .cs 符号 diff（M26 change_mapping 口径）。
#[allow(clippy::too_many_lines)]
fn commit_impact(
    service: &GitService,
    path: &Path,
    commit: &str,
    max_files: usize,
    cancel: &CancellationToken,
) -> Result<CommitImpactSummary, GitError> {
    use std::collections::BTreeSet;

    let changes = service.commit_changes(path, commit, max_files, cancel)?;
    let parent = format!("{commit}^");
    let mut changed: BTreeSet<String> = BTreeSet::new();
    let mut changed_files = 0usize;
    let mut cs_changed = 0usize;
    for change in &changes {
        changed_files += 1;
        let after = &change.path_after;
        if !after.ends_with(".cs") {
            continue;
        }
        cs_changed += 1;
        let read = |revision: &str, file: &str| -> Option<String> {
            match service.read_blob(path, revision, file, 4 * 1_048_576, cancel) {
                Ok(prism_git::RevisionBlob::Text { content }) => Some(content),
                _ => None,
            }
        };
        let old_source = change
            .path_before
            .as_deref()
            .and_then(|before| read(&parent, before))
            .or_else(|| read(&parent, after));
        let new_source = read(commit, after);
        if old_source.is_none() && new_source.is_none() {
            continue;
        }
        let old_extraction = old_source
            .as_deref()
            .map(|source| prism_csharp::extract::extract(after, source));
        let new_extraction = new_source
            .as_deref()
            .map(|source| prism_csharp::extract::extract(after, source));
        for changed_symbol in
            prism_csharp::map_file_change(old_extraction.as_ref(), new_extraction.as_ref())
        {
            changed.insert(changed_symbol.symbol_id);
        }
    }

    // 项目关系图（M25 语义关系 ⇒ M26 Evidence 边；沿 context --impact-aware）。
    let mut all_cs: Vec<String> = Vec::new();
    if let Ok(discovery) = prism_project::DiscoveryService::new().discover(
        path,
        prism_project::ScanOptions::default(),
        cancel,
        |_| {},
    ) {
        for file in &discovery.files {
            if file
                .relative_path
                .rsplit('/')
                .next()
                .map(|name| name.ends_with(".cs"))
                .unwrap_or(false)
            {
                all_cs.push(file.relative_path.clone());
            }
        }
    }
    all_cs.sort();
    let mut engine = prism_csharp::SymbolEngine::new();
    let mut semantic = prism_csharp::SemanticEngine::new();
    for relative_path in &all_cs {
        let Ok(source) = std::fs::read_to_string(path.join(relative_path)) else {
            continue;
        };
        engine.update_file(relative_path, &source);
        semantic.update_source(&engine.build_index(), relative_path, &source);
    }
    semantic.rebuild(&engine.build_index());

    let mut edges = Vec::new();
    for relation in semantic.relations() {
        let prism_csharp::SemanticTarget::Resolved { symbol_id, .. } = &relation.target else {
            continue;
        };
        if !changed.contains(symbol_id) {
            continue;
        }
        edges.push(prism_intel::GraphEdge {
            source: relation.source_symbol_id.clone(),
            target: symbol_id.clone(),
            relation: relation.kind.as_str().to_string(),
            file: relation.file.clone(),
            line: relation.range.start_line,
            resolution: Some(format!("{:?}", relation.resolution).to_lowercase()),
        });
    }
    let graph = prism_intel::EvidenceGraph::from_edges(edges.iter().cloned());
    let entity_changes: Vec<prism_intel::EntityChange> = changed
        .iter()
        .map(|symbol_id| prism_intel::EntityChange {
            entity_id: symbol_id.clone(),
            change_type: prism_intel::ImpactChangeType::BodyChanged,
        })
        .collect();
    let impact = prism_intel::analyze_impact(
        &graph,
        &entity_changes,
        &prism_intel::ImpactPolicy::default(),
    );
    let affected_tests = impact
        .affected
        .iter()
        .filter(|affected| affected.category == prism_intel::ImpactCategory::Test)
        .count();
    Ok(CommitImpactSummary {
        commit: commit.to_string(),
        changed_files,
        changed_symbols: changed.len(),
        affected: impact.affected,
        affected_tests,
        truncated: impact.metadata.truncated_by_hop_limit
            + impact.metadata.truncated_by_entity_limit
            > 0,
    })
}
