//! Snapshot Capture 编排（规格 M11a §10–§12/§17–§18/§105/§108–§111/§123/
//! §136/§145/§323）。
//!
//! Store 不跑分析器（§323）：本服务收集 M1/M4 事实 → 规范化 → 指纹 →
//! 去重 → 经 SnapshotStore 持久化。M2–M10 域默认 NotCaptured（§108/§109：
//! 不因 snapshot 触发重分析），调用方可经 `DomainFacts` 注入既有结果。

use crate::model::{
    combine_fingerprints, fingerprint_value, CaptureStatus, DomainFreshness, DomainStatus,
    ProjectSnapshot, SnapshotBasis, SnapshotCaptureResult, SnapshotDomainState,
    SNAPSHOT_SCHEMA_VERSION,
};
use prism_core::task::CancellationToken;
use prism_core::Timestamp;
use prism_store::{DomainRow, ProjectRecord, SnapshotRecord, SnapshotStore};
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

/// 十个域的固定名单（§6）。
pub const DOMAINS: [&str; 10] = [
    "structure",
    "code",
    "architecture",
    "git",
    "change",
    "impact",
    "health",
    "verification",
    "environment",
    "context_health",
];

/// 调用方可注入的既有域事实（§10/§263：复用 M1–M10 结果，不重算）。
#[derive(Default)]
pub struct DomainFacts {
    /// 各域规范化内容（domain → canonical JSON）。注入 ⇒ Available。
    pub data: BTreeMap<String, serde_json::Value>,
}

/// 稳定项目身份（§8）：canonical root 为本地身份依据；首见即注册。
pub fn resolve_project_identity(
    store: &dyn SnapshotStore,
    project_root: &Path,
    now: &str,
) -> Result<ProjectRecord, prism_core::PrismError> {
    let canonical_root = project_root.to_string_lossy().to_string();
    if let Some(existing) = store
        .get_project_by_root(&canonical_root)
        .map_err(prism_store_error)?
    {
        // §9：last_seen 更新。
        let mut seen = existing.clone();
        seen.last_seen_at = now.to_string();
        store
            .upsert_project(&seen)
            .map_err(prism_store_error)?;
        return Ok(existing);
    }
    let record = ProjectRecord {
        id: format!("proj:{}", uuid::Uuid::new_v4()),
        display_name: project_root
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| canonical_root.clone()),
        canonical_root,
        created_at: now.to_string(),
        updated_at: now.to_string(),
        last_seen_at: now.to_string(),
        persistence_enabled: true,
    };
    store.upsert_project(&record).map_err(prism_store_error)?;
    Ok(record)
}

fn prism_store_error(err: prism_store::StoreError) -> prism_core::PrismError {
    prism_core::PrismError::Internal {
        message: format!("snapshot store: {err}"),
    }
}

/// Capture 选项（§111）。
#[derive(Debug, Clone, Default)]
pub struct SnapshotCaptureOptions {
    /// §17：强制创建新 observation（跳过去重）。
    pub force: bool,
    /// §91：用户备注。
    pub note: Option<String>,
    /// §111：include_git（默认 true；git 域来自 M4 只读事实）。
    pub include_git: bool,
}

/// Capture（§10/§11 pipeline 的本地实现；§323 store 不跑分析器）。
pub struct SnapshotCaptureService {
    store: Arc<dyn SnapshotStore>,
}

impl SnapshotCaptureService {
    pub fn new(store: Arc<dyn SnapshotStore>) -> Self {
        SnapshotCaptureService { store }
    }

