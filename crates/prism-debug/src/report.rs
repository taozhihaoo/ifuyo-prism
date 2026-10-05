//! 分析编排（M210 §三十一 Parser Pipeline）：
//! `Sources → Parse → Redact(解析器内) → Fingerprint → Errors → Clusters
//! → Resolve Frames(Symbols/Scenes) → Report`。每步可追踪；零 IO（内容
//! 由调用方有界读取注入，§五 原则 2 / M29 collect 先例）。

use crate::model::{
    DebugError, DebugEvent, DebugReport, DebugRun, EventDomain, StackFrame, StackTrace,
    DEBUG_SCHEMA_VERSION,
};
use crate::parsers::{dedupe_events, logical_source_path, parse_source};
use prism_intel::identity::fnv1a_hex16;
use std::collections::BTreeMap;
use std::path::Path;

/// 分析输入（全部注入；不做文件系统访问）。
pub struct DebugAnalysisInput<'a> {
    /// 逻辑路径（项目相对，含轮转归并前）→ 有界读取内容。
    pub sources: &'a BTreeMap<String, String>,
    /// 事件总量上限（超出 ⇒ truncated + 诊断，§四十七）。
    pub max_events: usize,
}

/// 主入口：多源内容 → DebugReport。
pub fn analyze(input: &DebugAnalysisInput) -> DebugReport {
    let mut report = DebugReport {
        schema_version: DEBUG_SCHEMA_VERSION,
        ..Default::default()
    };
    let mut all_events: Vec<DebugEvent> = Vec::new();
    let mut source_ids: Vec<String> = Vec::new();
    for (path, content) in input.sources {
        let logical = logical_source_path(path);
        let (events, source) = parse_source(&logical, path, content, None);
        report.parser_failures +=
            u64::from(source.parse_state == crate::model::ParseState::Malformed);
        report.unsupported_sources +=
            u64::from(source.parse_state == crate::model::ParseState::Unsupported);
        if source.parse_state == crate::model::ParseState::Partial {
            report.diagnostics.push(format!(
                "{path}: partial parse — {}",
                source.diagnostics.join("; ")
            ));
        }
        source_ids.push(source.source_id.clone());
        report.redaction_count += source.redactions;
        report.sources.push(source);
        all_events.extend(events);
    }
    let events_total = all_events.len() as u64;
    let mut events = dedupe_events(all_events);
    report.events_total = events_total;
    if events.len() > input.max_events {
        report.diagnostics.push(format!(
            "events truncated: {} > cap {}",
            events.len(),
            input.max_events
        ));
        report.events_truncated = true;
        events.truncate(input.max_events);
    }
    report.events = events;
    report.runs.push(DebugRun {
        run_id: format!(
            "run:{}",
            fnv1a_hex16(&format!("{}|{}", source_ids.join(","), report.events_total))
        ),
        run_type: dominant_domain(&report.events),
        started: String::new(),
        source_ids,
        event_count: report.events.len() as u64,
    });
    rebuild_errors_and_clusters(&mut report);
    report
}

/// 指纹聚合（§八/§九）：Event → Error Signature → Cluster。
/// **不使用**行号/时间戳/GUID 作身份。
pub fn rebuild_errors_and_clusters(report: &mut DebugReport) {
    let mut by_fingerprint: BTreeMap<String, DebugError> = BTreeMap::new();
    for event in &report.events {
        let (exception_type, raw_message) = split_exception(&event.message);
        // 规范化（§八）只进指纹；例证消息保留 redacted 原文。
        let normalized_message = crate::normalize::normalize_message(&raw_message);
        let top_frames = crate::parsers::parse_frames_from_message(&event.message)
            .into_iter()
            .take(3)
            .map(|(call, _, _)| call)
            .collect::<Vec<_>>();
        let fingerprint = fingerprint_of(
            event.domain,
            exception_type.as_deref(),
            &normalized_message,
            event.code.as_deref(),
            &top_frames,
        );
        let entry = by_fingerprint
            .entry(fingerprint.clone())
            .or_insert_with(|| DebugError {
                error_id: format!("err:{fingerprint}"),
                fingerprint: fingerprint.clone(),
                domain: event.domain,
                exception_type: exception_type.clone(),
                normalized_message: normalized_message.clone(),
                example_message: event.message.clone(),
                code: event.code.clone(),
                top_frames: top_frames.clone(),
                stack_trace: None,
                occurrences: 0,
                occurrences_detail: Vec::new(),
                first_observed: None,
                last_observed: None,
                redaction_count: 0,
            });
        entry.occurrences += 1;
        if entry.occurrences_detail.len() < 5 {
            entry
                .occurrences_detail
                .push((event.source_id.clone(), event.line));
        }
        match (&entry.first_observed, &event.timestamp) {
            (None, Some(timestamp)) => entry.first_observed = Some(timestamp.clone()),
            (Some(first), Some(timestamp)) if timestamp.as_str() < first.as_str() => {
                entry.first_observed = Some(timestamp.clone())
            }
            _ => {}
        }
        match (&entry.last_observed, &event.timestamp) {
            (None, Some(timestamp)) => entry.last_observed = Some(timestamp.clone()),
            (Some(last), Some(timestamp)) if timestamp.as_str() > last.as_str() => {
                entry.last_observed = Some(timestamp.clone())
            }
            _ => {}
        }
    }
    let mut errors: Vec<DebugError> = by_fingerprint.into_values().collect();
    errors.sort_by(|a, b| a.error_id.cmp(&b.error_id));
    report.clusters = DebugReport::build_clusters(&errors);
    report.errors = errors;
}

