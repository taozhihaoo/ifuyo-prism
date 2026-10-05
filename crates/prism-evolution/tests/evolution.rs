//! prism-evolution 集成测试（真实 git 仓库夹具，沿 prism-git 测试口径）。

use prism_core::runtime::command::StdCommandRunner;
use prism_core::task::CancellationToken;
use prism_evolution::{change_couplings, file_evolution, hotspots, scan_repo, EvolutionQuery};
use prism_git::CliGitProvider;
use prism_git::GitService;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new(name: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "prism-evolution-fixture-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.subsec_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        Self::git(&root, &["init", "-b", "main"]);
        Self::git(&root, &["config", "user.name", "Alice"]);
        Self::git(&root, &["config", "user.email", "alice@prism.local"]);
        Fixture { root }
    }

    fn git(root: &Path, args: &[&str]) {
        let output = Command::new("git")
            .args(args)
            .current_dir(root)
            .env("GIT_AUTHOR_DATE", "2026-02-01T00:00:00+00:00")
            .env("GIT_COMMITTER_DATE", "2026-02-01T00:00:00+00:00")
            .output()
            .expect("git must be available for fixtures");
        assert!(
            output.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
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

    fn rename(&self, from: &str, to: &str) {
        Self::git(&self.root, &["mv", from, to]);
    }

    fn commit_as(&self, name: &str, message: &str) {
        Self::git(&self.root, &["add", "-A"]);
        Self::git(
            &self.root,
            &["-c", &format!("user.name={name}"), "commit", "-m", message],
        );
    }

    fn service(&self) -> (GitService, Arc<CancellationToken>) {
        (
            GitService::with_provider(Arc::new(CliGitProvider::new(Arc::new(
                StdCommandRunner::new(),
            )))),
            Arc::new(CancellationToken::new()),
        )
    }

    fn root(&self) -> &Path {
        &self.root
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

#[test]
fn file_evolution_tracks_renames_and_contributors() {
    let fixture = Fixture::new("evolution");
    fixture.write("src/battle.rs", "struct Battle;\n");
    fixture.commit_all("add battle");
    fixture.write("src/battle.rs", "struct Battle { hp: u32 }\n");
    fixture.commit_as("Bob", "extend battle");
    fixture.rename("src/battle.rs", "src/combat.rs");
    fixture.commit_all("rename battle to combat");

    let (service, cancel) = fixture.service();
    let evolution = file_evolution(&service, fixture.root(), "src/combat.rs", 50, &cancel).unwrap();
    assert_eq!(evolution.commit_count, 3, "--follow 穿透 rename");
    assert_eq!(evolution.lines_added + evolution.lines_deleted > 0, true);
    assert_eq!(evolution.contributors, vec!["Alice", "Bob"], "去重排序");
    assert_eq!(evolution.renames.len(), 1, "rename 链一条");
    assert!(evolution.renames[0].contains("src/battle.rs -> src/combat.rs"));
    assert!(evolution.first_committed_at.is_some());
}

#[test]
fn hotspots_and_couplings_rank_deterministically() {
    let fixture = Fixture::new("hotspot");
    // 初始骨架。
    fixture.write("src/core.rs", "a\n");
    fixture.write("src/manager.rs", "m\n");
    fixture.write("src/lonely.rs", "l\n");
    fixture.commit_all("skeleton");
    // core+manager 高频共同变更（3 次）。
    for index in 0..3 {
        fixture.write("src/core.rs", format!("a{index}\n").as_str());
        fixture.write("src/manager.rs", format!("m{index}\n").as_str());
        fixture.commit_all(&format!("co-change {index}"));
    }
    // lonely.rs 单独变更一次。
    fixture.write("src/lonely.rs", "l2\n");
    fixture.commit_all("lonely change");

    let (service, cancel) = fixture.service();
    let query = EvolutionQuery::default();
    let scan = scan_repo(&service, fixture.root(), query.max_commits, &cancel).unwrap();
    let hotspots = hotspots(&scan, query.top_hotspots);
    assert_eq!(hotspots.len(), 3);
    assert_eq!(hotspots[0].path, "src/core.rs", "core 4 commits 排第一");
    assert_eq!(hotspots[0].commit_count, 4);
    assert!(hotspots[0].churn > 0);
    assert_eq!(
        hotspots
            .iter()
            .find(|h| h.path == "src/lonely.rs")
            .unwrap()
            .commit_count,
        2
    );

    let couplings = change_couplings(&scan, query.min_co_change, query.max_couplings);
    assert_eq!(couplings.len(), 1, "仅 core+manager 达到 min_co_change=3");
    let coupling = &couplings[0];
    assert_eq!(coupling.co_change_count, 4, "skeleton + 3 次 co-change");
    assert_eq!(coupling.path_a, "src/core.rs");
    assert_eq!(coupling.path_b, "src/manager.rs");
    assert_eq!(
        coupling.evidence_commits.len(),
        4,
        "证据 commit 全留(cap 5 内)"
    );
}

#[test]
fn non_git_project_is_empty_not_error() {
    let outside =
        std::env::temp_dir().join(format!("prism-evolution-nonrepo-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&outside);
    std::fs::create_dir_all(&outside).unwrap();
    let (service, cancel) = (
        GitService::with_provider(Arc::new(CliGitProvider::new(Arc::new(
            StdCommandRunner::new(),
        )))),
        Arc::new(CancellationToken::new()),
    );
    let scan = scan_repo(&service, &outside, 10, &cancel).unwrap();
    assert!(scan.hotspots(10).is_empty(), "非仓库 ⇒ 空（非错误）");
    std::fs::remove_dir_all(&outside).ok();
}

#[test]
fn default_query_bounds_are_bounded() {
    let query = EvolutionQuery::default();
    assert_eq!(query.max_commits, 200, "§47 禁止默认全历史");
    assert!(query.top_hotspots > 0 && query.max_couplings > 0);
}
