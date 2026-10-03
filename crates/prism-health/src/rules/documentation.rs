//! 文档规则（规格 M7 §51–§54/§112/§156/§214/§215）。
//!
//! 只做**事实一致性**：README 代码块里引用的 npm script 在 package.json
//! 不存在（DOC001）；README 链接/代码块引用的项目相对路径不存在（DOC002）。
//! 措辞克制（§215 "appears inconsistent"）；只识别代码块与链接——普通
//! 散文里的路径不产生 finding（§112）。仅 Node 项目启用 DOC001（§114）。

use crate::model::{
    Evidence, EvidenceKind, Finding, FindingCategory, FindingExplanation, FindingLocation,
    FindingStatus, RuleResult,
};
use crate::registry::{HealthRule, HealthRuleMeta};
use prism_core::severity::{Confidence, Severity};
use prism_core::task::CancellationToken;
use prism_core::PrismError;
use std::sync::LazyLock;

static DOC001: LazyLock<HealthRuleMeta> = LazyLock::new(|| HealthRuleMeta {
    id: "DOC001",
    name: "Documented Command Not Found",
    category: FindingCategory::Documentation,
    description: "A README code-block command references an npm script that package.json does not define.",
    default_severity: Severity::Info,
    default_enabled: true,
    threshold_meaning: None,
    needs_derived: true,
});

static DOC002: LazyLock<HealthRuleMeta> = LazyLock::new(|| HealthRuleMeta {
    id: "DOC002",
    name: "Documented Path Not Found",
    category: FindingCategory::Documentation,
    description: "A README link or fenced reference points to a project-relative path that does not exist.",
    default_severity: Severity::Info,
    default_enabled: true,
    threshold_meaning: None,
    needs_derived: true,
});

/// fenced code block 内的行集合（``` 界定；~~~ 忽略——第一版）。
fn fenced_lines(content: &str) -> Vec<(usize, &str)> {
    let mut inside = false;
    let mut lines = Vec::new();
    for (index, line) in content.lines().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") {
            inside = !inside;
            continue;
        }
        if inside {
            lines.push((index + 1, line));
        }
    }
    lines
}

/// package.json 的 scripts 名集合（解析失败 ⇒ None，DOC001 安静不适用）。
fn package_scripts(content: &str) -> Option<Vec<String>> {
    let value: serde_json::Value = serde_json::from_str(content).ok()?;
    let scripts = value.get("scripts")?.as_object()?;
    Some(scripts.keys().cloned().collect())
}

/// fenced 行中的 `npm/yarn/pnpm run <name>` 引用。
fn referenced_runs(line: &str) -> Vec<String> {
    let mut names = Vec::new();
    for manager in ["npm run", "yarn run", "pnpm run"] {
        let mut cursor = 0;
        while let Some(found) = line[cursor..].find(manager) {
            let start = cursor + found + manager.len();
            let rest = &line[start..];
            let name: String = rest
                .chars()
                .skip_while(|c| c.is_whitespace())
                .take_while(|c| c.is_alphanumeric() || *c == ':' || *c == '-' || *c == '_')
                .collect();
            if !name.is_empty() {
                names.push(name);
            }
            cursor = start;
            if cursor >= line.len() {
                break;
            }
        }
    }
    names.sort();
    names.dedup();
    names
}

/// markdown 链接目标（项目相对路径，过滤 URL/锚点/邮箱）。
fn referenced_paths(line: &str) -> Vec<String> {
    let mut paths = Vec::new();
    let mut cursor = 0;
    while let Some(open) = line[cursor..].find("](") {
        let start = cursor + open + 2;
        let Some(close_rel) = line[start..].find(')') else {
            break;
        };
        let target = line[start..start + close_rel].to_string();
        cursor = start;
        let not_external = !target.starts_with("http://")
            && !target.starts_with("https://")
            && !target.starts_with('#')
            && !target.contains('@');
        if not_external && !target.is_empty() {
            paths.push(target);
        }
        if cursor >= line.len() {
            break;
        }
    }
    paths
}

