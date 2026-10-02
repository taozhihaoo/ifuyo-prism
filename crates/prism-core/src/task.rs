//! 任务基础抽象：状态、执行记录、取消令牌与最小任务登记表。
//!
//! M0 只建立足够的抽象，让 M1+（扫描、解析、Git、健康检查……）可以安全使用；
//! 不实现复杂任务调度器。所有耗时操作都应：
//!
//! 1. 产出 [`TaskRecord`]（可查询状态）；
//! 2. 接受可选的 [`CancellationToken`]；
//! 3. 通过 [`crate::progress::ProgressEvent`] 报告进度。

use crate::error::{PrismError, PrismResult};
use crate::ids::TaskId;
use crate::progress::Progress;
use crate::timestamp::Timestamp;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};

/// 任务状态。JSON 为 snake_case 字符串。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskState {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// 一条任务的执行记录（事实记录，不是调度器）。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TaskRecord {
    pub id: TaskId,
    /// 任务类型的通用标签（如 "scan" / "verify"），不绑定具体业务。
    pub kind: String,
    pub state: TaskState,
    pub message: Option<String>,
    pub progress: Option<Progress>,
    pub started_at: Option<Timestamp>,
    pub finished_at: Option<Timestamp>,
}

impl TaskRecord {
    pub fn queued(kind: impl Into<String>) -> Self {
        TaskRecord {
            id: TaskId::new(),
            kind: kind.into(),
            state: TaskState::Queued,
            message: None,
            progress: None,
            started_at: None,
            finished_at: None,
        }
    }
}

/// 取消令牌。M0 用原子布尔实现；未来异步运行时可替换实现而不改变语义。
#[derive(Debug, Clone, Default)]
pub struct CancellationToken {
    cancelled: Arc<std::sync::atomic::AtomicBool>,
}

impl CancellationToken {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.cancelled
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// 已取消则返回 `Err(Cancelled)`，否则 `Ok(())`。供长任务在循环中检查。
    pub fn check(&self) -> PrismResult<()> {
        if self.is_cancelled() {
            Err(PrismError::Cancelled)
        } else {
            Ok(())
        }
    }
}

/// 最小任务登记表：记录任务状态供 GUI / CLI 查询。
/// 内存态，不做持久化；跨会话的历史属于 M11（Snapshot）。
#[derive(Debug, Default)]
pub struct TaskTracker {
    tasks: Mutex<HashMap<TaskId, TaskRecord>>,
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    // Mutex 中只存可恢复的数据结构；poison 时取回内部数据继续工作，
    // 不在业务路径引入 panic。
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

impl TaskTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// 登记一个 Queued 任务。
    pub fn register(&self, kind: impl Into<String>) -> TaskRecord {
        let record = TaskRecord::queued(kind);
        lock(&self.tasks).insert(record.id.clone(), record.clone());
        record
    }

    pub fn mark_running(&self, id: &TaskId) {
        self.transition(id, TaskState::Running, |record| {
            record.started_at.get_or_insert_with(Timestamp::now_utc);
        });
    }

    pub fn mark_completed(&self, id: &TaskId) {
        self.transition(id, TaskState::Completed, |record| {
            record.finished_at = Some(Timestamp::now_utc());
            record.progress = None;
        });
    }

    pub fn mark_failed(&self, id: &TaskId, message: impl Into<String>) {
        self.transition(id, TaskState::Failed, |record| {
            record.message = Some(message.into());
            record.finished_at = Some(Timestamp::now_utc());
        });
    }

    pub fn mark_cancelled(&self, id: &TaskId) {
        self.transition(id, TaskState::Cancelled, |record| {
            record.finished_at = Some(Timestamp::now_utc());
        });
    }

    pub fn set_message(&self, id: &TaskId, message: impl Into<String>) {
        self.transition(id, TaskState::Running, |record| {
            record.message = Some(message.into());
        });
    }

