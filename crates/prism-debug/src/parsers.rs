//! Debug 解析器（M210 §三十-§三十三）。
//!
//! 手写逐行解析（无 regex 依赖；日志是**不可信输入** §二十八：只解析
//! 数据；ANSI 控制符剥离；malformed ⇒ Partial/Malformed + 诊断，绝不
//! 产出"无错误"）。

use crate::model::{DebugEvent, DebugSource, EventDomain, NormalizedLevel, ParseState, SourceType};
use prism_intel::identity::fnv1a_hex16;
use std::collections::BTreeMap;

/// 消息内换行（以数值形式构造，避免工具链转义歧义）。
const NEWLINE_CHAR: char = '\u{000a}';

/// 事件/消息上限（§四十七 资源限制；超出 ⇒ truncated + 诊断）。
pub const MAX_MESSAGE_CHARS: usize = 2000;
pub const MAX_EVENTS: usize = 50_000;

/// 解析器种类（§三十：按真实生态优先，不建空壳 Provider）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParserKind {
    Godot,
    DotNetException,
    BuildOutput,
    TestOutput,
    GenericTextLog,
}

impl ParserKind {
    pub const fn name(self) -> &'static str {
        match self {
            ParserKind::Godot => "godot-log/1",
            ParserKind::DotNetException => "dotnet-exception/1",
            ParserKind::BuildOutput => "build-output/1",
            ParserKind::TestOutput => "test-output/1",
            ParserKind::GenericTextLog => "generic-text-log/1",
        }
    }

    pub const fn source_type(self) -> SourceType {
        match self {
            ParserKind::Godot => SourceType::GodotLog,
            ParserKind::DotNetException => SourceType::DotNetException,
            ParserKind::BuildOutput => SourceType::BuildOutput,
            ParserKind::TestOutput => SourceType::TestOutput,
            ParserKind::GenericTextLog => SourceType::LogFile,
        }
    }

    pub const fn domain(self) -> EventDomain {
        match self {
            ParserKind::Godot | ParserKind::DotNetException => EventDomain::Runtime,
            ParserKind::BuildOutput => EventDomain::Build,
            ParserKind::TestOutput => EventDomain::Test,
            ParserKind::GenericTextLog => EventDomain::Unknown,
        }
    }
}

/// 内容嗅探（§三十 Source Detection；顺序即优先级）。
pub fn detect_parser(content: &str) -> ParserKind {
    if content.contains("Unhandled exception.")
        || content.contains("\n   at ")
        || content.contains("---> ")
    {
        return ParserKind::DotNetException;
    }
    if content.contains("SCRIPT ERROR:") || content.contains("Godot Engine") {
        return ParserKind::Godot;
    }
    if content.contains("ERROR: ") && content.contains("res://") {
        return ParserKind::Godot;
    }
    if content.contains(": error CS")
        || content.contains("error[E")
        || content.contains(": error TS")
    {
        return ParserKind::BuildOutput;
    }
    if content.contains("Failed ") || content.contains("\nFAIL ") {
        return ParserKind::TestOutput;
    }
    ParserKind::GenericTextLog
}

