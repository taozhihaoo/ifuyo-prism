//! # Prism Developer Deploy（2.0-M214）
//!
//! 受控部署系统（**不是**远程 Shell）：`Artifact → Target → Preflight →
//! Plan → Transfer → Apply → Verify → Result → History → Diff`。
//!
//! 边界（§一/§二/§十/§十九/§四十五/§119 最终原则）：
//! - **Credential Reference 边界**（§十）：数据库/manifest 只存引用
//!   （`os-keychain://…` / `env://…`），绝不存明文；无安全存储 ⇒
//!   Credential Capability = Unavailable（不偷偷发明明文库）。
//! - **受控 Remote Command**（§二十）：只有结构化动作
//!   （CreateDirectory/Activate/VerifyFile…），零任意 shell。SSH/SFTP
//!   transport 当前 = Unsupported（诚实能力边界，Local 是真实 transport
//!   §63）；无 host-key 自动信任。
//! - **Atomic**（§二十二）：Local apply = release 目录 + 原子 rename
//!   current；做不到时如实报告 Atomic=False。
//! - **Unknown ≠ False / Partial ≠ Success**（§十四/§五十七）：磁盘容量
//!   不可观测 ⇒ UNKNOWN；部分文件失败 ⇒ PartiallyApplied + 逐文件结果。
//! - **History 一等**（§三十/§四十一）：每次部署（含 Rollback）生成新
//!   DeploymentRecord，不删除历史。
//! - **Path Safety**（§四十六）：一切远端相对路径解析于 RemoteRoot 之下，
//!   `..`/绝对路径/符号链接逃逸拒绝。

pub mod artifact;
pub mod model;
pub mod path_safety;
pub mod transport;

pub use model::{
    ArtifactIdentity, ArtifactKind, ArtifactManifest, ArtifactSource, CredentialRef,
    DeploymentProfile, FileEntry, Sha256Hex, Target, TargetKind, TransportCapability,
};
pub use path_safety::{resolve_under_root, PathViolation};
pub use transport::{LocalTransport, Transport};

/// M214 输出契约版本。
pub const DEPLOY_SCHEMA_VERSION: u32 = 1;