fn doc_finding(
    rule_id: &'static str,
    title: &'static str,
    what: String,
    line_number: usize,
    subject: String,
    detail: String,
) -> Finding {
    Finding {
        id: Finding::derive_id(rule_id, &format!("README.md:{line_number}:{subject}")),
        rule_id: rule_id.to_string(),
        category: FindingCategory::Documentation,
        severity: Severity::Info,
        confidence: Confidence::Medium,
        title: title.to_string(),
        message: format!("README.md:{line_number} appears inconsistent with project facts."),
        explanation: FindingExplanation {
            what,
            evidence_summary: detail.clone(),
            why_it_matters: Some(
                "Documented commands or paths that no longer exist can mislead new contributors; READMEs may also intentionally reference legacy commands.".to_string(),
            ),
        },
        location: FindingLocation::new("README.md", line_number as u32, 1),
        related_locations: vec![],
        related_symbols: vec![],
        related_modules: vec![],
        related_files: vec!["README.md".to_string()],
        evidence: vec![Evidence::new(EvidenceKind::ConfigFact, subject, detail)
            .with_location(FindingLocation::new("README.md", line_number as u32, 1))],
        suggested_action: Some("Verify the documentation reference against current project facts.".to_string()),
        status: FindingStatus::Active,
            change_relation: None,
        metadata: BTreeMap::new(),
    }
}

use std::collections::BTreeMap;

fn readme_content(context: &crate::engine::HealthContext) -> Option<(String, String)> {
    let derived = context.derived?;
    let content = derived.file_contents.get("README.md")?.clone();
    Some(("README.md".to_string(), content))
}

fn project_has_node(scripts: &Option<Vec<String>>) -> bool {
    scripts.is_some()
}

pub struct CommandDriftRule;

impl HealthRule for CommandDriftRule {
    fn meta(&self) -> &'static HealthRuleMeta {
        &DOC001
    }

    fn evaluate(
        &self,
        context: &crate::engine::HealthContext,
        _config: &prism_core::config::HealthRuleConfig,
        cancel: &CancellationToken,
    ) -> prism_core::PrismResult<RuleResult> {
        let mut result = RuleResult::default();
        let Some((_, readme)) = readme_content(context) else {
            return Ok(result);
        };
        let Some(derived) = context.derived else {
            return Ok(result);
        };
        // §114：只有真实存在 package.json（Node 项目）才检查 npm scripts。
        let scripts = derived
            .file_contents
            .get("package.json")
            .and_then(|content| package_scripts(content));
        if !project_has_node(&scripts) {
            return Ok(result);
        }
        let scripts = scripts.unwrap();
        for (line_number, line) in fenced_lines(&readme) {
            if cancel.is_cancelled() {
                return Err(PrismError::Cancelled);
            }
            for name in referenced_runs(line) {
                if !scripts.iter().any(|script| script == &name) {
                    result.findings.push(doc_finding(
                        "DOC001",
                        "Documented Command Not Found",
                        format!(
                            "Documented command \"npm run {name}\" appears inconsistent with package.json scripts."
                        ),
                        line_number,
                        name.clone(),
                        format!(
                            "documented command = \"npm run {name}\"; package.json defines: {}",
                            scripts.join(", ")
                        ),
                    ));
                }
            }
        }
        Ok(result)
    }
}

pub struct PathDriftRule;

impl HealthRule for PathDriftRule {
    fn meta(&self) -> &'static HealthRuleMeta {
        &DOC002
    }

    fn evaluate(
        &self,
        context: &crate::engine::HealthContext,
        _config: &prism_core::config::HealthRuleConfig,
        cancel: &CancellationToken,
    ) -> prism_core::PrismResult<RuleResult> {
        let mut result = RuleResult::default();
        let Some((_, readme)) = readme_content(context) else {
            return Ok(result);
        };
        // §53：markdown 链接本身即优先信号（全文扫描）；代码块行同样纳入。
        let mut scan_targets: Vec<(usize, &str)> = fenced_lines(&readme);
        for (index, line) in readme.lines().enumerate() {
            scan_targets.push((index + 1, line));
        }
        scan_targets.sort();
        scan_targets.dedup();
        for (line_number, line) in scan_targets {
            if cancel.is_cancelled() {
                return Err(PrismError::Cancelled);
            }
            for target in referenced_paths(line) {
                let clean = target.split('#').next().unwrap_or(&target);
                let clean = clean.trim_end_matches('/');
                if clean.is_empty() {
                    continue;
                }
                if !context.root.join(clean).exists() {
                    result.findings.push(doc_finding(
                        "DOC002",
                        "Documented Path Not Found",
                        format!("Documented path \"{target}\" does not exist in the project."),
                        line_number,
                        target.clone(),
                        format!("documented path = \"{target}\"; not found on disk"),
                    ));
                }
            }
        }
        Ok(result)
    }
}
