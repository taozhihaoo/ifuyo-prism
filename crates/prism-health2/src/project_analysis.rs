//! 项目级 health2 全管线（CLI `prism health2` 与 GUI 健康页 2.0 轨道共用，
//! §71 一致性：同一函数、同一注入、同一注册表 ⇒ 输出必然一致）。
//!
//! 单一管线：M1 Discovery 清单 → 注入既有引擎事实（L0 行数 / M23-M25
//! C# / M24 Godot）→ `RuleRegistry::with_builtins()` 评估（supply/debug/
//! evolution/api 前提缺省 ⇒ 相应规则记诊断跳过，§48 prerequisite）→
//! 空基线生命周期（首轮全 New，§26）→ 默认 policy Quality Gate（§29-§30）。
//! 规则不自扫描文件系统、不重跑分析器（§2）；无网络/shell。
//! 调用方在此产物之上叠加基线文件对比与展示。

use crate::{
    diff_against_baseline, evaluate_gate, CSharpFacts, Health2Report, LifecycleDiff,
    QualityGatePolicy, QualityGateResult, RuleContext, RuleRegistry, RuleThresholds,
    SourceFileInput,
};
use prism_core::task::CancellationToken;
use prism_core::PrismResult;
use std::collections::BTreeMap;

/// 行数统计的有界读取上限（与 M23/M25 上下文路径一致的 4 MiB 预算）。
const LINE_COUNT_MAX_BYTES: u64 = 4 * 1024 * 1024;

/// 一次项目级 health2 分析的完整产物。
pub struct ProjectHealth2 {
    pub report: Health2Report,
    /// 空基线对比（首轮全 New）；调用方有基线时可重算。
    pub lifecycle: LifecycleDiff,
    /// 默认 policy 下的 Gate（调用方有基线时以重算 diff 为准）。
    pub gate: QualityGateResult,
    /// 内置规则注册表条数（P2-4b 来源标注用，运行时派生不硬编码）。
    pub builtin_rules: usize,
}

