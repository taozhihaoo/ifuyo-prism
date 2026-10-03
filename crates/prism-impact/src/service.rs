//! ImpactAnalysisService：ChangeSet × ProjectCodeModel × M3 graph → ImpactAnalysis
//! （规格 M6 §58 的分步实现；§124 稳定服务边界，CLI/GUI 同源）。
//!
//! 所有权（§121/§64）：M3 拥有关系事实、M5 拥有变更事实、M6 只解释；
//! 发现关系可疑时记 diagnostic，不私改副本。

use crate::model::{
    ImpactAnalysis, ImpactCompleteness, ImpactConfidence, ImpactDiagnostic, ImpactDiagnosticKind,
    ImpactKind, ImpactOptions, ImpactReason, ImpactSummary, ImpactableNode, NodeKind,
    RiskSignal, RiskSignalKind, IMPACT_SCHEMA_VERSION,
};
use crate::traversal::{build_seeds, traverse, ReverseIndex};
use prism_changes::{ChangeDiagnosticKind, ChangeSet};
use prism_core::task::CancellationToken;
use prism_core::PrismResult;
use prism_graph::DependencyGraphResult;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Default, Clone, Copy)]
pub struct ImpactAnalysisService;

impl ImpactAnalysisService {
    /// 分析入口（§124）：同输入 ⇒ 同输出（确定性，§4/§79）。
    #[allow(clippy::too_many_arguments)]
    pub fn analyze(
        &self,
        change_set: &ChangeSet,
        code_model: &prism_analysis::ProjectCodeModel,
        graph: &DependencyGraphResult,
        options: &ImpactOptions,
        cancel: &CancellationToken,
    ) -> PrismResult<ImpactAnalysis> {
        let index = ReverseIndex::build(graph);
        let seeds = build_seeds(change_set);
        let output = traverse(&seeds, &index, options, cancel);
        if output.cancelled {
            return Err(prism_core::PrismError::Cancelled);
        }

        // ---- 被改条目（§48/§49：与 impacted 严格分离）----
        let changed_items = self.changed_nodes(change_set);

        // ---- 受影响候选 + 测试候选分类（§26/§54）----
        let test_paths = Self::test_file_paths(code_model);
        let mut impacted_items: Vec<ImpactableNode> = Vec::new();
        for accumulator in output.impacted.values() {
            let mut item = ImpactableNode {
                node_id: accumulator.key.id.clone(),
                node_kind: accumulator.key.kind,
                impact_kinds: accumulator.kinds.iter().copied().collect(),
                distance: accumulator.distance,
                confidence: accumulator.confidence,
                reasons: accumulator.reasons.iter().copied().collect(),
                evidence_path_count: accumulator.path_count,
                additional_paths: accumulator.path_count.saturating_sub(1),
                path: accumulator.path.clone(),
                name: accumulator.name.clone(),
            };
            if options.include_tests
                && !accumulator.path.is_empty()
                && test_paths.contains(&accumulator.path)
            {
                item.impact_kinds.push(ImpactKind::TestCandidate);
                item.impact_kinds.sort();
                item.impact_kinds.dedup();
            }
            impacted_items.push(item);
        }
        impacted_items.sort_by(|a, b| {
            (&a.node_kind, &a.node_id).cmp(&(&b.node_kind, &b.node_id))
        });

        // ---- 文档候选（§28/§29：仅存在已知关系证据时；当前 M3 无文档引用边）----
        let mut diagnostics: Vec<ImpactDiagnostic> = Vec::new();
        let documentation_candidates = impacted_items
            .iter()
            .filter(|item| item.node_kind == NodeKind::Documentation)
            .count() as u64;
        if options.include_docs
            && documentation_candidates == 0
            && !change_set.symbols.is_empty()
        {
            diagnostics.push(ImpactDiagnostic {
                kind: ImpactDiagnosticKind::NoDocumentationEvidence,
                subject: None,
                detail: "no resolvable document reference relationships exist in the current graph; documentation candidates are not guessed"
                    .to_string(),
            });
        }

        // ---- 诊断（§128；M5 诊断映射 + 遍历截断 + 解析局限）----
        diagnostics.extend(self.map_change_diagnostics(change_set));
        if output.depth_exceeded {
            diagnostics.push(ImpactDiagnostic {
                kind: ImpactDiagnosticKind::TraversalDepthExceeded,
                subject: None,
                detail: format!("reverse traversal stopped at max_depth={}", options.max_depth),
            });
        }
        if output.node_limit_reached {
            diagnostics.push(ImpactDiagnostic {
                kind: ImpactDiagnosticKind::ImpactNodeLimitReached,
                subject: None,
                detail: format!("impacted node limit reached (max_nodes={})", options.max_nodes),
            });
        }
        if output.unresolved_edges > 0 {
            diagnostics.push(ImpactDiagnostic {
                kind: ImpactDiagnosticKind::UnresolvedRelationship,
                subject: None,
                detail: format!(
                    "{0} unresolved relationship(s) adjacent to changed entities; targets are not fabricated",
                    output.unresolved_edges
                ),
            });
        }
        if output.ambiguous_edges > 0 {
            diagnostics.push(ImpactDiagnostic {
                kind: ImpactDiagnosticKind::AmbiguousRelationship,
                subject: None,
                detail: format!("{0} ambiguous relationship(s) not traversed", output.ambiguous_edges),
            });
        }
        diagnostics.sort_by(|a, b| {
            (&a.kind, &a.subject, &a.detail).cmp(&(&b.kind, &b.subject, &b.detail))
        });

        // ---- 汇总（§129：全为事实计数）----
        let summary = self.summary(change_set, &impacted_items, &output, &diagnostics);

        // ---- 完整度（§127）----
        let analysis_incomplete = output.depth_exceeded || output.node_limit_reached;
        let analysis_completeness = if analysis_incomplete {
            ImpactCompleteness::Truncated
        } else if Self::has_mapping_limitations(change_set) {
            ImpactCompleteness::Partial
        } else {
            ImpactCompleteness::Complete
        };

        // ---- 风险信号（§40/§85/§86：事实计数 + 有界证据，无阈值无总分）----
        let risk_signals = self.risk_signals(
            change_set,
            &changed_items,
            &impacted_items,
            output.unresolved_edges,
            options,
        );

        Ok(ImpactAnalysis {
            schema_version: IMPACT_SCHEMA_VERSION,
            id: ImpactAnalysis::derive_id(&change_set.id, options.max_depth, options.max_nodes),
            change_set_id: change_set.id.clone(),
            scope: change_set.scope.as_str().to_string(),
            analysis_completeness,
            analysis_incomplete,
            changed_items,
            impacted_items,
            impact_paths: output.paths,
            summary,
            diagnostics,
            risk_signals,
        })
    }

