//! SnapshotComparator（规格 M11b §31–§42/§71–§78/§124/§160–§162/§245–§247）。
//!
//! 比较两个已持久化的快照（§113：直接用持久化规范化事实，不 checkout、
//! 不重跑分析器 §112）。全部输出为事实（§35/§46/§235 no hidden inference）。

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use serde_json::Value;

/// 单域 diff 条目（§124 域 diff 的公共形态）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DomainDiff {
    pub domain: String,
    /// changed | unchanged | unavailable | not_captured | incompatible
    pub state: String,
    /// 变化事实（每条可追溯，§172/§173）。
    pub facts: Vec<String>,
}

/// Finding 生命周期（§36/§72：FirstSeen/Active/Resolved/Recurring + unknown）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingLifecycle {
    New,
    Resolved,
    StillActive,
    Changed,
    Recurring,
    Unknown,
}

impl FindingLifecycle {
    pub const fn as_str(self) -> &'static str {
        match self {
            FindingLifecycle::New => "new",
            FindingLifecycle::Resolved => "resolved",
            FindingLifecycle::StillActive => "still_active",
            FindingLifecycle::Changed => "changed",
            FindingLifecycle::Recurring => "recurring",
            FindingLifecycle::Unknown => "unknown",
        }
    }
}

/// 健康 diff 条目（§36/§38）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FindingDiffEntry {
    pub finding_fingerprint: String,
    pub rule_id: String,
    pub lifecycle: FindingLifecycle,
    pub evidence: Vec<String>,
}

/// Verification 状态迁移（§73：只列状态变化，不解释原因）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationTransition {
    pub check: String,
    pub from: String,
    pub to: String,
}

/// SnapshotDiff（§32/§124）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotDiff {
    pub snapshot_a: String,
    pub snapshot_b: String,
    /// §71：compatible | limited（域模型版本不一致等）。
    pub compatibility: String,
    pub structure: DomainDiff,
    pub code: DomainDiff,
    pub architecture: DomainDiff,
    pub git: DomainDiff,
    pub change: DomainDiff,
    pub impact: DomainDiff,
    pub health: DomainDiff,
    pub verification: DomainDiff,
    pub environment: DomainDiff,
    pub context_health: DomainDiff,
    /// §38/§133/§134：finding 生命周期。
    pub finding_lifecycles: Vec<FindingDiffEntry>,
    /// §73：verification 状态迁移。
    pub verification_transitions: Vec<VerificationTransition>,
    pub limitations: Vec<String>,
}

/// 比较兼容性（§71）：域 data 版本不一致 ⇒ limited（不假装正常比较）。
fn compatibility_for(a: &Value, b: &Value) -> String {
    let version_a = a.get("model_version").and_then(|v| v.as_str());
    let version_b = b.get("model_version").and_then(|v| v.as_str());
    match (version_a, version_b) {
        (Some(va), Some(vb)) if va != vb => "limited".to_string(),
        _ => "compatible".to_string(),
    }
}

fn domain_state(domain_json: &Value) -> &str {
    domain_json
        .get("status")
        .and_then(|status| status.as_str())
        .unwrap_or("not_captured")
}

fn domain_content(domain_json: &Value) -> &Value {
    domain_json.get("data").unwrap_or(&Value::Null)
}

/// 通用标量集合 diff（added/removed 基于字符串集合）。
pub fn set_diff(a: &[String], b: &[String]) -> (Vec<String>, Vec<String>) {
    let set_a: std::collections::BTreeSet<&String> = a.iter().collect();
    let set_b: std::collections::BTreeSet<&String> = b.iter().collect();
    let added: Vec<String> = set_b.difference(&set_a).map(|item| (*item).clone()).collect();
    let removed: Vec<String> = set_a.difference(&set_b).map(|item| (*item).clone()).collect();
    (added, removed)
}