/// 运行项目级 health2 分析（CLI 与 GUI 的唯一入口）。
pub fn analyze_project(
    root: &std::path::Path,
    max_file_lines: u64,
    cancel: &CancellationToken,
) -> PrismResult<ProjectHealth2> {
    let discovery = prism_project::DiscoveryService::new().discover(
        root,
        prism_project::ScanOptions::default(),
        cancel,
        |_| {},
    )?;

    // ---- 清单 → 行数输入（有界读取；跳过者记诊断，§36/§59）----
    // .cs 内容顺手缓存（同一次有界读取复用给 M23/M25 引擎，不二次读盘）。
    let mut inputs: Vec<SourceFileInput> = Vec::new();
    let mut cs_sources: BTreeMap<String, String> = BTreeMap::new();
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
                if file
                    .relative_path
                    .rsplit('/')
                    .next()
                    .map(|name| name.ends_with(".cs"))
                    .unwrap_or(false)
                {
                    cs_sources.insert(
                        file.relative_path.clone(),
                        String::from_utf8_lossy(&bytes).into_owned(),
                    );
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
        ..RuleThresholds::default()
    };

    // ---- C# 引擎事实（M23 抽取 + M25 语义；注入规则，不自解析 §2）----
    let mut symbol_engine = prism_csharp::SymbolEngine::new();
    for (path, source) in &cs_sources {
        symbol_engine.update_file(path, source);
    }
    let symbol_index = symbol_engine.build_index();
    let mut semantic_engine = prism_csharp::SemanticEngine::new();
    for (path, source) in &cs_sources {
        semantic_engine.update_source(&symbol_index, path, source);
    }
    semantic_engine.rebuild(&symbol_index);
    let csharp_facts = CSharpFacts {
        extractions: symbol_engine.extraction_map(),
        semantics: if cs_sources.is_empty() {
            None
        } else {
            Some(&semantic_engine)
        },
    };

    // ---- Godot 事实（M24 Adapter；有 godot 文件才分析）----
    let godot_files: Vec<(String, String)> = discovery
        .files
        .iter()
        .filter(|f| {
            f.relative_path == "project.godot"
                || f.relative_path.ends_with(".tscn")
                || f.relative_path.ends_with(".tres")
                || f.relative_path.ends_with(".res")
        })
        .map(|f| (f.relative_path.clone(), String::new()))
        .collect();
    let godot_analysis = if godot_files.is_empty() {
        None
    } else {
        let project = prism_project::Project::from_root(root)?;
        let context = prism_intel::AnalysisContext::new(
            prism_intel::ProjectIdentity::from_project(&project),
            prism_intel::AnalysisScope::Domain("godot".to_string()),
            vec!["godot".to_string()],
            cancel.clone(),
        );
        let outcome = prism_godot::analyze(&godot_files, &project.root_path, &context);
        for warning in &outcome.warnings {
            diagnostics.push(format!("godot: {warning}"));
        }
        outcome.data
    };

    // ---- RuleRegistry 评估（§C：确定性顺序；取消保留部分并记诊断）----
    let registry = RuleRegistry::with_builtins();
    let builtin_rules = registry.rule_count();
    let rule_context = RuleContext {
        files: &inputs,
        thresholds: &thresholds,
        observed_at: &observed_at,
        csharp: Some(csharp_facts),
        godot: godot_analysis.as_ref(),
        supply: None,
        debug: None,
        evolution: None,
        api: None,
        cancel,
    };
    let mut report = registry.evaluate(&rule_context);
    report.diagnostics.extend(diagnostics);

    // ---- 生命周期（§26/§31/§33）：空基线 ⇒ 首轮全 New ----
    let empty_baseline = Health2Report::default();
    let diff = diff_against_baseline(&report, &empty_baseline);

    // ---- Quality Gate（§29-§30：策略判断；默认 policy，breaking=0）----
    let gate = evaluate_gate(Some(&diff), 0, &QualityGatePolicy::default());

    Ok(ProjectHealth2 {
        report,
        lifecycle: diff,
        gate,
        builtin_rules,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 临时项目夹具：small.cs（正常）+ big.cs（超阈值）+ project.godot。
    fn write_fixture() -> std::path::PathBuf {
        let root =
            std::env::temp_dir().join(format!("prism-health2-analyze-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("project.godot"), "config_version=5\n").unwrap();
        std::fs::write(root.join("small.cs"), "class Small { }\n").unwrap();
        let big: String = "// pad\n".repeat(1200);
        std::fs::write(root.join("big.cs"), &big).unwrap();
        root
    }

    #[test]
    fn analyzes_project_end_to_end_with_builtin_registry() {
        let cancel = CancellationToken::new();
        let root = write_fixture();
        let outcome = analyze_project(&root, 1000, &cancel);
        let _ = std::fs::remove_dir_all(&root);
        let analyzed = outcome.expect("analyze_project should succeed on fixture");

        // 内置规则数 = 注册表运行时派生。`prism health2`（CLI）与本管线
        // 同用 with_builtins()（STRUCT/COMPLEX/SEMANTIC/GODOT 四条）；
        // 防意外增减回归。
        assert_eq!(analyzed.builtin_rules, 4);
        // 结构规则产出 LARGE_FILE finding（big.cs 1200 行 > 1000）。
        assert!(analyzed
            .report
            .findings
            .iter()
            .any(|f| f.finding_type == "STRUCT/LARGE_FILE"));
        // 空基线 ⇒ 全部 New；Gate 默认 policy（max_new=0）⇒ Fail。
        assert_eq!(analyzed.lifecycle.new_findings.len(), 1);
        assert_eq!(analyzed.lifecycle.persistent_findings.len(), 0);
        assert_eq!(analyzed.gate.verdict, crate::GateVerdict::Fail);
        // Godot 前提满足（project.godot 存在）⇒ 无 godot 前提诊断。
        assert!(analyzed
            .report
            .diagnostics
            .iter()
            .all(|d| !d.contains("godot:")));
        // metrics 记录清单规模。
        assert!(analyzed.report.metrics.contains_key("files_total"));
    }

    #[test]
    fn godotless_project_reports_no_findings_but_no_failure() {
        let cancel = CancellationToken::new();
        let root = std::env::temp_dir().join(format!("prism-health2-plain-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("main.cs"), "class M { }\n").unwrap();
        let outcome = analyze_project(&root, 1000, &cancel);
        let _ = std::fs::remove_dir_all(&root);
        let analyzed = outcome.expect("plain project should analyze");
        assert!(analyzed.report.findings.is_empty());
        assert!(analyzed.lifecycle.new_findings.is_empty());
        assert_eq!(analyzed.gate.verdict, crate::GateVerdict::Pass);
    }
}
