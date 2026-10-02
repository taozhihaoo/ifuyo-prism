//! Git 输出解析器（纯函数，规格 §29/§30）。
//!
//! 只接受 porcelain/机器格式；文件名以 NUL 分隔（-z），
//! 路径含空格/tab/unicode/引号均安全；log 用 0x1f/0x1e 分隔。
//! 解析失败返回 MalformedGitOutput，不猜测。

use crate::error::{GitError, GitResult};
use crate::model::{
    GitBranch, GitChangeType, GitCommit, GitDiffFile, GitDiffHunk, GitDiffLine, GitDiffLineKind,
    GitFileState, GitFileStatus, GitPerson, GitRef, GitRefKind,
};
use std::collections::BTreeMap;

fn malformed(command: &str, detail: &str) -> GitError {
    GitError::MalformedGitOutput {
        command: command.to_string(),
        detail: detail.to_string(),
    }
}

/// porcelain v2 头部：branch.oid / branch.head / branch.upstream / branch.ab。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StatusHead {
    pub oid: Option<String>,
    /// `branch.head`（detached 时为 "(detached)"）。
    pub head: Option<String>,
    pub upstream: Option<String>,
    pub ahead: Option<u64>,
    pub behind: Option<u64>,
}

pub fn parse_status_head(output: &str) -> StatusHead {
    let mut head = StatusHead::default();
    for line in output.lines() {
        if let Some(value) = line.strip_prefix("# branch.oid ") {
            head.oid = Some(value.to_string());
        } else if let Some(value) = line.strip_prefix("# branch.head ") {
            head.head = Some(value.to_string());
        } else if let Some(value) = line.strip_prefix("# branch.upstream ") {
            head.upstream = Some(value.to_string());
        } else if let Some(value) = line.strip_prefix("# branch.ab ") {
            for part in value.split(' ') {
                if let Some(ahead) = part.strip_prefix('+') {
                    head.ahead = ahead.parse().ok();
                } else if let Some(behind) = part.strip_prefix('-') {
                    head.behind = behind.parse().ok();
                }
            }
        }
    }
    head
}

/// porcelain v2 -z 状态解析（规格 §29）。
///
/// 记录以 NUL 分隔；rename 的 path 与 origPath 以相邻两个 NUL 记录表达。
/// 返回 (状态条目, 忽略文件列表)。
pub fn parse_status_v2(output: &str) -> GitResult<(Vec<GitFileStatus>, Vec<String>)> {
    let records: Vec<&str> = output.split('\0').filter(|r| !r.is_empty()).collect();
    let mut entries: Vec<GitFileStatus> = Vec::new();
    let mut ignored: Vec<String> = Vec::new();

    let mut i = 0;
    while i < records.len() {
        // 保留行内空格语义，仅去掉首部空白（多行 Rust 字符串常带缩进）。
        let record = records[i];
        let record = record.trim_start();
        let mut fields = record.split(' ');
        let tag = fields.next().unwrap_or("");
        match tag {
            "1" => {
                let xy = fields.next().ok_or(malformed("status", "missing XY"))?;
                let path = record
                    .splitn(9, ' ')
                    .nth(8)
                    .ok_or(malformed("status", "missing path"))?;
                entries.push(make_status(path, xy, None));
                i += 1;
            }
            "2" => {
                eprintln!("DEBUG status 2 record: {:?}", record);
                let xy = fields.next().ok_or(malformed("status", "missing XY"))?;
                let path = record
                    .splitn(10, ' ')
                    .nth(9)
                    .ok_or(malformed("status", "missing path"))?;
                let orig_path = records.get(i + 1).copied();
                entries.push(make_status(path, xy, orig_path));
                // -z rename 的 origPath 占用下一条 NUL 记录。
                if orig_path.is_some() {
                    i += 1;
                }
                i += 1;
            }
            "u" => {
                let path = record.rsplit(' ').next().unwrap_or("");
                entries.push(GitFileStatus {
                    path: path.to_string(),
                    original_path: None,
                    index_state: GitFileState::Unmerged,
                    working_tree_state: GitFileState::Unmerged,
                    is_untracked: false,
                    is_ignored: false,
                    is_renamed: false,
                    is_copied: false,
                    is_conflicted: true,
                });
                i += 1;
            }
            "?" => {
                // 记录形如 "? <path>"（-z 下 NUL 分隔）。
                let path = record.strip_prefix("? ").unwrap_or(record);
                entries.push(GitFileStatus {
                    path: path.to_string(),
                    original_path: None,
                    index_state: GitFileState::Unmodified,
                    working_tree_state: GitFileState::Untracked,
                    is_untracked: true,
                    is_ignored: false,
                    is_renamed: false,
                    is_copied: false,
                    is_conflicted: false,
                });
                i += 1;
            }
            "!" => {
                let path = record.strip_prefix("! ").unwrap_or(record);
                ignored.push(path.to_string());
                i += 1;
            }
            _ => i += 1, // 头部行（# branch.*）在此跳过
        }
    }

    entries.sort_by(|a, b| a.path.cmp(&b.path));
    ignored.sort();
    Ok((entries, ignored))
}

