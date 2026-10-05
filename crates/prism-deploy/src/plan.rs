//! Deployment 状态机 / Preflight / Policy / Plan（M214 §十二-§十五/§五十四/
//! §六十一）。
//!
//! 状态机（§五十四）：`Planned → Preflight → AwaitingConfirmation →
//! Transferring → Applying → Verifying → Succeeded`（异常路径见枚举）。
//! Policy data-driven（§六十一）：声明规则/执行规则/结论 PASS/WARN/FAIL/
//! UNKNOWN 分离；UNKNOWN 默认不阻断除非 policy 明确（§十四）。

use crate::artifact::ArtifactManifest;
use crate::model::{DeploymentProfile, Sha256Hex, Target};
use serde::{Deserialize, Serialize};

/// 部署状态机（§二十五/§五十四）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeploymentState {
    Planned,
    Preflight,
    AwaitingConfirmation,
    Transferring,
    Applying,
    Verifying,
    Succeeded,
    SucceededWithWarnings,
    Failed,
    Cancelled,
    PartiallyApplied,
    RolledBack,
    RollbackFailed,
    Unknown,
}

/// 失败阶段（§二十六：结构化，不统一成 deployment failed）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureStage {
    None,
    Validation,
    Preflight,
    Credential,
    Connect,
    Transfer,
    VerifyUpload,
    Apply,
    Activate,
    ApplicationStart,
    HealthCheck,
    Rollback,
    Unknown,
}

/// Preflight 检查结论（§十三：PASS/WARN/FAIL/UNKNOWN，不伪装）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckVerdict {
    Pass,
    Warn,
    Fail,
    Unknown,
}

/// 单项 Preflight 检查。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreflightCheck {
    pub name: String,
    pub verdict: CheckVerdict,
    /// 检查详情（可解释 §80：Not just a word）。
    pub detail: String,
}

/// Preflight 汇总（§十三）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreflightReport {
    pub checks: Vec<PreflightCheck>,
    /// FAIL 存在 ⇒ Blocked；UNKNOWN 计数由 policy 决定是否阻断（§十四）。
    pub verdict: CheckVerdict,
    pub blocked: bool,
}

/// Policy 规则（§六十一：声明）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyRule {
    RequireCleanGit,
    RequireArtifactHash,
    BlockCriticalSecurityFindings,
    RequireRemoteVerification,
    RequireConfirmationForUnsafe,
    BlockUnknownPreflight,
}

/// Policy 结论（§61 PASS/WARN/FAIL/UNKNOWN）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyDecision {
    Pass,
    Warn,
    Fail,
    Unknown,
}

/// Deployment Policy（§六十一：开关集合）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeploymentPolicy {
    pub require_clean_git: bool,
    pub require_artifact_hash: bool,
    pub block_critical_security_findings: bool,
    pub require_remote_verification: bool,
    pub block_unknown_preflight: bool,
}

impl Default for DeploymentPolicy {
    fn default() -> Self {
        DeploymentPolicy {
            require_clean_git: false,
            require_artifact_hash: true,
            block_critical_security_findings: true,
            require_remote_verification: true,
            block_unknown_preflight: false,
        }
    }
}

impl DeploymentPolicy {
    /// 由 Profile 派生 policy（§十一 profile.require_* 字段）。
    pub fn from_profile(profile: &DeploymentProfile) -> Self {
        DeploymentPolicy {
            require_clean_git: profile.require_clean_git,
            require_artifact_hash: profile.require_artifact_hash,
            block_critical_security_findings: profile.block_critical_findings,
            ..DeploymentPolicy::default()
        }
    }
}

