//! `prism deploy`（M214 §49/§50/§76）：Plan 与 Execute 分离的 CLI 面。
//!
//! 安全（§55）：`deploy run` 必须显式 `--confirm`（§十二 确认前不
//! Apply）；`rollback` 同样显式；`plan`/`history`/`diff` 零网络动作。

use clap::Subcommand;
use prism_core::task::CancellationToken;
use prism_deploy::plan::DeploymentPolicy;
use prism_deploy::{build_plan, DeploymentProfile, Target, TargetKind, TransportCapability};
use std::path::{Path, PathBuf};

#[derive(Debug, Subcommand)]
pub enum DeployCommand {
    /// 构建 Artifact Manifest（目录/文件 → 身份+清单）
    Artifact {
        /// artifact 源目录/文件
        source: PathBuf,
        /// artifact 名称
        #[arg(long, default_value = "app")]
        name: String,
        /// Git commit（可选 linkage）
        #[arg(long)]
        git_commit: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// 生成 Deployment Plan（不执行，§50）
    Plan {
        /// artifact 源目录/文件
        source: PathBuf,
        /// Local 目标根目录
        #[arg(long)]
        target_root: PathBuf,
        /// 目标环境标签
        #[arg(long, default_value = "dev")]
        environment: String,
        /// Git commit（可选）
        #[arg(long)]
        git_commit: Option<String>,
        /// Git working tree 是否 dirty（显式传入；§61 require_clean_git）
        #[arg(long)]
        git_dirty: bool,
        /// critical security findings 数量（M28/M29 注入）
        #[arg(long, default_value_t = 0)]
        critical_findings: u64,
        #[arg(long)]
        json: bool,
    },
    /// 执行部署（Preflight → Transfer → Apply → Verify；须 --confirm）
    Run {
        /// artifact 源目录/文件
        source: PathBuf,
        /// Local 目标根目录
        #[arg(long)]
        target_root: PathBuf,
        /// 目标环境标签
        #[arg(long, default_value = "dev")]
        environment: String,
        /// Git commit（可选）
        #[arg(long)]
        git_commit: Option<String>,
        /// Git dirty 状态
        #[arg(long)]
        git_dirty: bool,
        /// critical findings 数量
        #[arg(long, default_value_t = 0)]
        critical_findings: u64,
        /// §十二：显式确认（缺省 ⇒ 只出 plan 不执行）
        #[arg(long)]
        confirm: bool,
        #[arg(long)]
        json: bool,
    },
    /// 部署历史
    History {
        /// Local 目标根目录（history.json 所在项目）
        #[arg(default_value = ".")]
        path: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// 回滚到指定 release（§29 生成新 Record；显式 --confirm）
    Rollback {
        /// Local 目标根目录
        #[arg(default_value = ".")]
        path: PathBuf,
        /// 目标 release id
        release: String,
        /// 回滚原因
        #[arg(long, default_value = "manual")]
        reason: String,
        #[arg(long)]
        confirm: bool,
    },
}

/// Deploy CLI 入口。
pub fn run(command: &DeployCommand, cancel: &CancellationToken) -> i32 {
    match command {
        DeployCommand::Artifact {
            source,
            name,
            git_commit,
            json,
        } => {
            match prism_deploy::artifact::build_manifest(
                source,
                name,
                prism_deploy::model::ArtifactSource::Build,
                git_commit.clone(),
                &Default::default(),
                cancel,
            ) {
                Ok(manifest) => {
                    if *json {
                        print_json(&manifest);
                    } else {
                        println!(
                            "artifact {} ({:?}) sha256={} files={} bytes={}",
                            manifest.name,
                            manifest.artifact.kind,
                            manifest.artifact.sha256.as_str(),
                            manifest.files.len(),
                            manifest.total_bytes
                        );
                        for file in &manifest.files {
                            println!("  {} ({} bytes)", file.relative_path, file.size_bytes);
                        }
                        for diagnostic in &manifest.diagnostics {
                            println!("diag: {diagnostic}");
                        }
                    }
                    crate::EXIT_OK
                }
                Err(err) => fail("artifact", &err),
            }
        }
        DeployCommand::Plan {
            source,
            target_root,
            environment,
            git_commit,
            git_dirty,
            critical_findings,
            json,
        } => run_plan(
            source,
            target_root,
            environment,
            git_commit.as_deref(),
            *git_dirty,
            *critical_findings,
            *json,
        ),
        DeployCommand::Run {
            source,
            target_root,
            environment,
            git_commit,
            git_dirty,
            critical_findings,
            confirm,
            json,
        } => run_deploy(
            source,
            target_root,
            environment,
            git_commit.as_deref(),
            *git_dirty,
            *critical_findings,
            *confirm,
            *json,
            cancel,
        ),
        DeployCommand::History { path, json } => {
            let history_path = path.join("deploy-history.json");
            match prism_deploy::DeploymentStore::load(&history_path) {
                Ok(store) => {
                    let records = store.records();
                    if *json {
                        print_json(&records);
                    } else if records.is_empty() {
                        println!("deployments: 0 recorded");
                    } else {
                        for record in records {
                            println!(
                                "{} {} {} {:?} {}",
                                &record.deployment_id[..record.deployment_id.len().min(16)],
                                record.release_id,
                                record.artifact_sha256.as_str(),
                                record.state,
                                record.failure_reason.as_deref().unwrap_or("")
                            );
                        }
                    }
                    crate::EXIT_OK
                }
                Err(err) => fail("history", &err),
            }
        }
        DeployCommand::Rollback {
            path,
            release,
            reason,
            confirm,
        } => {
            if !*confirm {
                // §55/§50：回滚是破坏性动作，必须显式确认。
                eprintln!(
                    "prism: rollback is destructive — re-run with --confirm to roll back to '{release}'"
                );
                return crate::EXIT_INVALID_INPUT;
            }
            let target = local_target(path);
            match prism_deploy::transport::LocalTransport::new(&target) {
                Ok(mut transport) => {
                    match prism_deploy::rollback(&mut transport, release, reason, cancel) {
                        Ok(record) => {
                            println!(
                                "rollback: {} => {:?} (release {release})",
                                record.deployment_id, record.state
                            );
                            crate::EXIT_OK
                        }
                        Err(err) => fail("rollback", &err),
                    }
                }
                Err(err) => fail("rollback target", &err),
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn run_plan(
    source: &Path,
    target_root: &Path,
    environment: &str,
    git_commit: Option<&str>,
    git_dirty: bool,
    critical_findings: u64,
    json: bool,
) -> i32 {
    let manifest = match build_manifest_checked(source, git_commit) {
        Ok(manifest) => manifest,
        Err(err) => return fail("artifact", err.to_string()),
    };
    let target = local_target_obj(target_root, environment);
    let profile = DeploymentProfile {
        profile_id: format!("profile:{}", target.target_id),
        target_id: target.target_id.clone(),
        releases_dir: "releases".to_string(),
        require_clean_git: false,
        require_artifact_hash: true,
        block_critical_findings: true,
    };
    let policy = DeploymentPolicy::from_profile(&profile);
    let preflight = preflight_report(&manifest, target_root, &policy);
    let plan = build_plan(
        &format!(
            "plan:{}",
            prism_intel::identity::fnv1a_hex16(manifest.artifact.sha256.as_str())
        ),
        &manifest,
        &target,
        &profile,
        &preflight,
        &policy,
        Some(git_dirty),
        critical_findings,
    );
    print_plan(&plan, json)
}

#[allow(clippy::too_many_arguments)]
fn run_deploy(
    source: &Path,
    target_root: &Path,
    environment: &str,
    git_commit: Option<&str>,
    git_dirty: bool,
    critical_findings: u64,
    confirm: bool,
    json: bool,
    cancel: &CancellationToken,
) -> i32 {
    // §50：先 plan（含 preflight/policy 评估），再按确认决定执行。
    let manifest = match build_manifest_checked(source, git_commit) {
        Ok(manifest) => manifest,
        Err(err) => return fail("artifact", err.to_string()),
    };
    let target = local_target_obj(target_root, environment);
    let profile = DeploymentProfile {
        profile_id: format!("profile:{}", target.target_id),
        target_id: target.target_id.clone(),
        releases_dir: "releases".to_string(),
        require_clean_git: false,
        require_artifact_hash: true,
        block_critical_findings: true,
    };
    let policy = DeploymentPolicy::from_profile(&profile);
    let preflight = preflight_report(&manifest, target_root, &policy);
    let plan = build_plan(
        &format!(
            "plan:{}",
            prism_intel::identity::fnv1a_hex16(manifest.artifact.sha256.as_str())
        ),
        &manifest,
        &target,
        &profile,
        &preflight,
        &policy,
        Some(git_dirty),
        critical_findings,
    );
    if plan.policy_decision == prism_deploy::PolicyDecision::Fail {
        eprintln!("prism: deployment BLOCKED by policy (see `prism deploy plan` for reasons)");
        return print_plan(&plan, json);
    }
    if plan.confirmation_required && !confirm {
        eprintln!(
            "prism: plan requires confirmation ({:?}) — re-run with --confirm to execute",
            plan.policy_decision
        );
        return print_plan(&plan, json);
    }
    if !confirm {
        eprintln!("prism: deploy requires --confirm to execute (§50 plan/execute separation)");
        return print_plan(&plan, json);
    }

    let transport = match prism_deploy::transport::LocalTransport::new(&target) {
        Ok(transport) => transport,
        Err(err) => return fail("transport", &err),
    };
    let release_id = prism_deploy::path_safety::make_release_id(
        &prism_core::Timestamp::now_utc().to_string()[..10].replace('-', ""),
        manifest.artifact.sha256.as_str(),
    );
    let result = prism_deploy::state::execute(
        &prism_deploy::state::ExecuteInput {
            plan: &plan,
            manifest: &manifest,
            staging_root: source,
            transport: &transport,
            remote_root: &target.remote_root,
            release_id: &release_id,
        },
        cancel,
    );
    // §三十 History 一等：结果落 history.json。
    let mut store = prism_deploy::DeploymentStore::load(&target_root.join("deploy-history.json"))
        .unwrap_or_default();
    store.append(result.record.clone());
    let _ = store.save();
    if json {
        print_json(&result);
    } else {
        println!(
            "deploy: {:?} stage={:?} files={}/{} atomic={} verified={}",
            result.state,
            result.failure_stage,
            result.transferred_files,
            result.transfers.len(),
            result.atomic,
            result.verified_hash.is_some()
        );
        if let Some(reason) = &result.failure_reason {
            println!("  reason: {reason}");
        }
    }
    if result.state == prism_deploy::DeploymentState::Succeeded {
        crate::EXIT_OK
    } else {
        crate::EXIT_ERROR
    }
}

fn build_manifest_checked(
    source: &Path,
    git_commit: Option<&str>,
) -> Result<prism_deploy::ArtifactManifest, String> {
    prism_deploy::artifact::build_manifest(
        source,
        &source
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| "artifact".to_string()),
        prism_deploy::model::ArtifactSource::Build,
        git_commit.map(str::to_string),
        &Default::default(),
        &CancellationToken::new(),
    )
    .map_err(|err| err.to_string())
}

fn local_target_obj(target_root: &Path, environment: &str) -> Target {
    let root_string = target_root.to_string_lossy().to_string();
    Target {
        target_id: format!(
            "target:{}",
            prism_intel::identity::fnv1a_hex16(&format!("local|{root_string}|{environment}"))
        ),
        name: environment.to_string(),
        kind: TargetKind::Local,
        address: root_string.clone(),
        remote_root: root_string,
        environment: environment.to_string(),
        credential: None,
        transport_capability: TransportCapability::Available,
    }
}

fn preflight_report(
    manifest: &prism_deploy::ArtifactManifest,
    target_root: &Path,
    policy: &DeploymentPolicy,
) -> prism_deploy::PreflightReport {
    use prism_deploy::plan::{CheckVerdict, PreflightCheck, PreflightReport};
    let mut checks = Vec::new();
    checks.push(PreflightCheck {
        name: "artifact_manifest".to_string(),
        verdict: if manifest.files.is_empty() {
            CheckVerdict::Fail
        } else {
            CheckVerdict::Pass
        },
        detail: format!(
            "{} files, {} bytes",
            manifest.files.len(),
            manifest.total_bytes
        ),
    });
    checks.push(PreflightCheck {
        name: "artifact_hash".to_string(),
        verdict: if policy.require_artifact_hash && manifest.artifact.sha256.0.is_empty() {
            CheckVerdict::Fail
        } else {
            CheckVerdict::Pass
        },
        detail: format!("sha256={}", manifest.artifact.sha256.as_str()),
    });
    let target_ok = target_root.is_dir();
    checks.push(PreflightCheck {
        name: "target_root".to_string(),
        verdict: if target_ok {
            CheckVerdict::Pass
        } else {
            CheckVerdict::Unknown
        },
        detail: format!(
            "target root {}",
            if target_ok {
                "exists"
            } else {
                "will be created"
            }
        ),
    });
    let blocked = checks
        .iter()
        .any(|check| check.verdict == CheckVerdict::Fail);
    let verdict = if checks
        .iter()
        .all(|check| check.verdict == CheckVerdict::Pass)
    {
        CheckVerdict::Pass
    } else if checks
        .iter()
        .any(|check| check.verdict == CheckVerdict::Fail)
    {
        CheckVerdict::Fail
    } else {
        CheckVerdict::Unknown
    };
    PreflightReport {
        checks,
        verdict,
        blocked,
    }
}

fn print_plan(plan: &prism_deploy::DeploymentPlan, json: bool) -> i32 {
    if json {
        match serde_json::to_string_pretty(plan) {
            Ok(text) => println!("{text}"),
            Err(err) => {
                eprintln!("prism: failed to serialize plan: {err}");
                return crate::EXIT_ERROR;
            }
        }
    } else {
        println!(
            "plan {} policy={:?} confirmation={}",
            plan.plan_id, plan.policy_decision, plan.confirmation_required
        );
        println!("  artifact sha256={}", plan.artifact_sha256.as_str());
        println!("  {}", plan.transfer_summary);
        println!("  {}", plan.apply_summary);
        println!("  {}", plan.verification_summary);
        for reason in &plan.reasons {
            println!("  reason: {reason}");
        }
        for check in &plan.preflight.checks {
            println!(
                "  preflight {}: {:?} ({})",
                check.name, check.verdict, check.detail
            );
        }
    }
    if plan.policy_decision == prism_deploy::PolicyDecision::Fail {
        crate::EXIT_ERROR
    } else {
        crate::EXIT_OK
    }
}

fn local_target(path: &Path) -> Target {
    let root_string = path.to_string_lossy().to_string();
    Target {
        target_id: format!(
            "target:{}",
            prism_intel::identity::fnv1a_hex16(&format!("local|{root_string}"))
        ),
        name: path.to_string_lossy().to_string(),
        kind: TargetKind::Local,
        address: root_string.clone(),
        remote_root: root_string,
        environment: "dev".to_string(),
        credential: None,
        transport_capability: TransportCapability::Available,
    }
}

fn print_json<T: serde::Serialize>(value: &T) {
    match serde_json::to_string_pretty(value) {
        Ok(text) => println!("{text}"),
        Err(err) => eprintln!("prism: failed to serialize: {err}"),
    }
}

fn fail(command: &str, err: impl std::fmt::Display) -> i32 {
    eprintln!("prism: {command} failed: {err}");
    crate::EXIT_ERROR
}
