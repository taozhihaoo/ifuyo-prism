//! ContextSelector（规格 M10a §90–§97/§138–§147/§197–§201/§252）。
//!
//! 归一化 → 去重 → 优先级带 → 预算 → 选择。全部确定性（§139）：
//! 排序键 = (priority band, source_type, path, location)，无 HashMap 迭代
//! 顺序、无浮点分（§143–§145）。Critical 预留（§198）：预算耗尽后
//! Critical 仍可入选，绝不静默挤掉 must-include（§197）。

use crate::model::{
    BudgetReport, ContextCandidate, ContextGroup, ContextItem, ContextScope, ContextSource,
    ContextTarget, ExcludedCandidate, ExcludedReason, Priority, SelectionReason, SelectionTrace,
    SourceType, TargetResolution, TaskKind,
};

/// 单次选择输出（item.content 尚未物化——懒加载在 materializer，§202）。
pub struct SelectionOutput {
    pub selected: Vec<ContextItem>,
    pub trace: SelectionTrace,
    pub budget: BudgetReport,
}

/// 预算为零/非法（§200：明确规定为 validation error，不随机行为）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidBudget(pub &'static str);

/// 归一化 + 去重（§91–§97）：身份 = (source_type, path|source_id)。
/// 同一文件被 target/change/impact 同时发现 ⇒ 合并为一个候选，保留多理由。
pub fn normalize(candidates: Vec<ContextCandidate>) -> Vec<ContextCandidate> {
    let mut merged: Vec<ContextCandidate> = Vec::new();
    for candidate in candidates {
        let identity = identity_of(&candidate.source);
        if let Some(existing) = merged
            .iter_mut()
            .find(|item| identity_of(&item.source) == identity)
        {
            for reason in candidate.reasons {
                if !existing.reasons.contains(&reason) {
                    existing.reasons.push(reason);
                }
            }
            for evidence in candidate.evidence {
                if !existing.evidence.contains(&evidence) {
                    existing.evidence.push(evidence);
                }
            }
            // 取更有信息量的元数据（更小距离 / 有符号 range / 有路径）。
            if existing.impact_distance.is_none() {
                existing.impact_distance = candidate.impact_distance;
            }
            if existing.symbol_range.is_none() {
                existing.symbol_range = candidate.symbol_range;
                existing.symbol_id = candidate.symbol_id.clone();
                existing.symbol_name = candidate.symbol_name.clone();
            }
            if existing.source.path.is_none() {
                existing.source.path = candidate.source.path.clone();
            }
        } else {
            merged.push(candidate);
        }
    }
    merged
}

fn identity_of(source: &ContextSource) -> String {
    match &source.path {
        Some(path) => format!("{:?}|file|{path}", source.source_type),
        None => format!("{:?}|id|{}", source.source_type, source.source_id),
    }
}

/// Scope 过滤（§82–§86）——范围外 ⇒ OutsideScope 排除记录（§141）。
pub fn apply_scope(
    candidates: Vec<ContextCandidate>,
    scope: ContextScope,
    target: &ContextTarget,
) -> (Vec<ContextCandidate>, Vec<ExcludedCandidate>) {
    let target_paths: Vec<String> = match &target.resolution {
        TargetResolution::Resolved { paths, .. } => paths.clone(),
        _ => Vec::new(),
    };
    let mut kept = Vec::new();
    let mut excluded = Vec::new();
    for candidate in candidates {
        let in_scope = match scope {
            ContextScope::Full | ContextScope::Auto => true,
            ContextScope::Target => match &candidate.source.path {
                Some(path) => {
                    target_paths.is_empty()
                        || target_paths.iter().any(|target_path| {
                            path == target_path || path.starts_with(target_path.as_str())
                        })
                }
                None => matches!(
                    candidate.source.source_type,
                    SourceType::ProjectMetadata | SourceType::ArchitectureNode
                ),
            },
            ContextScope::CurrentChange => matches!(
                candidate.group,
                ContextGroup::CurrentChange
                    | ContextGroup::Impact
                    | ContextGroup::Tests
                    | ContextGroup::Health
                    | ContextGroup::Verification
            ),
        };
        if in_scope {
            kept.push(candidate);
        } else {
            excluded.push(ExcludedCandidate {
                id: identity_of(&candidate.source),
                title: candidate.title.clone(),
                source: candidate.source.clone(),
                reason: ExcludedReason::OutsideScope,
                detail: format!("outside {} scope", crate::model::scope_str(scope)),
            });
        }
    }
    (kept, excluded)
}

