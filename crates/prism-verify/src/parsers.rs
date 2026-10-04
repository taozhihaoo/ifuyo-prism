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
    let column = parts
        .next()
        .and_then(|c| c.trim().parse::<u32>().ok())
        .unwrap_or(1);
    let remainder = &line[close + 3..];
    let (severity, code_and_message) = remainder.split_once(' ')?;
    let severity = severity.trim_end_matches(':');
    if severity != "error" && severity != "warning" {
        return None;
    }
    let (code, message) = code_and_message
        .split_once(": ")
        .unwrap_or(("", code_and_message));
    Some(ToolDiagnostic {
        source: "dotnet".to_string(),
        code: code.to_string(),
        severity: severity.to_string(),
        message: message.to_string(),
        location: Some(format!("{path}:{line_no}:{column}")),
    })
}

/// 解析 pytest `-rA`/失败摘要行（保守）：`FAILED path::test` / `ERROR path::test`，
/// 保留 ` - <断言消息>` 尾部。location 只在可解析时给出 file（§101 不猜 line）。
pub fn parse_pytest_line(line: &str) -> Option<ToolDiagnostic> {
    let trimmed = line.trim();
    // FAILED 与 ERROR 摘要行均为失败（§100）。
    let severity = "error";
    let rest = trimmed
        .strip_prefix("FAILED ")
        .or_else(|| trimmed.strip_prefix("ERROR "))?;
    let node_id = rest.split_whitespace().next()?;
    let (path, _name) = node_id.split_once("::")?;
    // 摘要行形如 "FAILED tests/x.py::test_y - assert 5 == 6"；
    // "- " 之后的断言消息保留（§281 pytest 规范化）。
    let tail = rest
        .split_once(" - ")
        .map(|(_, message)| message.trim())
        .unwrap_or("");
    // severity 单独携带（error）；message 以 node id 开头，便于展示去重。
    let message = if tail.is_empty() {
        node_id.to_string()
    } else {
        format!("{node_id} - {tail}")
    };
    Some(ToolDiagnostic {
        source: "pytest".to_string(),
        code: String::new(),
        severity: severity.to_string(),
        message,
        location: Some(path.to_string()),
    })
}

/// 解析 SARIF 2.x JSON（§49；保守，§288-§291）：遍历 `runs[].results[]`，
/// 只提取 ruleId / level / message.text / 物理位置。字段缺失的 result 跳过
/// 相应部分而非猜测（§101）；JSON 无效或缺 `runs` ⇒ `parser_error`、
/// 零诊断——调用方据此回退，绝不改变执行结果。
pub fn parse_sarif(text: &str) -> ParseOutcome {
    let mut outcome = ParseOutcome::default();
    let Ok(value) = serde_json::from_str::<serde_json::Value>(text) else {
        return ParseOutcome::failed("invalid JSON");
    };
    let Some(runs) = value.get("runs").and_then(|runs| runs.as_array()) else {
        return ParseOutcome::failed("no runs array");
    };
    for run in runs {
        // tool.driver.name 仅在真实存在时作为 source 标注（缺失 ⇒ "sarif"）。
        let tool = run
            .get("tool")
            .and_then(|tool| tool.get("driver"))
            .and_then(|driver| driver.get("name"))
            .and_then(|name| name.as_str())
            .unwrap_or("sarif");
        let Some(results) = run.get("results").and_then(|results| results.as_array()) else {
            continue;
        };
        for result in results {
            let rule = result
                .get("ruleId")
                .and_then(|rule| rule.as_str())
                .unwrap_or_default();
            let Some(message) = result
                .get("message")
                .and_then(|message| message.get("text"))
                .and_then(|text| text.as_str())
            else {
                continue; // 无消息文本 ⇒ 无法构成有意义的诊断，跳过（不猜）。
            };
            // level：error ⇒ error；其余（warning/note/none/缺失）保守 ⇒ warning。
            let severity = if result.get("level").and_then(|l| l.as_str()) == Some("error") {
                "error"
            } else {
                "warning"
            };
            let location = result
                .get("locations")
                .and_then(|locations| locations.as_array())
                .and_then(|locations| locations.first())
                .and_then(|location| location.get("physicalLocation"))
                .and_then(sarif_physical_location);
            outcome.diagnostics.push(ToolDiagnostic {
                source: tool.to_string(),
                code: rule.to_string(),
                severity: severity.to_string(),
                message: message.to_string(),
                location,
            });
        }
    }
    outcome
}

