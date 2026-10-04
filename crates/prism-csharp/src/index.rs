//! Symbol Index 与增量引擎（2.0-M23 §21-§22/§28-§29/§39-§40）。
//!
//! Source of Truth = 当前文件集合的逐文件抽取（content_hash + parser 版本
//! 绑定，§40）；Index 是派生视图，全量重建 O(N)（当前规模亚秒，无需持久
//! 缓存——§39 红线：不建第二套互不相知的缓存）。幽灵符号不可能存在：
//! 索引永远从当前文件集合重建。

use crate::identity::symbol_id;
use crate::model::{DeclarationOccurrence, FileExtraction, SymbolKindCs};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// PARSER/EXTRACTION 版本（§61）：任一变化 ⇒ 旧抽取不可复用。
pub const PARSER_VERSION: &str = "tree-sitter-c-sharp/0.23.5";
pub const EXTRACTION_VERSION: &str = "1";

/// 聚合后的逻辑符号（§9：partial 的全部声明挂在同一逻辑符号下）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Symbol {
    pub id: String,
    pub kind: SymbolKindCs,
    pub name: String,
    pub qualified_name: String,
    pub display_name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub namespace: String,
    /// 逻辑父符号 id（namespace 符号/外层类型/宿主方法；根为 None）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub modifiers: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub type_parameters: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub constraints: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub base_types: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attributes: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parameters: Vec<crate::model::ParamInfo>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub return_type_syntax: Option<String>,
    pub is_partial: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub doc_summary: Option<String>,
    /// 全部声明出现（partial ≥ 2 个文件片段；普通 = 1）。
    pub declarations: Vec<DeclarationOccurrence>,
}

/// 查询入口（§21；BTreeMap 保证确定性遍历，§41）。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SymbolIndex {
    pub parser_version: String,
    pub extraction_version: String,
    pub by_id: BTreeMap<String, Symbol>,
    pub by_file: BTreeMap<String, Vec<String>>,
    pub by_name: BTreeMap<String, Vec<String>>,
    pub by_qualified: BTreeMap<String, Vec<String>>,
}

impl SymbolIndex {
    pub fn get(&self, id: &str) -> Option<&Symbol> {
        self.by_id.get(id)
    }

    pub fn by_file(&self, file: &str) -> Vec<&Symbol> {
        self.by_file
            .get(file)
            .map(|ids| ids.iter().filter_map(|id| self.by_id.get(id)).collect())
            .unwrap_or_default()
    }

    pub fn children(&self, parent_id: &str) -> Vec<&Symbol> {
        self.by_id
            .values()
            .filter(|symbol| symbol.parent_id.as_deref() == Some(parent_id))
            .collect()
    }

    pub fn by_qualified(&self, qualified_name: &str) -> Vec<&Symbol> {
        self.by_qualified
            .get(qualified_name)
            .map(|ids| ids.iter().filter_map(|id| self.by_id.get(id)).collect())
            .unwrap_or_default()
    }

    /// 名称子串搜索（大小写不敏感；结果按 id 确定序，§41/§58：只查声明，
    /// 不做全文搜索）。
    pub fn search(&self, needle: &str, kind: Option<SymbolKindCs>) -> Vec<&Symbol> {
        let needle = needle.to_lowercase();
        self.by_id
            .values()
            .filter(|symbol| {
                symbol.name.to_lowercase().contains(&needle)
                    && kind.map(|k| symbol.kind == k).unwrap_or(true)
            })
            .collect()
    }
}

/// 逻辑符号聚合中间态（partial 声明合并）。
#[derive(Clone)]
struct Builder {
    kind: SymbolKindCs,
    name: String,
    qualified: String,
    namespace: String,
    modifiers: Vec<String>,
    type_parameters: Vec<String>,
    constraints: Vec<String>,
    base_types: Vec<String>,
    attributes: Vec<String>,
    parameters: Vec<crate::model::ParamInfo>,
    return_type: Option<String>,
    is_partial: bool,
    doc_summary: Option<String>,
    declarations: Vec<DeclarationOccurrence>,
}

/// 增量引擎（§28：changed file → reparse；unchanged → reuse；deleted → invalidate）。
#[derive(Debug, Default)]
pub struct SymbolEngine {
    /// Source of Truth：当前文件集合的抽取结果（content_hash 绑定）。
    extractions: BTreeMap<String, FileExtraction>,
}

impl SymbolEngine {
    pub fn new() -> Self {
        Self::default()
    }

    /// 更新/新增一个文件（内容 hash 未变 ⇒ 复用旧抽取，§40）。
    pub fn update_file(&mut self, file: &str, source: &str) {
        let content_hash = crate::identity::hash16(source);
        if let Some(existing) = self.extractions.get(file) {
            if existing.content_hash == content_hash {
                return; // unchanged → reuse
            }
        }
        self.extractions
            .insert(file.to_string(), crate::extract::extract(file, source));
    }

    /// 删除文件（§28：不残留 ghost symbols——索引由当前集合重建）。
    pub fn remove_file(&mut self, file: &str) {
        self.extractions.remove(file);
    }

    pub fn file_count(&self) -> usize {
        self.extractions.len()
    }

    pub fn extraction(&self, file: &str) -> Option<&FileExtraction> {
        self.extractions.get(file)
    }