/// 优先级带（§144 规则化；§26/§27 risk signals 只改带，不出分数）。
pub fn prioritize(
    candidate: &ContextCandidate,
    classification: &crate::model::TaskClassification,
) -> Priority {
    let reasons = &candidate.reasons;
    if reasons.contains(&SelectionReason::ExplicitTaskTarget) {
        return Priority::Critical;
    }
    // §99：review 任务 ⇒ 当前 diff Critical。
    if reasons.contains(&SelectionReason::CurrentDiff)
        && classification.kinds.contains(&TaskKind::Review)
    {
        return Priority::Critical;
    }
    // §106/§376：失败校验对 bug/test 任务是 Critical，其余 High。
    if reasons.contains(&SelectionReason::FailedVerification) {
        if classification
            .kinds
            .iter()
            .any(|kind| matches!(kind, TaskKind::BugInvestigation | TaskKind::Test))
        {
            return Priority::Critical;
        }
        return Priority::High;
    }
    if reasons.contains(&SelectionReason::DirectlyChanged) {
        return Priority::High;
    }
    if reasons.contains(&SelectionReason::HealthFinding)
        && candidate.severity.as_deref() == Some("Critical")
    {
        return Priority::High;
    }
    if reasons.contains(&SelectionReason::DirectlyImpacted) {
        // §45：距离决定 Direct/High/Normal。
        return match candidate.impact_distance {
            Some(distance) if distance <= 1 => Priority::High,
            Some(distance) if distance <= 2 => Priority::Normal,
            _ => Priority::Low,
        };
    }
    if reasons.contains(&SelectionReason::HealthFinding)
        || reasons.contains(&SelectionReason::RelatedTest)
        || reasons.contains(&SelectionReason::ReverseDependency)
        || reasons.contains(&SelectionReason::Dependency)
    {
        return Priority::Normal;
    }
    if reasons.contains(&SelectionReason::ProjectInstruction)
        || reasons.contains(&SelectionReason::RelevantDocumentation)
        || reasons.contains(&SelectionReason::RequiredEnvironment)
        || reasons.contains(&SelectionReason::ArchitectureOverview)
    {
        return Priority::Supporting;
    }
    Priority::Low
}

/// 候选预算估算（§201：物化**前**估算，不读内容）。
pub fn estimate_candidate(candidate: &ContextCandidate) -> u64 {
    candidate
        .line_count_estimate
        .map(|lines| lines * 80)
        .unwrap_or(match candidate.source.source_type {
            SourceType::File | SourceType::Symbol => 4 * 1024,
            SourceType::Documentation => 6 * 1024,
            _ => 1024,
        })
}