/// physicalLocation → "uri:line:column"（行/列存在且 ≥1 才追加；§101 不猜）。
fn sarif_physical_location(physical: &serde_json::Value) -> Option<String> {
    let uri = physical
        .get("artifactLocation")
        .and_then(|artifact| artifact.get("uri"))
        .and_then(|uri| uri.as_str())?;
    // file:// 前缀剥除（本地文件场景）；其余 URI 形式按原样保留。
    let uri = uri.strip_prefix("file://").unwrap_or(uri);
    let region = physical.get("region");
    let line = region
        .and_then(|region| region.get("startLine"))
        .and_then(|line| line.as_u64())
        .filter(|line| *line >= 1);
    let column = region
        .and_then(|region| region.get("startColumn"))
        .and_then(|column| column.as_u64())
        .filter(|column| *column >= 1);
    Some(match (line, column) {
        (Some(line), Some(column)) => format!("{uri}:{line}:{column}"),
        (Some(line), None) => format!("{uri}:{line}"),
        _ => uri.to_string(),
    })
}

/// 解析诊断条数上限（有界采集，§201 思路）。
pub const MAX_PARSED_DIAGNOSTICS: usize = 50;

/// §278/§279 失败输出 → 结构化诊断（保守，§288-§291）：
/// 只解析**已经打印**的 stdout/stderr；不认识的行直接跳过；
/// 解析失败/零匹配 ⇒ 空表——绝不改变执行状态或退出码。
/// 优先嗅探 SARIF JSON（任意工具都可能打印 SARIF：stdout 以 `{` 开头且含
/// `"runs"` 才尝试；解析失败 ⇒ 回退逐行解析器）；否则按 program 选择
/// 逐行解析器（python 家族 ⇒ pytest 摘要行，dotnet ⇒ MSBuild/VSTest 诊断行）。
pub fn parse_failure_output(program: &str, stdout: &str, stderr: &str) -> Vec<ToolDiagnostic> {
    let mut diagnostics = Vec::new();
    let trimmed = stdout.trim_start();
    if trimmed.starts_with('{') && stdout.contains("\"runs\"") {
        let sarif = parse_sarif(stdout);
        if sarif.parser_error.is_none() {
            diagnostics.extend(sarif.diagnostics);
        }
    }
    match program {
        "dotnet" => {
            for line in stdout.lines().chain(stderr.lines()) {
                if let Some(diagnostic) = parse_dotnet_line(line) {
                    diagnostics.push(diagnostic);
                }
            }
        }
        "python" | "python3" | "pytest" | "py" => {
            for line in stdout.lines() {
                if let Some(diagnostic) = parse_pytest_line(line) {
                    diagnostics.push(diagnostic);
                }
            }
        }
        _ => {}
    }
    diagnostics.truncate(MAX_PARSED_DIAGNOSTICS);
    diagnostics
}

