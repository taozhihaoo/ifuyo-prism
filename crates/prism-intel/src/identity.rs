//! 项目身份与实体身份（2.0-M21 §7–§10）。
//!
//! 不变量（M21 §58）：
//! - 实体 ID 在 project scope 内唯一且跨运行稳定（内容寻址，非显示名）；
//! - 不同项目的同一 canonical key 产生不同 EntityId（isolation）；
//! - 禁止 display name / 行号 / 绝对路径单独充当通用身份。

use prism_core::ids::ProjectId;
use prism_project::Project;
use serde::{Deserialize, Serialize};
use std::fmt;

/// 稳定项目键：canonical root 的 fnv1a64 十六进制（16 字符）。
/// 与 `Project::from_root` 的 canonicalize 口径一致（2.0-M20 §32），
/// 跨运行稳定；`project_id`（运行期 UUID）不参与派生。
pub fn stable_project_key(canonical_root: &str) -> String {
    fnv1a_hex16(canonical_root)
}

/// 统一项目身份（M21 §7.1）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectIdentity {
    /// 运行期/存储层项目 id（如 prism-store 的 `proj:uuid`）。
    pub project_id: String,
    /// 跨运行稳定键（canonical root 哈希）——实体 ID 派生的项目 scope。
    pub stable_key: String,
    /// 当前本地 canonical 根路径。
    pub root: String,
    pub display_name: String,
    pub observed_at: String,
}

impl ProjectIdentity {
    /// 从 M1 `Project` 构造（root_path 已 canonicalize，M20 §32）。
    pub fn from_project(project: &Project) -> Self {
        let root = project.root_path.to_string_lossy().to_string();
        ProjectIdentity {
            project_id: project.id.as_str().to_string(),
            stable_key: stable_project_key(&root),
            root,
            display_name: project.name.clone(),
            observed_at: prism_core::Timestamp::now_utc().to_string(),
        }
    }

    /// 测试/工具用显式构造。
    pub fn from_root(project_id: ProjectId, canonical_root: &str) -> Self {
        ProjectIdentity {
            project_id: project_id.as_str().to_string(),
            stable_key: stable_project_key(canonical_root),
            root: canonical_root.to_string(),
            display_name: std::path::Path::new(canonical_root)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| canonical_root.to_string()),
            observed_at: prism_core::Timestamp::now_utc().to_string(),
        }
    }
}

/// 实体种类（M21 §八）。
///
/// 开放注册：Core 只内置跨 adapter 的通用种类；引擎特有种类
/// （Godot scene/node 等）由 adapter 经 [`EntityKind::other`] 贡献，
/// 禁止硬编码进 Core（M21 §51）。
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct EntityKind(String);

impl EntityKind {
    pub fn project() -> Self {
        EntityKind("project".into())
    }
    pub fn directory() -> Self {
        EntityKind("directory".into())
    }
    pub fn file() -> Self {
        EntityKind("file".into())
    }
    pub fn module() -> Self {
        EntityKind("module".into())
    }
    pub fn package() -> Self {
        EntityKind("package".into())
    }
    pub fn namespace() -> Self {
        EntityKind("namespace".into())
    }
    pub fn symbol() -> Self {
        EntityKind("symbol".into())
    }
    pub fn commit() -> Self {
        EntityKind("commit".into())
    }
    pub fn test() -> Self {
        EntityKind("test".into())
    }

    /// adapter 贡献的扩展种类（如 "godot.scene"）；点分命名约定。
    pub fn other(name: impl Into<String>) -> Self {
        EntityKind(name.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for EntityKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// 实体身份：`{kind}:{hash16}`，hash 覆盖 `stable_key|canonical_key`。
/// 同一输入恒等；跨项目/跨 canonical key 不同（§38 isolation）。
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct EntityId(String);

impl EntityId {
    pub fn new(project_stable_key: &str, kind: &EntityKind, canonical_key: &str) -> Self {
        let hash = fnv1a_hex16(&format!("{project_stable_key}|{kind}|{canonical_key}"));
        EntityId(format!("{kind}:{hash}"))
    }

    /// 需要跨项目去重的场合显式携带项目键（默认构造已含）。
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for EntityId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// fnv1a64 → 16 字符小写十六进制。
pub(crate) fn fnv1a_hex16(input: &str) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in input.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_key_is_deterministic_and_16_hex() {
        let a = stable_project_key("C:/proj/a");
        let b = stable_project_key("C:/proj/a");
        assert_eq!(a, b);
        assert_eq!(a.len(), 16);
        assert_ne!(a, stable_project_key("C:/proj/b"));
    }

    #[test]
    fn entity_id_is_deterministic_per_project_scope() {
        let ka = stable_project_key("C:/proj/a");
        let kb = stable_project_key("C:/proj/b");
        let id1 = EntityId::new(&ka, &EntityKind::module(), "src/main.py");
        let id2 = EntityId::new(&ka, &EntityKind::module(), "src/main.py");
        assert_eq!(id1, id2, "same identity must be equal");
        let other_project = EntityId::new(&kb, &EntityKind::module(), "src/main.py");
        assert_ne!(id1, other_project, "different project must not collide");
        let other_kind = EntityId::new(&ka, &EntityKind::file(), "src/main.py");
        assert_ne!(id1, other_kind, "same name different kind stays distinct");
    }

    #[test]
    fn entity_id_does_not_embed_display_name() {
        // 同一 canonical key、不同显示名 ⇒ 同一实体（§43.1）。
        let k = stable_project_key("C:/proj/a");
        let id = EntityId::new(&k, &EntityKind::module(), "mod:abc");
        assert_eq!(
            id.as_str(),
            EntityId::new(&k, &EntityKind::module(), "mod:abc").as_str()
        );
        assert!(id.as_str().starts_with("module:"));
    }
}
