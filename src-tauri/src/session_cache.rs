//! 会话级结果缓存（P2-2）。
//!
//! `(canonical_root, domain)` 键控，生命周期 = GUI 进程：`open_project`
//! 与重扫描**不清除**缓存（清了就失去"重开项目秒级回显"的意义）。
//! 新鲜度用**工作区基线**对比判定：写入时记录 `head|dirty_count` 指纹，
//! 读取时重算当前指纹——不一致 ⇒ `stale:true`（结果可能过期，如实标注，
//! §46：不把旧数据画成新数据）。重跑永远由用户显式触发（Manual First）。

use crate::state::{lock_session, AppState};
use prism_core::runtime::command::StdCommandRunner;
use prism_core::task::CancellationToken;
use prism_git::GitService;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tauri::State;

/// 当前支持的缓存域（新增域：加 payload 分支 + 写入点即可）。
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CachedDomainData {
    Health {
        analysis: prism_health::HealthAnalysis,
    },
    Health2 {
        report: crate::intel_commands::Health2GuiReport,
    },
}

#[derive(Debug, Clone)]
pub struct CachedDomain {
    pub data: CachedDomainData,
    /// 写入时的工作区基线（`head|dirty`；git 不可用 ⇒ `unknown`）。
    pub basis: String,
    pub completed_at: String,
}

/// AppState 级缓存（跨 open_project 存活）。
#[derive(Default)]
pub struct SessionCache {
    entries: Mutex<HashMap<(String, String), CachedDomain>>,
}

impl SessionCache {
    pub fn put(&self, root: &str, domain: &str, entry: CachedDomain) {
        let mut entries = self.lock();
        entries.insert((root.to_string(), domain.to_string()), entry);
    }

    pub fn get(&self, root: &str, domain: &str) -> Option<CachedDomain> {
        self.lock()
            .get(&(root.to_string(), domain.to_string()))
            .cloned()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<(String, String), CachedDomain>> {
        self.entries
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

/// 读取时的工作区基线（对写入时基线不一致 ⇒ 结果可能过期）。
pub fn workspace_basis(root: &std::path::Path) -> String {
    let cancel = CancellationToken::new();
    let service = GitService::new(Arc::new(StdCommandRunner::new()));
    match service.status(root, false, &cancel) {
        Ok(status) => format!(
            "{}|{}",
            status
                .head
                .as_ref()
                .and_then(|head| head.commit_id.clone())
                .unwrap_or_else(|| "none".to_string()),
            status.entries.len()
        ),
        Err(_) => "unknown".to_string(),
    }
}

/// 1.0 健康结果写入缓存（start_health 与全面分析 health 阶段共用）。
pub fn cache_health(
    cache: &SessionCache,
    root: &std::path::Path,
    analysis: &prism_health::HealthAnalysis,
) {
    cache.put(
        &root.to_string_lossy(),
        "health",
        CachedDomain {
            data: CachedDomainData::Health {
                analysis: analysis.clone(),
            },
            basis: workspace_basis(root),
            completed_at: prism_core::Timestamp::now_utc().to_string(),
        },
    );
}

/// health2 结果写入缓存（start_health2 共用）。
pub fn cache_health2(
    cache: &SessionCache,
    root: &std::path::Path,
    gui: &crate::intel_commands::Health2GuiReport,
) {
    cache.put(
        &root.to_string_lossy(),
        "health2",
        CachedDomain {
            data: CachedDomainData::Health2 {
                report: gui.clone(),
            },
            basis: workspace_basis(root),
            completed_at: prism_core::Timestamp::now_utc().to_string(),
        },
    );
}

/// 缓存读取响应：`stale` = 当前工作区基线 ≠ 写入时基线（如实标注）。
#[derive(Serialize)]
pub struct CachedDomainDto {
    pub domain: String,
    pub completed_at: String,
    pub basis: String,
    pub stale: bool,
    pub data: CachedDomainData,
}

#[tauri::command]
pub fn get_cached_domain(state: State<'_, AppState>, domain: String) -> Option<CachedDomainDto> {
    let root = {
        let session = lock_session(&state.session);
        session
            .project
            .as_ref()
            .map(|project| project.root_path.clone())
    }?;
    let entry = state.cache.get(root.to_string_lossy().as_ref(), &domain)?;
    let stale = match entry.basis.as_str() {
        "unknown" => false,
        _ => entry.basis != workspace_basis(&root),
    };
    Some(CachedDomainDto {
        domain,
        completed_at: entry.completed_at,
        basis: entry.basis,
        stale,
        data: entry.data,
    })
}