    /// 被改节点：文件（含 config 事实标记）、符号、模块（§136 Changed 分区）。
    fn changed_nodes(&self, change_set: &ChangeSet) -> Vec<ImpactableNode> {
        let mut nodes: BTreeMap<String, ImpactableNode> = BTreeMap::new();
        for file in &change_set.files {
            let Some(path) = file.new_path.as_deref().or(file.old_path.as_deref()) else {
                continue;
            };
            let mut kinds = vec![ImpactKind::DirectlyChanged];
            let kind = file
                .new_file_kind
                .or(file.old_file_kind);
            if kind == Some(prism_project::FileKind::Config) {
                kinds.push(ImpactKind::ConfigurationCandidate);
            }
            let name = path.rsplit('/').next().unwrap_or(path).to_string();
            nodes.insert(
                format!("file:{path}"),
                ImpactableNode {
                    node_id: format!("file:{path}"),
                    node_kind: NodeKind::File,
                    impact_kinds: kinds,
                    distance: 0,
                    confidence: ImpactConfidence::Exact,
                    reasons: vec![],
                    evidence_path_count: 1,
                    additional_paths: 0,
                    path: path.to_string(),
                    name,
                },
            );
        }
        for symbol in &change_set.symbols {
            let Some(node_id) = symbol
                .after_symbol_id
                .as_ref()
                .or(symbol.before_symbol_id.as_ref())
            else {
                continue;
            };
            nodes.insert(
                node_id.as_str().to_string(),
                ImpactableNode {
                    node_id: node_id.as_str().to_string(),
                    node_kind: NodeKind::Symbol,
                    impact_kinds: vec![ImpactKind::DirectlyChanged],
                    distance: 0,
                    confidence: ImpactConfidence::Exact,
                    reasons: vec![],
                    evidence_path_count: 1,
                    additional_paths: 0,
                    path: symbol.path.clone(),
                    name: symbol.name.clone(),
                },
            );
        }
        for module in &change_set.modules {
            nodes.insert(
                module.module_id.as_str().to_string(),
                ImpactableNode {
                    node_id: module.module_id.as_str().to_string(),
                    node_kind: NodeKind::FileModule,
                    impact_kinds: vec![ImpactKind::DirectlyChanged],
                    distance: 0,
                    confidence: ImpactConfidence::Exact,
                    reasons: vec![],
                    evidence_path_count: 1,
                    additional_paths: 0,
                    path: module.changed_files.first().cloned().unwrap_or_default(),
                    name: module.module_name.clone(),
                },
            );
        }
        let mut list: Vec<ImpactableNode> = nodes.into_values().collect();
        list.sort_by(|a, b| (&a.node_kind, &a.node_id).cmp(&(&b.node_kind, &b.node_id)));
        list
    }

