//! C# adapter（tree-sitter-c-sharp）。
//!
//! 抽取（规格 §33）：namespace / class / struct / interface / enum / record /
//! method / constructor / property / field / using；attribute 完整抽取属 P1。
//! 可见性按 C# 语言真实默认：类成员无修饰符 = Private，顶层类型 = Internal。

use crate::model::{
    SourceLocation, SourceRange, Symbol, SymbolKind, SymbolVisibility, COORDINATE_CONTRACT,
};
use crate::parse::{ImportInfo, ParseDiagnostic, ParseRequest, ParseResult, ParseStatus};
use crate::registry::LanguageParser;
use prism_core::ids::{FileId, SymbolId};
use prism_core::task::CancellationToken;
use prism_core::{PrismError, PrismResult};
use prism_project::Language;
use std::time::Instant;
use tree_sitter::{Node, Parser};

pub struct CSharpParser {
    version: &'static str,
}

impl Default for CSharpParser {
    fn default() -> Self {
        Self::new()
    }
}

impl CSharpParser {
    pub fn new() -> Self {
        CSharpParser {
            version: "tree-sitter-c-sharp/1",
        }
    }

    fn node_location(node: Node<'_>, end: bool) -> SourceLocation {
        let point = if end {
            node.end_position()
        } else {
            node.start_position()
        };
        SourceLocation {
            line: point.row as u32 + 1,
            column: point.column as u32 + 1,
            byte_offset: if end {
                node.end_byte()
            } else {
                node.start_byte()
            },
        }
    }

    fn range(node: Node<'_>) -> SourceRange {
        SourceRange {
            start: Self::node_location(node, false),
            end: Self::node_location(node, true),
        }
    }

    fn text<'a>(node: Node<'a>, content: &'a str) -> &'a str {
        node.utf8_text(content.as_bytes()).unwrap_or("")
    }

    fn child_text<'a>(node: Node<'a>, field: &str, content: &'a str) -> Option<&'a str> {
        node.child_by_field_name(field)
            .map(|child| Self::text(child, content))
    }

    /// C# 可见性：显式 modifier 优先；类成员默认 Private，顶层类型默认 Internal。
    fn visibility_of(
        node: Node<'_>,
        content: &str,
        member_default: SymbolVisibility,
    ) -> SymbolVisibility {
        let mut child = node.child(0);
        while let Some(current) = child {
            if current.kind() == "modifier" {
                return match Self::text(current, content) {
                    "public" => SymbolVisibility::Public,
                    "private" => SymbolVisibility::Private,
                    "protected" => SymbolVisibility::Protected,
                    "internal" => SymbolVisibility::Internal,
                    _ => member_default,
                };
            }
            child = current.next_sibling();
        }
        member_default
    }

    fn method_signature<'a>(node: Node<'a>, content: &'a str) -> Option<String> {
        let params = node.child_by_field_name("parameters")?;
        let mut signature = Self::text(params, content).to_string();
        if let Some(returns) = node.child_by_field_name("returns") {
            signature = format!("{} {}", Self::text(returns, content), signature);
        }
        Some(signature)
    }
}

struct Collector<'a> {
    content: &'a str,
    file_id: &'a FileId,
    relative_path: &'a str,
    symbols: Vec<Symbol>,
    diagnostics: Vec<ParseDiagnostic>,
    imports: Vec<ImportInfo>,
    cancel: &'a CancellationToken,
}

impl<'a> Collector<'a> {
    /// 限定名前缀：相对路径去掉最后一个扩展名（无命名空间文件的兜底根）。
    fn qualified_root(relative_path: &str) -> String {
        match relative_path.rsplit_once('.') {
            Some((prefix, ext)) if !ext.is_empty() && ext.len() <= 5 && !ext.contains('/') => {
                prefix.to_string()
            }
            _ => relative_path.to_string(),
        }
    }

    fn emit(
        &mut self,
        node: Node<'a>,
        kind: SymbolKind,
        qualified_path: &str,
        parent_id: Option<&SymbolId>,
        visibility: SymbolVisibility,
        signature: Option<String>,
    ) -> Symbol {
        let mut symbol = Symbol::new(
            self.relative_path,
            qualified_path,
            kind,
            Language::CSharp,
            self.file_id.clone(),
            parent_id.cloned(),
            visibility,
            CSharpParser::range(node),
        );
        symbol.signature = signature;
        symbol.documentation_present = Self::has_adjacent_comment(node, self.content);
        self.symbols.push(symbol.clone());
        symbol
    }

