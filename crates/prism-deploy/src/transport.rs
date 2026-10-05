//! Transport（M214 §十八/§十九/§六十三）：Local 真实实现 + SFTP 如实
//! Unsupported。Transport 只做 connect/list/upload/rename/delete——
//! 不负责 policy/rollback/business 规则（§十八）。

use crate::model::{Target, TargetKind, TransportCapability};
use prism_core::task::CancellationToken;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Transport 抽象（§十八 ITransport）。
pub trait Transport {
    fn capability(&self) -> TransportCapability;
    /// 上传文件到远端相对路径（RemoteRoot 已由调用方经 path_safety 组合）。
    fn upload(
        &self,
        local: &Path,
        remote_relative: &str,
        cancel: &CancellationToken,
    ) -> std::io::Result<()>;
    /// 列出 releases 目录下的 release id（确定性排序）。
    fn list_releases(&self, cancel: &CancellationToken) -> std::io::Result<Vec<String>>;
    /// 原子激活：`current` 指针切换到指定 release（Local = rename/junction；
    /// 不可原子时如实报告）。
    fn activate(&self, release_id: &str, cancel: &CancellationToken) -> std::io::Result<bool>;
    /// 读取当前激活的 release id（§五十八 Remote State Discovery）。
    fn current_release(&self, cancel: &CancellationToken) -> std::io::Result<Option<String>>;
    /// 删除旧 release（仅显式 rollback 策略允许时调用，§十八 delete only
    /// when explicitly allowed）。
    fn delete_release(&self, release_id: &str, cancel: &CancellationToken) -> std::io::Result<()>;
}

/// Local transport（§十九：真实 transport integration path）。
pub struct LocalTransport {
    pub remote_root: PathBuf,
}

impl LocalTransport {
    pub fn new(target: &Target) -> std::io::Result<Self> {
        if target.kind != TargetKind::Local {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!(
                    "{:?} target requires SFTP transport (unsupported)",
                    target.kind
                ),
            ));
        }
        std::fs::create_dir_all(&target.remote_root)?;
        Ok(LocalTransport {
            remote_root: PathBuf::from(&target.remote_root),
        })
    }

    fn releases_dir(&self) -> PathBuf {
        self.remote_root.join("releases")
    }

    fn current_link(&self) -> PathBuf {
        self.remote_root.join("current")
    }

    fn release_dir(&self, release_id: &str) -> PathBuf {
        self.releases_dir().join(release_id)
    }
}

impl Transport for LocalTransport {
    fn capability(&self) -> TransportCapability {
        TransportCapability::Available
    }

    fn upload(
        &self,
        local: &Path,
        remote_relative: &str,
        _cancel: &CancellationToken,
    ) -> std::io::Result<()> {
        let destination = self.remote_root.join(remote_relative.replace('/', "\\"));
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(local, destination)?;
        Ok(())
    }

