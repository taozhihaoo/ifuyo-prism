//! 分析结果持久化（优化清单批 A：2.0 域 SQLite 化）。
//!
//! `analysis_runs` 表 = **append-only 运行历史**：(canonical_root, domain)
//! 键控，每次 GUI health/health2 完成插入一行；最新一行 = 会话缓存的
//! 持久回退（重开项目秒级回显），上一行 = health2 的**跨会话基线**
//! （Persistent/Resolved/Regressed 生命周期由此真实化，不再全 New）。
//! 读取方（MCP `prism mcp`）以只读连接消费，写入口只在分析 worker。
//!
//! 边界：payload 是分析完成时已脱敏的序列化报告——本层不做二次脱敏、
//! 不读项目文件（§46/§21）；保留策略 = 每 (root, domain) 最新 50 行。

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Mutex;

use crate::store::{migrate, StoreError};

/// 每 (project_root, domain) 保留的最新运行行数（防无界增长）。
pub const RETENTION_PER_KEY: usize = 50;

/// 一次分析运行的持久化记录。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnalysisRunRecord {
    /// SQLite 自增 id；内存实现按插入序分配。未落库为 None。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<i64>,
    /// 项目 canonical root（M20 路径身份）。
    pub project_root: String,
    /// 域名（"health" / "health2"；后续域按需扩展）。
    pub domain: String,
    /// 报告 schema 版本——跨版本基线对比被拒绝（不伪造，§46）。
    pub schema_version: i64,
    /// 写入时的工作区基线（git head|dirty 计数；git 不可用为 "unknown"）。
    pub basis: String,
    /// 分析完成时间（UTC ISO-8601）。
    pub captured_at: String,
    /// 报告内全部 finding 指纹（基线 diff 的键集，免反序列化整个报告）。
    pub finding_ids: Vec<String>,
    /// 该轮中为 New 的指纹（Regressed 判定输入，§33）。
    pub lifecycle_new: Vec<String>,
    /// 完整报告 JSON（GUI 侧 GUI 报告 / CLI 侧 --json 负载，读取方自解）。
    pub payload: Value,
}

/// 运行头（列表视图；不含大 payload）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnalysisRunHead {
    pub id: i64,
    pub project_root: String,
    pub domain: String,
    pub schema_version: i64,
    pub basis: String,
    pub captured_at: String,
    pub findings_count: usize,
}

/// 分析运行存储抽象（SQLite 落盘 / InMemory 测试）。
pub trait AnalysisRunsStore: Send + Sync {
    fn record_run(&self, run: &AnalysisRunRecord) -> Result<i64, StoreError>;
    fn latest_run(
        &self,
        project_root: &str,
        domain: &str,
    ) -> Result<Option<AnalysisRunRecord>, StoreError>;
    fn list_runs(
        &self,
        project_root: &str,
        domain: &str,
        limit: usize,
    ) -> Result<Vec<AnalysisRunHead>, StoreError>;
}

/// 基线选择：最新一行即基线；schema 不匹配 ⇒ None（首轮语义，不伪造）。
pub fn baseline_of(
    latest: Option<AnalysisRunRecord>,
    current_schema_version: i64,
) -> Option<AnalysisRunRecord> {
    latest.filter(|run| run.schema_version == current_schema_version)
}

// ---------- SQLite ----------

/// SQLite 实现：与快照库同文件、同迁移序列（0001+0002 幂等）；独立连接
/// + busy_timeout，与 snapshot store 并发安全（短写）。
pub struct SqliteAnalysisRunsStore {
    connection: Mutex<Connection>,
}

const RUN_COLUMNS: &str =
    "id, project_root, domain, schema_version, basis, captured_at, finding_ids, lifecycle_new, payload";

