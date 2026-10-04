//! AnalysisContext 与结果封套（2.0-M21 §22–§23/§26–§28）。
//!
//! AnalysisContext 是运行期对象（含 CancellationToken，不序列化）；
//! Core Error 复用 prism_core::PrismError（映射表见 INTELLIGENCE_CORE 文档），
//! Diagnostic（项目的问题）与 Error（Prism 自己失败）分属 Finding / Error 两轴。

use crate::identity::ProjectIdentity;
use prism_core::task::CancellationToken;
use serde::{Deserialize, Serialize};

/// 分析范围（M21 §22 analysis scope）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AnalysisScope {
    FullProject,
    /// 指定项目相对路径集合。
    Paths(Vec<String>),
    /// 单一能力域（如 "dependency"）。
    Domain(String),
}

/// 统一分析上下文（M21 §22）：一次分析的完整描述。
/// 运行期对象——持有取消令牌，不序列化、不入库（§42 transient）。
pub struct AnalysisContext {
    pub project: ProjectIdentity,
    pub scope: AnalysisScope,
    /// 本次请求的能力（如 ["dependency_graph"]；供未来 adapter 调度）。
    pub requested_capabilities: Vec<String>,
    pub observed_at: String,
    pub cancel: CancellationToken,
}

impl AnalysisContext {
    pub fn new(
        project: ProjectIdentity,
        scope: AnalysisScope,
        requested_capabilities: Vec<String>,
        cancel: CancellationToken,
    ) -> Self {
        AnalysisContext {
            project,
            scope,
            requested_capabilities,
            observed_at: prism_core::Timestamp::now_utc().to_string(),
            cancel,
        }
    }
}

/// 操作状态（M21 §23）：partial 不得伪装成 success。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntelStatus {
    Complete,
    Partial,
    Failed,
    Cancelled,
    Unknown,
}

/// 统一结果封套（M21 §26）：success/partial/failure/cancelled + data/warnings/errors。
/// 旧 service 逐步迁移；新 Core 服务一律使用。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IntelOutcome<T> {
    pub status: IntelStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<String>,
}

impl<T> IntelOutcome<T> {
    pub fn complete(data: T) -> Self {
        IntelOutcome {
            status: IntelStatus::Complete,
            data: Some(data),
            warnings: Vec::new(),
            errors: Vec::new(),
        }
    }

    /// 部分成功：数据 + 未完成原因（诚实降级，沿 collect_analysis 先例）。
    pub fn partial(data: T, warnings: Vec<String>) -> Self {
        IntelOutcome {
            status: IntelStatus::Partial,
            data: Some(data),
            warnings,
            errors: Vec::new(),
        }
    }

    pub fn failed(errors: Vec<String>) -> Self {
        IntelOutcome {
            status: IntelStatus::Failed,
            data: None,
            warnings: Vec::new(),
            errors,
        }
    }

    /// 取消：保留已完成部分（§23/§44），状态如实为 Cancelled。
    pub fn cancelled(data: Option<T>, warnings: Vec<String>) -> Self {
        IntelOutcome {
            status: IntelStatus::Cancelled,
            data,
            warnings,
            errors: Vec::new(),
        }
    }

    pub fn map<U, F: FnOnce(T) -> U>(self, f: F) -> IntelOutcome<U> {
        IntelOutcome {
            status: self.status,
            data: self.data.map(f),
            warnings: self.warnings,
            errors: self.errors,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partial_is_never_complete() {
        let outcome: IntelOutcome<u32> = IntelOutcome::partial(80, vec!["20 failed".into()]);
        assert_ne!(outcome.status, IntelStatus::Complete);
        assert_eq!(outcome.data, Some(80));
        assert_eq!(outcome.warnings.len(), 1);
    }

    #[test]
    fn failed_carries_no_data() {
        let outcome: IntelOutcome<u32> = IntelOutcome::failed(vec!["db unavailable".into()]);
        assert_eq!(outcome.status, IntelStatus::Failed);
        assert!(outcome.data.is_none());
        // Error（Prism 自己失败）与 Diagnostic（项目的问题）边界：
        // 失败进 outcome.errors；项目问题走 Finding 模型。
        assert!(outcome.errors[0].contains("db unavailable"));
    }

    #[test]
    fn cancelled_keeps_partial_data() {
        let outcome: IntelOutcome<u32> =
            IntelOutcome::cancelled(Some(3), vec!["stopped early".into()]);
        assert_eq!(outcome.status, IntelStatus::Cancelled);
        assert_eq!(outcome.data, Some(3));
    }
}
