//! 统一 ID 模型。
//!
//! 禁止各模块随意使用裸字符串作为实体 ID；所有 ID 都是新类型（newtype），
//! 可序列化（JSON 中为透明字符串）、可比较、不依赖 UI 与数据库。
//!
//! `new()` 生成 UUIDv4，适用于运行期身份（TaskId 等）；语义上需要跨运行稳定的 ID
//! （如 M2 的 SymbolId）由各自里程碑用 [`define_id`] 生成的 `from_string`
//! 以确定性方式派生，不用随机 UUID。

use std::fmt;

macro_rules! define_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            /// 生成随机 ID（UUIDv4）。仅用于运行期身份，不用于需要跨运行稳定的语义身份。
            pub fn new() -> Self {
                Self(uuid::Uuid::new_v4().to_string())
            }

            /// 从既有字符串构造（确定性派生 ID 的入口）。
            pub fn from_string(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

define_id!(
    /// 项目 ID。跨运行的稳定项目身份由 M11（Snapshot）正式定义；M0 为运行期 UUID。
    ProjectId
);
define_id!(
    /// 项目快照 ID（M11 启用）。
    SnapshotId
);
define_id!(
    /// 文件 ID（M1 启用）。
    FileId
);
define_id!(
    /// 符号 ID（M2 启用；生成策略必须确定性派生）。
    SymbolId
);
define_id!(
    /// 模块 ID（M3 启用）。
    ModuleId
);
define_id!(
    /// Finding ID（M7 启用；必须稳定可复现，禁止随机 UUID）。
    FindingId
);
define_id!(
    /// 一次检查执行实例的 ID（M8 启用；执行实例可用随机 UUID）。
    CheckRunId
);
define_id!(
    /// Context Pack ID（M10 启用）。
    ContextPackId
);
define_id!(
    /// 任务 ID（运行期身份）。
    TaskId
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_generates_unique_ids() {
        let a = TaskId::new();
        let b = TaskId::new();
        assert_ne!(a, b);
        assert!(!a.as_str().is_empty());
    }

    #[test]
    fn from_string_is_deterministic() {
        let a = ProjectId::from_string("proj:fixture");
        let b = ProjectId::from_string("proj:fixture");
        assert_eq!(a, b);
        assert_eq!(a.to_string(), "proj:fixture");
    }

    #[test]
    fn serializes_as_plain_string() {
        let id = SymbolId::from_string("sym:abc");
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"sym:abc\"");
        let back: SymbolId = serde_json::from_str(&json).unwrap();
        assert_eq!(back, id);
    }

    #[test]
    fn ordering_is_stable() {
        let mut ids = vec![TaskId::from_string("b"), TaskId::from_string("a")];
        ids.sort();
        assert_eq!(ids[0].as_str(), "a");
    }
}
