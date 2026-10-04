//! CandidateCollectors（规格 M10a §87–§89/§98–§108/§431–§438/§251）。
//!
//! 每个 collector 只产生候选（§89 隔离：不做 ranking/budget/rendering）；
//! 全部复用调用方传入的 M1–M9 既有事实（§48–§53/§263），绝不重新扫描。
//! 任一 collector 失败不影响其余（§260——collectors 本身不返回错误）。

use crate::model::{
    ContextCandidate, ContextGroup, ContextScope, ContextSource, ContextTarget,
    SelectionReason, SourceType, TargetResolution,
};
use prism_core::task::CancellationToken;

/// 调用方传入的 M1–M9 既有事实（§263：Context 引擎不重算任何引擎）。
pub struct FactsInputs<'a> {
    pub discovery: Option<&'a prism_project::ProjectDiscovery>,
    pub code_model: Option<&'a prism_analysis::ProjectCodeModel>,
    pub changeset: Option<&'a prism_changes::ChangeSet>,
    pub impact: Option<&'a prism_impact::ImpactAnalysis>,
    pub graph: Option<&'a prism_graph::DependencyGraphResult>,
    pub health: Option<&'a prism_health::HealthAnalysis>,
    pub verification: Option<&'a prism_verify::VerificationReport>,
    pub environment: Option<&'a prism_env::EnvironmentSnapshot>,
    pub classification: &'a crate::model::TaskClassification,
    pub target: &'a ContextTarget,
    pub scope: ContextScope,
    pub cancel: &'a CancellationToken,
    /// §136/§137：当前项目状态引用（M4 事实）。
    pub state: crate::model::ContextStateReference,
    /// §131–§132：M8 事实相对当前工作区的新鲜度。
    pub verification_stale: crate::model::ContextFreshness,
}

/// 收集全部候选（各 collector 顺序稳定；候选内部排序由 selector 保证）。
pub fn collect_candidates(inputs: &FactsInputs) -> Vec<ContextCandidate> {
    let mut candidates = Vec::new();
    candidates.extend(target_candidates(inputs));
    candidates.extend(change_candidates(inputs));
    candidates.extend(diff_candidates(inputs));
    candidates.extend(impact_candidates(inputs));
    candidates.extend(test_candidates(inputs));
    candidates.extend(health_candidates(inputs));
    candidates.extend(verification_candidates(inputs));
    candidates.extend(environment_candidates(inputs));
    candidates.extend(documentation_candidates(inputs));
    candidates.extend(architecture_candidates(inputs));
    candidates.extend(project_candidates(inputs));
    candidates
}

#[allow(clippy::too_many_arguments)]
fn candidate(
    source_type: SourceType,
    source_id: String,
    path: Option<String>,
    provenance: &str,
    group: ContextGroup,
    title: String,
    reasons: Vec<SelectionReason>,
    evidence: Vec<String>,
) -> ContextCandidate {
    ContextCandidate {
        source: ContextSource {
            source_type,
            source_id,
            path: path.clone(),
            location: None,
            provenance: provenance.to_string(),
        },
        group,
        title,
        reasons,
        evidence,
        impact_distance: None,
        change_state: None,
        language: None,
        symbol_range: None,
        symbol_id: None,
        symbol_name: None,
        severity: None,
        change_type: None,
        line_count_estimate: None,
    }
}

