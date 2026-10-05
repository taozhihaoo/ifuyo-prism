//! # Prism API / Webhook Debugging（2.0-M213）
//!
//! 把一次 API / Webhook 运行事件作为**工程事实**纳入统一知识体系：
//! `Endpoint Discovery → Observation → Correlation(Log/Error/Symbol/Git)
//! → Finding`。
//!
//! 边界（§2/§3/§18-§20/§42）：
//! - **Network Safety 是 P0**：SSRF guard（scheme 白名单 + host 白名单
//!   localhost-first + 私网/metadata 拒绝 + 无重定向跟随）；无任意 shell；
//!   一切网络动作显式 opt-in。
//! - **Secret Safety 是 P0**（§12-§14）：header/body 全链复用 M29
//!   redaction；fingerprint 不含 secret 原文。
//! - **Unknown ≠ False**（§11/§67）：签名 NotPresent ≠ Invalid；验签器
//!   不可用 ⇒ Unknown。
//! - **零新重量依赖**：HTTP/1.1 客户端/接收器基于 std::net（https ⇒
//!   Capability::Unsupported 如实，P1 升级 reqwest）；HMAC-SHA256 按
//!   RFC 2104 手写于 `signature.rs`（sha2 仅有依赖，RFC 4231 向量测试）。
//! - **不重建事实体系**（§79/§80）：Observation/Finding/Correlation 全走
//!   M21/M28/M210 模型；红action 复用 M29。

pub mod http;
pub mod model;
pub mod redaction;
pub mod server;
pub mod signature;
pub mod ssrf;

pub use http::{observe, HttpObservation};
pub use model::{
    ApiEndpoint, ApiObservation, FailureKind, HttpMethod, SignatureState, WebhookEvent,
    API_SCHEMA_VERSION,
};
pub use redaction::redact_value;
pub use server::{WebhookReceiver, WebhookReceiverConfig};
pub use signature::hmac_sha256_hex;
pub use ssrf::{TargetGuard, TargetViolation};
