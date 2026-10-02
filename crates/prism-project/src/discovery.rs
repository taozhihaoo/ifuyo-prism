//! Discovery 结果模型：GUI / CLI / JSON report 共同消费的标准结构。
//!
//! schema_version 从 1 起（纲领 §72）；对外 JSON 契约稳定，内部结构可演进。

use crate::directory::DirectoryInfo;
use crate::entry_point::EntryPoint;
use crate::file::FileInfo;
use crate::language::Language;
use crate::project::Project;
use crate::project_type::ProjectTypeCandidate;
use crate::scanner::ScanError;
use crate::statistics::ProjectStatistics;
use crate::ConfigurationInfo;
use prism_core::ids::TaskId;
use prism_core::task::TaskState;
use prism_core::timestamp::Timestamp;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 对外 JSON 契约版本。
pub const SCHEMA_VERSION: u32 = 1;

/// 一次扫描的元信息。status 复用 M0 TaskState，不另造任务状态系统；
/// “completed with warnings” = Completed + errors_count > 0（纲领 §18/§19）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScanInfo {
    pub scan_id: TaskId,
    /// 项目根（绝对路径；文件本身一律相对路径）。
    pub root_path: PathBuf,
    pub started_at: Timestamp,
    pub finished_at: Option<Timestamp>,
    pub duration_ms: u64,
    pub files_scanned: u64,
    pub directories_scanned: u64,
    pub files_ignored: u64,
    pub errors_count: u64,
    pub status: TaskState,
}

/// 语言 → 文件数。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LanguageCount {
    pub language: Language,
    pub count: u64,
}

/// 完整的 Project Discovery 结果。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectDiscovery {
    pub schema_version: u32,
    pub project: Project,
    pub scan: ScanInfo,
    /// 按相对路径字典序。
    pub files: Vec<FileInfo>,
    /// 按相对路径字典序。
    pub directories: Vec<DirectoryInfo>,
    /// 按数量降序，同数量按语言名字典序。
    pub languages: Vec<LanguageCount>,
    /// 按置信度降序，同置信度按名字典序。
    pub project_types: Vec<ProjectTypeCandidate>,
    /// 按相对路径字典序。
    pub configurations: Vec<ConfigurationInfo>,
    /// 按置信度降序，同置信度按路径字典序。
    pub entry_points: Vec<EntryPoint>,
    pub statistics: ProjectStatistics,
    /// 非致命错误（scan completed with warnings 的依据）。
    pub errors: Vec<ScanError>,
}