/// 主选择流程（§90 Selection stage；§197–§198 预算预留）。
pub fn select(
    candidates: Vec<ContextCandidate>,
    budget: &crate::model::ContextBudget,
    classification: &crate::model::TaskClassification,
) -> Result<SelectionOutput, InvalidBudget> {
    if budget.max_tokens_estimate == 0 {
        // §200：明确规定为 validation error（另一合法选项 metadata-only 未采用）。
        return Err(InvalidBudget(
            "max_tokens_estimate must be > 0 (zero budget is rejected, not random behavior)",
        ));
    }
    let normalized = normalize(candidates);

    // 附加 band + 稳定排序键（§139/§140）。
    let mut ranked: Vec<(Priority, ContextCandidate)> = normalized
        .into_iter()
        .map(|candidate| {
            let priority = prioritize(&candidate, classification);
            (priority, candidate)
        })
        .collect();
    ranked.sort_by(|(priority_a, candidate_a), (priority_b, candidate_b)| {
        priority_a
            .cmp(priority_b)
            .then_with(|| {
                candidate_a
                    .source
                    .source_type
                    .as_str()
                    .cmp(candidate_b.source.source_type.as_str())
            })
            .then_with(|| {
                candidate_a
                    .source
                    .path
                    .clone()
                    .unwrap_or_default()
                    .cmp(&candidate_b.source.path.clone().unwrap_or_default())
            })
            .then_with(|| {
                candidate_a
                    .source
                    .location
                    .clone()
                    .unwrap_or_default()
                    .cmp(&candidate_b.source.location.clone().unwrap_or_default())
            })
    });

    let mut selected: Vec<ContextItem> = Vec::new();
    let mut excluded: Vec<ExcludedCandidate> = Vec::new();
    let mut used_bytes: u64 = 0;
    let mut used_files: std::collections::BTreeSet<String> = Default::default();

    for (priority, candidate) in ranked {
        if selected.len() >= budget.max_items {
            excluded.push(excluded_candidate(
                &candidate,
                ExcludedReason::LowPriority,
                "max_items reached",
            ));
            continue;
        }
        if let Some(path) = &candidate.source.path {
            if used_files.len() >= budget.max_source_files && !used_files.contains(path) {
                excluded.push(excluded_candidate(
                    &candidate,
                    ExcludedReason::BudgetExceeded,
                    "max_source_files reached",
                ));
                continue;
            }
        }
        let estimate = estimate_candidate(&candidate);
        let over_budget = used_bytes + estimate > budget.max_bytes
            || (used_bytes + estimate) / 4 > budget.max_tokens_estimate;
        // §198：Critical 预留——预算耗尽时 Critical 仍入选（带 truncation 预期），
        // 其余 band 记 BudgetExceeded（§141）。
        if over_budget && priority != Priority::Critical {
            excluded.push(excluded_candidate(
                &candidate,
                ExcludedReason::BudgetExceeded,
                format!("estimate {estimate}B exceeds remaining budget"),
            ));
            continue;
        }
        if let Some(path) = &candidate.source.path {
            used_files.insert(path.clone());
        }
        used_bytes += estimate;
        let trust_level = trust_for(&candidate);
        selected.push(ContextItem {
            id: identity_of(&candidate.source),
            source: candidate.source.clone(),
            group: candidate.group,
            title: candidate.title.clone(),
            content: None, // §202 懒物化
            location: candidate.source.location.clone(),
            priority,
            reasons: candidate.reasons.clone(),
            evidence: candidate.evidence.clone(),
            trust_level,
            language: candidate.language.clone(),
            change_state: candidate.change_state.clone(),
            impact_distance: candidate.impact_distance,
            truncated: false,
            original_size: None,
            included_size: None,
            truncation_reason: None,
            content_level: crate::model::FileContentLevel::MetadataOnly,
            detail_level: detail_for(priority),
            ranges: Vec::new(),
            unavailable_reason: None,
            content_hash: None,
        });
    }

    let used_tokens_estimate = used_bytes / 4;
    Ok(SelectionOutput {
        budget: BudgetReport {
            selected_items: selected.len(),
            excluded_items: excluded.len(),
            used_bytes,
            used_tokens_estimate,
            limit_tokens_estimate: budget.max_tokens_estimate,
        },
        trace: SelectionTrace {
            candidates: selected.len() + excluded.len(),
            selected: selected.iter().map(|item| item.id.clone()).collect(),
            excluded,
        },
        selected,
    })
}

