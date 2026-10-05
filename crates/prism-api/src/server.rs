//! 本地 Webhook Receiver（M213 §21/§22/§63）。
//!
//! std::net 最小 HTTP 服务：**显式**绑定 127.0.0.1:<port>（永不 0.0.0.0）；
//! 有界 payload/事件数；HMAC-SHA256 签名校验（secret 显式提供）；状态机
//! Received→Parsed→(Validated|Rejected)→Succeeded/Failed；全链 redaction。

use crate::model::{ProcessingState, SignatureState, WebhookEvent};
use crate::signature::hmac_sha256_hex;
use prism_core::task::CancellationToken;
use prism_intel::identity::fnv1a_hex16;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// 接收器配置（§21 全部上限显式）。
#[derive(Debug, Clone)]
pub struct WebhookReceiverConfig {
    /// 绑定端口（bind 地址恒为 127.0.0.1，§21 永不 0.0.0.0）。
    pub port: u16,
    /// HMAC-SHA256 共享密钥（Some ⇒ 校验；None ⇒ 签名状态 Unknown，§11）。
    pub secret: Option<String>,
    /// 签名 header 名（如 X-Hub-Signature-256 / Stripe-Signature）。
    pub signature_header: String,
    pub max_payload_bytes: usize,
    pub max_events: usize,
    /// 监听最长时长（毫秒；超时自动停止 §21 Timeout）。
    pub listen_timeout_ms: u64,
}

impl Default for WebhookReceiverConfig {
    fn default() -> Self {
        WebhookReceiverConfig {
            port: 8085,
            secret: None,
            signature_header: "X-Hub-Signature-256".to_string(),
            max_payload_bytes: 256 * 1024,
            max_events: 100,
            listen_timeout_ms: 30_000,
        }
    }
}

/// 接收器：start → events 累积 → stop（显式生命周期 §22）。
pub struct WebhookReceiver {
    shutdown: Arc<AtomicBool>,
    port: u16,
    handle: Option<std::thread::JoinHandle<()>>,
    events: Arc<Mutex<Vec<WebhookEvent>>>,
}

impl WebhookReceiver {
    /// 显式启动（§21 Explicit Start；绑定 127.0.0.1）。
    pub fn start(
        config: WebhookReceiverConfig,
        cancel: &CancellationToken,
    ) -> std::io::Result<Self> {
        let listener = TcpListener::bind(("127.0.0.1", config.port))?;
        listener.set_nonblocking(true)?;
        let port = listener.local_addr()?.port();
        let shutdown = Arc::new(AtomicBool::new(false));
        let events: Arc<Mutex<Vec<WebhookEvent>>> = Arc::new(Mutex::new(Vec::new()));
        let worker_events = events.clone();
        let worker_shutdown = shutdown.clone();
        let worker_cancel = cancel.clone();
        let started = Instant::now();
        let handle = std::thread::spawn(move || {
            while !worker_shutdown.load(Ordering::Relaxed)
                && !worker_cancel.is_cancelled()
                && worker_events.lock().map(|events| events.len()).unwrap_or(0) < config.max_events
                && started.elapsed() < Duration::from_millis(config.listen_timeout_ms)
            {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        let event = receive_one(&mut stream, &config);
                        if let Ok(mut events) = worker_events.lock() {
                            events.push(event);
                        }
                    }
                    Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(20))
                    }
                    Err(_) => std::thread::sleep(Duration::from_millis(20)),
                }
            }
        });
        Ok(WebhookReceiver {
            shutdown,
            port,
            handle: Some(handle),
            events,
        })
    }

    /// 显式停止（§21 Explicit Stop）。
    pub fn stop(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            handle.join().ok();
        }
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    /// 是否仍在监听（未被 stop/cancel/timeout 停止）。
    pub fn is_running(&self) -> bool {
        !self.shutdown.load(Ordering::Relaxed) && self.handle.is_some()
    }

    pub fn events(&self) -> Vec<WebhookEvent> {
        self.events
            .lock()
            .map(|events| events.clone())
            .unwrap_or_default()
    }
}

impl Drop for WebhookReceiver {
    fn drop(&mut self) {
        self.stop();
    }
}