/// 单源解析入口：返回 (events, source 解析元数据)。
pub fn parse_source(
    logical_path: &str,
    physical_path: &str,
    content: &str,
    explicit_domain: Option<EventDomain>,
) -> (Vec<DebugEvent>, DebugSource) {
    let kind = match explicit_domain {
        Some(EventDomain::Build) => ParserKind::BuildOutput,
        Some(EventDomain::Test) => ParserKind::TestOutput,
        Some(EventDomain::Runtime) => ParserKind::Godot,
        _ => detect_parser(content),
    };
    let mut diagnostics = Vec::new();
    let source_id: String = format!("src:{}", fnv1a_hex16(logical_path));
    let (mut events, state, redactions) = match kind {
        // 注意：source_id 以 &str 借用传入（所有权留在 DebugSource）。
        ParserKind::DotNetException => parse_dotnet(source_id.as_str(), content, &mut diagnostics),
        ParserKind::Godot => parse_godot(source_id.as_str(), content, &mut diagnostics),
        ParserKind::BuildOutput => parse_build(source_id.as_str(), content, &mut diagnostics),
        ParserKind::TestOutput => parse_test(source_id.as_str(), content, &mut diagnostics),
        ParserKind::GenericTextLog => parse_generic(source_id.as_str(), content, &mut diagnostics),
    };
    if events.len() >= MAX_EVENTS {
        events.truncate(MAX_EVENTS);
        diagnostics.push(format!("event cap {MAX_EVENTS} reached; truncated"));
    }
    if content.trim().is_empty() {
        diagnostics.push("source is empty".to_string());
    }
    let source = DebugSource {
        source_id: source_id.to_string(),
        source_type: kind.source_type(),
        logical_path: logical_path.to_string(),
        physical_path: physical_path.to_string(),
        content_hash: fnv1a_hex16(content),
        bytes: content.len() as u64,
        parser: kind.name().to_string(),
        parse_state: state,
        redactions,
        diagnostics,
    };
    (events, source)
}

/// 剥离 ANSI 控制序列（§三十三：彩色终端输出）。
fn strip_ansi(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            // ESC [ ... 字母 —— 跳到终止字母。
            if chars.peek() == Some(&'[') {
                chars.next();
                for n in chars.by_ref() {
                    if n.is_ascii_alphabetic() {
                        break;
                    }
                }
                continue;
            }
            continue;
        }
        if c == '\u{7}' || c == '\r' {
            continue;
        }
        out.push(c);
    }
    out
}

fn make_event(
    source_id: &str,
    line_no: usize,
    timestamp: Option<String>,
    level: NormalizedLevel,
    original_level: Option<String>,
    domain: EventDomain,
    raw_message: &str,
) -> (DebugEvent, u64) {
    let (message, redactions) = crate::redact::redact_line(raw_message);
    let message = truncate_message(&message);
    let normalized = crate::normalize::normalize_message(&message);
    let event_id = format!(
        "evt:{}",
        fnv1a_hex16(&format!("{source_id}|{line_no}|{normalized}"))
    );
    (
        DebugEvent {
            event_id,
            source_id: source_id.to_string(),
            line: line_no,
            timestamp,
            level,
            original_level,
            domain,
            message,
            test_name: None,
            code: None,
            file: None,
            line_in_file: None,
        },
        redactions as u64,
    )
}

fn truncate_message(message: &str) -> String {
    if message.chars().count() <= MAX_MESSAGE_CHARS {
        return message.to_string();
    }
    message.chars().take(MAX_MESSAGE_CHARS).collect()
}

/// 拆出可选时间戳前缀（ISO 形 / `YYYY-MM-DD HH:MM:SS` 形）。
fn split_timestamp(line: &str) -> (Option<String>, &str) {
    let bytes = line.as_bytes();
    // 最短 `2026-10-6 1:2:3` 也有 8 字符；只认数字开头 + 日期形态。
    if bytes.len() < 8 || !bytes[0].is_ascii_digit() {
        return (None, line);
    }
    for candidate in [19usize, 23usize] {
        if line.len() >= candidate
            && bytes[4] == b'-'
            && bytes[7] == b'-'
            && (bytes[10] == b' ' || bytes[10] == b'T')
        {
            let (timestamp, rest) = line.split_at(candidate);
            return (Some(timestamp.trim().to_string()), rest.trim_start());
        }
    }
    (None, line)
}

