//! 域数据装配（规格 M11b §14/§26/§124：把 M1–M9 分析结果规范化为
//! 可比较的域 JSON）。CLI 与 GUI 共用（§475）；compare/timeline 直接
//! 消费这些 JSON（§113 持久化事实直比，不重跑分析器）。

use prism_changes::ChangeSet;
use prism_graph::DependencyGraphResult;
use prism_health::HealthAnalysis;
use prism_impact::ImpactAnalysis;
use prism_project::ProjectDiscovery;
use prism_verify::VerificationReport;
use serde_json::{json, Value};
use std::path::Path;
use std::sync::Arc;

/// M1–M9 既有分析结果（§263：全部由调用方注入，不重算）。
pub struct DomainInputs<'a> {
    pub discovery: Option<&'a ProjectDiscovery>,
    pub code_model: Option<&'a prism_analysis::ProjectCodeModel>,
    pub graph: Option<&'a DependencyGraphResult>,
    pub changeset: Option<&'a ChangeSet>,
    pub impact: Option<&'a ImpactAnalysis>,
    pub health: Option<&'a HealthAnalysis>,
    pub verification: Option<&'a VerificationReport>,
    pub environment: Option<&'a prism_env::EnvironmentSnapshot>,
}

/// `collect_analysis` 产出的所有权束（§475：CLI 与 GUI 共用同一条
/// 管线，各自不再拼装）。
pub struct AnalysisBundle {
    pub discovery: ProjectDiscovery,
    pub code_model: prism_analysis::ProjectCodeModel,
    pub graph: DependencyGraphResult,
    pub change_set: Option<ChangeSet>,
    pub impact: Option<ImpactAnalysis>,
    pub health: Option<HealthAnalysis>,
    pub verification: Option<VerificationReport>,
    pub environment: Option<prism_env::EnvironmentSnapshot>,
    /// 被跳过的辅助域与原因（§88 诚实记录，不伪造空域）。
    pub warnings: Vec<String>,
}

impl AnalysisBundle {
    pub fn inputs(&self) -> DomainInputs<'_> {
        DomainInputs {
            discovery: Some(&self.discovery),
            code_model: Some(&self.code_model),
            graph: Some(&self.graph),
            changeset: self.change_set.as_ref(),
            impact: self.impact.as_ref(),
            health: self.health.as_ref(),
            verification: self.verification.as_ref(),
            environment: self.environment.as_ref(),
        }
    }
}

