//! `prism health rules` — 规则注册表检视（规格 M7 §74/§129/§220）。
//!
//! 输出每条规则的 id/name/category 与**合并配置后的最终开关状态**
//! （precedence：built-in defaults → prism.config.json `health` 节；
//! CLI 覆盖属于完整 `prism health .` 命令，M7b 提供）。

use prism_analysis::{CodeAnalysisOptions, CodeAnalysisService};
use prism_changes::{ChangeScope, ChangeSetOptions, ChangeSetService};
use prism_core::config::PrismConfig;
use prism_core::runtime::command::{CommandRunner, StdCommandRunner};
use prism_core::task::CancellationToken;
use prism_graph::DependencyService;
use prism_health::{
    render_html, render_markdown, render_text, HealthOptions, HealthRuleRegistry, HealthScope,
};
use std::path::Path;
use std::sync::Arc;

pub fn run_rules(path: &Path, json: bool) -> i32 {
    if !path.exists() {
        eprintln!("prism: path does not exist: {}", path.to_string_lossy());
        return crate::EXIT_INVALID_INPUT;
    }

    // 项目配置（不存在 ⇒ 默认配置；§70 单一配置系统）。
    let config_path = path.join(PrismConfig::config_file_name());
    let (config, _loaded) = PrismConfig::load(&config_path).unwrap_or_else(|err| {
        eprintln!("prism: config invalid: {err}");
        std::process::exit(crate::EXIT_ERROR);
    });

    let registry = HealthRuleRegistry::with_builtins();
    let rows: Vec<_> = registry
        .rules()
        .map(|rule| {
            let meta = rule.meta();
            (
                meta.id.to_string(),
                meta.name.to_string(),
                meta.category,
                config
                    .health
                    .is_rule_enabled(meta.id, meta.default_enabled)
                    .unwrap_or(meta.default_enabled),
            )
        })
        .collect();

    if json {
        let value: Vec<serde_json::Value> = rows
            .iter()
            .map(|(id, name, category, enabled)| {
                serde_json::json!({
                    "id": id,
                    "name": name,
                    "category": format!("{category:?}").to_ascii_lowercase(),
                    "enabled": enabled,
                })
            })
            .collect();
        match serde_json::to_string_pretty(&value) {
            Ok(text) => println!("{text}"),
            Err(err) => {
                eprintln!("prism: failed to serialize rules: {err}");
                return crate::EXIT_ERROR;
            }
        }
    } else {
        for (id, name, category, enabled) in &rows {
            let state = if *enabled { "enabled" } else { "disabled" };
            println!("{id:<9} {name:<28} {category:?}  {state}");
        }
    }
    crate::EXIT_OK
}

/// `--fail-on` 词表（§79/§80：默认 findings 不导致失败；显式 opt-in 才非零）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum FailOn {
    Critical,
    Warning,
}