/// §85/§16：显式目标 ⇒ Critical（Resolved 才产生候选；Ambiguous/NotFound
/// 只进 diagnostics——由 service 处理）。
fn target_candidates(inputs: &FactsInputs) -> Vec<ContextCandidate> {
    let mut out = Vec::new();
    if let TargetResolution::Resolved { paths, symbols } = &inputs.target.resolution {
        for path in paths {
            out.push(candidate(
                SourceType::File,
                format!("file:{path}"),
                Some(path.clone()),
                "task target",
                ContextGroup::TargetCode,
                format!("Target file: {path}"),
                vec![SelectionReason::ExplicitTaskTarget],
                vec!["explicit task target".to_string()],
            ));
        }
        for symbol_id in symbols {
            // M2 SymbolId 形态（§94）：`symbol:{path}#{qualified}#{kind}`——
            // 注意 "symbol:" 前缀粘在 path 段上。
            let segments: Vec<&str> = symbol_id.split('#').collect();
            let path_part = segments
                .first()
                .and_then(|first| first.strip_prefix("symbol:"))
                .map(str::to_string);
            let qualified = segments.get(1).map(|qualified| qualified.to_string());
            let name_part = qualified
                .as_deref()
                .and_then(|qualified| qualified.rsplit('.').next())
                .map(str::to_string);
            out.push(ContextCandidate {
                source: ContextSource {
                    source_type: SourceType::Symbol,
                    source_id: format!("symbol:{symbol_id}"),
                    path: path_part.clone(),
                    location: None,
                    provenance: "task target symbol".to_string(),
                },
                group: ContextGroup::TargetCode,
                title: format!(
                    "Target symbol: {}",
                    name_part.clone().unwrap_or_else(|| symbol_id.clone())
                ),
                reasons: vec![SelectionReason::ExplicitTaskTarget],
                evidence: vec![format!("M2 SymbolId {symbol_id}")],
                symbol_id: Some(symbol_id.clone()),
                symbol_name: name_part,
                impact_distance: None,
                change_state: None,
                language: None,
                symbol_range: None,
                severity: None,
                change_type: None,
                line_count_estimate: None,
            });
        }
    }
    out
}

/// §28/§44：ChangeSet 变更文件/符号（M5 复用，§50）。
fn change_candidates(inputs: &FactsInputs) -> Vec<ContextCandidate> {
    let Some(changeset) = inputs.changeset else {
        return Vec::new();
    };
    if inputs.scope == ContextScope::Target && inputs.target.resolution != TargetResolution::None {
        // Target scope：变更文件仅在命中目标时入选（selector 按 scope 过滤）。
    }
    let mut out = Vec::new();
    for file in &changeset.files {
        let Some(path) = file.new_path.as_ref().or(file.old_path.as_ref()) else {
            continue;
        };
        let reasons = vec![SelectionReason::DirectlyChanged, SelectionReason::CurrentDiff];
        out.push(candidate(
            SourceType::ChangeSet,
            file.id.clone(),
            Some(path.clone()),
            "M5 ChangeSet",
            ContextGroup::CurrentChange,
            format!("Changed file: {path}"),
            reasons,
            vec![format!("changeset {}", changeset.id)],
        ));
    }
    for symbol in &changeset.symbols {
        out.push(ContextCandidate {
            source: ContextSource {
                source_type: SourceType::Symbol,
                source_id: format!(
                    "symbol:{}",
                    symbol
                        .after_symbol_id
                        .as_ref()
                        .map(|id| id.as_str().to_string())
                        .unwrap_or_else(|| format!("{}#{}", symbol.path, symbol.name))
                ),
                path: Some(symbol.path.clone()),
                location: symbol
                    .location_after
                    .as_ref()
                    .map(|range| format!("L{}-L{}", range.start_line, range.end_line)),
                provenance: "M5 ChangeSet symbols".to_string(),
            },
            group: ContextGroup::CurrentChange,
            title: format!("Changed symbol: {}", symbol.name),
            reasons: vec![SelectionReason::DirectlyChanged],
            evidence: vec![format!("change_type={:?} confidence={:?}", symbol.change_type, symbol.confidence)],
            impact_distance: None,
            change_state: Some(format!("{:?}", symbol.change_type)),
            language: None,
            symbol_range: symbol
                .location_after
                .as_ref()
                .map(|range| (u64::from(range.start_line), u64::from(range.end_line))),
            symbol_id: symbol
                .after_symbol_id
                .as_ref()
                .map(|id| id.as_str().to_string()),
            symbol_name: Some(symbol.name.clone()),
            severity: None,
            change_type: None,
            line_count_estimate: symbol
                .location_after
                .as_ref()
                .map(|range| u64::from(range.end_line.saturating_sub(range.start_line)) + 1),
        });
    }
    out
}