/// 异常形态消息拆分：`Unhandled exception. TYPE: MSG` / `---> TYPE: MSG`。
fn split_exception(message: &str) -> (Option<String>, String) {
    let first_line = message.lines().next().unwrap_or(message);
    for prefix in ["Unhandled exception. ", "---> ", "SCRIPT ERROR: "] {
        if let Some(rest) = first_line.strip_prefix(prefix) {
            if let Some((type_part, msg)) = rest.split_once(": ") {
                return (Some(type_part.trim().to_string()), msg.to_string());
            }
            return (Some(rest.trim().to_string()), String::new());
        }
    }
    // Godot/运行时日志常直接以 `TYPE: MSG` 开头（级别前缀已剥）。
    if let Some((type_part, msg)) = first_line.split_once(": ") {
        let candidate = type_part.trim();
        let dotted = !candidate.is_empty()
            && candidate.contains('.')
            && candidate
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'+' || b == b'_');
        if dotted {
            return (Some(candidate.to_string()), msg.to_string());
        }
    }
    (None, message.to_string())
}

/// 指纹（§九）：domain|exception|normalized message|code|top frames。
fn fingerprint_of(
    domain: EventDomain,
    exception_type: Option<&str>,
    normalized_message: &str,
    code: Option<&str>,
    top_frames: &[String],
) -> String {
    let frames = top_frames.join(";");
    format!(
        "fp:{}",
        fnv1a_hex16(&format!(
            "{domain:?}|{}|{normalized_message}|{}|{frames}",
            exception_type.unwrap_or(""),
            code.unwrap_or("")
        ))
    )
}

/// 帧解析 + Symbol/Scene 解析（注入引擎后调用；§三十一 Pipeline 尾段）。
pub fn resolve_report_frames(
    report: &mut DebugReport,
    index: Option<&prism_csharp::SymbolIndex>,
    godot: Option<&prism_godot::GodotIndex>,
    project_root: &Path,
) {
    report.symbol_resolution_available = index.is_some();
    let mut resolved_total = 0usize;
    for error in &mut report.errors {
        // 以错误消息里的 `at` 行构建帧（例证即第一条事件的消息）。
        let parsed = crate::parsers::parse_frames_from_message(&error.example_message);
        if parsed.is_empty() {
            continue;
        }
        let mut frames: Vec<StackFrame> = parsed
            .into_iter()
            .enumerate()
            .map(|(frame_index, (call, file, line))| {
                let (type_name, method) = call
                    .rsplit_once('.')
                    .map_or((None, call.clone()), |(prefix, method)| {
                        (Some(prefix.to_string()), method.to_string())
                    });
                StackFrame {
                    index: frame_index,
                    module: None,
                    type_name,
                    method: Some(method),
                    file_path: file,
                    line,
                    column: None,
                    resolution: crate::model::FrameResolution {
                        status: crate::model::ResolutionStatus::Unavailable,
                        match_state: crate::model::MatchState::Unsupported,
                        symbol_id: None,
                        scene: None,
                        node_path: None,
                        reasoning: String::new(),
                    },
                }
            })
            .collect();
        resolved_total += crate::resolve::resolve_frames(&mut frames, index, godot, project_root);
        let trace_id = format!(
            "trace:{}",
            fnv1a_hex16(
                &frames
                    .iter()
                    .map(|frame| frame.method.clone().unwrap_or_default())
                    .collect::<Vec<_>>()
                    .join(">")
            )
        );
        error.stack_trace = Some(StackTrace { trace_id, frames });
    }
    if index.is_some() {
        let eligible = report
            .errors
            .iter()
            .flat_map(|error| error.stack_trace.as_ref().map(|trace| trace.frames.len()))
            .sum::<usize>();
        if eligible > 0 {
            report.diagnostics.push(format!(
                "stack frame resolution coverage: {resolved_total}/{eligible} eligible frames resolved (resolved/eligible)"
            ));
        }
    }
}

