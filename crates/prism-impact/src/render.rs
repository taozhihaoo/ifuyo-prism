//! ImpactAnalysis 的 CLI 渲染（text / markdown，规格 M6 §89、§136–§137、§74/§131）。
//!
//! 只渲染已有事实；不确定性显式呈现，不隐藏（§131）。

use crate::model::{ImpactAnalysis, ImpactCompleteness, ImpactKind, ImpactReason, ImpactableNode};
use std::collections::BTreeMap;

fn completeness_label(completeness: ImpactCompleteness) -> &'static str {
    match completeness {
        ImpactCompleteness::Complete => "complete",
        ImpactCompleteness::Partial => "partial (some mappings unavailable)",
        ImpactCompleteness::Truncated => "truncated (limits reached)",
        ImpactCompleteness::Failed => "failed",
    }
}

/// 纯文本渲染（§89：Changed / Potentially affected / Analysis 三段）。
pub fn render_text(analysis: &ImpactAnalysis) -> String {
    let mut out = String::new();
    let summary = &analysis.summary;
    out.push_str(&format!("ImpactAnalysis: {}\n", analysis.id));
    out.push_str(&format!(
        "ChangeSet: {}  Scope: {}  Completeness: {}\n",
        analysis.change_set_id,
        analysis.scope,
        completeness_label(analysis.analysis_completeness)
    ));
    out.push_str(&format!(
        "Changed: {} files, {} symbols, {} modules\n",
        summary.changed_files, summary.changed_symbols, summary.changed_modules
    ));
    out.push_str(&format!(
        "Potentially affected: {} modules, {} symbols, {} tests, {} docs\n",
        summary.direct_impacted_modules + summary.transitive_impacted_modules,
        summary.direct_impacted_symbols + summary.transitive_impacted_symbols,
        summary.potentially_affected_tests,
        summary.documentation_candidates
    ));
    out.push_str(&format!(
        "Analysis: direct {}/{} transitive {}/{}; max distance {}; unresolved {}; ambiguous {}\n",
        summary.direct_impacted_modules,
        summary.direct_impacted_symbols,
        summary.transitive_impacted_modules,
        summary.transitive_impacted_symbols,
        summary.max_distance,
        summary.unresolved_relationships,
        summary.ambiguous_relationships
    ));

    // Top 直接影响（§137：默认只展示头部，不倾倒全图）。
    let direct: Vec<_> = analysis
        .impacted_items
        .iter()
        .filter(|item| item.distance == 1)
        .take(10)
        .collect();
    if !direct.is_empty() {
        out.push_str("Top direct impacts:\n");
        for item in direct {
            out.push_str(&format!(
                "  ~ {} {} ({}) L-{}\n",
                item.node_kind.as_str(),
                item.name,
                item.impact_kinds
                    .iter()
                    .map(|kind| kind_str(*kind))
                    .collect::<Vec<_>>()
                    .join(","),
                item.distance
            ));
        }
    }

    if !analysis.risk_signals.is_empty() {
        out.push_str("Risk signals (facts, no score):\n");
        for signal in &analysis.risk_signals {
            out.push_str(&format!(
                "  [{:?}] count={} {}\n",
                signal.kind, signal.count, signal.detail
            ));
        }
    }
    if !analysis.diagnostics.is_empty() {
        out.push_str("Analysis limitations:\n");
        for diagnostic in &analysis.diagnostics {
            out.push_str(&format!(
                "  [{}] {} {}\n",
                diagnostic.kind.as_str(),
                diagnostic.subject.as_deref().unwrap_or("-"),
                diagnostic.detail
            ));
        }
    }
    out
}

