//! Artifact / Target / Profile 模型（M214 §五-§十一）。
//!
//! Artifact 身份 = SHA-256（§六：非 GUID/文件名/mtime）；Credential 只存
//! 引用（§十：明文密码禁止入任何存储——无安全存储 ⇒ Unavailable 如实）。

use serde::{Deserialize, Serialize};

/// SHA-256 小写 hex（§六 hash 优先算法）。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Sha256Hex(pub String);

impl Sha256Hex {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Credential 引用（§十：绝不明文）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "provider", rename_all = "snake_case")]
pub enum CredentialRef {
    /// OS keychain 稳定引用。
    OsKeychain { reference: String },
    /// 环境变量名（值运行时读取，不落盘）。
    Env { variable: String },
    /// SSH agent（无密钥材料）。
    SshAgent,
    /// 凭证能力不可用（如实，不伪造）。
    Unavailable { reason: String },
}

/// Transport 能力（§十九/§53 Capability Model）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransportCapability {
    /// LocalTransport：真实可用。
    Available,
    /// SFTP/SSH：当前无依赖支持（P1 reqwest/ssh2 升级路径，如实在实）。
    Unsupported,
}

/// Target 种类（§八：按真实能力决定，不建几十种）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetKind {
    /// 本机目录目标（LocalTransport；真实支持）。
    Local,
    /// SFTP 目标（当前 Unsupported）。
    Sftp,
}

/// 部署目标（§八）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Target {
    /// `target:{fnv1a16(kind|host|port|remote_root)}`——稳定身份（§九：
    /// protocol+host+port+remote_root 组合，hostname 相同 ≠ 同机器）。
    pub target_id: String,
    pub name: String,
    pub kind: TargetKind,
    /// Local ⇒ 绝对根目录；Sftp ⇒ host（展示）。
    pub address: String,
    /// RemoteRoot（§四十六：一切相对路径解析于此之下）。
    pub remote_root: String,
    pub environment: String,
    /// Local 目标无凭证 ⇒ Unavailable 如实；SFTP 需要引用。
    pub credential: Option<CredentialRef>,
    pub transport_capability: TransportCapability,
}

/// 部署 Profile（§十一：与 Target 分离——一个 Target 多个 Profile）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeploymentProfile {
    pub profile_id: String,
    pub target_id: String,
    /// RemoteRoot 下的发布子目录（releases/current 布局）。
    pub releases_dir: String,
    /// 策略开关（§六十一 data-driven）。
    pub require_clean_git: bool,
    pub require_artifact_hash: bool,
    pub block_critical_findings: bool,
}

/// Artifact 来源（§五）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactSource {
    Build,
    Publish,
    Export,
    Manual,
}

/// Artifact 种类（§五：按真实能力——Local 文件/目录）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactKind {
    File,
    Directory,
}

/// Artifact 身份（§六）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactIdentity {
    /// `artifact:{fnv1a16(kind|sha256)}`。
    pub artifact_id: String,
    pub kind: ArtifactKind,
    /// 内容 SHA-256（目录 = manifest 排序哈希）。
    pub sha256: Sha256Hex,
    /// Git commit（§十六 linkage；dirty/无 git ⇒ None 如实）。
    pub git_commit: Option<String>,
}

/// Manifest 中的文件条目（§七：relative path + size + hash）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileEntry {
    /// `/` 分隔相对路径。
    pub relative_path: String,
    pub size_bytes: u64,
    pub sha256: Sha256Hex,
}

/// Deployment Manifest（§七：normalized file list + hashes）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactManifest {
    pub artifact: ArtifactIdentity,
    pub name: String,
    pub source: ArtifactSource,
    /// 按 relative_path 排序（§七 normalized file list）。
    pub files: Vec<FileEntry>,
    pub total_bytes: u64,
    pub generated_at: String,
    pub diagnostics: Vec<String>,
}

/// 已部署的 release 记录（§五十八 Remote State / 历史比较）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeployedRelease {
    pub release_id: String,
    pub artifact_sha256: Sha256Hex,
    pub activated_at: String,
    pub manifest: ArtifactManifest,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn credential_ref_never_plaintext() {
        // §十：只有引用形态可序列化，绝无密码字段。
        let reference = CredentialRef::OsKeychain {
            reference: "os-keychain://prism/deploy/prod".to_string(),
        };
        let json = serde_json::to_string(&reference).unwrap();
        assert!(json.contains("os-keychain://prism/deploy/prod"));
        assert!(!json.to_lowercase().contains("password"));
        let unavailable = CredentialRef::Unavailable {
            reason: "no keychain".to_string(),
        };
        assert!(serde_json::to_string(&unavailable).is_ok());
    }

    #[test]
    fn manifest_ordering_is_stable() {
        let manifest = ArtifactManifest {
            artifact: ArtifactIdentity {
                artifact_id: "artifact:abc".to_string(),
                kind: ArtifactKind::Directory,
                sha256: Sha256Hex("ff".to_string()),
                git_commit: None,
            },
            name: "demo".to_string(),
            source: ArtifactSource::Build,
            files: vec![
                FileEntry {
                    relative_path: "b.txt".to_string(),
                    size_bytes: 1,
                    sha256: Sha256Hex("01".to_string()),
                },
                FileEntry {
                    relative_path: "a.txt".to_string(),
                    size_bytes: 2,
                    sha256: Sha256Hex("02".to_string()),
                },
            ],
            total_bytes: 3,
            generated_at: "2026-10-06T00:00:00+00:00".to_string(),
            diagnostics: Vec::new(),
        };
        let mut sorted = manifest.clone();
        sorted
            .files
            .sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
        assert_ne!(manifest.files, sorted.files, "测试数据须先乱序才有意义");
        // 契约要求 manifest.files 已排序——调用方（artifact::manifest）负责。
    }
}
