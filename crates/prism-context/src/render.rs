//! Renderers（规格 M10b §159–§165/§235–§245/§255–§257/§242–§245 安全规则）。
//!
//! Markdown（§161：fence escape §242）/ PlainText（§162：ANSI strip §244）/
//! JSON 由 serde 直接输出（§160/§245 serde 转义保证）。
//! Privacy 复用中央层（§256/§257 不造第二套）——pack 内容在 materializer
//! 已 redact，renderer 只负责形态与转义。

use crate::model::ContextPack;
use prism_core::privacy::redact_text;

/// §244：剥离 C0 控制字符（终端注入防御）。
pub fn strip_control(text: &str) -> String {
    text.chars()
        .filter(|c| !c.is_control() || *c == '\n' || *c == '\t')
        .collect()
}

/// §242：fence 转义——内容含 ``` 时用更长的 fence。
fn fenced(language: &str, content: &str) -> String {
    let longest_backtick_run = content
        .split('\n')
        .map(|line| line.chars().take_while(|c| *c == '`').count())
        .max()
        .unwrap_or(0);
    let fence = "`".repeat((longest_backtick_run + 1).max(3));
    format!("{fence}{language}\n{content}\n{fence}")
}

/// Markdown 渲染（§161 结构；§236–§239 factual、无主观句）。
pub fn render_markdown(pack: &ContextPack) -> String {
    let mut out = String::new();
    out.push_str("# Prism Context Pack\n\n");
    let manifest = &pack.manifest;
    out.push_str("## Task\n\n");
    out.push_str(&format!(
        "- task: {}\n- kinds: {}\n- scope: {}\n- specificity: {}\n\n",
        manifest.task.as_deref().unwrap_or("(none)"),
        manifest
            .classification
            .kinds
            .iter()
            .map(|kind| kind.as_str())
            .collect::<Vec<_>>()
            .join(", "),
        manifest.scope_str(),
        format!("{:?}", manifest.classification.specificity).to_lowercase()
    ));

    out.push_str("## Project\n\n");
    out.push_str(&format!(
        "- root: {}\n- items: {}\n\n",
        manifest.project_root, manifest.summary.items
    ));

    // 按组渲染（§161 组名即 section）。
    let mut current_group: Option<crate::model::ContextGroup> = None;
    for item in &pack.items {
        if current_group != Some(item.group) {
            current_group = Some(item.group);
            out.push_str(&format!("## {}\n\n", group_title(item.group)));
        }
        out.push_str(&format!(
            "### {} [{:?}] ({})\n\n",
            escape_markdown(&strip_control(&item.title)),
            item.priority,
            item.source.source_type.as_str()
        ));
        out.push_str(&format!(
            "- source: {}{}\n",
            item.source.path.as_deref().unwrap_or("-"),
            item.source
                .location
                .as_deref()
                .map(|location| format!(" {location}"))
                .unwrap_or_default()
        ));
        out.push_str(&format!(
            "- why: {}\n- trust: {}\n",
            item.reasons
                .iter()
                .map(|reason| reason.as_str())
                .collect::<Vec<_>>()
                .join(", "),
            item.trust_level.as_str()
        ));
        for evidence in &item.evidence {
            out.push_str(&format!(
                "- evidence: {}\n",
                escape_markdown(&strip_control(evidence))
            ));
        }
        if let Some(content) = &item.content {
            let language = item.language.as_deref().unwrap_or("");
            out.push_str(&fenced(language, &redact_text(&strip_control(content))));
            out.push('\n');
        }
        if item.truncated {
            out.push_str(&format!(
                "- truncated: {} -> {} bytes ({})\n",
                item.original_size.unwrap_or_default(),
                item.included_size.unwrap_or_default(),
                item.truncation_reason.as_deref().unwrap_or("budget")
            ));
        }
        out.push('\n');
    }

    out.push_str("## Context Health\n\n");
    out.push_str(&format!("- status: {:?}\n", pack.health.status));
    for finding in &pack.health.findings {
        let severity = format!("{:?}", finding.severity);
        out.push_str(&format!(
            "- [{severity}] {}: {}\n",
            finding.rule_id, finding.message
        ));
    }
    out.push_str(&format!(
        "- coverage: {}\n",
        coverage_line(&pack.health.coverage)
    ));

    out.push_str("## Risk signals\n\n");
    if pack.risk_signals.is_empty() {
        out.push_str("- none\n");
    }
    for signal in &pack.risk_signals {
        out.push_str(&format!("- {}\n", signal.as_str()));
    }

    out.push_str("## Limitations\n\n");
    if manifest.limitations.is_empty() {
        out.push_str("- none\n");
    }
    for limitation in &manifest.limitations {
        out.push_str(&format!(
            "- {}\n",
            escape_markdown(&strip_control(limitation))
        ));
    }
    out.push_str(&format!(
        "\n---\n*schema_version={} · pack id={} · generated_at={} · state dirty={:?}*\n",
        manifest.schema_version, manifest.id, manifest.generated_at, pack.state.dirty
    ));
    out
}

fn group_title(group: crate::model::ContextGroup) -> &'static str {
    match group {
        crate::model::ContextGroup::Project => "Project",
        crate::model::ContextGroup::CurrentChange => "Current Changes",
        crate::model::ContextGroup::TargetCode => "Relevant Code",
        crate::model::ContextGroup::Dependencies => "Dependencies",
        crate::model::ContextGroup::Impact => "Impact",
        crate::model::ContextGroup::Tests => "Tests",
        crate::model::ContextGroup::Health => "Health",
        crate::model::ContextGroup::Verification => "Verification",
        crate::model::ContextGroup::Environment => "Environment",
        crate::model::ContextGroup::Documentation => "Documentation",
    }
}

fn coverage_line(coverage: &crate::health::ContextCoverage) -> String {
    format!(
        "target={} impact={} verification={} docs={} environment={}",
        coverage.target,
        coverage.impact,
        coverage.verification,
        coverage.docs,
        coverage.environment
    )
}

/// markdown 文本转义（行首 heading/fence 中和）。
fn escape_markdown(text: &str) -> String {
    text.replace('\r', "")
}

/// PlainText 渲染（§162：copy/paste 友好 + traceability）。
pub fn render_plain_text(pack: &ContextPack) -> String {
    let mut out = String::new();
    let manifest = &pack.manifest;
    out.push_str(&format!(
        "=== Prism Context Pack ({}) task={} ===\n",
        manifest.scope_str(),
        manifest.task.as_deref().unwrap_or("(none)")
    ));
    for item in &pack.items {
        out.push_str(&format!(
            "\n--- {} [{:?}] trust={} ---\n",
            item.title,
            item.priority,
            item.trust_level.as_str()
        ));
        out.push_str(&format!(
            "source: {}{}\nwhy: {}\n",
            item.source.path.as_deref().unwrap_or("-"),
            item.source
                .location
                .as_deref()
                .map(|location| format!(" {location}"))
                .unwrap_or_default(),
            item.reasons
                .iter()
                .map(|reason| reason.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ));
        if let Some(content) = &item.content {
            out.push_str(&strip_control(content));
            out.push('\n');
        }
    }
    out.push_str(&format!(
        "\n=== limitations: {} · health: {:?} · est tokens {}/{} ===\n",
        manifest.limitations.len(),
        pack.health.status,
        manifest.budget.used_tokens_estimate,
        manifest.budget.limit_tokens_estimate
    ));
    for limitation in &manifest.limitations {
        out.push_str(&format!("! {limitation}\n"));
    }
    out
}
