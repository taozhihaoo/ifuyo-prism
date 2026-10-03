//! 结构化输出解析（规格 M8 §44-§47/§100-§103/§278-§291）。
//!
//! 保守原则（§101）：解析不确定 ⇒ location = unknown；解析失败（§288/§290）
//! 不 panic、不改变 execution outcome——保留 exit code 与原始输出并附
//! parser 诊断（§291：exit 0 + parser 失败 ⇒ execution Passed、analysis Partial）。

use serde::{Deserialize, Serialize};

/// 工具输出的归一化诊断（§100/§279）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolDiagnostic {
    /// 工具名（pytest / dotnet / junit 来源标注）。
    pub source: String,
    /// 规则/错误码；未知为空串。
    pub code: String,
    /// error / warning（保守：无法判定时 warning）。
    pub severity: String,
    pub message: String,
    /// "path:line:column"；不确定为 None（§101 不猜）。
    pub location: Option<String>,
}

/// 解析结果（§290/§291）：diagnostics + 解析是否成功。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ParseOutcome {
    pub diagnostics: Vec<ToolDiagnostic>,
    pub parser_error: Option<String>,
}

impl ParseOutcome {
    fn failed(message: &str) -> Self {
        ParseOutcome {
            diagnostics: Vec::new(),
            parser_error: Some(message.to_string()),
        }
    }
}

/// 解析 JUnit XML（`<testcase name=... classname=...>` + `<failure>/<error>`）。
/// 极简保守实现：不建完整 DOM，只扫描 `<testcase` 块。
pub fn parse_junit(xml: &str) -> ParseOutcome {
    let mut outcome = ParseOutcome::default();
    if !xml.contains("<testsuite") && !xml.contains("<testsuites") {
        return ParseOutcome::failed("no testsuite element found");
    }
    let mut rest = xml;
    while let Some(start) = rest.find("<testcase") {
        rest = &rest[start..];
        let Some(gt_rel) = rest.find('>') else {
            break;
        };
        // 自闭合（`/>`）与非自闭合（`</testcase>`）分别定界。
        let self_closing = rest[..=gt_rel].ends_with("/>");
        let open_tag = &rest[..=gt_rel];
        let name = extract_attr(open_tag, "name").unwrap_or_default();
        let classname = extract_attr(open_tag, "classname").unwrap_or_default();
        let body = if self_closing {
            String::new()
        } else {
            match rest[gt_rel..].find("</testcase>") {
                Some(close_rel) => rest[gt_rel + 1..gt_rel + close_rel].to_string(),
                // 未见闭合标签 ⇒ 视为自闭合（保守：不跨元素搜失败标记）。
                None => String::new(),
            }
        };
        let failed = body.contains("<failure") || body.contains("<error");
        let location = (!classname.is_empty()).then(|| classname.replace("::", "/"));
        outcome.diagnostics.push(ToolDiagnostic {
            source: "junit".to_string(),
            code: String::new(),
            severity: if failed { "error" } else { "warning" }.to_string(),
            message: if failed {
                format!("testcase failed: {name}")
            } else {
                format!("testcase: {name}")
            },
            // §101：失败且可定位才给 location，否则 None（不猜）。
            location: if failed { location } else { None },
        });
        if self_closing {
            rest = &rest[gt_rel + 1..];
        } else {
            let close = rest[gt_rel..].find("</testcase>").map(|p| gt_rel + p);
            match close {
                Some(close_abs) => rest = &rest[close_abs + 11..],
                None => break,
            }
        }
    }
    if outcome.diagnostics.is_empty() {
        return ParseOutcome::failed("no testcase elements parsed");
    }
    outcome
}

fn extract_attr(tag: &str, name: &str) -> Option<String> {
    let pattern = format!("{name}=\"");
    let start = tag.find(&pattern)? + pattern.len();
    let rest = &tag[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

/// 解析 dotnet 编译器/测试错误行（§278）：
/// `path(line,column): error CODE: message`。
pub fn parse_dotnet_line(line: &str) -> Option<ToolDiagnostic> {
    let open = line.find('(')?;
    let close = line.find("): ")?;
    if open >= close {
        return None;
    }
    let path = &line[..open];
    if !path.ends_with(".cs") && !path.ends_with(".vb") && !path.ends_with(".fs") {
        return None;
    }
    let coords = &line[open + 1..close];
    let mut parts = coords.split(',');
    let line_no = parts.next()?.trim().parse::<u32>().ok()?;
    let column = parts.next().and_then(|c| c.trim().parse::<u32>().ok()).unwrap_or(1);
    let remainder = &line[close + 3..];
    let (severity, code_and_message) = remainder.split_once(' ')?;
    let severity = severity.trim_end_matches(':');
    if severity != "error" && severity != "warning" {
        return None;
    }
    let (code, message) = code_and_message.split_once(": ").unwrap_or(("", code_and_message));
    Some(ToolDiagnostic {
        source: "dotnet".to_string(),
        code: code.to_string(),
        severity: severity.to_string(),
        message: message.to_string(),
        location: Some(format!("{path}:{line_no}:{column}")),
    })
}

/// 解析 pytest `-rA`/失败摘要行（保守）：`FAILED path::test` / `ERROR path::test`。
/// location 只在可解析时给出 file（§101 不猜 line）。
pub fn parse_pytest_line(line: &str) -> Option<ToolDiagnostic> {
    let trimmed = line.trim();
    // FAILED 与 ERROR 摘要行均为失败（§100）。
    let severity = "error";
    let rest = trimmed
        .strip_prefix("FAILED ")
        .or_else(|| trimmed.strip_prefix("ERROR "))?;
    let node_id = rest.split_whitespace().next()?;
    let (path, _name) = node_id.split_once("::")?;
    Some(ToolDiagnostic {
        source: "pytest".to_string(),
        code: String::new(),
        severity: severity.to_string(),
        message: format!("{severity}: {node_id}"),
        location: Some(path.to_string()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn junit_parse_extracts_failures_and_successes() {
        let xml = r#"<testsuite name="t">
  <testcase name="ok" classname="mod::a"/>
  <testcase name="bad" classname="mod::b">
    <failure message="assert failed"/>
  </testcase>
</testsuite>"#;
        let outcome = parse_junit(xml);
        assert!(outcome.parser_error.is_none());
        assert_eq!(outcome.diagnostics.len(), 2);
        assert_eq!(outcome.diagnostics[1].severity, "error");
    }

    #[test]
    fn junit_parse_conservative_on_malformed() {
        let outcome = parse_junit("this is not xml at all");
        assert!(outcome.parser_error.is_some());
        assert!(outcome.diagnostics.is_empty(), "§288: no panic, no fake data");
    }

    #[test]
    fn dotnet_error_line_parsed_with_location() {
        let diagnostic = parse_dotnet_line(
            "src/foo.cs(42,13): error CS0103: The name 'x' does not exist",
        )
        .expect("should parse");
        assert_eq!(diagnostic.severity, "error");
        assert_eq!(diagnostic.code, "CS0103");
        assert_eq!(diagnostic.location.as_deref(), Some("src/foo.cs:42:13"));
        assert!(parse_dotnet_line("random text (1,1): nope").is_none());
    }

    #[test]
    fn pytest_lines_parsed() {
        let diagnostic =
            parse_pytest_line("FAILED tests/test_a.py::test_x - assert 1 == 2").unwrap();
        assert_eq!(diagnostic.severity, "error");
        assert_eq!(diagnostic.location.as_deref(), Some("tests/test_a.py"));
    }
}
