//! GitProvider trait 与基于系统 git CLI 的实现（规格 §4/§5/§28）。
//!
//! 全部 Git 调用经 M0 CommandRunner（timeout/cancel/cwd/stdout/stderr），
//! 只使用机器可读格式（porcelain v2、-z、%x01/%x1e 分隔），
//! 且均为只读命令（§40 安全审计清单：无任何 mutating 命令）。

use crate::error::{GitError, GitResult};
use crate::model::{
    GitBranch, GitCommit, GitDiff, GitDiffScope, GitHead, GitHeadKind, GitRef, GitRepository,
    GitStatus, RevisionBlob,
};
use crate::{model::GitDiffFile, parse};
use prism_core::task::CancellationToken;
use prism_core::CommandRunner;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Git 能力抽象（可 mock、可取消、错误明确，规格 §5）。
pub trait GitProvider: Send + Sync {
    /// 就近发现仓库；非 Git 项目返回 Ok(None)（规格 §25，不是错误）。
    fn discover(&self, path: &Path, cancel: &CancellationToken)
        -> GitResult<Option<GitRepository>>;

    fn head(&self, repo: &GitRepository, cancel: &CancellationToken) -> GitResult<GitHead>;

    /// status（porcelain v2 -z；include_ignored 时额外查询 ignored）。
    fn status(
        &self,
        repo: &GitRepository,
        include_ignored: bool,
        cancel: &CancellationToken,
    ) -> GitResult<GitStatus>;

    fn branches(
        &self,
        repo: &GitRepository,
        cancel: &CancellationToken,
    ) -> GitResult<Vec<GitBranch>>;

    fn refs(&self, repo: &GitRepository, cancel: &CancellationToken) -> GitResult<Vec<GitRef>>;

    /// 最近 N 条提交（bounded，规格 §18）。
    fn log(
        &self,
        repo: &GitRepository,
        limit: u64,
        cancel: &CancellationToken,
    ) -> GitResult<Vec<GitCommit>>;

    /// 按语义 scope 获取文件级 diff（§14–§17）。
    /// RevisionRange 时 base/target 必须给出；max_files 用于 bounded size（§35）。
    fn diff(
        &self,
        repo: &GitRepository,
        scope: GitDiffScope,
        base: Option<&str>,
        target: Option<&str>,
        max_files: usize,
        cancel: &CancellationToken,
    ) -> GitResult<GitDiff>;

    /// 将 revision 表达式解析为完整 commit SHA；无法解析返回 None（不猜测）。
    /// 只读（`git rev-parse --verify`）。
    fn resolve_revision(
        &self,
        repo: &GitRepository,
        revision: &str,
        cancel: &CancellationToken,
    ) -> GitResult<Option<String>>;

    /// 读取历史 revision 中某路径的 blob 内容（M5 历史源；只读 `cat-file`）。
    /// 只返回上限内、且字节长度核对一致的 UTF-8 文本；其余情况显式建模，
    /// 不返回失真内容（规格 M5 §39/§42）。
    fn read_blob(
        &self,
        repo: &GitRepository,
        revision: &str,
        path: &str,
        max_bytes: u64,
        cancel: &CancellationToken,
    ) -> GitResult<RevisionBlob>;
}

/// 基于系统 git CLI 的实现（经 CommandRunner，规格 §28）。
pub struct CliGitProvider {
    runner: Arc<dyn CommandRunner>,
    git_executable: String,
}

impl std::fmt::Debug for CliGitProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CliGitProvider")
            .field("git", &self.git_executable)
            .finish()
    }
}

impl CliGitProvider {
    pub fn new(runner: Arc<dyn CommandRunner>) -> Self {
        CliGitProvider {
            runner,
            git_executable: "git".to_string(),
        }
    }

    /// Git 缺失场景测试用：显式指定（不存在的）git 可执行文件。
    pub fn with_executable(runner: Arc<dyn CommandRunner>, git: &str) -> Self {
        CliGitProvider {
            runner,
            git_executable: git.to_string(),
        }
    }

    fn run_git(
        &self,
        cwd: &GitRepository,
        args: &[&str],
        cancel: &CancellationToken,
    ) -> GitResult<String> {
        let output = self
            .runner
            .run(&self.spec(Path::new(&cwd.root_path), args), Some(cancel))
            .map_err(GitError::from)?;
        self.check_exit(args, &output.exit_code, &output.stderr, &output.stdout)
    }

    fn spec(&self, cwd: &Path, args: &[&str]) -> prism_core::runtime::command::CommandSpec {
        prism_core::runtime::command::CommandSpec::new(
            self.git_executable.clone(),
            args.iter().map(|arg| arg.to_string()),
        )
        .with_working_directory(cwd.to_path_buf())
    }