fn row_to_run(row: &rusqlite::Row) -> rusqlite::Result<AnalysisRunRecord> {
    Ok(AnalysisRunRecord {
        id: Some(row.get("id")?),
        project_root: row.get("project_root")?,
        domain: row.get("domain")?,
        schema_version: row.get("schema_version")?,
        basis: row.get("basis")?,
        captured_at: row.get("captured_at")?,
        finding_ids: serde_json::from_str(&row.get::<_, String>("finding_ids")?)
            .unwrap_or_default(),
        lifecycle_new: serde_json::from_str(&row.get::<_, String>("lifecycle_new")?)
            .unwrap_or_default(),
        payload: serde_json::from_str(&row.get::<_, String>("payload")?).unwrap_or(Value::Null),
    })
}

impl SqliteAnalysisRunsStore {
    pub fn open(path: &std::path::Path) -> Result<Self, StoreError> {
        let connection =
            Connection::open(path).map_err(|err| StoreError::Unavailable(err.to_string()))?;
        connection
            .busy_timeout(std::time::Duration::from_secs(5))
            .map_err(|err| StoreError::Unavailable(err.to_string()))?;
        migrate(&connection)?;
        Ok(SqliteAnalysisRunsStore {
            connection: Mutex::new(connection),
        })
    }

    pub fn open_in_memory() -> Result<Self, StoreError> {
        let connection =
            Connection::open_in_memory().map_err(|err| StoreError::Unavailable(err.to_string()))?;
        migrate(&connection)?;
        Ok(SqliteAnalysisRunsStore {
            connection: Mutex::new(connection),
        })
    }
}

impl AnalysisRunsStore for SqliteAnalysisRunsStore {
    fn record_run(&self, run: &AnalysisRunRecord) -> Result<i64, StoreError> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| StoreError::Unavailable("lock poisoned".into()))?;
        let transaction = connection
            .transaction()
            .map_err(|err| StoreError::Unavailable(err.to_string()))?;
        transaction
            .execute(
                "INSERT INTO analysis_runs
                 (project_root, domain, schema_version, basis, captured_at, finding_ids, lifecycle_new, payload)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                rusqlite::params![
                    run.project_root,
                    run.domain,
                    run.schema_version,
                    run.basis,
                    run.captured_at,
                    serde_json::to_string(&run.finding_ids)
                        .map_err(|err| StoreError::Unavailable(err.to_string()))?,
                    serde_json::to_string(&run.lifecycle_new)
                        .map_err(|err| StoreError::Unavailable(err.to_string()))?,
                    serde_json::to_string(&run.payload)
                        .map_err(|err| StoreError::Unavailable(err.to_string()))?,
                ],
            )
            .map_err(|err| StoreError::Unavailable(err.to_string()))?;
        let id = transaction.last_insert_rowid();
        // 保留策略：每 (root, domain) 只留最新 RETENTION_PER_KEY 行。
        transaction
            .execute(
                "DELETE FROM analysis_runs WHERE project_root = ?1 AND domain = ?2 AND id NOT IN
                 (SELECT id FROM analysis_runs WHERE project_root = ?1 AND domain = ?2
                  ORDER BY id DESC LIMIT ?3)",
                rusqlite::params![run.project_root, run.domain, RETENTION_PER_KEY as i64],
            )
            .map_err(|err| StoreError::Unavailable(err.to_string()))?;
        transaction
            .commit()
            .map_err(|err| StoreError::Unavailable(err.to_string()))?;
        Ok(id)
    }

    fn latest_run(
        &self,
        project_root: &str,
        domain: &str,
    ) -> Result<Option<AnalysisRunRecord>, StoreError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| StoreError::Unavailable("lock poisoned".into()))?;
        let mut statement = connection
            .prepare(&format!(
                "SELECT {RUN_COLUMNS} FROM analysis_runs
                 WHERE project_root = ?1 AND domain = ?2 ORDER BY id DESC LIMIT 1"
            ))
            .map_err(|err| StoreError::Unavailable(err.to_string()))?;
        let mut rows = statement
            .query(rusqlite::params![project_root, domain])
            .map_err(|err| StoreError::Unavailable(err.to_string()))?;
        match rows.next() {
            Ok(Some(row)) => Ok(Some(row_to_run(row)?)),
            Ok(None) => Ok(None),
            Err(err) => Err(StoreError::Unavailable(err.to_string())),
        }
    }

    fn list_runs(
        &self,
        project_root: &str,
        domain: &str,
        limit: usize,
    ) -> Result<Vec<AnalysisRunHead>, StoreError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| StoreError::Unavailable("lock poisoned".into()))?;
        let mut statement = connection
            .prepare(
                "SELECT id, project_root, domain, schema_version, basis, captured_at,
                        json_array_length(finding_ids) AS findings_count
                 FROM analysis_runs WHERE project_root = ?1 AND domain = ?2
                 ORDER BY id DESC LIMIT ?3",
            )
            .map_err(|err| StoreError::Unavailable(err.to_string()))?;
        let rows = statement
            .query_map(
                rusqlite::params![project_root, domain, limit as i64],
                |row| {
                    Ok(AnalysisRunHead {
                        id: row.get("id")?,
                        project_root: row.get("project_root")?,
                        domain: row.get("domain")?,
                        schema_version: row.get("schema_version")?,
                        basis: row.get("basis")?,
                        captured_at: row.get("captured_at")?,
                        findings_count: row.get::<_, i64>("findings_count").unwrap_or(0) as usize,
                    })
                },
            )
            .map_err(|err| StoreError::Unavailable(err.to_string()))?;
        let mut heads = Vec::new();
        for row in rows {
            heads.push(row.map_err(|err| StoreError::Unavailable(err.to_string()))?);
        }
        Ok(heads)
    }
}

