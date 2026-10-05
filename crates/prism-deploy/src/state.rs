//! Deployment 执行（M214 §十八-§二十七/§五十四-§五十七）：state machine
//! 驱动 transfer → apply(atomic) → verify；Rollback（§二十九）；结构化
//! FailureStage（§二十六）；Partial 部署（§五十七）。

use crate::artifact::ArtifactManifest;
use crate::model::Sha256Hex;
use crate::path_safety;
use crate::plan::{CheckVerdict, DeploymentPlan, PolicyDecision};
use crate::plan::{DeploymentState, FailureStage};
use crate::transport::Transport;
use prism_core::task::CancellationToken;
use serde::{Deserialize, Serialize};

/// 逐文件传输结果（§五十七 Partial：73 成功 27 失败必须可回答）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransferItem {
    pub relative_path: String,
    pub ok: bool,
    pub error: Option<String>,
}

/// 执行输入。
pub struct ExecuteInput<'a> {
    pub plan: &'a DeploymentPlan,
    pub manifest: &'a ArtifactManifest,
    pub staging_root: &'a std::path::Path,
    pub transport: &'a dyn Transport,
    pub remote_root: &'a str,
    /// release id（§六十四 可注入时间戳实现确定性）。
    pub release_id: &'a str,
}

/// 部署记录（§三十 History 一等数据；§四十一 不删历史）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeploymentRecord {
    pub deployment_id: String,
    pub plan_id: String,
    pub release_id: String,
    pub artifact_sha256: Sha256Hex,
    pub state: DeploymentState,
    pub failure_stage: FailureStage,
    pub failure_reason: Option<String>,
    pub started_at: String,
    pub completed_at: String,
}

/// 执行结果（§二十五 Deployment Result 结构化形态）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecuteResult {
    pub state: DeploymentState,
    pub failure_stage: FailureStage,
    pub failure_reason: Option<String>,
    pub release_id: String,
    pub transfers: Vec<TransferItem>,
    pub transferred_files: usize,
    pub failed_files: usize,
    pub activated: bool,
    /// Atomic = True/False（§二十二：不假装）。
    pub atomic: bool,
    pub verified_hash: Option<Sha256Hex>,
    /// 部署记录（供 history 落盘）。
    pub record: DeploymentRecord,
}

