//! Snapshot 历史/比较/时间线渲染（规格 M11b §53/§59/§60/§61/§97/§124/§125）。
//!
//! Markdown 面向人 + AI（§97：human-readable、evidence-oriented、
//! deterministic）；JSON 走 serde。复用中央 redact（§261 export 前再脱敏）。

use crate::compare::SnapshotDiff;
use crate::timeline::Timeline;
use crate::model::ProjectSnapshot;
use prism_core::privacy::redact_text;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// §53/§304：EvolutionSummary——两快照间 counts 事实（无分数 §54）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvolutionSummary {
    pub entries: Vec<EvolutionEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvolutionEntry {
    pub label: String,
    pub from: u64,
    pub to: u64,
}

/// 从两快照 counts 推导（§238：直接聚合，§239 无语义推断）。
pub fn evolution_summary(a: &ProjectSnapshot, b: &ProjectSnapshot) -> EvolutionSummary {
    let mut entries = Vec::new();
    for label in ["files", "findings", "checks"] {
        let from = a.counts.get(label).copied().unwrap_or(0);
        let to = b.counts.get(label).copied().unwrap_or(0);
        if from != to {
            entries.push(EvolutionEntry {
                label: label.to_string(),
                from,
                to,
            });
        }
    }
    EvolutionSummary { entries }
}

/// Compare Markdown（§59/§97）。
pub fn render_compare_markdown(
    name_a: &str,
    name_b: &str,
    diff: &SnapshotDiff,
    evolution: &EvolutionSummary,
) -> String {
    let mut out = String::new();
    out.push_str("# Snapshot Comparison\n\n");
    out.push_str(&format!("- A: {name_a}\n- B: {name_b}\n"));
    out.push_str(&format!("- compatibility: {}\n\n", diff.compatibility));

    for (title, domain) in [
        ("Structure", &diff.structure),
        ("Code", &diff.code),
        ("Architecture", &diff.architecture),
        ("Git", &diff.git),
        ("Change", &diff.change),
        ("Impact", &diff.impact),
        ("Health", &diff.health),
        ("Verification", &diff.verification),
        ("Environment", &diff.environment),
        ("Context", &diff.context_health),
    ] {
        out.push_str(&format!("## {title}\n\n"));
        if domain.state == "unchanged" {
            out.push_str("- unchanged\n\n");
            continue;
        }
        if domain.facts.is_empty() {
            out.push_str(&format!("- {}\n\n", domain.state));
            continue;
        }
        for fact in &domain.facts {
            out.push_str(&format!("- {}\n", redact_text(&strip_marks(fact))));
        }
        out.push('\n');
    }

    if !diff.finding_lifecycles.is_empty() {
        out.push_str("## Finding lifecycle\n\n");
        for entry in &diff.finding_lifecycles {
            out.push_str(&format!(
                "- {} {}: {}\n",
                entry.rule_id,
                entry.lifecycle.as_str(),
                entry.finding_fingerprint
            ));
        }
        out.push('\n');
    }
    if !diff.verification_transitions.is_empty() {
        out.push_str("## Verification transitions\n\n");
        for transition in &diff.verification_transitions {
            out.push_str(&format!(
                "- {}: {} -> {}\n",
                transition.check, transition.from, transition.to
            ));
        }
        out.push('\n');
    }

    if !evolution.entries.is_empty() {
        out.push_str("## Evolution\n\n");
        for entry in &evolution.entries {
            out.push_str(&format!(
                "- {}: {} -> {}\n",
                entry.label, entry.from, entry.to
            ));
        }
        out.push('\n');
    }

    if !diff.limitations.is_empty() {
        out.push_str("## Limitations\n\n");
        for limitation in &diff.limitations {
            out.push_str(&format!("- {limitation}\n"));
        }
    }
    out
}

fn strip_marks(text: &str) -> String {
    text.trim_matches('"').to_string()
}

/// Timeline Markdown（§129 简单形态 + §90 source 标签）。
pub fn render_timeline_markdown(timeline: &Timeline) -> String {
    let mut out = String::new();
    out.push_str("# Project Timeline\n\n");
    for event in &timeline.events {
        out.push_str(&format!(
            "- {} [{}] {}: {}\n",
            event.timestamp,
            event.source.as_str(),
            format!("{:?}", event.kind).to_lowercase(),
            redact_text(&strip_control(&event.summary))
        ));
    }
    for limitation in &timeline.limitations {
        out.push_str(&format!("\n> {limitation}\n"));
    }
    out
}

/// History export（§61/§179/§180/§261/§262：JSON + export metadata + 再脱敏）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoryExport {
    pub schema_version: u32,
    pub exported_at: String,
    pub project: Value,
    pub snapshots: Vec<Value>,
    pub timeline: Timeline,
    pub limitations: Vec<String>,
}

pub fn render_history_export_json(
    project: &Value,
    snapshots: &[Value],
    timeline: &Timeline,
    limitations: &[String],
    exported_at: &str,
) -> String {
    let export = HistoryExport {
        schema_version: 1,
        exported_at: exported_at.to_string(),
        project: redact_value(project),
        snapshots: snapshots.iter().map(redact_value).collect(),
        timeline: redact_timeline(timeline),
        limitations: limitations.to_vec(),
    };
    serde_json::to_string_pretty(&export).unwrap_or_default()
}

fn redact_timeline(timeline: &Timeline) -> Timeline {
    let mut timeline = timeline.clone();
    for event in &mut timeline.events {
        event.summary = redact_text(&event.summary);
        if let Some(commit) = &event.git_commit {
            event.git_commit = Some(redact_text(commit));
        }
    }
    timeline
}

/// 值级脱敏（§261：export 前再过一次中央 redaction）。
fn redact_value(value: &Value) -> Value {
    match value {
        Value::String(text) => Value::String(redact_text(text)),
        Value::Array(items) => Value::Array(items.iter().map(redact_value).collect()),
        Value::Object(map) => {
            let mut out = BTreeMap::new();
            for (key, item) in map {
                out.insert(key.clone(), redact_value(item));
            }
            Value::Object(out.into_iter().collect())
        }
        other => other.clone(),
    }
}

fn strip_control(text: &str) -> String {
    // §244：终端/文档安全——剥离 C0 控制字符。
    text.chars()
        .filter(|c| !c.is_control() || *c == '\n' || *c == '\t')
        .collect()
}

/// counts map 从 snapshot JSON（store manifest/record）提取（compare 用）。
pub fn counts_from_json(json: &Value) -> BTreeMap<String, u64> {
    let mut out = BTreeMap::new();
    if let Some(counts) = json.get("counts").and_then(|counts| counts.as_object()) {
        for (key, value) in counts {
            if let Some(number) = value.as_u64() {
                out.insert(key.clone(), number);
            }
        }
    }
    out
}
