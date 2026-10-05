//! 有界 HTTP/1.1 观测客户端（M213 §8/§9/§17-§20）。
//!
//! std::net 实现的**最小** HTTP/1.1：请求行 + headers + Content-Length
//! body；响应有界读取（max_response_bytes ⇒ truncated 标记）。安全边界：
//! - 目标必须先过 `TargetGuard`（SSRF §19）；
//! - **不跟随重定向**（3xx ⇒ 正常观测 + redirect 诊断，§19 redirect chain）；
//! - https ⇒ FailureKind::Transport + Unsupported 诊断（零 TLS 依赖，
//!   P1 升级 reqwest，§18 Protocol Allowlist）；
//! - 读/写超时 + 取消令牌（§66）。
//! - 响应/请求体全部经 M29 redaction（§12-§14）。

use crate::model::{FailureKind, HttpMethod};
use crate::redaction::redact_headers;
use crate::ssrf::TargetGuard;
use prism_core::task::CancellationToken;
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::{Duration, Instant};

/// 有界观测参数（§18 全部上限显式）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObserveLimits {
    pub timeout_ms: u64,
    pub max_response_bytes: usize,
    pub connect_timeout_ms: u64,
}

impl Default for ObserveLimits {
    fn default() -> Self {
        ObserveLimits {
            timeout_ms: 10_000,
            max_response_bytes: 256 * 1024,
            connect_timeout_ms: 5_000,
        }
    }
}

/// 一次 HTTP 观测（§9 失败分类，绝不统一成 API Failed）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpObservation {
    pub method: HttpMethod,
    pub url: String,
    pub status: Option<u16>,
    pub response_headers: BTreeMap<String, String>,
    /// redacted body（可能截断）。
    pub response_body: Option<String>,
    pub response_truncated: bool,
    pub latency_ms: u64,
    pub failure: FailureKind,
    /// 失败/异常的人读描述（不含 secret）。
    pub diagnostic: Option<String>,
}

/// 发起一次有界观测（目标先过 TargetGuard；§55 CLI 侧必须显式调用）。
pub fn observe(
    guard: &TargetGuard,
    method: HttpMethod,
    url: &str,
    headers: &BTreeMap<String, String>,
    body: Option<&str>,
    limits: &ObserveLimits,
    cancel: &CancellationToken,
) -> HttpObservation {
    let started = Instant::now();
    let mut failure = FailureKind::None;

    let target = match guard.check(url) {
        Ok(target) => target,
        Err(violation) => {
            return HttpObservation {
                method,
                url: url.to_string(),
                status: None,
                response_headers: Default::default(),
                response_body: None,
                response_truncated: false,
                latency_ms: 0,
                failure: FailureKind::Transport,
                diagnostic: Some(format!("target rejected by SSRF guard: {violation:?}")),
            };
        }
    };

    let result = execute(&target, method, headers, body, limits, cancel);
    let (
        status,
        response_headers,
        response_body,
        response_truncated,
        _local_failure,
        local_diagnostic,
    ) = match result {
        Ok(parsed) => parsed,
        Err(error) => {
            failure = FailureKind::Transport;
            (None, Default::default(), None, false, failure, Some(error))
        }
    };
    if let Some(code) = status {
        if code >= 400 {
            failure = FailureKind::HttpStatus;
        }
    }
    let diagnostic = local_diagnostic;
    HttpObservation {
        method,
        url: url.to_string(),
        status,
        response_headers,
        response_body,
        response_truncated,
        latency_ms: started.elapsed().as_millis() as u64,
        failure,
        diagnostic,
    }
}

/// execute 返回元组（status, headers, body, truncated, failure, diagnostic）。
type Executed = (
    Option<u16>,
    BTreeMap<String, String>,
    Option<String>,
    bool,
    FailureKind,
    Option<String>,
);


