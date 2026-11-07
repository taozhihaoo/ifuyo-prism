//! 分析结果持久化（优化清单批 A：2.0 域 SQLite 化）。
//!
//! 写入口 = health/health2 分析 worker（完成即落一行，best-effort——
//! 落库失败不掩盖分析成功，只记诊断）；读取方 = 会话缓存回退
//! （`get_cached_domain`）与 MCP 只读工具（`prism mcp`）。
//! 数据库 = prism-store 默认路径（`PRISM_DB_PATH` 可覆盖，与快照同库）。

use prism_store::{baseline_of, AnalysisRunRecord, AnalysisRunsStore, SqliteAnalysisRunsStore};

/// 打开结果库（含父目录创建；与快照库同文件、同迁移序列）。
pub(crate) fn open_runs_store() -> Result<SqliteAnalysisRunsStore, String> {
    let path = prism_store::default_db_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    SqliteAnalysisRunsStore::open(&path).map_err(|err| err.to_string())
}

/// 追加一次分析运行（best-effort 由调用方决定是否吞错）。
pub(crate) fn persist_run(
    project_root: &str,
    domain: &str,
    schema_version: i64,
    basis: &str,
    finding_ids: Vec<String>,
    lifecycle_new: Vec<String>,
    payload: serde_json::Value,
) -> Result<(), String> {
    let store = open_runs_store()?;
    store
        .record_run(&AnalysisRunRecord {
            id: None,
            project_root: project_root.to_string(),
            domain: domain.to_string(),
            schema_version,
            basis: basis.to_string(),
            captured_at: prism_core::Timestamp::now_utc().to_string(),
            finding_ids,
            lifecycle_new,
            payload,
        })
        .map_err(|err| err.to_string())?;
    Ok(())
}

/// 最近一次运行（MCP / 缓存回退共用）。
pub(crate) fn load_latest_run(
    project_root: &str,
    domain: &str,
) -> Result<Option<AnalysisRunRecord>, String> {
    let store = open_runs_store()?;
    store
        .latest_run(project_root, domain)
        .map_err(|err| err.to_string())
}

/// 基线选择（同 schema 才可比；跨版本 ⇒ 首轮语义）。
pub(crate) fn usable_baseline(
    latest: Option<AnalysisRunRecord>,
    schema_version: i64,
) -> Option<AnalysisRunRecord> {
    baseline_of(latest, schema_version)
}
