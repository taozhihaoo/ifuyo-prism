//! Deployment History / Rollback / Diff（M214 §二十九/§三十/§三十一）。
//!
//! History 一等（§四十一）：每次部署（含 Rollback）生成新 Record，
//! 不删除历史；v1 存 JSON 文件（SQLite DEFER P1，沿 M210-M213 先例）。

use crate::plan::DeploymentState;
use crate::plan::FailureStage;
use crate::state::DeploymentRecord;
use crate::transport::Transport;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// History 存储（JSON 文件；确定性排序：插入序，§四十三）。
#[derive(Debug, Default)]
pub struct DeploymentStore {
    path: PathBuf,
    records: Vec<DeploymentRecord>,
}

impl DeploymentStore {
    /// 从 JSON 文件加载（不存在 ⇒ 空 store，§67 offline）。
    pub fn load(path: &Path) -> std::io::Result<Self> {
        let records = match std::fs::read_to_string(path) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_default(),
            Err(_) => Vec::new(),
        };
        Ok(DeploymentStore {
            path: path.to_path_buf(),
            records,
        })
    }

    /// 追加记录（§四十一：不删除历史）。
    pub fn append(&mut self, record: DeploymentRecord) {
        self.records.push(record);
    }

    /// 落盘（JSON pretty，确定性序列化 §四十三）。
    pub fn save(&self) -> std::io::Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let text = serde_json::to_string_pretty(&self.records)?;
        std::fs::write(&self.path, text)
    }

    pub fn records(&self) -> &[DeploymentRecord] {
        &self.records
    }

    /// 最近一次 Succeeded 部署（§二十九 Rollback 目标）。
    pub fn last_successful(&self) -> Option<&DeploymentRecord> {
        self.records
            .iter()
            .rev()
            .find(|record| record.state == DeploymentState::Succeeded)
    }

    /// Deployment Diff（§三十一）：A vs B。
    pub fn diff(&self, a_id: &str, b_id: &str) -> Option<DeploymentDiff> {
        let a = self.records.iter().find(|r| r.deployment_id == a_id)?;
        let b = self.records.iter().find(|r| r.deployment_id == b_id)?;
        Some(DeploymentDiff {
            deployment_a: a_id.to_string(),
            deployment_b: b_id.to_string(),
            artifact_changed: a.artifact_sha256 != b.artifact_sha256,
            release_changed: a.release_id != b.release_id,
            state_changed: a.state != b.state,
        })
    }
}

/// 部署差异（§三十一）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeploymentDiff {
    pub deployment_a: String,
    pub deployment_b: String,
    pub artifact_changed: bool,
    pub release_changed: bool,
    pub state_changed: bool,
}

/// Rollback（§二十九）：回退到上一个成功部署的 release。
/// 生成新 DeploymentRecord（§四十一 不删历史）；失败 ⇒ RollbackFailed。
pub fn rollback(
    transport: &mut dyn crate::transport::Transport,
    previous_release: &str,
    reason: &str,
    cancel: &prism_core::task::CancellationToken,
) -> std::io::Result<DeploymentRecord> {
    transport.activate(previous_release, cancel)?;
    Ok(DeploymentRecord {
        deployment_id: format!("dep:rollback-{}", previous_release),
        plan_id: format!("rollback:{reason}"),
        release_id: previous_release.to_string(),
        artifact_sha256: crate::model::Sha256Hex(String::new()),
        state: DeploymentState::RolledBack,
        failure_stage: FailureStage::None,
        failure_reason: None,
        started_at: String::new(),
        completed_at: prism_core::Timestamp::now_utc().to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Sha256Hex;

    fn record(id: &str, release: &str, sha: &str, state: DeploymentState) -> DeploymentRecord {
        DeploymentRecord {
            deployment_id: id.to_string(),
            plan_id: format!("plan:{id}"),
            release_id: release.to_string(),
            artifact_sha256: Sha256Hex(sha.to_string()),
            state,
            failure_stage: FailureStage::None,
            failure_reason: None,
            started_at: String::new(),
            completed_at: String::new(),
        }
    }

    #[test]
    fn history_persists_across_reopen() {
        let path = std::env::temp_dir().join(format!(
            "prism-deploy-history-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.subsec_nanos())
                .unwrap_or(0)
        ));
        {
            let mut store = DeploymentStore::load(&path).unwrap();
            store.append(record("dep:1", "r1", "aa", DeploymentState::Succeeded));
            store.append(record("dep:2", "r2", "bb", DeploymentState::Failed));
            store.save().unwrap();
        }
        // 重启后历史保留（§68 restart persistence）。
        let store = DeploymentStore::load(&path).unwrap();
        assert_eq!(store.records().len(), 2);
        assert_eq!(store.records()[0].deployment_id, "dep:1");
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn last_successful_finds_latest_success() {
        let mut store = DeploymentStore::default();
        store.append(record("dep:1", "r1", "aa", DeploymentState::Succeeded));
        store.append(record("dep:2", "r2", "bb", DeploymentState::Failed));
        assert_eq!(
            store.last_successful().map(|r| r.deployment_id.as_str()),
            Some("dep:1")
        );
    }

    #[test]
    fn diff_reports_artifact_change() {
        let mut store = DeploymentStore::default();
        store.append(record("dep:1", "r1", "aa", DeploymentState::Succeeded));
        store.append(record("dep:2", "r2", "bb", DeploymentState::Succeeded));
        let diff = store.diff("dep:1", "dep:2").unwrap();
        assert!(diff.artifact_changed);
        assert!(diff.release_changed);
        assert!(!diff.state_changed);
    }
}
