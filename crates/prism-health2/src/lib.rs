//! # Prism Health & Engineering Findings 2.0（2.0-M28）
//!
//! 职责分离（§3/§21）：**Finding 是分析结果；Metric 是测量结果；Health
//! 聚合是视图；Quality Gate 是策略判断**。Finding 沿 M21
//! `prism_intel::Finding`（唯一 SoT，§4——不建第二套模型）。
//!
//! 第一批规则（§11A）：结构面——大文件（阈值可配置）。输入 = L0/M22
//! 清单；不重新扫描文件系统（§2）。生命周期（fingerprint/new/persistent/
//! resolved/regressed）由 `baseline::diff_against_baseline` 对比两轮报告
//! 推导（§26/§31/§33）；Quality Gate 为策略判断（§29-§30）。
//!
//! 边界：不吞并 Debug/Security/Dependency Intelligence（§2）；无评分
//! 伪装（§21/§38）；纯内存分析、无 shell/网络。

pub mod baseline;
pub mod gate;
pub mod model;
pub mod registry;
pub mod rules;
pub mod rules_csharp;
pub mod rules_debug;
pub mod rules_godot;
pub mod rules_supply;

pub use baseline::{diff_against_baseline, mark_regressed};
pub use gate::{evaluate_gate, GateVerdict, QualityGatePolicy, QualityGateResult};
pub use model::{Health2Report, LifecycleDiff, Metric};
pub use registry::{
    CSharpFacts, RuleCategory, RuleContext, RuleDefinition, RuleOutcome, RuleRegistry,
};
pub use rules::{large_file_findings, RuleThresholds, SourceFileInput};

/// M28 输出契约版本（§71）。
pub const HEALTH2_SCHEMA_VERSION: u32 = 1;

/// 便捷分析入口：L0 清单 → 规则 → 报告（§73 Vertical Slice 的库形态）。
/// 行数 = 有界读取后的换行计数（仅文本、上限内；二进制/超大 ⇒ 跳过并
/// 记录诊断，§36/§59）。
pub fn analyze_inventory(
    files: &[SourceFileInput],
    thresholds: &RuleThresholds,
    observed_at: &str,
) -> Health2Report {
    let findings = rules::large_file_findings(files, thresholds, observed_at);
    let total_lines: u64 = files.iter().map(|f| f.lines).sum();
    Health2Report {
        schema_version: HEALTH2_SCHEMA_VERSION,
        metrics: metrics_from_counts(files.len() as u64, total_lines, findings.len() as u64),
        findings,
        evidence: Vec::new(),
        diagnostics: Vec::new(),
    }
}

fn metrics_from_counts(
    files_total: u64,
    lines_total: u64,
    large_files: u64,
) -> std::collections::BTreeMap<String, Metric> {
    let mut map = std::collections::BTreeMap::new();
    for (key, value, definition) in [
        (
            "files_total",
            files_total,
            "number of inventoried source files",
        ),
        (
            "lines_total",
            lines_total,
            "total lines across inventoried files",
        ),
        (
            "large_files",
            large_files,
            "files over the large-file threshold",
        ),
    ] {
        map.insert(
            key.to_string(),
            Metric {
                key: key.to_string(),
                value,
                unit: "count".to_string(),
                definition: definition.to_string(),
                producer: "prism-health2::analyze_inventory".to_string(),
            },
        );
    }
    map
}