fn make_status(path: &str, xy: &str, orig_path: Option<&str>) -> GitFileStatus {
    let mut chars = xy.chars();
    let index = chars.next().unwrap_or('.');
    let worktree = chars.next().unwrap_or('.');
    let is_renamed = index == 'R' || worktree == 'R';
    let is_copied = index == 'C' || worktree == 'C';
    GitFileStatus {
        path: path.to_string(),
        original_path: orig_path.map(str::to_string),
        index_state: GitFileState::from_letter(index),
        working_tree_state: GitFileState::from_letter(worktree),
        is_untracked: false,
        is_ignored: false,
        is_renamed,
        is_copied,
        is_conflicted: false,
    }
}

/// for-each-ref（字段以 0x01 分隔）：refname, objectname, objecttype,
/// refname:short, upstream:short, upstream:track。→ refs
pub fn parse_refs(output: &str) -> Vec<GitRef> {
    let mut refs = Vec::new();
    for record in output.split('\n') {
        if record.is_empty() {
            continue;
        }
        let fields: Vec<&str> = record.split('\u{1}').collect();
        if fields.len() < 4 {
            continue;
        }
        let full_name = fields[0];
        let kind = if full_name.starts_with("refs/heads/") {
            GitRefKind::Branch
        } else if full_name.starts_with("refs/remotes/") {
            GitRefKind::RemoteBranch
        } else if full_name.starts_with("refs/tags/") {
            GitRefKind::Tag
        } else {
            GitRefKind::Other
        };
        refs.push(GitRef {
            name: fields[3].to_string(),
            full_name: full_name.to_string(),
            kind,
            target: fields[1].to_string(),
        });
    }
    refs
}

/// 同一 for-each-ref 输出 → 分支（含 upstream/ahead/behind，规格 §8）。
pub fn parse_branches(output: &str, current: Option<&str>) -> Vec<GitBranch> {
    let mut branches = Vec::new();
    for record in output.split('\n') {
        if record.is_empty() {
            continue;
        }
        let fields: Vec<&str> = record.split('\u{1}').collect();
        if fields.len() < 4 {
            continue;
        }
        let full_name = fields[0];
        let is_remote = full_name.starts_with("refs/remotes/");
        if !(full_name.starts_with("refs/heads/") || is_remote) {
            continue;
        }
        let short = fields[3].to_string();
        let (ahead, behind) = parse_track(fields.get(5).copied());
        branches.push(GitBranch {
            name: short.clone(),
            full_ref: full_name.to_string(),
            is_current: !is_remote && current == Some(short.as_str()),
            is_remote,
            upstream: fields
                .get(4)
                .and_then(|u| (!u.is_empty()).then(|| u.to_string())),
            ahead,
            behind,
            commit_id: Some(fields[1].to_string()),
        });
    }
    branches.sort_by(|a, b| a.full_ref.cmp(&b.full_ref));
    branches
}