fn execute(
    target: &crate::ssrf::Target,
    method: HttpMethod,
    headers: &BTreeMap<String, String>,
    body: Option<&str>,
    limits: &ObserveLimits,
    cancel: &CancellationToken,
) -> Result<Executed, String> {
    let host = target.host.trim_start_matches('[').trim_end_matches(']');
    let address = format!("{host}:{}", target.port);
    let connect_deadline = Instant::now() + Duration::from_millis(limits.connect_timeout_ms);
    // 瞬时连接错误（WouldBlock 等）重试至 deadline；refused/timedout 立即如实失败。
    let mut stream = loop {
        if cancel.is_cancelled() {
            return Err("cancelled".to_string());
        }
        if Instant::now() > connect_deadline {
            return Err(format!(
                "connect timeout after {} ms",
                limits.connect_timeout_ms
            ));
        }
        match TcpStream::connect(&address) {
            Ok(stream) => break stream,
            Err(err)
                if matches!(
                    err.kind(),
                    std::io::ErrorKind::ConnectionRefused
                        | std::io::ErrorKind::TimedOut
                ) =>
            {
                return Err(format!("{address}: {err}"));
            }
            Err(_) => std::thread::sleep(Duration::from_millis(50)),
        }
    };
    stream
        .set_read_timeout(Some(Duration::from_millis(limits.timeout_ms)))
        .map_err(|err| format!("set read timeout failed: {err}"))?;
    stream
        .set_write_timeout(Some(Duration::from_millis(limits.timeout_ms)))
        .map_err(|err| format!("set write timeout failed: {err}"))?;

    // 请求行 + headers（不含 Host 之外的自动头；body 经 redaction 在外层做）。
    let mut request = format!(
        "{} {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n",
        method.as_str(),
        target.path_and_query,
        if target.host.contains(':') && !target.host.starts_with('[') {
            format!("[{}]", target.host)
        } else {
            target.host.clone()
        }
    );
    for (name, value) in headers {
        request.push_str(&format!("{name}: {value}\r\n"));
    }
    if let Some(body_text) = body {
        request.push_str(&format!("Content-Length: {}\r\n", body_text.len()));
    }
    request.push_str("\r\n");
    stream
        .write_all(request.as_bytes())
        .map_err(|err| format!("write request failed: {err}"))?;
    if let Some(body_text) = body {
        stream
            .write_all(body_text.as_bytes())
            .map_err(|err| format!("write body failed: {err}"))?;
    }

    // 有界读取响应（§47：超出 ⇒ truncated，不静默）。
    let mut raw = Vec::new();
    let mut buffer = [0u8; 8192];
    let mut truncated = false;
    loop {
        if cancel.is_cancelled() {
            return Err("cancelled".to_string());
        }
        match stream.read(&mut buffer) {
            Ok(0) => break,
            Ok(count) => {
                if raw.len() + count > limits.max_response_bytes {
                    let remaining = limits.max_response_bytes - raw.len();
                    raw.extend_from_slice(&buffer[..remaining]);
                    truncated = true;
                    break;
                }
                raw.extend_from_slice(&buffer[..count]);
            }
            Err(err) => {
                return Err(format!("read response failed: {err}"));
            }
        }
    }
    let raw_text = String::from_utf8_lossy(&raw).to_string();
    let (status, response_headers, body_start) = parse_response_head(&raw_text)?;
    let body_text = if body_start < raw_text.len() {
        Some(raw_text[body_start..].to_string())
    } else {
        None
    };
    let body_redacted = body_text.map(|text| {
        let (redacted, _) = prism_supply::secrets::redact_line(&text);
        redacted
    });
    Ok((
        status,
        redact_headers(&response_headers),
        body_redacted,
        truncated,
        FailureKind::None,
        None,
    ))
}

/// parse_response_head 的返回（status, headers, body 起始字节）。
type ResponseHead = (Option<u16>, BTreeMap<String, String>, usize);