/// 拆出级别前缀（`[ERR]` / `ERROR:` / `WRN ` 形态）。
fn split_level(rest: &str) -> (NormalizedLevel, Option<String>, &str) {
    let trimmed = rest.trim_start();
    if let Some(close) = trimmed.find(']') {
        if trimmed.starts_with('[') && close <= 12 {
            let raw = &trimmed[1..close];
            let level = NormalizedLevel::normalize(raw);
            if level != NormalizedLevel::Unknown {
                return (
                    level,
                    Some(raw.to_string()),
                    trimmed[close + 1..].trim_start(),
                );
            }
        }
    }
    for (word, level) in [
        ("SCRIPT ERROR:", NormalizedLevel::Error),
        ("ERROR:", NormalizedLevel::Error),
        ("WARNING:", NormalizedLevel::Warning),
        ("FATAL:", NormalizedLevel::Critical),
        ("INFO:", NormalizedLevel::Info),
        ("DEBUG:", NormalizedLevel::Debug),
    ] {
        if let Some(stripped) = trimmed.strip_prefix(word) {
            return (
                level,
                Some(word.trim_end_matches(':').to_string()),
                stripped.trim_start(),
            );
        }
    }
    let first = trimmed.split_whitespace().next().unwrap_or("");
    let level = NormalizedLevel::normalize(first);
    if level != NormalizedLevel::Unknown && first.len() <= 8 {
        return (
            level,
            Some(first.to_string()),
            trimmed[first.len()..].trim_start(),
        );
    }
    (NormalizedLevel::Unknown, None, rest)
}

/// 通用文本日志（兜底；逐行事件，无级别行并入上一条消息——§三十三
/// 多行消息/交错输出）。
fn parse_generic(
    source_id: &str,
    content: &str,
    diagnostics: &mut Vec<String>,
) -> (Vec<DebugEvent>, ParseState, u64) {
    let mut events: Vec<DebugEvent> = Vec::new();
    let mut redactions = 0u64;
    for (index, raw_line) in content.lines().enumerate() {
        let line = strip_ansi(raw_line);
        if line.trim().is_empty() {
            continue;
        }
        let (timestamp, rest) = split_timestamp(&line);
        let (level, original, message) = split_level(rest);
        if level == NormalizedLevel::Unknown && !events.is_empty() && !message.is_empty() {
            // 无级别/时间戳 ⇒ 上一条事件的续行（多行消息）。
            let last = events.last_mut().expect("non-empty checked");
            if last.message.chars().count() < MAX_MESSAGE_CHARS {
                // 续行独立成行（dotnet 帧解析按行切分）。
                last.message.push(NEWLINE_CHAR);
                last.message.push_str(&truncate_message(message));
            }
            continue;
        }
        let (event, count) = make_event(
            source_id,
            index + 1,
            timestamp,
            level,
            original,
            EventDomain::Unknown,
            message,
        );
        redactions += count;
        events.push(event);
    }
    if events.is_empty() && !content.trim().is_empty() {
        diagnostics.push("no parsable events in non-empty source".to_string());
        return (events, ParseState::Malformed, redactions);
    }
    (events, ParseState::Complete, redactions)
}

/// .NET 异常块（§十/§六十一 caused-by/InnerException）：
/// `Unhandled exception. TYPE: MSG` + `   at Frame`（`in path:line` 可选）
/// + `---> INNER TYPE: MSG` 嵌套。
pub fn parse_dotnet(
    source_id: &str,
    content: &str,
    diagnostics: &mut Vec<String>,
) -> (Vec<DebugEvent>, ParseState, u64) {
    let mut events: Vec<DebugEvent> = Vec::new();
    let mut seen_exception = false;
    let mut redactions = 0u64;
    for (index, raw_line) in content.lines().enumerate() {
        let line = strip_ansi(raw_line);
        let trimmed = line.trim_start();
        let line_no = index + 1;
        if trimmed.starts_with("at ") {
            // 栈帧行：归入事件消息供上层解析帧（帧解析在 report 层统一做）。
            if let Some(last) = events.last_mut() {
                last.message.push('\n');
                last.message.push_str(trimmed);
            }
            continue;
        }
        let is_exception_header = trimmed.starts_with("Unhandled exception.")
            || trimmed.starts_with("---> ")
            || (looks_like_exception_type(trimmed) && trimmed.contains(": "));
        if !is_exception_header {
            continue;
        }
        seen_exception = true;
        let (event, count) = make_event(
            source_id,
            line_no,
            None,
            NormalizedLevel::Error,
            Some("EXCEPTION".to_string()),
            EventDomain::Runtime,
            trimmed,
        );
        redactions += count;
        events.push(event);
    }
    if !seen_exception {
        diagnostics.push("no exception block found".to_string());
        return (events, ParseState::Malformed, redactions);
    }
    (events, ParseState::Complete, redactions)
}

