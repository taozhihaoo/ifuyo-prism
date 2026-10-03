//! prism-project — Project Discovery 领域层（M1）。
//!
//! 职责（发现事实，不解释事实）：
//!
//! - [`ignore`]：系统安全规则 + 项目 `.gitignore` + 用户规则的统一判断
//! - [`language`]：扩展名 → 语言（可扩展 registry）
//! - [`file`]：文件元数据、FileKind 分类（kind + confidence + reason）
//! - [`configuration`]：常见配置文件存在性识别（不解析内容）
//! - [`evidence`]：检测结果的可定位证据
//!
//! 路径规则：文件身份一律 `project root + relative_path`（`/` 分隔），
//! 绝对路径只在真正执行文件系统操作时使用。
//!
//! 明确不做（M2+）：AST / 符号 / 依赖图 / 架构分析 / Git 分析 / Health /
//! Verify / Context / 快照历史。

pub mod configuration;
pub mod directory;
pub mod discovery;
pub mod entry_point;
pub mod evidence;
pub mod file;
pub mod ignore;
pub mod language;
pub mod project;
pub mod project_type;
pub mod scanner;
pub mod service;
pub mod statistics;

pub use configuration::{ConfigurationInfo, ConfigurationType};
pub use directory::DirectoryInfo;
pub use discovery::{ProjectDiscovery, ScanInfo, SCHEMA_VERSION};
pub use entry_point::{EntryPoint, EntryPointKind};
pub use evidence::Evidence;
pub use file::{build_file_info, FileClassification, FileClassifier, FileInfo, FileKind};
pub use ignore::{IgnoreEngine, ScanOptions};
pub use language::{Language, LanguageRegistry};
pub use project::Project;
pub use project_type::ProjectTypeCandidate;
pub use scanner::{ScanError, ScanErrorKind, ScanProgress};
pub use service::DiscoveryService;
pub use statistics::ProjectStatistics;
