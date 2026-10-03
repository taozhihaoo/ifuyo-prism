//! Findings 引擎（规格 M9b §51–§53/§128–§137/§350–§359）。
//!
//! findings 来自 matcher / tool discovery / path analysis / port analysis /
//! config conflict（§128），不是随意业务规则。无分数、无 "Environment
//! Ready"（§136/§137/§352）；status 从具体 matches/findings 确定性聚合
//! （§354–§358，无隐藏权重 §359）。

use crate::model::{
    EnvironmentFinding, EnvironmentFindingKind, EnvironmentSnapshot, EnvironmentStatus,
    FindingCategory, FindingConfidence, MatchResult, RequirementKind,
};
use prism_core::Severity;

/// 从快照派生 findings（§128 输入全部来自既有事实，不新增探测）。
pub fn derive_findings(snapshot: &EnvironmentSnapshot) -> Vec<EnvironmentFinding> {
    let mut findings = Vec::new();

    // ---- matcher 结果 → findings（§130/§131/§134/§327）----
    for matched in &snapshot.matches {
        let requirement = snapshot
            .requirements
            .iter()
            .find(|requirement| requirement.id == matched.requirement_id);
        let source = requirement
            .map(|requirement| requirement.source.clone())
            .unwrap_or_default();
        let source_kind = requirement.map(|requirement| requirement.source_kind);
        if matched.result != MatchResult::Satisfied && matched.result != MatchResult::NotApplicable
        {
            let (kind, category, severity) = match requirement.map(|r| r.kind) {
                Some(RequirementKind::PackageManager) => (
                    // §105/§131：缺失（lookup 确认）⇒ MissingPackageManager；
                    // 装了但版本不符 ⇒ VersionMismatch。
                    if matched.evidence.contains("not found") {
                        EnvironmentFindingKind::MissingPackageManager
                    } else {
                        EnvironmentFindingKind::VersionMismatch
                    },
                    FindingCategory::PackageManager,
                    Severity::Warning,
                ),
                Some(RequirementKind::Sdk) => (
                    EnvironmentFindingKind::MissingSdk,
                    FindingCategory::Sdk,
                    Severity::Warning,
                ),
                Some(RequirementKind::Runtime | RequirementKind::Tool) => (
                    // §51/§370：lookup 确认缺失 ⇒ ToolMissing（如 Docker 项目
                    // 无 docker 可执行）；版本不符 ⇒ VersionMismatch。
                    if matched.evidence.contains("absent") || matched.evidence.contains("not found")
                    {
                        EnvironmentFindingKind::ToolMissing
                    } else {
                        EnvironmentFindingKind::VersionMismatch
                    },
                    FindingCategory::Toolchain,
                    Severity::Warning,
                ),
                Some(RequirementKind::EnvironmentVariable) => (
                    EnvironmentFindingKind::MissingEnvironmentVariable,
                    FindingCategory::Variable,
                    Severity::Warning,
                ),
                Some(RequirementKind::Port) => (
                    EnvironmentFindingKind::PortConflict,
                    FindingCategory::Port,
                    Severity::Warning,
                ),
                _ => (
                    EnvironmentFindingKind::UnexpectedEnvironment,
                    FindingCategory::Configuration,
                    Severity::Info,
                ),
            };
            let confidence = match matched.result {
                MatchResult::Unsatisfied => FindingConfidence::Exact,
                MatchResult::Unknown => FindingConfidence::Unknown,
                _ => FindingConfidence::Ambiguous,
            };
            let suggestion = match kind {
                EnvironmentFindingKind::MissingPackageManager => Some(
                    "Use the project's configured package manager (no auto-install).".to_string(),
                ),
                EnvironmentFindingKind::VersionMismatch => requirement.map(|r| {
                    format!(
                        "Align the environment with the requirement ({}) — suggestion only.",
                        r.expected
                    )
                }),
                EnvironmentFindingKind::PortConflict => Some(
                    "Free the port or change the project's expected port (no process kill)."
                        .to_string(),
                ),
                EnvironmentFindingKind::RequirementConflict => Some(
                    "Resolve the conflicting sources, then refresh the environment scan."
                        .to_string(),
                ),
                _ => None,
            };
            findings.push(EnvironmentFinding {
                id: format!("envf:{:?}:{}", kind, matched.subject),
                kind,
                category,
                severity,
                confidence,
                subject: matched.subject.clone(),
                message: format!(
                    "{} requirement {}: {}",
                    category_label(category),
                    matched.result.as_str(),
                    matched.evidence
                ),
                evidence: {
                    let mut evidence = vec![matched.evidence.clone()];
                    if !source.is_empty() {
                        evidence.insert(0, format!("required by {source}"));
                    }
                    if let Some(kind) = source_kind {
                        evidence.insert(0, format!("source layer: {}", kind.as_str()));
                    }
                    evidence
                },
                suggestion,
            });
        }
    }

    // ---- 需求冲突（§45/§327 RequirementConflict finding）----
    for conflict in &snapshot.conflicts {
        findings.push(EnvironmentFinding {
            id: format!("envf:requirement_conflict:{}", conflict.subject),
            kind: EnvironmentFindingKind::RequirementConflict,
            category: FindingCategory::Configuration,
            severity: Severity::Warning,
            confidence: FindingConfidence::Exact,
            subject: conflict.subject.clone(),
            message: format!(
                "Requirement conflict — {} cannot be interpreted as a single unambiguous requirement.",
                conflict.subject
            ),
            evidence: conflict
                .entries
                .iter()
                .map(|entry| format!(
                    "{} ({}): {} {}",
                    entry.source,
                    entry.source_kind.as_str(),
                    entry.operator.as_str(),
                    entry.expected
                ))
                .collect(),
            suggestion: Some(
                "Resolve the conflicting sources, then refresh the environment scan.".to_string(),
            ),
        });
    }

    // ---- PATH 分析（§132/§373：Informational/Warning，不修改 PATH）----
    if !snapshot.paths.duplicates.is_empty() {
        findings.push(EnvironmentFinding {
            id: "envf:path_issue:duplicates".to_string(),
            kind: EnvironmentFindingKind::PathIssue,
            category: FindingCategory::Path,
            severity: Severity::Info,
            confidence: FindingConfidence::Exact,
            subject: "PATH".to_string(),
            message: format!(
                "PATH contains {} duplicate entr{}.",
                snapshot.paths.duplicates.len(),
                if snapshot.paths.duplicates.len() == 1 { "y" } else { "ies" }
            ),
            evidence: snapshot
                .paths
                .duplicates
                .iter()
                .map(|entry| format!("duplicate: {entry}"))
                .collect(),
            suggestion: None,
        });
    }
    if !snapshot.paths.missing.is_empty() {
        findings.push(EnvironmentFinding {
            id: "envf:path_issue:missing".to_string(),
            kind: EnvironmentFindingKind::PathIssue,
            category: FindingCategory::Path,
            severity: Severity::Warning,
            confidence: FindingConfidence::Exact,
            subject: "PATH".to_string(),
            message: format!(
                "PATH contains {} nonexistent director{}.",
                snapshot.paths.missing.len(),
                if snapshot.paths.missing.len() == 1 { "y" } else { "ies" }
            ),
            evidence: snapshot
                .paths
                .missing
                .iter()
                .map(|entry| format!("missing: {entry}"))
                .collect(),
            suggestion: Some("Review PATH entries (no modification by Prism).".to_string()),
        });
    }

    // ---- 同名多候选（§133：context information，Info）----
    for tool in &snapshot.tools {
        if !tool.additional_paths.is_empty() {
            findings.push(EnvironmentFinding {
                id: format!("envf:executable_shadowing:{}", tool.name),
                kind: EnvironmentFindingKind::ExecutableShadowing,
                category: FindingCategory::Toolchain,
                severity: Severity::Info,
                confidence: FindingConfidence::Exact,
                subject: tool.name.clone(),
                message: format!(
                    "Multiple '{}' executables found; active resolution is first in PATH.",
                    tool.name
                ),
                evidence: tool
                    .path
                    .iter()
                    .map(|path| format!("active: {path}"))
                    .chain(
                        tool.additional_paths
                            .iter()
                            .map(|path| format!("also found: {path}")),
                    )
                    .collect(),
                suggestion: None,
            });
        }
    }

    // ---- 期望端口被未知/外部进程占用（§134/§135/§368；§367 同项目 Informational
    //      需要进程关联证据，v1 association 未实现 ⇒ 按 Potential conflict 报告）----
    for expected in &snapshot.expected_ports {
        let occupant = snapshot
            .ports
            .iter()
            .find(|info| info.port == expected.port);
        if let Some(occupant) = occupant {
            findings.push(EnvironmentFinding {
                id: format!("envf:port_conflict:{}", expected.port),
                kind: EnvironmentFindingKind::PortConflict,
                category: FindingCategory::Port,
                severity: Severity::Warning,
                confidence: FindingConfidence::Probable,
                subject: expected.port.to_string(),
                message: format!(
                    "Expected port {} is occupied (potential conflict).",
                    expected.port
                ),
                evidence: vec![
                    format!("expected by {}", expected.source),
                    format!(
                        "observed LISTENING on {} (pid {:?}, process {:?}, observed {})",
                        occupant.address, occupant.pid, occupant.process_name, occupant.observed_at
                    ),
                ],
                suggestion: Some(
                    "If this is the project's own dev server, this is informational — no action."
                        .to_string(),
                ),
            });
        }
    }

    // ---- §59 Git + env 集成：敏感环境文件被跟踪（事实；M7 可据此产生 Health finding）----
    for file in &snapshot.env_files {
        if file.kind == crate::model::EnvFileKind::SecretCandidate
            && file.tracked == Some(true)
        {
            let sensitive = file.keys.iter().filter(|key| key.possibly_sensitive).count();
            findings.push(EnvironmentFinding {
                id: format!("envf:sensitive_file_tracked:{}", file.path),
                kind: EnvironmentFindingKind::SensitiveFileTracked,
                category: FindingCategory::Configuration,
                severity: Severity::Warning,
                confidence: FindingConfidence::Exact,
                subject: file.path.clone(),
                message: "Sensitive environment file appears tracked.".to_string(),
                evidence: vec![
                    format!(
                        "{} is a secret candidate ({} keys, {} possibly sensitive; values never read)",
                        file.path,
                        file.keys.len(),
                        sensitive
                    ),
                    "git status: file is tracked (M4 fact)".to_string(),
                ],
                suggestion: Some(
                    "Consider untracking it and adding it to .gitignore (no action taken by Prism)."
                        .to_string(),
                ),
            });
        }
    }

    findings.sort_by(|a, b| {
        a.category
            .cmp(&b.category)
            .then_with(|| a.subject.cmp(&b.subject))
            .then_with(|| a.kind.as_str().cmp(b.kind.as_str()))
    });
    findings
}

