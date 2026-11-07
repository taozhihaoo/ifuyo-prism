//! `prism mcp`（优化清单批 B）：只读 MCP stdio 服务器。
//!
//! 让本地 AI 客户端（Claude 等）直接消费 Prism 的**持久化分析结果**——
//! 与 GUI 共用 `analysis_runs` 库（PRISM_DB_PATH 同口径），不重跑分析、
//! 不读项目文件、不写任何数据（§MCP charter：read-only）。
//!
//! 协议：newline-delimited JSON-RPC 2.0（MCP stdio transport）。
//! stdout 只承载协议响应；任何日志走 stderr（否则破坏客户端分帧）。
//! 协议处理是纯函数（`handle_message`），IO 只是薄壳——可测性优先。

use serde_json::{json, Value};
use std::io::{BufRead, Write};

pub const PROTOCOL_VERSION: &str = "2024-11-05";

/// 工具目录（确定性顺序；无隐藏工具）。json! 非常量上下文 ⇒ 函数形态。
fn tools() -> Vec<Value> {
    vec![
        json!({
            "name": "list_projects",
            "description": "List projects and domains that have persisted analysis results (from the Prism desktop app).",
            "inputSchema": {"type": "object", "properties": {}}
        }),
        json!({
            "name": "list_runs",
            "description": "List analysis runs (history heads) for one project domain. Args: project_root, domain (health|health2), limit?.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project_root": {"type": "string"},
                    "domain": {"type": "string", "enum": ["health", "health2"]},
                    "limit": {"type": "integer", "minimum": 1, "maximum": 200}
                },
                "required": ["project_root", "domain"]
            }
        }),
        json!({
            "name": "get_report",
            "description": "Latest persisted analysis report summary for one project domain: findings, gate and lifecycle (health2) or findings by rule (health). Args: project_root, domain.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project_root": {"type": "string"},
                    "domain": {"type": "string", "enum": ["health", "health2"]}
                },
                "required": ["project_root", "domain"]
            }
        }),
        json!({
            "name": "get_finding",
            "description": "One finding with its evidence (traceability). Args: project_root, domain, finding_id.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project_root": {"type": "string"},
                    "domain": {"type": "string", "enum": ["health", "health2"]},
                    "finding_id": {"type": "string"}
                },
                "required": ["project_root", "domain", "finding_id"]
            }
        }),
    ]
}

/// stdio 主循环（EOF ⇒ 退出 0）。
pub fn run() -> i32 {
    let state = McpState {
        db_path: prism_store::default_db_path(),
    };
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    for line in stdin.lock().lines() {
        let line = match line {
            Ok(line) => line,
            Err(_) => return crate::EXIT_ERROR,
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let response = match serde_json::from_str::<Value>(trimmed) {
            Ok(message) => handle_message(&state, &message),
            Err(err) => Some(json!({
                "jsonrpc": "2.0", "id": Value::Null,
                "error": {"code": -32700, "message": format!("parse error: {err}")}
            })),
        };
        if let Some(response) = response {
            if writeln!(stdout, "{response}")
                .and_then(|_| stdout.flush())
                .is_err()
            {
                return crate::EXIT_ERROR;
            }
        }
    }
    crate::EXIT_OK
}

/// MCP 服务器状态：只持数据库路径（连接按调用打开，read-only）。
pub struct McpState {
    pub db_path: std::path::PathBuf,
}

/// 一条 JSON-RPC 消息 → 至多一条响应（notification ⇒ None）。
pub fn handle_message(state: &McpState, message: &Value) -> Option<Value> {
    let method = message.get("method")?.as_str()?.to_string();
    let id = message.get("id").cloned();
    // JSON-RPC：无 id = notification，不响应。
    let id = id?;

    let result = match method.as_str() {
        "initialize" => Ok(json!({
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": {"tools": {}},
            "serverInfo": {"name": "prism", "version": env!("CARGO_PKG_VERSION")}
        })),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({ "tools": tools() })),
        "tools/call" => {
            let name = message
                .pointer("/params/name")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let arguments = message
                .pointer("/params/arguments")
                .cloned()
                .unwrap_or(json!({}));
            match call_tool(state, &name, &arguments) {
                Ok(text) => Ok(json!({
                    "content": [{"type": "text", "text": text}], "isError": false
                })),
                Err(err) => Ok(json!({
                    "content": [{"type": "text", "text": err}], "isError": true
                })),
            }
        }
        other => Err(format!("method not found: {other}")),
    };

    Some(match result {
        Ok(result) => json!({"jsonrpc": "2.0", "id": id, "result": result}),
        Err(message) => json!({
            "jsonrpc": "2.0", "id": id,
            "error": {"code": -32601, "message": message}
        }),
    })
}

