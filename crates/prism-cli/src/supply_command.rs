//! `prism supply`（2.0-M29 §75/§103）：Dependency / Security / License /
//! Environment / SBOM 的 CLI 只读入口。
//!
//! 流程：M1 Discovery 清单 → 有界读取 manifest/lock/源码文本 →
//! prism-supply 事实汇总（声明/解析/版本对照/环/secret/环境对照）→
//! M28 RuleRegistry 供给规则 ⇒ 统一 Finding + Metric + 生命周期 +
//! Quality Gate。advisory 库缺省 ⇒ Vulnerability analysis = Unavailable
//! （§105：不是 0 漏洞）。可选 `--sbom` 输出确定性 prism-sbom/1。

use prism_core::config::PrismConfig;
use prism_core::runtime::command::StdCommandRunner;
use prism_core::task::CancellationToken;
use prism_health2::{
    diff_against_baseline, evaluate_gate, mark_regressed, Health2Report, RuleContext, RuleRegistry,
    RuleThresholds,
};
use prism_supply::{collect, SupplyInputs, SUPPLY_SCHEMA_VERSION};
use std::collections::BTreeMap;
use std::path::Path;

/// manifest/lock 有界读取上限（与 M22 manifest 预算一致）。
const MANIFEST_MAX_BYTES: u64 = 1024 * 1024;
/// secret 扫描文本文件读取上限（扫描器自身还有 256 KiB 单文件上限）。
const TEXT_MAX_BYTES: u64 = 1024 * 1024;

const MANIFEST_SUFFIXES: [&str; 9] = [
    ".csproj",
    "directory.packages.props",
    "global.json",
    "packages.lock.json",
    "cargo.lock",
    "cargo.toml",
    "package.json",
    "project.godot",
    "packages.props",
];

