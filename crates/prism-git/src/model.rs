//! Git facts 归一化领域模型（规格 §6–§22）。
//!
//! 全部结构 serde snake_case、字段稳定、可进 JSON contract（§37）；
//! 路径一律项目/仓库相对（§31），绝对路径仅执行时使用。

use serde::{Deserialize, Serialize};

/// Git 仓库（规格 §6）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitRepository {
    /// 仓库工作树根（绝对路径，仅执行用）。
    pub root_path: String,
    /// .git 目录（绝对路径；worktree 场景指向真实 gitdir）。
    pub git_dir: String,
    /// 共享 gitdir（worktree 与主仓库共享）。
    pub common_git_dir: String,
    pub is_bare: bool,
    /// `.git` 为文件（linked worktree）时为 true。
    pub is_worktree: bool,
}

/// HEAD 状态（规格 §7）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitHead {
    pub kind: GitHeadKind,
    /// Resolved commit（unborn 时为 None）。
    pub commit_id: Option<String>,
    /// 当前分支短名（如 "main"；detached 时 None）。
    pub branch_name: Option<String>,
    /// 完整符号引用（如 "refs/heads/main"）。
    pub symbolic_ref: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GitHeadKind {
    Branch,
    Detached,
    Unborn,
    Unknown,
}

/// 分支（规格 §8）。ahead/behind 只有 Git 主动报告时才存在。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitBranch {
    pub name: String,
    pub full_ref: String,
    pub is_current: bool,
    pub is_remote: bool,
    pub upstream: Option<String>,
    pub ahead: Option<u64>,
    pub behind: Option<u64>,
    pub commit_id: Option<String>,
}

/// 引用（规格 §9）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitRef {
    pub name: String,
    pub full_name: String,
    pub kind: GitRefKind,
    pub target: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GitRefKind {
    Branch,
    RemoteBranch,
    Tag,
    Other,
}

impl GitRefKind {
    /// 稳定的对外字符串（与 serde 序列化一致）。
    pub const fn as_str(self) -> &'static str {
        match self {
            GitRefKind::Branch => "branch",
            GitRefKind::RemoteBranch => "remote_branch",
            GitRefKind::Tag => "tag",
            GitRefKind::Other => "other",
        }
    }
}

/// Git 作者/提交者身份。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitPerson {
    pub name: String,
    pub email: String,
}

/// 提交（规格 §10）：完整 SHA；merge 多 parent；initial 零 parent。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitCommit {
    pub id: String,
    pub short_id: Option<String>,
    pub parents: Vec<String>,
    pub author: GitPerson,
    pub committer: GitPerson,
    pub authored_at: Option<String>,
    pub committed_at: Option<String>,
    pub subject: String,
    pub message: String,
    pub refs: Vec<String>,
}

/// 文件状态（两个维度独立保留；规格 §11/§12：MM ≠ M）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitFileStatus {
    /// 当前路径（rename 后）。
    pub path: String,
    /// rename 前的原始路径。
    pub original_path: Option<String>,
    pub index_state: GitFileState,
    pub working_tree_state: GitFileState,
    pub is_untracked: bool,
    pub is_ignored: bool,
    pub is_renamed: bool,
    pub is_copied: bool,
    pub is_conflicted: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GitFileState {
    Unmodified,
    Added,
    Modified,
    Deleted,
    Renamed,
    Copied,
    TypeChanged,
    Unmerged,
    Untracked,
    Ignored,
    Unknown,
}

impl GitFileState {
    /// porcelain v1/v2 状态字母 → 状态。未知字母 → Unknown（不猜测）。
    pub const fn from_letter(letter: char) -> Self {
        match letter {
            '.' | ' ' => GitFileState::Unmodified,
            'A' => GitFileState::Added,
            'M' => GitFileState::Modified,
            'D' => GitFileState::Deleted,
            'R' => GitFileState::Renamed,
            'C' => GitFileState::Copied,
            'T' => GitFileState::TypeChanged,
            'U' => GitFileState::Unmerged,
            '?' => GitFileState::Untracked,
            '!' => GitFileState::Ignored,
            _ => GitFileState::Unknown,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            GitFileState::Unmodified => "unmodified",
            GitFileState::Added => "added",
            GitFileState::Modified => "modified",
            GitFileState::Deleted => "deleted",
            GitFileState::Renamed => "renamed",
            GitFileState::Copied => "copied",
            GitFileState::TypeChanged => "type_changed",
            GitFileState::Unmerged => "unmerged",
            GitFileState::Untracked => "untracked",
            GitFileState::Ignored => "ignored",
            GitFileState::Unknown => "unknown",
        }
    }
}

