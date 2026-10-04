//! L1 → M21 Core 集成（2.0-M23 §23-§26/§34）。
//!
//! Symbol = Intel Entity（kind=symbol，canonical=qualified#sig）；
//! containment/file 关系 + SourceRange Evidence + 聚合事实；
//! 与 M22 bundle merge 时 file 实体按同口径 id 去重（§三十四 链路）。

use crate::index::SymbolIndex;
use prism_intel::entity::{Entity, EntityLocation, EntityScope};
use prism_intel::evidence::{Evidence, EvidenceKind, SourceRange};
use prism_intel::fact::{Fact, FactType, FactValue};
use prism_intel::identity::{EntityKind, ProjectIdentity};
use prism_intel::meta::{Confidence, ConfidenceReason, Freshness, Origin, Provenance};
use prism_intel::relation::{Relation, RelationTarget, RelationType};
use prism_intel::{IntelBundle, IntelOutcome};

/// 依赖声明抽取上限对照（防噪声，沿 M22 模式）。
const MAX_EVIDENCE_PER_SYMBOL: usize = 4;

/// 把符号索引翻译为 M21 Core 集合。
///
/// `parse_errors` = 各文件诊断计数总和（>0 时状态仍为 Complete + 诊断
/// warnings——单文件语法错误不拖垮项目，§30）。
pub fn symbols_to_intel(
    index: &SymbolIndex,
    identity: &ProjectIdentity,
    observed_at: &str,
    parse_error_files: usize,
) -> IntelOutcome<IntelBundle> {
    let mut warnings = Vec::new();
    if parse_error_files > 0 {
        warnings.push(format!(
            "{parse_error_files} file(s) had parse errors; recovered symbols still indexed"
        ));
    }
    let provenance = Provenance {
        producer: "prism-csharp::symbols_to_intel".to_string(),
        producer_version: format!(
            "{}/{}",
            crate::index::PARSER_VERSION,
            crate::index::EXTRACTION_VERSION
        ),
        observed_at: observed_at.to_string(),
    };
    let mut bundle = IntelBundle::new(identity.clone(), provenance.clone());
    let project_id = bundle.entity_id(&EntityKind::project(), "");

    // 文件实体（与 M22 L0 同口径 id —— merge 时自然去重）。
    for file in index.by_file.keys() {
        let file_id = bundle.entity_id(&EntityKind::file(), file);
        bundle.insert_entity(Entity::new(
            file_id.clone(),
            EntityKind::file(),
            file.rsplit('/').next().unwrap_or(file).to_string(),
            EntityLocation::FileOnly { path: file.clone() },
            EntityScope::Internal,
        ));
    }

    // 符号实体 + SourceRange Evidence（§25：可回溯到源码）。
    for symbol in index.by_id.values() {
        let canonical = format!(
            "{}#{}",
            symbol.qualified_name,
            symbol.id.rsplit('#').next().unwrap_or_default()
        );
        let entity_id = bundle.entity_id(&EntityKind::symbol(), &canonical);
        let mut entity = Entity::new(
            entity_id.clone(),
            EntityKind::symbol(),
            symbol.display_name.clone(),
            EntityLocation::FileOnly {
                path: symbol.declarations[0].file.clone(),
            },
            EntityScope::Internal,
        );
        entity
            .metadata
            .insert("kind".to_string(), symbol.kind.as_str().to_string());
        if !symbol.namespace.is_empty() {
            entity
                .metadata
                .insert("namespace".to_string(), symbol.namespace.clone());
        }
        if !symbol.modifiers.is_empty() {
            entity
                .metadata
                .insert("modifiers".to_string(), symbol.modifiers.join(" "));
        }
        if !symbol.type_parameters.is_empty() {
            entity.metadata.insert(
                "generic_arity".to_string(),
                symbol.type_parameters.len().to_string(),
            );
        }
        if !symbol.attributes.is_empty() {
            entity
                .metadata
                .insert("attributes".to_string(), symbol.attributes.join(","));
        }
        if let Some(doc) = &symbol.doc_summary {
            entity.metadata.insert("doc".to_string(), doc.clone());
        }
        bundle.insert_entity(entity);

        // Evidence：每个声明一条 SourceRange（cap 防噪声）。
        for declaration in symbol.declarations.iter().take(MAX_EVIDENCE_PER_SYMBOL) {
            let evidence = Evidence::new(
                EvidenceKind::SourceRange,
                Some(declaration.file.clone()),
                Some(SourceRange {
                    path: declaration.file.clone(),
                    start_line: declaration.range.start_line,
                    start_column: declaration.range.start_column,
                    end_line: declaration.range.end_line,
                    end_column: declaration.range.end_column,
                }),
                None,
                format!("declaration of {}", symbol.qualified_name),
            );
            bundle.insert_evidence(evidence);
        }

        // file declares symbol 关系（文件实体已建/由 M22 去重）。
        let primary_file = &symbol.declarations[0].file;
        let file_id = bundle.entity_id(&EntityKind::file(), primary_file);
        bundle.insert_relation(Relation::new(
            RelationType::defines(),
            file_id,
            RelationTarget::Resolved {
                entity: entity_id.clone(),
            },
            Origin::Fact,
            Confidence::High,
            vec![],
        ));
    }

    // containment 关系（parent 链；§24 纯语法包含，非语义）。
    for symbol in index.by_id.values() {
        let Some(parent_id) = &symbol.parent_id else {
            continue;
        };
        let Some(parent) = index.by_id.get(parent_id) else {
            continue;
        };
        let child_canonical = format!(
            "{}#{}",
            symbol.qualified_name,
            symbol.id.rsplit('#').next().unwrap_or_default()
        );
        let parent_canonical = format!(
            "{}#{}",
            parent.qualified_name,
            parent.id.rsplit('#').next().unwrap_or_default()
        );
        bundle.insert_relation(Relation::new(
            RelationType::contains(),
            bundle.entity_id(&EntityKind::symbol(), &parent_canonical),
            RelationTarget::Resolved {
                entity: bundle.entity_id(&EntityKind::symbol(), &child_canonical),
            },
            Origin::Fact,
            Confidence::High,
            vec![],
        ));
    }

    // 聚合事实（§41 合格 FACT）。
    let mut by_kind: std::collections::BTreeMap<&str, i64> = Default::default();
    for symbol in index.by_id.values() {
        *by_kind.entry(symbol.kind.as_str()).or_default() += 1;
    }
    bundle.insert_fact(Fact::new(
        FactType::other("symbols_total"),
        Some(project_id.clone()),
        FactValue::Int(index.by_id.len() as i64),
        Origin::Fact,
        Confidence::High,
        ConfidenceReason::direct_observation(),
        Freshness::Fresh,
        vec![],
        observed_at,
        provenance.clone(),
    ));
    for (kind, count) in by_kind {
        bundle.insert_fact(Fact::new(
            FactType::other(format!("symbols_{kind}")),
            Some(project_id.clone()),
            FactValue::Int(count),
            Origin::Fact,
            Confidence::High,
            ConfidenceReason::direct_observation(),
            Freshness::Fresh,
            vec![],
            observed_at,
            provenance.clone(),
        ));
    }
    let _ = &mut warnings;

    if warnings.is_empty() {
        IntelOutcome::complete(bundle)
    } else {
        IntelOutcome {
            status: prism_intel::IntelStatus::Complete,
            data: Some(bundle),
            warnings,
            errors: Vec::new(),
        }
    }
}