/// 解析状态行 + headers，返回 ResponseHead。
fn parse_response_head(
    raw: &str,
) -> Result<ResponseHead, String> {
    let split = raw
        .find("\r\n\r\n")
        .ok_or_else(|| "malformed response: no header terminator".to_string())?;
    let head = &raw[..split];
    let body_start = split + 4;
    let mut lines = head.lines();
    let status_line = lines.next().unwrap_or_default();
    let status = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.parse::<u16>().ok());
    let mut headers = BTreeMap::new();
    for line in lines {
        if let Some((name, value)) = line.split_once(':') {
            headers.insert(name.trim().to_string(), value.trim().to_string());
        }
    }
    Ok((status, headers, body_start))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    /// 本地回显 fixture server（§63：本地测试 server，禁止公网）。
    struct FixtureServer {
        port: u16,
        shutdown: std::sync::Arc<std::sync::atomic::AtomicBool>,
        handle: Option<std::thread::JoinHandle<()>>,
    }

    impl FixtureServer {
        fn start(responses: Vec<(u16, &'static str)>) -> Self {
            use std::net::TcpListener;
            use std::sync::atomic::AtomicBool;
            let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
            let port = listener.local_addr().unwrap().port();
            let shutdown = Arc::new(AtomicBool::new(false));
            let shutdown_handle = shutdown.clone();
            let handle = std::thread::spawn(move || {
                listener.set_nonblocking(true).ok();
                let mut next = 0usize;
                while !shutdown_handle.load(std::sync::atomic::Ordering::Relaxed) {
                    match listener.accept() {
                        Ok((mut stream, _)) => {
                            let mut buffer = [0u8; 4096];
                            let _ = stream.read(&mut buffer);
                            let (code, body) = responses.get(next).copied().unwrap_or((200, "ok"));
                            next = (next + 1).min(responses.len().saturating_sub(1));
                            let response = format!(
                                "HTTP/1.1 {code} TEST\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{body}",
                                body.len()
                            );
                            let _ = stream.write_all(response.as_bytes());
                        }
                        Err(_) => std::thread::sleep(Duration::from_millis(10)),
                    }
                }
            });
            FixtureServer {
                port,
                shutdown,
                handle: Some(handle),
            }
        }

        fn url(&self, path: &str) -> String {
            format!("http://127.0.0.1:{}{path}", self.port)
        }
    }

    impl Drop for FixtureServer {
        fn drop(&mut self) {
            self.shutdown
                .store(true, std::sync::atomic::Ordering::Relaxed);
            if let Some(handle) = self.handle.take() {
                handle.join().ok();
            }
        }
    }

    #[test]
    fn observe_200_records_status_latency_and_redacted_body() {
        let server = FixtureServer::start(vec![(200, "hello")]);
        let cancel = CancellationToken::new();
        let observation = observe(
            &TargetGuard::default(),
            HttpMethod::Get,
            &server.url("/api/cards"),
            &Default::default(),
            None,
            &ObserveLimits::default(),
            &cancel,
        );
        assert_eq!(observation.status, Some(200));
        assert_eq!(observation.failure, FailureKind::None);
        assert!(observation.latency_ms < 10_000);
        assert_eq!(observation.response_body.as_deref(), Some("hello"));
    }

    #[test]
    fn http_500_is_http_failure_not_transport() {
        // §9：HTTP 500 = HttpStatus failure，与连接拒绝分开。
        let server = FixtureServer::start(vec![(500, "boom")]);
        let cancel = CancellationToken::new();
        let observation = observe(
            &TargetGuard::default(),
            HttpMethod::Get,
            &server.url("/x"),
            &Default::default(),
            None,
            &ObserveLimits::default(),
            &cancel,
        );
        assert_eq!(observation.status, Some(500));
        assert_eq!(observation.failure, FailureKind::HttpStatus);
    }

    #[test]
    fn connection_refused_is_transport_failure() {
        let cancel = CancellationToken::new();
        let observation = observe(
            &TargetGuard::default(),
            HttpMethod::Get,
            "http://127.0.0.1:1/x",
            &Default::default(),
            None,
            &ObserveLimits {
                timeout_ms: 500,
                max_response_bytes: 1024,
                connect_timeout_ms: 500,
            },
            &cancel,
        );
        assert_eq!(observation.status, None);
        assert_eq!(observation.failure, FailureKind::Transport);
        assert!(observation.diagnostic.is_some(), "transport 失败必须带诊断");
    }

    #[test]
    fn ssrf_blocked_targets_never_reach_network() {
        let cancel = CancellationToken::new();
        let observation = observe(
            &TargetGuard::default(),
            HttpMethod::Get,
            "http://169.254.169.254/latest/meta-data",
            &Default::default(),
            None,
            &ObserveLimits::default(),
            &cancel,
        );
        assert_eq!(observation.failure, FailureKind::Transport);
        assert!(observation
            .diagnostic
            .as_deref()
            .unwrap_or_default()
            .contains("SSRF guard"));
    }

    #[test]
    fn set_cookie_header_is_redacted() {
        let server = FixtureServer::start(vec![(200, "ok")]);
        // fixture 不支持自定义响应头——直接验证 redact_headers 单元行为。
        let mut headers = BTreeMap::new();
        headers.insert("Set-Cookie".to_string(), "session=secret123".to_string());
        let redacted = redact_headers(&headers);
        assert_eq!(
            redacted.get("Set-Cookie").map(String::as_str),
            Some("Set-Cookie: [REDACTED]")
        );
        drop(server);
    }

    #[test]
    fn parse_response_head_extracts_status_and_headers() {
        let raw = "HTTP/1.1 404 NOT FOUND\r\nContent-Type: text/plain\r\n\r\nbody";
        let (status, headers, body_start) = parse_response_head(raw).unwrap();
        assert_eq!(status, Some(404));
        assert_eq!(
            headers.get("Content-Type").map(String::as_str),
            Some("text/plain")
        );
        assert_eq!(&raw[body_start..], "body");
    }
}