    /// 捕获快照（快速路径：M1 结构 + M4 git；其余域按注入/NotCaptured）。
    #[allow(clippy::too_many_arguments)]
    pub fn capture(
        &self,
        project_root: &Path,
        facts: &DomainFacts,
        options: &SnapshotCaptureOptions,
        cancel: &CancellationToken,
    ) -> prism_core::PrismResult<SnapshotCaptureResult> {
        let started = Instant::now();
        let now = Timestamp::now_utc().to_string();

        // ---- Resolve Project Identity（§8/§9）----
        let identity = resolve_project_identity(&*self.store, project_root, &now)?;

        // ---- Git state（§5：basis；M4 只读，§114 不改工作树）----
        let runner: Arc<dyn prism_core::runtime::command::CommandRunner> =
            Arc::new(prism_core::runtime::command::StdCommandRunner::new());
        let git = prism_git::GitService::new(runner.clone());
        let status = git.status(project_root, false, cancel).ok();
        let (basis, git_revision, git_branch, working_tree_state) = match &status {
            Some(status) if status.repository.is_some() => {
                let dirty = !status.entries.is_empty();
                let revision = status.head.as_ref().and_then(|head| head.commit_id.clone());
                let branch = status.head.as_ref().and_then(|head| head.branch_name.clone());
                let _ = &branch;
                if dirty {
                    (SnapshotBasis::WorkingTree, revision, branch, "dirty".to_string())
                } else {
                    (SnapshotBasis::GitRevision, revision, branch, "clean".to_string())
                }
            }
            _ => (SnapshotBasis::WorkingTree, None, None, "non_git".to_string()),
        };

        // ---- M1 structure facts（规范化：文件计数 + 语言 + 目录）----
        let structure_data = if cancel.is_cancelled() {
            serde_json::Value::Null
        } else {
            prism_project::DiscoveryService::new()
                .discover(project_root, prism_project::ScanOptions::default(), cancel, |_| {})
                .ok()
                .map(|discovery| {
                    serde_json::json!({
                        "total_files": discovery.statistics.total_files,
                        "total_directories": discovery.statistics.total_directories,
                        "languages": discovery.languages.iter()
                            .map(|language| { serde_json::json!({
                                "language": format!("{:?}", language.language),
                                "count": language.count,
                            }) })
                            .collect::<Vec<_>>(),
                    })
                })
                .unwrap_or(serde_json::Value::Null)
        };

        // ---- 域状态装配（§13；未注入域 ⇒ NotCaptured，诚实降级 §260）----
        let captured_at = now.clone();
        let mut domains: Vec<SnapshotDomainState> = Vec::new();
        let mut domain_rows: Vec<DomainRow> = Vec::new();
        let mut fingerprints: BTreeMap<String, String> = BTreeMap::new();
        let mut counts: BTreeMap<String, u64> = BTreeMap::new();
        let mut limitations = Vec::new();

        for domain in DOMAINS {
            let (status, data, freshness) = match domain {
                "structure" => match &structure_data {
                    serde_json::Value::Null => (DomainStatus::Unavailable, serde_json::Value::Null, DomainFreshness::Unknown),
                    value => {
                        if let Some(total) = value.get("total_files").and_then(|v| v.as_u64()) {
                            counts.insert("files".to_string(), total);
                        }
                        (DomainStatus::Available, value.clone(), DomainFreshness::Fresh)
                    }
                },
                "git" => {
                    if !options.include_git || status.is_none() {
                        (DomainStatus::NotApplicable, serde_json::json!({ "present": false }), DomainFreshness::Unknown)
                    } else {
                        let git_data = serde_json::json!({
                            "basis": basis.as_str(),
                            "revision": git_revision,
                            "branch": git_branch,
                            "working_tree_state": working_tree_state,
                        });
                        (DomainStatus::Available, git_data, DomainFreshness::Fresh)
                    }
                }
                other => match facts.data.get(other) {
                    Some(value) => (DomainStatus::Available, value.clone(), DomainFreshness::Fresh),
                    None => (DomainStatus::NotCaptured, serde_json::Value::Null, DomainFreshness::Unknown),
                },
            };
            let content_fingerprint = if data.is_null() {
                None
            } else {
                Some(fingerprint_value(&data))
            };
            fingerprints.insert(domain.to_string(), fingerprint_value(&serde_json::json!({
                "status": status.as_str(),
                "content": data,
            })));
            limitations.push(format!(
                "domain {domain}: {} ({})",
                status.as_str(),
                if data.is_null() { "no facts" } else { "facts captured" }
            ));
            domain_rows.push(DomainRow {
                snapshot_id: String::new(), // insert 时由 service 填
                domain: domain.to_string(),
                status: status.as_str().to_string(),
                captured_at: captured_at.clone(),
                source_revision: git_revision.clone(),
                input_fingerprint: None,
                content_fingerprint: content_fingerprint.clone(),
                freshness: freshness.as_str().to_string(),
                data_json: data.clone(),
            });
            domains.push(SnapshotDomainState {
                domain: domain.to_string(),
                status,
                captured_at: captured_at.clone(),
                source_revision: git_revision.clone(),
                input_fingerprint: None,
                content_fingerprint,
                freshness,
                data,
                limitations: Vec::new(),
            });
        }

        let state_fingerprint = combine_fingerprints(&fingerprints);

        // ---- 去重（§17/§18：状态未变 ⇒ 返回既有，不创建重复）----
        if !options.force {
            if let Some(existing) = self
                .store
                .find_snapshot_by_fingerprint(&identity.id, &state_fingerprint)
                .map_err(prism_store_error)?
            {
                return Ok(SnapshotCaptureResult {
                    status: CaptureStatus::Deduplicated,
                    snapshot_id: existing.id,
                    state_fingerprint,
                    created: false,
                    deduplicated: true,
                    duration_ms: started.elapsed().as_millis() as u64,
                    domain_results: domains
                        .iter()
                        .map(|domain| format!("{}: {}", domain.domain, domain.status.as_str()))
                        .collect(),
                    warnings: vec!["Snapshot unchanged; existing snapshot returned (§17).".to_string()],
                    limitations,
                    persisted: true,
                });
            }
        }

        // ---- Build + Persist（§65/§66 原子写）----
        let id = format!("snap:{}", uuid::Uuid::new_v4());
        let snapshot = ProjectSnapshot {
            id: id.clone(),
            project_id: identity.id.clone(),
            schema_version: SNAPSHOT_SCHEMA_VERSION,
            captured_at: captured_at.clone(),
            duration_ms: started.elapsed().as_millis() as u64,
            basis,
            git_revision: git_revision.clone(),
            git_branch: git_branch.clone(),
            working_tree_state: working_tree_state.clone(),
            project_name: identity.display_name.clone(),
            project_root: project_root.to_string_lossy().to_string(),
            state_fingerprint: state_fingerprint.clone(),
            structure_fingerprint: fingerprints["structure"].clone(),
            code_fingerprint: fingerprints["code"].clone(),
            architecture_fingerprint: fingerprints["architecture"].clone(),
            git_fingerprint: fingerprints["git"].clone(),
            change_fingerprint: fingerprints["change"].clone(),
            impact_fingerprint: fingerprints["impact"].clone(),
            health_fingerprint: fingerprints["health"].clone(),
            verification_fingerprint: fingerprints["verification"].clone(),
            environment_fingerprint: fingerprints["environment"].clone(),
            context_health_fingerprint: fingerprints["context_health"].clone(),
            source_versions: BTreeMap::new(),
            counts,
            domains: domains.clone(),
            limitations: limitations.clone(),
            note: options.note.clone(),
        };
        let manifest_json = serde_json::json!({
            "snapshot_id": snapshot.id,
            "project_id": snapshot.project_id,
            "schema_version": snapshot.schema_version,
            "basis": snapshot.basis.as_str(),
            "counts": snapshot.counts,
            "limitations": snapshot.limitations,
        });
        for row in &mut domain_rows {
            row.snapshot_id = id.clone();
        }
        let record = SnapshotRecord {
            id: snapshot.id.clone(),
            project_id: snapshot.project_id.clone(),
            captured_at: snapshot.captured_at.clone(),
            basis: snapshot.basis.as_str().to_string(),
            git_revision: snapshot.git_revision.clone(),
            git_branch: snapshot.git_branch.clone(),
            working_tree_state: snapshot.working_tree_state.clone(),
            state_fingerprint: snapshot.state_fingerprint.clone(),
            domain_fingerprints: fingerprints.clone(),
            manifest_json,
            limitations_json: serde_json::json!(snapshot.limitations),
            note: snapshot.note.clone(),
            schema_version: snapshot.schema_version,
        };
        self.store
            .insert_snapshot_atomic(&record, &domain_rows)
            .map_err(prism_store_error)?;

        Ok(SnapshotCaptureResult {
            status: CaptureStatus::Created,
            snapshot_id: id,
            state_fingerprint,
            created: true,
            deduplicated: false,
            duration_ms: started.elapsed().as_millis() as u64,
            domain_results: domains
                .iter()
                .map(|domain| format!("{}: {}", domain.domain, domain.status.as_str()))
                .collect(),
            warnings: Vec::new(),
            limitations,
            persisted: true,
        })
    }

