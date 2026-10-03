//! 环境报告渲染（规格 M9b §234–§236/§249–§250/§322/§341–§342/§344–§345/§347/§396）。
//!
//! §347：原始 snapshot 可比报告完整，但任何 `to_json/to_markdown/to_html`
//! 都必须过隐私/脱敏层（复用 prism-core privacy，§346）：
//! 无 env 值（结构性保证）、无 secret 值（redact_text 兜底）、
//! 私有家目录路径缩短（§250）、HTML 转义（§342）、终端控制字符剥离。

use crate::model::{EnvironmentSnapshot, MatchResult};
use prism_core::privacy::redact_text;

/// §250：把家目录前缀替换为 `<HOME>`（Windows 大小写不敏感）。
pub fn shorten_home_paths(text: &str) -> String {
    let Some(home) = std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(std::path::PathBuf::from)
    else {
        return text.to_string();
    };
    let home_text = home.to_string_lossy().to_string();
    if home_text.is_empty() {
        return text.to_string();
    }
    let mut out = text.replace(&home_text, "<HOME>");
    if cfg!(windows) {
        // 大小写不敏感的第二遍（§192 身份规则）：逐行小写匹配替换。
        let lower_home = home_text.to_ascii_lowercase();
        let mut rebuilt = String::with_capacity(out.len());
        for line in out.split_inclusive('\n') {
            if line.to_ascii_lowercase().contains(&lower_home) {
                // 找到首个大小写不敏感命中并替换全部。
                let mut rest = line;
                let mut line_out = String::new();
                while let Some(position) = rest.to_ascii_lowercase().find(&lower_home) {
                    line_out.push_str(&rest[..position]);
                    line_out.push_str("<HOME>");
                    rest = &rest[position + home_text.len()..];
                }
                line_out.push_str(rest);
                rebuilt.push_str(&line_out);
            } else {
                rebuilt.push_str(line);
            }
        }
        out = rebuilt;
    }
    out
}

/// HTML 转义（§342 防注入）。
pub fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// 行级隐私处理：redact + 家目录缩短。
fn privacy(text: &str) -> String {
    shorten_home_paths(&redact_text(text))
}

