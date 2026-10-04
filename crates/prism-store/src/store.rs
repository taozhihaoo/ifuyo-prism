//! SnapshotStore 抽象 + SQLite/InMemory 实现（规格 M11a §19–§25/§64–§67/
//! §144/§226–§231/§299）。
//!
//! SQL 只存在于本文件（§228）；domain crate 只依赖 trait 与 JSON 行契约
//! （§229/§299 infra isolation）。写入走事务，保证 snapshot 原子性（§65/§66）。

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::sync::Mutex;

/// 存储层错误（§119/§121/§185：复用现有错误体系形态，不造第二套大系统）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StoreError {
    Unavailable(String),
    MigrationFailed(String),
    CorruptStore(String),
    PermissionDenied(String),
    NotFound,
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StoreError::Unavailable(message) => write!(f, "storage unavailable: {message}"),
            StoreError::MigrationFailed(message) => write!(f, "migration failed: {message}"),
            StoreError::CorruptStore(message) => write!(f, "store corrupted: {message}"),
            StoreError::PermissionDenied(message) => write!(f, "permission denied: {message}"),
            StoreError::NotFound => write!(f, "not found"),
        }
    }
}

impl std::error::Error for StoreError {}

impl From<rusqlite::Error> for StoreError {
    fn from(err: rusqlite::Error) -> Self {
        if matches!(err, rusqlite::Error::QueryReturnedNoRows) {
            return StoreError::NotFound;
        }
        StoreError::Unavailable(err.to_string())
    }
}

/// 项目注册记录（§8/§9 ProjectRegistry；canonical_root 是本地身份依据，§8）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectRecord {
    pub id: String,
    pub display_name: String,
    pub canonical_root: String,
    pub created_at: String,
    pub updated_at: String,
    pub last_seen_at: String,
    pub persistence_enabled: bool,
}

/// snapshot 头行（snapshots 表映射，§25）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotRecord {
    pub id: String,
    pub project_id: String,
    pub captured_at: String,
    pub basis: String,
    pub git_revision: Option<String>,
    pub git_branch: Option<String>,
    pub working_tree_state: String,
    pub state_fingerprint: String,
    /// 十个域指纹（§6）：field → hex。
    pub domain_fingerprints: BTreeMap<String, String>,
    pub manifest_json: Value,
    pub limitations_json: Value,
    pub note: Option<String>,
    pub schema_version: u32,
}

/// 域行（snapshot_domains 表映射，§25：域可独立判 freshness/availability）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DomainRow {
    pub snapshot_id: String,
    pub domain: String,
    pub status: String,
    pub captured_at: String,
    pub source_revision: Option<String>,
    pub input_fingerprint: Option<String>,
    pub content_fingerprint: Option<String>,
    pub freshness: String,
    pub data_json: Value,
}

/// Store 抽象（§226/§227：上层不写 SQL）。
pub trait SnapshotStore: Send + Sync {
    fn upsert_project(&self, project: &ProjectRecord) -> Result<(), StoreError>;
    fn get_project_by_root(
        &self,
        canonical_root: &str,
    ) -> Result<Option<ProjectRecord>, StoreError>;
    fn get_project(&self, id: &str) -> Result<Option<ProjectRecord>, StoreError>;
    fn find_snapshot_by_fingerprint(
        &self,
        project_id: &str,
        state_fingerprint: &str,
    ) -> Result<Option<SnapshotRecord>, StoreError>;
    fn insert_snapshot_atomic(
        &self,
        snapshot: &SnapshotRecord,
        domains: &[DomainRow],
    ) -> Result<(), StoreError>;
    fn list_snapshots(&self, project_id: &str) -> Result<Vec<SnapshotRecord>, StoreError>;
    fn get_snapshot(&self, id: &str) -> Result<Option<SnapshotRecord>, StoreError>;
    fn delete_project_history(&self, project_id: &str) -> Result<usize, StoreError>;
}

