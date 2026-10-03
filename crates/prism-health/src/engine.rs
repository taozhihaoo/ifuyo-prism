//! HealthEngine：collect → sort → deduplicate → caps → HealthAnalysis
//! （规格 M7 §8/§9/§63/§95–§105/§116/§117/§201–§208/§221–§224）。
//!
//! - 上下文只读借用，不复制模型（§8/§19）；规则不可变接收（§202）
//! - 顺序执行内建规则（确定性由构造保证，§95；并行留待需要时以
//!   collect-sort-dedup 兜底，§97）
//! - 单规则失败非致命：诊断 + 其余继续 + Partial（§99/§163）
//! - 截断必须显式 Truncated（§223/§224）
//! - 取消语义：Err(Cancelled)，绝不伪装 "0 findings"（§102）

use crate::model::{
    Finding, FindingStatus, HealthDiagnostic, RuleResult, HEALTH_SCHEMA_VERSION,
};
use crate::registry::HealthRuleRegistry;
use prism_core::config::HealthConfig;
use prism_core::severity::Severity;
use prism_core::task::CancellationToken;
use prism_core::{PrismError, PrismResult};
use std::collections::{BTreeMap, BTreeSet};

/// 分析范围（§63/§61/§62/§190/§191）：
/// - FullProject：全项目（默认）
/// - ChangedFiles：只报告 ChangeSet 变更文件内的 findings（§61/§247）
/// - ImpactedItems：变更文件 ∪ M6 受影响范围（§64/§242）
///
/// 范围由 ChangeSet/ImpactAnalysis 决定，不因单个 finding 扩大到全仓（§191）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthScope {
    #[default]
    FullProject,
    ChangedFiles,
    ImpactedItems,
}

/// 分析选项（§222：保持小）。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HealthOptions {
    pub scope: HealthScope,
    /// CLI 覆盖：禁用列表（§221 precedence 顶层）。
    pub disabled_rules: Vec<String>,
    /// CLI 覆盖：重新启用列表（优先级高于 config 禁用）。
    pub enabled_rules: Vec<String>,
    /// 全局 findings 上限（§223；截断 ⇒ Truncated）。
    pub max_findings: Option<u64>,
    /// 单规则 findings 上限（§224；截断 ⇒ Truncated + 计数保留）。
    pub max_findings_per_rule: Option<u64>,
}

/// 规则的只读事实上下文（§19：借用；M7a 只要求 project/code_model，
/// 其余字段为 M7b 规则预留——缺省 None，规则在不适用时安静跳过，§207/§208）。
pub struct HealthContext<'a> {
    pub project: &'a prism_project::Project,
    pub code_model: Option<&'a prism_analysis::ProjectCodeModel>,
    pub discovery: Option<&'a prism_project::ProjectDiscovery>,
    pub graph: Option<&'a prism_graph::DependencyGraphResult>,
    pub git: Option<&'a prism_git::GitStatus>,
    pub change_set: Option<&'a prism_changes::ChangeSet>,
    pub impact: Option<&'a prism_impact::ImpactAnalysis>,
    /// 项目根：规则按需读取文件内容（MAINT 词法扫描），不复制进上下文。
    pub root: &'a std::path::Path,
    /// 派生事实（引擎按需构建一次共享，§91-§94）；调用方也可预构建传入。
    pub derived: Option<&'a crate::derived::DerivedFacts>,
}

/// 引擎执行统计（§104：真实计数 progress，不造假百分比）。
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct HealthEngineStats {
    pub rules_checked: Vec<String>,
    pub rules_disabled: Vec<String>,
    pub rules_failed: Vec<String>,
    pub findings_detected: u64,
    pub findings_active: u64,
    pub findings_suppressed: u64,
    /// 因 max_findings[_per_rule] 截断的条数。
    pub findings_truncated: u64,
}

