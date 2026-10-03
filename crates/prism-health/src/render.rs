//! HealthAnalysis 渲染（规格 M7 §76/§77/§89/§90/§131/§136/§225/§231/§236）。
//!
//! - 只渲染事实与证据，无健康分、无宣传语（§76/§89）
//! - 不确定性显式呈现（§131 Analysis limitations）
//! - 报告默认项目相对路径（§231），无 secret 值（模型层已 REDACTED）
//! - HTML 为简单静态离线页（§90：不是第二套 SPA）

use crate::engine::HealthAnalysis;
use crate::model::{Finding, FindingCategory, FindingStatus};
use std::fmt::Write as _;

fn severity_label(severity: prism_core::severity::Severity) -> &'static str {
    match severity {
        prism_core::severity::Severity::Critical => "CRITICAL",
        prism_core::severity::Severity::Warning => "WARNING",
        prism_core::severity::Severity::Info => "INFO",
    }
}

fn category_label(category: FindingCategory) -> &'static str {
    match category {
        FindingCategory::Architecture => "architecture",
        FindingCategory::Dependency => "dependency",
        FindingCategory::Code => "code",
        FindingCategory::Security => "security",
        FindingCategory::Maintenance => "maintenance",
        FindingCategory::Documentation => "documentation",
        FindingCategory::Configuration => "configuration",
        FindingCategory::Git => "git",
        FindingCategory::Environment => "environment",
    }
}

fn completeness_label(completeness: prism_core::PrismCompleteness) -> &'static str {
    match completeness {
        prism_core::PrismCompleteness::Complete => "complete",
        prism_core::PrismCompleteness::Partial => "partial (some rules failed or inputs unavailable)",
        prism_core::PrismCompleteness::Truncated => "truncated (limits reached)",
        prism_core::PrismCompleteness::Failed => "failed",
    }
}

fn status_suffix(status: FindingStatus) -> &'static str {
    match status {
        FindingStatus::Active => "",
        FindingStatus::Suppressed => " [suppressed]",
        FindingStatus::Ignored => " [ignored]",
    }
}

fn finding_block(out: &mut String, finding: &Finding) {
    let _ = writeln!(
        out,
        "[{}] {} {}{}",
        severity_label(finding.severity),
        finding.rule_id,
        finding.title,
        status_suffix(finding.status)
    );
    let _ = writeln!(
        out,
        "  {}:{}:{}",
        finding.location.relative_path, finding.location.line, finding.location.column
    );
    let _ = writeln!(out, "  {}", finding.message);
    let _ = writeln!(out, "  What: {}", finding.explanation.what);
    let _ = writeln!(out, "  Evidence: {}", finding.explanation.evidence_summary);
    if let Some(why) = &finding.explanation.why_it_matters {
        let _ = writeln!(out,  "  Why it matters: {why}");
    }
    let _ = writeln!(
        out,
        "  Confidence: {}",
        format!("{:?}", finding.confidence).to_ascii_lowercase().trim_matches('"')
    );
    out.push('\n');
}

/// 纯文本（§76：Health Findings 汇总 + 分类计数 + 明细）。
pub fn render_text(analysis: &HealthAnalysis) -> String {
    let mut out = String::new();
    let summary = &analysis.summary;
    out.push_str("Health Findings\n\n");
    let _ = writeln!(out, "Critical  {}", summary.critical_count);
    let _ = writeln!(out, "Warning   {}", summary.warning_count);
    let _ = writeln!(out, "Info      {}", summary.info_count);
    out.push('\n');
    out.push_str("Categories\n");
    for (category, count) in &summary.by_category {
        let _ = writeln!(out, "{category:<14} {count}");
    }
    out.push('\n');
    let _ = writeln!(
        out,
        "Completeness: {}  Scope: {:?}  Change-related: {}",
        completeness_label(analysis.analysis_completeness),
        analysis.scope,
        analysis.change_related_findings
    );
    if !analysis.diagnostics.is_empty() {
        out.push_str("\nAnalysis limitations:\n");
        for diagnostic in &analysis.diagnostics {
            let _ = writeln!(
                out,
                "  [{}] {} {}",
                diagnostic.kind,
                diagnostic.subject.as_deref().unwrap_or("-"),
                diagnostic.detail
            );
        }
    }
    if !analysis.findings.is_empty() {
        out.push('\n');
        for finding in &analysis.findings {
            finding_block(&mut out, finding);
        }
    }
    out
}