/// 工具分发（全部只读；失败 = 诚实错误文本，不伪造空结果）。
fn call_tool(state: &McpState, name: &str, args: &Value) -> Result<String, String> {
    let connection = open_read_only(&state.db_path)?;
    match name {
        "list_projects" => tool_list_projects(&connection),
        "list_runs" => tool_list_runs(&connection, args),
        "get_report" => tool_get_report(&connection, args),
        "get_finding" => tool_get_finding(&connection, args),
        other => Err(format!("unknown tool: {other}")),
    }
}

/// 只读打开；库不存在 ⇒ 明确错误（工具层转成诚实提示）。
fn open_read_only(path: &std::path::Path) -> Result<rusqlite::Connection, String> {
    if !path.exists() {
        return Err("no stored analysis results (database not created yet); run an analysis in the Prism desktop app first".to_string());
    }
    rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|err| format!("cannot open results database read-only: {err}"))
}

fn str_arg(args: &Value, key: &str) -> Result<String, String> {
    args.get(key)
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .ok_or_else(|| format!("missing required argument: {key}"))
}

/// 最新 payload + (captured_at, basis)。表不存在 ⇒ 诚实错误。
fn latest_payload(
    connection: &rusqlite::Connection,
    project_root: &str,
    domain: &str,
) -> Result<Option<(String, String, Value)>, String> {
    let mut statement = connection
        .prepare(
            "SELECT captured_at, basis, payload FROM analysis_runs
             WHERE project_root = ?1 AND domain = ?2 ORDER BY id DESC LIMIT 1",
        )
        .map_err(|err| format!("results table unavailable: {err}"))?;
    let mut rows = statement
        .query(rusqlite::params![project_root, domain])
        .map_err(|err| format!("query failed: {err}"))?;
    match rows.next() {
        Ok(Some(row)) => {
            let captured_at: String = row.get(0).map_err(|err| err.to_string())?;
            let basis: String = row.get(1).map_err(|err| err.to_string())?;
            let payload: String = row.get(2).map_err(|err| err.to_string())?;
            let payload: Value =
                serde_json::from_str(&payload).map_err(|err| format!("payload corrupt: {err}"))?;
            Ok(Some((captured_at, basis, payload)))
        }
        Ok(None) => Ok(None),
        Err(err) => Err(format!("query failed: {err}")),
    }
}

fn require_payload(
    connection: &rusqlite::Connection,
    args: &Value,
) -> Result<(String, String, String, Value), String> {
    let project_root = str_arg(args, "project_root")?;
    let domain = str_arg(args, "domain")?;
    match latest_payload(connection, &project_root, &domain)? {
        Some((captured_at, basis, payload)) => {
            Ok((project_root, captured_at, basis, payload))
        }
        None => Err(format!(
            "no persisted '{domain}' results for {project_root}; run the analysis in the Prism desktop app first"
        )),
    }
}