/// 接收单个连接 → WebhookEvent（§22 生命周期前半：Capture→Parse→Signature）。
fn receive_one(stream: &mut std::net::TcpStream, config: &WebhookReceiverConfig) -> WebhookEvent {
    // 关键：accepted stream 继承 listener 的 nonblocking（§21——listener
    // 必须 nonblocking 才能 poll 关闭）；receive 的 read 需要 blocking
    // 语义，否则无数据时立即 WouldBlock => 假 EOF => 丢请求。
    stream.set_nonblocking(false).ok();
    stream.set_read_timeout(Some(Duration::from_secs(5))).ok();
    let received_at = prism_core::Timestamp::now_utc().to_string();
    let mut buffer = Vec::new();
    let mut chunk = [0u8; 4096];
    let mut payload = String::new();
    let mut signature_state = SignatureState::Unknown;
    let mut event_type: Option<String> = None;
    let mut correlation_id: Option<String> = None;
    let mut processing: Option<ProcessingState> = None;
    loop {
        match stream.read(&mut chunk) {
            Ok(0) => break,
            Ok(count) => {
                buffer.extend_from_slice(&chunk[..count]);
                if buffer.len() > config.max_payload_bytes {
                    // §47：超限 ⇒ 截断 + Rejected（不静默）。
                    processing = Some(ProcessingState::Rejected);
                    break;
                }
            }
            Err(_) => break,
        }
        if let Ok(text) = std::str::from_utf8(&buffer) {
            if let Some(header_end) = text.find("\r\n\r\n") {
                let body = &text[header_end + 4..];
                if body.len() >= expected_content_length(text) {
                    payload = body.to_string();
                    let (parsed_type, parsed_correlation) = parse_event_meta(text);
                    event_type = parsed_type;
                    correlation_id = parsed_correlation;
                    // 签名状态（§11 六态）。
                    let header_signature = header_value(text, &config.signature_header);
                    signature_state = match (
                        config.secret.as_deref(),
                        if header_signature.is_empty() {
                            None
                        } else {
                            Some(header_signature.as_str())
                        },
                    ) {
                        (None, _) => SignatureState::Unknown,
                        (Some(_), None) => SignatureState::NotPresent,
                        (Some(secret), Some(provided)) => {
                            let expected = hmac_sha256_hex(secret.as_bytes(), body.as_bytes());
                            // 常见 `sha256=<hex>` 前缀兼容。
                            let provided_hex = provided
                                .split_once('=')
                                .map(|(_, hex)| hex.trim().to_string())
                                .unwrap_or_else(|| provided.trim().to_string());
                            if constant_time_eq(expected.as_bytes(), provided_hex.as_bytes()) {
                                SignatureState::Validated
                            } else {
                                SignatureState::Invalid
                            }
                        }
                    };
                    processing = Some(match signature_state {
                        SignatureState::Invalid | SignatureState::NotPresent => {
                            ProcessingState::Rejected
                        }
                        _ => ProcessingState::Succeeded,
                    });
                    break;
                }
            }
        }
    }
    // 处理失败（Rejected）仍返回 200 之外的语义由调用方决定——这里回 500
    // 表示 Rejected（§22 Processing Result 有真实输出）。
    let status_line = if processing == Some(ProcessingState::Rejected) {
        "HTTP/1.1 500 REJECTED\r\nContent-Length: 0\r\n\r\n"
    } else {
        "HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n"
    };
    let _ = stream.write_all(status_line.as_bytes());
    let _ = stream.flush();
    let event_id = header_value(
        if buffer.is_empty() {
            ""
        } else {
            std::str::from_utf8(&buffer).unwrap_or_default()
        },
        "X-Event-Id",
    );
    WebhookEvent {
        event_record_id: format!("wh:{}", fnv1a_hex16(&format!("{event_id}|{received_at}"))),
        event_id: if event_id.is_empty() {
            None
        } else {
            Some(event_id)
        },
        event_type,
        payload: if payload.is_empty() {
            None
        } else {
            let (redacted, _) = prism_supply::secrets::redact_line(&payload);
            Some(redacted)
        },
        payload_truncated: processing == Some(ProcessingState::Rejected),
        signature: signature_state,
        processing: processing.unwrap_or(ProcessingState::Unknown),
        received_at,
        correlation_id,
        handler_symbol: None,
    }
}

fn expected_content_length(request_text: &str) -> usize {
    header_value(request_text, "Content-Length")
        .parse()
        .unwrap_or(0)
}

