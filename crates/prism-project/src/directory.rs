//! 目录模型：扁平列表，不构建树数据库（GUI 可按路径关系生成树）。

use prism_core::ids::FileId;
use serde::{Deserialize, Serialize};

/// 一个被发现的目录。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirectoryInfo {
    /// 确定性 ID：`dir:{relative_path}`。
    pub id: FileId,
    /// 项目相对路径，`/` 分隔。
    pub relative_path: String,
    pub name: String,
    /// 深度：根的直接子目录为 1。
    pub depth: u32,
    pub is_hidden: bool,
}