/// 当前 diff 概览（review 任务 ⇒ Critical 提升，§44/§99；提升在 selector）。
fn diff_candidates(inputs: &FactsInputs) -> Vec<ContextCandidate> {
    let Some(changeset) = inputs.changeset else {
        return Vec::new();
    };
    if !inputs
        .classification
        .kinds
        .contains(&crate::model::TaskKind::Review)
    {
        return Vec::new();
    }
    vec![candidate(
        SourceType::GitDiff,
        format!("git-diff:{}", changeset.id),
        None,
        "M5 ChangeSet (review scope)",
        ContextGroup::CurrentChange,
        "Current diff overview (staged/unstaged preserved)".to_string(),
        vec![SelectionReason::CurrentDiff, SelectionReason::TaskMatch],
        vec![format!("{} files changed", changeset.files.len())],
    )]
}

/// §45/§100/§427：Impact 受影响候选（M6 复用，§48；bounded，不重扩图）。
fn impact_candidates(inputs: &FactsInputs) -> Vec<ContextCandidate> {
    let Some(impact) = inputs.impact else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for item in &impact.impacted_items {
        let reasons = vec![SelectionReason::DirectlyImpacted];
        let path = if item.path.is_empty() { None } else { Some(item.path.clone()) };
        out.push(ContextCandidate {
            source: ContextSource {
                source_type: SourceType::ImpactAnalysis,
                source_id: item.node_id.clone(),
                path,
                location: None,
                provenance: format!("M6 ImpactAnalysis distance={}", item.distance),
            },
            group: ContextGroup::Impact,
            title: format!("Impacted: {} ({:?})", item.name, item.node_kind),
            reasons,
            evidence: vec![format!(
                "distance={} impact_kinds={:?} paths={}",
                item.distance, item.impact_kinds, item.evidence_path_count
            )],
            impact_distance: Some(item.distance),
            change_state: None,
            language: None,
            symbol_range: None,
            symbol_id: Some(item.node_id.clone()),
            symbol_name: Some(item.name.clone()),
            severity: None,
            change_type: None,
            line_count_estimate: None,
        });
    }
    out
}

/// §101/§204：相关测试（M6 TestCandidate；§102 不冒充完整集合）。
fn test_candidates(inputs: &FactsInputs) -> Vec<ContextCandidate> {
    let Some(impact) = inputs.impact else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for item in &impact.impacted_items {
        if !item
            .impact_kinds
            .contains(&prism_impact::ImpactKind::TestCandidate)
            || item.path.is_empty()
        {
            continue;
        }
        out.push(candidate(
            SourceType::File,
            format!("file:{}", item.path),
            Some(item.path.clone()),
            "M6 TestCandidate (candidate, not complete)",
            ContextGroup::Tests,
            format!("Candidate test: {}", item.path),
            vec![SelectionReason::RelatedTest],
            vec!["M6 TestCandidate — potential, not a complete affected-test set".to_string()],
        ));
    }
    out
}

/// §105/§430：只带与目标/变更相关的 findings（不 dump 全部，§105）。
fn health_candidates(inputs: &FactsInputs) -> Vec<ContextCandidate> {
    let Some(health) = inputs.health else {
        return Vec::new();
    };
    let relevant_paths: Vec<String> = relevant_paths_from(inputs);
    let mut out = Vec::new();
    for finding in &health.findings {
        let finding_path = finding.location.relative_path.clone();
        let related = relevant_paths.is_empty()
            || relevant_paths
                .iter()
                .any(|path| path == &finding_path || finding_path.starts_with(path.as_str()));
        if !related {
            continue;
        }
        out.push(ContextCandidate {
            source: ContextSource {
                source_type: SourceType::HealthFinding,
                source_id: finding.id.as_str().to_string(),
                path: Some(finding_path.clone()),
                location: Some(format!("L{}", finding.location.line)),
                provenance: "M7 HealthAnalysis".to_string(),
            },
            group: ContextGroup::Health,
            title: format!("{}: {}", finding.rule_id, finding.title),
            reasons: vec![SelectionReason::HealthFinding],
            evidence: vec![format!("severity={:?} at {} L{}", finding.severity, finding_path, finding.location.line)],
            impact_distance: None,
            change_state: Some(format!("{:?}", finding.change_relation)),
            language: None,
            symbol_range: None,
            symbol_id: None,
            symbol_name: None,
            severity: Some(format!("{:?}", finding.severity)),
            change_type: None,
            line_count_estimate: None,
        });
    }
    out
}