/// `Namespace.Type.Method` 形态嗅探（避免把普通 ERROR 行当异常头）。
fn looks_like_exception_type(line: &str) -> bool {
    let Some((type_part, _)) = line.split_once(": ") else {
        return false;
    };
    let type_part = type_part.trim_start_matches("---> ").trim();
    !type_part.is_empty()
        && type_part
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'`' || b == b'+' || b == b'_')
        && type_part.contains('.')
}

/// Godot 日志（§十二/§十三）：`ERROR:`/`SCRIPT ERROR:`/`WARNING:` +
/// `at: Func (res://path:line)` 尾缀；常见运行时错误原文保留在消息里
/// （规则层按前缀识别，不硬编码控制台文本清单——§十三）。
fn parse_godot(
    source_id: &str,
    content: &str,
    diagnostics: &mut Vec<String>,
) -> (Vec<DebugEvent>, ParseState, u64) {
    let mut events: Vec<DebugEvent> = Vec::new();
    let mut matched = 0usize;
    let mut redactions = 0u64;
    for (index, raw_line) in content.lines().enumerate() {
        let line = strip_ansi(raw_line);
        let trimmed = line.trim_start();
        if trimmed.is_empty() {
            continue;
        }
        let (level, original, message) = split_level(trimmed);
        // 帧续行（`at: ...`）永远并入上一条——即便位置里含 res://。
        let is_frame_continuation = trimmed.starts_with("at:") && !events.is_empty();
        // 引擎横幅（"Godot Engine v4.3"）只是识别线索，不是事件。
        let is_godot = !is_frame_continuation
            && (original
                .as_deref()
                .map(|o| o.ends_with("ERROR") || o.ends_with("WARNING") || o == "INFO")
                .unwrap_or(false)
                || message.contains("res://"));
        if !is_godot {
            // 引擎日志里的杂项行：并入上一条（如有），否则跳过。
            if let Some(last) = events.last_mut() {
                // 帧行独立成行（帧解析按行切分）。
                last.message.push(NEWLINE_CHAR);
                last.message.push_str(trimmed);
            }
            continue;
        }
        matched += 1;
        let (event, count) = make_event(
            source_id,
            index + 1,
            None,
            level,
            original,
            EventDomain::Runtime,
            message,
        );
        redactions += count;
        events.push(event);
    }
    if matched == 0 {
        diagnostics.push("no Godot-style lines found; parse may be wrong".to_string());
        return (events, ParseState::Partial, redactions);
    }
    (events, ParseState::Complete, redactions)
}

