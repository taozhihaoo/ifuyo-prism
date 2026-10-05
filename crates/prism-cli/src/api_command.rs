//! `prism api`（M213 §54/§55）：API Endpoint 发现 / 观测 / 关联。
//!
//! 安全（§55）：`observe` 显式 --url（SSRF guard 强制）；`replay` 显式
//! --yes-confirm-unsafe（非 safe 方法默认拒绝 §31）；无 flag 不发任何
//! 网络请求。

use prism_api::{HttpMethod, TargetGuard};
use prism_core::task::CancellationToken;
use std::path::Path;

pub fn run(
    path: &Path,
    command: &ApiCommand,
    runner: std::sync::Arc<dyn prism_core::runtime::command::CommandRunner>,
) -> i32 {
    let cancel = CancellationToken::new();
    let _ = runner;
    match command {
        ApiCommand::Endpoints { json, .. } => {
            let files = collect_sources(path, &cancel);
            let endpoints = prism_api::discover_endpoints(&prism_api::DiscoverInput {
                files: &files,
            });
            if *json {
                match serde_json::to_string_pretty(&endpoints) {
                    Ok(text) => println!("{text}"),
                    Err(err) => {
                        eprintln!("prism: failed to serialize endpoints: {err}");
                        return crate::EXIT_ERROR;
                    }
                }
            } else if endpoints.is_empty() {
                println!("api: no endpoints discovered (source discovery; runtime observation is a separate opt-in capability)");
            } else {
                for endpoint in &endpoints {
                    println!(
                        "{:<6} {:<40} [{}] handler: {}",
                        endpoint.method.as_str(),
                        endpoint.route_template,
                        endpoint.evidence,
                        endpoint.handler_symbol.as_deref().unwrap_or("?")
                    );
                }
            }
            crate::EXIT_OK
        }
        ApiCommand::Observe { url, method, header, json } => {
            // §55/§19：显式 URL + SSRF guard；headers 显式传入。
            let method = match HttpMethod::parse(method) {
                Some(method) => method,
                None => {
                    eprintln!("prism: invalid method '{method}'");
                    return crate::EXIT_INVALID_INPUT;
                }
            };
            let mut headers = BTreeMap::new();
            for pair in header {
                if let Some((name, value)) = pair.split_once('=') {
                    headers.insert(name.to_string(), value.to_string());
                }
            }
            let observation = prism_api::observe(
                &TargetGuard::default(),
                method,
                url,
                &headers,
                None,
                &Default::default(),
                &cancel,
            );
            if *json {
                match serde_json::to_string_pretty(&observation) {
                    Ok(text) => println!("{text}"),
                    Err(err) => {
                        eprintln!("prism: failed to serialize observation: {err}");
                        return crate::EXIT_ERROR;
                    }
                }
            } else {
                println!(
                    "{} {} => status {:?} failure {:?} latency {} ms",
                    observation.method.as_str(),
                    observation.url,
                    observation.status,
                    observation.failure,
                    observation.latency_ms
                );
                if let Some(diagnostic) = &observation.diagnostic {
                    println!("diag: {diagnostic}");
                }
            }
            crate::EXIT_OK
        }
        ApiCommand::WebhookListen { port, secret, signature_header, duration_secs, json } => {
            let config = prism_api::WebhookReceiverConfig {
                port: *port,
                secret: secret.clone(),
                signature_header: signature_header.clone(),
                listen_timeout_ms: duration_secs * 1000,
                ..Default::default()
            };
            match prism_api::WebhookReceiver::start(config, &cancel) {
                Ok(mut receiver) => {
                    eprintln!(
                        "prism: webhook receiver listening on 127.0.0.1:{} for {duration_secs}s (Ctrl+C to stop early)",
                        receiver.port()
                    );
                    // 接收器自身按 listen_timeout_ms 自动停止；主线程等待它结束。
                    let deadline = std::time::Instant::now()
                        + std::time::Duration::from_millis(duration_secs * 1000 + 500);
                    while receiver.is_running() && std::time::Instant::now() < deadline {
                        std::thread::sleep(std::time::Duration::from_millis(100));
                    }
                    receiver.stop();
                    let events = receiver.events();
                    if *json {
                        match serde_json::to_string_pretty(&events) {
                            Ok(text) => println!("{text}"),
                            Err(err) => {
                                eprintln!("prism: failed to serialize events: {err}");
                                return crate::EXIT_ERROR;
                            }
                        }
                    } else if events.is_empty() {
                        println!("webhook: 0 events received (receiver ran; not an error)");
                    } else {
                        for event in &events {
                            println!(
                                "{} type={:?} signature={:?} processing={:?}",
                                event.received_at,
                                event.event_type,
                                event.signature,
                                event.processing
                            );
                            if let Some(payload) = &event.payload {
                                println!("  payload: {payload}");
                            }
                        }
                    }
                    crate::EXIT_OK
                }
                Err(err) => {
                    eprintln!("prism: webhook receiver failed to bind 127.0.0.1:{port}: {err}");
                    crate::EXIT_ERROR
                }
            }
        }
        ApiCommand::Correlate { url, method, max, .. } => {
            // 垂直切片（§70）：endpoint 匹配 → handler symbol → 近期 git 变更。
            let method = match HttpMethod::parse(method) {
                Some(method) => method,
                None => {
                    eprintln!("prism: invalid method '{method}'");
                    return crate::EXIT_INVALID_INPUT;
                }
            };
            let files = collect_sources(path, &cancel);
            let endpoints = prism_api::discover_endpoints(&prism_api::DiscoverInput {
                files: &files,
            });
            let path_part: &str = url
                .split_once("://")
                .map(|(_, rest)| rest)
                .and_then(|rest| rest.find('/').map(|at| &rest[at..]))
                .unwrap_or(url);
            let Some(endpoint) = prism_api::match_endpoint(&endpoints, method, path_part) else {
                println!("no endpoint match for {method:?} {path_part}");
                return crate::EXIT_OK;
            };
            println!(
                "endpoint: {} {} [{}]",
                endpoint.method.as_str(),
                endpoint.route_template,
                endpoint.evidence
            );
            // handler symbol（M23 by_file 首类型，INFERENCE）。
            let handler_file = endpoint.evidence.split(':').next().unwrap_or_default().to_string();
            let mut engine = prism_csharp::SymbolEngine::new();
            let mut symbol_index: Option<prism_csharp::SymbolIndex> = None;
            if handler_file.ends_with(".cs") {
                if let Ok(source) = std::fs::read_to_string(path.join(&handler_file)) {
                    engine.update_file(&handler_file, &source);
                    symbol_index = Some(engine.build_index());
                }
            }
            if let Some(index) = &symbol_index {
                if let Some(handler) =
                    prism_api::correlate_handler_symbol(endpoint, index)
                {
                    println!("handler (inference): {handler}");
                }
            }
            // 最近 Git 变更（M211 file_history；不下因果 §27）。
            let git = prism_git::GitService::new(runner);
            match prism_api::correlate_recent_change(&git, path, &handler_file, *max, &cancel) {
                Ok(commits) => {
                    if commits.is_empty() {
                        println!("recent changes: none found (no evidence)");
                    } else {
                        for commit in commits.iter().take(5) {
                            println!("recent change: {}", &commit[..commit.len().min(10)]);
                        }
                        println!(
                            "note: recent change is a temporal correlation, not a causal claim"
                        );
                    }
                }
                Err(err) => {
                    eprintln!("prism: git history unavailable: {err}");
                }
            }
            crate::EXIT_OK
        }
    }
}

