//! prism-core — Prism 的核心基础层。
//!
//! 本 crate 承载 M0 建立的工程地基：
//!
//! - `domain` 语义的基础类型：统一 ID、Severity/Confidence、时间戳、Project 骨架
//! - 统一错误模型（[`error::PrismError`]）
//! - 统一配置系统（[`config`]）
//! - 统一日志（[`logging`]）
//! - 任务 / 取消 / 进度基础抽象（[`task`]、[`progress`]）
//! - 命令执行与文件系统抽象（[`runtime`]）
//! - Application 层（[`application`]）：GUI（Tauri）与 CLI 共享的服务边界
//!
//! ## 依赖方向
//!
//! prism-core 不依赖 Tauri、前端、具体解析器或任何具体外部工具；
//! GUI 与 CLI 必须经由 Application 层访问 Core，禁止反向依赖。
//!
//! ## JSON 契约
//!
//! 对外序列化统一使用 snake_case 字段名；时间戳一律 RFC3339 UTC。

pub mod application;
pub mod config;
pub mod error;
pub mod ids;
pub mod logging;
pub mod progress;
pub mod runtime;
pub mod severity;
pub mod task;
pub mod timestamp;

pub use error::{PrismError, PrismErrorCode, PrismErrorDto, PrismResult};
pub use runtime::command::{CommandOutput, CommandRunner, CommandSpec};
pub use severity::{Confidence, PrismCompleteness, Severity};
pub use timestamp::Timestamp;