    /// 抽取表直读（L2 Resolver 的 usings 作用域来源，M25）。
    pub fn extraction_map(&self) -> &BTreeMap<String, FileExtraction> {
        &self.extractions
    }

    pub fn extractions(&self) -> impl Iterator<Item = &FileExtraction> {
        self.extractions.values()
    }

    /// 从当前文件集合重建索引（确定性：BTreeMap 遍历序 + 稳定 id）。
    pub fn build_index(&self) -> SymbolIndex {
        let mut index = SymbolIndex {
            parser_version: PARSER_VERSION.to_string(),
            extraction_version: EXTRACTION_VERSION.to_string(),
            ..SymbolIndex::default()
        };

        // 逻辑聚合：同 id 的 RawSymbol 合并为一个逻辑符号（partial）。
        let mut builders: BTreeMap<String, Builder> = BTreeMap::new();
        for extraction in self.extractions.values() {
            for raw in &extraction.symbols {
                let id = symbol_id(raw.kind, &raw.qualified_name, &raw.parameters);
                let builder = builders.entry(id).or_insert_with(|| Builder {
                    kind: raw.kind,
                    name: raw.name.clone(),
                    qualified: raw.qualified_name.clone(),
                    namespace: raw.namespace.clone(),
                    modifiers: raw.modifiers.clone(),
                    type_parameters: raw.type_parameters.clone(),
                    constraints: raw.constraints.clone(),
                    base_types: raw.base_types.clone(),
                    attributes: raw.attributes.iter().map(|a| a.name.clone()).collect(),
                    parameters: raw.parameters.clone(),
                    return_type: raw.return_type_syntax.clone(),
                    is_partial: raw.is_partial,
                    doc_summary: raw.doc_summary.clone(),
                    declarations: Vec::new(),
                });
                builder.is_partial |= raw.is_partial;
                if builder.doc_summary.is_none() {
                    builder.doc_summary = raw.doc_summary.clone();
                }
                builder.declarations.push(DeclarationOccurrence {
                    file: raw.file.clone(),
                    range: raw.range,
                    name_range: raw.name_range,
                });
            }
        }

        // 父链：qualified 去掉最后一段（方法签名不含于 qualified，天然成立）。
        let parent_qualified = |qualified: &str| -> Option<String> {
            let without_signature = qualified;
            match without_signature.rsplit_once('.') {
                Some((parent, _)) if !parent.is_empty() => Some(parent.to_string()),
                _ => None,
            }
        };
        fn id_of_qualified_kind(
            builders: &BTreeMap<String, Builder>,
            qualified: &str,
            kind: SymbolKindCs,
        ) -> Option<String> {
            builders
                .iter()
                .find(|(id, builder)| {
                    builder.qualified == qualified && id.rsplit('#').next() == Some(kind.as_str())
                })
                .map(|(id, _)| id.clone())
        }

        for (id, builder) in builders.iter() {
            let builder = builder.clone();
            // 父符号：优先外层类型/命名空间（qualified 前缀）。
            let parent_id = parent_qualified(&builder.qualified).and_then(|parent| {
                // 父 = 外层类型（qualified 前缀）；类型优先，命名空间兜底。
                let kinds = [
                    SymbolKindCs::Class,
                    SymbolKindCs::Struct,
                    SymbolKindCs::Interface,
                    SymbolKindCs::Enum,
                    SymbolKindCs::Record,
                    SymbolKindCs::RecordStruct,
                    SymbolKindCs::Method,
                ];
                kinds
                    .iter()
                    .find_map(|k| id_of_qualified_kind(&builders, &parent, *k))
                    .or_else(|| id_of_qualified_kind(&builders, &parent, SymbolKindCs::Namespace))
            });
            let display_name = display_for(&builder);
            let symbol = Symbol {
                id: id.clone(),
                kind: builder.kind,
                name: builder.name,
                qualified_name: builder.qualified,
                display_name,
                namespace: builder.namespace,
                parent_id,
                modifiers: builder.modifiers,
                type_parameters: builder.type_parameters,
                constraints: builder.constraints,
                base_types: builder.base_types,
                attributes: builder.attributes,
                parameters: builder.parameters,
                return_type_syntax: builder.return_type,
                is_partial: builder.is_partial,
                doc_summary: builder.doc_summary,
                declarations: builder.declarations,
            };
            index
                .by_file
                .entry(symbol.declarations[0].file.clone())
                .or_default()
                .push(id.clone());
            for declaration in &symbol.declarations {
                let entry = index.by_file.entry(declaration.file.clone()).or_default();
                if !entry.contains(id) {
                    entry.push(id.clone());
                }
            }
            index
                .by_name
                .entry(symbol.name.to_lowercase())
                .or_default()
                .push(id.clone());
            index
                .by_qualified
                .entry(symbol.qualified_name.clone())
                .or_default()
                .push(id.clone());
            index.by_id.insert(id.clone(), symbol);
        }
        index
    }
}

fn display_for(builder: &Builder) -> String {
    if matches!(
        builder.kind,
        SymbolKindCs::Method
            | SymbolKindCs::Constructor
            | SymbolKindCs::Delegate
            | SymbolKindCs::LocalFunction
            | SymbolKindCs::Operator
    ) {
        let types: Vec<String> = builder
            .parameters
            .iter()
            .map(|p| p.type_syntax.clone())
            .collect();
        format!("{}({})", builder.qualified, types.join(","))
    } else {
        builder.qualified.clone()
    }
}
