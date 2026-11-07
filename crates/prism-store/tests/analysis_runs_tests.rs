//! 分析运行存储测试（优化清单批 A）：round-trip / 保留策略 / 基线门。

use prism_store::{
    baseline_of, AnalysisRunRecord, AnalysisRunsStore, InMemoryAnalysisRunsStore,
    SqliteAnalysisRunsStore, RETENTION_PER_KEY,
};

fn run(root: &str, domain: &str, captured_at: &str, findings: usize) -> AnalysisRunRecord {
    AnalysisRunRecord {
        id: None,
        project_root: root.to_string(),
        domain: domain.to_string(),
        schema_version: 1,
        basis: "abc|0".to_string(),
        captured_at: captured_at.to_string(),
        finding_ids: (0..findings).map(|i| format!("find:{i}")).collect(),
        lifecycle_new: vec!["find:0".to_string()],
        payload: serde_json::json!({ "findings": findings }),
    }
}

fn round_trip<S: AnalysisRunsStore>(store: &S) {
    store
        .record_run(&run("/a", "health2", "2026-10-07T01:00:00Z", 3))
        .unwrap();
    store
        .record_run(&run("/a", "health2", "2026-10-07T02:00:00Z", 1))
        .unwrap();
    store
        .record_run(&run("/a", "health", "2026-10-07T02:00:00Z", 5))
        .unwrap();
    store
        .record_run(&run("/b", "health2", "2026-10-07T03:00:00Z", 0))
        .unwrap();

    // latest 按 (root, domain) 隔离。
    let latest = store.latest_run("/a", "health2").unwrap().expect("latest");
    assert_eq!(latest.captured_at, "2026-10-07T02:00:00Z");
    assert_eq!(latest.finding_ids.len(), 1);
    assert_eq!(latest.payload["findings"], 1);
    let health = store.latest_run("/a", "health").unwrap().expect("health");
    assert_eq!(health.finding_ids.len(), 5);
    assert!(store.latest_run("/a", "debug").unwrap().is_none());

    // list：新→旧 + 头行计数。
    let heads = store.list_runs("/a", "health2", 10).unwrap();
    assert_eq!(heads.len(), 2);
    assert_eq!(heads[0].captured_at, "2026-10-07T02:00:00Z");
    assert_eq!(heads[0].findings_count, 1);
}

#[test]
fn in_memory_round_trip_and_retention() {
    let store = InMemoryAnalysisRunsStore::default();
    round_trip(&store);

    // 保留策略：只留最新 50 行。
    for i in 0..(RETENTION_PER_KEY + 10) {
        store
            .record_run(&run(
                "/ret",
                "health2",
                &format!("2026-01-01T{i:04}:00:00Z"),
                1,
            ))
            .unwrap();
    }
    let heads = store.list_runs("/ret", "health2", 1000).unwrap();
    assert_eq!(heads.len(), RETENTION_PER_KEY);
}

#[test]
fn sqlite_round_trip_and_retention() {
    let path = std::env::temp_dir().join(format!(
        "prism-store-runs-{}-{}.db",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0)
    ));
    let _ = std::fs::remove_file(&path);
    let store = SqliteAnalysisRunsStore::open(&path).expect("open");
    round_trip(&store);

    for i in 0..(RETENTION_PER_KEY + 5) {
        store
            .record_run(&run(
                "/ret",
                "health2",
                &format!("2026-01-01T{i:04}:00:00Z"),
                1,
            ))
            .unwrap();
    }
    let heads = store.list_runs("/ret", "health2", 1000).unwrap();
    assert_eq!(heads.len(), RETENTION_PER_KEY);

    // 重开幂等：迁移不重复执行，历史可读。
    drop(store);
    let reopened = SqliteAnalysisRunsStore::open(&path).expect("reopen");
    assert!(reopened.latest_run("/a", "health2").unwrap().is_some());
    let _ = std::fs::remove_file(&path);
}

#[test]
fn baseline_requires_matching_schema_version() {
    let store = InMemoryAnalysisRunsStore::default();
    store
        .record_run(&run("/a", "health2", "2026-10-07T01:00:00Z", 3))
        .unwrap();
    let latest = store.latest_run("/a", "health2").unwrap().unwrap();

    // 同 schema ⇒ 基线可用。
    assert!(baseline_of(Some(latest.clone()), 1).is_some());
    // schema 漂移 ⇒ 拒绝（首轮语义，不伪造对比）。
    assert!(baseline_of(Some(latest), 2).is_none());
    assert!(baseline_of(None, 1).is_none());
}