/// 聚合状态（§354–§358，确定性、无权重）：
/// 冲突 > Unsatisfied > 扫描不完整(Partial) > Unknown > Attention(有发现) > Satisfied。
pub fn aggregate_status(
    snapshot: &EnvironmentSnapshot,
    findings: &[EnvironmentFinding],
) -> EnvironmentStatus {
    let has_conflict = !snapshot.conflicts.is_empty()
        || findings
            .iter()
            .any(|finding| finding.kind == EnvironmentFindingKind::RequirementConflict);
    if has_conflict {
        return EnvironmentStatus::Conflict;
    }
    let has_unsatisfied = snapshot
        .matches
        .iter()
        .any(|matched| matched.result == MatchResult::Unsatisfied);
    if has_unsatisfied {
        return EnvironmentStatus::Unsatisfied;
    }
    if snapshot.scan_outcome == crate::model::ScanOutcome::Partial
        || snapshot.scan_outcome == crate::model::ScanOutcome::Cancelled
    {
        return EnvironmentStatus::Partial;
    }
    let has_unknown = snapshot
        .matches
        .iter()
        .any(|matched| matched.result == MatchResult::Unknown);
    if has_unknown {
        return EnvironmentStatus::Unknown;
    }
    let has_warnings = findings
        .iter()
        .any(|finding| finding.severity != Severity::Info);
    if has_warnings {
        return EnvironmentStatus::Attention;
    }
    let has_findings = !findings.is_empty();
    if has_findings {
        return EnvironmentStatus::Attention;
    }
    EnvironmentStatus::Satisfied
}

