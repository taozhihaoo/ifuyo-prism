//! `prism snapshot <path>` / `prism history <path>` — M11a 基础命令
//! （规格 M11a §10/§17/§105/§146/§147/§351）。
//!
//! Store：CLI 与 GUI 同一个 SqliteSnapshotStore（§22：默认
//! `<app-data>/Prism/prism.db`，`PRISM_DB_PATH` 可覆盖 §21/§220）。
//! 默认 Manual Snapshot First（§146/§147：无后台自动快照）。

use prism_core::task::CancellationToken;
use prism_snapshot::{DomainFacts, SnapshotCaptureOptions, SnapshotCaptureService};
use prism_store::{SnapshotStore, SqliteSnapshotStore};
use std::path::Path;
use std::sync::Arc;

/// 打开全局 store（§22：CLI/GUI 同一 SQLite）。
pub fn open_store() -> Result<Arc<dyn SnapshotStore>, prism_store::StoreError> {
    let path = prism_store::default_db_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let store = SqliteSnapshotStore::open(&path)?;
    Ok(Arc::new(store))
}

/// `prism snapshot <path> [--force] [--note <text>] [--json]`。
pub fn run_capture(path: &Path, force: bool, note: Option<String>, json: bool) -> i32 {
    if !path.exists() {
        eprintln!("prism: path does not exist: {}", path.to_string_lossy());
        return crate::EXIT_INVALID_INPUT;
    }
    let store = match open_store() {
        Ok(store) => store,
        Err(err) => {
            // §119/§120：storage 失败是可行动错误，不 panic、不吞。
            eprintln!("prism: snapshot store unavailable: {err}");
            return crate::EXIT_ERROR;
        }
    };
    let service = SnapshotCaptureService::new(store);
    let options = SnapshotCaptureOptions {
        force,
        note,
        ..Default::default()
    };
    let result = match service.capture(
        path,
        &DomainFacts::default(),
        &options,
        &CancellationToken::new(),
    ) {
        Ok(result) => result,
        Err(err) => {
            eprintln!("prism: snapshot capture failed: {err}");
            return crate::EXIT_ERROR;
        }
    };
    if json {
        match serde_json::to_string_pretty(&result) {
            Ok(text) => println!("{text}"),
            Err(err) => {
                eprintln!("prism: failed to serialize capture result: {err}");
                return crate::EXIT_ERROR;
            }
        }
    } else if result.deduplicated {
        println!("Snapshot unchanged.");
        println!("Existing snapshot: {}", result.snapshot_id);
    } else {
        println!("Snapshot created: {}", result.snapshot_id);
        println!("State fingerprint: {}", result.state_fingerprint);
        println!("Domains:");
        for domain in &result.domain_results {
            println!("  {domain}");
        }
    }
    // §351：incomplete/去重都不是失败。
    crate::EXIT_OK
}

/// `prism history <path> [--json] [--limit N]`。
pub fn run_history(path: &Path, json: bool, limit: Option<usize>) -> i32 {
    if !path.exists() {
        eprintln!("prism: path does not exist: {}", path.to_string_lossy());
        return crate::EXIT_INVALID_INPUT;
    }
    let store = match open_store() {
        Ok(store) => store,
        Err(err) => {
            eprintln!("prism: snapshot store unavailable: {err}");
            return crate::EXIT_ERROR;
        }
    };
    let service = SnapshotCaptureService::new(store);
    let mut snapshots = match service.list(path) {
        Ok(snapshots) => snapshots,
        Err(err) => {
            eprintln!("prism: failed to list history: {err}");
            return crate::EXIT_ERROR;
        }
    };
    if let Some(limit) = limit {
        snapshots.truncate(limit);
    }
    if json {
        match serde_json::to_string_pretty(&snapshots) {
            Ok(text) => println!("{text}"),
            Err(err) => {
                eprintln!("prism: failed to serialize history: {err}");
                return crate::EXIT_ERROR;
            }
        }
        return crate::EXIT_OK;
    }
    if snapshots.is_empty() {
        println!("No snapshots for this project yet (manual snapshot first, §147).");
        return crate::EXIT_OK;
    }
    println!("Snapshots: {}", snapshots.len());
    for snapshot in &snapshots {
        println!(
            "  {}  {}  [{}] {}{}",
            snapshot.id,
            snapshot.captured_at,
            snapshot.basis.as_str(),
            snapshot.working_tree_state,
            snapshot
                .note
                .as_deref()
                .map(|note| format!(" · note: {note}"))
                .unwrap_or_default()
        );
        println!("    fingerprint: {}", snapshot.state_fingerprint);
    }
    crate::EXIT_OK
}