/// 比较两个快照的同一域 JSON（§113：持久化事实直比）。
pub fn compare_domain(domain: &str, a: &Value, b: &Value) -> DomainDiff {
    let state_a = domain_state(a);
    let state_b = domain_state(b);
    let mut facts = Vec::new();

    if state_a == "not_captured" && state_b == "not_captured" {
        return DomainDiff {
            domain: domain.to_string(),
            state: "not_captured".to_string(),
            facts,
        };
    }
    if state_a == "not_captured" || state_b == "not_captured" {
        facts.push(format!(
            "domain not captured in snapshot {} (other snapshot: {})",
            if state_a == "not_captured" { "A" } else { "B" },
            if state_a == "not_captured" { state_b } else { state_a }
        ));
        return DomainDiff {
            domain: domain.to_string(),
            state: "unavailable".to_string(),
            facts,
        };
    }

    // §245：Unavailable 语义 —— Unavailable ≠ empty。
    if state_a == "unavailable" || state_b == "unavailable" {
        facts.push(format!(
            "availability changed: {} -> {} (§245: unavailable ≠ empty)",
            state_a, state_b
        ));
        return DomainDiff {
            domain: domain.to_string(),
            state: if state_a == state_b { "unchanged" } else { "changed" }.to_string(),
            facts,
        };
    }

    // 结构化比较：**域内容**（wrapper.data）的顶层标量变化 + 集合 added/removed。
    let (content_a, content_b) = (domain_content(a), domain_content(b));
    let (obj_a, obj_b) = (content_a.as_object(), content_b.as_object());
    if let (Some(obj_a), Some(obj_b)) = (obj_a, obj_b) {
        let mut keys: std::collections::BTreeSet<&String> = obj_a.keys().collect();
        keys.extend(obj_b.keys());
        for key in keys {
            let value_a = obj_a.get(key).unwrap_or(&Value::Null);
            let value_b = obj_b.get(key).unwrap_or(&Value::Null);
            if value_a == value_b {
                continue;
            }
            match (value_a.as_array(), value_b.as_array()) {
                (Some(array_a), Some(array_b)) => {
                    let strings_a: Vec<String> = array_a
                        .iter()
                        .map(|item| item.to_string())
                        .collect();
                    let strings_b: Vec<String> = array_b
                        .iter()
                        .map(|item| item.to_string())
                        .collect();
                    let (added, removed) = set_diff(&strings_a, &strings_b);
                    for item in &added {
                        facts.push(format!("{key} added: {item}"));
                    }
                    for item in &removed {
                        facts.push(format!("{key} removed: {item}"));
                    }
                }
                _ => {
                    // §35：事实式陈述。
                    facts.push(format!(
                        "{key}: {} -> {}",
                        summary_value(value_a),
                        summary_value(value_b)
                    ));
                }
            }
        }
    }

    let state = if facts.is_empty() { "unchanged" } else { "changed" };
    DomainDiff {
        domain: domain.to_string(),
        state: state.to_string(),
        facts,
    }
}