    /// §105：列出项目快照。
    pub fn list(&self, project_root: &Path) -> prism_core::PrismResult<Vec<ProjectSnapshot>> {
        let canonical_root = project_root.to_string_lossy().to_string();
        let Some(project) = self
            .store
            .get_project_by_root(&canonical_root)
            .map_err(prism_store_error)?
        else {
            return Ok(Vec::new());
        };
        let records = self.store.list_snapshots(&project.id).map_err(prism_store_error)?;
        Ok(records
            .into_iter()
            .map(|record| snapshot_from_record(record, &project))
            .collect())
    }

    /// §105：取单条。
    pub fn get(&self, snapshot_id: &str) -> prism_core::PrismResult<Option<ProjectSnapshot>> {
        let Some(record) = self.store.get_snapshot(snapshot_id).map_err(prism_store_error)? else {
            return Ok(None);
        };
        let identity = self
            .store
            .get_project(&record.project_id.clone())
            .map_err(prism_store_error)?;
        let project = identity.unwrap_or(ProjectRecord {
            id: record.project_id.clone(),
            display_name: "unknown".to_string(),
            canonical_root: String::new(),
            created_at: String::new(),
            updated_at: String::new(),
            last_seen_at: String::new(),
            persistence_enabled: true,
        });
        Ok(Some(snapshot_from_record(record, &project)))
    }
}

