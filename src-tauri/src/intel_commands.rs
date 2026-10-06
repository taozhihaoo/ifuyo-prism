//! 2.0 智能面板命令（P1-1/P1-2）：health2 第二轨道、演化热点/耦合、
//! 概览 Intelligence 摘要。
//!
//! 沿 viz_commands.rs 模式：按需调用既有引擎。health2 走
//! `prism_health2::analyze_project`——与 CLI `prism health2` **同一函数**，
//! GUI 与 CLI 输出一致由构造保证（§71）。演化走 M211 bounded
//! `scan_repo`（§47/§52：单次 `log --numstat`，禁全历史）。缺数据 ⇒
//! 显式 Unknown/unavailable + 原因，不用错误掩盖能力缺口（§46）。

use crate::commands::HealthAnalysisStatusDto;
use crate::state::{lock_session, AppState};
use prism_core::error::PrismErrorDto;
use prism_core::runtime::command::StdCommandRunner;
use prism_core::task::CancellationToken;
use prism_health2::{LifecycleDiff, QualityGateResult};
use serde::Serialize;
use std::collections::BTreeMap;
use std::sync::Arc;
use tauri::State;

/// GUI 默认大文件阈值 = CLI `prism health2 --max-file-lines` 缺省值。
const DEFAULT_MAX_FILE_LINES: u64 = 1000;
/// 演化窗口（§52 RecentCommits）与表格显示上限（缺多少如实标注）。
const EVOLUTION_MAX_COMMITS: u64 = 200;
const HOTSPOT_LIMIT: usize = 10;
const COUPLING_LIMIT: usize = 20;

// ---------- health2（P1-1 GUI 第二轨道；后台任务模式同 1.0 health）----------

/// P2-4b：规则来源标注（条数运行时派生，不硬编码）。
#[derive(Debug, Clone, Serialize)]
pub struct Health2RuleSourceDto {
    pub engine: String,
    pub builtin_rules: usize,
}

/// GUI health2 报告 = CLI `prism health2 --json` 同构负载（§71）+ 来源标注。
#[derive(Debug, Clone, Serialize)]
pub struct Health2GuiReport {
    pub schema_version: u32,
    pub metrics: BTreeMap<String, prism_health2::Metric>,
    pub findings: Vec<prism_intel::Finding>,
    pub evidence: Vec<prism_intel::evidence::Evidence>,
    pub diagnostics: Vec<String>,
    pub lifecycle: LifecycleDiff,
    pub quality_gate: QualityGateResult,
    pub rule_source: Health2RuleSourceDto,
}

/// 启动一次 health2 分析（后台线程；会话内有效，Manual First 按需触发）。
#[tauri::command]
pub fn start_health2(state: State<'_, AppState>) -> Result<String, PrismErrorDto> {
    let (root, token, session_arc, cache) = {
        let mut session = lock_session(&state.session);
        let Some(project) = session.project.as_ref() else {
            return Err(PrismErrorDto::new(
                "invalid_input",
                "no project opened; call open_project first",
            ));
        };
        if session.health2_status == Some(prism_core::task::TaskState::Running) {
            return Err(PrismErrorDto::new(
                "invalid_input",
                "a health2 analysis is already running",
            ));
        }
        let root = project.root_path.clone();
        session.begin_health2();
        let token = session.health2_token.clone();
        (root, token, state.session.clone(), state.cache.clone())
    };

    std::thread::spawn(move || {
        let outcome = prism_health2::analyze_project(&root, DEFAULT_MAX_FILE_LINES, &token);
        let mut session = lock_session(&session_arc);
        match outcome {
            Ok(analyzed) => {
                session.health2_status = Some(prism_core::task::TaskState::Completed);
                // P2-2：完成即写入会话缓存（重开项目后页面可秒级回显）。
                crate::session_cache::cache_health2(
                    &cache,
                    std::path::Path::new(&root),
                    &build_health2_gui_report(
                        &analyzed.report,
                        Some(&analyzed.lifecycle),
                        Some(&analyzed.gate),
                        analyzed.builtin_rules,
                    ),
                );
                session.health2_model = Some(analyzed.report);
                session.health2_lifecycle = Some(analyzed.lifecycle);
                session.health2_gate = Some(analyzed.gate);
                session.health2_builtin_rules = analyzed.builtin_rules;
            }
            Err(prism_core::PrismError::Cancelled) => {
                session.health2_status = Some(prism_core::task::TaskState::Cancelled);
            }
            Err(err) => {
                session.health2_status = Some(prism_core::task::TaskState::Failed);
                session.health2_error = Some(err.to_string());
            }
        }
    });

    Ok("started".to_string())
}

