//! CLI compare/timeline/export/delete 渲染与命令逻辑（规格 M11b §59–§62/
//! §97/§124–§125/§179–§182/§261–§263）。

use prism_core::task::CancellationToken;
use prism_core::Timestamp;
use crate::compare::build_snapshot_diff;
use crate::report::{
    counts_from_json, evolution_summary, render_compare_markdown, render_timeline_markdown,
};
use prism_store::SnapshotStore;
use std::path::Path;
use std::sync::Arc;

/// 加载 snapshot 记录与域 JSON（两个 map：domain → Value）。
pub fn load_snapshot_json(
    store: &dyn SnapshotStore,
    snapshot_id: &str,
) -> Result<(serde_json::Value, std::collections::BTreeMap<String, serde_json::Value>), String>
{
    let Some(record) = store.get_snapshot(snapshot_id).map_err(|err| err.to_string())? else {
        return Err(format!("snapshot not found: {snapshot_id}"));
    };
    let domains = store.get_domains(snapshot_id).map_err(|err| err.to_string())?;
    let mut domain_json = std::collections::BTreeMap::new();
    for row in domains {
        domain_json.insert(row.domain.clone(), row.data_json.clone());
    }
    let header = serde_json::json!({
        "id": record.id,
        "project_id": record.project_id,
        "captured_at": record.captured_at,
        "basis": record.basis,
        "git_revision": record.git_revision,
        "git_branch": record.git_branch,
        "working_tree_state": record.working_tree_state,
        "state_fingerprint": record.state_fingerprint,
        "counts": counts_from_json(&serde_json::json!({ "counts": record.manifest_json
            .get("counts").cloned().unwrap_or(serde_json::Value::Null) })),
        "note": record.note,
    });
    Ok((header, domain_json))
}

/// §59：compare 渲染。
pub fn render_compare(
    store: &dyn SnapshotStore,
    snapshot_a: &str,
    snapshot_b: &str,
    json: bool,
) -> Result<String, String> {
    let (header_a, domains_a) = load_snapshot_json(store, snapshot_a)?;
    let (header_b, domains_b) = load_snapshot_json(store, snapshot_b)?;
    let diff = build_snapshot_diff(snapshot_a, snapshot_b, &domains_a, &domains_b);
    let counts_a = counts_from_json(&header_a);
    let counts_b = counts_from_json(&header_b);
    let snapshot_a_model = snapshot_model_from_header(&header_a, counts_a.clone());
    let snapshot_b_model = snapshot_model_from_header(&header_b, counts_b.clone());
    let evolution = evolution_summary(&snapshot_a_model, &snapshot_b_model);
    if json {
        serde_json::to_string_pretty(&serde_json::json!({
            "snapshot_a": header_a,
            "snapshot_b": header_b,
            "diff": diff,
            "evolution": evolution,
        }))
        .map_err(|err| err.to_string())
    } else {
        let name_a = header_a
            .get("captured_at")
            .and_then(|value| value.as_str())
            .unwrap_or(snapshot_a)
            .to_string();
        let name_b = header_b
            .get("captured_at")
            .and_then(|value| value.as_str())
            .unwrap_or(snapshot_b)
            .to_string();
        Ok(render_compare_markdown(&name_a, &name_b, &diff, &evolution))
    }
}

fn snapshot_model_from_header(
    header: &serde_json::Value,
    counts: std::collections::BTreeMap<String, u64>,
) -> crate::model::ProjectSnapshot {
    crate::model::ProjectSnapshot {
        id: header
            .get("id")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_string(),
        project_id: header
            .get("project_id")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_string(),
        schema_version: 1,
        captured_at: header
            .get("captured_at")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_string(),
        duration_ms: 0,
        basis: crate::model::SnapshotBasis::Unknown,
        git_revision: None,
        git_branch: None,
        working_tree_state: String::new(),
        project_name: String::new(),
        project_root: String::new(),
        state_fingerprint: header
            .get("state_fingerprint")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_string(),
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
        note: None,
    }
}