fn tool_list_projects(connection: &rusqlite::Connection) -> Result<String, String> {
    let mut statement = connection
        .prepare(
            "SELECT project_root, domain, MAX(captured_at) FROM analysis_runs
             GROUP BY project_root, domain ORDER BY project_root, domain",
        )
        .map_err(|err| format!("results table unavailable: {err}"))?;
    let rows = statement
        .query_map([], |row| {
            Ok(json!({
                "project_root": row.get::<_, String>(0)?,
                "domain": row.get::<_, String>(1)?,
                "latest_captured_at": row.get::<_, String>(2)?,
            }))
        })
        .map_err(|err| format!("query failed: {err}"))?;
    let mut entries = Vec::new();
    for row in rows {
        entries.push(row.map_err(|err| err.to_string())?);
    }
    serde_json::to_string_pretty(&json!({"projects": entries})).map_err(|err| err.to_string())
}

fn tool_list_runs(connection: &rusqlite::Connection, args: &Value) -> Result<String, String> {
    let project_root = str_arg(args, "project_root")?;
    let domain = str_arg(args, "domain")?;
    let limit = args
        .get("limit")
        .and_then(|v| v.as_u64())
        .unwrap_or(20)
        .min(200) as i64;
    let mut statement = connection
        .prepare(
            "SELECT id, captured_at, basis, schema_version,
                    json_array_length(finding_ids)
             FROM analysis_runs WHERE project_root = ?1 AND domain = ?2
             ORDER BY id DESC LIMIT ?3",
        )
        .map_err(|err| format!("results table unavailable: {err}"))?;
    let rows = statement
        .query_map(rusqlite::params![project_root, domain, limit], |row| {
            Ok(json!({
                "run_id": row.get::<_, i64>(0)?,
                "captured_at": row.get::<_, String>(1)?,
                "basis": row.get::<_, String>(2)?,
                "schema_version": row.get::<_, i64>(3)?,
                "findings_count": row.get::<_, i64>(4).unwrap_or(0),
            }))
        })
        .map_err(|err| format!("query failed: {err}"))?;
    let mut runs = Vec::new();
    for row in rows {
        runs.push(row.map_err(|err| err.to_string())?);
    }
    serde_json::to_string_pretty(&json!({"runs": runs})).map_err(|err| err.to_string())
}

fn tool_get_report(connection: &rusqlite::Connection, args: &Value) -> Result<String, String> {
    let (_root, captured_at, basis, payload) = require_payload(connection, args)?;
    let domain = str_arg(args, "domain")?;
    let findings = payload
        .get("findings")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let findings_json = match domain.as_str() {
        "health2" => findings
            .iter()
            .map(|finding| {
                json!({
                    "id": finding.get("id"),
                    "type": finding.get("finding_type"),
                    "severity": finding.get("severity"),
                    "title": finding.get("title"),
                    "target": finding.get("target"),
                })
            })
            .collect::<Vec<_>>(),
        _ => findings
            .iter()
            .map(|finding| {
                json!({
                    "id": finding.get("id"),
                    "rule_id": finding.get("rule_id"),
                    "severity": finding.get("severity"),
                    "title": finding.get("title"),
                    "location": finding.pointer("/location/relative_path"),
                })
            })
            .collect::<Vec<_>>(),
    };
    let mut report = json!({
        "domain": domain,
        "captured_at": captured_at,
        "workspace_basis": basis,
        "findings_count": findings.len(),
        "findings": findings_json,
    });
    if domain == "health2" {
        report["quality_gate"] = payload.get("quality_gate").cloned().unwrap_or(Value::Null);
        report["lifecycle"] = json!({
            "new": payload.pointer("/lifecycle/new_findings").and_then(|v| v.as_array()).map(Vec::len).unwrap_or(0),
            "persistent": payload.pointer("/lifecycle/persistent_findings").and_then(|v| v.as_array()).map(Vec::len).unwrap_or(0),
            "resolved": payload.pointer("/lifecycle/resolved_findings").and_then(|v| v.as_array()).map(Vec::len).unwrap_or(0),
            "regressed": payload.pointer("/lifecycle/regressed_findings").and_then(|v| v.as_array()).map(Vec::len).unwrap_or(0),
        });
    }
    serde_json::to_string_pretty(&report).map_err(|err| err.to_string())
}