/// Markdown 报告（§235 结构；§322 Detected/Required/Matched/Finding 分开；
/// §256 计数式结论；§296 表格对齐）。
pub fn render_markdown(snapshot: &EnvironmentSnapshot) -> String {
    let mut out = String::new();
    out.push_str("# Environment Report\n\n");
    out.push_str(&format!(
        "- Project: {}\n- Snapshot: {} (captured {})\n- Scan: {:?} · completeness {:?}\n",
        privacy(&snapshot.project_root),
        snapshot.id,
        snapshot.captured_at,
        snapshot.scan_outcome,
        snapshot.completeness
    ));
    out.push_str(
        "- This report describes the current local environment; it does not guarantee \
         reproducibility.\n\n",
    );

    // ## Host（§279 简洁）
    out.push_str("## Host\n\n");
    out.push_str(&format!(
        "- OS: {} {}\n- Architecture: {}\n- Shell: {}\n",
        snapshot.host.os_name,
        snapshot
            .host
            .os_version
            .as_deref()
            .unwrap_or("version unknown"),
        snapshot.host.architecture.as_str(),
        snapshot.shell.kind.as_str()
    ));
    out.push_str(&format!(
        "- Resources: {} cpus · memory {} · disk free {}\n\n",
        snapshot
            .resources
            .logical_cpus
            .map(|value| value.to_string())
            .unwrap_or_else(|| "unknown".to_string()),
        format_bytes(snapshot.resources.memory_total_bytes),
        format_bytes(snapshot.resources.disk_free_bytes)
    ));

    // ## Toolchain（§222）
    out.push_str("## Toolchain\n\n");
    for tool in &snapshot.tools {
        out.push_str(&format!(
            "- {}: {} ({})\n",
            privacy(&tool.name),
            privacy(tool.raw_version.as_deref().unwrap_or("not found")),
            privacy(tool.path.as_deref().unwrap_or("path unknown"))
        ));
    }
    if !snapshot.virtualenvs.is_empty() {
        for venv in &snapshot.virtualenvs {
            out.push_str(&format!(
                "- venv {}: {} ({}){}\n",
                venv.kind,
                privacy(&venv.path),
                venv.raw_version.as_deref().unwrap_or("version unknown"),
                if venv.active { " [active]" } else { "" }
            ));
        }
    }
    out.push('\n');

    // ## Requirements（§223/§256 计数式）
    out.push_str("## Requirements\n\n");
    if snapshot.matches.is_empty() {
        out.push_str("_No project requirements discovered._\n\n");
    } else {
        out.push_str("| Requirement | Source | Result |\n|---|---|---|\n");
        let mut satisfied = 0usize;
        let mut unsatisfied = 0usize;
        let mut unknown = 0usize;
        for (requirement, matched) in snapshot
            .requirements
            .iter()
            .zip(snapshot.matches.iter())
        {
            let expected = if requirement.expected.is_empty() {
                requirement.operator.as_str().to_string()
            } else {
                format!("{} {}", requirement.operator.as_str(), requirement.expected)
            };
            match matched.result {
                MatchResult::Satisfied => satisfied += 1,
                MatchResult::Unsatisfied => unsatisfied += 1,
                MatchResult::Unknown => unknown += 1,
                MatchResult::NotApplicable => {}
            }
            out.push_str(&format!(
                "| {} {} | {} | {} |\n",
                privacy(&requirement.subject),
                expected,
                privacy(&requirement.source),
                matched.result.as_str()
            ));
        }
        out.push('\n');
        out.push_str(&format!(
            "Requirements: {} satisfied · {} unsatisfied · {} unknown · {} conflicts\n\n",
            satisfied,
            unsatisfied,
            unknown,
            snapshot.conflicts.len()
        ));
    }

    // ## Findings（§224/§256：事实 + evidence）
    out.push_str("## Findings\n\n");
    if snapshot.findings.is_empty() {
        out.push_str("_No environment findings._\n\n");
    } else {
        for (index, finding) in snapshot.findings.iter().enumerate() {
            out.push_str(&format!(
                "{}. [{}] {} {} — {}\n",
                index + 1,
                format_args!("{:?}", finding.severity),
                finding.kind.as_str(),
                privacy(&finding.subject),
                privacy(&finding.message)
            ));
            for evidence in &finding.evidence {
                out.push_str(&format!("   - {}\n", privacy(evidence)));
            }
            if let Some(suggestion) = &finding.suggestion {
                out.push_str(&format!("   - suggestion: {suggestion} (not executed)\n"));
            }
        }
        out.push('\n');
    }

    // ## Ports / Processes（§225/§226 动态事实）
    if !snapshot.ports.is_empty() || !snapshot.expected_ports.is_empty() {
        out.push_str("## Ports\n\n");
        for expected in &snapshot.expected_ports {
            out.push_str(&format!(
                "- expected {} (from {})\n",
                expected.port, expected.source
            ));
        }
        for port in &snapshot.ports {
            out.push_str(&format!(
                "- {} LISTENING {} (pid {:?}, process {:?}, observed {})\n",
                port.port,
                privacy(&port.address),
                port.pid,
                port.process_name.as_deref().map(privacy),
                port.observed_at
            ));
        }
        out.push('\n');
    }
    if !snapshot.processes.is_empty() {
        out.push_str("## Processes\n\n");
        for process in &snapshot.processes {
            out.push_str(&format!(
                "- {} pid {} ({} ports) [{}] observed {}\n",
                privacy(&process.name),
                process.pid,
                process.ports.len(),
                process.classification.as_str(),
                process.observed_at
            ));
        }
        out.push('\n');
    }

    // ## Variables / PATH（§229/§230 摘要级）
    out.push_str(&format!(
        "## Variables\n\n{} captured ({} possibly sensitive; values never read).\n\n",
        snapshot.variables.len(),
        snapshot
            .variables
            .iter()
            .filter(|fact| fact.possibly_sensitive)
            .count()
    ));
    if !snapshot.env_files.is_empty() {
        out.push_str("## Environment files\n\n");
        for file in &snapshot.env_files {
            let sensitive = file
                .keys
                .iter()
                .filter(|key| key.possibly_sensitive)
                .count();
            out.push_str(&format!(
                "- {} [{}] {} keys ({} sensitive names){}\n",
                privacy(&file.path),
                file.kind.as_str(),
                file.keys.len(),
                sensitive,
                match file.tracked {
                    Some(true) => " · tracked",
                    Some(false) => " · untracked",
                    None => "",
                }
            ));
        }
        out.push('\n');
    }
    out.push_str(&format!(
        "## PATH\n\n{} entries ({} duplicates, {} missing{}).\n",
        snapshot.paths.entries.len(),
        snapshot.paths.duplicates.len(),
        snapshot.paths.missing.len(),
        if snapshot.paths.truncated { ", truncated" } else { "" }
    ));
    out
}