fn summary_value(value: &Value) -> String {
    match value {
        Value::Null => "unavailable".to_string(),
        Value::String(text) => text.clone(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn structure_diff_reports_added_removed() {
        // wrapper 形态：{status, data}（persist 列映射）。
        let a = json!({ "status": "available", "data": { "files": ["a.ts", "b.ts"] } });
        let b = json!({ "status": "available", "data": { "files": ["b.ts", "c.ts"] } });
        let diff = compare_domain("structure", &a, &b);
        assert_eq!(diff.state, "changed");
        assert!(diff.facts.iter().any(|fact| fact.contains("added: \"c.ts\"")));
        assert!(diff.facts.iter().any(|fact| fact.contains("removed: \"a.ts\"")));
    }

    #[test]
    fn unavailable_is_not_empty() {
        // §245：Unavailable ≠ empty —— 状态迁移是事实，不当成空集合。
        let a = json!({ "status": "available", "data": { "tools": ["node"] } });
        let b = json!({ "status": "unavailable" });
        let diff = compare_domain("environment", &a, &b);
        assert!(diff.facts.iter().any(|fact| fact.contains("§245") || fact.contains("availability changed")));
    }

    #[test]
    fn not_captured_stays_not_captured() {
        let a = json!({ "status": "not_captured" });
        let b = json!({ "status": "not_captured" });
        let diff = compare_domain("health", &a, &b);
        assert_eq!(diff.state, "not_captured");
        assert!(diff.facts.is_empty());
    }

    #[test]
    fn unchanged_domain_reports_no_facts() {
        let a = json!({ "status": "available", "data": { "modules": 5 } });
        let b = json!({ "status": "available", "data": { "modules": 5 } });
        let diff = compare_domain("architecture", &a, &b);
        assert_eq!(diff.state, "unchanged");
        assert!(diff.facts.is_empty());
    }

    // §158/§205：数组顺序不影响 diff（canonicalize 在 fingerprint 侧已排序，
    // compare 直读持久化 JSON，顺序不同会产生 same-string 集合）。
    #[test]
    fn set_diff_is_order_insensitive() {
        let (added, removed) = set_diff(&["b".into(), "a".into()], &["a".into(), "c".into()]);
        assert_eq!(added, vec!["c".to_string()]);
        assert_eq!(removed, vec!["b".to_string()]);
    }
}

/// FindingFingerprint（§37：稳定身份 = rule_id + 归一化 subject/path，
/// 不含行号——避免"行号变了 => 误判 resolved+new"）。
pub fn finding_fingerprint(rule_id: &str, subject: &str, path: &str) -> String {
    let normalized_path = path.replace('\\', "/").to_lowercase();
    crate::model::fnv1a_hex(&format!("{rule_id}|{subject}|{normalized_path}"))
}

/// 从健康域 JSON 抽取 finding 指纹集合（持久化事实直读）。
pub fn collect_findings(health_json: &Value) -> std::collections::BTreeMap<String, String> {
    let mut out = std::collections::BTreeMap::new();
    if let Some(findings) = health_json.get("findings").and_then(|findings| findings.as_array()) {
        for finding in findings {
            let rule_id = finding
                .get("rule_id")
                .and_then(|value| value.as_str())
                .unwrap_or("unknown");
            let subject = finding
                .get("subject")
                .and_then(|value| value.as_str())
                .unwrap_or("");
            let path = finding
                .get("path")
                .and_then(|value| value.as_str())
                .unwrap_or("");
            let fingerprint = finding
                .get("finding_fingerprint")
                .and_then(|value| value.as_str())
                .map(str::to_string)
                .unwrap_or_else(|| finding_fingerprint(rule_id, subject, path));
            out.insert(fingerprint, rule_id.to_string());
        }
    }
    out
}

/// Finding 生命周期推导（§36/§38/§72/§133–§134）。
/// `unknown` 当 ruleset/model 不可确认时使用（不硬推断）。
pub fn finding_lifecycles(
    health_a: &Value,
    health_b: &Value,
    ruleset_compatible: bool,
) -> Vec<FindingDiffEntry> {
    let findings_a = collect_findings(health_a);
    let findings_b = collect_findings(health_b);
    let mut out = Vec::new();
    for (fingerprint, rule_id) in &findings_b {
        let lifecycle = if findings_a.contains_key(fingerprint) {
            FindingLifecycle::StillActive
        } else if !ruleset_compatible {
            FindingLifecycle::Unknown
        } else {
            FindingLifecycle::New
        };
        out.push(FindingDiffEntry {
            finding_fingerprint: fingerprint.clone(),
            rule_id: rule_id.clone(),
            lifecycle,
            evidence: vec![format!("present in B: {fingerprint}")],
        });
    }
    for (fingerprint, rule_id) in &findings_a {
        if !findings_b.contains_key(fingerprint) {
            let lifecycle = if !ruleset_compatible {
                FindingLifecycle::Unknown
            } else {
                FindingLifecycle::Resolved
            };
            out.push(FindingDiffEntry {
                finding_fingerprint: fingerprint.clone(),
                rule_id: rule_id.clone(),
                lifecycle,
                evidence: vec![format!("absent from B: {fingerprint}")],
            });
        }
    }
    out.sort_by(|a, b| {
        a.rule_id
            .cmp(&b.rule_id)
            .then_with(|| a.finding_fingerprint.cmp(&b.finding_fingerprint))
    });
    out
}

/// Verification 状态迁移（§73：状态变化是事实，不解释原因）。
pub fn verification_transitions(verification_a: &Value, verification_b: &Value) -> Vec<VerificationTransition> {
    let runs = |json: &Value| -> std::collections::BTreeMap<String, String> {
        let mut map = std::collections::BTreeMap::new();
        if let Some(items) = json.get("checks").and_then(|checks| checks.as_array()) {
            for check in items {
                if let (Some(name), Some(status)) = (
                    check.get("name").and_then(|name| name.as_str()),
                    check.get("status").and_then(|status| status.as_str()),
                ) {
                    map.insert(name.to_string(), status.to_string());
                }
            }
        }
        map
    };
    let runs_a = runs(verification_a);
    let runs_b = runs(verification_b);
    let mut out = Vec::new();
    for (name, status_b) in &runs_b {
        if let Some(status_a) = runs_a.get(name) {
            if status_a != status_b {
                out.push(VerificationTransition {
                    check: name.clone(),
                    from: status_a.clone(),
                    to: status_b.clone(),
                });
            }
        }
    }
    out
}

#[cfg(test)]
mod finding_tests {
    use super::*;
    use serde_json::json;

    fn finding(rule: &str, subject: &str, path: &str, line: u64) -> Value {
        json!({
            "rule_id": rule,
            "subject": subject,
            "path": path,
            "line": line,
        })
    }

    // §37/§134：行号变化不改变身份（fingerprint 不含行号）。
    #[test]
    fn finding_fingerprint_stable_across_line_changes() {
        let a = finding("ARCH003", "Registry", "src/registry.ts", 52);
        let b = finding("ARCH003", "Registry", "src/registry.ts", 73);
        let findings_a = collect_findings(&json!({ "findings": [a] }));
        let findings_b = collect_findings(&json!({ "findings": [b] }));
        let keys_a: Vec<&String> = findings_a.keys().collect();
        let keys_b: Vec<&String> = findings_b.keys().collect();
        assert_eq!(keys_a, keys_b, "§37: same logical finding = same fingerprint");
        // 且不误判成 resolved+new。
        let lifecycles = finding_lifecycles(
            &json!({ "findings": [finding("ARCH003", "Registry", "src/registry.ts", 52)] }),
            &json!({ "findings": [finding("ARCH003", "Registry", "src/registry.ts", 73)] }),
            true,
        );
        assert_eq!(lifecycles.len(), 1);
        assert_eq!(lifecycles[0].lifecycle, FindingLifecycle::StillActive);
    }

    // §161：new/resolved/still_active/unknown 全覆盖。
    #[test]
    fn finding_lifecycle_all_states() {
        let health_a = json!({ "findings": [
            finding("R1", "s1", "a.ts", 1),
            finding("R2", "s2", "b.ts", 2),
            finding("R3", "s3", "c.ts", 3),
        ]});
        let health_b = json!({ "findings": [
            finding("R1", "s1", "a.ts", 1),
            finding("R4", "s4", "d.ts", 4),
        ]});
        let lifecycles = finding_lifecycles(&health_a, &health_b, true);
        let by_rule: std::collections::BTreeMap<&str, FindingLifecycle> = lifecycles
            .iter()
            .map(|entry| (entry.rule_id.as_str(), entry.lifecycle))
            .collect();
        assert_eq!(by_rule["R1"], FindingLifecycle::StillActive);
        assert_eq!(by_rule["R2"], FindingLifecycle::Resolved);
        assert_eq!(by_rule["R3"], FindingLifecycle::Resolved);
        assert_eq!(by_rule["R4"], FindingLifecycle::New);

        // ruleset 不兼容 ⇒ unknown（不硬推断 §72）。
        let lifecycles = finding_lifecycles(&health_a, &health_b, false);
        assert!(lifecycles
            .iter()
            .all(|entry| entry.lifecycle == FindingLifecycle::Unknown
                || entry.lifecycle == FindingLifecycle::StillActive));
    }

    // §73：verification 状态迁移只列事实。
    #[test]
    fn verification_transitions_report_state_changes() {
        let a = json!({ "checks": [
            { "name": "build", "status": "passed" },
            { "name": "test", "status": "passed" },
        ]});
        let b = json!({ "checks": [
            { "name": "build", "status": "passed" },
            { "name": "test", "status": "failed" },
        ]});
        let transitions = verification_transitions(&a, &b);
        assert_eq!(transitions.len(), 1);
        assert_eq!(transitions[0].check, "test");
        assert_eq!(transitions[0].from, "passed");
        assert_eq!(transitions[0].to, "failed");
    }
}

/// 完整 SnapshotDiff 构建（§124）：对十个域逐域比较，并推导
/// finding 生命周期与 verification 迁移（§36/§38/§39/§72/§73）。
pub fn build_snapshot_diff(
    snapshot_a_id: &str,
    snapshot_b_id: &str,
    domain_json_a: &BTreeMap<String, Value>,
    domain_json_b: &BTreeMap<String, Value>,
) -> SnapshotDiff {
    let domain_names = [
        "structure", "code", "architecture", "git", "change", "impact", "health",
        "verification", "environment", "context_health",
    ];
    let empty = Value::Null;
    let mut domain_diffs: BTreeMap<String, DomainDiff> = BTreeMap::new();
    let mut limitations = Vec::new();
    let mut compatibility = "compatible".to_string();

    for domain in domain_names {
        let json_a = domain_json_a.get(domain).unwrap_or(&empty);
        let json_b = domain_json_b.get(domain).unwrap_or(&empty);
        let compatibility = compatibility_for(json_a, json_b);
        if compatibility == "limited" {
            limitations.push(format!(
                "domain {domain}: model version mismatch — comparison limited (§71)"
            ));
        }
        let diff = compare_domain(domain, json_a, json_b);
        domain_diffs.insert(domain.to_string(), diff);
    }
    if !limitations.is_empty() {
        compatibility = "limited".to_string();
    }

    let health_a = domain_json_a.get("health").unwrap_or(&empty);
    let health_b = domain_json_b.get("health").unwrap_or(&empty);
    let finding_lifecycles = finding_lifecycles(health_a, health_b, compatibility == "compatible");
    let verification_a = domain_json_a.get("verification").unwrap_or(&empty);
    let verification_b = domain_json_b.get("verification").unwrap_or(&empty);
    let verification_transitions = verification_transitions(verification_a, verification_b);

    let get = |domain: &str| domain_diffs.get(domain).cloned().unwrap_or(DomainDiff {
        domain: domain.to_string(),
        state: "unchanged".to_string(),
        facts: vec![],
    });

    SnapshotDiff {
        snapshot_a: snapshot_a_id.to_string(),
        snapshot_b: snapshot_b_id.to_string(),
        compatibility,
        structure: get("structure"),
        code: get("code"),
        architecture: get("architecture"),
        git: get("git"),
        change: get("change"),
        impact: get("impact"),
        health: get("health"),
        verification: get("verification"),
        environment: get("environment"),
        context_health: get("context_health"),
        finding_lifecycles,
        verification_transitions,
        limitations,
    }
}
