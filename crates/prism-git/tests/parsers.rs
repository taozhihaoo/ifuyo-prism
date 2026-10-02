//! porcelain v2 / log / refs / numstat / raw / patch 解析器单元测试。
//! 这些只验证解析逻辑；真实 git 行为由 tests/git_fixture.rs 覆盖（规格 §65）。

use prism_git::model::{GitChangeType, GitDiffLineKind, GitFileState, GitRefKind};
use prism_git::parse::{
    parse_branches, parse_log, parse_numstat, parse_patch, parse_raw_diff, parse_refs,
    parse_status_head, parse_status_v2,
};

#[test]
fn status_v2_parses_entries_and_ignored() {
    // NUL 用 \0 显式表达（与真实 -z 输出一致）。
    let output = "# branch.oid abc123\0# branch.head main\0\
        1 .M N... 100644 100644 100644 hashH hashI src/modified.ts\0\
        1 AM N... 100644 100644 100644 hashH hashI src/added.ts\0\
        ? src/untracked.ts\0\
        ! target/debug.bin\0";
    let (entries, ignored) = parse_status_v2(output).unwrap();
    assert_eq!(entries.len(), 3);

    let modified = entries
        .iter()
        .find(|e| e.path == "src/modified.ts")
        .unwrap();
    assert_eq!(modified.index_state, GitFileState::Unmodified);
    assert_eq!(modified.working_tree_state, GitFileState::Modified);

    let added = entries.iter().find(|e| e.path == "src/added.ts").unwrap();
    // AM：index=Added，worktree=Modified，两维度独立（规格 §11）。
    assert_eq!(added.index_state, GitFileState::Added);
    assert_eq!(added.working_tree_state, GitFileState::Modified);

    let untracked = entries
        .iter()
        .find(|e| e.path == "src/untracked.ts")
        .unwrap();
    assert!(untracked.is_untracked);
    assert!(!untracked.is_ignored);

    assert_eq!(ignored, vec!["target/debug.bin".to_string()]);
}

#[test]
fn status_v2_parses_rename_with_both_paths() {
    let output = "2 R. N... 100644 100644 100644 hashH hashI R78 new-name.ts\0old-name.ts\0";
    let (entries, _) = parse_status_v2(output).unwrap();
    assert_eq!(entries.len(), 1);
    let renamed = &entries[0];
    assert!(renamed.is_renamed);
    assert_eq!(renamed.path, "new-name.ts");
    assert_eq!(renamed.original_path.as_deref(), Some("old-name.ts"));
}

#[test]
fn status_head_parses_branch_and_ab() {
    let head = parse_status_head(
        "# branch.oid abc\n# branch.head main\n# branch.upstream origin/main\n# branch.ab +3 -1\n",
    );
    assert_eq!(head.oid.as_deref(), Some("abc"));
    assert_eq!(head.head.as_deref(), Some("main"));
    assert_eq!(head.ahead, Some(3));
    assert_eq!(head.behind, Some(1));
}

#[test]
fn refs_parse_kinds() {
    let output = "refs/heads/main\u{1}abc\u{1}commit\u{1}main\u{1}\u{1}\n\
        refs/remotes/origin/main\u{1}def\u{1}commit\u{1}origin/main\u{1}\u{1}\n\
        refs/tags/v1\u{1}123\u{1}tag\u{1}v1\u{1}\u{1}\n";
    let refs = parse_refs(output);
    assert_eq!(refs.len(), 3);
    assert_eq!(refs[0].kind, GitRefKind::Branch);
    assert_eq!(refs[1].kind, GitRefKind::RemoteBranch);
    assert_eq!(refs[2].kind, GitRefKind::Tag);
}

