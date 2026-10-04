//! M12 GUI：Recent projects + Settings 的 Tauri 命令（§12/§48/§68）。

use crate::state::{lock_session, AppState};
use prism_core::error::PrismErrorDto;
use prism_core::Timestamp;
use prism_store::{default_db_path, ProjectRecord, SnapshotStore, SqliteSnapshotStore};
use serde::Serialize;
use std::path::PathBuf;
use tauri::State;
use uuid::Uuid;

fn store_error(err: prism_store::StoreError) -> PrismErrorDto {
    PrismErrorDto::new("internal", format!("snapshot store: {err}"))
}

#[derive(Debug, Clone, Serialize)]
pub struct RecentProjectDto {
    pub id: String,
    pub display_name: String,
    pub canonical_root: String,
    pub last_seen_at: String,
}

/// §68 Recent Projects：来自 ProjectRegistry（M11a projects 表），按
/// last_seen 降序。读取失败 ⇒ 空列表（persistence 是能力不是前置条件 §82）。
#[tauri::command]
pub fn list_recent_projects(
    state: State<'_, AppState>,
) -> Result<Vec<RecentProjectDto>, PrismErrorDto> {
    let session = lock_session(&state.session);
    let current_root = session
        .project
        .as_ref()
        .map(|project| project.root_path.to_string_lossy().to_string());
    drop(session);

    let path = default_db_path();
    if !path.exists() {
        return Ok(vec![]);
    }
    let store = SqliteSnapshotStore::open(&path).map_err(store_error)?;
    let projects = store.list_projects().map_err(store_error)?;
    let mut current_root = current_root;
    Ok(projects
        .into_iter()
        .filter(|record| record.persistence_enabled)
        .map(|record| {
            // §68/§69：当前打开的项目排最前并标记。
            let is_current = current_root.as_deref() == Some(record.canonical_root.as_str());
            if is_current {
                current_root = None; // 只标记第一个命中
            }
            RecentProjectDto {
                id: record.id,
                display_name: record.display_name,
                canonical_root: record.canonical_root,
                last_seen_at: record.last_seen_at,
            }
        })
        .collect())
}

/// §8/§9：打开项目时在 Registry 中登记（last_seen 更新）。best-effort：
/// Registry 写失败不阻塞项目打开（§82 persistence 是能力不是前置条件）。
#[tauri::command]
pub fn register_project_open(state: State<'_, AppState>) -> Result<(), PrismErrorDto> {
    let root = {
        let session = lock_session(&state.session);
        session
            .project
            .as_ref()
            .map(|project| project.root_path.clone())
    };
    let Some(root) = root else {
        return Ok(());
    };
    let path = default_db_path();
    if !path.exists() {
        return Ok(());
    }
    let store = SqliteSnapshotStore::open(&path).map_err(store_error)?;
    let canonical_root = root.to_string_lossy().to_string();
    let now = Timestamp::now_utc().to_string();
    let existing = store
        .get_project_by_root(&canonical_root)
        .map_err(store_error)?;
    let record = existing.unwrap_or(ProjectRecord {
        id: format!("proj:{}", Uuid::new_v4()),
        display_name: root
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| canonical_root.clone()),
        canonical_root: canonical_root.clone(),
        created_at: now.clone(),
        updated_at: now.clone(),
        last_seen_at: now.clone(),
        persistence_enabled: true,
    });
    store.upsert_project(&record).map_err(store_error)?;
    Ok(())
}

/// §48：设置页展示的存储信息（不暴露用户名/内部绝对路径以外的东西——
/// DB 路径属于本机信息，用户需要知道数据在哪，§291/§292）。
#[derive(Debug, Clone, Serialize)]
pub struct StorageInfoDto {
    pub db_path: String,
    pub db_exists: bool,
}

#[tauri::command]
pub fn get_storage_info() -> StorageInfoDto {
    let path: PathBuf = default_db_path();
    StorageInfoDto {
        db_exists: path.exists(),
        db_path: path.to_string_lossy().to_string(),
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectPathDto {
    pub root: String,
}

/// 供设置页显示当前项目 root。
#[tauri::command]
pub fn get_current_project_path(state: State<'_, AppState>) -> Option<ProjectPathDto> {
    let session = lock_session(&state.session);
    session.project.as_ref().map(|project| ProjectPathDto {
        root: project.root_path.to_string_lossy().to_string(),
    })
}