/// Markdown 渲染（§136 分区）。
pub fn render_markdown(analysis: &ImpactAnalysis) -> String {
    let mut out = String::new();
    let summary = &analysis.summary;
    out.push_str(&format!("# ImpactAnalysis `{}`\n\n", analysis.id));
    out.push_str(&format!(
        "- ChangeSet: `{}`\n- Scope: `{}`\n- Completeness: **{}**\n\n",
        analysis.change_set_id,
        analysis.scope,
        completeness_label(analysis.analysis_completeness)
    ));

    out.push_str("## Summary\n\n");
    out.push_str(&format!(
        "| changed files | changed symbols | changed modules |\n|---|---|---|\n| {} | {} | {} |\n\n",
        summary.changed_files, summary.changed_symbols, summary.changed_modules
    ));
    out.push_str(&format!(
        "Potentially affected: {} modules, {} symbols, {} tests, {} docs; max distance {}\n\n",
        summary.direct_impacted_modules + summary.transitive_impacted_modules,
        summary.direct_impacted_symbols + summary.transitive_impacted_symbols,
        summary.potentially_affected_tests,
        summary.documentation_candidates,
        summary.max_distance
    ));

    // 按节点种类分组列出影响（§136 Impact 分区；§137 有界展示）。
    let mut by_kind: BTreeMap<String, Vec<&ImpactableNode>> = BTreeMap::new();
    for item in &analysis.impacted_items {
        by_kind
            .entry(item.node_kind.as_str().to_string())
            .or_default()
            .push(item);
    }
    out.push_str("## Impact\n\n");
    for (kind, items) in &by_kind {
        out.push_str(&format!("### {kind} ({})\n\n", items.len()));
        for item in items.iter().take(50) {
            out.push_str(&format!(
                "- `{}` — distance {} · {} · {}\n",
                item.node_id,
                item.distance,
                item.confidence.as_str(),
                item.reasons
                    .iter()
                    .map(|reason| reason_str(*reason))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        if items.len() > 50 {
            out.push_str(&format!(
                "- … {} more (see JSON for full list)\n",
                items.len() - 50
            ));
        }
        out.push('\n');
    }

    out.push_str("## Impact Paths\n\n");
    if analysis.impact_paths.is_empty() {
        out.push_str("_none_\n\n");
    }
    for path in analysis.impact_paths.iter().take(30) {
        out.push_str(&format!(
            "- `{}` → `{}` (distance {})\n",
            path.source_change, path.target, path.distance
        ));
        for hop in &path.hops {
            out.push_str(&format!(
                "  - `{}` --{}({})--> `{}`\n",
                hop.from,
                relation_str(hop.relation),
                hop.resolution,
                hop.to
            ));
        }
    }
    out.push('\n');

    out.push_str("## Risk Signals\n\n");
    if analysis.risk_signals.is_empty() {
        out.push_str("_none_\n\n");
    }
    for signal in &analysis.risk_signals {
        out.push_str(&format!(
            "- `{:?}` × {} — {}\n",
            signal.kind, signal.count, signal.detail
        ));
    }
    out.push('\n');

    out.push_str("## Analysis Limitations\n\n");
    out.push_str(
        "> Impact analysis is based on resolvable static relationships; some relationships could not be resolved, so results may be incomplete.\n\n",
    );
    if analysis.diagnostics.is_empty() {
        out.push_str("_none_\n");
    } else {
        for diagnostic in &analysis.diagnostics {
            out.push_str(&format!(
                "- `{}` {} — {}\n",
                diagnostic.kind.as_str(),
                diagnostic.subject.as_deref().unwrap_or("-"),
                diagnostic.detail
            ));
        }
    }
    out
}

fn kind_str(kind: ImpactKind) -> &'static str {
    match kind {
        ImpactKind::DirectlyChanged => "directly_changed",
        ImpactKind::DirectDependent => "direct",
        ImpactKind::TransitiveDependent => "transitive",
        ImpactKind::Caller => "caller",
        ImpactKind::TransitiveCaller => "transitive_caller",
        ImpactKind::ModuleConsumer => "module_consumer",
        ImpactKind::ProjectConsumer => "project_consumer",
        ImpactKind::TestCandidate => "test",
        ImpactKind::DocumentationCandidate => "doc",
        ImpactKind::ConfigurationCandidate => "config",
    }
}

fn reason_str(reason: ImpactReason) -> &'static str {
    match reason {
        ImpactReason::CallsChangedSymbol => "calls changed symbol",
        ImpactReason::TransitiveCall => "transitive call",
        ImpactReason::ReferencesChangedSymbol => "references changed symbol",
        ImpactReason::DependsOnChangedModule => "depends on changed module",
        ImpactReason::TransitiveDependency => "transitive dependency",
        ImpactReason::ProjectReference => "project reference",
        ImpactReason::CallsDeletedSymbol => "calls deleted symbol",
        ImpactReason::DocumentsChangedEntity => "documents changed entity",
    }
}

fn relation_str(relation: crate::model::RelationKind) -> &'static str {
    match relation {
        crate::model::RelationKind::Calls => "calls",
        crate::model::RelationKind::DependsOn => "depends_on",
        crate::model::RelationKind::ProjectReferences => "project_references",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        ImpactAnalysis, ImpactCompleteness, ImpactConfidence, ImpactSummary, ImpactableNode,
        NodeKind,
    };

    #[test]
    fn text_and_markdown_render_empty_analysis() {
        let analysis = ImpactAnalysis {
            schema_version: 1,
            id: "impact:changeset:x#10#2000".to_string(),
            change_set_id: "changeset:x".to_string(),
            scope: "working_tree".to_string(),
            analysis_completeness: ImpactCompleteness::Complete,
            analysis_incomplete: false,
            changed_items: vec![],
            impacted_items: vec![],
            impact_paths: vec![],
            summary: ImpactSummary::default(),
            diagnostics: vec![],
            risk_signals: vec![],
        };
        let text = render_text(&analysis);
        assert!(text.contains("ImpactAnalysis: impact:"));
        assert!(text.contains("Potentially affected"));
        let markdown = render_markdown(&analysis);
        assert!(markdown.contains("# ImpactAnalysis"));
        assert!(markdown.contains("Analysis Limitations"));
    }

    #[test]
    fn item_fields_render_in_markdown() {
        let analysis = ImpactAnalysis {
            schema_version: 1,
            id: "impact:x".to_string(),
            change_set_id: "changeset:x".to_string(),
            scope: "working_tree".to_string(),
            analysis_completeness: ImpactCompleteness::Partial,
            analysis_incomplete: false,
            changed_items: vec![],
            impacted_items: vec![ImpactableNode {
                node_id: "symbol:src/a.ts#src.a.caller#function".to_string(),
                node_kind: NodeKind::Symbol,
                impact_kinds: vec![ImpactKind::Caller],
                distance: 1,
                confidence: ImpactConfidence::HighConfidence,
                reasons: vec![ImpactReason::CallsChangedSymbol],
                evidence_path_count: 1,
                additional_paths: 0,
                path: "src/a.ts".to_string(),
                name: "caller".to_string(),
            }],
            impact_paths: vec![],
            summary: ImpactSummary::default(),
            diagnostics: vec![],
            risk_signals: vec![],
        };
        let markdown = render_markdown(&analysis);
        assert!(markdown.contains("partial"));
        assert!(markdown.contains("calls changed symbol"));
        assert!(markdown.contains("high_confidence"));
    }
}
