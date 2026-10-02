//! 应用信息服务：版本与运行时信息的唯一事实来源。
//!
//! CLI（`prism --version` / `prism info`）与 GUI（`get_app_info` 等 IPC 命令）
//! 都必须经由本服务获取，禁止各自硬编码。

use crate::timestamp::Timestamp;
use serde::Serialize;

/// Prism 应用名（对外产品名）。
pub const PRISM_NAME: &str = "Prism";

/// Prism 版本（与 workspace 版本一致，编译期固定）。
pub fn prism_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// 当前构建 profile（"debug" / "release"）。
pub fn build_profile() -> &'static str {
    if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    }
}

/// 应用信息（GUI Overview 页与 CLI `prism info` 的共同数据源）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AppInfo {
    pub name: &'static str,
    pub version: &'static str,
    pub os: &'static str,
    pub arch: &'static str,
    pub family: &'static str,
    pub build_profile: &'static str,
    pub generated_at: Timestamp,
}

/// 运行时信息（更侧重平台/构建事实）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RuntimeInfo {
    pub os: &'static str,
    pub arch: &'static str,
    pub family: &'static str,
    pub build_profile: &'static str,
    pub prism_version: &'static str,
}

/// 应用信息服务。
#[derive(Debug, Default, Clone, Copy)]
pub struct AppInfoService;

impl AppInfoService {
    pub fn new() -> Self {
        Self
    }

    pub fn app_info(&self) -> AppInfo {
        AppInfo {
            name: PRISM_NAME,
            version: prism_version(),
            os: std::env::consts::OS,
            arch: std::env::consts::ARCH,
            family: std::env::consts::FAMILY,
            build_profile: build_profile(),
            generated_at: Timestamp::now_utc(),
        }
    }

    pub fn runtime_info(&self) -> RuntimeInfo {
        RuntimeInfo {
            os: std::env::consts::OS,
            arch: std::env::consts::ARCH,
            family: std::env::consts::FAMILY,
            build_profile: build_profile(),
            prism_version: prism_version(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_matches_workspace_version() {
        assert_eq!(prism_version(), "0.1.0");
    }

    #[test]
    fn app_info_carries_real_platform_facts() {
        let info = AppInfoService::new().app_info();
        assert_eq!(info.name, "Prism");
        assert!(!info.os.is_empty());
        assert!(!info.arch.is_empty());
        assert!(!info.family.is_empty());
    }

    #[test]
    fn runtime_info_is_consistent_with_app_info() {
        let service = AppInfoService::new();
        let app = service.app_info();
        let runtime = service.runtime_info();
        assert_eq!(app.os, runtime.os);
        assert_eq!(app.version, runtime.prism_version);
    }

    #[test]
    fn app_info_serializes_with_snake_case_fields() {
        let info = AppInfoService::new().app_info();
        let json = serde_json::to_value(&info).unwrap();
        assert!(json.get("build_profile").is_some());
        assert!(json.get("generated_at").is_some());
        assert!(json.get("version").is_some());
    }
}