/// §106/§431：失败/相关的 CheckRun（M8 复用，§52；output 只 excerpt，§107）。
fn verification_candidates(inputs: &FactsInputs) -> Vec<ContextCandidate> {
    let Some(verification) = inputs.verification else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for run in &verification.runs {
        let failed = run.state != prism_verify::CheckState::Passed;
        if !failed {
            continue; // §431：优先 failed；passed 仅在 Full scope 需要（v1 略）。
        }
        let excerpt = excerpt_from_output(&run.stderr, &run.stdout);
        out.push(ContextCandidate {
            source: ContextSource {
                source_type: SourceType::CheckRun,
                source_id: run.run_id.clone(),
                path: None,
                location: None,
                provenance: format!(
                    "M8 CheckRun {} at {}",
                    run.check_id,
                    run.finished_at.as_deref().unwrap_or("in-flight")
                ),
            },
            group: ContextGroup::Verification,
            title: format!("Check failed: {}", run.command_display),
            reasons: vec![SelectionReason::FailedVerification],
            evidence: vec![
                format!("state={:?} exit={:?}", run.state, run.exit_code),
                excerpt,
            ],
            impact_distance: None,
            change_state: None,
            language: None,
            symbol_range: None,
            symbol_id: None,
            symbol_name: None,
            severity: Some("Warning".to_string()),
            change_type: None,
            line_count_estimate: None,
        });
    }
    out
}

/// §104/§432：只带 required/失败的环境需求（不 dump 全部）。
fn environment_candidates(inputs: &FactsInputs) -> Vec<ContextCandidate> {
    let Some(environment) = inputs.environment else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for matched in &environment.matches {
        if matched.result != prism_env::MatchResult::Unsatisfied
            && matched.result != prism_env::MatchResult::Unknown
        {
            continue;
        }
        out.push(candidate(
            SourceType::EnvironmentFact,
            format!("env:{}", matched.requirement_id),
            None,
            "M9 EnvironmentSnapshot requirement match",
            ContextGroup::Environment,
            format!(
                "Environment {}: {}",
                matched.result.as_str(),
                matched.subject
            ),
            vec![SelectionReason::RequiredEnvironment],
            vec![matched.evidence.clone()],
        ));
    }
    for finding in &environment.findings {
        if finding.severity == prism_core::Severity::Info {
            continue;
        }
        out.push(candidate(
            SourceType::EnvironmentFact,
            finding.id.clone(),
            None,
            "M9 EnvironmentSnapshot finding",
            ContextGroup::Environment,
            finding.message.clone(),
            vec![SelectionReason::RequiredEnvironment],
            finding.evidence.clone(),
        ));
    }
    out
}

/// §54–§60/§103/§176–§177：文档与 instruction（path-scoped，含层级优先）。
fn documentation_candidates(inputs: &FactsInputs) -> Vec<ContextCandidate> {
    let Some(discovery) = inputs.discovery else {
        return Vec::new();
    };
    // 目标路径目录前缀（§58/§59 path-scoped instructions）。
    let target_dirs: Vec<String> = match &inputs.target.resolution {
        TargetResolution::Resolved { paths, .. } => paths
            .iter()
            .filter_map(|path| path.rsplit_once('/').map(|(dir, _)| dir.to_string()))
            .collect(),
        _ => Vec::new(),
    };
    let instruction_names = ["AGENTS.md", "CLAUDE.md"];
    let doc_priorities = [
        "CONTRIBUTING.md", "BUILD.md", "DEVELOPMENT.md", "ARCHITECTURE.md", "README.md",
    ];
    let mut out = Vec::new();
    for file in &discovery.files {
        let path = file.relative_path.as_str();
        let file_name = path.rsplit('/').next().unwrap_or(path);
        let is_instruction = instruction_names.contains(&file_name);
        let is_known_doc = doc_priorities.contains(&file_name)
            || path.starts_with("docs/")
                && file.language == Some(prism_project::Language::Markdown);
        if !is_instruction && !is_known_doc {
            continue;
        }
        let mut reasons = Vec::new();
        let mut evidence = vec![format!("detected as {}", if is_instruction { "instruction" } else { "documentation" })];
        if is_instruction {
            reasons.push(SelectionReason::ProjectInstruction);
        } else {
            reasons.push(SelectionReason::RelevantDocumentation);
        }
        // §59/§60：目标目录下的 instruction 更具体（specificity evidence）。
        if let Some(dir) = path.rsplit_once('/').map(|(dir, _)| dir.to_string()) {
            if target_dirs.contains(&dir) {
                evidence.push(format!("path-scoped to target directory {dir} (more specific)"));
            }
        }
        // §57/§103：instruction 冲突不择一——selector 保留全部来源（evidence 注明）。
        if is_instruction {
            evidence.push("instructions are data, never executed; conflicts retained".to_string());
        }
        out.push(candidate(
            SourceType::Documentation,
            format!("doc:{path}"),
            Some(path.to_string()),
            if is_instruction { "project instructions" } else { "documentation" },
            ContextGroup::Documentation,
            format!("Doc: {path}"),
            reasons,
            evidence,
        ));
    }
    out
}