/// 构建输出（§十四/§十五）：`path(line,col): error CS0246: msg`（MSBuild）
/// 与 `error[E0308]: msg` + ` --> path:line:col`（cargo）。
fn parse_build(
    source_id: &str,
    content: &str,
    diagnostics: &mut Vec<String>,
) -> (Vec<DebugEvent>, ParseState, u64) {
    let mut events: Vec<DebugEvent> = Vec::new();
    let mut matched = 0usize;
    let mut redactions = 0u64;
    for (index, raw_line) in content.lines().enumerate() {
        let line = strip_ansi(raw_line);
        let trimmed = line.trim();
        let line_no = index + 1;
        // MSBuild 形态。
        if let Some(open) = trimmed.find('(') {
            if let Some(close) = trimmed[open..].find(')') {
                let close = open + close;
                let path = &trimmed[..open];
                let position = &trimmed[open + 1..close];
                // 从闭合括号之后开始（括号内是行列位置）。
                let rest = trimmed[close..].trim_start_matches(')').trim_start();
                if let Some(rest) = rest.strip_prefix(':') {
                    let rest = rest.trim_start();
                    let (severity_word, after) = rest.split_once(' ').unwrap_or((rest, ""));
                    let is_error = severity_word.eq_ignore_ascii_case("error");
                    let is_warning = severity_word.eq_ignore_ascii_case("warning");
                    if is_error || is_warning {
                        matched += 1;
                        let (code, message) = after.split_once(": ").unwrap_or(("", after));
                        let (mut event, count) = make_event(
                            source_id,
                            line_no,
                            None,
                            if is_error {
                                NormalizedLevel::Error
                            } else {
                                NormalizedLevel::Warning
                            },
                            Some(severity_word.to_string()),
                            EventDomain::Build,
                            message,
                        );
                        redactions += count;
                        event.code = if code.is_empty() {
                            None
                        } else {
                            Some(code.to_string())
                        };
                        event.file = Some(path.to_string());
                        event.line_in_file =
                            position.split(',').next().and_then(|n| n.parse().ok());
                        events.push(event);
                        continue;
                    }
                }
            }
        }
        // cargo 形态：`error[E0308]: msg` / ` --> src/x.rs:12:34`。
        if let Some(rest) = trimmed.strip_prefix("error[") {
            matched += 1;
            let (code, message) = rest.split_once("]: ").unwrap_or(("", rest));
            let (mut event, count) = make_event(
                source_id,
                line_no,
                None,
                NormalizedLevel::Error,
                Some("error".to_string()),
                EventDomain::Build,
                message,
            );
            redactions += count;
            event.code = Some(code.to_string());
            events.push(event);
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("warning: ") {
            matched += 1;
            let (event, count) = make_event(
                source_id,
                line_no,
                None,
                NormalizedLevel::Warning,
                Some("warning".to_string()),
                EventDomain::Build,
                rest,
            );
            redactions += count;
            events.push(event);
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("--> ") {
            // cargo 的位置行跟随错误行 ⇒ 回填最近一条无位置的事件。
            let parts: Vec<&str> = rest.split(':').collect();
            if parts.len() >= 2 {
                // 位置行无对应事件时不回填——回填条件（code.is_some）
                // 保证只命中最近的构建错误事件。
                let file = parts[0].to_string();
                let line_in_file = parts[1].parse().unwrap_or(0);
                if let Some(last) = events
                    .iter_mut()
                    .rev()
                    .find(|event| event.file.is_none() && event.code.is_some())
                {
                    last.file = Some(file);
                    last.line_in_file = Some(line_in_file);
                }
            }
            continue;
        }
    }
    if matched == 0 {
        diagnostics.push("no compiler diagnostics found in build output".to_string());
        return (events, ParseState::Partial, redactions);
    }
    (events, ParseState::Complete, redactions)
}

/// 测试输出（§十六）：`Failed <name>`（xUnit）与 `FAIL <name>`（pytest/
/// node 形态）；错误详情并入消息。
fn parse_test(
    source_id: &str,
    content: &str,
    diagnostics: &mut Vec<String>,
) -> (Vec<DebugEvent>, ParseState, u64) {
    let mut events: Vec<DebugEvent> = Vec::new();
    let mut matched = 0usize;
    let mut redactions = 0u64;
    for (index, raw_line) in content.lines().enumerate() {
        let line = strip_ansi(raw_line);
        let trimmed = line.trim();
        let mut failed_name: Option<String> = None;
        if let Some(rest) = trimmed.strip_prefix("Failed ") {
            let name = rest.split_whitespace().next().unwrap_or("");
            if !name.is_empty() {
                failed_name = Some(name.to_string());
            }
        } else if let Some(rest) = trimmed.strip_prefix("FAIL ") {
            let name = rest.split_whitespace().next().unwrap_or("");
            if !name.is_empty() {
                failed_name = Some(name.to_string());
            }
        }
        if let Some(name) = failed_name {
            matched += 1;
            let (mut event, count) = make_event(
                source_id,
                index + 1,
                None,
                NormalizedLevel::Error,
                Some("FAILED".to_string()),
                EventDomain::Test,
                trimmed,
            );
            redactions += count;
            event.test_name = Some(name);
            events.push(event);
            continue;
        }
        // 失败详情行并入最近一条测试事件。
        if let Some(last) = events.last_mut() {
            if trimmed.starts_with("Expected:")
                || trimmed.starts_with("Actual:")
                || trimmed.starts_with("Exception:")
            {
                last.message.push('\n');
                last.message.push_str(trimmed);
            }
        }
    }
    if matched == 0 {
        diagnostics.push("no test failures found in test output".to_string());
        return (events, ParseState::Partial, redactions);
    }
    (events, ParseState::Complete, redactions)
}

/// 从 .NET 事件消息解析栈帧（§十；帧文本在消息内 `at ...` 行）。
pub fn parse_frames_from_message(message: &str) -> Vec<(String, Option<String>, Option<u32>)> {
    let mut frames = Vec::new();
    for line in message.lines() {
        let trimmed = line.trim();
        // dotnet at 帧行 + Godot at: 帧行（res:// 位置）双形态。
        let rest = match trimmed
            .strip_prefix("at ")
            .or_else(|| trimmed.strip_prefix("at: "))
        {
            Some(rest) => rest,
            None => continue,
        };
        // `Namespace.Type.Method(args) in C:\path\File.cs:line 218`
        // `Namespace.Type.Method(args) in /path/File.cs:218`
        let (call, location) = match rest.split_once(" in ") {
            Some((call, location)) => (call.trim(), Some(location.trim())),
            None => match rest.rfind('(') {
                // Godot 形态：`Func (res://path:line)` —— 尾括号组即位置。
                Some(at) if rest.ends_with(')') => {
                    (rest[..at].trim(), Some(&rest[at + 1..rest.len() - 1]))
                }
                _ => (rest.trim(), None),
            },
        };
        let (file_path, line_no) = parse_frame_location(location);
        frames.push((call.to_string(), file_path, line_no));
    }
    frames
}

/// `C:\path\File.cs:line 218` / `/path/File.cs:218` / `File.cs:218,5`。
fn parse_frame_location(location: Option<&str>) -> (Option<String>, Option<u32>) {
    let Some(location) = location else {
        return (None, None);
    };
    let location = location.trim_start_matches('&').trim().trim_matches('"');
    let (path, line_part) = if let Some((path, rest)) = location.split_once(":line ") {
        (path.to_string(), rest.to_string())
    } else {
        match location.rsplit_once(':') {
            Some((path, line)) if line.bytes().all(|b| b.is_ascii_digit() || b == b',') => {
                (path.to_string(), line.to_string())
            }
            _ => (location.to_string(), String::new()),
        }
    };
    let line_no = line_part
        .split(',')
        .next()
        .and_then(|part| part.trim().parse().ok());
    (Some(path), line_no)
}

/// 逻辑源归并（§三十五 rotation：app.log.1 → 逻辑 app.log）。
pub fn logical_source_path(path: &str) -> String {
    for suffix in [".1", ".2", ".3", ".4", ".5", ".6", ".7", ".8", ".9"] {
        if let Some(stripped) = path.strip_suffix(suffix) {
            return stripped.to_string();
        }
    }
    path.to_string()
}

/// 事件按 (source, line) 去重合并（§四十六 抑制的底层保证）。
pub fn dedupe_events(events: Vec<DebugEvent>) -> Vec<DebugEvent> {
    let mut seen = std::collections::BTreeSet::new();
    let mut out = Vec::new();
    for event in events {
        if seen.insert(event.event_id.clone()) {
            out.push(event);
        }
    }
    out
}

/// 多源合并（BTreeMap 按物理路径 ⇒ 确定性顺序，§四十八）。
pub type SourceContents = BTreeMap<String, String>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detection_routes_by_content() {
        assert_eq!(
            detect_parser("Unhandled exception. System.NullReferenceException: x"),
            ParserKind::DotNetException
        );
        assert_eq!(
            detect_parser("Godot Engine v4.3\nERROR: Cannot open file 'res://x.tscn'"),
            ParserKind::Godot
        );
        assert_eq!(
            detect_parser("Scripts/A.cs(12,5): error CS0246: The type missing"),
            ParserKind::BuildOutput
        );
        assert_eq!(
            detect_parser("Failed App.Tests.Battle"),
            ParserKind::TestOutput
        );
        assert_eq!(
            detect_parser("[INFO] service started"),
            ParserKind::GenericTextLog
        );
    }

    #[test]
    fn generic_log_levels_timestamps_and_continuation() {
        let content = "2026-10-06 01:02:03 [ERR] connection refused\n    at retry (net.rs:10)\n2026-10-06 01:02:04 INFO: started\n";
        let (events, source) = parse_source("logs/app.log", "logs/app.log", content, None);
        assert_eq!(source.parser, "generic-text-log/1");
        assert_eq!(source.parse_state, ParseState::Complete);
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].level, NormalizedLevel::Error);
        assert_eq!(events[0].original_level.as_deref(), Some("ERR"));
        assert_eq!(events[0].timestamp.as_deref(), Some("2026-10-06 01:02:03"));
        assert!(events[0].message.contains("connection refused"));
        assert!(
            events[0].message.contains("at retry (net.rs:10)"),
            "续行并入上一条"
        );
        assert_eq!(events[1].level, NormalizedLevel::Info);
    }

    #[test]
    fn ansi_and_redaction_in_generic_lines() {
        // TEST FIXTURE ONLY — 合成假凭据。
        let content = "\u{1b}[31m[ERR]\u{1b}[0m auth failed with AKIAABCDEFGHIJKLMNOP\n";
        let (events, _) = parse_source("a.log", "a.log", content, None);
        assert_eq!(events.len(), 1);
        assert!(
            !events[0].message.contains("ABCDEFGHIJKLMNOP"),
            "原文不得进入事件"
        );
        assert!(events[0].message.contains("…("));
    }

    #[test]
    fn dotnet_exception_with_frames_and_inner() {
        let content = "Unhandled exception. System.NullReferenceException: Object reference not set\n   at App.Battle.ResolveCard(System.String card) in E:/proj/Scripts/Battle.cs:line 218\n   at App.Battle.Update() in E:/proj/Scripts/Battle.cs:line 90\n---> System.IO.IOException: disk full\n   at App.IO.Save() in E:/proj/Scripts/IO.cs:line 12\n";
        let (events, source) = parse_source("godot.log", "godot.log", content, None);
        assert_eq!(source.parser, "dotnet-exception/1");
        assert_eq!(events.len(), 2, "主异常 + 内层异常");
        let (main, inner) = (&events[0], &events[1]);
        assert!(main.message.contains("NullReferenceException"));
        assert!(
            main.message.contains("at App.Battle.ResolveCard"),
            "帧行并入主异常"
        );
        assert!(inner.message.contains("---> System.IO.IOException"));
        let frames = parse_frames_from_message(&main.message);
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0].0, "App.Battle.ResolveCard(System.String card)");
        // 位置解析：`E:/proj/Scripts/Battle.cs:line 218`。
        let (file, line) = parse_frame_location(Some("E:/proj/Scripts/Battle.cs:line 218"));
        assert_eq!(file.as_deref(), Some("E:/proj/Scripts/Battle.cs"));
        assert_eq!(line, Some(218));
    }

    #[test]
    fn godot_script_error_and_at_suffix() {
        let content = "Godot Engine v4.3\nSCRIPT ERROR: Invalid get index 'card' on base Node\n   at: ResolveCard (res://Scripts/Battle/BattleController.cs:218)\nERROR: Cannot open file 'res://Scenes/Missing.tscn'\n";
        let (events, source) = parse_source("godot.log", "godot.log", content, None);
        assert_eq!(source.parser, "godot-log/1");
        assert_eq!(source.parse_state, ParseState::Complete);
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].level, NormalizedLevel::Error);
        assert!(events[0].message.contains("Invalid get index"));
        assert!(events[0]
            .message
            .contains("res://Scripts/Battle/BattleController.cs:218"));
        assert!(events[1].message.contains("Cannot open file"));
    }

    #[test]
    fn build_output_msbuild_and_cargo() {
        let msbuild = "Scripts/A.cs(12,5): error CS0246: The type or namespace 'Foo' could not be found\nScripts/B.cs(3,1): warning CS0168: variable declared but never used\n";
        let (events, source) =
            parse_source("build.log", "build.log", msbuild, Some(EventDomain::Build));
        assert_eq!(source.parser, "build-output/1");
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].code.as_deref(), Some("CS0246"));
        assert_eq!(events[0].file.as_deref(), Some("Scripts/A.cs"));
        assert_eq!(events[0].line_in_file, Some(12));

        let cargo = "error[E0308]: mismatched types\n --> src/main.rs:12:34\n";
        let (events, _) = parse_source("build.log", "build.log", cargo, Some(EventDomain::Build));
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].code.as_deref(), Some("E0308"));
        assert_eq!(events[0].file.as_deref(), Some("src/main.rs"));
        assert_eq!(events[0].line_in_file, Some(12));
    }

    #[test]
    fn test_output_failures_with_details() {
        let content = "Failed App.Tests.Battle.ResolveCard [128 ms]\n  Expected: 3\n  Actual: 2\nException: System.Exception: boom\nFAIL test_battle\n";
        let (events, source) =
            parse_source("test.log", "test.log", content, Some(EventDomain::Test));
        assert_eq!(source.parser, "test-output/1");
        assert_eq!(events.len(), 2);
        assert_eq!(
            events[0].test_name.as_deref(),
            Some("App.Tests.Battle.ResolveCard")
        );
        assert!(events[0].message.contains("Expected: 3"));
        assert!(events[0]
            .message
            .contains("Exception: System.Exception: boom"));
        assert_eq!(events[1].test_name.as_deref(), Some("test_battle"));
    }

    #[test]
    fn malformed_and_empty_sources_are_honest() {
        let (_, source) = parse_source("x.log", "x.log", "   \n\t\n", None);
        assert_eq!(
            source.parse_state,
            ParseState::Complete,
            "空源 Complete + 诊断"
        );
        assert!(source.diagnostics.iter().any(|d| d.contains("empty")));
        let (events, source) = parse_source("y.log", "y.log", "random text no markers", None);
        // 通用解析器契约：任意非空文本都是事件（level Unknown 如实）。
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].level, NormalizedLevel::Unknown);
        assert_eq!(source.parse_state, ParseState::Complete);
    }

    #[test]
    fn rotation_merges_to_logical_path() {
        assert_eq!(logical_source_path("logs/app.log.1"), "logs/app.log");
        assert_eq!(logical_source_path("logs/app.log"), "logs/app.log");
    }

    #[test]
    fn frame_location_formats() {
        let (file, line) = parse_frame_location(Some("/p/File.cs:218"));
        assert_eq!(file.as_deref(), Some("/p/File.cs"));
        assert_eq!(line, Some(218));
        let (file, line) = parse_frame_location(Some("C:\\p\\File.cs:218,5"));
        assert_eq!(file.as_deref(), Some("C:\\p\\File.cs"));
        assert_eq!(line, Some(218));
        let (file, line) = parse_frame_location(None);
        assert!(file.is_none() && line.is_none());
    }
}