/// Schema migrations（§64 P0：schema_migrations 表 + 顺序执行 + 失败即停）。
const MIGRATIONS: &[(&str, &str)] = &[(
    "0001_initial",
    "CREATE TABLE projects (
            id TEXT PRIMARY KEY,
            display_name TEXT NOT NULL,
            canonical_root TEXT NOT NULL UNIQUE,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            last_seen_at TEXT NOT NULL,
            persistence_enabled INTEGER NOT NULL DEFAULT 1,
            schema_version INTEGER NOT NULL DEFAULT 1
        );
        CREATE TABLE snapshots (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL REFERENCES projects(id),
            captured_at TEXT NOT NULL,
            created_at TEXT NOT NULL,
            basis TEXT NOT NULL,
            git_revision TEXT,
            git_branch TEXT,
            working_tree_state TEXT NOT NULL,
            state_fingerprint TEXT NOT NULL,
            domain_fingerprints_json TEXT NOT NULL,
            manifest_json TEXT NOT NULL,
            limitations_json TEXT NOT NULL,
            note TEXT,
            schema_version INTEGER NOT NULL DEFAULT 1
        );
        CREATE INDEX idx_snapshots_project ON snapshots(project_id, captured_at);
        CREATE INDEX idx_snapshots_fingerprint ON snapshots(project_id, state_fingerprint);
        CREATE TABLE snapshot_domains (
            snapshot_id TEXT NOT NULL REFERENCES snapshots(id),
            domain TEXT NOT NULL,
            status TEXT NOT NULL,
            captured_at TEXT NOT NULL,
            source_revision TEXT,
            input_fingerprint TEXT,
            content_fingerprint TEXT,
            freshness TEXT NOT NULL,
            data_json TEXT NOT NULL,
            PRIMARY KEY (snapshot_id, domain)
        );",
)];

fn migrate(connection: &Connection) -> Result<(), StoreError> {
    connection
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_migrations (
                name TEXT PRIMARY KEY,
                applied_at TEXT NOT NULL DEFAULT (datetime('now'))
            );",
        )
        .map_err(|err| StoreError::MigrationFailed(err.to_string()))?;
    for (name, sql) in MIGRATIONS {
        let applied: bool = connection
            .query_row(
                "SELECT COUNT(*) FROM schema_migrations WHERE name = ?1",
                [name],
                |row| row.get::<_, i64>(0),
            )
            .map(|count| count > 0)
            .map_err(|err| StoreError::MigrationFailed(err.to_string()))?;
        if applied {
            continue;
        }
        connection
            .execute_batch(sql)
            .map_err(|err| StoreError::MigrationFailed(format!("{name}: {err}")))?;
        connection
            .execute("INSERT INTO schema_migrations (name) VALUES (?1)", [name])
            .map_err(|err| StoreError::MigrationFailed(err.to_string()))?;
    }
    Ok(())
}

const SNAPSHOT_COLUMNS: &str = "id, project_id, captured_at, basis, git_revision, git_branch, working_tree_state, state_fingerprint, domain_fingerprints_json, manifest_json, limitations_json, note, schema_version";

fn row_to_snapshot(row: &rusqlite::Row) -> rusqlite::Result<SnapshotRecord> {
    let fingerprints_json: String = row.get("domain_fingerprints_json")?;
    Ok(SnapshotRecord {
        id: row.get("id")?,
        project_id: row.get("project_id")?,
        captured_at: row.get("captured_at")?,
        basis: row.get("basis")?,
        git_revision: row.get("git_revision")?,
        git_branch: row.get("git_branch")?,
        working_tree_state: row.get("working_tree_state")?,
        state_fingerprint: row.get("state_fingerprint")?,
        domain_fingerprints: serde_json::from_str(&fingerprints_json).unwrap_or_default(),
        manifest_json: serde_json::from_str(&row.get::<_, String>("manifest_json")?)
            .unwrap_or(Value::Null),
        limitations_json: serde_json::from_str(&row.get::<_, String>("limitations_json")?)
            .unwrap_or(Value::Null),
        note: row.get("note")?,
        schema_version: row.get("schema_version")?,
    })
}

/// SQLite 实现（§20：local/embedded/migration-based/transactional）。
pub struct SqliteSnapshotStore {
    connection: Mutex<Connection>,
}

impl SqliteSnapshotStore {
    /// 打开（或创建）数据库并应用迁移（§64：启动时 read→apply→verify）。
    pub fn open(path: &std::path::Path) -> Result<Self, StoreError> {
        let connection =
            Connection::open(path).map_err(|err| StoreError::Unavailable(err.to_string()))?;
        // §144：SQLite busy 时等待而不是立刻失败。
        connection
            .busy_timeout(std::time::Duration::from_secs(5))
            .map_err(|err| StoreError::Unavailable(err.to_string()))?;
        migrate(&connection)?;
        Ok(SqliteSnapshotStore {
            connection: Mutex::new(connection),
        })
    }