/// §188–§191：按来源/理由映射信任层级（不 flatten）。
fn trust_for(candidate: &ContextCandidate) -> crate::model::TrustLevel {
    use crate::model::TrustLevel;
    if candidate
        .reasons
        .contains(&SelectionReason::ProjectInstruction)
        || candidate
            .reasons
            .contains(&SelectionReason::RelevantDocumentation)
    {
        return TrustLevel::Documentation;
    }
    if candidate
        .reasons
        .contains(&SelectionReason::FailedVerification)
    {
        return TrustLevel::ObservedToolOutput;
    }
    if candidate
        .reasons
        .contains(&SelectionReason::RequiredEnvironment)
    {
        return TrustLevel::PrismFact;
    }
    match candidate.source.source_type {
        SourceType::HealthFinding
        | SourceType::ImpactAnalysis
        | SourceType::ChangeSet
        | SourceType::EnvironmentFact
        | SourceType::ProjectMetadata
        | SourceType::ArchitectureNode => TrustLevel::PrismFact,
        SourceType::CheckRun | SourceType::GitCommit | SourceType::GitDiff => {
            TrustLevel::ObservedToolOutput
        }
        SourceType::Documentation => TrustLevel::Documentation,
        SourceType::Configuration => TrustLevel::ProjectConfig,
        SourceType::File | SourceType::Symbol | SourceType::Module | SourceType::Dependency => {
            TrustLevel::PrismFact
        }
    }
}

fn detail_for(priority: Priority) -> crate::model::DetailLevel {
    match priority {
        Priority::Critical => crate::model::DetailLevel::Full,
        Priority::High => crate::model::DetailLevel::Focused,
        Priority::Normal => crate::model::DetailLevel::Focused,
        Priority::Supporting => crate::model::DetailLevel::Structural,
        Priority::Low => crate::model::DetailLevel::Metadata,
    }
}