/// §59：compare 渲染（markdown/json）。
#[allow(clippy::too_many_arguments)]
pub fn run_compare(
    path: &Path,
    snapshot_a: &str,
    snapshot_b: &str,
    json: bool,
    markdown: bool,
) -> i32 {
    if !path.exists() {
        eprintln!("prism: path does not exist: {}", path.to_string_lossy());
        return crate::EXIT_INVALID_INPUT;
    }
    let store = match open_store() {
        Ok(store) => store,
        Err(err) => {
            eprintln!("prism: snapshot store unavailable: {err}");
            return crate::EXIT_ERROR;
        }
    };
    match prism_snapshot::render_compare(store.as_ref(), snapshot_a, snapshot_b, json) {
        Ok(text) => {
            if markdown && !json {
                print!("{}", text);
            } else if json {
                println!("{text}");
            } else {
                print!("{}", text);
            }
            crate::EXIT_OK
        }
        Err(err) => {
            eprintln!("prism: compare failed: {err}");
            crate::EXIT_ERROR
        }
    }
}

/// §60：timeline 渲染（git + snapshot 派生）。
pub fn run_timeline(path: &Path, limit: Option<usize>, json: bool, markdown: bool) -> i32 {
    if !path.exists() {
        eprintln!("prism: path does not exist: {}", path.to_string_lossy());
        return crate::EXIT_INVALID_INPUT;
    }
    let store = match open_store() {
        Ok(store) => store,
        Err(err) => {
            eprintln!("prism: snapshot store unavailable: {err}");
            return crate::EXIT_ERROR;
        }
    };
    // json=false 时 render_timeline 本身输出 markdown（§60 --markdown）。
    let as_json = json || !markdown;
    match prism_snapshot::render_timeline(store.as_ref(), path, limit, as_json) {
        Ok(text) => {
            println!("{text}");
            crate::EXIT_OK
        }
        Err(err) => {
            eprintln!("prism: timeline failed: {err}");
            crate::EXIT_ERROR
        }
    }
}

/// §61：history export（用户显式操作 §260；导出前再脱敏 §261）。
pub fn run_history_export(path: &Path, output: Option<String>) -> i32 {
    if !path.exists() {
        eprintln!("prism: path does not exist: {}", path.to_string_lossy());
        return crate::EXIT_INVALID_INPUT;
    }
    let store = match open_store() {
        Ok(store) => store,
        Err(err) => {
            eprintln!("prism: snapshot store unavailable: {err}");
            return crate::EXIT_ERROR;
        }
    };
    match prism_snapshot::render_history_export(store.as_ref(), path) {
        Ok(content) => match output {
            Some(output_path) => {
                if std::fs::write(&output_path, content).is_err() {
                    eprintln!("prism: failed to write export file");
                    return crate::EXIT_ERROR;
                }
                println!("History exported to {output_path}");
                crate::EXIT_OK
            }
            None => {
                println!("{content}");
                crate::EXIT_OK
            }
        },
        Err(err) => {
            eprintln!("prism: export failed: {err}");
            crate::EXIT_ERROR
        }
    }
}

/// §62：delete selected snapshot / clear project history（--yes 必须 §62）。
pub fn run_history_delete(path: &Path, snapshot: Option<String>, yes: bool, all: bool) -> i32 {
    if !yes {
        eprintln!("prism: destructive operation requires --yes (§62)");
        return crate::EXIT_INVALID_INPUT;
    }
    let store = match open_store() {
        Ok(store) => store,
        Err(err) => {
            eprintln!("prism: snapshot store unavailable: {err}");
            return crate::EXIT_ERROR;
        }
    };
    let canonical_root = path.to_string_lossy().to_string();
    let Some(project) = store
        .get_project_by_root(&canonical_root)
        .map_err(|err| {
            eprintln!("prism: store error: {err}");
        })
        .ok()
        .flatten()
    else {
        eprintln!("prism: no history for this project");
        return crate::EXIT_OK;
    };
    let deleted = if let Some(snapshot_id) = snapshot {
        match store.delete_snapshot(&snapshot_id) {
            Ok(deleted) => deleted as usize,
            Err(err) => {
                eprintln!("prism: delete failed: {err}");
                return crate::EXIT_ERROR;
            }
        }
    } else if all {
        match store.delete_project_history(&project.id) {
            Ok(deleted) => deleted,
            Err(err) => {
                eprintln!("prism: delete failed: {err}");
                return crate::EXIT_ERROR;
            }
        }
    } else {
        eprintln!("prism: specify --snapshot <id> or --all");
        return crate::EXIT_INVALID_INPUT;
    };
    println!("Deleted {deleted} snapshot(s). Project files untouched (§116).");
    crate::EXIT_OK
}