/// ToolDiagnostic → CheckRun diagnostic detail（用户可见，§282）。
/// location 已包含在 message 开头时不重复（pytest 摘要行自带 node id）。
pub fn format_tool_diagnostic(diagnostic: &ToolDiagnostic) -> String {
    let code = if diagnostic.code.is_empty() {
        String::new()
    } else {
        format!("{}: ", diagnostic.code)
    };
    let prefix = match &diagnostic.location {
        Some(location)
            if !location.is_empty() && !diagnostic.message.starts_with(location.as_str()) =>
        {
            format!("{location}: ")
        }
        _ => String::new(),
    };
    format!(
        "[{}] {}{}{}",
        diagnostic.source, prefix, code, diagnostic.message
    )
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
        assert!(
            outcome.diagnostics.is_empty(),
            "§288: no panic, no fake data"
        );
    }

    #[test]
    fn dotnet_error_line_parsed_with_location() {
        let diagnostic =
            parse_dotnet_line("src/foo.cs(42,13): error CS0103: The name 'x' does not exist")
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

    // ---------- SARIF（§49，保守） ----------

    const SARIF_SAMPLE: &str = r#"{
  "version": "2.1.0",
  "runs": [
    {
      "tool": { "driver": { "name": "roslyn" } },
      "results": [
        {
          "ruleId": "CS0103",
          "level": "error",
          "message": { "text": "The name 'x' does not exist" },
          "locations": [
            { "physicalLocation": {
                "artifactLocation": { "uri": "src/foo.cs" },
                "region": { "startLine": 42, "startColumn": 13 } } }
          ]
        },
        {
          "ruleId": "STYLE001",
          "level": "warning",
          "message": { "text": "unused variable" },
          "locations": [
            { "physicalLocation": {
                "artifactLocation": { "uri": "file:///abs/bar.cs" },
                "region": { "startLine": 0 } } }
          ]
        },
        {
          "ruleId": "NOTE002",
          "message": { "text": "no level and no location" }
        }
      ]
    }
  ]
}"#;

    #[test]
    fn sarif_parse_extracts_results_conservatively() {
        let outcome = parse_sarif(SARIF_SAMPLE);
        assert!(outcome.parser_error.is_none());
        assert_eq!(outcome.diagnostics.len(), 3);
        let error = &outcome.diagnostics[0];
        assert_eq!(error.source, "roslyn");
        assert_eq!(error.code, "CS0103");
        assert_eq!(error.severity, "error");
        assert_eq!(error.location.as_deref(), Some("src/foo.cs:42:13"));
        // file:// 剥除；region 行 <1 视为缺失（§101 不猜）⇒ 仅 uri。
        let warning = &outcome.diagnostics[1];
        assert_eq!(warning.severity, "warning");
        assert_eq!(warning.location.as_deref(), Some("/abs/bar.cs"));
        // level 缺失 ⇒ 保守 warning；无 locations ⇒ location None。
        let note = &outcome.diagnostics[2];
        assert_eq!(note.severity, "warning");
        assert!(note.location.is_none());
    }

    #[test]
    fn sarif_parse_conservative_on_invalid_input() {
        for input in ["not json", "{\"other\": 1}", "[1, 2]"] {
            let outcome = parse_sarif(input);
            assert!(outcome.parser_error.is_some(), "{input}");
            assert!(outcome.diagnostics.is_empty(), "§288: no fake data");
        }
    }

    #[test]
    fn sarif_sniff_only_when_output_looks_like_sarif() {
        // SARIF JSON 打印在 stdout ⇒ 提取诊断，与 program 无关。
        let diagnostics = parse_failure_output("some-sarif-tool", SARIF_SAMPLE, "");
        assert_eq!(diagnostics.len(), 3);
        assert_eq!(diagnostics[0].code, "CS0103");

        // 看起来像 JSON 但解析失败 ⇒ 回退（pytest 行仍可解析）。
        let fallback = parse_failure_output(
            "python",
            "{\"runs\": broken\nFAILED tests/test_a.py::test_x",
            "",
        );
        assert_eq!(fallback.len(), 1);
        assert_eq!(fallback[0].source, "pytest");

        // 普通输出不触发嗅探（无 '{' 开头）。
        let none = parse_failure_output("some-tool", "plain failure text", "");
        assert!(none.is_empty());
    }
}