    /// 内存库（§202 crash-safety / §217 可重复测试；不落盘）。
    pub fn open_in_memory() -> Result<Self, StoreError> {
        let connection =
            Connection::open_in_memory().map_err(|err| StoreError::Unavailable(err.to_string()))?;
        migrate(&connection)?;
        Ok(SqliteSnapshotStore {
            connection: Mutex::new(connection),
        })
    }
}

fn row_to_snapshot_checked(row: &rusqlite::Row) -> rusqlite::Result<SnapshotRecord> {
    row_to_snapshot(row)
}

impl SnapshotStore for SqliteSnapshotStore {
    fn upsert_project(&self, project: &ProjectRecord) -> Result<(), StoreError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| StoreError::Unavailable("lock poisoned".into()))?;
        connection
            .execute(
                "INSERT INTO projects (id, display_name, canonical_root, created_at, updated_at, last_seen_at, persistence_enabled)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT(canonical_root) DO UPDATE SET
                    display_name = excluded.display_name,
                    updated_at = excluded.updated_at,
                    last_seen_at = excluded.last_seen_at",
                rusqlite::params![
                    project.id,
                    project.display_name,
                    project.canonical_root,
                    project.created_at,
                    project.updated_at,
                    project.last_seen_at,
                    project.persistence_enabled as i64,
                ],
            )
            .map_err(|err| StoreError::Unavailable(err.to_string()))?;
        Ok(())
    }

    fn get_project_by_root(
        &self,
        canonical_root: &str,
    ) -> Result<Option<ProjectRecord>, StoreError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| StoreError::Unavailable("lock poisoned".into()))?;
        let mut statement = connection
            .prepare(
                "SELECT id, display_name, canonical_root, created_at, updated_at, last_seen_at, persistence_enabled FROM projects WHERE canonical_root = ?1",
            )
            .map_err(|err| StoreError::Unavailable(err.to_string()))?;
        let mut rows = statement
            .query([canonical_root])
            .map_err(|err| StoreError::Unavailable(err.to_string()))?;
        if let Some(row) = rows
            .next()
            .map_err(|err| StoreError::Unavailable(err.to_string()))?
        {
            return Ok(Some(ProjectRecord {
                id: row.get(0)?,
                display_name: row.get(1)?,
                canonical_root: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
                last_seen_at: row.get(5)?,
                persistence_enabled: row.get::<_, i64>(6)? != 0,
            }));
        }
        Ok(None)
    }

    fn get_project(&self, id: &str) -> Result<Option<ProjectRecord>, StoreError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| StoreError::Unavailable("lock poisoned".into()))?;
        let mut statement = connection
            .prepare(
                "SELECT id, display_name, canonical_root, created_at, updated_at, last_seen_at, persistence_enabled FROM projects WHERE id = ?1",
            )
            .map_err(|err| StoreError::Unavailable(err.to_string()))?;
        let mut rows = statement
            .query([id])
            .map_err(|err| StoreError::Unavailable(err.to_string()))?;
        if let Some(row) = rows
            .next()
            .map_err(|err| StoreError::Unavailable(err.to_string()))?
        {
            return Ok(Some(ProjectRecord {
                id: row.get(0)?,
                display_name: row.get(1)?,
                canonical_root: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
                last_seen_at: row.get(5)?,
                persistence_enabled: row.get::<_, i64>(6)? != 0,
            }));
        }
        Ok(None)
    }

    fn find_snapshot_by_fingerprint(
        &self,
        project_id: &str,
        state_fingerprint: &str,
    ) -> Result<Option<SnapshotRecord>, StoreError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| StoreError::Unavailable("lock poisoned".into()))?;
        let mut statement = connection
            .prepare(
                "SELECT id, project_id, captured_at, basis, git_revision, git_branch, working_tree_state, state_fingerprint, domain_fingerprints_json, manifest_json, limitations_json, note, schema_version
                 FROM snapshots WHERE project_id = ?1 AND state_fingerprint = ?2 ORDER BY captured_at LIMIT 1",
            )
            .map_err(|err| StoreError::Unavailable(err.to_string()))?;
        let mut rows = statement
            .query(rusqlite::params![project_id, state_fingerprint])
            .map_err(|err| StoreError::Unavailable(err.to_string()))?;
        if let Some(row) = rows
            .next()
            .map_err(|err| StoreError::Unavailable(err.to_string()))?
        {
            return Ok(Some(row_to_snapshot_checked(row)?));
        }
        Ok(None)
    }

    fn insert_snapshot_atomic(
        &self,
        snapshot: &SnapshotRecord,
        domains: &[DomainRow],
    ) -> Result<(), StoreError> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| StoreError::Unavailable("lock poisoned".into()))?;
        // §65/§66：事务保证 header + domains 原子写入（无半成品）。
        let transaction = connection
            .transaction()
            .map_err(|err| StoreError::Unavailable(err.to_string()))?;
        transaction
            .execute(
                "INSERT INTO snapshots (id, project_id, captured_at, created_at, basis, git_revision, git_branch, working_tree_state, state_fingerprint, domain_fingerprints_json, manifest_json, limitations_json, note, schema_version)
                 VALUES (?1, ?2, ?3, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                rusqlite::params![
                    snapshot.id,
                    snapshot.project_id,
                    snapshot.captured_at,
                    snapshot.basis,
                    snapshot.git_revision,
                    snapshot.git_branch,
                    snapshot.working_tree_state,
                    snapshot.state_fingerprint,
                    serde_json::to_string(&snapshot.domain_fingerprints).unwrap_or_default(),
                    serde_json::to_string(&snapshot.manifest_json).unwrap_or_default(),
                    serde_json::to_string(&snapshot.limitations_json).unwrap_or_default(),
                    snapshot.note,
                    snapshot.schema_version,
                ],
            )
            .map_err(|err| StoreError::Unavailable(err.to_string()))?;
        for domain in domains {
            transaction
                .execute(
                    "INSERT INTO snapshot_domains (snapshot_id, domain, status, captured_at, source_revision, input_fingerprint, content_fingerprint, freshness, data_json)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    rusqlite::params![
                        domain.snapshot_id,
                        domain.domain,
                        domain.status,
                        domain.captured_at,
                        domain.source_revision,
                        domain.input_fingerprint,
                        domain.content_fingerprint,
                        domain.freshness,
                        serde_json::to_string(&domain.data_json).unwrap_or_default(),
                    ],
                )
                .map_err(|err| StoreError::Unavailable(err.to_string()))?;
        }
        transaction
            .commit()
            .map_err(|err| StoreError::Unavailable(err.to_string()))?;
        Ok(())
    }

    fn list_snapshots(&self, project_id: &str) -> Result<Vec<SnapshotRecord>, StoreError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| StoreError::Unavailable("lock poisoned".into()))?;
        let mut statement = connection
            .prepare(
                "SELECT id, project_id, captured_at, basis, git_revision, git_branch, working_tree_state, state_fingerprint, domain_fingerprints_json, manifest_json, limitations_json, note, schema_version
                 FROM snapshots WHERE project_id = ?1 ORDER BY captured_at DESC",
            )
            .map_err(|err| StoreError::Unavailable(err.to_string()))?;
        let mut rows = statement
            .query([project_id])
            .map_err(|err| StoreError::Unavailable(err.to_string()))?;
        let mut out = Vec::new();
        while let Some(row) = rows
            .next()
            .map_err(|err| StoreError::Unavailable(err.to_string()))?
        {
            out.push(row_to_snapshot(row)?);
        }
        Ok(out)
    }

    fn get_snapshot(&self, id: &str) -> Result<Option<SnapshotRecord>, StoreError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| StoreError::Unavailable("lock poisoned".into()))?;
        let mut statement = connection
            .prepare(&format!(
                "SELECT {SNAPSHOT_COLUMNS} FROM snapshots WHERE id = ?1"
            ))
            .map_err(|err| StoreError::Unavailable(err.to_string()))?;
        let mut rows = statement
            .query([id])
            .map_err(|err| StoreError::Unavailable(err.to_string()))?;
        if let Some(row) = rows
            .next()
            .map_err(|err| StoreError::Unavailable(err.to_string()))?
        {
            return Ok(Some(row_to_snapshot(row)?));
        }
        Ok(None)
    }

    fn delete_project_history(&self, project_id: &str) -> Result<usize, StoreError> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| StoreError::Unavailable("lock poisoned".into()))?;
        // §116：只删 Prism 历史（SQLite），绝不动项目文件。
        let transaction = connection
            .transaction()
            .map_err(|err| StoreError::Unavailable(err.to_string()))?;
        transaction
            .execute(
                "DELETE FROM snapshot_domains WHERE snapshot_id IN (SELECT id FROM snapshots WHERE project_id = ?1)",
                [project_id],
            )
            .map_err(|err| StoreError::Unavailable(err.to_string()))?;
        let deleted = transaction
            .execute("DELETE FROM snapshots WHERE project_id = ?1", [project_id])
            .map_err(|err| StoreError::Unavailable(err.to_string()))?;
        transaction
            .commit()
            .map_err(|err| StoreError::Unavailable(err.to_string()))?;
        Ok(deleted)
    }
}

