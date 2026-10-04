//! 数据库路径（规格 M11a §21/§220：OS app-data 目录；不入项目仓库；
//! `PRISM_DB_PATH` 环境变量可覆盖——测试与高级用户用途，显式且文档化）。
//!
//! 不硬编码单一平台：Windows 用 `%APPDATA%\Prism`，Unix 用
//! `$XDG_DATA_HOME|$HOME/.local/share` 下的 `Prism`。

use std::path::{Path, PathBuf};

/// 默认数据库路径：`<app-data>/Prism/prism.db`。
pub fn default_db_path() -> PathBuf {
    match std::env::var("PRISM_DB_PATH") {
        Ok(value) if !value.trim().is_empty() => db_path_from_env(&value),
        _ => {
            let mut path = app_data_dir();
            path.push("Prism");
            path.push("prism.db");
            path
        }
    }
}

/// 显式路径 → 数据库文件路径（§220：允许传目录或文件；目录时补 prism.db）。
pub fn db_path_from_env(value: &str) -> PathBuf {
    let path = Path::new(value);
    if path.extension().is_some() {
        path.to_path_buf()
    } else {
        path.join("prism.db")
    }
}

/// 平台 app-data 目录（§21：Windows AppData\Roaming；Unix XDG data home）。
fn app_data_dir() -> PathBuf {
    if cfg!(windows) {
        let base = std::env::var("APPDATA")
            .unwrap_or_else(|_| std::env::var("USERPROFILE").unwrap_or_else(|_| ".".to_string()));
        PathBuf::from(base)
    } else {
        std::env::var("XDG_DATA_HOME")
            .ok()
            .filter(|value| !value.is_empty())
            .or_else(|| std::env::var("HOME").ok())
            .map(|home| PathBuf::from(home).join(".local/share"))
            .unwrap_or_else(|| PathBuf::from("."))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_override_with_file_name_used_directly() {
        let path = db_path_from_env("/tmp/custom/my.db");
        assert!(path.to_string_lossy().ends_with("my.db"));
    }

    #[test]
    fn env_override_directory_gets_default_file() {
        let path = db_path_from_env("/tmp/custom-dir");
        assert!(path.to_string_lossy().ends_with("prism.db"));
    }

    #[test]
    fn default_path_lands_under_prism_dir() {
        let path = default_db_path();
        let text = path.to_string_lossy().to_string();
        assert!(text.contains("Prism"), "path: {text}");
        assert!(text.ends_with("prism.db"));
    }
}