fn dominant_domain(events: &[DebugEvent]) -> EventDomain {
    let mut counts: BTreeMap<EventDomain, usize> = BTreeMap::new();
    for event in events {
        *counts.entry(event.domain).or_default() += 1;
    }
    counts
        .into_iter()
        .max_by_key(|(domain, count)| (*count, std::cmp::Reverse(*domain as u8)))
        .map(|(domain, _)| domain)
        .unwrap_or(EventDomain::Unknown)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map(entries: &[(&str, &str)]) -> BTreeMap<String, String> {
        entries
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect()
    }

    #[test]
    fn same_error_aggregates_into_one_signature() {
        // §九：1000 次 NullReference 不能是 1000 个新问题。
        let content = (0..50)
            .map(|index| {
                format!("Unhandled exception. System.NullReferenceException: Object ref at save/{index}.dat")
            })
            .collect::<Vec<_>>()
            .join("\n");
        let sources = map(&[("godot.log", content.as_str())]);
        let report = analyze(&DebugAnalysisInput {
            sources: &sources,
            max_events: 1000,
        });
        assert_eq!(report.events.len(), 50);
        assert_eq!(report.errors.len(), 1, "数字归一 ⇒ 同一签名");
        assert_eq!(
            report.errors[0].exception_type.as_deref(),
            Some("System.NullReferenceException")
        );
        assert_eq!(report.errors[0].occurrences, 50);
        assert!(report.errors[0].normalized_message.contains("<PATH>"));
        assert_eq!(report.clusters.len(), 1);
        assert_eq!(report.clusters[0].occurrences, 50);
        assert!(report.clusters[0].explanation.same_exception_type);
    }

    #[test]
    fn genuinely_different_errors_stay_separate() {
        let sources = map(&[
            (
                "a.log",
                "Unhandled exception. System.NullReferenceException: obj null\nUnhandled exception. System.IO.IOException: disk full\n",
            ),
        ]);
        let report = analyze(&DebugAnalysisInput {
            sources: &sources,
            max_events: 100,
        });
        assert_eq!(report.errors.len(), 2);
        assert_eq!(report.clusters.len(), 2);
    }

    #[test]
    fn fingerprint_stable_across_line_numbers_and_runs() {
        let sources_a = map(&[("m.log", "[ERR] connection refused to 10.0.0.1\n")]);
        let sources_b = map(&[("m.log", "\n\n[ERR] connection refused to 10.0.0.99\n")]);
        let report_a = analyze(&DebugAnalysisInput {
            sources: &sources_a,
            max_events: 10,
        });
        let report_b = analyze(&DebugAnalysisInput {
            sources: &sources_b,
            max_events: 10,
        });
        assert_eq!(
            report_a.errors[0].fingerprint, report_b.errors[0].fingerprint,
            "§九：行号/具体数字不影响指纹"
        );
    }

    #[test]
    fn truncation_is_visible() {
        let content = (0..200)
            .map(|index| format!("[ERR] thing {index}\n"))
            .collect::<Vec<_>>()
            .join("\n");
        let sources = map(&[("big.log", content.as_str())]);
        let report = analyze(&DebugAnalysisInput {
            sources: &sources,
            max_events: 30,
        });
        assert!(report.events_truncated);
        assert_eq!(report.events.len(), 30);
        assert_eq!(report.events_total, 200);
        assert!(report.diagnostics.iter().any(|d| d.contains("truncated")));
    }

    #[test]
    fn secrets_never_enter_error_messages() {
        // TEST FIXTURE ONLY。
        let sources = map(&[("a.log", "[ERR] login failed key=AKIAABCDEFGHIJKLMNOP\n")]);
        let report = analyze(&DebugAnalysisInput {
            sources: &sources,
            max_events: 10,
        });
        let rendered = format!("{:?}", report);
        assert!(
            !rendered.contains("ABCDEFGHIJKLMNOP"),
            "§二十八：原文不得进入报告"
        );
        assert!(rendered.contains("…("), "候选在报告内保持 redacted 表示");
    }

    #[test]
    fn run_and_metrics_summary() {
        let sources = map(&[
            ("godot.log", "ERROR: Cannot open file 'res://x.tscn'\n"),
            ("build.log", "a.cs(1,1): error CS0246: missing\n"),
        ]);
        let report = analyze(&DebugAnalysisInput {
            sources: &sources,
            max_events: 100,
        });
        assert_eq!(report.sources.len(), 2);
        assert_eq!(report.runs.len(), 1);
        assert_eq!(report.runs[0].event_count, 2);
    }
}
