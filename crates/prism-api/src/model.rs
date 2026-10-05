//! API/Webhook 数据模型（M213 §6-§11/§33-§35）。
//!
//! 数据边界（§71）：Runtime Observation（status/latency）与
//! Inference（likely handler）分离在不同字段；不混写。

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// M213 输出契约版本。
pub const API_SCHEMA_VERSION: u32 = 1;

/// HTTP 方法（§18 Method Allowlist 词表）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Head,
    Options,
}

impl HttpMethod {
    pub const fn as_str(self) -> &'static str {
        match self {
            HttpMethod::Get => "GET",
            HttpMethod::Post => "POST",
            HttpMethod::Put => "PUT",
            HttpMethod::Patch => "PATCH",
            HttpMethod::Delete => "DELETE",
            HttpMethod::Head => "HEAD",
            HttpMethod::Options => "OPTIONS",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw.to_ascii_uppercase().as_str() {
            "GET" => Some(HttpMethod::Get),
            "POST" => Some(HttpMethod::Post),
            "PUT" => Some(HttpMethod::Put),
            "PATCH" => Some(HttpMethod::Patch),
            "DELETE" => Some(HttpMethod::Delete),
            "HEAD" => Some(HttpMethod::Head),
            "OPTIONS" => Some(HttpMethod::Options),
            _ => None,
        }
    }

    /// Replay 安全分级（§31）：GET/HEAD/OPTIONS 默认 ReplaySafe；
    /// POST/PUT/PATCH PotentiallyUnsafe；DELETE Blocked 默认拒绝。
    pub const fn replay_safety(self) -> ReplaySafety {
        match self {
            HttpMethod::Get | HttpMethod::Head | HttpMethod::Options => ReplaySafety::Safe,
            HttpMethod::Post | HttpMethod::Put | HttpMethod::Patch => {
                ReplaySafety::PotentiallyUnsafe
            }
            HttpMethod::Delete => ReplaySafety::Blocked,
        }
    }
}

/// Replay 安全分级（§31）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReplaySafety {
    Safe,
    PotentiallyUnsafe,
    Blocked,
    Unknown,
}

/// 端点来源（§16：必须标记 Source）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EndpointSource {
    SourceCode,
    Config,
    Runtime,
    OpenApi,
    UserInput,
}

/// API Endpoint（§6.1/§7）：身份 = method + 归一化 route template
/// （**不是**完整 URL + 时间戳，§7）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiEndpoint {
    /// `api:{METHOD} {normalized_route}`。
    pub endpoint_id: String,
    pub method: HttpMethod,
    /// 归一化模板（`/api/cards/{id}`；具体 id 不是身份的一部分）。
    pub route_template: String,
    pub source: EndpointSource,
    /// 证据定位（源文件:行）。
    pub evidence: String,
    /// 推断的 handler 符号（INFERENCE；有 M23 证据才填，§24）。
    pub handler_symbol: Option<String>,
}

/// 失败种类（§9：不能全部统一成 API Failed）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureKind {
    None,
    /// DNS/连接拒绝/TLS 等传输层失败。
    Transport,
    /// HTTP 4xx/5xx。
    HttpStatus,
    /// 响应体解析失败。
    Parsing,
    Timeout,
    Cancelled,
    Unknown,
}

/// 一次 API 观测（§8/§9 合并视图；请求/响应事实分离字段）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiObservation {
    /// `obs:{fnv1a16(method|url|observed_at)}`——稳定身份（§60）。
    pub observation_id: String,
    pub method: HttpMethod,
    pub url: String,
    /// 匹配到的端点模板（INFERENCE，有匹配才填，§24）。
    pub matched_endpoint: Option<String>,
    pub request_headers: BTreeMap<String, String>,
    /// redacted 后的请求体（§12）。
    pub request_body: Option<String>,
    pub status: Option<u16>,
    pub response_headers: BTreeMap<String, String>,
    /// redacted 后的响应体（可能截断 §47）。
    pub response_body: Option<String>,
    pub response_truncated: bool,
    pub latency_ms: u64,
    pub failure: FailureKind,
    /// 观测时间（RFC3339）。
    pub observed_at: String,
    pub correlation_id: Option<String>,
}

/// Webhook 签名状态（§11：六态，绝不是 IsValid=false 二元）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignatureState {
    NotPresent,
    Present,
    Validated,
    Invalid,
    Unsupported,
    Unknown,
}

/// Webhook 处理状态（§10）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcessingState {
    Received,
    Parsed,
    Rejected,
    Succeeded,
    Failed,
    TimedOut,
    Duplicate,
    Unknown,
}

/// Webhook 事件（§10）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WebhookEvent {
    /// `wh:{fnv1a16(event_id|received_at)}`。
    pub event_record_id: String,
    /// 上游事件 id（header/body 内，可能缺失）。
    pub event_id: Option<String>,
    pub event_type: Option<String>,
    /// redacted payload（可能截断）。
    pub payload: Option<String>,
    pub payload_truncated: bool,
    pub signature: SignatureState,
    pub processing: ProcessingState,
    pub received_at: String,
    pub correlation_id: Option<String>,
    /// 推断的 handler 符号（INFERENCE）。
    pub handler_symbol: Option<String>,
}

/// API Debug Session（§33：关联一次观测集合；不变成新数据库世界）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiDebugSession {
    pub session_id: String,
    pub started_at: String,
    pub observation_count: u64,
    pub webhook_count: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replay_safety_ladder() {
        // §31：GET 默认安全；POST/PUT/PATCH 潜在非安全；DELETE 默认拒绝。
        assert_eq!(HttpMethod::Get.replay_safety(), ReplaySafety::Safe);
        assert_eq!(
            HttpMethod::Post.replay_safety(),
            ReplaySafety::PotentiallyUnsafe
        );
        assert_eq!(HttpMethod::Delete.replay_safety(), ReplaySafety::Blocked);
    }

    #[test]
    fn endpoint_identity_is_template_not_url() {
        // §7：GET /api/cards/{id} 是身份；具体 URL 不是。
        let endpoint = ApiEndpoint {
            endpoint_id: "api:GET /api/cards/{id}".to_string(),
            method: HttpMethod::Get,
            route_template: "/api/cards/{id}".to_string(),
            source: EndpointSource::SourceCode,
            evidence: "Controllers/CardsController.cs:14".to_string(),
            handler_symbol: None,
        };
        assert!(endpoint.endpoint_id.starts_with("api:GET /api/cards/"));
        assert!(!endpoint.endpoint_id.contains("127.0.0.1"));
    }
}