/// §60/§125：timeline 渲染（snapshots + git → derived）。
pub fn render_timeline(
    store: &dyn SnapshotStore,
    project_root: &Path,
    limit: Option<usize>,
    json: bool,
) -> Result<String, String> {
    let canonical_root = project_root.to_string_lossy().to_string();
    let Some(project) = store
        .get_project_by_root(&canonical_root)
        .map_err(|err| err.to_string())?
    else {
        return Ok("No snapshots for this project yet.".to_string());
    };
    let mut snapshots = store
        .list_snapshots(&project.id)
        .map_err(|err| err.to_string())?;
    if let Some(limit) = limit {
        snapshots.truncate(limit);
    }
    // git commits（M4 只读；§89 git timeline 独立有效）。
    let runner: Arc<dyn prism_core::runtime::command::CommandRunner> =
        Arc::new(prism_core::runtime::command::StdCommandRunner::new());
    let git = prism_git::GitService::new(runner.clone());
    let commits = git
        .log(project_root, 200, &CancellationToken::new())
        .unwrap_or_default()
        .into_iter()
        .map(|commit| crate::timeline::GitCommitFact {
            id: commit.id.clone(),
            timestamp: commit
                .committed_at
                .clone()
                .or(commit.authored_at.clone())
                .unwrap_or_default(),
            subject: commit.subject.clone(),
        })
        .collect::<Vec<_>>();

    let models: Vec<crate::model::ProjectSnapshot> = snapshots
        .iter()
        .map(|record| {
            let counts = counts_from_json(&serde_json::json!({
                "counts": record.manifest_json.get("counts").cloned().unwrap_or(serde_json::Value::Null)
            }));
            snapshot_model_from_header(
                &serde_json::json!({
                    "id": record.id,
                    "project_id": record.project_id,
                    "captured_at": record.captured_at,
                    "state_fingerprint": record.state_fingerprint,
                }),
                counts,
            )
        })
        .collect();
    let timeline = crate::timeline::build_timeline(&project.id, &models, &commits);
    if json {
        serde_json::to_string_pretty(&timeline).map_err(|err| err.to_string())
    } else {
        Ok(render_timeline_markdown(&timeline))
    }
}

/// §61/§179/§262：export JSON（export metadata + 再脱敏）。
pub fn render_history_export(
    store: &dyn SnapshotStore,
    project_root: &Path,
) -> Result<String, String> {
    let canonical_root = project_root.to_string_lossy().to_string();
    let Some(project) = store
        .get_project_by_root(&canonical_root)
        .map_err(|err| err.to_string())?
    else {
        return Err("no snapshots for this project yet".to_string());
    };
    let snapshots = store.list_snapshots(&project.id).map_err(|err| err.to_string())?;
    let snapshot_values: Vec<serde_json::Value> = snapshots
        .iter()
        .map(|record| {
            let domains = store
                .get_domains(&record.id)
                .map_err(|err| err.to_string())?;
            Ok::<_, String>(serde_json::json!({
                "record": record,
                "domains": domains,
            }))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let timeline = render_timeline(store, project_root, None, true)?;
    let timeline: crate::timeline::Timeline =
        serde_json::from_str(&timeline).unwrap_or(crate::timeline::Timeline {
            project_id: project.id.clone(),
            events: vec![],
            limitations: vec!["timeline unavailable".to_string()],
        });
    let limitations = vec![
        "Export contains engineering facts, not source code (spec 61).".to_string(),
    ];
    let project_json = serde_json::json!({
        "id": project.id,
        "display_name": project.display_name,
        "canonical_root": project.canonical_root,
        "created_at": project.created_at,
        "last_seen_at": project.last_seen_at,
    });
    Ok(crate::report::render_history_export(
        &project_json,
        &snapshot_values,
        &timeline,
        &limitations,
        &Timestamp::now_utc().to_string(),
    ))
}