    fn has_adjacent_comment(node: Node<'_>, _content: &str) -> bool {
        let mut sibling = node;
        while let Some(prev) = sibling.prev_sibling() {
            if prev.kind() == "comment" {
                let gap = node.start_position().row as i64 - prev.end_position().row as i64;
                return gap <= 3;
            }
            if matches!(prev.kind(), "modifier" | "attribute_list") {
                sibling = prev;
                continue;
            }
            return false;
        }
        false
    }

    fn walk(
        &mut self,
        node: Node<'a>,
        parent_id: Option<&SymbolId>,
        prefix: &str,
        top_level: bool,
    ) {
        if self.cancel.is_cancelled() {
            return;
        }

        match node.kind() {
            "namespace_declaration" | "file_scoped_namespace_declaration" => {
                if let Some(name) = CSharpParser::child_text(node, "name", self.content) {
                    let qualified = format!("{prefix}.{name}");
                    let visibility =
                        CSharpParser::visibility_of(node, self.content, SymbolVisibility::Internal);
                    let symbol = self.emit(
                        node,
                        SymbolKind::Namespace,
                        &qualified,
                        parent_id,
                        visibility,
                        None,
                    );
                    self.walk_children(node, Some(&symbol.id), &qualified, false);
                }
            }
            "class_declaration" | "record_declaration" => {
                if let Some(name) = CSharpParser::child_text(node, "name", self.content) {
                    let qualified = format!("{prefix}.{name}");
                    let visibility = CSharpParser::visibility_of(
                        node,
                        self.content,
                        if top_level {
                            SymbolVisibility::Internal
                        } else {
                            SymbolVisibility::Private
                        },
                    );
                    let symbol = self.emit(
                        node,
                        SymbolKind::Class,
                        &qualified,
                        parent_id,
                        visibility,
                        None,
                    );
                    self.walk_children(node, Some(&symbol.id), &qualified, false);
                }
            }
            "struct_declaration" => {
                if let Some(name) = CSharpParser::child_text(node, "name", self.content) {
                    let qualified = format!("{prefix}.{name}");
                    let visibility = CSharpParser::visibility_of(
                        node,
                        self.content,
                        if top_level {
                            SymbolVisibility::Internal
                        } else {
                            SymbolVisibility::Private
                        },
                    );
                    let symbol = self.emit(
                        node,
                        SymbolKind::Struct,
                        &qualified,
                        parent_id,
                        visibility,
                        None,
                    );
                    self.walk_children(node, Some(&symbol.id), &qualified, false);
                }
            }
            "interface_declaration" => {
                if let Some(name) = CSharpParser::child_text(node, "name", self.content) {
                    let qualified = format!("{prefix}.{name}");
                    let visibility = CSharpParser::visibility_of(
                        node,
                        self.content,
                        if top_level {
                            SymbolVisibility::Internal
                        } else {
                            SymbolVisibility::Private
                        },
                    );
                    let symbol = self.emit(
                        node,
                        SymbolKind::Interface,
                        &qualified,
                        parent_id,
                        visibility,
                        None,
                    );
                    self.walk_children(node, Some(&symbol.id), &qualified, false);
                }
            }
            "enum_declaration" => {
                if let Some(name) = CSharpParser::child_text(node, "name", self.content) {
                    let qualified = format!("{prefix}.{name}");
                    let visibility = CSharpParser::visibility_of(
                        node,
                        self.content,
                        if top_level {
                            SymbolVisibility::Internal
                        } else {
                            SymbolVisibility::Private
                        },
                    );
                    let symbol = self.emit(
                        node,
                        SymbolKind::Enum,
                        &qualified,
                        parent_id,
                        visibility,
                        None,
                    );
                    self.walk_children(node, Some(&symbol.id), &qualified, false);
                }
            }
            "method_declaration" => {
                if let Some(name) = CSharpParser::child_text(node, "name", self.content) {
                    let qualified = format!("{prefix}.{name}");
                    let visibility =
                        CSharpParser::visibility_of(node, self.content, SymbolVisibility::Private);
                    let signature = CSharpParser::method_signature(node, self.content);
                    let symbol = self.emit(
                        node,
                        SymbolKind::Method,
                        &qualified,
                        parent_id,
                        visibility,
                        signature,
                    );
                    self.walk_children(node, Some(&symbol.id), &qualified, false);
                }
            }
            "constructor_declaration" => {
                if let Some(name) = CSharpParser::child_text(node, "name", self.content) {
                    let qualified = format!("{prefix}.{name}");
                    let visibility =
                        CSharpParser::visibility_of(node, self.content, SymbolVisibility::Private);
                    let signature = node
                        .child_by_field_name("parameters")
                        .map(|params| CSharpParser::text(params, self.content).to_string());
                    self.emit(
                        node,
                        SymbolKind::Constructor,
                        &qualified,
                        parent_id,
                        visibility,
                        signature,
                    );
                }
            }
            "property_declaration" => {
                if let Some(name) = CSharpParser::child_text(node, "name", self.content) {
                    let qualified = format!("{prefix}.{name}");
                    let visibility =
                        CSharpParser::visibility_of(node, self.content, SymbolVisibility::Private);
                    self.emit(
                        node,
                        SymbolKind::Property,
                        &qualified,
                        parent_id,
                        visibility,
                        None,
                    );
                }
            }
            "field_declaration" => {
                // 一条 field_declaration 可含多个 declarator。
                let mut child = node.child(0);
                while let Some(current) = child {
                    if current.kind() == "variable_declaration" {
                        let mut declarator = current.child(0);
                        while let Some(decl) = declarator {
                            if decl.kind() == "variable_declarator" {
                                if let Some(name) =
                                    CSharpParser::child_text(decl, "name", self.content)
                                {
                                    let qualified = format!("{prefix}.{name}");
                                    let visibility = CSharpParser::visibility_of(
                                        node,
                                        self.content,
                                        SymbolVisibility::Private,
                                    );
                                    self.emit(
                                        node,
                                        SymbolKind::Field,
                                        &qualified,
                                        parent_id,
                                        visibility,
                                        None,
                                    );
                                }
                            }
                            declarator = decl.next_sibling();
                        }
                    }
                    child = current.next_sibling();
                }
            }
            "enum_member_declaration" => {
                if let Some(name) = CSharpParser::child_text(node, "name", self.content) {
                    let qualified = format!("{prefix}.{name}");
                    self.emit(
                        node,
                        SymbolKind::Constant,
                        &qualified,
                        parent_id,
                        SymbolVisibility::Public,
                        None,
                    );
                }
            }
            "using_directive" => {
                // field 名不可靠（裸 identifier 无 name 字段），扫描 named children。
                let mut child = node.child(0);
                let mut emitted = false;
                while let Some(current) = child {
                    if matches!(current.kind(), "qualified_name" | "identifier") && !emitted {
                        let name = CSharpParser::text(current, self.content);
                        if !name.is_empty() {
                            self.imports.push(ImportInfo {
                                specifier: name.to_string(),
                                kind: crate::parse::ImportKind::Plain,
                                names: Vec::new(),
                                location: Some(CSharpParser::node_location(node, false)),
                            });
                            let qualified = format!("{prefix}.{name}");
                            self.emit(
                                node,
                                SymbolKind::Import,
                                &qualified,
                                parent_id,
                                SymbolVisibility::Unknown,
                                None,
                            );
                            emitted = true;
                        }
                    }
                    child = current.next_sibling();
                }
            }
            // ERROR 节点：受限恢复（同 TS adapter，规格 §24）。
            "ERROR" => self.walk_error(node, parent_id, prefix, top_level),
            _ => self.walk_children(node, parent_id, prefix, top_level),
        }
    }