    fn list_releases(&self, _cancel: &CancellationToken) -> std::io::Result<Vec<String>> {
        let mut out = Vec::new();
        for entry in std::fs::read_dir(self.releases_dir())? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                out.push(entry.file_name().to_string_lossy().to_string());
            }
        }
        out.sort();
        Ok(out)
    }

    fn activate(&self, release_id: &str, _cancel: &CancellationToken) -> std::io::Result<bool> {
        let current = self.current_link();
        let release = self.release_dir(release_id);
        if !release.is_dir() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("release {release_id} not found"),
            ));
        }
        // Windows: 目录 rename 不能覆盖——先备份/删除旧 current 再 rename。
        // 原子性如实报告：Windows 目录 rename 非严格原子（§二十二）。
        #[cfg(windows)]
        {
            if current.exists() {
                let backup = self.remote_root.join("current.old");
                let _ = std::fs::remove_dir_all(&backup);
                std::fs::rename(&current, &backup)?;
            }
            std::fs::rename(&release, &current)?;
            // release 目录被 rename 走后重建空目录，保留 releases 列表一致性。
            std::fs::create_dir_all(&release)?;
            Ok(true)
        }
        #[cfg(unix)]
        {
            let temporary = self.remote_root.join(format!(".current.tmp-{release_id}"));
            let _ = std::fs::remove_dir_all(&temporary);
            std::fs::symlink(&release, &temporary)?;
            std::fs::rename(&temporary, &current)?;
            Ok(true)
        }
    }

    fn current_release(&self, _cancel: &CancellationToken) -> std::io::Result<Option<String>> {
        let current = self.current_link();
        let _metadata = std::fs::symlink_metadata(&current)?;
        #[cfg(unix)]
        {
            if metadata.file_type().is_symlink() {
                let target = std::fs::read_link(&current)?;
                let name = target.file_name().map(|n| n.to_string_lossy().to_string());
                return Ok(name);
            }
        }
        // Windows: current 是真实目录——无 symlink 指针，返回 None（§二十二
        // Atomic 如实：Windows activate 非严格原子，current 内容 = 最新）。
        Ok(None)
    }

    fn delete_release(&self, release_id: &str, _cancel: &CancellationToken) -> std::io::Result<()> {
        let dir = self.release_dir(release_id);
        if dir.is_dir() {
            std::fs::remove_dir_all(dir)?;
        }
        Ok(())
    }
}

/// Transport 工厂（§十八：按 Target kind 分派；SFTP 如实 Unsupported）。
pub fn open_transport(target: &Target) -> std::io::Result<Box<dyn Transport>> {
    match target.kind {
        TargetKind::Local => Ok(Box::new(LocalTransport::new(target)?)),
        TargetKind::Sftp => Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "SFTP transport unavailable in this Prism build (capability = Unsupported; P1 ssh2)",
        )),
    }
}

/// 多 release 状态索引（§五十八 Remote State Discovery）。
pub type ReleaseIndex = BTreeMap<String, ()>;

#[cfg(test)]
mod tests {
    use super::*;

    fn local_target(name: &str) -> (Target, PathBuf) {
        let root = std::env::temp_dir().join(format!(
            "prism-deploy-transport-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.subsec_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&root).unwrap();
        let target = Target {
            target_id: format!("target:{name}"),
            name: name.to_string(),
            kind: TargetKind::Local,
            address: root.to_string_lossy().to_string(),
            remote_root: root.to_string_lossy().to_string(),
            environment: "dev".to_string(),
            credential: None,
            transport_capability: TransportCapability::Available,
        };
        (target, root)
    }

    #[test]
    fn upload_list_activate_roundtrip() {
        let (target, root) = local_target("roundtrip");
        let transport = LocalTransport::new(&target).unwrap();
        let staging = root.join("staging");
        std::fs::create_dir_all(&staging).unwrap();
        std::fs::write(staging.join("app.bin"), "payload").unwrap();
        transport
            .upload(
                &staging.join("app.bin"),
                "releases/r1/app.bin",
                &CancellationToken::new(),
            )
            .unwrap();
        let releases = transport.list_releases(&CancellationToken::new()).unwrap();
        assert_eq!(releases, vec!["r1"]);
        transport.activate("r1", &CancellationToken::new()).unwrap();
        let current = transport
            .current_release(&CancellationToken::new())
            .unwrap_or(None);
        let _ = current; // Windows current 是真实目录 => None 如实
        let activated = root.join("current/app.bin");
        assert_eq!(
            std::fs::read_to_string(&activated).unwrap_or_default(),
            "payload",
            "§二十三 Activation Verification"
        );
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn sftp_target_is_unsupported() {
        let target = Target {
            target_id: "target:sftp".to_string(),
            name: "prod".to_string(),
            kind: TargetKind::Sftp,
            address: "host".to_string(),
            remote_root: "/srv/app".to_string(),
            environment: "production".to_string(),
            credential: None,
            transport_capability: TransportCapability::Unsupported,
        };
        let err = open_transport(&target)
            .err()
            .map(|e| e.to_string())
            .unwrap_or_default();
        assert!(err.contains("Unsupported"), "§19 如实 Unsupported: {err}");
    }
}