    pub fn update_progress(&self, id: &TaskId, current: u64, total: Option<u64>) {
        self.transition(id, TaskState::Running, |record| {
            record.progress = Some(Progress { current, total });
        });
    }

    pub fn get(&self, id: &TaskId) -> Option<TaskRecord> {
        lock(&self.tasks).get(id).cloned()
    }

    /// 全部任务，按登记先后（started_at / id）稳定排序。
    pub fn list(&self) -> Vec<TaskRecord> {
        let mut all: Vec<TaskRecord> = lock(&self.tasks).values().cloned().collect();
        all.sort_by(|a, b| {
            a.started_at
                .cmp(&b.started_at)
                .then_with(|| a.id.cmp(&b.id))
        });
        all
    }

    fn transition(&self, id: &TaskId, state: TaskState, mutate: impl FnOnce(&mut TaskRecord)) {
        let mut tasks = lock(&self.tasks);
        if let Some(record) = tasks.get_mut(id) {
            record.state = state;
            mutate(record);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_record_starts_queued() {
        let record = TaskRecord::queued("scan");
        assert_eq!(record.state, TaskState::Queued);
        assert!(record.started_at.is_none());
    }

    #[test]
    fn task_state_serializes_snake_case() {
        assert_eq!(
            serde_json::to_string(&TaskState::Queued).unwrap(),
            "\"queued\""
        );
        assert_eq!(
            serde_json::to_string(&TaskState::Running).unwrap(),
            "\"running\""
        );
        let back: TaskState = serde_json::from_str("\"running\"").unwrap();
        assert_eq!(back, TaskState::Running);
    }

    #[test]
    fn cancellation_token_lifecycle() {
        let token = CancellationToken::new();
        assert!(!token.is_cancelled());
        assert!(token.check().is_ok());
        token.cancel();
        assert!(token.is_cancelled());
        let err = token.check().unwrap_err();
        assert_eq!(err.code(), crate::error::PrismErrorCode::Cancelled);
        let clone = token.clone();
        assert!(clone.is_cancelled(), "clone shares state");
    }

    #[test]
    fn tracker_transitions() {
        let tracker = TaskTracker::new();
        let record = tracker.register("unit-test");
        tracker.mark_running(&record.id);
        tracker.update_progress(&record.id, 3, Some(10));
        tracker.set_message(&record.id, "working");

        let running = tracker.get(&record.id).unwrap();
        assert_eq!(running.state, TaskState::Running);
        assert_eq!(running.progress.as_ref().unwrap().current, 3);
        assert_eq!(running.message.as_deref(), Some("working"));
        assert!(running.started_at.is_some());

        tracker.mark_completed(&record.id);
        let done = tracker.get(&record.id).unwrap();
        assert_eq!(done.state, TaskState::Completed);
        assert!(done.finished_at.is_some());
        assert!(done.progress.is_none());
    }

    #[test]
    fn tracker_records_failures() {
        let tracker = TaskTracker::new();
        let record = tracker.register("unit-test");
        tracker.mark_failed(&record.id, "boom");
        let failed = tracker.get(&record.id).unwrap();
        assert_eq!(failed.state, TaskState::Failed);
        assert_eq!(failed.message.as_deref(), Some("boom"));
    }

    #[test]
    fn list_is_deterministic() {
        let tracker = TaskTracker::new();
        let a = tracker.register("one");
        let b = tracker.register("two");
        let first = tracker.list();
        let second = tracker.list();
        assert_eq!(first.len(), 2);
        // 同一状态下多次 list 结果完全一致（排序稳定）。
        assert_eq!(first, second);
        // 包含全部任务（顺序由 id 排序决定，不依赖登记顺序）。
        let ids: Vec<&str> = first.iter().map(|t| t.id.as_str()).collect();
        assert!(ids.contains(&a.id.as_str()));
        assert!(ids.contains(&b.id.as_str()));
    }
}
