//! SymbolIndex：file/symbol/parent/children/qualified/name 五类查询。
//!
//! `by_id` 存 (parse_result 下标, symbol 下标) 定位对以便 O(1) 解引用；
//! 其余映射直接存 SymbolId（name/qualified 允许重复 → Vec，规格 §26）。
//! 查询方法接收 [`ProjectCodeModel`] 完成解引用，索引本身不复制符号。

use crate::model::ProjectCodeModel;
use prism_core::ids::{FileId, SymbolId};
use prism_parser::Symbol;
use std::collections::HashMap;

#[derive(Debug, Default, Clone)]
pub struct SymbolIndex {
    by_id: HashMap<SymbolId, (usize, usize)>,
    by_file: HashMap<FileId, Vec<SymbolId>>,
    children: HashMap<SymbolId, Vec<SymbolId>>,
    by_qualified: HashMap<String, Vec<SymbolId>>,
    by_name: HashMap<String, Vec<SymbolId>>,
}

impl SymbolIndex {
    /// 从 Code Model 构建（parse_results 必须已按 relative_path 排序）。
    pub fn build(model: &ProjectCodeModel) -> Self {
        let mut index = SymbolIndex::default();
        for (result_idx, result) in model.parse_results.iter().enumerate() {
            for (symbol_idx, symbol) in result.symbols.iter().enumerate() {
                index
                    .by_id
                    .insert(symbol.id.clone(), (result_idx, symbol_idx));
                index
                    .by_file
                    .entry(symbol.file_id.clone())
                    .or_default()
                    .push(symbol.id.clone());
                if let Some(parent) = &symbol.parent_id {
                    index
                        .children
                        .entry(parent.clone())
                        .or_default()
                        .push(symbol.id.clone());
                }
                index
                    .by_qualified
                    .entry(symbol.qualified_name.clone())
                    .or_default()
                    .push(symbol.id.clone());
                index
                    .by_name
                    .entry(symbol.name.clone())
                    .or_default()
                    .push(symbol.id.clone());
            }
        }
        index
    }

    pub fn get<'a>(&self, model: &'a ProjectCodeModel, id: &SymbolId) -> Option<&'a Symbol> {
        self.by_id.get(id).and_then(|(r, s)| {
            model
                .parse_results
                .get(*r)
                .and_then(|result| result.symbols.get(*s))
        })
    }

    pub fn ids_in_file(&self, file_id: &FileId) -> Vec<SymbolId> {
        self.by_file.get(file_id).cloned().unwrap_or_default()
    }

    pub fn children_ids(&self, id: &SymbolId) -> Vec<SymbolId> {
        self.children.get(id).cloned().unwrap_or_default()
    }

    pub fn by_qualified_name(&self, qualified: &str) -> Vec<SymbolId> {
        self.by_qualified
            .get(qualified)
            .cloned()
            .unwrap_or_default()
    }

    pub fn by_name(&self, name: &str) -> Vec<SymbolId> {
        self.by_name.get(name).cloned().unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ParseSummary, ProjectCodeModel, CODE_MODEL_SCHEMA_VERSION};
    use prism_core::Timestamp;
    use prism_parser::{
        ParseResult, ParseStatus, SourceLocation, SourceRange, SymbolKind, SymbolVisibility,
    };
    use prism_project::{FileClassification, FileInfo, FileKind, Project};
    use std::path::PathBuf;

    fn file_info(path: &str) -> FileInfo {
        FileInfo {
            id: FileId::from_string(format!("file:{path}")),
            relative_path: path.to_string(),
            file_name: path.to_string(),
            extension: Some("ts".to_string()),
            size_bytes: 10,
            language: Some(prism_project::Language::TypeScript),
            classification: FileClassification {
                kind: FileKind::Source,
                confidence: prism_core::Confidence::High,
                reason: "test".to_string(),
            },
            analyzable: true,
            non_analyzable_reason: None,
            modified_at: None,
            is_hidden: false,
            is_symlink: false,
        }
    }

    fn symbol(path: &str, qualified: &str, kind: SymbolKind, parent: Option<SymbolId>) -> Symbol {
        Symbol::new(
            path,
            qualified,
            kind,
            prism_project::Language::TypeScript,
            FileId::from_string(format!("file:{path}")),
            parent,
            SymbolVisibility::Unknown,
            SourceRange {
                start: SourceLocation::START,
                end: SourceLocation::START,
            },
        )
    }

    fn model_with(symbols: Vec<Symbol>, path: &str) -> ProjectCodeModel {
        let file = file_info(path);
        let file_id = file.id.clone();
        ProjectCodeModel {
            schema_version: CODE_MODEL_SCHEMA_VERSION,
            project: Project {
                id: prism_core::ids::ProjectId::from_string("proj"),
                name: "demo".to_string(),
                root_path: PathBuf::from("/tmp/demo"),
                discovered_at: Timestamp::default(),
            },
            files: vec![file],
            parse_results: vec![ParseResult {
                file_id,
                relative_path: path.to_string(),
                language: Some(prism_project::Language::TypeScript),
                status: ParseStatus::Successful,
                symbols,
                diagnostics: vec![],
                duration_ms: 0,
                parser_version: "test".to_string(),
                reason: None,
                imports: Vec::new(),
                calls: Vec::new(),
            }],
            summary: ParseSummary::default(),
        }
    }

    #[test]
    fn index_supports_all_five_queries() {
        // 可见性用 core 的 SymbolVisibility；此处通过模型构造校验查询语义。
        let class = symbol("a.ts", "a.Widget", SymbolKind::Class, None);
        let class_id = class.id.clone();
        let method = symbol(
            "a.ts",
            "a.Widget.render",
            SymbolKind::Method,
            Some(class_id.clone()),
        );
        let model = model_with(vec![class, method], "a.ts");
        let index = SymbolIndex::build(&model);

        assert!(index.get(&model, &class_id).is_some());
        assert_eq!(
            index.ids_in_file(&FileId::from_string("file:a.ts")).len(),
            2
        );
        assert_eq!(index.children_ids(&class_id).len(), 1);
        assert_eq!(index.by_qualified_name("a.Widget.render").len(), 1);
        assert_eq!(index.by_name("render").len(), 1);
        assert_eq!(index.by_name("missing").len(), 0);
        // get 解引用回真实符号
        let resolved = index.get(&model, &class_id).unwrap();
        assert_eq!(resolved.qualified_name, "a.Widget");
    }
}