#[test]
fn branches_parse_upstream_track() {
    let output =
        "refs/heads/main\u{1}abc\u{1}commit\u{1}main\u{1}origin/main\u{1}[ahead 2, behind 1]\n\
        refs/heads/feature\u{1}def\u{1}commit\u{1}feature\u{1}\u{1}\n";
    let branches = parse_branches(output, Some("main"));
    assert_eq!(branches.len(), 2);
    let main = branches
        .iter()
        .find(|b| b.name == "main")
        .expect("main branch");
    assert!(main.is_current);
    assert_eq!(main.ahead, Some(2));
    assert_eq!(main.behind, Some(1));
    let feature = branches.iter().find(|b| b.name == "feature").unwrap();
    assert!(!feature.is_current);
}

#[test]
fn log_parses_merge_and_initial() {
    let sep = '\u{1f}';
    let record = '\u{1e}';
    let output = format!(
        "merge{sep}p1 p2{sep}A{sep}a@x{sep}2026-01-01T00:00:00+00:00{sep}A{sep}a@x{sep}2026-01-01T00:00:00+00:00{sep}Merge branch 'feature'{sep}{sep}HEAD -> main{record}\n\
         initial{sep}{sep}B{sep}b@x{sep}2026-01-02T00:00:00+00:00{sep}B{sep}b@x{sep}2026-01-02T00:00:00+00:00{sep}initial{sep}body{sep}{record}",
        sep = sep,
        record = record,
    );
    let commits = parse_log(&output);
    assert_eq!(commits.len(), 2);
    // merge：两个 parents（规格 §19）。
    assert_eq!(commits[0].parents, vec!["p1", "p2"]);
    // initial：零 parents（规格 §10）。
    assert!(commits[1].parents.is_empty());
    assert_eq!(commits[0].refs, vec!["HEAD -> main"]);
}

#[test]
fn raw_diff_parses_rename_and_modes() {
    let output = ":100644 100755 abc def M\0src/script.sh\0\
        :100644 100644 123 456 R100\0new.ts\0old.ts\0";
    let entries = parse_raw_diff(output);
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].change_type, GitChangeType::Modified);
    assert_eq!(entries[0].new_mode.as_deref(), Some("100755"));
    assert!(entries[1].is_rename);
    assert_eq!(entries[1].path, "new.ts");
    assert_eq!(entries[1].original_path.as_deref(), Some("old.ts"));
    assert_eq!(entries[1].similarity, Some(100));
}

#[test]
fn numstat_parses_binary_and_rename() {
    let output = "10\t2\tsrc/a.ts\0-\t-\tlogo.png\0";
    let map = parse_numstat(output);
    let a = map.get("src/a.ts").unwrap();
    assert_eq!(a.0, Some(10));
    assert!(!a.2);
    let logo = map.get("logo.png").unwrap();
    assert!(logo.2, "binary marker");
}

#[test]
fn patch_parses_hunks_and_line_numbers() {
    // 用 raw string：Rust 行 continuation 会吞掉行首空格（" context" 是内容）。
    let patch = r#"diff --git a/src/app.ts b/src/app.ts
index abc..def 100644
--- a/src/app.ts
+++ b/src/app.ts
@@ -1,3 +1,3 @@ section
 context
-removed
+added
@@ -10 +10,2 @@
+new line
"#;
    let files = parse_patch(patch);
    assert_eq!(files.len(), 1);
    let file = &files[0];
    assert_eq!(file.hunks.len(), 2);
    let first = &file.hunks[0];
    assert_eq!(first.old_start, 1);
    assert_eq!(first.new_start, 1);
    assert_eq!(first.header, "section");
    assert_eq!(first.lines.len(), 3);
    let removed = first
        .lines
        .iter()
        .find(|l| l.kind == GitDiffLineKind::Deletion)
        .unwrap();
    assert_eq!(removed.old_line, Some(2));
    assert_eq!(removed.new_line, None);
    let added = first
        .lines
        .iter()
        .find(|l| l.kind == GitDiffLineKind::Addition)
        .unwrap();
    assert_eq!(added.new_line, Some(2));
    // 第二个 hunk 单行 range
    assert_eq!(file.hunks[1].old_start, 10);
    assert_eq!(file.hunks[1].old_lines, 1);
}
