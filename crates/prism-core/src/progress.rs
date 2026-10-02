//! 统一进度模型。
//!
//! `ProgressEvent` 保持通用（task_id + current/total + message），
//! 不绑定“扫描文件”这类具体语义——Git / Verify / Context 未来都要复用。

use crate::ids::TaskId;

/// 进度快照（任务记录内嵌）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Progress {
    pub current: u64,
    pub total: Option<u64>,
}

impl Progress {
    pub fn percentage(&self) -> Option<f64> {
        self.total
            .filter(|total| *total > 0)
            .map(|total| (self.current as f64 / total as f64) * 100.0)
    }
}

/// 一次进度上报事件（对外 JSON 契约的一部分）。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProgressEvent {
    pub task_id: TaskId,
    pub current: u64,
    pub total: Option<u64>,
    pub message: Option<String>,
}

impl ProgressEvent {
    pub fn percentage(&self) -> Option<f64> {
        self.total
            .filter(|total| *total > 0)
            .map(|total| (self.current as f64 / total as f64) * 100.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percentage_computes_from_total() {
        let progress = Progress {
            current: 25,
            total: Some(100),
        };
        assert!((progress.percentage().unwrap() - 25.0).abs() < f64::EPSILON);
    }

    #[test]
    fn percentage_is_none_without_total() {
        let progress = Progress {
            current: 25,
            total: None,
        };
        assert!(progress.percentage().is_none());
    }

    #[test]
    fn percentage_is_none_for_zero_total() {
        let progress = Progress {
            current: 0,
            total: Some(0),
        };
        assert!(progress.percentage().is_none());
    }

    #[test]
    fn event_serializes_with_snake_case_fields() {
        let event = ProgressEvent {
            task_id: TaskId::from_string("task-1"),
            current: 1,
            total: Some(2),
            message: Some("halfway".into()),
        };
        let json = serde_json::to_value(&event).unwrap();
        assert!(json.get("task_id").is_some());
        assert!(json.get("total").is_some());
    }
}
