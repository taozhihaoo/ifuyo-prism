//! prism-git — Git facts 层（M4，只读，规格 §0/§2/§40）。
//!
//! - [`model`]：GitRepository / GitHead / GitBranch / GitRef / GitCommit /
//!   GitStatus / GitFileStatus / GitDiff*（归一化领域模型，非 CLI 字符串）
//! - [`error`]：GitError（区分 NotGitRepository / GitUnavailable /
//!   MalformedGitOutput 等，§26）
//! - [`parse`]：porcelain v2 -z / for-each-ref / log / numstat / raw / patch
//!   纯函数解析器
//! - [`provider`]：GitProvider trait + CliGitProvider（系统 git 经
//!   CommandRunner，只读命令白名单）
//! - [`service`]：GitService 门面（CLI/GUI 同源管线）
//!
//! 只读保证（§40）：不执行任何 mutating git 命令；非 Git 项目不是错误；
//! 不读取大量 binary 内容；日志不记录源码/diff 内容。

pub mod error;
pub mod model;
pub mod parse;
pub mod provider;
pub mod service;

pub use error::{GitError, GitResult};
pub use model::{
    GitBranch, GitChangeType, GitCommit, GitDiff, GitDiffFile, GitDiffHunk, GitDiffLine,
    GitDiffLineKind, GitDiffScope, GitFileState, GitFileStatus, GitHead, GitHeadKind, GitPerson,
    GitRef, GitRefKind, GitRepository, GitStatus, RevisionBlob,
};
pub use provider::{CliGitProvider, GitProvider};
pub use service::GitService;