/// HTML 报告（§236：静态表格，复用 M7/M8 的朴素形态，不造新框架；
/// §342 全部动态文本经 escape_html）。
pub fn render_html(snapshot: &EnvironmentSnapshot) -> String {
    let mut rows = String::new();
    for tool in &snapshot.tools {
        writeln!(
            rows,
            "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
            escape_html(&tool.name),
            escape_html(&privacy(
                tool.raw_version.as_deref().unwrap_or("not found"),
            )),
            escape_html(&format!("{:?}", tool.probe)),
            escape_html(tool.path.as_deref().unwrap_or("-")),
        )
        .ok();
    }
    let mut requirement_rows = String::new();
    for (requirement, matched) in snapshot.requirements.iter().zip(snapshot.matches.iter()) {
        writeln!(
            requirement_rows,
            "<tr><td>{}</td><td>{}</td><td>{}</td></tr>",
            escape_html(&requirement.subject),
            escape_html(&requirement.expected),
            escape_html(matched.result.as_str()),
        )
        .ok();
    }
    let mut finding_rows = String::new();
    for finding in &snapshot.findings {
        writeln!(
            finding_rows,
            "<tr class=\"{severity}\"><td>{severity:?}</td><td>{kind}</td><td>{subject}</td><td>{message}</td></tr>",
            severity = escape_html(&format!("{:?}", finding.severity)),
            kind = escape_html(finding.kind.as_str()),
            subject = escape_html(&finding.subject),
            message = escape_html(&privacy(&finding.message)),
        )
        .ok();
    }
    format!(
        "<html><head><meta charset=\"utf-8\"><title>Environment Report</title>\
         <style>body{{font-family:sans-serif;margin:2rem}}table{{border-collapse:collapse}}\
         td,th{{border:1px solid #ccc;padding:4px 8px;text-align:left}}</style></head><body>\
         <h1>Environment Report</h1>\
         <p>{} · captured {} · status {:?}</p>\
         <h2>Toolchain</h2><table><tr><th>Tool</th><th>Version</th><th>Probe</th><th>Path</th></tr>{}\
         </table><h2>Requirements</h2><table><tr><th>Subject</th><th>Expected</th><th>Result</th></tr>{}\
         </table><h2>Findings</h2><table><tr><th>Severity</th><th>Kind</th><th>Subject</th><th>Message</th></tr>{}\
         </table><p>This report describes the current local environment; it does not \
         guarantee reproducibility.</p></body></html>\n",
        escape_html(&privacy(&snapshot.project_root)),
        escape_html(&snapshot.captured_at),
        snapshot.status,
        rows,
        requirement_rows,
        finding_rows
    )
}

fn format_bytes(bytes: Option<u64>) -> String {
    match bytes {
        Some(value) if value >= 1024 * 1024 * 1024 => {
            format!("{:.1} GiB", value as f64 / (1024.0 * 1024.0 * 1024.0))
        }
        Some(value) if value >= 1024 * 1024 => {
            format!("{:.1} MiB", value as f64 / (1024.0 * 1024.0))
        }
        Some(value) => format!("{value} B"),
        None => "unknown".to_string(),
    }
}