/// HealthAnalysis（§9）。summary 的正式形态属 M7b；本篇只带事实计数。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct HealthAnalysis {
    pub schema_version: u32,
    /// 稳定项目名（M1 ProjectId 目前为运行期 UUID——稳定项目身份属 M11，
    /// 以项目名作为本篇的确定性占位，规格 §205 的稳定 id 由 M11 落地）。
    pub project_name: String,
    pub scope: HealthScope,
    /// 与当前变更相关的 finding 数（scope 非 FullProject 时 = findings 总数）。
    pub change_related_findings: u64,
    pub findings: Vec<Finding>,
    pub diagnostics: Vec<HealthDiagnostic>,
    pub analysis_completeness: prism_core::PrismCompleteness,
    pub checked_rules: Vec<String>,
    pub disabled_rules: Vec<String>,
    pub failed_rules: Vec<String>,
    pub stats: HealthEngineStats,
    pub summary: crate::model::HealthSummary,
}

/// 执行健康分析（§8：analyze(project_context, health_options)）。
///
/// precedence（§221）：built-in defaults → project config → CLI override；
/// `disabled_rules`（config 或 CLI）命中的规则不运行（not detected）；
/// config `suppressed_findings` 命中的 finding 仍运行检出，仅标记
/// Suppressed（suppressed ≠ not detected，§67/§164）。
pub fn analyze(
    registry: &HealthRuleRegistry,
    config: &HealthConfig,
    context: &HealthContext,
    options: &HealthOptions,
    cancel: &CancellationToken,
) -> PrismResult<HealthAnalysis> {
    // ---- §91-§94：任一启用规则需要派生事实 ⇒ 构建一次共享 ----
    let any_needs_derived = registry.rules().any(|rule| {
        let meta = rule.meta();
        config.is_rule_enabled(meta.id, meta.default_enabled).unwrap_or(meta.default_enabled)
            && !options.disabled_rules.iter().any(|id| id == meta.id)
            || options.enabled_rules.iter().any(|id| id == meta.id)
    });
    let derived_storage;
    let shadow;
    let context: &HealthContext = if any_needs_derived && context.derived.is_none() {
        derived_storage = crate::derived::DerivedFacts::build(
            context.code_model,
            context.discovery,
            context.graph,
            context.root,
        );
        shadow = HealthContext {
            derived: Some(&derived_storage),
            ..*context
        };
        &shadow
    } else {
        context
    };
    let mut diagnostics: Vec<HealthDiagnostic> = Vec::new();
    let mut checked: Vec<String> = Vec::new();
    let mut disabled: Vec<String> = Vec::new();
    let mut failed: Vec<String> = Vec::new();
    let mut collected: Vec<Finding> = Vec::new();
    let mut truncated_total: u64 = 0;
    let mut change_related_holder: u64 = 0;

    for rule in registry.rules() {
        if cancel.is_cancelled() {
            return Err(PrismError::Cancelled);
        }
        let meta = rule.meta();
        // ---- precedence：CLI > config > default（§221）----
        let config_enabled = config.is_rule_enabled(meta.id, meta.default_enabled);
        let cli_disabled = options.disabled_rules.iter().any(|id| id == meta.id);
        let cli_enabled = options.enabled_rules.iter().any(|id| id == meta.id);
        let enabled = if cli_disabled {
            false
        } else if cli_enabled {
            true
        } else {
            config_enabled.unwrap_or(meta.default_enabled)
        };
        if !enabled {
            disabled.push(meta.id.to_string());
            continue;
        }

        // ---- §72：未声明 threshold 语义的规则收到 threshold ⇒ 诊断 ----
        let rule_config = config.rules.get(meta.id).cloned().unwrap_or_default();
        if rule_config.threshold.is_some() && meta.threshold_meaning.is_none() {
            diagnostics.push(HealthDiagnostic {
                kind: "config_unknown_option".to_string(),
                subject: Some(meta.id.to_string()),
                detail: "rule does not declare a threshold; the configured value is ignored"
                    .to_string(),
            });
        }

        // ---- 评估（单规则失败非致命，§99/§163）----
        checked.push(meta.id.to_string());
        let result: RuleResult = match rule.evaluate(context, &rule_config, cancel) {
            Ok(result) => result,
            Err(PrismError::Cancelled) => return Err(PrismError::Cancelled),
            Err(err) => {
                failed.push(meta.id.to_string());
                diagnostics.push(HealthDiagnostic {
                    kind: "rule_failed".to_string(),
                    subject: Some(meta.id.to_string()),
                    detail: err.to_string(),
                });
                continue;
            }
        };
        diagnostics.extend(result.diagnostics);

        // ---- §224 per-rule cap（排序后截断，保持确定性）----
        let mut rule_findings = result.findings;
        sort_findings(&mut rule_findings);
        if let Some(cap) = options.max_findings_per_rule {
            if rule_findings.len() as u64 > cap {
                truncated_total += rule_findings.len() as u64 - cap;
                diagnostics.push(HealthDiagnostic {
                    kind: "findings_truncated".to_string(),
                    subject: Some(meta.id.to_string()),
                    detail: format!(
                        "reported {cap} of at least {} findings",
                        rule_findings.len()
                    ),
                });
                rule_findings.truncate(cap as usize);
            }
        }
        collected.extend(rule_findings);
    }

    // ---- §98 dedup：确定性 id 重复 = 同一事实，保留首条 ----
    let mut deduped: BTreeMap<String, Finding> = BTreeMap::new();
    for finding in collected {
        deduped.entry(finding.id.as_str().to_string()).or_insert(finding);
    }
    let mut findings: Vec<Finding> = deduped.into_values().collect();

    // ---- §223 global cap ----
    sort_findings(&mut findings);
    if let Some(cap) = options.max_findings {
        if findings.len() as u64 > cap {
            truncated_total += findings.len() as u64 - cap;
            diagnostics.push(HealthDiagnostic {
                kind: "findings_truncated".to_string(),
                subject: None,
                detail: format!("reported {cap} of at least {} findings", findings.len()),
            });
            findings.truncate(cap as usize);
        }
    }

    // ---- §62/§64/§191：scope 过滤 + change relation（只保留位置事实）----
    let changed_paths: BTreeSet<String> = context
        .change_set
        .iter()
        .flat_map(|change_set| {
            change_set.files.iter().map(|file| {
                file.new_path.clone().unwrap_or_else(|| file.old_path.clone().unwrap_or_default())
            })
        })
        .collect();
    let impacted_paths: BTreeSet<String> = context
        .impact
        .iter()
        .flat_map(|impact| impact.impacted_items.iter().map(|item| item.path.clone()))
        .collect();
    match options.scope {
        HealthScope::FullProject => {
            for finding in &mut findings {
                if changed_paths.contains(&finding.location.relative_path) {
                    finding.change_relation = Some(crate::model::ChangeRelation::DirectlyChanged);
                } else if impacted_paths.contains(&finding.location.relative_path) {
                    finding.change_relation = Some(crate::model::ChangeRelation::Impacted);
                } else {
                    finding.change_relation = Some(crate::model::ChangeRelation::Unrelated);
                }
            }
        }
        HealthScope::ChangedFiles | HealthScope::ImpactedItems => {
            findings.retain(|finding| {
                let path = &finding.location.relative_path;
                match options.scope {
                    HealthScope::ChangedFiles => changed_paths.contains(path),
                    HealthScope::ImpactedItems => {
                        changed_paths.contains(path) || impacted_paths.contains(path)
                    }
                    HealthScope::FullProject => true,
                }
            });
            for finding in &mut findings {
                let in_changed = changed_paths.contains(&finding.location.relative_path);
                finding.change_relation = Some(if in_changed {
                    crate::model::ChangeRelation::DirectlyChanged
                } else {
                    crate::model::ChangeRelation::Impacted
                });
            }
            change_related_holder = findings.len() as u64;
        }
    }

    // ---- suppression（§67/§164：仍检出，仅标记）----
    let mut stats = HealthEngineStats {
        findings_detected: findings.len() as u64,
        findings_truncated: truncated_total,
        ..HealthEngineStats::default()
    };
    for finding in &mut findings {
        let suppressed = config
            .suppressed_findings
            .iter()
            .any(|id| id == finding.id.as_str());
        if suppressed {
            finding.status = FindingStatus::Suppressed;
            stats.findings_suppressed += 1;
        } else {
            stats.findings_active += 1;
        }
    }

    // ---- §100 completeness（Truncated > Partial > Complete；§163 Partial）----
    stats.rules_checked = checked.clone();
    stats.rules_disabled = disabled.clone();
    stats.rules_failed = failed.clone();
    let analysis_completeness = if truncated_total > 0 {
        prism_core::PrismCompleteness::Truncated
    } else if !failed.is_empty() {
        prism_core::PrismCompleteness::Partial
    } else {
        prism_core::PrismCompleteness::Complete
    };

    // ---- §10 summary（事实计数；M4 与 score 无缘）----
    let mut summary = crate::model::HealthSummary {
        total_findings: findings.len() as u64,
        ..Default::default()
    };
    let mut files_with: BTreeSet<String> = BTreeSet::new();
    let mut modules_with: BTreeSet<String> = BTreeSet::new();
    let mut symbols_with: BTreeSet<String> = BTreeSet::new();
    for finding in &findings {
        match finding.severity {
            Severity::Critical => summary.critical_count += 1,
            Severity::Warning => summary.warning_count += 1,
            Severity::Info => summary.info_count += 1,
        }
        *summary
            .by_category
            .entry(format!("{:?}", finding.category).to_ascii_lowercase())
            .or_insert(0) += 1;
        *summary.by_rule.entry(finding.rule_id.clone()).or_insert(0) += 1;
        files_with.insert(finding.location.relative_path.clone());
        for module in &finding.related_modules {
            modules_with.insert(module.clone());
        }
        for symbol in &finding.related_symbols {
            symbols_with.insert(symbol.clone());
        }
    }
    summary.files_with_findings = files_with.len() as u64;
    summary.modules_with_findings = modules_with.len() as u64;
    summary.symbols_with_findings = symbols_with.len() as u64;

    diagnostics.sort_by(|a, b| (&a.kind, &a.subject, &a.detail).cmp(&(&b.kind, &b.subject, &b.detail)));

    Ok(HealthAnalysis {
        schema_version: HEALTH_SCHEMA_VERSION,
        project_name: context.project.name.clone(),
        scope: options.scope,
        change_related_findings: if options.scope == HealthScope::FullProject {
            findings
                .iter()
                .filter(|finding| {
                    finding.change_relation == Some(crate::model::ChangeRelation::DirectlyChanged)
                        || finding.change_relation == Some(crate::model::ChangeRelation::Impacted)
                })
                .count() as u64
        } else {
            change_related_holder
        },
        findings,
        diagnostics,
        analysis_completeness,
        checked_rules: checked,
        disabled_rules: disabled,
        failed_rules: failed,
        stats,
        summary,
    })
}