/// Markdown 报告（§89：Summary + 分类小节 + Finding 小节）。
pub fn render_markdown(analysis: &HealthAnalysis) -> String {
    let mut out = String::new();
    let summary = &analysis.summary;
    out.push_str("# Prism Health Report\n\n");
    let _ = writeln!(out, "- Project: `{}`", analysis.project_name);
    let _ = writeln!(out, "- Scope: `{:?}`", analysis.scope);
    let _ = writeln!(out, "- Completeness: **{}**", completeness_label(analysis.analysis_completeness));
    let _ = writeln!(out, "- Findings: {}", summary.total_findings);
    let _ = writeln!(
        out,
        "- Critical: {} · Warning: {} · Info: {}",
        summary.critical_count, summary.warning_count, summary.info_count
    );
    let _ = writeln!(
        out,
        "- Change-related: {}\n",
        analysis.change_related_findings
    );

    // 按分类分组小节（§89 结构）。
    let mut by_category: std::collections::BTreeMap<String, Vec<&Finding>> = Default::default();
    for finding in &analysis.findings {
        by_category
            .entry(category_label(finding.category).to_string())
            .or_default()
            .push(finding);
    }
    for (category, findings) in &by_category {
        let _ = writeln!(out, "## {} ({})\n", cap(category), findings.len());
        for finding in findings {
            let _ = writeln!(
                out,
                "### {} — {}\n",
                finding.rule_id,
                finding.title
            );
            let _ = writeln!(
                out,
                "- Location: `{}:{}`",
                finding.location.relative_path, finding.location.line
            );
            let _ = writeln!(
                out,
                "- Severity: `{:?}` · Confidence: `{:?}` · Status: `{:?}`",
                format!("{:?}", finding.severity).to_ascii_lowercase(),
                format!("{:?}", finding.confidence).to_ascii_lowercase(),
                finding.status
            );
            let _ = writeln!(out, "- What: {}", finding.explanation.what);
            let _ = writeln!(out, "- Evidence: {}", finding.explanation.evidence_summary);
            if let Some(why) = &finding.explanation.why_it_matters {
                let _ = writeln!(out, "- Why it matters: {why}");
            }
            out.push('\n');
        }
    }

    // §131：不确定性显式呈现，即使没有诊断也输出声明（§89）。
    out.push_str("## Analysis Limitations\n\n");
    out.push_str(
        "> Health analysis is based on resolvable static relationships; some relationships could not be resolved.\n\n",
    );
    let diagnostics = analysis.riskless_diagnostics();
    if diagnostics.is_empty() {
        out.push_str("_none_\n\n");
    } else {
        for diagnostic in diagnostics {
            let _ = writeln!(
                out,
                "- `{}` {} — {}",
                diagnostic.kind,
                diagnostic.subject.as_deref().unwrap_or("-"),
                diagnostic.detail
            );
        }
        out.push('\n');
    }
    out
}

fn cap(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
        None => String::new(),
    }
}

/// 静态 HTML（§90：简单、离线、可过滤的最小实现；内联 CSS 无依赖）。
pub fn render_html(analysis: &HealthAnalysis) -> String {
    let mut items = String::new();
    for finding in &analysis.findings {
        let _ = writeln!(
            items,
            "<tr class=\"sev-{sev}\"><td>{sev}</td><td>{rule}</td><td>{title}{status}</td>\
             <td><code>{path}:{line}</code></td><td>{confidence}</td></tr>",
            sev = format!("{:?}", finding.severity).to_ascii_lowercase(),
            rule = finding.rule_id,
            title = html_escape(&finding.title),
            status = html_escape(status_suffix(finding.status)),
            path = html_escape(&finding.location.relative_path),
            line = finding.location.line,
            confidence = format!("{:?}", finding.confidence).to_ascii_lowercase(),
        );
    }
    let mut diag = String::new();
    for diagnostic in analysis.riskless_diagnostics() {
        let _ = writeln!(
            diag,
            "<li><code>{}</code> {} — {}</li>",
            diagnostic.kind,
            html_escape(diagnostic.subject.as_deref().unwrap_or("-")),
            html_escape(&diagnostic.detail)
        );
    }
    format!(
        r#"<!DOCTYPE html>
<html lang="en"><head><meta charset="utf-8"><title>Prism Health Report — {project}</title>
<style>
body {{ font-family: system-ui, sans-serif; margin: 2rem; }}
table {{ border-collapse: collapse; width: 100%; }}
td, th {{ border: 1px solid #ddd; padding: .35rem .6rem; text-align: left; }}
tr.sev-critical td:first-child {{ color: #b00020; font-weight: 600; }}
tr.sev-warning td:first-child {{ color: #9a6700; }}
code {{ background: #f5f5f5; }}
</style></head><body>
<h1>Prism Health Report — {project}</h1>
<p>Scope: {scope:?} · Completeness: {completeness} · Findings: {total}
(Critical {critical} / Warning {warning} / Info {info}) · Change-related: {changed}</p>
<h2>Findings</h2>
<table><tr><th>Severity</th><th>Rule</th><th>Title</th><th>Location</th><th>Confidence</th></tr>
{items}</table>
<h2>Analysis Limitations</h2>
<p>Impact and health analyses are based on resolvable static relationships; some
relationships could not be resolved, so results may be incomplete.</p>
<ul>{diag}</ul>
</body></html>"#,
        project = html_escape(&analysis.project_name),
        scope = analysis.scope,
        completeness = completeness_label(analysis.analysis_completeness),
        total = analysis.summary.total_findings,
        critical = analysis.summary.critical_count,
        warning = analysis.summary.warning_count,
        info = analysis.summary.info_count,
        changed = analysis.change_related_findings,
    )
}

fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

impl HealthAnalysis {
    /// 诊断列表（渲染辅助；避免渲染层直接触碰内部字段命名漂移）。
    pub fn riskless_diagnostics(
        &self,
    ) -> &[crate::model::HealthDiagnostic] {
        &self.diagnostics
    }
}
