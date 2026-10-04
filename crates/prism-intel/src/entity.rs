//! 统一实体与位置模型（2.0-M21 §八/§十）。

use crate::identity::{EntityId, EntityKind};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// 实体位置（M21 §十）：不强迫所有语言都有行号；
/// res:// 等引擎路径由 adapter 映射为 `External`/`Virtual`，不进 Core 语义。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EntityLocation {
    /// 无位置信息（诚实缺省，不伪造）。
    Unknown,
    /// 仅知文件（项目相对路径，`/` 分隔）。
    FileOnly { path: String },
    /// 已知精确范围。
    Range {
        path: String,
        start_line: u32,
        start_column: u32,
        end_line: u32,
        end_column: u32,
    },
    /// 生成代码（低信任来源显式化）。
    Generated { path: String },
    /// 项目外引用（包/框架/系统库；URI 或 specifier 原样保留）。
    External { reference: String },
    /// 虚拟实体（无文件系统位置；key 由 producer 定义且稳定）。
    Virtual { key: String },
}

/// 实体范围（M21 §六十）：internal / external / unknown。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityScope {
    Internal,
    External,
    Unknown,
}

/// 统一实体（M21 §八）：Core 不解释 kind 的领域语义，只承载身份与描述。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entity {
    pub id: EntityId,
    pub kind: EntityKind,
    /// 展示名（非身份；同名实体允许并存）。
    pub display_name: String,
    pub location: EntityLocation,
    pub scope: EntityScope,
    /// adapter/producer 特有附加事实（§53 收编判据之外的出口）。
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub metadata: BTreeMap<String, String>,
}

impl Entity {
    pub fn new(
        id: EntityId,
        kind: EntityKind,
        display_name: impl Into<String>,
        location: EntityLocation,
        scope: EntityScope,
    ) -> Self {
        Entity {
            id,
            kind,
            display_name: display_name.into(),
            location,
            scope,
            metadata: BTreeMap::new(),
        }
    }
}
