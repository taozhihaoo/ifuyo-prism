//! `prism debug`（2.0-M210 §五十/§六十二）：Debug Intelligence CLI 只读
//! 入口 —— Vertical Slice。
//!
//! 链路：日志源（自动发现 *.log 与轮转片段 / `--source` 显式指定）→
//! prism-debug 解析（redaction 内建）→ 指纹/聚类 → M23 Symbol + M24
//! Scene/Node 帧解析 → M28 RuleRegistry（DEBUG 规则 + 全量内建）⇒
//! Finding + Metric + 生命周期（`--baseline`）+ Quality Gate。
//! 无日志源 ⇒ 如实报告 Unavailable（**不是"没有错误"**，§二.7）。

use prism_core::config::PrismConfig;
use prism_core::task::CancellationToken;
use prism_csharp::{SemanticEngine, SymbolEngine};
use prism_debug::DebugAnalysisInput;
use prism_health2::{
    diff_against_baseline, evaluate_gate, mark_regressed, Health2Report, RuleContext, RuleRegistry,
    RuleThresholds,
};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// 单源有界读取上限（大日志分段引用，§五 原则 2）。
const SOURCE_MAX_BYTES: u64 = 8 * 1024 * 1024;
const MAX_SOURCES: usize = 20;
/// 全项目 .cs 有界读取（符号解析面，与 health2 一致）。
const CS_MAX_BYTES: u64 = 4 * 1024 * 1024;