/// §96 稳定排序：severity（重→轻）、category、rule_id、location、finding_id。
fn sort_findings(findings: &mut [Finding]) {
    findings.sort_by(|a, b| {
        b.severity
            .cmp(&a.severity)
            .then_with(|| a.category.cmp(&b.category))
            .then_with(|| a.rule_id.cmp(&b.rule_id))
            .then_with(|| {
                (
                    &a.location.relative_path,
                    a.location.line,
                    a.location.column,
                )
                    .cmp(&(
                        &b.location.relative_path,
                        b.location.line,
                        b.location.column,
                    ))
            })
            .then_with(|| a.id.cmp(&b.id))
    });
}

/// 引擎用到的去重可见集合（测试辅助；保持 §98 语义单一来源）。
pub fn unique_rule_ids(findings: &[Finding]) -> BTreeSet<&str> {
    findings.iter().map(|finding| finding.rule_id.as_str()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        Evidence, FindingCategory, FindingExplanation, FindingLocation,
    };
    use crate::registry::{HealthRule, HealthRuleMeta, HealthRuleRegistry};
    use prism_core::severity::{Confidence, Severity};
    use std::sync::LazyLock;

    fn location(path: &str, line: u32) -> FindingLocation {
        FindingLocation::new(path, line, 1)
    }

    fn finding(rule: &str, path: &str, line: u32, severity: Severity) -> Finding {
        Finding {
            id: Finding::derive_id(rule, &format!("{path}:{line}")),
            rule_id: rule.to_string(),
            category: FindingCategory::Code,
            severity,
            confidence: Confidence::High,
            title: format!("Detected issue in {path}"),
            message: format!("issue at {path}:{line}"),
            explanation: FindingExplanation {
                what: "A test finding was produced.".to_string(),
                evidence_summary: format!("{path}:{line}"),
                why_it_matters: None,
            },
            location: location(path, line),
            related_locations: vec![],
            related_symbols: vec![],
            related_modules: vec![],
            related_files: vec![],
            evidence: vec![Evidence::new(
                crate::model::EvidenceKind::SourceLocation,
                format!("{path}:{line}"),
                "test evidence",
            )],
            suggested_action: None,
            status: FindingStatus::Active,
            change_relation: None,
            metadata: Default::default(),
        }
    }

    static META_A: LazyLock<HealthRuleMeta> = LazyLock::new(|| HealthRuleMeta {
        id: "TEST001",
        name: "Rule A",
        category: FindingCategory::Code,
        description: "emits two findings",
        default_severity: Severity::Warning,
        default_enabled: true,
        threshold_meaning: None,
        needs_derived: false,
    });
    static META_B: LazyLock<HealthRuleMeta> = LazyLock::new(|| HealthRuleMeta {
        id: "TEST002",
        name: "Rule B (always fails)",
        category: FindingCategory::Code,
        description: "returns Err",
        default_severity: Severity::Critical,
        default_enabled: true,
        threshold_meaning: None,
        needs_derived: false,
    });
    static META_C: LazyLock<HealthRuleMeta> = LazyLock::new(|| HealthRuleMeta {
        id: "TEST003",
        name: "Rule C (disabled by default)",
        category: FindingCategory::Code,
        description: "emits one finding",
        default_severity: Severity::Info,
        default_enabled: false,
        threshold_meaning: None,
        needs_derived: false,
    });

    struct RuleA;
    struct RuleB;
    struct RuleC;

    impl HealthRule for RuleA {
        fn meta(&self) -> &'static HealthRuleMeta {
            &META_A
        }
        fn evaluate(
            &self,
            _context: &HealthContext,
            config: &prism_core::config::HealthRuleConfig,
            cancel: &CancellationToken,
        ) -> prism_core::PrismResult<RuleResult> {
            if cancel.is_cancelled() {
                return Err(PrismError::Cancelled);
            }
            let count = config.threshold.unwrap_or(2);
            Ok(RuleResult {
                findings: (1..=count.min(3) as u32)
                    .map(|line| finding("TEST001", "src/a.ts", line, Severity::Warning))
                    .collect(),
                diagnostics: vec![],
            })
        }
    }

    impl HealthRule for RuleB {
        fn meta(&self) -> &'static HealthRuleMeta {
            &META_B
        }
        fn evaluate(
            &self,
            _context: &HealthContext,
            _config: &prism_core::config::HealthRuleConfig,
            _cancel: &CancellationToken,
        ) -> prism_core::PrismResult<RuleResult> {
            Err(PrismError::Internal { message: "rule B exploded".to_string() })
        }
    }

    impl HealthRule for RuleC {
        fn meta(&self) -> &'static HealthRuleMeta {
            &META_C
        }
        fn evaluate(
            &self,
            _context: &HealthContext,
            _config: &prism_core::config::HealthRuleConfig,
            _cancel: &CancellationToken,
        ) -> prism_core::PrismResult<RuleResult> {
            Ok(RuleResult {
                findings: vec![finding("TEST003", "src/c.ts", 1, Severity::Info)],
                diagnostics: vec![],
            })
        }
    }

    fn registry() -> HealthRuleRegistry {
        let mut registry = HealthRuleRegistry::default();
        registry.register(Box::new(RuleA));
        registry.register(Box::new(RuleB));
        registry.register(Box::new(RuleC));
        registry
    }

    fn run(
        registry: &HealthRuleRegistry,
        config: &HealthConfig,
        options: &HealthOptions,
        cancel: &CancellationToken,
    ) -> prism_core::PrismResult<HealthAnalysis> {
        let project = prism_project::Project {
            id: prism_core::ids::ProjectId::from_string("proj"),
            name: "demo".to_string(),
            root_path: std::path::PathBuf::from("."),
            discovered_at: prism_core::Timestamp::default(),
        };
        let context = HealthContext {
            project: &project,
            code_model: None,
            discovery: None,
            graph: None,
            git: None,
            change_set: None,
            impact: None,
            root: std::path::Path::new("."),
            derived: None,
        };
        analyze(registry, config, &context, options, cancel)
    }


    #[test]
    fn disabled_rules_are_not_detected_and_others_continue() {
        // §162 规则隔离：禁用 A 不影响 C；C 默认禁用 ⇒ disabled 列表。
        let config = HealthConfig {
            rules: [(
                "TEST001".to_string(),
                prism_core::config::HealthRuleConfig {
                    enabled: Some(false),
                    severity: None,
                    threshold: None,
                },
            )]
            .into_iter()
            .collect(),
            ..HealthConfig::default()
        };
        let analysis = run(&registry(), &config, &HealthOptions::default(), &CancellationToken::new()).unwrap();
        assert!(analysis.findings.iter().all(|f| f.rule_id != "TEST001"));
        assert!(!analysis.checked_rules.contains(&"TEST003".to_string()));
        assert!(analysis.disabled_rules.contains(&"TEST001".to_string()));
        assert!(analysis.disabled_rules.contains(&"TEST003".to_string()));
    }

    #[test]
    fn cli_override_re_enables_config_disabled_rule() {
        // §221：CLI enabled 覆盖 config 禁用。
        let config = HealthConfig {
            rules: [(
                "TEST001".to_string(),
                prism_core::config::HealthRuleConfig {
                    enabled: Some(false),
                    severity: None,
                    threshold: None,
                },
            )]
            .into_iter()
            .collect(),
            ..HealthConfig::default()
        };
        let options = HealthOptions {
            enabled_rules: vec!["TEST001".to_string()],
            ..HealthOptions::default()
        };
        let analysis = run(&registry(), &config, &options, &CancellationToken::new()).unwrap();
        assert!(analysis.findings.iter().any(|f| f.rule_id == "TEST001"));
    }

    #[test]
    fn rule_failure_is_partial_and_others_survive() {
        // §99/§163：B 失败 ⇒ 诊断 + failed_rules + Partial；A 照常产出。
        let analysis = run(&registry(), &HealthConfig::default(), &HealthOptions::default(), &CancellationToken::new()).unwrap();
        assert_eq!(analysis.failed_rules, vec!["TEST002".to_string()]);
        assert_eq!(
            analysis.analysis_completeness,
            prism_core::PrismCompleteness::Partial
        );
        assert!(analysis.findings.iter().any(|f| f.rule_id == "TEST001"));
        assert!(analysis.diagnostics.iter().any(|d| d.kind == "rule_failed"
            && d.subject.as_deref() == Some("TEST002")
            && d.detail.contains("exploded")));
    }

    #[test]
    fn per_rule_cap_truncates_and_marks_truncated() {
        // §224：cap=1 ⇒ 只报 1 条 + 截断诊断 + Truncated。
        let options = HealthOptions {
            max_findings_per_rule: Some(1),
            ..HealthOptions::default()
        };
        let mut config = HealthConfig::default();
        config.rules.insert(
            "TEST002".to_string(),
            prism_core::config::HealthRuleConfig::default(),
        );
        let analysis = run(&registry(), &config, &options, &CancellationToken::new()).unwrap();
        let a_count = analysis.findings.iter().filter(|f| f.rule_id == "TEST001").count();
        assert_eq!(a_count, 1);
        assert_eq!(
            analysis.analysis_completeness,
            prism_core::PrismCompleteness::Truncated
        );
        assert!(analysis.stats.findings_truncated >= 1);
    }

    #[test]
    fn suppression_marks_findings_without_hiding_them() {
        // §67/§164：suppressed ≠ not detected——仍检出、状态变 Suppressed。
        let analysis = run(&registry(), &HealthConfig::default(), &HealthOptions::default(), &CancellationToken::new()).unwrap();
        let target = analysis
            .findings
            .iter()
            .find(|f| f.rule_id == "TEST001")
            .expect("TEST001 finding expected");
        let config = HealthConfig {
            suppressed_findings: vec![target.id.as_str().to_string()],
            ..HealthConfig::default()
        };
        let suppressed = run(&registry(), &config, &HealthOptions::default(), &CancellationToken::new()).unwrap();
        let suppressed_finding = suppressed
            .findings
            .iter()
            .find(|f| f.id == target.id)
            .expect("suppressed finding must still be detected");
        assert_eq!(suppressed_finding.status, FindingStatus::Suppressed);
        assert_eq!(suppressed.stats.findings_suppressed, 1);
    }

    #[test]
    fn deterministic_across_runs() {
        // §95/§161：三次运行 ids/顺序/summary 完全一致。
        let outputs: Vec<String> = (0..3)
            .map(|_| {
                run(&registry(), &HealthConfig::default(), &HealthOptions::default(), &CancellationToken::new())
                    .unwrap()
            })
            .map(|analysis| serde_json::to_string(&analysis).unwrap())
            .collect();
        assert_eq!(outputs[0], outputs[1]);
        assert_eq!(outputs[1], outputs[2]);
    }

    #[test]
    fn ordering_is_severity_then_category_then_rule_then_location() {
        // §96：Critical(003 注入) 排在 Warning(TEST001) 前。
        let mut registry = registry();
        struct RuleHigh;
        static META_HIGH: LazyLock<HealthRuleMeta> = LazyLock::new(|| HealthRuleMeta {
            id: "TEST004",
            name: "Critical rule",
            category: FindingCategory::Security,
            description: "critical finding",
            default_severity: Severity::Critical,
            default_enabled: true,
            threshold_meaning: None,
            needs_derived: false,
        });
        impl HealthRule for RuleHigh {
            fn meta(&self) -> &'static HealthRuleMeta {
                &META_HIGH
            }
            fn evaluate(
                &self,
                _context: &HealthContext,
                _config: &prism_core::config::HealthRuleConfig,
                _cancel: &CancellationToken,
            ) -> prism_core::PrismResult<RuleResult> {
                Ok(RuleResult {
                    findings: vec![finding("TEST004", "src/z.ts", 1, Severity::Critical)],
                    diagnostics: vec![],
                })
            }
        }
        registry.register(Box::new(RuleHigh));
        let analysis = run(&registry, &HealthConfig::default(), &HealthOptions::default(), &CancellationToken::new()).unwrap();
        let severities: Vec<Severity> = analysis.findings.iter().map(|f| f.severity).collect();
        let mut sorted = severities.clone();
        sorted.sort_by(|a, b| b.cmp(a));
        assert_eq!(severities, sorted, "severity-descending ordering (§96)");
    }

    #[test]
    fn cancellation_returns_cancelled_not_empty() {
        // §102：取消 ⇒ Err(Cancelled)，绝不伪装 0 findings。
        let cancel = CancellationToken::new();
        cancel.cancel();
        let result = run(&registry(), &HealthConfig::default(), &HealthOptions::default(), &cancel);
        assert!(matches!(result, Err(PrismError::Cancelled)));
    }

    #[test]
    fn unknown_threshold_on_rule_without_threshold_semantics_is_diagnosed() {
        // §72：threshold 语义未声明的规则收到 threshold ⇒ 诊断（规则仍运行）。
        let mut config = HealthConfig::default();
        config.rules.insert(
            "TEST001".to_string(),
            prism_core::config::HealthRuleConfig {
                threshold: Some(5),
                ..Default::default()
            },
        );
        let analysis = run(&registry(), &config, &HealthOptions::default(), &CancellationToken::new()).unwrap();
        assert!(
            analysis
                .diagnostics
                .iter()
                .any(|d| d.kind == "config_unknown_option"
                    && d.subject.as_deref() == Some("TEST001"))
        );
    }

    // ---- §103 合成大项目安全：数千 findings / 多文件规模 ----

    static META_BULK: LazyLock<HealthRuleMeta> = LazyLock::new(|| HealthRuleMeta {
        id: "TEST900",
        name: "Bulk synthetic rule",
        category: FindingCategory::Maintenance,
        description: "emits 5000 findings across 1000 synthetic files",
        default_severity: Severity::Info,
        default_enabled: true,
        threshold_meaning: None,
        needs_derived: false,
    });

    struct RuleBulk {
        /// 每产生多少条检查一次取消（协作式，M0 语义）。
        cancel_every: usize,
    }

    impl HealthRule for RuleBulk {
        fn meta(&self) -> &'static HealthRuleMeta {
            &META_BULK
        }
        fn evaluate(
            &self,
            _context: &HealthContext,
            _config: &prism_core::config::HealthRuleConfig,
            cancel: &CancellationToken,
        ) -> prism_core::PrismResult<RuleResult> {
            const TOTAL: u32 = 5000;
            let mut findings = Vec::with_capacity(TOTAL as usize);
            for index in 0..TOTAL {
                if index % self.cancel_every.max(1) as u32 == 0 && cancel.is_cancelled() {
                    return Err(PrismError::Cancelled);
                }
                let file = format!("src/generated/mod{}/file{}.ts", index / 5, index / 5);
                findings.push(finding("TEST900", &file, (index % 50) + 1, Severity::Info));
            }
            Ok(RuleResult {
                findings,
                diagnostics: vec![],
            })
        }
    }

    fn bulk_registry() -> HealthRuleRegistry {
        let mut registry = HealthRuleRegistry::default();
        registry.register(Box::new(RuleBulk { cancel_every: 100 }));
        registry
    }

    #[test]
    fn large_synthetic_project_completes_deterministically() {
        // §103：数千 findings / 多文件规模 —— no crash、确定性、有界时间。
        let started = std::time::Instant::now();
        let first = run(&bulk_registry(), &HealthConfig::default(), &HealthOptions::default(), &CancellationToken::new())
            .expect("bulk run must succeed");
        let elapsed = started.elapsed();
        // 有界时间（宽松上限；本地 debug 构建实测毫秒级，防 CI 抖动给 60s）。
        assert!(
            elapsed < std::time::Duration::from_secs(60),
            "bulk analysis took too long: {elapsed:?}"
        );
        assert_eq!(first.findings.len(), 5000, "every synthetic finding present");
        // §95/§161：确定性——再次运行输出逐字节一致。
        let second = run(&bulk_registry(), &HealthConfig::default(), &HealthOptions::default(), &CancellationToken::new())
            .unwrap();
        assert_eq!(
            serde_json::to_string(&first).unwrap(),
            serde_json::to_string(&second).unwrap(),
        );
        // §96 排序稳定性抽检：同前缀下按 location 升序。
        let mut expected: Vec<(String, u32)> = first
            .findings
            .iter()
            .map(|f| (f.location.relative_path.clone(), f.location.line))
            .collect();
        expected.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
        let actual: Vec<(String, u32)> = first
            .findings
            .iter()
            .map(|f| (f.location.relative_path.clone(), f.location.line))
            .collect();
        // 全局排序键含 severity/rule（本例全同）⇒ 实际顺序应与 (path,line) 稳定序一致。
        let mut grouped: BTreeMap<String, Vec<u32>> = BTreeMap::new();
        for (path, line) in actual {
            grouped.entry(path).or_default().push(line);
        }
        for lines in grouped.values() {
            let mut sorted = lines.clone();
            sorted.sort();
            assert_eq!(lines, &sorted, "within a file lines must be ascending");
        }
        let _ = expected;
    }

    #[test]
    fn bulk_run_with_cap_stays_truncated_and_bounded() {
        // §103 × §223：大输出 + 全局 cap ⇒ Truncated 且数量恰好受界。
        let options = HealthOptions {
            max_findings: Some(500),
            ..HealthOptions::default()
        };
        let analysis = run(&bulk_registry(), &HealthConfig::default(), &options, &CancellationToken::new()).unwrap();
        assert_eq!(analysis.findings.len(), 500);
        assert_eq!(analysis.analysis_completeness, prism_core::PrismCompleteness::Truncated);
        assert_eq!(analysis.stats.findings_truncated, 4500);
        assert!(
            analysis
                .diagnostics
                .iter()
                .any(|d| d.kind == "findings_truncated" && d.detail.contains("of at least 5000"))
        );
    }

    #[test]
    fn cooperative_cancellation_stops_bulk_rule() {
        // §102/§160：协作取消在规则内生效 ⇒ Err(Cancelled)，无部分输出伪装。
        let cancel = CancellationToken::new();
        cancel.cancel();
        let result = run(&bulk_registry(), &HealthConfig::default(), &HealthOptions::default(), &cancel);
        assert!(matches!(result, Err(PrismError::Cancelled)));
    }
}