fn parse_track(track: Option<&str>) -> (Option<u64>, Option<u64>) {
    let mut ahead = None;
    let mut behind = None;
    if let Some(track) = track {
        if !track.is_empty() && track != "gone" {
            // git 输出形如 "[ahead 2, behind 1]"（括号由 git 添加）。
            let inner = track.trim_start_matches('[').trim_end_matches(']');
            for part in inner.split(", ") {
                if let Some(value) = part.strip_prefix("ahead ") {
                    ahead = value.parse().ok();
                } else if let Some(value) = part.strip_prefix("behind ") {
                    behind = value.parse().ok();
                }
            }
        }
    }
    (ahead, behind)
}

/// git log 解析：记录以 0x1e 结尾，字段以 0x1f 分隔。
/// 字段：id, parents, author_name, author_email, authored_at,
/// committer_name, committer_email, committed_at, subject, body, refs。
pub fn parse_log(output: &str) -> Vec<GitCommit> {
    let mut commits = Vec::new();
    for record in output.split('\u{1e}') {
        let record = record.trim_start_matches('\n');
        if record.is_empty() {
            continue;
        }
        let fields: Vec<&str> = record.split('\u{1f}').collect();
        if fields.len() < 10 {
            eprintln!(
                "DEBUG parse_log skip: {} fields, first={:?}",
                fields.len(),
                fields
                    .first()
                    .map(|f| f.chars().take(24).collect::<String>())
            );
            continue;
        }
        let person = |name: &str, email: &str| GitPerson {
            name: name.to_string(),
            email: email.to_string(),
        };
        commits.push(GitCommit {
            id: fields[0].to_string(),
            short_id: None,
            parents: fields[1]
                .split(' ')
                .filter(|p| !p.is_empty())
                .map(str::to_string)
                .collect(),
            author: person(fields[2], fields[3]),
            committer: person(fields[5], fields[6]),
            authored_at: Some(fields[4].to_string()),
            committed_at: Some(fields[7].to_string()),
            subject: fields[8].to_string(),
            message: fields[9].to_string(),
            refs: fields
                .get(10)
                .map(|refs| {
                    refs.split(", ")
                        .filter(|r| !r.is_empty())
                        .map(str::to_string)
                        .collect()
                })
                .unwrap_or_default(),
        });
    }
    commits
}

/// numstat -z 解析 → path → (additions, deletions, is_binary)。
/// rename 时 git 输出 `add\tdel\tnew\0orig\0`。
pub fn parse_numstat(output: &str) -> BTreeMap<String, (Option<u64>, Option<u64>, bool)> {
    let mut map = BTreeMap::new();
    let records: Vec<&str> = output.split('\0').filter(|r| !r.is_empty()).collect();
    let mut i = 0;
    while i < records.len() {
        let record = records[i];
        let mut parts = record.split('\t');
        let additions = parts.next().unwrap_or("");
        let deletions = parts.next().unwrap_or("");
        let Some(path) = parts.next() else {
            i += 1;
            continue;
        };
        let is_binary = additions == "-" || deletions == "-";
        let additions = (!is_binary)
            .then(|| additions.parse())
            .transpose()
            .ok()
            .flatten();
        let deletions = (!is_binary)
            .then(|| deletions.parse())
            .transpose()
            .ok()
            .flatten();
        if parts.next().is_some() {
            // rename：orig path 在下一条 NUL 记录；以 orig path 为键（旧路径统计）。
            if let Some(orig) = records.get(i + 1) {
                map.insert(orig.to_string(), (additions, deletions, is_binary));
            }
            i += 2;
        } else {
            map.insert(path.to_string(), (additions, deletions, is_binary));
            i += 1;
        }
    }
    map
}