    /// ERROR 节点内的受限恢复：仅合成直接可见的 `class/struct/interface/enum/
    /// namespace <identifier>` 序列；已恢复的结构化声明挂到其下。
    fn walk_error(
        &mut self,
        node: Node<'a>,
        parent_id: Option<&SymbolId>,
        prefix: &str,
        _top_level: bool,
    ) {
        let mut pending: Option<SymbolKind> = None;
        let mut inner: Option<(SymbolId, String)> = None;

        let mut child = node.child(0);
        while let Some(current) = child {
            if current.is_named() {
                let kind = current.kind();
                if kind == "identifier" && pending.is_some() {
                    let keyword_kind = pending.take().expect("checked above");
                    let name = CSharpParser::text(current, self.content);
                    if !name.is_empty() {
                        let qualified = format!("{prefix}.{name}");
                        let symbol = self.emit(
                            current,
                            keyword_kind,
                            &qualified,
                            parent_id,
                            SymbolVisibility::Unknown,
                            None,
                        );
                        inner = Some((symbol.id, qualified));
                    }
                } else if matches!(
                    kind,
                    "method_declaration"
                        | "constructor_declaration"
                        | "property_declaration"
                        | "field_declaration"
                        | "class_declaration"
                        | "struct_declaration"
                        | "interface_declaration"
                        | "enum_declaration"
                ) {
                    let (owner_id, owner_prefix) = match inner.as_ref() {
                        Some((id, qualified)) => (Some(id), qualified.as_str()),
                        None => (parent_id, prefix),
                    };
                    self.walk(current, owner_id, owner_prefix, false);
                }
            } else {
                match current.kind() {
                    "class" => pending = Some(SymbolKind::Class),
                    "struct" => pending = Some(SymbolKind::Struct),
                    "interface" => pending = Some(SymbolKind::Interface),
                    "enum" => pending = Some(SymbolKind::Enum),
                    "namespace" => pending = Some(SymbolKind::Namespace),
                    _ => {}
                }
            }
            child = current.next_sibling();
        }
    }

