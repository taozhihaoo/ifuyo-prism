//! Application 层：GUI（Tauri）与 CLI 之外的第三个视角——共享服务边界。
//!
//! 依赖方向：
//!
//! ```text
//! Frontend → Tauri commands → Application Services → Core (domain / runtime)
//! CLI                            ↑ 同一 Application 层
//! ```
//!
//! M0 只包含 [`app_info::AppInfoService`]；ProjectService / GitService /
//! AnalysisService / HealthService / VerifyService / ContextService 等
//! 随各自里程碑加入，不预先创建空壳。

pub mod app_info;

pub use app_info::{AppInfo, AppInfoService, RuntimeInfo};
