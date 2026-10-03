//! Git facts 编排入口（CLI 与 GUI 同源，规格 §56）。
//!
//! 高层方法组合 provider 的底层事实；快照式查询供 GUI Changes 页一次性消费。
//! 非 Git 项目：status/snapshot 返回 `repository: None` 而非错误（规格 §25）。

use crate::error::GitResult;
use crate::model::{GitDiff, GitDiffScope, GitStatus};
use crate::provider::{CliGitProvider, GitProvider};
use prism_core::task::CancellationToken;
use prism_core::CommandRunner;
use std::path::Path;
use std::sync::Arc;

#[derive(Clone)]
pub struct GitService {
    provider: Arc<dyn GitProvider>,
}

impl std::fmt::Debug for GitService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GitService").finish()
    }
}

impl GitService {
    pub fn new(runner: Arc<dyn CommandRunner>) -> Self {
        GitService {
            provider: Arc::new(CliGitProvider::new(runner)),
        }
    }

    pub fn with_provider(provider: Arc<dyn GitProvider>) -> Self {
        GitService { provider }
    }

    pub fn provider(&self) -> &dyn GitProvider {
        self.provider.as_ref()
    }

    /// 就近发现仓库；非 Git 项目返回 Ok(None)（规格 §25）。
    pub fn discover(
        &self,
        path: &Path,
        cancel: &CancellationToken,
    ) -> GitResult<Option<crate::model::GitRepository>> {
        self.provider.discover(path, cancel)
    }

    /// status；非 Git 项目返回 repository=None 的空状态。
    pub fn status(
        &self,
        path: &Path,
        include_ignored: bool,
        cancel: &CancellationToken,
    ) -> GitResult<GitStatus> {
        let Some(repository) = self.provider.discover(path, cancel)? else {
            return Ok(empty_status());
        };
        self.provider.status(&repository, include_ignored, cancel)
    }

    /// 最近 N 条提交；非 Git 项目返回空。
    pub fn log(
        &self,
        path: &Path,
        limit: u64,
        cancel: &CancellationToken,
    ) -> GitResult<Vec<crate::model::GitCommit>> {
        let Some(repository) = self.provider.discover(path, cancel)? else {
            return Ok(Vec::new());
        };
        self.provider.log(&repository, limit, cancel)
    }

    /// 按语义 scope 获取 diff；非 Git 项目返回空 diff。
    pub fn diff(
        &self,
        path: &Path,
        scope: GitDiffScope,
        base: Option<&str>,
        target: Option<&str>,
        max_files: usize,
        cancel: &CancellationToken,
    ) -> GitResult<GitDiff> {
        let Some(repository) = self.provider.discover(path, cancel)? else {
            return Ok(GitDiff {
                scope,
                base: base.map(str::to_string),
                target: target.map(str::to_string),
                files: Vec::new(),
                truncated: false,
            });
        };
        self.provider
            .diff(&repository, scope, base, target, max_files, cancel)
    }

    /// refs；非 Git 项目返回空。
    pub fn refs(
        &self,
        path: &Path,
        cancel: &CancellationToken,
    ) -> GitResult<Vec<crate::model::GitRef>> {
        let Some(repository) = self.provider.discover(path, cancel)? else {
            return Ok(Vec::new());
        };
        self.provider.refs(&repository, cancel)
    }

    /// 将 revision 表达式解析为完整 SHA；非 Git 项目返回 None。
    pub fn resolve_revision(
        &self,
        path: &Path,
        revision: &str,
        cancel: &CancellationToken,
    ) -> GitResult<Option<String>> {
        let Some(repository) = self.provider.discover(path, cancel)? else {
            return Ok(None);
        };
        self.provider.resolve_revision(&repository, revision, cancel)
    }

    /// 读取历史 revision 中某路径的 blob；非 Git 项目返回 Missing。
    /// 调用方应先用 [`GitService::discover`] 确认仓库存在。
    pub fn read_blob(
        &self,
        path: &Path,
        revision: &str,
        relative_path: &str,
        max_bytes: u64,
        cancel: &CancellationToken,
    ) -> GitResult<crate::model::RevisionBlob> {
        let Some(repository) = self.provider.discover(path, cancel)? else {
            return Ok(crate::model::RevisionBlob::Missing);
        };
        self.provider
            .read_blob(&repository, revision, relative_path, max_bytes, cancel)
    }
}

fn empty_status() -> GitStatus {
    GitStatus {
        repository: None,
        head: None,
        entries: Vec::new(),
        ignored: Vec::new(),
    }
}
