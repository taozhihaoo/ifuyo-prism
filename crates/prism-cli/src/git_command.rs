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