pub fn run(
    path: &Path,
    advisories: Option<&Path>,
    policy: Option<&Path>,
    sbom_output: Option<&Path>,
    baseline: Option<&Path>,
    json: bool,
) -> i32 {
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
            eprintln!("prism: supply failed building discovery: {err}");
            return crate::EXIT_ERROR;
        }
    };

    // ---- 有界读取：manifest/lock + 源码文本（secret 扫描面）----
    let mut manifest_files: BTreeMap<String, String> = BTreeMap::new();
    let mut text_files: Vec<(String, String)> = Vec::new();
    let mut cli_diagnostics: Vec<String> = Vec::new();
    for file in &discovery.files {
        let lower_relative = file.relative_path.to_ascii_lowercase();
        let is_manifest = file.relative_path == "project.godot"
            || MANIFEST_SUFFIXES
                .iter()
                .any(|suffix| lower_relative.ends_with(suffix));
        let is_text = matches!(
            file.classification.kind,
            prism_project::FileKind::Source | prism_project::FileKind::Test
        );
        if !is_manifest && !is_text {
            continue;
        }
        let cap = if is_manifest {
            MANIFEST_MAX_BYTES
        } else {
            TEXT_MAX_BYTES
        };
        if file.size_bytes > cap {
            cli_diagnostics.push(format!(
                "skipped {}: {} bytes > cap",
                file.relative_path, file.size_bytes
            ));
            continue;
        }
        match std::fs::read_to_string(path.join(&file.relative_path)) {
            Ok(content) => {
                if is_manifest {
                    manifest_files.insert(file.relative_path.clone(), content);
                } else {
                    text_files.push((file.relative_path.clone(), content));
                }
            }
            Err(err) => cli_diagnostics.push(format!("read {} failed: {err}", file.relative_path)),
        }
    }

    // ---- advisory provider（offline-first：缺省即 Unavailable）----
    let advisory_database =
        advisories.map(|database_path| -> prism_supply::LocalAdvisoryDatabase {
            let content = match std::fs::read_to_string(database_path) {
                Ok(content) => content,
                Err(err) => {
                    eprintln!(
                        "prism: supply failed reading advisories {}: {err}",
                        database_path.to_string_lossy()
                    );
                    std::process::exit(crate::EXIT_INVALID_INPUT);
                }
            };
            match prism_supply::LocalAdvisoryDatabase::load(&content) {
                Ok(database) => database,
                Err(err) => {
                    eprintln!("prism: supply advisory database invalid: {err}");
                    std::process::exit(crate::EXIT_INVALID_INPUT);
                }
            }
        });
    let unavailable_provider = prism_supply::advisory::UnavailableAdvisoryProvider {
        reason: "no advisory database provided".to_string(),
    };

    // ---- license policy（§65：M28 Gate 的 Policy 数据）----
    let license_policy = policy.map(|policy_path| -> prism_supply::LicensePolicy {
        let content = match std::fs::read_to_string(policy_path) {
            Ok(content) => content,
            Err(err) => {
                eprintln!(
                    "prism: supply failed reading policy {}: {err}",
                    policy_path.to_string_lossy()
                );
                std::process::exit(crate::EXIT_INVALID_INPUT);
            }
        };
        match serde_json::from_str::<prism_supply::LicensePolicy>(&content) {
            Ok(parsed) => parsed,
            Err(err) => {
                eprintln!("prism: supply policy invalid: {err}");
                std::process::exit(crate::EXIT_INVALID_INPUT);
            }
        }
    });

    // ---- 真实观测工具（§38/§40：只探测 allowlist 内且项目有声明倾向的
    // dotnet；命令沿 M9 安全探测体系，无任意 shell）----
    let mut observed_tools: BTreeMap<String, String> = BTreeMap::new();
    let dotnet_declared = manifest_files
        .keys()
        .any(|path| path.to_ascii_lowercase().ends_with(".csproj") || path == "global.json");
    if dotnet_declared {
        if let Some(definition) = prism_env::tools::definition("dotnet") {
            let runner = StdCommandRunner::new();
            let fact = prism_env::tools::discover_tool(definition, &runner, path, &cancel);
            observed_tools.insert(
                fact.name.clone(),
                fact.raw_version.clone().unwrap_or_default(),
            );
        }
    }

    let secret_limits = prism_supply::secrets::SecretScanLimits::default();
    let provider: &dyn prism_supply::AdvisoryProvider = match &advisory_database {
        Some(database) => database,
        None => &unavailable_provider,
    };
    let inputs = SupplyInputs {
        files: &manifest_files,
        text_files: &text_files,
        observed_tools: &observed_tools,
        advisory_provider: Some(provider),
        license_policy: license_policy.as_ref(),
        secret_limits: &secret_limits,
    };
    let report = collect(&inputs);

    // ---- SBOM 导出（确定性 prism-sbom/1，§23-§25）----
    if let Some(sbom_path) = sbom_output {
        let document = prism_supply::SbomDocument::from_resolved(
            &report.resolved,
            &report.licenses,
            concat!("prism-supply ", env!("CARGO_PKG_VERSION")),
        );
        match std::fs::write(sbom_path, document.to_deterministic_json()) {
            Ok(()) => eprintln!("prism: sbom written to {}", sbom_path.to_string_lossy()),
            Err(err) => {
                eprintln!("prism: supply failed writing sbom: {err}");
                return crate::EXIT_ERROR;
            }
        }
    }

    // ---- M28 规则评估（§47：同一 Registry/RuleContext/Finding）----
    let registry = RuleRegistry::with_supply_builtins();
    let observed_at = prism_core::Timestamp::now_utc().to_string();
    let thresholds = RuleThresholds::default();
    let rule_context = RuleContext {
        files: &[],
        thresholds: &thresholds,
        observed_at: &observed_at,
        csharp: None,
        godot: None,
        supply: Some(&report),
        debug: None,
        evolution: None,
        cancel: &cancel,
    };
    let mut health_report = registry.evaluate(&rule_context);
    health_report.diagnostics.extend(cli_diagnostics);

    // ---- 生命周期 + Gate（复用 M28，§53/§59/§60）----
    let empty_baseline = Health2Report::default();
    let mut baseline_new_ids: Vec<String> = Vec::new();
    let mut schema_mismatch = false;
    let baseline_report: Option<Health2Report> = baseline.map(|baseline_path| {
        let text = match std::fs::read_to_string(baseline_path) {
            Ok(text) => text,
            Err(err) => {
                eprintln!(
                    "prism: supply failed reading baseline {}: {err}",
                    baseline_path.to_string_lossy()
                );
                std::process::exit(crate::EXIT_INVALID_INPUT);
            }
        };
        let value: serde_json::Value = match serde_json::from_str(&text) {
            Ok(value) => value,
            Err(err) => {
                eprintln!("prism: supply baseline is not valid JSON: {err}");
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
        let parsed: Health2Report = match serde_json::from_value(value) {
            Ok(parsed) => parsed,
            Err(err) => {
                eprintln!("prism: supply baseline is not a valid Health2Report: {err}");
                std::process::exit(crate::EXIT_INVALID_INPUT);
            }
        };
        if parsed.schema_version != health_report.schema_version {
            schema_mismatch = true;
        }
        parsed
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

    // ---- 输出（§104 最低可用产品体验形态；Unknown/Partial 不隐藏）----
    if json {
        let payload = serde_json::json!({
            "schema_version": SUPPLY_SCHEMA_VERSION,
            "supply": report,
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
                eprintln!("prism: failed to serialize supply report: {err}");
                crate::EXIT_ERROR
            }
        };
    }

    let direct = report
        .resolved
        .iter()
        .filter(|dependency| dependency.direct)
        .count();
    println!(
        "supply report (schema v{}, advisories: {:?}{})",
        report.schema_version,
        report.advisory_capability,
        report
            .advisory_database_version
            .as_ref()
            .map(|version| format!(" db={version}"))
            .unwrap_or_default()
    );
    println!(
        "dependencies: declared={} resolved={} (direct {direct}) unresolved={} cycles={}",
        report.declarations.len(),
        report.resolved.len(),
        report
            .version_comparisons
            .iter()
            .filter(|comparison| comparison.state == prism_supply::DriftState::Unresolved)
            .count(),
        report.cycles.len()
    );
    println!(
        "security: secret_candidates={} known_vulnerabilities={} (analysis {:?})",
        report.secret_candidates.len(),
        report.advisory_matches.len(),
        report.advisory_capability
    );
    println!(
        "licenses: known={} unknown={} policy_violations={}",
        report
            .licenses
            .iter()
            .filter(|fact| fact.spdx_known)
            .count(),
        report
            .licenses
            .iter()
            .filter(|fact| !fact.spdx_known)
            .count(),
        report.license_policy_violations.len()
    );
    for comparison in &report.toolchain_comparisons {
        println!(
            "environment: {} declared {:?} observed {:?} => {:?}",
            comparison.tool, comparison.declared, comparison.observed, comparison.state
        );
    }
    println!("findings ({}):", health_report.findings.len());
    for finding in &health_report.findings {
        println!(
            "  {:<24} {:?}  {}",
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
