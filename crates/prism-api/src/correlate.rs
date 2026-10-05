//! 关联（M213 §23-§27）：Request ↔ Endpoint ↔ Handler Symbol ↔ Log ↔
//! Recent Change。
//!
//! 关联全部 INFERENCE/FACT 分级：
//! - method+path 匹配 route template ⇒ endpoint match（INFERENCE，§24）；
//! - correlation id 出现在日志行 ⇒ Request→Log（§25）；
//! - handler 文件的最近 Git 变更 ⇒ Potentially affected（§27，不下因果）。

use crate::model::{ApiEndpoint, HttpMethod};
use prism_core::task::CancellationToken;

/// route template 与 (method, path) 的匹配（§24：{id} 段通配；ASP.NET
/// 路由大小写不敏感 ⇒ 小写比较；匹配是 INFERENCE 非 FACT）。
pub fn match_endpoint<'a>(
    endpoints: &'a [ApiEndpoint],
    method: HttpMethod,
    path: &str,
) -> Option<&'a ApiEndpoint> {
    let segments: Vec<String> = path
        .split('/')
        .filter(|s| !s.is_empty())
        .map(|s| s.to_ascii_lowercase())
        .collect();
    endpoints.iter().find(|endpoint| {
        if endpoint.method != method {
            return false;
        }
        let template: Vec<String> = endpoint
            .route_template
            .split('/')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_ascii_lowercase())
            .collect();
        template.len() == segments.len()
            && template
                .iter()
                .zip(segments.iter())
                .all(|(t, s)| *t == *s || t.starts_with('{') || t.starts_with(':'))
    })
}

/// Request → Log（§25）：correlation id 出现在日志行中 ⇒ 关联
/// （M210 DebugEvent.message 已 redacted；证据 = 日志行号）。
pub fn correlate_request_logs(
    correlation_id: &str,
    log_events: &[(String, usize, String)],
) -> Vec<(String, usize)> {
    log_events
        .iter()
        .filter(|(_, _, message)| message.contains(correlation_id))
        .map(|(source, line, _)| (source.clone(), *line))
        .collect()
}

/// Request → Error cluster（§26）：错误事件消息含 correlation id。
pub fn correlate_error_cluster(
    correlation_id: &str,
    error_events: &[(String, String)],
) -> Option<String> {
    error_events
        .iter()
        .find(|(cluster_id, message)| message.contains(correlation_id) && !cluster_id.is_empty())
        .map(|(cluster_id, _)| cluster_id.clone())
}

/// Endpoint → handler symbol（§24 INFERENCE）：
/// handler 文件在 M23 索引中的首个类型符号。
pub fn correlate_handler_symbol(
    endpoint: &ApiEndpoint,
    symbol_index: &prism_csharp::SymbolIndex,
) -> Option<String> {
    let file = endpoint.evidence.split(':').next()?;
    let candidates = symbol_index.by_file.get(file)?;
    // handler 关联优先类型符号（class/struct/record），namespace 不算（§24）。
    let is_type_symbol = |kind: prism_csharp::SymbolKindCs| {
        matches!(
            kind,
            prism_csharp::SymbolKindCs::Class
                | prism_csharp::SymbolKindCs::Struct
                | prism_csharp::SymbolKindCs::Record
                | prism_csharp::SymbolKindCs::RecordStruct
                | prism_csharp::SymbolKindCs::Interface
        )
    };
    candidates
        .iter()
        .filter_map(|id| symbol_index.by_id.get(id))
        .find(|symbol| is_type_symbol(symbol.kind))
        .map(|symbol| symbol.qualified_name.clone())
}

/// Endpoint → Recent Change（§27）：handler 文件的最近 commit
/// （**不**下因果——输出即"最近变更了此端点文件"，evidence = commit id）。
pub fn correlate_recent_change(
    service: &prism_git::GitService,
    root: &std::path::Path,
    handler_file: &str,
    max_commits: u64,
    cancel: &CancellationToken,
) -> prism_git::GitResult<Vec<String>> {
    let entries = service.file_history(root, handler_file, max_commits, cancel)?;
    Ok(entries.into_iter().map(|entry| entry.commit.id).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::EndpointSource;

    fn endpoint(method: HttpMethod, template: &str) -> ApiEndpoint {
        ApiEndpoint {
            endpoint_id: format!("api:{method:?} {template}"),
            method,
            route_template: template.to_string(),
            source: EndpointSource::SourceCode,
            evidence: "Controllers/CardsController.cs:14".to_string(),
            handler_symbol: None,
        }
    }

    #[test]
    fn endpoint_matching_with_path_parameters() {
        let endpoints = vec![
            endpoint(HttpMethod::Get, "/api/cards/{id}"),
            endpoint(HttpMethod::Post, "/api/cards"),
        ];
        let matched = match_endpoint(&endpoints, HttpMethod::Get, "/api/cards/42");
        assert!(matched.is_some());
        assert_eq!(matched.unwrap().route_template, "/api/cards/{id}");
        assert!(
            match_endpoint(&endpoints, HttpMethod::Delete, "/api/cards/42").is_none(),
            "方法不匹配 ⇒ 不关联"
        );
        assert!(match_endpoint(&endpoints, HttpMethod::Get, "/api/other/42").is_none());
    }

    #[test]
    fn correlation_id_matches_log_lines() {
        let logs = vec![
            (
                "logs/app.log".to_string(),
                10,
                "[request abc123] loading deck".to_string(),
            ),
            ("logs/app.log".to_string(), 20, "unrelated".to_string()),
            (
                "logs/db.log".to_string(),
                5,
                "request abc123: exception".to_string(),
            ),
        ];
        let matched = correlate_request_logs("abc123", &logs);
        assert_eq!(matched.len(), 2);
        assert_eq!(matched[0], ("logs/app.log".to_string(), 10));
    }

    #[test]
    fn error_cluster_correlation() {
        let errors = vec![(
            "cluster:abc".to_string(),
            "NullReference at abc123 request".to_string(),
        )];
        assert_eq!(
            correlate_error_cluster("abc123", &errors).as_deref(),
            Some("cluster:abc")
        );
        assert!(correlate_error_cluster("no-match-id", &errors).is_none());
    }
}
