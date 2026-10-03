//! 应用共享状态：配置、服务实例与“当前项目扫描会话”。
//!
//! M1 的会话是单项目的内存态（纲领 §56：关闭 Prism 后结果可以消失，
//! 持久化属于 M11）。

use prism_analysis::ProjectCodeModel;
use prism_core::application::AppInfoService;
use prism_core::config::PrismConfig;
use prism_core::ids::TaskId;
use prism_core::task::{CancellationToken, TaskState};
use prism_core::timestamp::Timestamp;
use prism_graph::DependencyGraphResult;
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
    /// ---- M2 Code Understanding ----
    /// 最近一次 inspect 的任务 ID。
    pub inspect_task_id: Option<TaskId>,
    /// inspect 取消令牌（独立于扫描令牌）。
    pub inspect_token: CancellationToken,
    /// inspect 状态：None = 尚未执行。
    pub inspect_status: Option<TaskState>,
    /// inspect 实时进度。
    pub inspect_progress: crate::state::InspectProgress,
    /// inspect 完成后的 Code Model。
    pub code_model: Option<ProjectCodeModel>,
    /// inspect Failed 时的错误描述。
    pub inspect_error: Option<String>,
    /// ---- M3 Dependency & Architecture ----
    /// graph 取消令牌。
    pub graph_token: CancellationToken,
    /// graph 状态：None = 尚未执行。
    pub graph_status: Option<TaskState>,
    /// graph 完成后的结果。
    pub graph_model: Option<DependencyGraphResult>,
    /// graph Failed 时的错误描述。
    pub graph_error: Option<String>,
    /// ---- M6 Impact ----
    /// impact 取消令牌（独立于其他任务）。
    pub impact_token: CancellationToken,
    /// impact 状态：None = 尚未执行。
    pub impact_status: Option<TaskState>,
    /// 最近一次完成的 ImpactAnalysis。
    pub impact_model: Option<prism_impact::ImpactAnalysis>,
    /// impact Failed 时的错误描述。
    pub impact_error: Option<String>,
    /// ---- M7 Health ----
    /// health 取消令牌（独立于其他任务）。
    pub health_token: CancellationToken,
    /// health 状态：None = 尚未执行。
    pub health_status: Option<TaskState>,
    /// 最近一次完成的 HealthAnalysis。
    pub health_model: Option<prism_health::HealthAnalysis>,
    /// health Failed 时的错误描述。
    pub health_error: Option<String>,
    /// ---- M5 ChangeSet ----
    /// changeset 取消令牌（独立于扫描/inspect/graph）。
    pub changeset_token: CancellationToken,
    /// changeset 状态：None = 尚未执行。
    pub changeset_status: Option<TaskState>,
    /// 最近一次完成的 ChangeSet。
    pub changeset_model: Option<prism_changes::ChangeSet>,
    /// changeset Failed 时的错误描述。
    pub changeset_error: Option<String>,
}

/// inspect 的实时计数（真实统计）。
#[derive(Debug, Clone, Copy, Default)]
pub struct InspectProgress {
    pub files_total: u64,
    pub files_done: u64,
    pub symbols: u64,
    pub errors: u64,
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
        // 新扫描使既有代码模型、依赖图与变更集过期。
        self.inspect_task_id = None;
        self.changeset_status = None;
        self.changeset_model = None;
        self.changeset_error = None;
        self.impact_status = None;
        self.impact_model = None;
        self.impact_error = None;
        self.health_status = None;
        self.health_model = None;
        self.health_error = None;
        self.inspect_status = None;
        self.inspect_progress = InspectProgress::default();
        self.code_model = None;
        self.inspect_error = None;
        self.graph_status = None;
        self.graph_model = None;
        self.graph_error = None;
        task_id
    }

    /// 准备一次 graph 分析。
    pub fn begin_graph(&mut self) -> TaskId {
        let task_id = TaskId::new();
        self.graph_token = CancellationToken::new();
        self.graph_status = Some(TaskState::Running);
        self.graph_model = None;
        self.graph_error = None;
        task_id
    }

    /// 准备一次 health 分析。
    pub fn begin_health(&mut self) {
        self.health_token = CancellationToken::new();
        self.health_status = Some(TaskState::Running);
        self.health_model = None;
        self.health_error = None;
    }

    /// 准备一次 impact 分析。
    pub fn begin_impact(&mut self) {
        self.impact_token = CancellationToken::new();
        self.impact_status = Some(TaskState::Running);
        self.impact_model = None;
        self.impact_error = None;
    }

    /// 准备一次 changeset 分析。
    pub fn begin_changeset(&mut self) {
        self.changeset_token = CancellationToken::new();
        self.changeset_status = Some(TaskState::Running);
        self.changeset_model = None;
        self.changeset_error = None;
    }

    /// 准备一次 inspect（生成新 token 与 task id）。
    pub fn begin_inspect(&mut self) -> TaskId {
        let task_id = TaskId::new();
        self.inspect_token = CancellationToken::new();
        self.inspect_task_id = Some(task_id.clone());
        self.inspect_status = Some(TaskState::Running);
        self.inspect_progress = InspectProgress::default();
        self.code_model = None;
        self.inspect_error = None;
        // inspect 完成后 graph 也会重新链式执行。
        self.graph_status = None;
        self.graph_model = None;
        self.graph_error = None;
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