fn tool_get_finding(connection: &rusqlite::Connection, args: &Value) -> Result<String, String> {
    let (_root, captured_at, _basis, payload) = require_payload(connection, args)?;
    let domain = str_arg(args, "domain")?;
    let finding_id = str_arg(args, "finding_id")?;
    let findings = payload
        .get("findings")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let finding = findings
        .iter()
        .find(|finding| finding.get("id").and_then(|v| v.as_str()) == Some(finding_id.as_str()))
        .ok_or_else(|| format!("finding not found: {finding_id}"))?;
    let mut detail = finding.clone();
    detail["captured_at"] = json!(captured_at);
    if domain == "health2" {
        // 证据引用 → 证据库展开（§37 可追溯）。
        let evidence_pool = payload
            .get("evidence")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let referenced: Vec<Value> = finding
            .get("evidence")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default()
            .iter()
            .filter_map(|id| {
                let id = id.as_str()?;
                evidence_pool
                    .iter()
                    .find(|entry| entry.get("id").and_then(|v| v.as_str()) == Some(id))
                    .cloned()
            })
            .collect();
        detail["evidence_resolved"] = json!(referenced);
    }
    serde_json::to_string_pretty(&detail).map_err(|err| err.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use prism_store::{AnalysisRunRecord, AnalysisRunsStore, SqliteAnalysisRunsStore};

    fn state_with(path: &std::path::Path) -> McpState {
        McpState {
            db_path: path.to_path_buf(),
        }
    }

    /// 种子库：一条 health2 运行（最小 GUI 报告负载）。
    fn seed(path: &std::path::Path) {
        let store = SqliteAnalysisRunsStore::open(path).unwrap();
        let payload = json!({
            "schema_version": 1,
            "findings": [
                {
                    "id": "find:abc",
                    "finding_type": "STRUCT/LARGE_FILE",
                    "severity": "warning",
                    "status": "open",
                    "title": "Large source file: src/big.cs",
                    "origin": "fact",
                    "confidence": "high",
                    "target": "file:src/big.cs",
                    "evidence": ["ev:1"]
                }
            ],
            "evidence": [
                {"id": "ev:1", "kind": "metric", "path": "src/big.cs",
                 "range": null, "reference": null,
                 "note": "1200 lines counted from bounded read"}
            ],
            "lifecycle": {
                "new_findings": ["find:abc"], "persistent_findings": [],
                "resolved_findings": [], "regressed_findings": [],
                "suppressed_findings": []
            },
            "quality_gate": {"verdict": "fail", "new_count": 1,
                             "breaking_count": 0, "reasons": ["max_new=0"]},
            "rule_source": {"engine": "prism-health2", "builtin_rules": 4},
            "baseline": {"kind": "empty"}
        });
        store
            .record_run(&AnalysisRunRecord {
                id: None,
                project_root: "C:/tmp/demo".into(),
                domain: "health2".into(),
                schema_version: 1,
                basis: "abc|0".into(),
                captured_at: "2026-10-07T01:00:00Z".into(),
                finding_ids: vec!["find:abc".into()],
                lifecycle_new: vec!["find:abc".into()],
                payload,
            })
            .unwrap();
    }

    fn temp_db(tag: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "prism-mcp-{tag}-{}-{}.db",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.subsec_nanos())
                .unwrap_or(0)
        ))
    }

    #[test]
    fn initialize_and_tools_list_follow_mcp_shape() {
        let state = McpState {
            db_path: std::path::PathBuf::from("Z:/definitely-missing/prism.db"),
        };
        let response = handle_message(
            &state,
            &json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}}),
        )
        .unwrap();
        assert_eq!(response["result"]["protocolVersion"], PROTOCOL_VERSION);
        assert_eq!(response["result"]["serverInfo"]["name"], "prism");
        assert!(response["result"]["capabilities"]["tools"].is_object());

        let response = handle_message(
            &state,
            &json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"}),
        )
        .unwrap();
        let tools = response["result"]["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 4);
        assert!(tools.iter().any(|t| t["name"] == "get_report"));
    }

    #[test]
    fn notifications_get_no_response_and_unknown_methods_error() {
        let state = McpState {
            db_path: std::path::PathBuf::from("Z:/definitely-missing/prism.db"),
        };
        assert!(handle_message(
            &state,
            &json!({"jsonrpc": "2.0", "method": "notifications/initialized"})
        )
        .is_none());
        let response = handle_message(
            &state,
            &json!({"jsonrpc": "2.0", "id": 7, "method": "resources/list"}),
        )
        .unwrap();
        assert_eq!(response["error"]["code"], -32601);
    }

    #[test]
    fn tools_serve_persisted_results_and_resolve_evidence() {
        let path = temp_db("serve");
        seed(&path);
        let state = state_with(&path);

        let response = handle_message(
            &state,
            &json!({
                "jsonrpc": "2.0", "id": 3, "method": "tools/call",
                "params": {"name": "list_projects", "arguments": {}}
            }),
        )
        .unwrap();
        let text = response["result"]["content"][0]["text"].as_str().unwrap();
        let projects: Value = serde_json::from_str(text).unwrap();
        assert_eq!(projects["projects"][0]["project_root"], "C:/tmp/demo");
        assert_eq!(projects["projects"][0]["domain"], "health2");

        let response = handle_message(
            &state,
            &json!({
                "jsonrpc": "2.0", "id": 4, "method": "tools/call",
                "params": {"name": "get_report",
                           "arguments": {"project_root": "C:/tmp/demo", "domain": "health2"}}
            }),
        )
        .unwrap();
        assert_eq!(response["result"]["isError"], false);
        let report: Value =
            serde_json::from_str(response["result"]["content"][0]["text"].as_str().unwrap())
                .unwrap();
        assert_eq!(report["findings_count"], 1);
        assert_eq!(report["quality_gate"]["verdict"], "fail");
        assert_eq!(report["lifecycle"]["new"], 1);

        let response = handle_message(
            &state,
            &json!({
                "jsonrpc": "2.0", "id": 5, "method": "tools/call",
                "params": {"name": "get_finding",
                           "arguments": {"project_root": "C:/tmp/demo",
                                         "domain": "health2",
                                         "finding_id": "find:abc"}}
            }),
        )
        .unwrap();
        let finding: Value =
            serde_json::from_str(response["result"]["content"][0]["text"].as_str().unwrap())
                .unwrap();
        // 证据可追溯：引用 id 展开为证据库条目（§37）。
        assert_eq!(
            finding["evidence_resolved"][0]["note"],
            "1200 lines counted from bounded read"
        );

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn missing_db_and_missing_finding_are_honest_errors() {
        let state = McpState {
            db_path: std::path::PathBuf::from("Z:/definitely-missing/prism.db"),
        };
        let response = handle_message(
            &state,
            &json!({
                "jsonrpc": "2.0", "id": 8, "method": "tools/call",
                "params": {"name": "list_projects", "arguments": {}}
            }),
        )
        .unwrap();
        assert_eq!(response["result"]["isError"], true);
        assert!(response["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("no stored analysis results"));

        let path = temp_db("missing-finding");
        seed(&path);
        let state = state_with(&path);
        let response = handle_message(
            &state,
            &json!({
                "jsonrpc": "2.0", "id": 9, "method": "tools/call",
                "params": {"name": "get_finding",
                           "arguments": {"project_root": "C:/tmp/demo",
                                         "domain": "health2",
                                         "finding_id": "find:nope"}}
            }),
        )
        .unwrap();
        assert_eq!(response["result"]["isError"], true);
        assert!(response["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("finding not found"));
        let _ = std::fs::remove_file(&path);
    }
}