pub fn run(path: &Path, sources: &[PathBuf], baseline: Option<&Path>, json: bool) -> i32 {
    if !path.exists() {
        eprintln!("prism: path does not exist: {}", path.to_string_lossy());
        return crate::EXIT_INVALID_INPUT;
    }
    let config_path = path.join(PrismConfig::config_file_name());
    let (_config, _loaded) = PrismConfig::load(&config_path).unwrap_or_else(|err| {
        eprintln!("prism: config invalid: {err}");
        std::process::exit(crate::EXIT_ERROR);
    });
    let cancel = CancellationToken::new();
    let discovery = match prism_project::DiscoveryService::new().discover(
        path,
        prism_project::ScanOptions::default(),
        &cancel,
        |_| {},
    ) {
        Ok(discovery) => discovery,
        Err(err) => {
            eprintln!("prism: debug failed building discovery: {err}");
            return crate::EXIT_ERROR;
        }
    };

    // ---- 日志源（§六/§三十五：*.log + 轮转片段归并逻辑源；显式优先）----
    let mut log_paths: Vec<String> = Vec::new();
    if sources.is_empty() {
        for file in &discovery.files {
            let lower = file.relative_path.to_ascii_lowercase();
            let is_log = lower.ends_with(".log")
                || lower
                    .rsplit('.')
                    .next()
                    .map(|ext| ext.starts_with("log"))
                    .unwrap_or(false);
            if is_log && log_paths.len() < MAX_SOURCES {
                log_paths.push(file.relative_path.clone());
            }
        }
    } else {
        for source in sources {
            let relative = source
                .strip_prefix(path)
                .map(|rest| rest.to_string_lossy().replace('\\', "/"))
                .unwrap_or_else(|_| source.to_string_lossy().replace('\\', "/"));
            log_paths.push(relative);
        }
    }
    let mut source_contents: BTreeMap<String, String> = BTreeMap::new();
    let mut cli_diagnostics: Vec<String> = Vec::new();
    for relative in &log_paths {
        let full = path.join(relative);
        let size = std::fs::metadata(&full).map(|meta| meta.len()).unwrap_or(0);
        if size > SOURCE_MAX_BYTES {
            cli_diagnostics.push(format!(
                "skipped {relative}: {size} bytes > cap {SOURCE_MAX_BYTES} (bounded storage; reference kept in diagnostics)"
            ));
            continue;
        }
        match std::fs::read_to_string(&full) {
            Ok(content) => {
                source_contents.insert(relative.clone(), content);
            }
            Err(err) => cli_diagnostics.push(format!("read {relative} failed: {err}")),
        }
    }

    // ---- 解析 + 指纹 + 聚类（prism-debug；redaction 内建）----
    let mut report = prism_debug::analyze(&DebugAnalysisInput {
        sources: &source_contents,
        max_events: 50_000,
    });
    report.diagnostics.extend(cli_diagnostics);

    // ---- 符号解析引擎（M23 + M25；§十一 帧解析）----
    let mut cs_sources: BTreeMap<String, String> = BTreeMap::new();
    for file in &discovery.files {
        let is_cs = file
            .relative_path
            .rsplit('/')
            .next()
            .map(|name| name.ends_with(".cs"))
            .unwrap_or(false);
        if !is_cs || file.size_bytes > CS_MAX_BYTES {
            continue;
        }
        if let Ok(content) = std::fs::read_to_string(path.join(&file.relative_path)) {
            cs_sources.insert(file.relative_path.clone(), content);
        }
    }
    let mut symbol_engine = SymbolEngine::new();
    for (cs_path, source) in &cs_sources {
        symbol_engine.update_file(cs_path, source);
    }
    let symbol_index = symbol_engine.build_index();
    let mut semantic_engine = SemanticEngine::new();
    for (cs_path, source) in &cs_sources {
        semantic_engine.update_source(&symbol_index, cs_path, source);
    }
    semantic_engine.rebuild(&symbol_index);

    // ---- Godot 事实（M24；场景/节点映射 §十二）----
    let godot_files: Vec<(String, String)> = discovery
        .files
        .iter()
        .filter(|file| {
            file.relative_path == "project.godot"
                || file.relative_path.ends_with(".tscn")
                || file.relative_path.ends_with(".tres")
        })
        .map(|file| (file.relative_path.clone(), String::new()))
        .collect();
    let godot_analysis = if godot_files.is_empty() {
        None
    } else {
        match prism_project::Project::from_root(path) {
            Ok(project) => {
                let context = prism_intel::AnalysisContext::new(
                    prism_intel::ProjectIdentity::from_project(&project),
                    prism_intel::AnalysisScope::Domain("godot".to_string()),
                    vec!["godot".to_string()],
                    cancel.clone(),
                );
                let outcome = prism_godot::analyze(&godot_files, &project.root_path, &context);
                for warning in &outcome.warnings {
                    report.diagnostics.push(format!("godot: {warning}"));
                }
                outcome.data
            }
            Err(err) => {
                eprintln!("prism: debug failed opening project: {err}");
                return crate::EXIT_ERROR;
            }
        }
    };
    let godot_index = godot_analysis.as_ref().map(prism_godot::GodotIndex::build);

    // ---- 帧解析 → Symbol/Scene（§三十一 Pipeline 尾段）----
    let project_root = prism_project::Project::from_root(path)
        .map(|project| project.root_path.clone())
        .unwrap_or_else(|_| path.to_path_buf());
    let report_root = project_root.clone();
    prism_debug::report::resolve_report_frames(
        &mut report,
        if cs_sources.is_empty() {
            None
        } else {
            Some(&symbol_index)
        },
        godot_index.as_ref(),
        &report_root,
    );

    // ---- M28 规则评估（§二十六：同一 Registry）----
    let registry = RuleRegistry::with_all_builtins();
    let observed_at = prism_core::Timestamp::now_utc().to_string();
    let thresholds = RuleThresholds::default();
    let rule_context = RuleContext {
        files: &[],
        thresholds: &thresholds,
        observed_at: &observed_at,
        csharp: None,
        godot: None,
        supply: None,
        debug: Some(&report),
        evolution: None,
        cancel: &cancel,
    };
    let health_report = registry.evaluate(&rule_context);

    // ---- 生命周期 + Gate（§二十一：复用 M28）----
    let empty_baseline = Health2Report::default();
    let mut baseline_new_ids: Vec<String> = Vec::new();
    let mut schema_mismatch = false;
    let baseline_report: Option<Health2Report> = baseline.map(|baseline_path| {
        let text = match std::fs::read_to_string(baseline_path) {
            Ok(text) => text,
            Err(err) => {
                eprintln!(
                    "prism: debug failed reading baseline {}: {err}",
                    baseline_path.to_string_lossy()
                );
                std::process::exit(crate::EXIT_INVALID_INPUT);
            }
        };
        let value: serde_json::Value = match serde_json::from_str(&text) {
            Ok(value) => value,
            Err(err) => {
                eprintln!("prism: debug baseline is not valid JSON: {err}");
                std::process::exit(crate::EXIT_INVALID_INPUT);
            }
        };
        if let Some(new_ids) = value
            .get("lifecycle")
            .and_then(|lifecycle| lifecycle.get("new"))
            .and_then(|ids| ids.as_array())
        {
            baseline_new_ids = new_ids
                .iter()
                .filter_map(|id| id.as_str().map(str::to_string))
                .collect();
        }
        match serde_json::from_value::<Health2Report>(value) {
            Ok(parsed) => {
                if parsed.schema_version != health_report.schema_version {
                    schema_mismatch = true;
                }
                parsed
            }
            Err(err) => {
                eprintln!("prism: debug baseline is not a valid Health2Report: {err}");
                std::process::exit(crate::EXIT_INVALID_INPUT);
            }
        }
    });
    let comparable = baseline_report.as_ref().filter(|_| !schema_mismatch);
    let diff = diff_against_baseline(&health_report, comparable.unwrap_or(&empty_baseline));
    let diff = if baseline_new_ids.is_empty() {
        diff
    } else {
        mark_regressed(diff, &baseline_new_ids, &health_report)
    };
    let gate = evaluate_gate(
        if schema_mismatch { None } else { Some(&diff) },
        0,
        &prism_health2::QualityGatePolicy::default(),
    );

    // ---- 输出（§五十 数值全部来自真实分析）----
    let unresolved_frames: usize = report
        .errors
        .iter()
        .filter_map(|error| error.stack_trace.as_ref())
        .flat_map(|trace| trace.frames.iter())
        .filter(|frame| frame.resolution.status == prism_debug::ResolutionStatus::Unresolved)
        .count();
    if json {
        let payload = serde_json::json!({
            "schema_version": prism_debug::DEBUG_SCHEMA_VERSION,
            "debug": report,
            "metrics": health_report.metrics,
            "findings": health_report.findings,
            "evidence": health_report.evidence,
            "diagnostics": health_report.diagnostics,
            "lifecycle": {
                "new": diff.new_findings,
                "persistent": diff.persistent_findings,
                "resolved": diff.resolved_findings,
                "regressed": diff.regressed_findings,
                "suppressed": diff.suppressed_findings,
            },
            "quality_gate": {
                "verdict": gate.verdict,
                "new_count": gate.new_count,
                "breaking_count": gate.breaking_count,
                "reasons": gate.reasons,
            },
        });
        return match serde_json::to_string_pretty(&payload) {
            Ok(text) => {
                println!("{text}");
                crate::EXIT_OK
            }
            Err(err) => {
                eprintln!("prism: failed to serialize debug report: {err}");
                crate::EXIT_ERROR
            }
        };
    }
    if report.sources.is_empty() {
        println!(
            "debug: no log sources found under {} — analysis Unavailable (this is NOT \"no errors\")",
            path.to_string_lossy()
        );
    }
    println!(
        "debug report (schema v{}, sources: {})",
        report.schema_version,
        report.sources.len()
    );
    for source in &report.sources {
        println!(
            "  source {} [{:?}] {} events, parser {}, {:?}{}",
            source.logical_path,
            source.source_type,
            report
                .events
                .iter()
                .filter(|event| event.source_id == source.source_id)
                .count(),
            source.parser,
            source.parse_state,
            if source.redactions > 0 {
                format!(", redacted {}", source.redactions)
            } else {
                String::new()
            }
        );
    }
    println!(
        "events: {} (total {}) | unique errors: {} | clusters: {} | unresolved frames: {}",
        report.events.len(),
        report.events_total,
        report.errors.len(),
        report.clusters.len(),
        unresolved_frames
    );
    println!("findings ({}):", health_report.findings.len());
    for finding in &health_report.findings {
        println!(
            "  {:<30} {:?}  {}",
            finding.finding_type, finding.severity, finding.title
        );
    }
    println!(
        "lifecycle: new={} persistent={} resolved={} regressed={} suppressed={}",
        diff.new_findings.len(),
        diff.persistent_findings.len(),
        diff.resolved_findings.len(),
        diff.regressed_findings.len(),
        diff.suppressed_findings.len()
    );
    println!(
        "quality gate: {:?} (new {} / breaking {})",
        gate.verdict, gate.new_count, gate.breaking_count
    );
    for reason in &gate.reasons {
        println!("  - {reason}");
    }
    for diagnostic in &health_report.diagnostics {
        println!("diag: {diagnostic}");
    }
    crate::EXIT_OK
}