    fn check_exit(
        &self,
        args: &[&str],
        exit_code: &Option<i32>,
        stderr: &str,
        stdout: &str,
    ) -> GitResult<String> {
        match exit_code {
            Some(0) => Ok(stdout.to_string()),
            Some(128) if stderr.contains("not a git repository") => {
                Err(GitError::NotGitRepository {
                    path: String::new(),
                })
            }
            Some(code) => Err(GitError::CommandFailed {
                command: args.first().copied().unwrap_or("git").to_string(),
                exit_code: *code,
                stderr: stderr.to_string(),
            }),
            None => Err(GitError::Unknown {
                message: "git process terminated by signal".to_string(),
            }),
        }
    }

    fn try_git(
        &self,
        cwd: &Path,
        args: &[&str],
        cancel: &CancellationToken,
    ) -> GitResult<Option<String>> {
        let output = self
            .runner
            .run(&self.spec(cwd, args), Some(cancel))
            .map_err(GitError::from)?;
        if output.exit_code == Some(0) {
            Ok(Some(output.stdout.trim().to_string()))
        } else {
            Ok(None)
        }
    }
}

impl GitProvider for CliGitProvider {
    fn discover(
        &self,
        path: &Path,
        cancel: &CancellationToken,
    ) -> GitResult<Option<GitRepository>> {
        let output = match self.runner.run(
            &self.spec(
                path,
                &[
                    "rev-parse",
                    "--show-toplevel",
                    "--absolute-git-dir",
                    "--git-common-dir",
                    "--is-bare-repository",
                ],
            ),
            Some(cancel),
        ) {
            Ok(output) => output,
            Err(prism_core::PrismError::Command { message }) => {
                // spawn 失败 = git 不可用；rev-parse 退出 128 = 非仓库。
                return if message.contains("program not found")
                    || message.contains("os error 2")
                    || message.to_lowercase().contains("not found")
                {
                    Err(GitError::GitUnavailable { message })
                } else {
                    Err(GitError::NotGitRepository {
                        path: path.to_string_lossy().to_string(),
                    })
                };
            }
            Err(prism_core::PrismError::Cancelled) => return Err(GitError::Cancelled),
            Err(err) => {
                return Err(GitError::Unknown {
                    message: err.to_string(),
                })
            }
        };

        // 非 Git 项目：rev-parse 退出 128。规格 §25：这不是错误，返回 Ok(None)。
        if output.exit_code != Some(0) {
            return Ok(None);
        }

        let lines: Vec<&str> = output.stdout.lines().collect();
        if lines.len() < 4 {
            return Err(GitError::MalformedGitOutput {
                command: "rev-parse".to_string(),
                detail: format!("expected 4 lines, got {}", lines.len()),
            });
        }
        let root_path = lines[0].to_string();
        let git_dir = lines[1].to_string();
        let common_git_dir = lines[2].to_string();
        let is_bare = lines[3] == "true";
        let normalize = |path: &str| path.replace('\\', "/").trim_end_matches('/').to_string();
        let is_worktree = normalize(&git_dir) != normalize(&common_git_dir);
        Ok(Some(GitRepository {
            root_path,
            git_dir,
            common_git_dir,
            is_bare,
            is_worktree,
        }))
    }

    fn head(&self, repo: &GitRepository, cancel: &CancellationToken) -> GitResult<GitHead> {
        let cwd = Path::new(&repo.root_path);
        let symbolic = self.try_git(cwd, &["symbolic-ref", "--quiet", "HEAD"], cancel)?;
        let commit = self.try_git(cwd, &["rev-parse", "--verify", "--quiet", "HEAD"], cancel)?;

        match (symbolic, commit) {
            (Some(symbolic_ref), commit) => {
                let unborn = commit.is_none();
                Ok(GitHead {
                    kind: if unborn {
                        GitHeadKind::Unborn
                    } else {
                        GitHeadKind::Branch
                    },
                    commit_id: commit,
                    branch_name: symbolic_ref.strip_prefix("refs/heads/").map(str::to_string),
                    symbolic_ref: Some(symbolic_ref),
                })
            }
            (None, Some(commit_id)) => Ok(GitHead {
                kind: GitHeadKind::Detached,
                commit_id: Some(commit_id),
                branch_name: None,
                symbolic_ref: None,
            }),
            (None, None) => Ok(GitHead {
                kind: GitHeadKind::Unknown,
                commit_id: None,
                branch_name: None,
                symbolic_ref: None,
            }),
        }
    }