/// `git diff --raw -z` 解析。
/// 记录：`: <old_mode> <new_mode> <old_blob> <new_blob> <status>\0<path>[\0<orig>]`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawDiffEntry {
    pub old_mode: Option<String>,
    pub new_mode: Option<String>,
    pub old_blob: Option<String>,
    pub new_blob: Option<String>,
    pub change_type: GitChangeType,
    pub path: String,
    pub original_path: Option<String>,
    pub is_rename: bool,
    pub similarity: Option<u64>,
}

pub fn parse_raw_diff(output: &str) -> Vec<RawDiffEntry> {
    let records: Vec<&str> = output.split('\0').filter(|r| !r.is_empty()).collect();
    let mut entries = Vec::new();
    let mut i = 0;
    while i < records.len() {
        let record = records[i];
        let Some(inner) = record.strip_prefix(':') else {
            i += 1;
            continue;
        };
        let mut fields = inner.split(' ');
        let old_mode = fields.next().map(str::to_string);
        let new_mode = fields.next().map(str::to_string);
        let old_blob = fields.next().map(str::to_string);
        let new_blob = fields.next().map(str::to_string);
        let status = fields.next().unwrap_or("X");
        let mut status_chars = status.chars();
        let change = status_chars.next().unwrap_or('X');
        let similarity: Option<u64> = status_chars.as_str().parse().ok();
        let is_rename = change == 'R' || change == 'C';
        let Some(path) = records.get(i + 1) else {
            break;
        };
        let orig_path = if is_rename {
            let orig = records.get(i + 2).cloned();
            i += 1;
            orig
        } else {
            None
        };
        entries.push(RawDiffEntry {
            old_mode,
            new_mode,
            old_blob,
            new_blob,
            change_type: GitChangeType::from_letter(change),
            path: path.to_string(),
            original_path: orig_path.map(str::to_string),
            is_rename,
            similarity,
        });
        i += 2;
    }
    entries
}

/// unified diff patch → 每文件 hunk/行（规格 §15/§16）。
/// 行号计数遵循 hunk 头；"\ No newline" 行跳过不计。
pub fn parse_patch(output: &str) -> Vec<GitDiffFile> {
    let mut files: Vec<GitDiffFile> = Vec::new();
    let mut current: Option<GitDiffFile> = None;
    let mut hunk: Option<HunkState> = None;

    for line in output.split('\n') {
        if let Some(rest) = line.strip_prefix("diff --git ") {
            if let Some(mut file) = current.take() {
                if let Some(state) = hunk.take() {
                    file.hunks.push(state.finish());
                }
                files.push(file);
            }
            let (old_path, new_path) = split_diff_git_paths(rest);
            current = Some(GitDiffFile {
                old_path: Some(old_path),
                new_path: Some(new_path),
                change_type: GitChangeType::Modified,
                old_mode: None,
                new_mode: None,
                old_blob: None,
                new_blob: None,
                is_binary: false,
                is_rename: false,
                similarity: None,
                additions: None,
                deletions: None,
                hunks: Vec::new(),
            });
            hunk = None;
            continue;
        }
        let Some(file) = current.as_mut() else {
            continue;
        };
        if let Some(mode) = line.strip_prefix("old mode ") {
            file.old_mode = Some(mode.to_string());
        } else if let Some(mode) = line.strip_prefix("new mode ") {
            file.new_mode = Some(mode.to_string());
        } else if let Some(similarity) = line
            .strip_prefix("similarity index ")
            .and_then(|v| v.strip_suffix('%'))
        {
            file.similarity = similarity.parse().ok();
        } else if line.starts_with("Binary files ") || line.starts_with("GIT binary patch") {
            file.is_binary = true;
        } else if let Some(old) = line.strip_prefix("rename from ") {
            file.is_rename = true;
            file.change_type = GitChangeType::Renamed;
            file.old_path = Some(old.to_string());
        } else if let Some(new) = line.strip_prefix("rename to ") {
            file.is_rename = true;
            file.change_type = GitChangeType::Renamed;
            file.new_path = Some(new.to_string());
        } else if line.starts_with("deleted file mode ") {
            file.change_type = GitChangeType::Deleted;
        } else if line.starts_with("new file mode ") {
            file.change_type = GitChangeType::Added;
        } else if let Some(header) = line.strip_prefix("@@ ") {
            if let Some(state) = hunk.take() {
                file.hunks.push(state.finish());
            }
            let (ranges, section) = split_hunk_header(header);
            let values = parse_hunk_ranges(ranges);
            hunk = Some(HunkState {
                old_start: values.0,
                old_count: values.1,
                new_start: values.2,
                new_count: values.3,
                old_cursor: values.0,
                new_cursor: values.2,
                header: section.to_string(),
                lines: Vec::new(),
            });
        } else if let Some(state) = hunk.as_mut() {
            let Some(first) = line.chars().next() else {
                continue;
            };
            let kind = match first {
                '+' => GitDiffLineKind::Addition,
                '-' => GitDiffLineKind::Deletion,
                ' ' => GitDiffLineKind::Context,
                _ => continue, // "\ No newline..." 等不计行
            };
            eprintln!("DEBUG patch line kind={:?} content={:?}", kind, line);
            let (old_line, new_line) = match kind {
                GitDiffLineKind::Context => (Some(state.old_cursor), Some(state.new_cursor)),
                GitDiffLineKind::Addition => (None, Some(state.new_cursor)),
                GitDiffLineKind::Deletion => (Some(state.old_cursor), None),
            };
            if kind != GitDiffLineKind::Addition {
                state.old_cursor += 1;
            }
            if kind != GitDiffLineKind::Deletion {
                state.new_cursor += 1;
            }
            state.lines.push(GitDiffLine {
                kind,
                old_line,
                new_line,
                content: line[1..].to_string(),
            });
        }
    }
    if let Some(state) = hunk.take() {
        if let Some(file) = current.as_mut() {
            file.hunks.push(state.finish());
        }
    }
    if let Some(file) = current.take() {
        files.push(file);
    }
    files
}