/// 查询 health2 会话状态。
#[tauri::command]
pub fn get_health2_status(state: State<'_, AppState>) -> HealthAnalysisStatusDto {
    let session = lock_session(&state.session);
    let status = match session.health2_status {
        None => "idle".to_string(),
        Some(status) => serde_json::to_value(status)
            .ok()
            .and_then(|v| v.as_str().map(str::to_string))
            .unwrap_or_else(|| "unknown".to_string()),
    };
    HealthAnalysisStatusDto {
        status,
        error: session.health2_error.clone(),
    }
}

/// 取消 health2（§102；页面卸载/切轨道时调用，idle 取消为无操作）。
#[tauri::command]
pub fn cancel_health2(state: State<'_, AppState>) -> Result<(), PrismErrorDto> {
    let session = lock_session(&state.session);
    session.health2_token.cancel();
    Ok(())
}

/// 会话字段 → GUI 报告（get_health2_results 与缓存写入共用）。
pub(crate) fn build_health2_gui_report(
    report: &prism_health2::Health2Report,
    lifecycle: Option<&LifecycleDiff>,
    gate: Option<&QualityGateResult>,
    builtin_rules: usize,
) -> Health2GuiReport {
    Health2GuiReport {
        schema_version: report.schema_version,
        metrics: report.metrics.clone(),
        findings: report.findings.clone(),
        evidence: report.evidence.clone(),
        diagnostics: report.diagnostics.clone(),
        lifecycle: lifecycle.cloned().unwrap_or_default(),
        quality_gate: gate.cloned().unwrap_or(QualityGateResult {
            verdict: prism_health2::GateVerdict::Unknown,
            new_count: 0,
            breaking_count: 0,
            reasons: Vec::new(),
        }),
        rule_source: Health2RuleSourceDto {
            engine: "prism-health2".to_string(),
            builtin_rules,
        },
    }
}

/// 最近一次完成的 health2 报告（§71：与 CLI --json 同构 + 来源标注）。
#[tauri::command]
pub fn get_health2_results(state: State<'_, AppState>) -> Option<Health2GuiReport> {
    let session = lock_session(&state.session);
    let report = session.health2_model.as_ref()?;
    Some(build_health2_gui_report(
        report,
        session.health2_lifecycle.as_ref(),
        session.health2_gate.as_ref(),
        session.health2_builtin_rules,
    ))
}

// ---------- 演化（M211 bounded：hotspots + couplings 表格）----------

/// 演化面板数据。非 git 目录/无历史 ⇒ `available:false` + 原因（§46：
/// 不把"没有 git 历史"伪装成"没有热点"）。
#[derive(Serialize)]
pub struct EvolutionGuiReport {
    pub available: bool,
    pub unavailable_reason: Option<String>,
    pub window: String,
    pub commits_scanned: usize,
    pub hotspots: Vec<prism_evolution::Hotspot>,
    pub couplings: Vec<prism_evolution::ChangeCoupling>,
}

#[tauri::command]
pub fn get_evolution_report(state: State<'_, AppState>) -> EvolutionGuiReport {
    let unavailable = |reason: String| EvolutionGuiReport {
        available: false,
        unavailable_reason: Some(reason),
        window: String::new(),
        commits_scanned: 0,
        hotspots: Vec::new(),
        couplings: Vec::new(),
    };
    let root = {
        let session = lock_session(&state.session);
        match session.project.as_ref() {
            Some(project) => project.root_path.clone(),
            None => return unavailable("no project opened".to_string()),
        }
    };
    let service = prism_git::GitService::new(Arc::new(StdCommandRunner::new()));
    let cancel = CancellationToken::new();
    let scan = match prism_evolution::scan_repo(&service, &root, EVOLUTION_MAX_COMMITS, &cancel) {
        Ok(scan) => scan,
        Err(err) => return unavailable(err.to_string()),
    };
    EvolutionGuiReport {
        available: true,
        unavailable_reason: None,
        window: scan.window().as_label(),
        commits_scanned: scan.commit_count(),
        hotspots: scan.hotspots(HOTSPOT_LIMIT),
        couplings: scan.change_couplings(3, COUPLING_LIMIT),
    }
}

// ---------- 概览 Intelligence 摘要（P1-2；缺数据 ⇒ None ⇒ GUI 显 Unknown）----------

#[derive(Serialize)]
pub struct SeverityCountsDto {
    pub critical: u64,
    pub warning: u64,
    pub info: u64,
}

#[derive(Serialize)]
pub struct GodotCountsDto {
    pub scenes: usize,
    pub resources: usize,
    pub nodes: usize,
}

#[derive(Serialize)]
pub struct DepsCountsDto {
    pub declarations: usize,
    pub resolved: usize,
    pub cycles: usize,
}

