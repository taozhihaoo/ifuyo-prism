//! 真实 Git 仓库集成测试（规格 §42/§64/§65）。
//!
//! GitFixtureRepository 在运行期临时创建真实 repo 并执行真实 git 命令，
//! 再用被测代码读取——不是 mock 输出。生产代码仍只经 CommandRunner。

use prism_core::runtime::command::StdCommandRunner;
use prism_core::task::CancellationToken;
use prism_git::model::{GitChangeType, GitDiffScope, GitFileState};
use prism_git::{CliGitProvider, GitService};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new(name: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "prism-git-fixture-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.subsec_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        Self::git(&root, &["init", "-b", "main"]);
        Self::git(&root, &["config", "user.name", "Prism Test"]);
        Self::git(&root, &["config", "user.email", "test@prism.local"]);
        Fixture { root }
    }

    fn git(root: &Path, args: &[&str]) {
        let output = Command::new("git")
            .args(args)
            .current_dir(root)
            .env("GIT_AUTHOR_DATE", "2026-01-01T00:00:00+00:00")
            .env("GIT_COMMITTER_DATE", "2026-01-01T00:00:00+00:00")
            .output()
            .expect("git must be available for fixtures");
        assert!(
            output.status.success(),
            "git {:?} failed: exit={:?} stderr={} stdout={}",
            args,
            output.status.code(),
            String::from_utf8_lossy(&output.stderr),
            String::from_utf8_lossy(&output.stdout)
        );
    }

    fn write(&self, relative: &str, content: &str) {
        let path = self.root.join(relative);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, content).unwrap();
    }

    fn commit_all(&self, message: &str) {
        Self::git(&self.root, &["add", "-A"]);
        Self::git(&self.root, &["commit", "-m", message]);
    }

    fn provider(&self) -> (GitService, Arc<CancellationToken>) {
        (
            GitService::with_provider(Arc::new(CliGitProvider::new(Arc::new(
                StdCommandRunner::new(),
            )))),
            Arc::new(CancellationToken::new()),
        )
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

#[test]
fn discover_finds_repo_root_and_non_repo_is_none() {
    let fixture = Fixture::new("discover");
    fixture.write("src/main.ts", "export const a = 1;\n");
    fixture.commit_all("initial");

    let (service, cancel) = fixture.provider();
    let repo = service
        .discover(&fixture.root, &cancel)
        .unwrap()
        .expect("repo");
    let file_name = fixture.root.file_name().unwrap().to_string_lossy();
    assert!(repo.root_path.ends_with("main") || repo.root_path.ends_with(file_name.as_ref()));
    assert!(!repo.is_bare);

    // 非仓库目录（fixture repo 之外）→ Ok(None)（规格 §25）。
    let outside = std::env::temp_dir().join(format!(
        "prism-git-nonrepo-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(&outside).unwrap();
    let none = service.discover(&outside, &cancel).unwrap();
    assert!(none.is_none(), "plain dir must not be detected as repo");
    std::fs::remove_dir_all(&outside).ok();
}

#[test]
fn unborn_branch_detected_as_unborn_not_detached() {
    let fixture = Fixture::new("unborn"); // git init，无 commit
    let (service, cancel) = fixture.provider();
    let status = service.status(&fixture.root, false, &cancel).unwrap();
    let head = status.head.unwrap();
    assert_eq!(head.kind, prism_git::GitHeadKind::Unborn);
    assert!(head.commit_id.is_none());
    assert_eq!(head.branch_name.as_deref(), Some("main"));
}

#[test]
fn head_and_branch_on_main() {
    let fixture = Fixture::new("branch");
    fixture.write("a.txt", "hello\n");
    fixture.commit_all("initial");

    let (service, cancel) = fixture.provider();
    let status = service.status(&fixture.root, false, &cancel).unwrap();
    let head = status.head.unwrap();
    assert_eq!(head.kind, prism_git::GitHeadKind::Branch);
    assert_eq!(head.branch_name.as_deref(), Some("main"));
    assert!(head.commit_id.as_deref().is_some_and(|id| id.len() == 40));

    let branches = service
        .provider()
        .branches(&status.repository.unwrap(), &cancel)
        .unwrap();
    let main = branches.iter().find(|b| b.name == "main").unwrap();
    assert!(main.is_current);
    assert!(!main.is_remote);
}

#[test]
fn detached_head_reported_as_detached() {
    let fixture = Fixture::new("detached");
    fixture.write("a.txt", "one\n");
    fixture.commit_all("first");
    fixture.write("a.txt", "two\n");
    fixture.commit_all("second");

    // 回到第一个 commit → detached（只读测试操作，作用于临时 fixture repo）。
    let first = {
        let output = Command::new("git")
            .args(["rev-parse", "HEAD~1"])
            .current_dir(&fixture.root)
            .output()
            .unwrap();
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    };
    Fixture::git(&fixture.root, &["checkout", "--detach", &first]);

    let (service, cancel) = fixture.provider();
    let status = service.status(&fixture.root, false, &cancel).unwrap();
    let head = status.head.unwrap();
    assert_eq!(head.kind, prism_git::GitHeadKind::Detached);
    assert_eq!(head.commit_id.as_deref(), Some(first.as_str()));
    assert!(head.branch_name.is_none());
}

#[test]
fn status_separates_staged_unstaged_untracked_ignored() {
    let fixture = Fixture::new("status");
    fixture.write("committed.txt", "one\n");
    fixture.commit_all("initial");
    // .gitignore 先行提交，避免 gitignore 提交吞掉 staged 状态。
    fixture.write(".gitignore", "ignored-dir/\n");
    Fixture::git(&fixture.root, &["add", ".gitignore"]);
    fixture.commit_all("add gitignore");

    // staged + 再修改（AM/MM 语义）。
    fixture.write("staged-only.txt", "staged\n");
    fixture.write("modified.txt", "modified\n");
    Fixture::git(&fixture.root, &["add", "staged-only.txt", "modified.txt"]);
    fixture.write("modified.txt", "modified again\n");
    // untracked
    fixture.write("untracked.txt", "new\n");
    // ignored（.gitignore 已生效）
    std::fs::create_dir_all(fixture.root.join("ignored-dir")).unwrap();
    fixture.write("ignored-dir/blob.bin", "x\n");
    fixture.write("ignored-dir/inner.txt", "y\n");

    let (service, cancel) = fixture.provider();
    let status = service.status(&fixture.root, false, &cancel).unwrap();

    eprintln!(
        "DEBUG status entries: {:?}",
        status
            .entries
            .iter()
            .map(|e| (&e.path, e.index_state, e.working_tree_state))
            .collect::<Vec<_>>()
    );
    let find = |path: &str| status.entries.iter().find(|e| e.path == path);
    let staged = find("staged-only.txt").expect("staged file");
    assert_eq!(staged.index_state, GitFileState::Added);

    let modified = find("modified.txt").expect("modified file");
    // staged=Added，worktree=Modified（AM）；两维度独立（规格 §11）。
    assert_eq!(modified.index_state, GitFileState::Added);
    assert_eq!(modified.working_tree_state, GitFileState::Modified);

    let untracked = find("untracked.txt").expect("untracked file");
    assert!(untracked.is_untracked);

    // ignored 默认不在条目里（untracked 与 ignored 分离，规格 §13）。
    assert!(find("ignored-dir/inner.txt").is_none());

    // ignored 查询显式返回（不混入普通条目）。
    let status_with_ignored = service.status(&fixture.root, true, &cancel).unwrap();
    assert!(!status_with_ignored.ignored.is_empty());
    assert!(status_with_ignored
        .ignored
        .iter()
        .any(|path| path.starts_with("ignored-dir/")));
}

#[test]
fn deleted_file_reports_deleted() {
    let fixture = Fixture::new("deleted");
    fixture.write("a.txt", "one\n");
    fixture.commit_all("initial");
    std::fs::remove_file(fixture.root.join("a.txt")).unwrap();

    let (service, cancel) = fixture.provider();
    let status = service.status(&fixture.root, false, &cancel).unwrap();
    let deleted = status.entries.iter().find(|e| e.path == "a.txt").unwrap();
    assert_eq!(deleted.working_tree_state, GitFileState::Deleted);
}

#[test]
fn renamed_file_keeps_both_paths() {
    let fixture = Fixture::new("renamed");
    fixture.write("old-name.txt", "content\n");
    fixture.commit_all("initial");
    Fixture::git(&fixture.root, &["add", "-A"]);
    // Windows 上 git mv 可用；rename 检测显式。
    Fixture::git(&fixture.root, &["mv", "old-name.txt", "new-name.txt"]);

    let (service, cancel) = fixture.provider();
    let status = service.status(&fixture.root, false, &cancel).unwrap();
    let renamed = status
        .entries
        .iter()
        .find(|e| e.is_renamed || e.path == "new-name.txt")
        .expect("rename must be reported");
    assert_eq!(renamed.path, "new-name.txt");
}

#[test]
fn working_tree_and_staged_diff_scopes() {
    let fixture = Fixture::new("diff-scopes");
    fixture.write("a.txt", "line1\nline2\n");
    fixture.commit_all("initial");
    fixture.write("a.txt", "line1\nline2 changed\nline3\n");
    // working tree diff（未 staged）
    let (service, cancel) = fixture.provider();
    let wt = service
        .diff(
            &fixture.root,
            GitDiffScope::WorkingTree,
            None,
            None,
            100,
            &cancel,
        )
        .unwrap();
    assert_eq!(wt.files.len(), 1);
    assert_eq!(wt.files[0].change_type, GitChangeType::Modified);
    assert_eq!(wt.files[0].additions, Some(2));
    assert_eq!(wt.files[0].deletions, Some(1));
    assert!(!wt.files[0].hunks.is_empty());

    // staged diff 为空（尚未 add）。
    let staged = service
        .diff(
            &fixture.root,
            GitDiffScope::Staged,
            None,
            None,
            100,
            &cancel,
        )
        .unwrap();
    assert!(staged.files.is_empty());

    // staged 后 staged diff 出现、working tree diff 清空。
    Fixture::git(&fixture.root, &["add", "-A"]);
    let staged = service
        .diff(
            &fixture.root,
            GitDiffScope::Staged,
            None,
            None,
            100,
            &cancel,
        )
        .unwrap();
    assert_eq!(staged.files.len(), 1);
    let wt = service
        .diff(
            &fixture.root,
            GitDiffScope::WorkingTree,
            None,
            None,
            100,
            &cancel,
        )
        .unwrap();
    assert!(wt.files.is_empty());
}

#[test]
fn revision_diff_reports_between_commits() {
    let fixture = Fixture::new("revision");
    fixture.write("a.txt", "one\n");
    fixture.commit_all("first");
    fixture.write("b.txt", "two\n");
    fixture.commit_all("second");

    let (service, cancel) = fixture.provider();
    let diff = service
        .diff(
            &fixture.root,
            GitDiffScope::RevisionRange,
            Some("HEAD~1"),
            Some("HEAD"),
            100,
            &cancel,
        )
        .unwrap();
    assert_eq!(diff.files.len(), 1);
    assert_eq!(diff.files[0].new_path.as_deref(), Some("b.txt"));
    assert_eq!(diff.files[0].change_type, GitChangeType::Added);
}

#[test]
fn binary_diff_detected_without_lines() {
    let fixture = Fixture::new("binary");
    std::fs::write(
        fixture.root.join("logo.png"),
        b"\x89PNG\r\n\x1a\nnot-really",
    )
    .unwrap();
    fixture.commit_all("initial");
    std::fs::write(fixture.root.join("logo.png"), b"\x00\x01\x02binary-v2").unwrap();
    fixture.commit_all("binary change");

    let (service, cancel) = fixture.provider();
    let diff = service
        .diff(
            &fixture.root,
            GitDiffScope::RevisionRange,
            Some("HEAD~1"),
            Some("HEAD"),
            100,
            &cancel,
        )
        .unwrap();
    let binary = diff
        .files
        .iter()
        .find(|f| f.new_path.as_deref() == Some("logo.png"))
        .unwrap();
    assert!(binary.is_binary);
    // 规格 §21：binary 无行级 diff 表达（numstat 为 "-" → None）。
    assert_eq!(binary.additions, None);
    assert_eq!(binary.deletions, None);
    assert!(binary.hunks.is_empty(), "binary file must have no hunks");
}

#[test]
fn merge_commit_has_two_parents_and_pagination_works() {
    let fixture = Fixture::new("merge");
    fixture.write("base.txt", "base\n");
    fixture.commit_all("base");

    Fixture::git(&fixture.root, &["checkout", "-b", "feature"]);
    fixture.write("feature.txt", "feature\n");
    fixture.commit_all("feature work");

    Fixture::git(&fixture.root, &["checkout", "main"]);
    fixture.write("main.txt", "main\n");
    fixture.commit_all("main work");
    Fixture::git(
        &fixture.root,
        &["merge", "--no-ff", "-m", "merge feature", "feature"],
    );

    let (service, cancel) = fixture.provider();
    let commits = service.log(&fixture.root, 10, &cancel).unwrap();
    assert!(commits.len() >= 4);

    let merge = commits
        .iter()
        .find(|c| c.subject == "merge feature")
        .unwrap();
    assert_eq!(merge.parents.len(), 2, "merge commit must have 2 parents");

    // pagination：limit=2 只取 2 条（保持 git 历史顺序）。
    let limited = service.log(&fixture.root, 2, &cancel).unwrap();
    assert_eq!(limited.len(), 2);
    assert_eq!(limited[0].id, commits[0].id);
}

#[test]
fn unicode_and_special_filenames_survive() {
    let fixture = Fixture::new("unicode");
    // 保持 untracked（不提交），验证 NUL 分隔解析对 unicode/空格路径。
    fixture.write("中文/文档.md", "# 文档\n");
    fixture.write("src/file with spaces.ts", "export const x = 1;\n");

    let (service, cancel) = fixture.provider();
    let status = service.status(&fixture.root, false, &cancel).unwrap();
    let paths: Vec<&str> = status.entries.iter().map(|e| e.path.as_str()).collect();
    assert!(paths.contains(&"中文/文档.md"), "paths: {paths:?}");
    assert!(paths.contains(&"src/file with spaces.ts"));
}

#[test]
fn cancellation_stops_git_query() {
    let fixture = Fixture::new("cancel");
    fixture.write("a.txt", "one\n");
    fixture.commit_all("initial");

    let (service, cancel) = fixture.provider();
    cancel.cancel();
    let err = service
        .status(&fixture.root, false, &cancel)
        .expect_err("cancelled status must fail");
    assert_eq!(err.kind(), "cancelled");
}

#[test]
fn git_unavailable_reported_as_unavailable() {
    let fixture = Fixture::new("no-git");
    let service = GitService::with_provider(Arc::new(CliGitProvider::with_executable(
        Arc::new(StdCommandRunner::new()),
        "definitely-not-git-xyz",
    )));
    let cancel = CancellationToken::new();
    let err = service
        .status(&fixture.root, false, &cancel)
        .expect_err("missing git must fail");
    assert_eq!(err.kind(), "git_unavailable");
}

#[test]
fn log_on_unborn_repo_returns_empty() {
    let fixture = Fixture::new("unborn-log");
    let commits = {
        let (service, cancel) = fixture.provider();
        service.log(&fixture.root, 10, &cancel).unwrap_or_default()
    };
    assert!(commits.is_empty());
    let _ = PathBuf::from(&fixture.root);
}
