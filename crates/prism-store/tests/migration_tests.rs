//! Store 迁移与损坏处理测试（规格 M11a §64/§119/§157）。

use prism_store::{ProjectRecord, SnapshotStore, SqliteSnapshotStore, StoreError};

fn record(id: &str, root: &str) -> ProjectRecord {
    ProjectRecord {
        id: id.to_string(),
        display_name: id.to_string(),
        canonical_root: root.to_string(),
        created_at: "2026-10-03T00:00:00Z".to_string(),
        updated_at: "2026-10-03T00:00:00Z".to_string(),
        last_seen_at: "2026-10-03T00:00:00Z".to_string(),
        persistence_enabled: true,
    }
}

// §157：empty DB → migration 0001 应用；re-open 幂等（不重复执行、不破坏）。
#[test]
fn migration_applies_once_and_reopen_is_idempotent() {
    let path = std::env::temp_dir().join(format!(
        "prism-store-mig-{}-{}.db",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0)
    ));
    let _ = std::fs::remove_file(&path);

    // 首次打开：建表 + 写入。
    let store = SqliteSnapshotStore::open(&path).expect("first open");
    store.upsert_project(&record("proj:a", "/tmp/a")).unwrap();
    drop(store);

    // 重开：迁移幂等，历史可读（§196 store 层对照）。
    let store = SqliteSnapshotStore::open(&path).expect("reopen");
    let project = store
        .get_project_by_root("/tmp/a")
        .expect("read ok")
        .expect("project persisted");
    assert_eq!(project.id, "proj:a");
    let _ = std::fs::remove_file(&path);
}

// §119/§157：corrupt DB ⇒ 明确 StoreError，不 panic。
#[test]
fn corrupt_db_file_yields_error_not_panic() {
    let path = std::env::temp_dir().join(format!(
        "prism-store-corrupt-{}-{}.db",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0)
    ));
    std::fs::write(&path, "this is definitely not a sqlite database").unwrap();

    let result = SqliteSnapshotStore::open(&path);
    match result {
        Err(err) => match err {
            StoreError::MigrationFailed(_) | StoreError::Unavailable(_) => {}
            other => panic!("unexpected error kind: {other:?}"),
        },
        Ok(_) => panic!("corrupt db must not open successfully"),
    }
    let _ = std::fs::remove_file(&path);
}