fn category_label(category: FindingCategory) -> &'static str {
    match category {
        FindingCategory::Toolchain => "Toolchain",
        FindingCategory::Runtime => "Runtime",
        FindingCategory::Sdk => "SDK",
        FindingCategory::PackageManager => "Package manager",
        FindingCategory::Path => "PATH",
        FindingCategory::Variable => "Variable",
        FindingCategory::Port => "Port",
        FindingCategory::Process => "Process",
        FindingCategory::Container => "Container",
        FindingCategory::Configuration => "Configuration",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        Architecture, EnvironmentCompleteness, EnvironmentDiagnostic, HostInfo, OsFamily,
        PathFacts, RequirementMatch, ResourceFacts,
        ScanOutcome, ShellInfo, ShellKind, ENVIRONMENT_SCHEMA_VERSION,
    };

    fn requirement(id: &str, subject: &str, kind: RequirementKind) -> crate::model::EnvironmentRequirement {
        crate::model::EnvironmentRequirement {
            id: id.to_string(),
            kind,
            subject: subject.to_string(),
            operator: crate::model::RequirementOperator::AtLeast,
            expected: "22".to_string(),
            source: "test".to_string(),
            source_kind: crate::model::SourceKind::ProjectConfig,
            evidence: "test".to_string(),
            confidence: prism_core::Confidence::High,
            applicable: true,
        }
    }

    fn snapshot() -> EnvironmentSnapshot {
        EnvironmentSnapshot {
            schema_version: ENVIRONMENT_SCHEMA_VERSION,
            id: "test".to_string(),
            project_id: String::new(),
            project_root: ".".to_string(),
            captured_at: "2026-10-03T00:00:00Z".to_string(),
            host: HostInfo {
                os: OsFamily::Windows,
                os_name: "Windows".to_string(),
                os_version: None,
                architecture: Architecture::X86_64,
                hostname: None,
                username: None,
                locale: None,
                timezone: None,
            },
            shell: ShellInfo { kind: ShellKind::Unknown, executable: None, version: None },
            paths: PathFacts {
                scope: crate::model::FactScope::Process,
                entries: vec![],
                duplicates: vec![],
                missing: vec![],
                truncated: false,
            },
            tools: vec![],
            virtualenvs: vec![],
            runtimes: vec![],
            sdks: vec![],
            package_managers: vec![],
            variables: vec![],
            env_files: vec![],
            ports: vec![],
            processes: vec![],
            expected_ports: vec![],
            resources: ResourceFacts::default(),
            requirements: vec![],
            conflicts: vec![],
            matches: vec![],
            findings: vec![],
            status: EnvironmentStatus::Satisfied,
            diagnostics: Vec::<EnvironmentDiagnostic>::new(),
            scan_outcome: ScanOutcome::Completed,
            completeness: EnvironmentCompleteness::Complete,
        }
    }

    // §362 golden path #3：Node >=22 vs Node 20 ⇒ VersionMismatch + Unsatisfied。
    #[test]
    fn version_mismatch_finding_from_unsatisfied_match() {
        let mut snapshot = snapshot();
        snapshot
            .requirements
            .push(requirement("req:test:node", "node", RequirementKind::Runtime));
        snapshot.matches.push(RequirementMatch {
            requirement_id: "req:test:node".to_string(),
            subject: "node".to_string(),
            result: MatchResult::Unsatisfied,
            evidence: "actual: node 20.1.0 (expected at_least 22)".to_string(),
        });
        let findings = derive_findings(&snapshot);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].kind, EnvironmentFindingKind::VersionMismatch);
        assert_eq!(findings[0].severity, Severity::Warning);
        assert_eq!(findings[0].confidence, FindingConfidence::Exact);
        assert_eq!(aggregate_status(&snapshot, &findings), EnvironmentStatus::Unsatisfied);
    }

    // §131：pnpm missing ⇒ MissingPackageManager + suggestion（不自动安装）。
    #[test]
    fn missing_package_manager_has_suggestion_only() {
        let mut snapshot = snapshot();
        snapshot
            .requirements
            .push(requirement(
                "req:test:pnpm",
                "pnpm",
                RequirementKind::PackageManager,
            ));
        snapshot.matches.push(RequirementMatch {
            requirement_id: "req:test:pnpm".to_string(),
            subject: "pnpm".to_string(),
            result: MatchResult::Unsatisfied,
            evidence: "actual: pnpm not found on this machine (lookup confirmed absence)".to_string(),
        });
        let findings = derive_findings(&snapshot);
        assert_eq!(findings[0].kind, EnvironmentFindingKind::MissingPackageManager);
        let suggestion = findings[0].suggestion.as_deref().unwrap();
        assert!(suggestion.contains("no auto-install"));
    }

    // §355/§356/§357/§358 聚合规则。
    #[test]
    fn status_aggregation_is_deterministic() {
        let mut snapshot = snapshot();
        let findings = derive_findings(&snapshot);
        assert_eq!(aggregate_status(&snapshot, &findings), EnvironmentStatus::Satisfied);

        snapshot.matches.push(RequirementMatch {
            requirement_id: "r1".to_string(),
            subject: "node".to_string(),
            result: MatchResult::Unknown,
            evidence: String::new(),
        });
        assert_eq!(aggregate_status(&snapshot, &findings), EnvironmentStatus::Unknown);
    }

    // §136/§352：结构上无分数。
    #[test]
    fn no_score_field_exists_in_findings_or_status() {
        let json = serde_json::to_string(&EnvironmentStatus::Satisfied).unwrap();
        assert_eq!(json, "\"satisfied\"");
        let finding = EnvironmentFinding {
            id: "x".to_string(),
            kind: EnvironmentFindingKind::PathIssue,
            category: FindingCategory::Path,
            severity: Severity::Info,
            confidence: FindingConfidence::Exact,
            subject: "PATH".to_string(),
            message: "m".to_string(),
            evidence: vec![],
            suggestion: None,
        };
        let serialized = serde_json::to_string(&finding).unwrap();
        assert!(!serialized.contains("score"));
        assert!(!serialized.contains("grade"));
    }

    // §373：PATH missing dir ⇒ PathIssue finding，不改 PATH。
    #[test]
    fn path_issues_become_findings() {
        let mut snapshot = snapshot();
        snapshot.paths.duplicates = vec!["C:\\dup".to_string()];
        snapshot.paths.missing = vec!["C:\\old-node\\bin".to_string()];
        let findings = derive_findings(&snapshot);
        let path_findings: Vec<_> = findings
            .iter()
            .filter(|finding| finding.kind == EnvironmentFindingKind::PathIssue)
            .collect();
        assert_eq!(path_findings.len(), 2);
        assert_eq!(path_findings[0].severity, Severity::Info, "duplicates informational");
        assert_eq!(path_findings[1].severity, Severity::Warning, "missing dirs warning");
        assert_eq!(aggregate_status(&snapshot, &findings), EnvironmentStatus::Attention);
    }
}
