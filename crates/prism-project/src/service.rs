//! Discovery 服务：M1 管线的编排入口（CLI 与 GUI 共用，纲领 §69/§70）。
//!
//! ```text
//! Project Root → Walker → Ignore → Metadata → Classification → Language
//!   → Configuration → Project Type → Entry Point → Statistics → ProjectDiscovery
//! ```

use crate::discovery::ProjectDiscovery;
use crate::ignore::ScanOptions;
use crate::project::Project;
use crate::scanner::{ScanProgress, Scanner};
use prism_core::task::CancellationToken;
use prism_core::PrismResult;
use std::path::Path;

/// 项目发现服务（无状态；GUI 侧的会话管理在应用壳中）。
#[derive(Debug, Default, Clone, Copy)]
pub struct DiscoveryService;

impl DiscoveryService {
    pub fn new() -> Self {
        DiscoveryService
    }

    /// 打开项目：验证目录有效性并建立 Project。
    pub fn open_project(&self, root: &Path) -> PrismResult<Project> {
        Project::from_root(root)
    }

    /// 执行完整扫描（同步；GUI 在后台线程调用并轮询进度）。
    pub fn discover(
        &self,
        root: &Path,
        options: ScanOptions,
        cancel: &CancellationToken,
        on_progress: impl FnMut(ScanProgress),
    ) -> PrismResult<ProjectDiscovery> {
        let project = self.open_project(root)?;
        Scanner::new(options).scan(&project, cancel, on_progress)
    }
}