use std::fmt::Write;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        Architecture, EnvironmentFinding, EnvironmentFindingKind, EnvironmentStatus, HostInfo,
        OsFamily, PathFacts, ShellInfo, ShellKind, ToolFact, ToolSource, ProbeOutcome,
        ENVIRONMENT_SCHEMA_VERSION,
    };

    fn snapshot() -> EnvironmentSnapshot {
        EnvironmentSnapshot {
            schema_version: ENVIRONMENT_SCHEMA_VERSION,
            id: "snap-1".to_string(),
            project_id: String::new(),
            project_root: "C:/tmp/demo".to_string(),
            captured_at: "2026-10-03T00:00:00Z".to_string(),
            host: HostInfo {
                os: OsFamily::Windows,
                os_name: "Windows".to_string(),
                os_version: Some("10.0.26200".to_string()),
                architecture: Architecture::X86_64,
                hostname: None,
                username: None,
                locale: None,
                timezone: None,
            },
            shell: ShellInfo { kind: ShellKind::Cmd, executable: None, version: None },
            paths: PathFacts {
                scope: crate::model::FactScope::Process,
                entries: vec![],
                duplicates: vec![],
                missing: vec![],
                truncated: false,
            },
            tools: vec![],
            virtualenvs: vec![],
            runtimes: vec![],
            sdks: vec![],
            package_managers: vec![],
            variables: vec![],
            env_files: vec![],
            ports: vec![],
            processes: vec![],
            expected_ports: vec![],
            resources: Default::default(),
            requirements: vec![],
            conflicts: vec![],
            matches: vec![],
            findings: vec![],
            status: EnvironmentStatus::Satisfied,
            diagnostics: vec![],
            scan_outcome: crate::model::ScanOutcome::Completed,
            completeness: crate::model::EnvironmentCompleteness::Complete,
        }
    }

    // §341：报告不得泄漏 secret fixture。
    #[test]
    fn reports_redact_secret_fixture() {
        let mut snapshot = snapshot();
        snapshot.tools.push(ToolFact {
            name: "weird".to_string(),
            path: None,
            additional_paths: vec![],
            version: None,
            raw_version: Some("API_TOKEN=super-secret-value-123".to_string()),
            source: ToolSource::Path,
            probe: ProbeOutcome::Succeeded,
        });
        let markdown = render_markdown(&snapshot);
        let html = render_html(&snapshot);
        for rendered in [markdown, html] {
            assert!(
                !rendered.contains("super-secret-value-123"),
                "secret leaked: {rendered}"
            );
            assert!(rendered.contains("[REDACTED]"), "must be redacted");
        }
    }

    // §342：HTML 注入转义。
    #[test]
    fn html_escapes_injection() {
        let mut snapshot = snapshot();
        snapshot.tools.push(ToolFact {
            name: "<script>alert(1)</script>".to_string(),
            path: None,
            additional_paths: vec![],
            version: None,
            raw_version: None,
            source: ToolSource::Path,
            probe: ProbeOutcome::NotFound,
        });
        let html = render_html(&snapshot);
        assert!(!html.contains("<script>alert"));
        assert!(html.contains("&lt;script&gt;"));
    }

    // §250：家目录缩短。
    #[test]
    fn home_paths_shortened() {
        let home = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .unwrap_or_default();
        if home.is_empty() {
            return;
        }
        let text = format!("{}\\Projects\\Foo", home);
        assert_eq!(shorten_home_paths(&text), "<HOME>\\Projects\\Foo");
    }

    // §396：同环境两次报告稳定。
    #[test]
    fn reports_are_deterministic() {
        let snapshot = snapshot();
        assert_eq!(render_markdown(&snapshot), render_markdown(&snapshot));
        assert_eq!(render_html(&snapshot), render_html(&snapshot));
    }

    // §322：Finding 出现在 Markdown 且带 evidence。
    #[test]
    fn findings_rendered_with_evidence() {
        let mut snapshot = snapshot();
        snapshot.findings.push(EnvironmentFinding {
            id: "envf:version_mismatch:node".to_string(),
            kind: EnvironmentFindingKind::VersionMismatch,
            category: crate::model::FindingCategory::Toolchain,
            severity: prism_core::Severity::Warning,
            confidence: crate::model::FindingConfidence::Exact,
            subject: "node".to_string(),
            message: "Toolchain requirement unsatisfied".to_string(),
            evidence: vec!["required by package.json engines".to_string()],
            suggestion: Some("Align the environment (suggestion only).".to_string()),
        });
        let markdown = render_markdown(&snapshot);
        assert!(markdown.contains("## Findings"));
        assert!(markdown.contains("version_mismatch"));
        assert!(markdown.contains("required by package.json engines"));
        assert!(markdown.contains("suggestion only"));
    }
}