fn header_value(request_text: &str, name: &str) -> String {
    request_text
        .lines()
        .find_map(|line| {
            let (header_name, value) = line.split_once(':')?;
            if header_name.trim().eq_ignore_ascii_case(name) {
                Some(value.trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_default()
}

/// 从请求文本提取 event type / correlation id（X-Event-Type / X-Correlation-Id）。
fn parse_event_meta(request_text: &str) -> (Option<String>, Option<String>) {
    (
        Some(header_value(request_text, "X-Event-Type")).filter(|value| !value.is_empty()),
        Some(header_value(request_text, "X-Correlation-Id")).filter(|value| !value.is_empty()),
    )
}

/// 常量时间比较（防时序侧信道，§11）。
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (byte_a, byte_b) in a.iter().zip(b.iter()) {
        diff |= byte_a ^ byte_b;
    }
    diff == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn post(
        port: u16,
        path: &str,
        headers: &[(&str, &str)],
        body: &str,
    ) -> std::io::Result<std::net::TcpStream> {
        let mut stream = std::net::TcpStream::connect(("127.0.0.1", port))?;
        let mut request = format!(
            "POST {path} HTTP/1.1\r\nHost: localhost\r\nContent-Length: {}\r\n",
            body.len()
        );
        for (name, value) in headers {
            request.push_str(&format!("{name}: {value}\r\n"));
        }
        request.push_str("\r\n");
        request.push_str(body);
        stream.write_all(request.as_bytes())?;
        stream.flush()?;
        std::thread::sleep(Duration::from_millis(80));
        Ok(stream)
    }

    #[test]
    fn receiver_validates_hmac_signature_states() {
        let cancel = CancellationToken::new();
        let config = WebhookReceiverConfig {
            port: 0, // OS 分配（测试）；生产 CLI 显式端口
            secret: Some("whsec_test".to_string()),
            signature_header: "X-Hub-Signature-256".to_string(),
            ..WebhookReceiverConfig::default()
        };
        let mut receiver = WebhookReceiver::start(config, &cancel).expect("bind");
        let port = receiver.port();
        let expected = hmac_sha256_hex(b"whsec_test", b"{\"type\":\"payment\"}");
        // Validated。
        post(
            port,
            "/webhook/payment",
            &[
                ("X-Hub-Signature-256", &format!("sha256={expected}")),
                ("X-Event-Type", "payment.updated"),
            ],
            "{\"type\":\"payment\"}",
        )
        .ok();
        // Invalid。
        post(
            port,
            "/webhook/payment",
            &[("X-Hub-Signature-256", "sha256=deadbeef")],
            "{\"type\":\"payment\"}",
        )
        .ok();
        // NotPresent。
        post(port, "/webhook/payment", &[], "{\"type\":\"payment\"}").ok();
        std::thread::sleep(Duration::from_millis(150));
        receiver.stop();
        let events = receiver.events();
        assert_eq!(events.len(), 3);
        assert_eq!(events[0].signature, SignatureState::Validated);
        assert_eq!(events[0].processing, ProcessingState::Succeeded);
        assert_eq!(events[0].event_type.as_deref(), Some("payment.updated"));
        assert_eq!(events[1].signature, SignatureState::Invalid);
        assert_eq!(events[1].processing, ProcessingState::Rejected);
        assert_eq!(events[2].signature, SignatureState::NotPresent);
    }

    #[test]
    fn no_secret_means_signature_unknown() {
        // §11：验证器不可用 ⇒ Unknown，不是 Invalid。
        let cancel = CancellationToken::new();
        let config = WebhookReceiverConfig {
            port: 0,
            secret: None,
            ..WebhookReceiverConfig::default()
        };
        let mut receiver = WebhookReceiver::start(config, &cancel).expect("bind");
        post(
            receiver.port(),
            "/webhook/x",
            &[("X-Hub-Signature-256", "sha256=abc")],
            "{}",
        )
        .ok();
        std::thread::sleep(Duration::from_millis(150));
        receiver.stop();
        let events = receiver.events();
        assert_eq!(events[0].signature, SignatureState::Unknown);
        assert_eq!(events[0].processing, ProcessingState::Succeeded);
    }

    #[test]
    fn payload_is_redacted_before_storage() {
        // TEST FIXTURE ONLY。
        let cancel = CancellationToken::new();
        let secret = hmac_sha256_hex(b"whsec_test", b"{\"key\":\"AKIAABCDEFGHIJKLMNOP\"}");
        let config = WebhookReceiverConfig {
            port: 0,
            secret: Some("whsec_test".to_string()),
            ..WebhookReceiverConfig::default()
        };
        let mut receiver = WebhookReceiver::start(config, &cancel).expect("bind");
        post(
            receiver.port(),
            "/webhook/x",
            &[("X-Hub-Signature-256", &format!("sha256={secret}"))],
            "{\"key\":\"AKIAABCDEFGHIJKLMNOP\"}",
        )
        .ok();
        std::thread::sleep(Duration::from_millis(150));
        receiver.stop();
        let events = receiver.events();
        let rendered = format!("{:?}", events);
        assert!(!rendered.contains("ABCDEFGHIJKLMNOP"), "§12 原文不得存储");
    }
}
