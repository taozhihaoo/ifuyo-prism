//! `prism health2`（2.0-M28）：Health & Engineering Findings 2.0。
//!
//! 流程（§73 Vertical Slice）：M1 Discovery 清单 → 结构面规则（大文件，
//! 阈值可配置）→ Metric + Finding 报告 → 生命周期（`--baseline` 对比，
//! 缺省 ⇒ 首轮全 New）→ Quality Gate（默认 policy）。规则只消费既有
//! 清单事实，不重新扫描文件系统（§2）；无网络/shell。

use prism_core::config::PrismConfig;
use prism_core::task::CancellationToken;
use prism_health2::{
    analyze_inventory, diff_against_baseline, evaluate_gate, Health2Report, RuleThresholds,
    SourceFileInput,
};
use std::path::Path;

/// 行数统计的有界读取上限（与 M23/M25 上下文路径一致的 4 MiB 预算）。
const LINE_COUNT_MAX_BYTES: u64 = 4 * 1024 * 1024;

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
    let discovery = match prism_project::DiscoveryService::new().discover(
        path,
        prism_project::ScanOptions::default(),
        &cancel,
        |_| {},
    ) {
        Ok(discovery) => discovery,
        Err(err) => {
            eprintln!("prism: health2 failed building discovery: {err}");
            return crate::EXIT_ERROR;
        }
    };

    // ---- 清单 → 行数输入（有界读取；跳过者记诊断，§36/§59）----
    let root = path;
    let mut inputs: Vec<SourceFileInput> = Vec::new();
    let mut diagnostics: Vec<String> = Vec::new();
    for file in &discovery.files {
        // 结构规则只看源/测试文本；资产/二进制/配置不参与（§11A）。
        let kind = file.classification.kind;
        if !matches!(
            kind,
            prism_project::FileKind::Source | prism_project::FileKind::Test
        ) {
            continue;
        }
        if file.size_bytes > LINE_COUNT_MAX_BYTES {
            diagnostics.push(format!(
                "skipped line count for {} ({} bytes > cap)",
                file.relative_path, file.size_bytes
            ));
            continue;
        }
        let full = root.join(&file.relative_path);
        match std::fs::read(&full) {
            Ok(bytes) => {
                let mut lines = bytes.iter().filter(|b| **b == b'\n').count() as u64;
                if !bytes.is_empty() && *bytes.last().unwrap() != b'\n' {
                    lines += 1;
                }
                inputs.push(SourceFileInput {
                    path: file.relative_path.clone(),
                    lines,
                });
            }
            Err(err) => {
                diagnostics.push(format!("read {} failed: {err}", file.relative_path));
            }
        }
    }

    let observed_at = prism_core::Timestamp::now_utc().to_string();
    let thresholds = RuleThresholds {
        large_file_lines: max_file_lines,
    };
    let mut report = analyze_inventory(&inputs, &thresholds, &observed_at);
    report.diagnostics = diagnostics;

    // ---- 生命周期（§26/§31/§33）：baseline 缺省 ⇒ 首轮全 New ----
    // 基线文件 = 上一次 `prism health2 --json` 的完整输出：报告字段 +
    // `lifecycle.new`（供 Regressed 判定）。schema 不兼容 ⇒ 对比置空
    // （Gate = UNKNOWN，不伪造结论）。
    let empty_baseline = Health2Report::default();
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
    let diff = diff_against_baseline(&report, comparable.unwrap_or(&empty_baseline));
    let diff = if baseline_new_ids.is_empty() {
        diff
    } else {
        prism_health2::mark_regressed(diff, &baseline_new_ids, &report)
    };

    // ---- Quality Gate（§29-§30：策略判断；默认 policy，breaking=0）----
    let gate = evaluate_gate(
        comparable.map(|_| &diff),
        0,
        &prism_health2::QualityGatePolicy::default(),
    );

    if json {
        let payload = serde_json::json!({
            "schema_version": report.schema_version,
            "metrics": report.metrics,
            "findings": report.findings,
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