/// 跑一次 M1–M9 管线并收集全部域事实。M1–M3（discovery/code/graph）
/// 失败 ⇒ Err（capture 无从谈起）；辅助域（change/impact/health/
/// verification/environment）失败 ⇒ 记入 warnings 并跳过，capture 继续。
/// `refresh_verification=false` 时不跑 M8（Manual Snapshot First §146/§147：
/// verify 会真实执行测试命令，仅在显式要求时刷新）。
pub fn collect_analysis(
    path: &Path,
    refresh_verification: bool,
) -> Result<AnalysisBundle, prism_core::PrismError> {
    let cancel = prism_core::task::CancellationToken::new();
    let runner: Arc<dyn prism_core::runtime::command::CommandRunner> =
        Arc::new(prism_core::runtime::command::StdCommandRunner::new());
    let mut warnings = Vec::new();

    let config_path = path.join(prism_core::config::PrismConfig::config_file_name());
    let (config, _loaded) = prism_core::config::PrismConfig::load(&config_path)?;

    let discovery = prism_project::DiscoveryService::new().discover(
        path,
        prism_project::ScanOptions::default(),
        &cancel,
        |_| {},
    )?;
    let code_model = prism_analysis::CodeAnalysisService::new().analyze(
        &discovery,
        prism_analysis::CodeAnalysisOptions::default(),
        &cancel,
        |_| {},
    )?;
    let graph = prism_graph::DependencyService::new().analyze(&discovery, &code_model)?;

    // 非 git 目录 ⇒ None（change/impact 域缺席，§88 不伪造）。
    let change_set = match prism_changes::ChangeSetService::new(runner.clone()).build(
        path,
        prism_changes::ChangeScope::WorkingTree,
        None,
        None,
        &prism_changes::ChangeSetOptions::default(),
        &cancel,
    ) {
        Ok(change_set) => change_set,
        Err(err) => {
            warnings.push(format!("change domain skipped: {err}"));
            None
        }
    };
    let impact = change_set.as_ref().and_then(|changeset| {
        prism_impact::ImpactAnalysisService
            .analyze(
                changeset,
                &code_model,
                &graph,
                &prism_impact::ImpactOptions::default(),
                &cancel,
            )
            .map_err(|err| warnings.push(format!("impact domain skipped: {err}")))
            .ok()
    });

    let health = match analyze_health(
        path,
        &config,
        &discovery,
        &code_model,
        &graph,
        &change_set,
        &impact,
        &cancel,
    ) {
        Ok(analysis) => Some(analysis),
        Err(err) => {
            warnings.push(format!("health domain skipped: {err}"));
            None
        }
    };

    let verification = if refresh_verification {
        let git = prism_git::GitService::new(runner.clone());
        let options = prism_verify::VerifyOptions {
            selected: None,
            dry_run: false,
            fail_on_unavailable: false,
        };
        match prism_verify::run_verification(
            &prism_verify::discover_checks(path, &config.verify),
            path,
            path.file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_else(|| "project".to_string())
                .as_str(),
            &options,
            runner.clone(),
            2,
            true,
            Some(&git),
            &[],
            &cancel,
        ) {
            Ok(report) => Some(report),
            Err(err) => {
                warnings.push(format!("verification domain skipped: {err}"));
                None
            }
        }
    } else {
        None
    };

    let environment = match prism_env::EnvironmentService::scan(
        path,
        Some(&discovery),
        &prism_env::EnvironmentScanOptions::default(),
        runner,
        &cancel,
    ) {
        Ok(snapshot) => Some(snapshot),
        Err(err) => {
            warnings.push(format!("environment domain skipped: {err}"));
            None
        }
    };

    Ok(AnalysisBundle {
        discovery,
        code_model,
        graph,
        change_set,
        impact,
        health,
        verification,
        environment,
        warnings,
    })
}

#[allow(clippy::too_many_arguments)]
fn analyze_health(
    path: &Path,
    config: &prism_core::config::PrismConfig,
    discovery: &ProjectDiscovery,
    code_model: &prism_analysis::ProjectCodeModel,
    graph: &DependencyGraphResult,
    change_set: &Option<ChangeSet>,
    impact: &Option<ImpactAnalysis>,
    cancel: &prism_core::task::CancellationToken,
) -> Result<HealthAnalysis, prism_core::PrismError> {
    let context = prism_health::HealthContext {
        project: &discovery.project,
        code_model: Some(code_model),
        discovery: Some(discovery),
        graph: Some(graph),
        git: None,
        change_set: change_set.as_ref(),
        impact: impact.as_ref(),
        root: path,
        derived: None,
        health_config: &config.health,
    };
    let options = prism_health::HealthOptions {
        scope: prism_health::HealthScope::FullProject,
        ..prism_health::HealthOptions::default()
    };
    prism_health::analyze(
        &prism_health::HealthRuleRegistry::with_builtins(),
        &config.health,
        &context,
        &options,
        cancel,
    )
}

