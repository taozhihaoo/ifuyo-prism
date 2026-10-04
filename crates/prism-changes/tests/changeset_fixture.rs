//! 真实 Git 仓库的 ChangeSet 端到端测试（规格 M5 §81/§92/§118）。
//!
//! Fixture 在临时目录创建真实 repo 并执行真实 git 命令；
//! ChangeSetService 全程只读消费，不改写工作区。

use prism_changes::{
    build_side_model, ChangeScope, ChangeSetOptions, ChangeSetService, CurrentWorkingTreeSource,
    SymbolChangeType,
};
use prism_core::runtime::command::StdCommandRunner;
use prism_core::task::CancellationToken;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new(name: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "prism-changes-fixture-{name}-{}-{}",
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
            .expect("git must be available");
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

    fn service(&self) -> (ChangeSetService, CancellationToken) {
        (
            ChangeSetService::new(Arc::new(StdCommandRunner::new())),
            CancellationToken::new(),
        )
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

const BASE_TS: &str = "\
export function greet(name: string) {
  return 'hello ' + name;
}
";

#[test]
fn working_tree_changeset_reports_body_change_end_to_end() {
    let fixture = Fixture::new("worktree-body");
    fixture.write("src/greet.ts", BASE_TS);
    fixture.commit_all("base");
    fixture.write("src/greet.ts", &BASE_TS.replace("'hello '", "'hi '"));

    let (service, cancel) = fixture.service();
    let changeset = service
        .build(
            &fixture.root,
            ChangeScope::WorkingTree,
            None,
            None,
            &ChangeSetOptions::default(),
            &cancel,
        )
        .unwrap()
        .expect("git repo expected");

    assert_eq!(changeset.scope, ChangeScope::WorkingTree);
    assert_eq!(changeset.summary.files_modified, 1);
    assert_eq!(changeset.summary.files_total, 1);
    let file = &changeset.files[0];
    assert_eq!(file.path(), "src/greet.ts");
    assert_eq!(file.change_type, prism_changes::FileChangeType::Modified);

    let greet = changeset
        .symbols
        .iter()
        .find(|symbol| symbol.name == "greet")
        .expect("greet symbol change expected");
    assert_eq!(greet.change_type, SymbolChangeType::Modified);
    assert!(greet
        .characteristics
        .contains(&prism_changes::SymbolCharacteristic::BodyChanged));

    assert_eq!(changeset.summary.modules_changed, 1);
    let module = &changeset.modules[0];
    assert_eq!(
        module.module_id.as_str(),
        "module:file:src/greet.ts",
        "reuses M3 module identity"
    );
    assert!(!changeset.truncated);
    // 身份确定性：同一状态重复构建得到同一 id。
    let again = service
        .build(
            &fixture.root,
            ChangeScope::WorkingTree,
            None,
            None,
            &ChangeSetOptions::default(),
            &cancel,
        )
        .unwrap()
        .unwrap();
    assert_eq!(changeset.id, again.id);
}

#[test]
fn revision_range_changeset_between_commits() {
    let fixture = Fixture::new("revision-range");
    fixture.write("src/a.ts", "export const one = 1;\n");
    fixture.commit_all("c1");
    fixture.write(
        "src/a.ts",
        "export const one = 111;\nexport const two = 2;\n",
    );
    fixture.commit_all("c2");

    let (service, cancel) = fixture.service();
    let changeset = service
        .build(
            &fixture.root,
            ChangeScope::RevisionRange,
            Some("HEAD~1"),
            Some("HEAD"),
            &ChangeSetOptions::default(),
            &cancel,
        )
        .unwrap()
        .expect("changeset expected");

    assert_eq!(changeset.scope, ChangeScope::RevisionRange);
    assert!(changeset
        .base
        .as_deref()
        .is_some_and(|base| base.len() == 40));
    assert_eq!(changeset.files.len(), 1);
    assert_eq!(
        changeset.files[0].change_type,
        prism_changes::FileChangeType::Modified
    );
    // one = Modified；two = Added（新符号出现在变更行内）。
    assert!(changeset
        .symbols
        .iter()
        .any(|symbol| symbol.name == "one" && symbol.change_type == SymbolChangeType::Modified));
    assert!(changeset
        .symbols
        .iter()
        .any(|symbol| symbol.name == "two" && symbol.change_type == SymbolChangeType::Added));
}

#[test]
fn untracked_file_included_in_working_tree_scope() {
    let fixture = Fixture::new("untracked");
    fixture.write("src/keep.ts", "export const keep = 1;\n");
    fixture.commit_all("base");
    fixture.write(
        "src/fresh.ts",
        "export function fresh() {\n  return 9;\n}\n",
    );

    let (service, cancel) = fixture.service();
    let changeset = service
        .build(
            &fixture.root,
            ChangeScope::WorkingTree,
            None,
            None,
            &ChangeSetOptions::default(),
            &cancel,
        )
        .unwrap()
        .unwrap();
    assert_eq!(changeset.summary.files_added, 1);
    let added = &changeset.files[0];
    assert_eq!(added.path(), "src/fresh.ts");
    assert!(changeset
        .symbols
        .iter()
        .any(|symbol| symbol.name == "fresh" && symbol.change_type == SymbolChangeType::Added));
}

#[test]
fn staged_scope_reads_index_side_via_git() {
    let fixture = Fixture::new("staged");
    fixture.write("src/a.ts", "export const a = 1;\n");
    fixture.commit_all("base");
    fixture.write("src/a.ts", "export const a = 2;\n");
    Fixture::git(&fixture.root, &["add", "-A"]);

    let (service, cancel) = fixture.service();
    let changeset = service
        .build(
            &fixture.root,
            ChangeScope::Staged,
            None,
            None,
            &ChangeSetOptions::default(),
            &cancel,
        )
        .unwrap()
        .unwrap();
    assert_eq!(changeset.scope, ChangeScope::Staged);
    assert_eq!(changeset.target.as_deref(), Some("index"));
    assert_eq!(changeset.summary.files_modified, 1);
    assert!(changeset.files[0].mapping_status == prism_changes::MappingStatus::Mapped);
}

#[test]
fn side_model_reads_working_tree_directly() {
    let fixture = Fixture::new("side-model");
    fixture.write("x.ts", "export const x = 1;\n");
    let source = CurrentWorkingTreeSource::new(&fixture.root);
    let cancel = CancellationToken::new();
    let side = build_side_model(&["x.ts".to_string()], &source, 1024, 2, &cancel).unwrap();
    assert_eq!(side.files.len(), 1);
    assert!(side.files[0].symbols_available());
}