/// §439/§440：目标相关架构概览（M3 facts；§441 不给架构建议）。
fn architecture_candidates(inputs: &FactsInputs) -> Vec<ContextCandidate> {
    let Some(graph) = inputs.graph else {
        return Vec::new();
    };
    // 概览级：模块数/依赖数/环数（来自 M3 summary/diagnostics；轻量、有界）。
    let mut evidence = vec![format!(
        "modules={} dependencies={}",
        graph.modules.len(),
        graph.dependencies.len()
    )];
    let cycle_count = graph.cycles.len();
    if cycle_count > 0 {
        evidence.push(format!("{cycle_count} dependency cycle(s) detected (M3 fact)"));
    }
    vec![candidate(
        SourceType::ArchitectureNode,
        "architecture:overview".to_string(),
        None,
        "M3 DependencyGraphResult summary",
        ContextGroup::Dependencies,
        "Architecture overview (M3 facts only)".to_string(),
        vec![SelectionReason::ArchitectureOverview],
        evidence,
    )]
}

/// §18/§29：项目概览（Unknown task 的 Generic overview 基座）。
fn project_candidates(inputs: &FactsInputs) -> Vec<ContextCandidate> {
    let Some(discovery) = inputs.discovery else {
        return Vec::new();
    };
    vec![candidate(
        SourceType::ProjectMetadata,
        "project:overview".to_string(),
        None,
        "M1 ProjectDiscovery",
        ContextGroup::Project,
        format!(
            "Project: {} ({} files, {} languages)",
            discovery.project.name,
            discovery.statistics.total_files,
            discovery.languages.len()
        ),
        vec![SelectionReason::TaskMatch],
        vec![
            format!("root={}", discovery.project.root_path.to_string_lossy()),
            format!(
                "entry_points={}",
                discovery.entry_points.len()
            ),
        ],
    )]
}

fn relevant_paths_from(inputs: &FactsInputs) -> Vec<String> {
    let mut paths = Vec::new();
    if let TargetResolution::Resolved { paths: target_paths, .. } = &inputs.target.resolution {
        paths.extend(target_paths.iter().cloned());
    }
    if let Some(changeset) = inputs.changeset {
        for file in &changeset.files {
            if let Some(path) = file.new_path.as_ref().or(file.old_path.as_ref()) {
                paths.push(path.clone());
            }
        }
    }
    paths.sort();
    paths.dedup();
    paths
}

/// §107/§207：输出 excerpt——错误区域优先，有界（不做全文搬运）。
fn excerpt_from_output(stderr: &str, stdout: &str) -> String {
    let combined = if stderr.trim().is_empty() { stdout } else { stderr };
    let lines: Vec<&str> = combined
        .lines()
        .filter(|line| {
            let lower = line.to_lowercase();
            lower.contains("error")
                || lower.contains("fail")
                || lower.contains("assert")
                || lower.contains("expect")
                || lower.contains("panic")
        })
        .take(8)
        .collect();
    if lines.is_empty() {
        let head: Vec<&str> = combined.lines().take(6).collect();
        format!("excerpt: {}", head.join(" | "))
    } else {
        format!("excerpt: {}", lines.join(" | "))
    }
}