/// Deployment Plan（§十二：可展示、可确认；确认前不 Apply）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeploymentPlan {
    pub plan_id: String,
    pub target_name: String,
    pub environment: String,
    pub artifact_sha256: Sha256Hex,
    pub file_count: usize,
    pub total_bytes: u64,
    /// Preflight 结论（§十三）。
    pub preflight: PreflightReport,
    /// Policy 结论（§六十一）。
    pub policy_decision: PolicyDecision,
    /// 需要 用户确认才可执行（§十二：高风险 Apply 前确认）。
    pub confirmation_required: bool,
    /// 传输/激活/验证计划摘要（§十二）。
    pub transfer_summary: String,
    pub apply_summary: String,
    pub verification_summary: String,
    /// Policy 结论原因（§80 可解释性：结论必须能展开）。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reasons: Vec<String>,
}

/// 组装 Deployment Plan（§十二）。
#[allow(clippy::too_many_arguments)]
pub fn build_plan(
    plan_id: &str,
    manifest: &ArtifactManifest,
    target: &Target,
    _profile: &DeploymentProfile,
    preflight: &PreflightReport,
    policy: &DeploymentPolicy,
    git_dirty: Option<bool>,
    critical_findings: u64,
) -> DeploymentPlan {
    // Policy 评估（§六十一：声明规则执行）。
    let mut decision = PolicyDecision::Pass;
    let mut confirmation_required = false;
    let mut reasons: Vec<String> = Vec::new();
    if preflight.verdict == CheckVerdict::Fail {
        decision = PolicyDecision::Fail;
        reasons.push("preflight FAIL".to_string());
    } else if preflight.blocked {
        decision = PolicyDecision::Fail;
        reasons.push("preflight blocked (UNKNOWN with block_unknown_preflight)".to_string());
    } else if preflight.verdict == CheckVerdict::Unknown {
        decision = if policy.block_unknown_preflight {
            PolicyDecision::Fail
        } else {
            confirmation_required = true;
            PolicyDecision::Unknown
        };
    }
    if policy.require_clean_git && git_dirty == Some(true) {
        decision = PolicyDecision::Fail;
        reasons.push("git working tree dirty (require_clean_git)".to_string());
    }
    if policy.block_critical_security_findings && critical_findings > 0 {
        decision = PolicyDecision::Fail;
        reasons.push(format!("{critical_findings} critical security finding(s)"));
    }
    if confirmation_required && decision == PolicyDecision::Unknown {
        // UNKNOWN 保留原结论（不伪装 WARN/FAIL §14），但要确认。
    }
    let _ = target;
    DeploymentPlan {
        plan_id: plan_id.to_string(),
        target_name: target.name.clone(),
        environment: target.environment.clone(),
        artifact_sha256: manifest.artifact.sha256.clone(),
        file_count: manifest.files.len(),
        total_bytes: manifest.total_bytes,
        preflight: preflight.clone(),
        policy_decision: decision,
        confirmation_required: confirmation_required || decision == PolicyDecision::Warn,
        transfer_summary: format!(
            "{} files / {:.1} KiB -> {}/releases/<release-id>",
            manifest.files.len(),
            manifest.total_bytes as f64 / 1024.0,
            target.remote_root
        ),
        apply_summary: "upload to temp release dir, verify manifest, atomic activate".to_string(),
        verification_summary: if policy.require_remote_verification {
            "manifest hash match + release activation check".to_string()
        } else {
            "verification disabled by policy".to_string()
        },
        reasons,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn preflight(verdict: CheckVerdict, blocked: bool) -> PreflightReport {
        PreflightReport {
            checks: vec![PreflightCheck {
                name: "artifact".to_string(),
                verdict,
                detail: "test".to_string(),
            }],
            verdict,
            blocked,
        }
    }

    fn target() -> Target {
        Target {
            target_id: "target:dev".to_string(),
            name: "dev".to_string(),
            kind: crate::model::TargetKind::Local,
            address: ".".to_string(),
            remote_root: "/tmp/app".to_string(),
            environment: "dev".to_string(),
            credential: None,
            transport_capability: crate::model::TransportCapability::Available,
        }
    }

    fn profile(require_clean_git: bool) -> DeploymentProfile {
        DeploymentProfile {
            profile_id: "p1".to_string(),
            target_id: "target:dev".to_string(),
            releases_dir: "releases".to_string(),
            require_clean_git,
            require_artifact_hash: true,
            block_critical_findings: true,
        }
    }

    fn manifest() -> ArtifactManifest {
        ArtifactManifest {
            artifact: crate::model::ArtifactIdentity {
                artifact_id: "artifact:x".to_string(),
                kind: crate::model::ArtifactKind::Directory,
                sha256: Sha256Hex("ff".to_string()),
                git_commit: None,
            },
            name: "app".to_string(),
            source: crate::model::ArtifactSource::Build,
            files: vec![],
            total_bytes: 100,
            generated_at: String::new(),
            diagnostics: Vec::new(),
        }
    }

    #[test]
    fn preflight_fail_blocks_plan() {
        let plan = build_plan(
            "plan-1",
            &manifest(),
            &target(),
            &profile(false),
            &preflight(CheckVerdict::Fail, false),
            &DeploymentPolicy::default(),
            Some(false),
            0,
        );
        assert_eq!(plan.policy_decision, PolicyDecision::Fail);
        assert!(!plan.confirmation_required, "FAIL = 阻断，不进确认流");
    }

    #[test]
    fn dirty_git_blocks_when_policy_requires() {
        let policy = DeploymentPolicy {
            require_clean_git: true,
            ..Default::default()
        };
        let plan = build_plan(
            "plan-1",
            &manifest(),
            &target(),
            &profile(true),
            &preflight(CheckVerdict::Pass, false),
            &policy,
            Some(true),
            0,
        );
        assert_eq!(plan.policy_decision, PolicyDecision::Fail);
        // 无 require_clean_git policy 时 dirty 不阻断。
        let lenient_plan = build_plan(
            "plan-1",
            &manifest(),
            &target(),
            &profile(false),
            &preflight(CheckVerdict::Pass, false),
            &DeploymentPolicy {
                require_clean_git: false,
                ..Default::default()
            },
            Some(true),
            0,
        );
        assert_eq!(lenient_plan.policy_decision, PolicyDecision::Pass);
    }

    #[test]
    fn unknown_preflight_defaults_to_confirmation_not_block() {
        // §14：UNKNOWN 不应默认伪装 FAIL——除非 policy 要求。
        let policy = DeploymentPolicy {
            block_unknown_preflight: true,
            ..Default::default()
        };
        let plan = build_plan(
            "plan-1",
            &manifest(),
            &target(),
            &profile(false),
            &preflight(CheckVerdict::Unknown, false),
            &policy,
            Some(false),
            0,
        );
        assert_eq!(
            plan.policy_decision,
            PolicyDecision::Fail,
            "policy 要求 UNKNOWN => BLOCK"
        );
        let lenient_policy = DeploymentPolicy {
            block_unknown_preflight: false,
            ..Default::default()
        };
        let plan = build_plan(
            "plan-1",
            &manifest(),
            &target(),
            &profile(false),
            &preflight(CheckVerdict::Unknown, false),
            &lenient_policy,
            Some(false),
            0,
        );
        assert_eq!(
            plan.policy_decision,
            PolicyDecision::Unknown,
            "§14 UNKNOWN 不伪装 WARN/FAIL"
        );
        assert!(plan.confirmation_required, "UNKNOWN 需确认才可执行");
    }

    #[test]
    fn critical_findings_block() {
        let plan = build_plan(
            "plan-1",
            &manifest(),
            &target(),
            &profile(true),
            &preflight(CheckVerdict::Pass, false),
            &DeploymentPolicy::default(),
            Some(false),
            2,
        );
        assert_eq!(
            plan.policy_decision,
            PolicyDecision::Fail,
            "§15 critical security findings => BLOCK"
        );
    }
}