struct HunkState {
    old_start: u64,
    old_count: u64,
    new_start: u64,
    new_count: u64,
    old_cursor: u64,
    new_cursor: u64,
    header: String,
    lines: Vec<GitDiffLine>,
}

impl HunkState {
    fn finish(self) -> GitDiffHunk {
        GitDiffHunk {
            old_start: self.old_start,
            old_lines: self.old_count,
            new_start: self.new_start,
            new_lines: self.new_count,
            header: self.header,
            lines: self.lines,
        }
    }
}

/// `@@ -a[,b] +c[,d] @@ section` → ((a,b),(c,d),section)。
fn split_hunk_header(rest: &str) -> (&str, &str) {
    match rest.split_once("@@") {
        Some((ranges, section)) => (ranges, section.trim()),
        None => (rest, ""),
    }
}

fn parse_hunk_ranges(ranges: &str) -> (u64, u64, u64, u64) {
    let mut old = (0u64, 1u64);
    let mut new = (0u64, 1u64);
    for part in ranges.split(' ') {
        let Some(value) = part.strip_prefix('-') else {
            if let Some(value) = part.strip_prefix('+') {
                new = parse_range(value);
            }
            continue;
        };
        old = parse_range(value);
    }
    (old.0, old.1, new.0, new.1)
}

fn parse_range(value: &str) -> (u64, u64) {
    match value.split_once(',') {
        Some((start, count)) => (start.parse().unwrap_or(0), count.parse().unwrap_or(1)),
        None => (value.parse().unwrap_or(0), 1),
    }
}

/// `a/path b/path` → (path, path)。不猜测转义；引号原样保留字段截取。
fn split_diff_git_paths(rest: &str) -> (String, String) {
    if let Some(pos) = rest.find(" b/") {
        let old = rest[..pos]
            .trim_start_matches('a')
            .trim_start_matches('/')
            .to_string();
        let new = rest[pos + 3..].to_string();
        (old, new)
    } else {
        (rest.to_string(), rest.to_string())
    }
}