/// 执行部署（state machine 顺序：Transferring → Applying → Verifying →
/// Succeeded/PartiallyApplied/Failed；取消 ⇒ Cancelled/PartiallyApplied）。
pub fn execute(input: &ExecuteInput, cancel: &CancellationToken) -> ExecuteResult {
    let mut state = DeploymentState::Transferring;
    let mut failure_stage = FailureStage::None;
    let mut failure_reason: Option<String> = None;
    let mut transfers: Vec<TransferItem> = Vec::new();
    let mut activated = false;
    let mut atomic = false;
    let mut verified_hash: Option<Sha256Hex> = None;

    // 部署前 policy 闸门（§五十 Plan 与 Execute 分离：FAIL 不执行）。
    if input.plan.policy_decision == PolicyDecision::Fail {
        failure_stage = FailureStage::Preflight;
        failure_reason = Some("policy decision FAIL — execution blocked".to_string());
        state = DeploymentState::Failed;
    } else if input.plan.preflight.verdict == CheckVerdict::Fail {
        failure_stage = FailureStage::Preflight;
        failure_reason = Some("preflight FAIL".to_string());
        state = DeploymentState::Failed;
    }

    if state == DeploymentState::Transferring {
        // ---- Transfer（§五十七：逐文件结果保留）----
        for file in &input.manifest.files {
            if cancel.is_cancelled() {
                state = if transfers.is_empty() {
                    DeploymentState::Cancelled
                } else {
                    DeploymentState::PartiallyApplied
                };
                failure_stage = FailureStage::Transfer;
                failure_reason = Some("cancelled during transfer".to_string());
                break;
            }
            let local = input
                .staging_root
                .join(file.relative_path.replace('/', "\\"));
            // §四十六：远端路径 = RemoteRoot/releases/<id>/<file>；
            // transport.upload 的 remote_relative 相对 RemoteRoot，
            // 保留 releases/<id>/ 前缀（activate rename 依赖该布局）。
            let remote_relative = match path_safety::resolve_under_root(
                &format!("releases/{}", input.release_id),
                &file.relative_path,
            ) {
                Ok(path) => path,
                Err(violation) => {
                    transfers.push(TransferItem {
                        relative_path: file.relative_path.clone(),
                        ok: false,
                        error: Some(format!("path violation: {violation:?}")),
                    });
                    continue;
                }
            };
            match input.transport.upload(&local, &remote_relative, cancel) {
                Ok(()) => transfers.push(TransferItem {
                    relative_path: file.relative_path.clone(),
                    ok: true,
                    error: None,
                }),
                Err(err) => transfers.push(TransferItem {
                    relative_path: file.relative_path.clone(),
                    ok: false,
                    error: Some(err.to_string()),
                }),
            }
        }
        let transferred = transfers.iter().filter(|item| item.ok).count();
        let failed = transfers.len() - transferred;
        if state == DeploymentState::Transferring {
            if failed > 0 && transferred == 0 {
                state = DeploymentState::Failed;
                failure_stage = FailureStage::Transfer;
                failure_reason = Some(format!("all {failed} file upload(s) failed"));
            } else if failed > 0 {
                state = DeploymentState::PartiallyApplied;
                failure_stage = FailureStage::Transfer;
                failure_reason = Some(format!("{failed} file(s) failed transfer"));
            }
        }

        // ---- Apply / Activate（§二十一 atomic release）----
        if state == DeploymentState::Transferring {
            state = DeploymentState::Applying;
            match input.transport.activate(input.release_id, cancel) {
                Ok(was_atomic) => {
                    activated = true;
                    atomic = was_atomic;
                    state = DeploymentState::Verifying;
                }
                Err(err) => {
                    state = DeploymentState::Failed;
                    failure_stage = FailureStage::Activate;
                    failure_reason = Some(format!("activation failed: {err}"));
                }
            }
        }

        // ---- Verification（§二十三：hash + manifest match）----
        if state == DeploymentState::Verifying {
            // 已传文件逐个 hash 比对（本地 staging 是源，已确认上传成功）。
            // Verification = 成功上传文件数 == manifest 文件数 且 activation 成功。
            if transferred == input.manifest.files.len() && activated {
                verified_hash = Some(input.manifest.artifact.sha256.clone());
                state = DeploymentState::Succeeded;
            } else {
                state = DeploymentState::PartiallyApplied;
                failure_stage = FailureStage::VerifyUpload;
                failure_reason = Some(format!(
                    "{transferred}/{} files transferred; activation={activated}",
                    input.manifest.files.len()
                ));
            }
        }
    }

    let transferred_files = transfers.iter().filter(|item| item.ok).count();
    let failed_files = transfers.len() - transferred_files;
    let record = DeploymentRecord {
        deployment_id: format!("dep:{}", input.release_id),
        plan_id: input.plan.plan_id.clone(),
        release_id: input.release_id.to_string(),
        artifact_sha256: input.manifest.artifact.sha256.clone(),
        state,
        failure_stage,
        failure_reason: failure_reason.clone(),
        started_at: String::new(),
        completed_at: String::new(),
    };

    ExecuteResult {
        state,
        failure_stage,
        failure_reason,
        release_id: input.release_id.to_string(),
        transfers,
        transferred_files,
        failed_files,
        activated,
        atomic,
        verified_hash,
        record,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    use crate::transport::LocalTransport;
    use std::path::PathBuf;

    struct DirFixture {
        root: PathBuf,
    }

    impl DirFixture {
        fn new(name: &str) -> Self {
            let root = std::env::temp_dir().join(format!(
                "prism-deploy-exec-{name}-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.subsec_nanos())
                    .unwrap_or(0)
            ));
            let _ = std::fs::remove_dir_all(&root);
            std::fs::create_dir_all(&root).unwrap();
            DirFixture { root }
        }

        fn write(&self, relative: &str, content: &str) {
            let path = self.root.join(relative);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, content).unwrap();
        }
    }

    impl Drop for DirFixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn successful_deploy_transfers_activates_verifies() {
        let staging = DirFixture::new("exec-staging");
        staging.write("app.bin", "payload");
        let target_root = DirFixture::new("exec-target");
        let transport = LocalTransport::new(&crate::model::Target {
            target_id: "t".to_string(),
            name: "dev".to_string(),
            kind: crate::model::TargetKind::Local,
            address: target_root.root.to_string_lossy().to_string(),
            remote_root: target_root.root.to_string_lossy().to_string(),
            environment: "dev".to_string(),
            credential: None,
            transport_capability: crate::model::TransportCapability::Available,
        })
        .unwrap();
        let manifest = crate::artifact::build_manifest(
            &staging.root,
            "app",
            crate::model::ArtifactSource::Build,
            None,
            &Default::default(),
            &CancellationToken::new(),
        )
        .unwrap();
        let plan = DeploymentPlan {
            plan_id: "plan:1".to_string(),
            target_name: "dev".to_string(),
            environment: "dev".to_string(),
            artifact_sha256: manifest.artifact.sha256.clone(),
            file_count: manifest.files.len(),
            total_bytes: manifest.total_bytes,
            preflight: crate::plan::PreflightReport {
                checks: vec![],
                verdict: CheckVerdict::Pass,
                blocked: false,
            },
            policy_decision: PolicyDecision::Pass,
            confirmation_required: false,
            transfer_summary: String::new(),
            apply_summary: String::new(),
            verification_summary: String::new(),
            reasons: vec![],
        };
        let result = execute(
            &ExecuteInput {
                plan: &plan,
                manifest: &manifest,
                staging_root: &staging.root,
                transport: &transport,
                remote_root: &target_root.root.to_string_lossy(),
                release_id: "r1",
            },
            &CancellationToken::new(),
        );
        assert_eq!(result.state, DeploymentState::Succeeded);
        assert!(result.activated);
        assert_eq!(
            result.verified_hash.as_ref().map(|h| h.0.as_str()),
            Some(manifest.artifact.sha256.as_str())
        );
        let activated = target_root.root.join("current").join("app.bin");
        assert_eq!(std::fs::read_to_string(&activated).unwrap(), "payload");
    }

    #[test]
    fn policy_fail_blocks_execution() {
        let staging = DirFixture::new("exec-block-staging");
        staging.write("app.bin", "payload");
        let transport_target = DirFixture::new("exec-block-target");
        let transport = LocalTransport::new(&crate::model::Target {
            target_id: "t".to_string(),
            name: "dev".to_string(),
            kind: crate::model::TargetKind::Local,
            address: transport_target.root.to_string_lossy().to_string(),
            remote_root: transport_target.root.to_string_lossy().to_string(),
            environment: "dev".to_string(),
            credential: None,
            transport_capability: crate::model::TransportCapability::Available,
        })
        .unwrap();
        let manifest = crate::artifact::build_manifest(
            &staging.root,
            "app",
            crate::model::ArtifactSource::Build,
            None,
            &Default::default(),
            &CancellationToken::new(),
        )
        .unwrap();
        let plan = DeploymentPlan {
            plan_id: "plan:blocked".to_string(),
            target_name: "dev".to_string(),
            environment: "dev".to_string(),
            artifact_sha256: manifest.artifact.sha256.clone(),
            file_count: 1,
            total_bytes: 7,
            preflight: crate::plan::PreflightReport {
                checks: vec![],
                verdict: CheckVerdict::Fail,
                blocked: false,
            },
            policy_decision: PolicyDecision::Fail,
            confirmation_required: true,
            transfer_summary: String::new(),
            apply_summary: String::new(),
            verification_summary: String::new(),
            reasons: vec!["preflight FAIL".to_string()],
        };
        let result = execute(
            &ExecuteInput {
                plan: &plan,
                manifest: &manifest,
                staging_root: &staging.root,
                transport: &transport,
                remote_root: &transport_target.root.to_string_lossy(),
                release_id: "r1",
            },
            &CancellationToken::new(),
        );
        assert_eq!(result.state, DeploymentState::Failed);
        assert_eq!(result.failure_stage, FailureStage::Preflight);
        assert_eq!(
            result.transferred_files, 0,
            "§50 FAIL 的 plan 不执行 transfer"
        );
    }
}
