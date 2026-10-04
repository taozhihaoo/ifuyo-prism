//! `prism snapshot <path>` / `prism history <path>` — M11a 基础命令
//! （规格 M11a §10/§17/§105/§146/§147/§351）。
//!
//! Store：CLI 与 GUI 同一个 SqliteSnapshotStore（§22：默认
//! `<app-data>/Prism/prism.db`，`PRISM_DB_PATH` 可覆盖 §21/§220）。
//! 默认 Manual Snapshot First（§146/§147：无后台自动快照）。

use prism_core::task::CancellationToken;
use prism_snapshot::{DomainFacts, SnapshotCaptureOptions, SnapshotCaptureService};
use prism_store::{SqliteSnapshotStore, SnapshotStore};
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
pub fn run_capture(
    path: &Path,
    force: bool,
    note: Option<String>,
    json: bool,
) -> i32 {
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
    let options = SnapshotCaptureOptions { force, note, ..Default::default() };
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
        println!(
            "    fingerprint: {}",
            snapshot.state_fingerprint
        );
    }
    crate::EXIT_OK
}
