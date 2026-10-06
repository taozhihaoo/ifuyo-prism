//! `prism health2`（2.0-M28）：Health & Engineering Findings 2.0。
//!
//! 管线 = `prism_health2::analyze_project`（与 GUI 健康页 2.0 轨道**同一
//! 函数**，§71 一致性）：M1 Discovery 清单 → 注入既有引擎事实（L0 行数 /
//! M23-M25 C# / M24 Godot）→ RuleRegistry 内置规则评估 → 生命周期 →
//! Quality Gate。本文件只叠加 CLI 专属部分：`--baseline` 基线文件对比
//! （§26/§31/§33）与 json/文本输出。

use prism_core::config::PrismConfig;
use prism_core::task::CancellationToken;
use prism_health2::{diff_against_baseline, evaluate_gate, Health2Report};
use std::path::Path;

pub fn run(path: &Path, baseline: Option<&Path>, max_file_lines: u64, json: bool) -> i32 {
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
    let analyzed = match prism_health2::analyze_project(path, max_file_lines, &cancel) {
        Ok(analyzed) => analyzed,
        Err(err) => {
            eprintln!("prism: health2 failed: {err}");
            return crate::EXIT_ERROR;
        }
    };
    let report = analyzed.report;

    // ---- 生命周期（§26/§31/§33）：`--baseline` 缺省 ⇒ 共享管线已按空基
    // 线对比（首轮全 New）。基线文件 = 上一次 `prism health2 --json` 的完整
    // 输出：报告字段 + `lifecycle.new`（供 Regressed 判定）。schema 不兼容
    // ⇒ 对比置空（Gate = UNKNOWN，不伪造结论）。
    let mut baseline_new_ids: Vec<String> = Vec::new();
    let mut schema_mismatch = false;
    let baseline_report: Option<Health2Report> = baseline.map(|p| {
        let text = match std::fs::read_to_string(p) {
            Ok(text) => text,
            Err(err) => {
                eprintln!(
                    "prism: health2 failed reading baseline {}: {err}",
                    p.to_string_lossy()
                );
                std::process::exit(crate::EXIT_INVALID_INPUT);
            }
        };
        let value: serde_json::Value = match serde_json::from_str(&text) {
            Ok(value) => value,
            Err(err) => {
                eprintln!("prism: health2 baseline is not valid JSON: {err}");
                std::process::exit(crate::EXIT_INVALID_INPUT);
            }
        };
        if let Some(new_ids) = value
            .get("lifecycle")
            .and_then(|l| l.get("new"))
            .and_then(|v| v.as_array())
        {
            baseline_new_ids = new_ids
                .iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect();
        }
        let parsed: Health2Report = match serde_json::from_value(value) {
            Ok(parsed) => parsed,
            Err(err) => {
                eprintln!("prism: health2 baseline is not a valid Health2Report: {err}");
                std::process::exit(crate::EXIT_INVALID_INPUT);
            }
        };
        if parsed.schema_version != report.schema_version {
            eprintln!(
                "prism: health2 baseline schema v{} != current v{}; gate will be UNKNOWN",
                parsed.schema_version, report.schema_version
            );
            schema_mismatch = true;
        }
        parsed
    });
    let comparable = baseline_report.as_ref().filter(|_| !schema_mismatch);
    let mut diff = match comparable {
        Some(baseline) => diff_against_baseline(&report, baseline),
        None => analyzed.lifecycle,
    };
    // Regressed（§33）：基线轮中为 New、当前仍存在的指纹。
    if !baseline_new_ids.is_empty() {
        diff = prism_health2::mark_regressed(diff, &baseline_new_ids, &report);
    }

    // ---- Quality Gate（§29-§30：策略判断；默认 policy，breaking=0）----
    // 无基线 = 空基线对比（首轮全 New，可判定）；仅 schema 不兼容
    // 才 UNKNOWN（无对比依据，不伪造）。
    let gate = evaluate_gate(
        if schema_mismatch { None } else { Some(&diff) },
        0,
        &prism_health2::QualityGatePolicy::default(),
    );

    if json {
        let payload = serde_json::json!({
            "schema_version": report.schema_version,
            "metrics": report.metrics,
            "findings": report.findings,
            "evidence": report.evidence,
            "diagnostics": report.diagnostics,
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
        match serde_json::to_string_pretty(&payload) {
            Ok(text) => println!("{text}"),
            Err(err) => {
                eprintln!("prism: failed to serialize health2 report: {err}");
                return crate::EXIT_ERROR;
            }
        }
        return crate::EXIT_OK;
    }

    println!("health2 report (schema v{})", report.schema_version);
    println!("metrics:");
    for metric in report.metrics.values() {
        println!(
            "  {:<14} {:>8} {}  ({})",
            metric.key, metric.value, metric.unit, metric.definition
        );
    }
    println!("findings ({}):", report.findings.len());
    for finding in &report.findings {
        let target = finding
            .target
            .as_ref()
            .map(|t| t.as_str().to_string())
            .unwrap_or_default();
        println!(
            "  {:<8} {:<18} {:?}  {}",
            finding.finding_type, target, finding.severity, finding.title
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
    for diagnostic in &report.diagnostics {
        println!("diag: {diagnostic}");
    }
    crate::EXIT_OK
}