    fn status(
        &self,
        repo: &GitRepository,
        include_ignored: bool,
        cancel: &CancellationToken,
    ) -> GitResult<GitStatus> {
        // -c 固定配置：不受用户全局/系统配置影响（规格 §4/§27 确定性要求）。
        let mut args = vec![
            "-c",
            "status.showUntrackedFiles=all",
            "-c",
            "core.quotepath=false",
            "status",
            "--porcelain=v2",
            "-z",
            "--branch",
            "--untracked-files=all",
        ];
        if include_ignored {
            args.push("--ignored=matching");
        }
        let output = self.run_git(repo, &args, cancel)?;
        let (entries, ignored) = parse::parse_status_v2(&output)?;
        let head = self.head(repo, cancel)?;
        Ok(GitStatus {
            repository: Some(repo.clone()),
            head: Some(head),
            entries,
            ignored,
        })
    }

    fn branches(
        &self,
        repo: &GitRepository,
        cancel: &CancellationToken,
    ) -> GitResult<Vec<GitBranch>> {
        let output = self.run_git(
            repo,
            &[
                "for-each-ref",
                "--format=%(refname)%01%(objectname)%01%(objecttype)%01%(refname:short)%01%(upstream:short)%01%(upstream:track)",
            ],
            cancel,
        )?;
        let head = self.head(repo, cancel)?;
        Ok(parse::parse_branches(&output, head.branch_name.as_deref()))
    }

    fn refs(&self, repo: &GitRepository, cancel: &CancellationToken) -> GitResult<Vec<GitRef>> {
        let output = self.run_git(
            repo,
            &[
                "for-each-ref",
                "--format=%(refname)%01%(objectname)%01%(objecttype)%01%(refname:short)",
            ],
            cancel,
        )?;
        Ok(parse::parse_refs(&output))
    }

    fn log(
        &self,
        repo: &GitRepository,
        limit: u64,
        cancel: &CancellationToken,
    ) -> GitResult<Vec<GitCommit>> {
        let limit_arg = limit.to_string();
        let output = self.run_git(
            repo,
            &[
                "log",
                "-n",
                &limit_arg,
                "--pretty=format:%H%x1f%P%x1f%an%x1f%ae%x1f%aI%x1f%cn%x1f%ce%x1f%cI%x1f%s%x1f%b%x1f%D%x1e",
            ],
            cancel,
        )?;
        Ok(parse::parse_log(&output))
    }

    fn diff(
        &self,
        repo: &GitRepository,
        scope: GitDiffScope,
        base: Option<&str>,
        target: Option<&str>,
        max_files: usize,
        cancel: &CancellationToken,
    ) -> GitResult<GitDiff> {
        // pin quotepath：patch 无 -z，unicode 路径默认会被八进制转义，
        // 导致与 raw/numstat 的 UTF-8 路径不匹配（M5 special-paths 回归）。
        let mut args: Vec<String> = vec![
            "-c".to_string(),
            "core.quotepath=false".to_string(),
            "diff".to_string(),
        ];
        if scope == GitDiffScope::Staged {
            args.push("--cached".to_string());
        }
        let range: Vec<String> = match scope {
            GitDiffScope::RevisionRange => {
                let (Some(base), Some(target)) = (base, target) else {
                    return Err(GitError::Unsupported {
                        message: "revision diff requires base and target revisions".to_string(),
                    });
                };
                vec![base.to_string(), target.to_string()]
            }
            _ => Vec::new(),
        };
        args.extend(range.clone());
        let args_refs: Vec<&str> = args.iter().map(String::as_str).collect();

        // 三条批量查询：numstat -z（统计）+ raw -z（元数据）+ patch（hunk）。
        let mut numstat_args = args.clone();
        numstat_args.insert(3, "--numstat".to_string());
        numstat_args.insert(4, "-z".to_string());
        let numstat_output = self.run_git(repo, &to_refs(&numstat_args), cancel)?;

        let mut raw_args = args.clone();
        raw_args.insert(3, "--raw".to_string());
        raw_args.insert(4, "-z".to_string());
        let raw_output = self.run_git(repo, &to_refs(&raw_args), cancel)?;

        let mut patch_args = args.clone();
        patch_args.insert(3, "--patch".to_string());
        let patch_output = self.run_git(repo, &to_refs(&patch_args), cancel)?;

        let _ = args_refs;

        let raw_entries = parse::parse_raw_diff(&raw_output);
        let numstat = parse::parse_numstat(&numstat_output);
        let patch_files = parse::parse_patch(&patch_output);

        let mut files: Vec<GitDiffFile> = raw_entries
            .into_iter()
            .map(|entry| {
                let lookup_path = entry
                    .original_path
                    .as_deref()
                    .unwrap_or(&entry.path)
                    .to_string();
                let (additions, deletions, is_binary) = numstat
                    .get(&lookup_path)
                    .copied()
                    .unwrap_or((None, None, false));
                let patch_file = patch_files.iter().find(|patch| {
                    patch.old_path.as_deref() == Some(lookup_path.as_str())
                        || patch.new_path.as_deref() == Some(entry.path.as_str())
                });
                GitDiffFile {
                    old_path: entry
                        .original_path
                        .clone()
                        .or_else(|| patch_file.and_then(|f| f.old_path.clone())),
                    new_path: Some(entry.path.clone()),
                    change_type: entry.change_type,
                    old_mode: entry.old_mode,
                    new_mode: entry.new_mode,
                    old_blob: entry.old_blob,
                    new_blob: entry.new_blob,
                    is_binary: is_binary || patch_file.is_some_and(|f| f.is_binary),
                    is_rename: entry.is_rename || patch_file.is_some_and(|f| f.is_rename),
                    similarity: entry
                        .similarity
                        .or_else(|| patch_file.and_then(|f| f.similarity)),
                    additions,
                    deletions,
                    hunks: patch_file.map(|f| f.hunks.clone()).unwrap_or_default(),
                }
            })
            .collect();

        files.sort_by(|a, b| {
            a.new_path
                .as_deref()
                .unwrap_or("")
                .cmp(b.new_path.as_deref().unwrap_or(""))
        });

        let truncated = files.len() > max_files;
        files.truncate(max_files);

        Ok(GitDiff {
            scope,
            base: base.map(str::to_string),
            target: target.map(str::to_string),
            files,
            truncated,
        })
    }