    fn walk_children(
        &mut self,
        node: Node<'a>,
        parent_id: Option<&SymbolId>,
        prefix: &str,
        top_level: bool,
    ) {
        let mut child = node.child(0);
        while let Some(current) = child {
            self.walk(current, parent_id, prefix, top_level);
            child = current.next_sibling();
        }
    }

    fn collect_error_diagnostics(&mut self, root: Node<'a>) {
        let mut stack = vec![root];
        let mut count = 0;
        while let Some(node) = stack.pop() {
            if self.cancel.is_cancelled() || count >= 20 {
                break;
            }
            if node.kind() == "ERROR" {
                self.diagnostics.push(ParseDiagnostic::error(
                    "syntax_error",
                    format!(
                        "syntax error near `{}`",
                        CSharpParser::text(node, self.content)
                            .chars()
                            .take(40)
                            .collect::<String>()
                    ),
                    Some(CSharpParser::node_location(node, false)),
                ));
                count += 1;
            } else if node.is_missing() {
                self.diagnostics.push(ParseDiagnostic::error(
                    "missing_syntax",
                    format!("missing `{}`", node.kind()),
                    Some(CSharpParser::node_location(node, false)),
                ));
                count += 1;
            }
            let mut child = node.child(0);
            while let Some(current) = child {
                stack.push(current);
                child = current.next_sibling();
            }
        }
        self.diagnostics.sort_by(|a, b| {
            a.location
                .map(|l| l.byte_offset)
                .cmp(&b.location.map(|l| l.byte_offset))
        });
    }
}

impl LanguageParser for CSharpParser {
    fn language(&self) -> Language {
        Language::CSharp
    }

    fn parser_version(&self) -> &str {
        self.version
    }

    fn parse(
        &self,
        request: &ParseRequest,
        cancellation: &CancellationToken,
    ) -> PrismResult<ParseResult> {
        let started = Instant::now();
        cancellation.check()?;

        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_c_sharp::LANGUAGE.into())
            .map_err(|err| PrismError::Parse {
                message: format!(
                    "failed to load c-sharp grammar: {err} (contract: {COORDINATE_CONTRACT})"
                ),
            })?;
        let tree = parser
            .parse(&request.content, None)
            .ok_or_else(|| PrismError::Parse {
                message: "tree-sitter returned no tree".to_string(),
            })?;
        cancellation.check()?;

        // C# 限定名以命名空间为根；无命名空间时退回文件路径根（规格 §17）。
        let qualified_root = Collector::qualified_root(&request.relative_path);
        let file_symbol = Symbol::new(
            &request.relative_path,
            &qualified_root,
            SymbolKind::File,
            Language::CSharp,
            request.file_id.clone(),
            None,
            SymbolVisibility::Unknown,
            SourceRange {
                start: SourceLocation::START,
                end: CSharpParser::node_location(tree.root_node(), true),
            },
        );
        let file_symbol_id = file_symbol.id.clone();

        let mut collector = Collector {
            content: &request.content,
            file_id: &request.file_id,
            relative_path: &request.relative_path,
            symbols: vec![file_symbol],
            diagnostics: Vec::new(),
            imports: Vec::new(),
            cancel: cancellation,
        };
        collector.walk_children(
            tree.root_node(),
            Some(&file_symbol_id),
            &qualified_root,
            true,
        );

        if tree.root_node().has_error() {
            collector.collect_error_diagnostics(tree.root_node());
        }

        let status = if collector.diagnostics.is_empty() {
            ParseStatus::Successful
        } else {
            ParseStatus::SuccessfulWithDiagnostics
        };

        Ok(ParseResult {
            file_id: request.file_id.clone(),
            relative_path: request.relative_path.clone(),
            language: Some(Language::CSharp),
            status,
            symbols: collector.symbols,
            diagnostics: collector.diagnostics,
            duration_ms: started.elapsed().as_millis() as u64,
            parser_version: self.version.to_string(),
            reason: None,
            imports: collector.imports,
        })
    }
}
