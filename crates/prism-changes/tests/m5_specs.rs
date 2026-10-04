//! M5 规格 §84–§91 的真实 Git 集成 fixture 测试。
//!
//! 每个 fixture 是一个临时真实仓库；验证 parse-failure、binary、rename、
//! 交叉语言、路径边界、whitespace、git-only 变更与截断的部分成功。

use prism_changes::{
    ChangeScope, ChangeSetOptions, ChangeSetService, MappingStatus, SymbolChangeType,
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
            "prism-changes-m5-{name}-{}-{}",
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

    fn write_bytes(&self, relative: &str, content: &[u8]) {
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

    fn move_file(&self, from: &str, to: &str) {
        Self::git(&self.root, &["mv", from, to]);
    }

    fn build(&self, scope: ChangeScope) -> prism_changes::ChangeSet {
        self.build_with(scope, None, None)
    }

    fn build_with(
        &self,
        scope: ChangeScope,
        from: Option<&str>,
        to: Option<&str>,
    ) -> prism_changes::ChangeSet {
        let service = ChangeSetService::new(Arc::new(StdCommandRunner::new()));
        service
            .build(
                &self.root,
                scope,
                from,
                to,
                &ChangeSetOptions::default(),
                &CancellationToken::new(),
            )
            .expect("changeset build must succeed")
            .expect("git repo expected")
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// §84：after 侧语法损坏 ⇒ 绝不产生"全部删除"。
/// 事实（以实际 parser 为准）：tree-sitter 是恢复式解析 —— 损坏文件
/// SuccessfulWithDiagnostics，能恢复的符号照常映射；恢复不了的符号
/// 因变更行无 range 重叠而保持沉默，不会被伪造成 Deleted（§42 核心）。
#[test]
fn broken_syntax_does_not_fabricate_deletions() {
    let fixture = Fixture::new("parse-failure");
    fixture.write(
        "src/a.ts",
        "export function good() {\n  return 1;\n}\n\nexport function fine() {\n  return 2;\n}\n",
    );
    fixture.commit_all("base");
    fixture.write("src/a.ts", "export function good() {\n  return (((\n");

    let changeset = fixture.build(ChangeScope::WorkingTree);
    assert_eq!(changeset.summary.files_modified, 1);
    let file = &changeset.files[0];
    assert_eq!(file.change_type, prism_changes::FileChangeType::Modified);
    // 关键正确性：fine() 在 after 侧消失，但变更行不在其旧范围内 ⇒
    // 不得伪造 Deleted（§42/§84：解析损坏 ≠ 全部符号删除）。
    assert!(
        changeset
            .symbols
            .iter()
            .all(|s| s.change_type != SymbolChangeType::Deleted),
        "no fabricated deletions allowed: {:?}",
        changeset.symbols
    );
    // good() 所在行确实被改 ⇒ Modified（恢复式解析出的 after 符号可映射）。
    assert!(
        changeset
            .symbols
            .iter()
            .any(|s| s.name == "good" && s.change_type == SymbolChangeType::Modified),
        "good expected modified: {:?}",
        changeset.symbols
    );
    assert_eq!(file.mapping_status, MappingStatus::Mapped);
}

/// §85：纯文件 rename（内容不变）⇒ Renamed 文件 + 符号 Moved。
#[test]
fn pure_rename_keeps_rename_fact_and_moves_symbols() {
    let fixture = Fixture::new("pure-rename");
    fixture.write(
        "src/util.ts",
        "export function helper() {\n  return 1;\n}\n",
    );
    fixture.commit_all("base");
    fixture.move_file("src/util.ts", "src/utils.ts");
    fixture.commit_all("rename");

    let changeset = fixture.build_with(ChangeScope::RevisionRange, Some("HEAD~1"), Some("HEAD"));
    assert_eq!(changeset.summary.files_renamed, 1, "{:?}", changeset.files);
    let file = &changeset.files[0];
    assert_eq!(file.change_type, prism_changes::FileChangeType::Renamed);
    assert_eq!(file.old_path.as_deref(), Some("src/util.ts"));
    assert_eq!(file.new_path.as_deref(), Some("src/utils.ts"));
    // 符号：helper 以 Moved 呈现（local identity + git rename 证据）。
    assert!(
        changeset
            .symbols
            .iter()
            .any(|s| s.name == "helper" && s.change_type == SymbolChangeType::Moved),
        "helper expected moved: {:?}",
        changeset.symbols
    );
}

/// §85：rename + 内容修改 ⇒ Renamed 文件；命中符号 Modified（非 Moved 伪装）。
#[test]
fn rename_with_body_modification_records_both_facts() {
    let fixture = Fixture::new("rename-mod");
    fixture.write("src/old.ts", "export function worker() {\n  return 1;\n}\n");
    fixture.commit_all("base");
    fixture.move_file("src/old.ts", "src/new.ts");
    fixture.write(
        "src/new.ts",
        "export function worker() {\n  return 99;\n}\n",
    );
    fixture.commit_all("rename+edit");

    let changeset = fixture.build_with(ChangeScope::RevisionRange, Some("HEAD~1"), Some("HEAD"));
    assert_eq!(changeset.summary.files_renamed, 1);
    // §50：跨文件符号移动 ⇒ Moved 为 primary；主体修改进 characteristics（§17）。
    let worker = changeset
        .symbols
        .iter()
        .find(|s| s.name == "worker")
        .expect("worker expected: {:?}");
    assert_eq!(worker.change_type, SymbolChangeType::Moved);
    assert!(
        worker
            .characteristics
            .contains(&prism_changes::SymbolCharacteristic::BodyChanged),
        "body change expected as characteristic: {:?}",
        worker.characteristics
    );
}

/// §40/§41：Added 文件产出全部 Added 符号；Deleted 文件产出全部 Deleted 符号。
#[test]
fn added_and_deleted_files_produce_whole_side_symbols() {
    let fixture = Fixture::new("add-del");
    fixture.write("src/keep.ts", "export const keep = 1;\n");
    fixture.write(
        "src/gone.ts",
        "export function legacy() {\n  return 1;\n}\n",
    );
    fixture.commit_all("base");
    std::fs::remove_file(fixture.root.join("src/gone.ts")).unwrap();
    fixture.write("src/new.ts", "export function shiny() {\n  return 2;\n}\n");

    let changeset = fixture.build(ChangeScope::WorkingTree);
    assert_eq!(changeset.summary.files_added, 1);
    assert_eq!(changeset.summary.files_deleted, 1);
    assert!(changeset
        .symbols
        .iter()
        .any(|s| s.name == "shiny" && s.change_type == SymbolChangeType::Added));
    assert!(changeset
        .symbols
        .iter()
        .any(|s| s.name == "legacy" && s.change_type == SymbolChangeType::Deleted));
    // 删除侧 before 源可得 ⇒ 不产生 without-baseline 诊断。
    assert!(!changeset
        .diagnostics
        .iter()
        .any(|d| d.kind == prism_changes::ChangeDiagnosticKind::DeletedFileWithoutBaseline));
}

/// §88：二进制文件 ⇒ FileChange Modified + is_binary + NotApplicable。
#[test]
fn binary_file_change_is_not_symbol_mapped() {
    let fixture = Fixture::new("binary");
    fixture.write_bytes("assets/logo.png", &[0x89, 0x50, 0x4E, 0x47, 0x00, 0x01]);
    fixture.commit_all("base");
    fixture.write_bytes("assets/logo.png", &[0x89, 0x50, 0x4E, 0x47, 0x00, 0x02]);

    let changeset = fixture.build(ChangeScope::WorkingTree);
    assert_eq!(changeset.summary.files_modified, 1);
    let file = &changeset.files[0];
    assert!(file.is_binary);
    assert_eq!(file.mapping_status, MappingStatus::NotApplicable);
    assert!(changeset
        .diagnostics
        .iter()
        .any(|d| d.kind == prism_changes::ChangeDiagnosticKind::BinaryFile));
    assert!(changeset.symbols.is_empty());
}

/// §87：不支持语言 ⇒ ChangeSet 有效 + NotApplicable + 诊断。
#[test]
fn unsupported_language_keeps_changeset_valid() {
    let fixture = Fixture::new("unsupported");
    fixture.write("data.xyz", "opaque content v1\n");
    fixture.commit_all("base");
    fixture.write("data.xyz", "opaque content v2\n");

    let changeset = fixture.build(ChangeScope::WorkingTree);
    assert_eq!(changeset.summary.files_modified, 1);
    assert_eq!(
        changeset.files[0].mapping_status,
        MappingStatus::NotApplicable
    );
    assert!(changeset
        .diagnostics
        .iter()
        .any(|d| d.kind == prism_changes::ChangeDiagnosticKind::UnsupportedLanguage));
}

/// §86：跨语言（Python / C#）符号级映射。
#[test]
fn cross_language_python_and_csharp_symbol_changes() {
    // Python：方法体修改。
    let fixture = Fixture::new("py");
    fixture.write(
        "app/service.py",
        "class Greeter:\n    def hello(self):\n        return 1\n",
    );
    fixture.commit_all("base");
    fixture.write(
        "app/service.py",
        "class Greeter:\n    def hello(self):\n        return 2\n",
    );

    let changeset = fixture.build(ChangeScope::WorkingTree);
    assert!(
        changeset
            .symbols
            .iter()
            .any(|s| s.name == "hello" && s.change_type == SymbolChangeType::Modified),
        "python method expected modified: {:?}",
        changeset.symbols
    );

    // C#：方法签名修改。
    let fixture = Fixture::new("cs");
    fixture.write(
        "src/Greet.cs",
        "namespace App;\npublic class Greeter\n{\n    public int Add(int a)\n    {\n        return a;\n    }\n}\n",
    );
    fixture.commit_all("base");
    fixture.write(
        "src/Greet.cs",
        "namespace App;\npublic class Greeter\n{\n    public int Add(int a, int b)\n    {\n        return a + b;\n    }\n}\n",
    );

    let changeset = fixture.build(ChangeScope::WorkingTree);
    let add = changeset
        .symbols
        .iter()
        .find(|s| s.name == "Add")
        .expect("Add symbol expected");
    assert_eq!(add.change_type, SymbolChangeType::Modified);
    assert!(
        add.characteristics
            .contains(&prism_changes::SymbolCharacteristic::SignatureChanged),
        "signature change expected: {:?}",
        add.characteristics
    );
}

/// §91：中文/空格/括号/emoji 路径不损坏映射链路。
#[test]
fn special_paths_survive_the_whole_pipeline() {
    let fixture = Fixture::new("paths");
    fixture.write(
        "src/中文 模块(新).ts",
        "export function 特殊() {\n  return 1;\n}\n",
    );
    fixture.write(
        "src/emoji 🚀.ts",
        "export function rocket() {\n  return 1;\n}\n",
    );
    fixture.commit_all("base");
    fixture.write(
        "src/中文 模块(新).ts",
        "export function 特殊() {\n  return 2;\n}\n",
    );
    fixture.write(
        "src/emoji 🚀.ts",
        "export function rocket() {\n  return 2;\n}\n",
    );

    let changeset = fixture.build(ChangeScope::WorkingTree);
    assert_eq!(changeset.summary.files_modified, 2);
    assert!(
        changeset
            .symbols
            .iter()
            .any(|s| s.name == "特殊" && s.change_type == SymbolChangeType::Modified),
        "unicode symbol expected: {:?}",
        changeset.symbols
    );
    assert!(changeset
        .symbols
        .iter()
        .any(|s| s.name == "rocket" && s.change_type == SymbolChangeType::Modified));
}

/// §90：whitespace-only 变更不崩溃、不虚构语义结果。
#[test]
fn whitespace_only_change_stays_honest() {
    let fixture = Fixture::new("whitespace");
    fixture.write("src/a.ts", "export function spaced() {\n  return 1;\n}\n");
    fixture.commit_all("base");
    fixture.write("src/a.ts", "export function spaced() {\n    return 1;\n}\n");

    let changeset = fixture.build(ChangeScope::WorkingTree);
    assert_eq!(changeset.summary.files_modified, 1);
    // M5 不做语义等价证明（§46/§48）：diff 行变化 ⇒ 符号 Modified 是事实；
    // 不得出现"语义未变"之类无证据结论。
    assert!(changeset
        .symbols
        .iter()
        .all(|s| s.change_type == SymbolChangeType::Modified
            || s.change_type == SymbolChangeType::Added,));
}

/// §89：git-only 变更（rename-only 无 hunk）仍产生 FileChange。
#[test]
fn git_only_rename_without_hunks_still_recorded() {
    let fixture = Fixture::new("git-only");
    fixture.write("src/a.ts", "export const value = 1;\n");
    fixture.commit_all("base");
    fixture.move_file("src/a.ts", "src/b.ts");
    // rename 未提交（working tree stage）：mv 已入 index ⇒ Staged scope 可见 R。
    let changeset = fixture.build(ChangeScope::Staged);
    let renamed = changeset
        .files
        .iter()
        .find(|f| f.change_type == prism_changes::FileChangeType::Renamed);
    assert!(
        renamed.is_some(),
        "rename must be recorded: {:?}",
        changeset.files
    );
    assert_eq!(renamed.unwrap().old_path.as_deref(), Some("src/a.ts"));
}

/// §71/§35：文件数上限 ⇒ truncated 如实标注。
#[test]
fn file_limit_marks_changeset_truncated() {
    let fixture = Fixture::new("truncated");
    fixture.write("src/a.ts", "export const a = 1;\n");
    fixture.commit_all("base");
    for i in 0..5 {
        fixture.write(
            &format!("src/f{i}.ts"),
            &format!("export const f{i} = {i};\n"),
        );
    }

    let service = ChangeSetService::new(Arc::new(StdCommandRunner::new()));
    let changeset = service
        .build(
            &fixture.root,
            ChangeScope::WorkingTree,
            None,
            None,
            &ChangeSetOptions {
                max_files: 2,
                ..ChangeSetOptions::default()
            },
            &CancellationToken::new(),
        )
        .unwrap()
        .unwrap();
    assert!(changeset.truncated);
    assert_eq!(changeset.files.len(), 2);
}

/// §76/§77：历史源分析绝不改写用户工作区（对比后工作区文件原样保留）。
#[test]
fn revision_range_does_not_mutate_working_tree() {
    let fixture = Fixture::new("no-mutate");
    fixture.write("src/a.ts", "export const a = 1;\n");
    fixture.commit_all("c1");
    fixture.write("src/a.ts", "export const a = 2;\n");
    fixture.commit_all("c2");
    // 工作区再放一个未跟踪文件 + 一次未提交修改。
    fixture.write("scratch.txt", "user data");
    fixture.write("src/a.ts", "export const a = 3;\n");

    let before_a = std::fs::read_to_string(fixture.root.join("src/a.ts")).unwrap();
    let _ = fixture.build_with(ChangeScope::RevisionRange, Some("HEAD~1"), Some("HEAD"));
    let after_a = std::fs::read_to_string(fixture.root.join("src/a.ts")).unwrap();
    assert_eq!(before_a, after_a, "working tree must not be touched");
    assert_eq!(
        std::fs::read_to_string(fixture.root.join("scratch.txt")).unwrap(),
        "user data"
    );
    assert_eq!(fixture.git_status_lines(), 2, "untracked + modified a.ts");
}

impl Fixture {
    fn git_status_lines(&self) -> usize {
        let output = Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir(&self.root)
            .output()
            .unwrap();
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter(|line| !line.trim().is_empty())
            .count()
    }
}
