//! M11 GUI：Snapshot History / Compare / Timeline IPC（规格 M11b §127–§133/
//! §49–§51/§125–§126；§475 GUI/CLI 同源 SnapshotStore）。

use crate::state::{lock_session, AppState};
use prism_core::error::PrismErrorDto;
use prism_core::task::CancellationToken;
use prism_snapshot::{counts_from_json, SnapshotCaptureOptions, SnapshotCaptureService};
use prism_store::{default_db_path, SnapshotStore, SqliteSnapshotStore};
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::State;

fn snapshot_root(state: &State<'_, AppState>) -> Result<PathBuf, PrismErrorDto> {
    let session = lock_session(&state.session);
    session
        .project
        .as_ref()
        .map(|project| project.root_path.clone())
        .ok_or_else(|| {
            PrismErrorDto::new(
                "invalid_input",
                "no project opened; call open_project first",
            )
        })
}

fn store_error(err: prism_store::StoreError) -> PrismErrorDto {
    PrismErrorDto::new("internal", format!("snapshot store: {err}"))
}

fn open_store() -> Result<Arc<dyn SnapshotStore>, PrismErrorDto> {
    let path = default_db_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let store = SqliteSnapshotStore::open(&path).map_err(store_error)?;
    Ok(Arc::new(store))
}

fn snapshot_service(
    state: &State<'_, AppState>,
) -> Result<(PathBuf, Arc<SnapshotCaptureService>, CancellationToken), PrismErrorDto> {
    let root = snapshot_root(state)?;
    let store = open_store()?;
    let service = SnapshotCaptureService::new(store);
    Ok((root, Arc::new(service), CancellationToken::new()))
}

/// `snapshot capture`（M11a §147 Manual First；session 防并发 §145）。
#[tauri::command]
pub fn snapshot_capture(
    state: State<'_, AppState>,
    force: Option<bool>,
    note: Option<String>,
) -> Result<Value, PrismErrorDto> {
    let (root, service, cancel) = snapshot_service(&state)?;
    let options = SnapshotCaptureOptions {
        force: force.unwrap_or(false),
        note,
        ..Default::default()
    };
    let result = service
        .capture(
            &root,
            &prism_snapshot::DomainFacts::default(),
            &options,
            &cancel,
        )
        .map_err(|err| PrismErrorDto::new("internal", format!("snapshot capture failed: {err}")))?;
    serde_json::to_value(&result)
        .map_err(|err| PrismErrorDto::new("internal", format!("serialize failed: {err}")))
}

/// 历史列表（§130：date/basis/revision/state/counts/note）。
#[tauri::command]
pub fn snapshot_history(state: State<'_, AppState>) -> Result<Value, PrismErrorDto> {
    let root = snapshot_root(&state)?;
    let store = open_store()?;
    let canonical_root = root.to_string_lossy().to_string();
    let Some(project) = store
        .get_project_by_root(&canonical_root)
        .map_err(|err| PrismErrorDto::new("internal", err.to_string()))?
    else {
        return Ok(serde_json::json!([]));
    };
    let snapshots = store
        .list_snapshots(&project.id)
        .map_err(|err| PrismErrorDto::new("internal", err.to_string()))?;
    let out: Vec<Value> = snapshots
        .iter()
        .map(|record| {
            serde_json::json!({
                "id": record.id,
                "captured_at": record.captured_at,
                "basis": record.basis,
                "git_revision": record.git_revision,
                "git_branch": record.git_branch,
                "working_tree_state": record.working_tree_state,
                "state_fingerprint": record.state_fingerprint,
                "counts": counts_from_json(&serde_json::json!({
                    "counts": record.manifest_json.get("counts").cloned().unwrap_or(Value::Null)
                })),
                "note": record.note,
            })
        })
        .collect();
    Ok(serde_json::json!(out))
}

