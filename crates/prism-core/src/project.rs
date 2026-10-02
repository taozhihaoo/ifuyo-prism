//! Project 基础对象。
//!
//! M0 只保留最小身份字段；语言、符号、依赖、Git、健康、环境等
//! 全部由后续里程碑以独立模型扩展，不塞进本结构。

use crate::error::PrismResult;
use crate::ids::ProjectId;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Project {
    pub id: ProjectId,
    pub name: String,
    /// 项目的绝对根路径（构造时已 canonicalize）。
    pub root_path: PathBuf,
}

impl Project {
    /// 从磁盘上的一个根目录构造 Project。
    ///
    /// 目录必须存在。名字取目录名；root_path 规范化为绝对路径，
    /// 后续所有文件路径都以“project root + 相对路径”表达（M1 起生效）。
    pub fn from_root(root: impl AsRef<Path>) -> PrismResult<Self> {
        let canonical = root.as_ref().canonicalize()?;
        if !canonical.is_dir() {
            return Err(crate::error::PrismError::invalid_input(format!(
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
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn constructs_from_existing_directory() {
        let temp = std::env::temp_dir().join("prism-core-project-test");
        fs::create_dir_all(&temp).unwrap();
        let project = Project::from_root(&temp).unwrap();
        assert_eq!(project.name, temp.file_name().unwrap().to_string_lossy());
        assert!(project.root_path.is_absolute());
        fs::remove_dir_all(&temp).ok();
    }

    #[test]
    fn rejects_missing_directory() {
        let missing = std::env::temp_dir().join("prism-core-project-missing-xyz");
        let err = Project::from_root(&missing).unwrap_err();
        assert_eq!(err.code(), crate::error::PrismErrorCode::NotFound);
    }

    #[test]
    fn serializes_with_snake_case_fields() {
        let json = serde_json::json!({
            "id": "proj-1",
            "name": "demo",
            "root_path": "/tmp/demo"
        });
        let project: Project = serde_json::from_value(json).unwrap();
        assert_eq!(project.name, "demo");
    }
}