/// 内存实现（§226：unit tests 不必启动 SQLite）。
#[derive(Default)]
pub struct InMemorySnapshotStore {
    inner: Mutex<InMemoryInner>,
}

#[derive(Default)]
struct InMemoryInner {
    projects: BTreeMap<String, ProjectRecord>,
    projects_by_root: BTreeMap<String, String>,
    snapshots: BTreeMap<String, SnapshotRecord>,
    domains: BTreeMap<String, Vec<DomainRow>>,
}

impl SnapshotStore for InMemorySnapshotStore {
    fn upsert_project(&self, project: &ProjectRecord) -> Result<(), StoreError> {
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| StoreError::Unavailable("lock".into()))?;
        let existing_id = inner.projects_by_root.get(&project.canonical_root).cloned();
        if let Some(existing_id) = existing_id {
            if let Some(existing) = inner.projects.get_mut(&existing_id) {
                existing.display_name = project.display_name.clone();
                existing.updated_at = project.updated_at.clone();
                existing.last_seen_at = project.last_seen_at.clone();
                return Ok(());
            }
        }
        inner
            .projects_by_root
            .insert(project.canonical_root.clone(), project.id.clone());
        inner.projects.insert(project.id.clone(), project.clone());
        Ok(())
    }

    fn get_project_by_root(
        &self,
        canonical_root: &str,
    ) -> Result<Option<ProjectRecord>, StoreError> {
        let inner = self
            .inner
            .lock()
            .map_err(|_| StoreError::Unavailable("lock".into()))?;
        Ok(inner
            .projects_by_root
            .get(canonical_root)
            .and_then(|id| inner.projects.get(id))
            .cloned())
    }

    fn get_project(&self, id: &str) -> Result<Option<ProjectRecord>, StoreError> {
        let inner = self
            .inner
            .lock()
            .map_err(|_| StoreError::Unavailable("lock".into()))?;
        Ok(inner.projects.get(id).cloned())
    }

    fn find_snapshot_by_fingerprint(
        &self,
        project_id: &str,
        state_fingerprint: &str,
    ) -> Result<Option<SnapshotRecord>, StoreError> {
        let inner = self
            .inner
            .lock()
            .map_err(|_| StoreError::Unavailable("lock".into()))?;
        Ok(inner
            .snapshots
            .values()
            .find(|snapshot| {
                snapshot.project_id == project_id && snapshot.state_fingerprint == state_fingerprint
            })
            .cloned())
    }

    fn insert_snapshot_atomic(
        &self,
        snapshot: &SnapshotRecord,
        domains: &[DomainRow],
    ) -> Result<(), StoreError> {
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| StoreError::Unavailable("lock".into()))?;
        inner
            .snapshots
            .insert(snapshot.id.clone(), snapshot.clone());
        inner.domains.insert(snapshot.id.clone(), domains.to_vec());
        Ok(())
    }

    fn list_snapshots(&self, project_id: &str) -> Result<Vec<SnapshotRecord>, StoreError> {
        let inner = self
            .inner
            .lock()
            .map_err(|_| StoreError::Unavailable("lock".into()))?;
        let mut out: Vec<SnapshotRecord> = inner
            .snapshots
            .values()
            .filter(|snapshot| snapshot.project_id == project_id)
            .cloned()
            .collect();
        out.sort_by(|a, b| b.captured_at.cmp(&a.captured_at));
        Ok(out)
    }

    fn get_snapshot(&self, id: &str) -> Result<Option<SnapshotRecord>, StoreError> {
        let inner = self
            .inner
            .lock()
            .map_err(|_| StoreError::Unavailable("lock".into()))?;
        Ok(inner.snapshots.get(id).cloned())
    }

    fn delete_project_history(&self, project_id: &str) -> Result<usize, StoreError> {
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| StoreError::Unavailable("lock".into()))?;
        let ids: Vec<String> = inner
            .snapshots
            .values()
            .filter(|snapshot| snapshot.project_id == project_id)
            .map(|snapshot| snapshot.id.clone())
            .collect();
        for id in &ids {
            inner.snapshots.remove(id);
            inner.domains.remove(id);
        }
        Ok(ids.len())
    }
}