    fn test_file_paths(code_model: &prism_analysis::ProjectCodeModel) -> BTreeSet<String> {
        code_model
            .files
            .iter()
            .filter(|file| {
                file.classification.kind == prism_project::FileKind::Test
                    || file.classification.kind == prism_project::FileKind::Documentation
            })
            .map(|file| file.relative_path.clone())
            .collect()
    }

    fn has_mapping_limitations(change_set: &ChangeSet) -> bool {
        change_set.diagnostics.iter().any(|diagnostic| {
            matches!(
                diagnostic.kind,
                ChangeDiagnosticKind::ParseFailed
                    | ChangeDiagnosticKind::UnsupportedLanguage
                    | ChangeDiagnosticKind::EncodingUnsupported
                    | ChangeDiagnosticKind::FileTooLarge
                    | ChangeDiagnosticKind::HistoricalModelUnavailable
                    | ChangeDiagnosticKind::SymbolMappingAmbiguous
                    | ChangeDiagnosticKind::SymbolMappingUnavailable
                    | ChangeDiagnosticKind::GitQueryFailed
            )
        })
    }

    /// M5 诊断 → M6 诊断（复用命名约定，§128）。
    fn map_change_diagnostics(&self, change_set: &ChangeSet) -> Vec<ImpactDiagnostic> {
        change_set
            .diagnostics
            .iter()
            .map(|diagnostic| {
                let kind = match diagnostic.kind {
                    ChangeDiagnosticKind::ParseFailed => ImpactDiagnosticKind::ParseFailure,
                    ChangeDiagnosticKind::UnsupportedLanguage
                    | ChangeDiagnosticKind::GeneratedFile => ImpactDiagnosticKind::UnsupportedLanguage,
                    ChangeDiagnosticKind::SymbolMappingAmbiguous
                    | ChangeDiagnosticKind::RenameMappingUncertain => {
                        ImpactDiagnosticKind::AmbiguousRelationship
                    }
                    ChangeDiagnosticKind::SymbolMappingUnavailable
                    | ChangeDiagnosticKind::DeletedFileWithoutBaseline
                    | ChangeDiagnosticKind::HistoricalModelUnavailable
                    | ChangeDiagnosticKind::EncodingUnsupported
                    | ChangeDiagnosticKind::FileTooLarge
                    | ChangeDiagnosticKind::GitQueryFailed => {
                        ImpactDiagnosticKind::HistoricalModelUnavailable
                    }
                    ChangeDiagnosticKind::BinaryFile => ImpactDiagnosticKind::UnmappedChange,
                };
                ImpactDiagnostic {
                    kind,
                    subject: diagnostic.path.clone(),
                    detail: diagnostic.detail.clone(),
                }
            })
            .collect()
    }

    fn summary(
        &self,
        change_set: &ChangeSet,
        impacted: &[ImpactableNode],
        output: &crate::traversal::TraversalOutput,
        diagnostics: &[ImpactDiagnostic],
    ) -> ImpactSummary {
        let mut summary = ImpactSummary {
            changed_files: change_set.summary.files_total,
            changed_symbols: change_set.summary.symbols_total,
            changed_modules: change_set.summary.modules_changed,
            unresolved_relationships: output.unresolved_edges,
            ambiguous_relationships: output.ambiguous_edges,
            diagnostics_total: diagnostics.len() as u64,
            ..ImpactSummary::default()
        };
        for item in impacted {
            match item.node_kind {
                NodeKind::FileModule
                | NodeKind::Package
                | NodeKind::Namespace
                | NodeKind::Project => {
                    if item.distance == 1 {
                        summary.direct_impacted_modules += 1;
                    } else {
                        summary.transitive_impacted_modules += 1;
                    }
                }
                NodeKind::Symbol => {
                    if item.distance == 1 {
                        summary.direct_impacted_symbols += 1;
                    } else {
                        summary.transitive_impacted_symbols += 1;
                    }
                }
                _ => {}
            }
            if item.impact_kinds.contains(&ImpactKind::TestCandidate) {
                summary.potentially_affected_tests += 1;
            }
            if item.node_kind == NodeKind::Documentation {
                summary.documentation_candidates += 1;
            }
            summary.max_distance = summary.max_distance.max(item.distance);
        }
        summary
    }