fn snapshot_from_record(
    record: prism_store::SnapshotRecord,
    project: &ProjectRecord,
) -> ProjectSnapshot {
    let domains: Vec<SnapshotDomainState> = Vec::new();
    ProjectSnapshot {
        id: record.id,
        project_id: record.project_id,
        schema_version: record.schema_version,
        captured_at: record.captured_at,
        duration_ms: 0,
        basis: match record.basis.as_str() {
            "git_revision" => SnapshotBasis::GitRevision,
            "working_tree" => SnapshotBasis::WorkingTree,
            _ => SnapshotBasis::Unknown,
        },
        git_revision: record.git_revision,
        git_branch: record.git_branch,
        working_tree_state: record.working_tree_state,
        project_name: project.display_name.clone(),
        project_root: project.canonical_root.clone(),
        state_fingerprint: record.state_fingerprint,
        structure_fingerprint: record
            .domain_fingerprints
            .get("structure")
            .cloned()
            .unwrap_or_default(),
        code_fingerprint: record
            .domain_fingerprints
            .get("code")
            .cloned()
            .unwrap_or_default(),
        architecture_fingerprint: record
            .domain_fingerprints
            .get("architecture")
            .cloned()
            .unwrap_or_default(),
        git_fingerprint: record
            .domain_fingerprints
            .get("git")
            .cloned()
            .unwrap_or_default(),
        change_fingerprint: record
            .domain_fingerprints
            .get("change")
            .cloned()
            .unwrap_or_default(),
        impact_fingerprint: record
            .domain_fingerprints
            .get("impact")
            .cloned()
            .unwrap_or_default(),
        health_fingerprint: record
            .domain_fingerprints
            .get("health")
            .cloned()
            .unwrap_or_default(),
        verification_fingerprint: record
            .domain_fingerprints
            .get("verification")
            .cloned()
            .unwrap_or_default(),
        environment_fingerprint: record
            .domain_fingerprints
            .get("environment")
            .cloned()
            .unwrap_or_default(),
        context_health_fingerprint: record
            .domain_fingerprints
            .get("context_health")
            .cloned()
            .unwrap_or_default(),
        source_versions: BTreeMap::new(),
        counts: BTreeMap::new(),
        domains,
        limitations: serde_json::from_value(record.limitations_json).unwrap_or_default(),
        note: record.note,
    }
}