/// 汇总状态（规格 §32/§33 的 GUI 事实来源）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitStatus {
    pub repository: Option<GitRepository>,
    pub head: Option<GitHead>,
    pub entries: Vec<GitFileStatus>,
    /// 忽略的文件（仅 ignored=matching 查询时填充）。
    pub ignored: Vec<String>,
}

/// 三种 diff 语义显式建模（规格 §17）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GitDiffScope {
    /// 工作区 vs index（`git diff`）。
    WorkingTree,
    /// index vs HEAD（`git diff --cached`）。
    Staged,
    /// 两个 revision 之间（`git diff from to`）。
    RevisionRange,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GitChangeType {
    Added,
    Modified,
    Deleted,
    Renamed,
    Copied,
    TypeChanged,
    Unmerged,
    Unknown,
}

impl GitChangeType {
    /// `--raw` 状态字母 → 变更类型。
    pub const fn from_letter(letter: char) -> Self {
        match letter {
            'A' => GitChangeType::Added,
            'M' => GitChangeType::Modified,
            'D' => GitChangeType::Deleted,
            'R' => GitChangeType::Renamed,
            'C' => GitChangeType::Copied,
            'T' => GitChangeType::TypeChanged,
            'U' => GitChangeType::Unmerged,
            _ => GitChangeType::Unknown,
        }
    }
}

/// diff 行（规格 §16）。old_line/new_line 从 1 计，None 表示该侧不存在。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitDiffLine {
    pub kind: GitDiffLineKind,
    pub old_line: Option<u64>,
    pub new_line: Option<u64>,
    pub content: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GitDiffLineKind {
    Context,
    Addition,
    Deletion,
}

/// hunk（规格 §15）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitDiffHunk {
    pub old_start: u64,
    pub old_lines: u64,
    pub new_start: u64,
    pub new_lines: u64,
    /// @@ 行尾的 section header（如函数名），可为空串。
    pub header: String,
    pub lines: Vec<GitDiffLine>,
}

/// 单文件 diff（规格 §14；保留 M5 需要的原始信息：路径/类型/hunk/blob/mode/rename）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitDiffFile {
    pub old_path: Option<String>,
    pub new_path: Option<String>,
    pub change_type: GitChangeType,
    pub old_mode: Option<String>,
    pub new_mode: Option<String>,
    pub old_blob: Option<String>,
    pub new_blob: Option<String>,
    pub is_binary: bool,
    pub is_rename: bool,
    /// Git 报告的相似度（rename/copy 时，百分比整数）；未提供为 None。
    pub similarity: Option<u64>,
    pub additions: Option<u64>,
    pub deletions: Option<u64>,
    pub hunks: Vec<GitDiffHunk>,
}

/// 一次 diff 查询的完整事实。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitDiff {
    pub scope: GitDiffScope,
    /// RevisionRange 起止（其他 scope 为 None）。
    pub base: Option<String>,
    pub target: Option<String>,
    /// 按新旧路径排序。
    pub files: Vec<GitDiffFile>,
    /// 结果被截断（超出文件数/字节上限）时为 true，如实标注（规格 §35）。
    pub truncated: bool,
}

/// 一次历史 revision blob 读取的结果（M5 历史源；只读，规格 M5 §39）。
///
/// 编码契约：`CommandRunner` 以 UTF-8 读取子进程 stdout；blob 字节长度
/// 与 `cat-file -s` 大小不一致时不得假装解析（会伪造"全部符号删除"）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RevisionBlob {
    /// UTF-8 文本，字节长度已与 blob 大小核对一致。
    Text { content: String },
    /// 超过调用方字节上限（未读取内容）。
    TooLarge { size_bytes: u64 },
    /// 非 UTF-8（或读取失真）：内容不可靠，显式不可用而非空内容。
    EncodingUnsupported,
    /// 该 revision 中不存在此路径。
    Missing,
}