/// 概览 Intelligence 摘要卡。`None` = 该域本会话没有事实（GUI 显 Unknown，
/// 不画成正常，§46）；health2/health 来自会话内存，godot/deps 为按需
/// bounded 投影（单次 discovery，缺 godot/manifest 文件 ⇒ None）。
#[derive(Serialize)]
pub struct IntelligenceSummaryDto {
    pub health2: Option<SeverityCountsDto>,
    pub health: Option<u64>,
    pub godot: Option<GodotCountsDto>,
    pub deps: Option<DepsCountsDto>,
}

#[tauri::command]
pub fn get_intelligence_summary(state: State<'_, AppState>) -> IntelligenceSummaryDto {
    let (root, health2, health) = {
        let session = lock_session(&state.session);
        let health2 = session.health2_model.as_ref().map(|report| {
            let mut counts = SeverityCountsDto {
                critical: 0,
                warning: 0,
                info: 0,
            };
            for finding in &report.findings {
                match finding.severity {
                    prism_core::Severity::Critical => counts.critical += 1,
                    prism_core::Severity::Warning => counts.warning += 1,
                    prism_core::Severity::Info => counts.info += 1,
                }
            }
            counts
        });
        let health = session
            .health_model
            .as_ref()
            .map(|analysis| analysis.findings.len() as u64);
        let root = session
            .project
            .as_ref()
            .map(|project| project.root_path.clone());
        (root, health2, health)
    };
    let Some(root) = root else {
        return IntelligenceSummaryDto {
            health2,
            health,
            godot: None,
            deps: None,
        };
    };

    // 单次 discovery 共享给 godot/deps 两个投影（各为 bounded 只读）。
    let cancel = CancellationToken::new();
    let discovery = match prism_project::DiscoveryService::new().discover(
        &root,
        prism_project::ScanOptions::default(),
        &cancel,
        |_| {},
    ) {
        Ok(discovery) => discovery,
        Err(_) => {
            return IntelligenceSummaryDto {
                health2,
                health,
                godot: None,
                deps: None,
            }
        }
    };

    // ---- Godot 计数（有 godot 文件才分析；project 缺失 ⇒ None）----
    let godot_files: Vec<(String, String)> = discovery
        .files
        .iter()
        .filter(|f| {
            f.relative_path == "project.godot"
                || f.relative_path.ends_with(".tscn")
                || f.relative_path.ends_with(".tres")
        })
        .map(|f| (f.relative_path.clone(), String::new()))
        .collect();
    let godot = if godot_files.is_empty() {
        None
    } else {
        (|| {
            let project = prism_project::Project::from_root(&root).ok()?;
            let context = prism_intel::AnalysisContext::new(
                prism_intel::ProjectIdentity::from_project(&project),
                prism_intel::AnalysisScope::Domain("godot".to_string()),
                vec!["godot".to_string()],
                cancel.clone(),
            );
            let outcome = prism_godot::analyze(&godot_files, &project.root_path, &context);
            let analysis = outcome.data?;
            let nodes = analysis.scenes.values().map(|s| s.nodes.len()).sum();
            Some(GodotCountsDto {
                scenes: analysis.scenes.len(),
                resources: analysis.resources.len(),
                nodes,
            })
        })()
    };

    // ---- 依赖计数（manifests 有界读取 ⇒ supply collect）----
    let deps = (|| {
        let mut manifests: BTreeMap<String, String> = BTreeMap::new();
        for file in &discovery.files {
            let lower = file.relative_path.to_ascii_lowercase();
            let is_manifest = file.relative_path == "project.godot"
                || lower.ends_with(".csproj")
                || lower.ends_with("directory.packages.props")
                || lower == "global.json"
                || lower.ends_with("packages.lock.json")
                || lower.ends_with("cargo.lock")
                || lower.ends_with("cargo.toml")
                || lower.ends_with("package.json");
            if !is_manifest || file.size_bytes > 1024 * 1024 {
                continue;
            }
            if let Ok(content) = std::fs::read_to_string(root.join(&file.relative_path)) {
                manifests.insert(file.relative_path.clone(), content);
            }
        }
        if manifests.is_empty() {
            return None;
        }
        let inputs = prism_supply::SupplyInputs {
            files: &manifests,
            text_files: &[],
            observed_tools: &BTreeMap::new(),
            advisory_provider: None,
            license_policy: None,
            secret_limits: Box::leak(Box::new(prism_supply::secrets::SecretScanLimits::default())),
        };
        let report = prism_supply::collect(&inputs);
        Some(DepsCountsDto {
            declarations: report.declarations.len(),
            resolved: report.resolved.len(),
            cycles: report.cycles.len(),
        })
    })();

    IntelligenceSummaryDto {
        health2,
        health,
        godot,
        deps,
    }
}