/// `prism health <path>`（无子命令）——完整健康分析（M7b §75-§80）。
#[allow(clippy::too_many_arguments)] // CLI 参数直传
pub fn run_analyze(
    path: &Path,
    format: crate::cli::HealthFormatArg,
    changed: bool,
    rule: Option<&str>,
    category: Option<&str>,
    fail_on: Option<FailOn>,
    max_files: usize,
) -> i32 {
    if !path.exists() {
        eprintln!("prism: path does not exist: {}", path.to_string_lossy());
        return crate::EXIT_INVALID_INPUT;
    }
    let runner: Arc<dyn CommandRunner> = Arc::new(StdCommandRunner::new());
    let cancel = CancellationToken::new();

    let discovery = match prism_project::DiscoveryService::new().discover(
        path,
        prism_project::ScanOptions::default(),
        &cancel,
        |_| {},
    ) {
        Ok(discovery) => discovery,
        Err(err) => {
            eprintln!("prism: health failed building discovery: {err}");
            return crate::EXIT_ERROR;
        }
    };
    let model = match CodeAnalysisService::new().analyze(
        &discovery,
        CodeAnalysisOptions::default(),
        &cancel,
        |_| {},
    ) {
        Ok(model) => model,
        Err(err) => {
            eprintln!("prism: health failed building code model: {err}");
            return crate::EXIT_ERROR;
        }
    };
    let graph = match DependencyService::new().analyze(&discovery, &model) {
        Ok(graph) => graph,
        Err(err) => {
            eprintln!("prism: health failed building graph: {err}");
            return crate::EXIT_ERROR;
        }
    };

    let (change_set, impact) = if changed {
        let changeset = ChangeSetService::new(runner.clone())
            .build(
                path,
                ChangeScope::WorkingTree,
                None,
                None,
                &ChangeSetOptions {
                    max_files,
                    ..ChangeSetOptions::default()
                },
                &cancel,
            )
            .ok()
            .flatten();
        let impact = changeset.as_ref().and_then(|changeset| {
            prism_impact::ImpactAnalysisService
                .analyze(
                    changeset,
                    &model,
                    &graph,
                    &prism_impact::ImpactOptions::default(),
                    &cancel,
                )
                .ok()
        });
        (changeset, impact)
    } else {
        (None, None)
    };

    let project_config_path = path.join(PrismConfig::config_file_name());
    let (project_config, _loaded) = match PrismConfig::load(&project_config_path) {
        Ok(loaded) => loaded,
        Err(err) => {
            eprintln!("prism: config invalid: {err}");
            return crate::EXIT_ERROR;
        }
    };

    let scope = if changed {
        HealthScope::ImpactedItems
    } else {
        HealthScope::FullProject
    };
    let options = HealthOptions {
        scope,
        ..HealthOptions::default()
    };
    let context = prism_health::HealthContext {
        project: &discovery.project,
        code_model: Some(&model),
        discovery: Some(&discovery),
        graph: Some(&graph),
        git: None,
        change_set: change_set.as_ref(),
        impact: impact.as_ref(),
        root: path,
        derived: None,
        health_config: &project_config.health,
    };
    match prism_health::analyze(
        &HealthRuleRegistry::with_builtins(),
        &project_config.health,
        &context,
        &options,
        &cancel,
    ) {
        Ok(mut analysis) => {
            if let Some(rule_id) = rule {
                analysis
                    .findings
                    .retain(|finding| finding.rule_id == rule_id);
            }
            if let Some(category) = category {
                analysis.findings.retain(|finding| {
                    format!("{:?}", finding.category).eq_ignore_ascii_case(category)
                });
            }
            // 过滤后 summary 重算（§10 计数保持真实）。
            analysis.summary = prism_health::HealthSummary::from_findings(analysis.findings.iter());
            match format {
                crate::cli::HealthFormatArg::Json => {
                    match serde_json::to_string_pretty(&analysis) {
                        Ok(text) => println!("{text}"),
                        Err(err) => {
                            eprintln!("prism: failed to serialize health analysis: {err}");
                            return crate::EXIT_ERROR;
                        }
                    }
                }
                crate::cli::HealthFormatArg::Text => print!("{}", render_text(&analysis)),
                crate::cli::HealthFormatArg::Markdown => {
                    print!("{}", render_markdown(&analysis))
                }
                crate::cli::HealthFormatArg::Html => print!("{}", render_html(&analysis)),
            }
            match fail_on {
                None => crate::EXIT_OK,
                Some(FailOn::Critical) if analysis.summary.critical_count > 0 => {
                    eprintln!(
                        "prism: --fail-on critical: {} critical finding(s)",
                        analysis.summary.critical_count
                    );
                    crate::EXIT_ERROR
                }
                Some(FailOn::Warning)
                    if analysis.summary.critical_count > 0
                        || analysis.summary.warning_count > 0 =>
                {
                    eprintln!(
                        "prism: --fail-on warning: {} critical / {} warning finding(s)",
                        analysis.summary.critical_count, analysis.summary.warning_count
                    );
                    crate::EXIT_ERROR
                }
                Some(_) => crate::EXIT_OK,
            }
        }
        Err(prism_core::PrismError::Cancelled) => {
            eprintln!("prism: health cancelled");
            crate::EXIT_CANCELLED
        }
        Err(err) => {
            eprintln!("prism: health failed: {err}");
            crate::EXIT_ERROR
        }
    }
}
