//! 应用共享状态：配置、服务实例与“当前项目扫描会话”。
//!
//! M1 的会话是单项目的内存态（纲领 §56：关闭 Prism 后结果可以消失，
//! 持久化属于 M11）。

use prism_core::application::AppInfoService;
use prism_core::config::PrismConfig;
use prism_core::ids::TaskId;
use prism_core::task::{CancellationToken, TaskState};
use prism_core::timestamp::Timestamp;
use prism_project::{Project, ProjectDiscovery, ScanProgress};
use std::sync::Mutex;

/// 一次扫描会话的完整状态。
#[derive(Debug, Default)]
pub struct ScanSession {
    /// 当前打开的项目（未打开为 None）。
    pub project: Option<Project>,
    /// 最近一次扫描的任务 ID。
    pub task_id: Option<TaskId>,
    /// 取消令牌（每次扫描新建）。
    pub token: CancellationToken,
    /// 扫描状态：None = 尚未扫描（idle）。
    pub status: Option<TaskState>,
    /// 扫描过程中的真实计数。
    pub progress: ScanProgress,
    /// 扫描完成后的结果（Cancelled 也保留部分事实）。
    pub discovery: Option<ProjectDiscovery>,
    /// Failed 时的错误描述。
    pub error: Option<String>,
    /// 扫描开始时间。
    pub started_at: Option<Timestamp>,
}

impl ScanSession {
    /// 重置为“已打开项目、未扫描”状态。
    pub fn open(&mut self, project: Project) {
        *self = ScanSession {
            project: Some(project),
            ..ScanSession::default()
        };
        self.token = CancellationToken::new();
    }

    /// 准备一次新扫描（生成新 token 与 task id）。
    pub fn begin_scan(&mut self) -> TaskId {
        let task_id = TaskId::new();
        self.token = CancellationToken::new();
        self.task_id = Some(task_id.clone());
        self.status = Some(TaskState::Running);
        self.progress = ScanProgress::default();
        self.discovery = None;
        self.error = None;
        self.started_at = Some(Timestamp::now_utc());
        task_id
    }
}

/// Tauri managed state。只保存配置与服务实例，不缓存业务结果。
pub struct AppState {
    /// 当前里程碑命令尚未直接消费配置；保留为后续里程碑（读取 analysis/logging
    /// 配置）的入口，删除会让后续里程碑重复接线。
    #[allow(dead_code)]
    pub config: PrismConfig,
    pub app_info: AppInfoService,
    /// 扫描会话；Arc 便于后台扫描线程与 IPC 命令共享同一份状态。
    pub session: std::sync::Arc<Mutex<ScanSession>>,
}

impl AppState {
    pub fn new(config: PrismConfig) -> Self {
        AppState {
            config,
            app_info: AppInfoService::new(),
            session: std::sync::Arc::new(Mutex::new(ScanSession::default())),
        }
    }
}

/// 取锁（poison 时取回内部数据继续工作，不在业务路径 panic）。
pub fn lock_session(session: &Mutex<ScanSession>) -> std::sync::MutexGuard<'_, ScanSession> {
    session
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// status 的对外字符串（None → "idle"）。
pub fn status_string(status: Option<TaskState>) -> String {
    match status {
        None => "idle".to_string(),
        Some(state) => serde_json::to_value(state)
            .ok()
            .and_then(|value| value.as_str().map(str::to_string))
            .unwrap_or_else(|| "unknown".to_string()),
    }
}