fn collect_sources(
    path: &Path,
    cancel: &CancellationToken,
) -> Vec<(String, String)> {
    let mut files = Vec::new();
    if let Ok(discovery) = prism_project::DiscoveryService::new().discover(
        path,
        prism_project::ScanOptions::default(),
        cancel,
        |_| {},
    ) {
        for file in &discovery.files {
            if !prism_api::is_discoverable_source(&file.relative_path)
                || file.size_bytes > 1024 * 1024
            {
                continue;
            }
            if let Ok(content) = std::fs::read_to_string(path.join(&file.relative_path)) {
                files.push((file.relative_path.clone(), content));
            }
        }
    }
    files.sort();
    files
}

use clap::Subcommand;
use std::collections::BTreeMap;

#[derive(Debug, Subcommand)]
pub enum ApiCommand {
    /// 发现项目中的 API Endpoint（静态源码证据）
    Endpoints {
        #[arg(default_value = ".")]
        path: std::path::PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// 对显式 URL 做一次有界观测（SSRF guard 强制；§55 显式网络动作）
    Observe {
        #[arg(long)]
        url: String,
        #[arg(long, default_value = "GET")]
        method: String,
        /// 请求 header，name=value 形式（可多次）
        #[arg(long = "header")]
        header: Vec<String>,
        #[arg(long)]
        json: bool,
    },
    /// 本地 Webhook 接收器（§21：显式 start/stop；恒绑 127.0.0.1）
    WebhookListen {
        #[arg(long, default_value_t = 8085)]
        port: u16,
        /// HMAC-SHA256 secret（缺省 => 签名状态 Unknown，§11）
        #[arg(long)]
        secret: Option<String>,
        /// 签名 header 名
        #[arg(long, default_value = "X-Hub-Signature-256")]
        signature_header: String,
        /// 监听时长（秒，超时自动停止）
        #[arg(long, default_value_t = 30)]
        duration_secs: u64,
        #[arg(long)]
        json: bool,
    },
    /// 端点 → handler symbol → 近期变更 关联链（垂直切片 §70）
    Correlate {
        #[arg(default_value = ".")]
        path: std::path::PathBuf,
        #[arg(long)]
        url: String,
        #[arg(long, default_value = "GET")]
        method: String,
        #[arg(long, default_value_t = 10)]
        max: u64,
    },
}
