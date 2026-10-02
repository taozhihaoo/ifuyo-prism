//! Project：被打开的项目身份。
//!
//! 从 M0 的 prism-core 迁入并增加 `discovered_at`。
//! 只保留身份字段；语言/符号/依赖/Git/健康等由后续里程碑扩展。

use prism_core::error::{PrismError, PrismResult};
use prism_core::ids::ProjectId;
use prism_core::timestamp::Timestamp;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Project {
    pub id: ProjectId,
    pub name: String,
    /// 项目的绝对根路径（构造时已 canonicalize）。仅用于执行文件系统操作；
    /// 文件身份一律使用 root + relative_path。
    pub root_path: PathBuf,
    /// 项目被发现的时间。
    pub discovered_at: Timestamp,
}

impl Project {
    /// 从磁盘上的一个根目录构造 Project（目录必须存在）。
    pub fn from_root(root: impl AsRef<Path>) -> PrismResult<Self> {
        let canonical = root.as_ref().canonicalize()?;
        if !canonical.is_dir() {
            return Err(PrismError::invalid_input(format!(
                "project root is not a directory: {}",
                canonical.display()
            )));
        }
        let name = canonical.file_name().map_or_else(
            || "unknown".to_string(),
            |n| n.to_string_lossy().to_string(),
        );
        Ok(Project {
            // 运行期身份；跨运行的稳定项目身份由 M11 正式定义。
            id: ProjectId::new(),
            name,
            root_path: canonical,
            discovered_at: Timestamp::now_utc(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructs_from_existing_directory_with_discovered_at() {
        let temp = std::env::temp_dir().join("prism-project-test");
        std::fs::create_dir_all(&temp).unwrap();
        let project = Project::from_root(&temp).unwrap();
        assert_eq!(project.name, temp.file_name().unwrap().to_string_lossy());
        assert!(project.root_path.is_absolute());
        assert!(project.discovered_at.as_rfc3339().ends_with('Z'));
        std::fs::remove_dir_all(&temp).ok();
    }

    #[test]
    fn rejects_missing_directory() {
        let missing = std::env::temp_dir().join("prism-project-missing-xyz");
        let err = Project::from_root(&missing).unwrap_err();
        assert_eq!(err.code(), prism_core::PrismErrorCode::NotFound);
    }

    #[test]
    fn serializes_with_snake_case_fields() {
        let json = serde_json::json!({
            "id": "proj-1",
            "name": "demo",
            "root_path": "/tmp/demo",
            "discovered_at": "2026-10-03T00:00:00Z"
        });
        let project: Project = serde_json::from_value(json).unwrap();
        assert_eq!(project.name, "demo");
    }
}