    /// 风险信号（§85/§86：计数 + 有界证据；不定义阈值，不出总分）。
    fn risk_signals(
        &self,
        change_set: &ChangeSet,
        changed: &[ImpactableNode],
        impacted: &[ImpactableNode],
        unresolved_edges: u64,
        options: &ImpactOptions,
    ) -> Vec<RiskSignal> {
        let mut signals: Vec<RiskSignal> = Vec::new();
        let cap = options.max_evidence_per_signal;

        // PublicApiChanged：可见性/导出特征变化或 public 符号签名变化（§42）。
        let public_api: Vec<String> = change_set
            .symbols
            .iter()
            .filter(|symbol| {
                symbol.characteristics.iter().any(|characteristic| {
                    matches!(
                        characteristic,
                        prism_changes::SymbolCharacteristic::VisibilityChanged
                    )
                }) || (symbol.kind == prism_parser::SymbolKind::Export
                    && symbol.characteristics.iter().any(|characteristic| {
                        matches!(
                            characteristic,
                            prism_changes::SymbolCharacteristic::ExportChanged
                        )
                    }))
            })
            .map(|symbol| {
                symbol
                    .after_symbol_id
                    .as_ref()
                    .or(symbol.before_symbol_id.as_ref())
                    .map(|id| id.as_str().to_string())
                    .unwrap_or_else(|| symbol.name.clone())
            })
            .collect();
        if !public_api.is_empty() {
            signals.push(RiskSignal {
                kind: RiskSignalKind::PublicApiChanged,
                count: public_api.len() as u64,
                detail: "changed symbols affect visibility or export surface".to_string(),
                evidence: public_api.iter().take(cap as usize).cloned().collect(),
            });
        }

        // DeletedReferencedSymbol（§20/§96）。
        let deleted_referenced: Vec<String> = impacted
            .iter()
            .filter(|item| item.reasons.contains(&ImpactReason::CallsDeletedSymbol))
            .map(|item| item.node_id.clone())
            .collect();
        if !deleted_referenced.is_empty() {
            signals.push(RiskSignal {
                kind: RiskSignalKind::DeletedReferencedSymbol,
                count: deleted_referenced.len() as u64,
                detail: "deleted symbols still referenced by unresolved call sites".to_string(),
                evidence: deleted_referenced.iter().take(cap as usize).cloned().collect(),
            });
        }

        // CrossModuleImpact：影响落在被改文件集合之外（事实，非阈值）。
        let changed_paths: BTreeSet<String> = changed
            .iter()
            .filter(|item| item.node_kind == NodeKind::File)
            .map(|item| item.path.clone())
            .collect();
        let cross_module: Vec<String> = impacted
            .iter()
            .filter(|item| {
                matches!(
                    item.node_kind,
                    NodeKind::FileModule | NodeKind::Package | NodeKind::Namespace
                ) && !changed_paths.contains(&item.path)
            })
            .map(|item| item.node_id.clone())
            .collect();
        if !cross_module.is_empty() {
            signals.push(RiskSignal {
                kind: RiskSignalKind::CrossModuleImpact,
                count: cross_module.len() as u64,
                detail: "impact extends beyond the changed files themselves".to_string(),
                evidence: cross_module.iter().take(cap as usize).cloned().collect(),
            });
        }

        if unresolved_edges > 0 {
            signals.push(RiskSignal {
                kind: RiskSignalKind::UnresolvedRelationship,
                count: unresolved_edges,
                detail: "relationships adjacent to changed entities that could not be resolved; targets were not fabricated"
                    .to_string(),
                evidence: Vec::new(),
            });
        }

        if change_set.summary.files_with_ambiguous_symbols > 0 {
            signals.push(RiskSignal {
                kind: RiskSignalKind::AmbiguousSymbolMapping,
                count: change_set.summary.files_with_ambiguous_symbols,
                detail: "M5 symbol mapping was ambiguous for some files".to_string(),
                evidence: Vec::new(),
            });
        }

        let config_changed = change_set
            .files
            .iter()
            .filter(|file| {
                file.new_file_kind.or(file.old_file_kind)
                    == Some(prism_project::FileKind::Config)
            })
            .filter_map(|file| file.new_path.clone().or_else(|| file.old_path.clone()))
            .collect::<Vec<_>>();
        if !config_changed.is_empty() {
            signals.push(RiskSignal {
                kind: RiskSignalKind::ConfigurationFileChanged,
                count: config_changed.len() as u64,
                detail: "configuration files changed; blast radius is not inferred (§53)"
                    .to_string(),
                evidence: config_changed.iter().take(cap as usize).cloned().collect(),
            });
        }

        signals.sort_by_key(|signal| signal.kind);
        signals
    }
}