    fn resolve_revision(
        &self,
        repo: &GitRepository,
        revision: &str,
        cancel: &CancellationToken,
    ) -> GitResult<Option<String>> {
        let object = format!("{revision}^{{commit}}");
        let output = self.runner.run(
            &self.spec(
                Path::new(&repo.root_path),
                &["rev-parse", "--verify", "--quiet", &object],
            ),
            Some(cancel),
        )?;
        Ok(if output.exit_code == Some(0) {
            Some(output.stdout.trim().to_string())
        } else {
            None
        })
    }

    fn read_blob(
        &self,
        repo: &GitRepository,
        revision: &str,
        path: &str,
        max_bytes: u64,
        cancel: &CancellationToken,
    ) -> GitResult<RevisionBlob> {
        let root = Path::new(&repo.root_path);
        let object = format!("{revision}:{path}");

        // 先取 blob 大小：缺失路径在此暴露，超大内容不读取（bounded，§78）。
        let size_output = self
            .runner
            .run(&self.spec(root, &["cat-file", "-s", &object]), Some(cancel))?;
        if size_output.exit_code != Some(0) {
            return blob_failure("cat-file -s", &size_output);
        }
        let size: u64 =
            size_output
                .stdout
                .trim()
                .parse()
                .map_err(|_| GitError::MalformedGitOutput {
                    command: "git cat-file -s".to_string(),
                    detail: format!("non-numeric size output: {:?}", size_output.stdout),
                })?;
        if size > max_bytes {
            return Ok(RevisionBlob::TooLarge { size_bytes: size });
        }

        let blob_output = self.runner.run(
            &self.spec(root, &["cat-file", "blob", &object]),
            Some(cancel),
        )?;
        if blob_output.exit_code != Some(0) {
            return blob_failure("cat-file blob", &blob_output);
        }
        // read_to_string 遇非 UTF-8 整体失败 → 空串；长度核对防止把失真内容当文本。
        if blob_output.stdout.len() as u64 != size {
            return Ok(RevisionBlob::EncodingUnsupported);
        }
        Ok(RevisionBlob::Text {
            content: blob_output.stdout,
        })
    }
}

fn to_refs(args: &[String]) -> Vec<&str> {
    args.iter().map(String::as_str).collect()
}

/// cat-file 失败：路径缺失是正常事实（Missing），其余按命令失败上抛。
fn blob_failure(
    command: &str,
    output: &prism_core::runtime::command::CommandOutput,
) -> GitResult<RevisionBlob> {
    let stderr = output.stderr.to_lowercase();
    let exit_code = output.exit_code;
    if stderr.contains("not a valid object name")
        || stderr.contains("does not exist")
        || stderr.contains("exists on disk, but not in")
    {
        Ok(RevisionBlob::Missing)
    } else {
        Err(GitError::CommandFailed {
            command: command.to_string(),
            exit_code: exit_code.unwrap_or(-1),
            stderr: output.stderr.clone(),
        })
    }
}

// PathBuf 用于 discover 的 working directory 类型推导。
#[allow(dead_code)]
type _PathBufAlias = PathBuf;