/// Compare A vs B（§49–§50/§95：分域 added/removed/changed facts）。
#[tauri::command]
pub fn compare_snapshots(
    state: State<'_, AppState>,
    snapshot_a: String,
    snapshot_b: String,
) -> Result<Value, PrismErrorDto> {
    let _root = snapshot_root(&state)?;
    let store = open_store()?;
    prism_snapshot::render_compare(store.as_ref(), &snapshot_a, &snapshot_b, true)
        .map_err(|err| PrismErrorDto::new("invalid_input", err))
        .and_then(|text| {
            serde_json::from_str(&text)
                .map_err(|err| PrismErrorDto::new("internal", format!("parse failed: {err}")))
        })
}

/// Timeline（§43–§47/§125：snapshots + git 派生）。
#[tauri::command]
pub fn snapshot_timeline(state: State<'_, AppState>) -> Result<Value, PrismErrorDto> {
    let root = snapshot_root(&state)?;
    let store = open_store()?;
    let canonical_root = root.to_string_lossy().to_string();
    let Some(project) = store
        .get_project_by_root(&canonical_root)
        .map_err(|err| PrismErrorDto::new("internal", err.to_string()))?
    else {
        return Ok(
            serde_json::json!({ "project_id": null, "events": [], "limitations": ["No snapshots yet."] }),
        );
    };
    let snapshots = store
        .list_snapshots(&project.id)
        .map_err(|err| PrismErrorDto::new("internal", err.to_string()))?;
    let models: Vec<prism_snapshot::ProjectSnapshot> = snapshots
        .iter()
        .map(|record| {
            let counts = counts_from_json(&serde_json::json!({
                "counts": record.manifest_json.get("counts").cloned().unwrap_or(Value::Null)
            }));
            prism_snapshot::ProjectSnapshot {
                id: record.id.clone(),
                project_id: record.project_id.clone(),
                schema_version: record.schema_version,
                captured_at: record.captured_at.clone(),
                duration_ms: 0,
                basis: prism_snapshot::SnapshotBasis::Unknown,
                git_revision: record.git_revision.clone(),
                git_branch: record.git_branch.clone(),
                working_tree_state: record.working_tree_state.clone(),
                project_name: project.display_name.clone(),
                project_root: project.canonical_root.clone(),
                state_fingerprint: record.state_fingerprint.clone(),
                structure_fingerprint: String::new(),
                code_fingerprint: String::new(),
                architecture_fingerprint: String::new(),
                git_fingerprint: String::new(),
                change_fingerprint: String::new(),
                impact_fingerprint: String::new(),
                health_fingerprint: String::new(),
                verification_fingerprint: String::new(),
                environment_fingerprint: String::new(),
                context_health_fingerprint: String::new(),
                source_versions: Default::default(),
                counts,
                domains: vec![],
                limitations: vec![],
                note: record.note.clone(),
            }
        })
        .collect();

    // git commits（M4 只读 §115）。
    let runner: Arc<dyn prism_core::runtime::command::CommandRunner> =
        Arc::new(prism_core::runtime::command::StdCommandRunner::new());
    let git = prism_git::GitService::new(runner);
    let commits = git
        .log(&root, 200, &CancellationToken::new())
        .unwrap_or_default()
        .into_iter()
        .map(|commit| prism_snapshot::GitCommitFact {
            id: commit.id.clone(),
            timestamp: commit
                .committed_at
                .clone()
                .or(commit.authored_at.clone())
                .unwrap_or_default(),
            subject: commit.subject.clone(),
        })
        .collect::<Vec<_>>();

    let timeline = prism_snapshot::build_timeline(&project.id, &models, &commits);
    serde_json::to_value(&timeline)
        .map_err(|err| PrismErrorDto::new("internal", format!("serialize failed: {err}")))
}

/// 删除单条快照（§62：--yes 语义由前端确认对话框承担；此处仍要求显式 id）。
#[tauri::command]
pub fn delete_snapshot(
    state: State<'_, AppState>,
    snapshot_id: String,
) -> Result<Value, PrismErrorDto> {
    let _root = snapshot_root(&state)?;
    let store = open_store()?;
    let deleted = store
        .delete_snapshot(&snapshot_id)
        .map_err(|err| PrismErrorDto::new("internal", err.to_string()))?;
    Ok(serde_json::json!({ "deleted": deleted }))
}