/// 组装十个域的规范化 JSON（key = 域名；结构域由 capture 自身负责，
/// 不在此列）。数组一律字符串化以便 compare 的集合 diff（§158）。
pub fn build_domain_data(inputs: &DomainInputs) -> std::collections::BTreeMap<String, Value> {
    let mut data: std::collections::BTreeMap<String, Value> = Default::default();

    // ---- code（M2）----
    if let Some(model) = inputs.code_model {
        let symbols: Vec<String> = model
            .parse_results
            .iter()
            .flat_map(|result| {
                let file = result.relative_path.clone();
                result
                    .symbols
                    .iter()
                    .map(move |symbol| format!("{}#{}", file, symbol.name))
            })
            .take(500)
            .collect();
        data.insert(
            "code".to_string(),
            json!({
                "model_version": model.schema_version,
                "symbols_total": model.summary.symbols_total,
                "symbols": symbols,
            }),
        );
    }

    // ---- architecture（M3）----
    if let Some(graph) = inputs.graph {
        let edges: Vec<String> = graph
            .dependencies
            .iter()
            .take(500)
            .map(|dependency| {
                // resolved 目标优先，未解析时退回原样 specifier（§88 记录事实）。
                let target = dependency
                    .target_path
                    .clone()
                    .unwrap_or_else(|| dependency.raw_target.clone());
                format!("{}->{}", dependency.source_path, target)
            })
            .collect();
        let cycles: Vec<String> = graph.cycles.iter().map(|cycle| cycle.id.clone()).collect();
        data.insert(
            "architecture".to_string(),
            json!({
                "modules_total": graph.modules.len(),
                "dependencies_total": graph.dependencies.len(),
                "cycles": cycles,
                "edges": edges,
            }),
        );
    }

    // ---- change（M5）----
    if let Some(changeset) = inputs.changeset {
        let files: Vec<String> = changeset
            .files
            .iter()
            .filter_map(|file| file.new_path.clone().or(file.old_path.clone()))
            .take(500)
            .collect();
        let symbols: Vec<String> = changeset
            .symbols
            .iter()
            .map(|symbol| symbol.name.clone())
            .take(500)
            .collect();
        data.insert(
            "change".to_string(),
            json!({ "files": files, "symbols": symbols }),
        );
    }

    // ---- impact（M6）----
    if let Some(impact) = inputs.impact {
        let impacted: Vec<String> = impact
            .impacted_items
            .iter()
            .filter(|item| !item.path.is_empty())
            .take(500)
            .map(|item| item.path.clone())
            .collect();
        let test_candidates: Vec<String> = impact
            .impacted_items
            .iter()
            .filter(|item| {
                item.impact_kinds
                    .contains(&prism_impact::ImpactKind::TestCandidate)
                    && !item.path.is_empty()
            })
            .take(200)
            .map(|item| item.path.clone())
            .collect();
        data.insert(
            "impact".to_string(),
            json!({ "impacted": impacted, "test_candidates": test_candidates }),
        );
    }

    // ---- health（M7）：findings 含稳定指纹（§37 compare 生命周期）----
    if let Some(health) = inputs.health {
        let findings: Vec<Value> = health
            .findings
            .iter()
            .map(|finding| {
                // §37：指纹只用 rule+subject(标题)+归一化路径，不含行号——
                // 同一 finding 在行内移动不产生假生命周期。
                let path_text = finding.location.relative_path.clone();
                json!({
                    "rule_id": finding.rule_id,
                    "subject": finding.title,
                    "path": path_text,
                    "severity": format!("{:?}", finding.severity),
                    "finding_fingerprint": crate::compare::finding_fingerprint(
                        &finding.rule_id,
                        &finding.title,
                        &path_text,
                    ),
                })
            })
            .collect();
        data.insert(
            "health".to_string(),
            json!({
                "model_version": health.schema_version,
                "findings": findings,
            }),
        );
    }

    // ---- verification（M8）：check 状态迁移的事实来源 ----
    if let Some(report) = inputs.verification {
        let checks: Vec<Value> = report
            .runs
            .iter()
            .map(|run| {
                json!({
                    "name": run.check_id,
                    "status": format!("{:?}", run.state).to_lowercase(),
                })
            })
            .collect();
        data.insert(
            "verification".to_string(),
            json!({
                "model_version": report.schema_version,
                "checks": checks,
            }),
        );
    }

    // ---- environment（M9）：工具/需求匹配/findings ----
    if let Some(env) = inputs.environment {
        let tools: Vec<String> = env
            .tools
            .iter()
            .map(|tool| {
                format!(
                    "{} {}",
                    tool.name,
                    tool.raw_version.as_deref().unwrap_or("unavailable")
                )
            })
            .collect();
        let requirements: Vec<String> = env
            .matches
            .iter()
            .map(|matched| format!("{}: {}", matched.subject, matched.result.as_str()))
            .collect();
        let env_findings: Vec<String> = env
            .findings
            .iter()
            .map(|finding| format!("{}: {}", finding.id, finding.message))
            .collect();
        data.insert(
            "environment".to_string(),
            json!({
                "tools": tools,
                "requirements": requirements,
                "findings": env_findings,
            }),
        );
    }

    data
}