// ---------- InMemory ----------

/// 内存实现（单测不必启动 SQLite）。
#[derive(Default)]
pub struct InMemoryAnalysisRunsStore {
    inner: Mutex<InMemoryRuns>,
}

#[derive(Default)]
struct InMemoryRuns {
    next_id: i64,
    runs: Vec<AnalysisRunRecord>,
}

impl AnalysisRunsStore for InMemoryAnalysisRunsStore {
    fn record_run(&self, run: &AnalysisRunRecord) -> Result<i64, StoreError> {
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| StoreError::Unavailable("lock".into()))?;
        inner.next_id += 1;
        let id = inner.next_id;
        let mut stored = run.clone();
        stored.id = Some(id);
        inner.runs.push(stored);
        // 与 SQLite 相同的保留策略（行为一致性）。
        let root = run.project_root.clone();
        let domain = run.domain.clone();
        let keep: std::collections::BTreeSet<i64> = inner
            .runs
            .iter()
            .filter(|r| r.project_root == root && r.domain == domain)
            .map(|r| r.id.unwrap_or(0))
            .rev()
            .take(RETENTION_PER_KEY)
            .collect();
        inner.runs.retain(|r| {
            !(r.project_root == root && r.domain == domain && !keep.contains(&r.id.unwrap_or(0)))
        });
        Ok(id)
    }

    fn latest_run(
        &self,
        project_root: &str,
        domain: &str,
    ) -> Result<Option<AnalysisRunRecord>, StoreError> {
        let inner = self
            .inner
            .lock()
            .map_err(|_| StoreError::Unavailable("lock".into()))?;
        Ok(inner
            .runs
            .iter()
            .filter(|r| r.project_root == project_root && r.domain == domain)
            .max_by_key(|r| r.id.unwrap_or(0))
            .cloned())
    }

    fn list_runs(
        &self,
        project_root: &str,
        domain: &str,
        limit: usize,
    ) -> Result<Vec<AnalysisRunHead>, StoreError> {
        let inner = self
            .inner
            .lock()
            .map_err(|_| StoreError::Unavailable("lock".into()))?;
        let mut heads: Vec<AnalysisRunHead> = inner
            .runs
            .iter()
            .filter(|r| r.project_root == project_root && r.domain == domain)
            .map(|r| AnalysisRunHead {
                id: r.id.unwrap_or(0),
                project_root: r.project_root.clone(),
                domain: r.domain.clone(),
                schema_version: r.schema_version,
                basis: r.basis.clone(),
                captured_at: r.captured_at.clone(),
                findings_count: r.finding_ids.len(),
            })
            .collect();
        heads.sort_by_key(|head| std::cmp::Reverse(head.id));
        heads.truncate(limit);
        Ok(heads)
    }
}