fn excluded_candidate(
    candidate: &ContextCandidate,
    reason: ExcludedReason,
    detail: impl Into<String>,
) -> ExcludedCandidate {
    ExcludedCandidate {
        id: identity_of(&candidate.source),
        title: candidate.title.clone(),
        source: candidate.source.clone(),
        reason,
        detail: detail.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::TaskClassification;

    fn classification(kinds: &[TaskKind]) -> TaskClassification {
        TaskClassification {
            kinds: kinds.to_vec(),
            focus: vec![],
            specificity: crate::model::TaskSpecificity::Specific,
            raw_task: None,
        }
    }

    fn candidate(
        source_type: SourceType,
        path: Option<&str>,
        reasons: Vec<SelectionReason>,
    ) -> ContextCandidate {
        ContextCandidate {
            source: ContextSource {
                source_type,
                source_id: format!("test:{path:?}"),
                path: path.map(str::to_string),
                location: None,
                provenance: "test".to_string(),
            },
            group: crate::model::ContextGroup::TargetCode,
            title: format!("cand {path:?}"),
            reasons,
            evidence: vec![],
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

    // §91/§92/§93：同文件多来源 ⇒ 合并一个候选 + 多理由。
    #[test]
    fn normalize_merges_same_file_with_multiple_reasons() {
        let merged = normalize(vec![
            candidate(
                SourceType::File,
                Some("src/a.ts"),
                vec![SelectionReason::ExplicitTaskTarget],
            ),
            candidate(
                SourceType::File,
                Some("src/a.ts"),
                vec![SelectionReason::DirectlyChanged],
            ),
            candidate(
                SourceType::File,
                Some("src/b.ts"),
                vec![SelectionReason::DirectlyChanged],
            ),
        ]);
        assert_eq!(merged.len(), 2);
        let a = merged
            .iter()
            .find(|item| item.source.path.as_deref() == Some("src/a.ts"))
            .unwrap();
        assert!(a.reasons.contains(&SelectionReason::ExplicitTaskTarget));
        assert!(a.reasons.contains(&SelectionReason::DirectlyChanged));
    }

    // §144：规则化 band（无浮点）。
    #[test]
    fn priority_bands_follow_spec_rules() {
        let classification = classification(&[TaskKind::BugInvestigation]);
        assert_eq!(
            prioritize(
                &candidate(
                    SourceType::File,
                    Some("t.ts"),
                    vec![SelectionReason::ExplicitTaskTarget]
                ),
                &classification
            ),
            Priority::Critical
        );
        assert_eq!(
            prioritize(
                &candidate(
                    SourceType::ChangeSet,
                    Some("c.ts"),
                    vec![SelectionReason::DirectlyChanged]
                ),
                &classification
            ),
            Priority::High
        );
        let mut impacted = candidate(
            SourceType::ImpactAnalysis,
            Some("i.ts"),
            vec![SelectionReason::DirectlyImpacted],
        );
        impacted.impact_distance = Some(3);
        assert_eq!(prioritize(&impacted, &classification), Priority::Low);
        assert_eq!(
            prioritize(
                &candidate(
                    SourceType::Documentation,
                    Some("README.md"),
                    vec![SelectionReason::RelevantDocumentation]
                ),
                &classification
            ),
            Priority::Supporting
        );
    }

    // §198/§141：预算耗尽 ⇒ BudgetExceeded 排除记录；Critical 保留。
    #[test]
    fn budget_reserves_critical_and_records_exclusions() {
        let classification = classification(&[TaskKind::BugInvestigation]);
        let candidates = vec![
            candidate(
                SourceType::File,
                Some("target.ts"),
                vec![SelectionReason::ExplicitTaskTarget],
            ),
            candidate(
                SourceType::Documentation,
                Some("docs/a.md"),
                vec![SelectionReason::RelevantDocumentation],
            ),
        ];
        let budget = crate::model::ContextBudget {
            max_items: 10,
            max_source_files: 10,
            max_bytes: 5 * 1024,
            max_tokens_estimate: 5 * 1024 / 4,
        };
        let output = select(candidates, &budget, &classification).unwrap();
        assert_eq!(output.selected.len(), 1);
        assert_eq!(output.selected[0].priority, Priority::Critical);
        assert_eq!(output.trace.excluded.len(), 1);
        assert_eq!(
            output.trace.excluded[0].reason,
            ExcludedReason::BudgetExceeded
        );
    }

    // §200：零预算 ⇒ 明确 validation error。
    #[test]
    fn zero_budget_is_validation_error() {
        let classification = classification(&[TaskKind::Unknown]);
        let budget = crate::model::ContextBudget {
            max_tokens_estimate: 0,
            ..crate::model::ContextBudget::default()
        };
        let result = select(
            vec![candidate(
                SourceType::File,
                Some("a.ts"),
                vec![SelectionReason::DirectlyChanged],
            )],
            &budget,
            &classification,
        );
        assert!(result.is_err(), "§200 zero budget => validation error");
    }

    // §139/§140：稳定排序（同 band 内按 source_type/path tie-break）。
    #[test]
    fn deterministic_ordering_with_tie_breaker() {
        let classification = classification(&[TaskKind::Change]);
        let candidates = vec![
            candidate(
                SourceType::Documentation,
                Some("z.md"),
                vec![SelectionReason::RelevantDocumentation],
            ),
            candidate(
                SourceType::ChangeSet,
                Some("a.ts"),
                vec![SelectionReason::DirectlyChanged],
            ),
            candidate(
                SourceType::ChangeSet,
                Some("b.ts"),
                vec![SelectionReason::DirectlyChanged],
            ),
        ];
        let output = select(
            candidates.clone(),
            &crate::model::ContextBudget::default(),
            &classification,
        )
        .unwrap();
        let order: Vec<String> = output.selected.iter().map(|item| item.id.clone()).collect();
        let rerun = select(
            candidates,
            &crate::model::ContextBudget::default(),
            &classification,
        )
        .unwrap();
        let rerun_order: Vec<String> = rerun.selected.iter().map(|item| item.id.clone()).collect();
        assert_eq!(order, rerun_order, "§138/§220 reproducible selection");
    }
}
