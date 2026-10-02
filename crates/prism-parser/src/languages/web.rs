//! TypeScript / TSX / JavaScript adapter（共享 tree-sitter walker）。
//!
//! 抽取范围（规格 §33，P0）：
//! function / class / method / constructor / property / field / variable /
//! interface / type alias / enum / namespace / import / export 子句。
//!
//! 明确不抽取（保持身份稳定，P1 再议）：匿名函数（无可稳定限定路径）、
//! 局部变量、参数、decorator。

use crate::model::{
    SourceLocation, SourceRange, Symbol, SymbolKind, SymbolVisibility, COORDINATE_CONTRACT,
};
use crate::parse::{
    ImportInfo, ImportKind, ParseDiagnostic, ParseRequest, ParseResult, ParseStatus,
};
use crate::registry::LanguageParser;
use prism_core::ids::{FileId, SymbolId};
use prism_core::task::CancellationToken;
use prism_core::{PrismError, PrismResult};
use prism_project::Language;
use std::time::Instant;
use tree_sitter::{Node, Parser, Tree};

const MAX_SYNTAX_DIAGNOSTICS: usize = 20;

/// TypeScript / TSX / JavaScript 共用 parser（同一 walker，不同 grammar）。
pub struct WebParser {
    language: Language,
    ts_language: tree_sitter::Language,
    version: &'static str,
}

impl WebParser {
    pub fn typescript() -> Self {
        WebParser {
            language: Language::TypeScript,
            ts_language: tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            version: "tree-sitter-typescript/1",
        }
    }

    pub fn tsx() -> Self {
        WebParser {
            language: Language::TypeScriptReact,
            ts_language: tree_sitter_typescript::LANGUAGE_TSX.into(),
            version: "tree-sitter-typescript-tsx/1",
        }
    }

    pub fn javascript() -> Self {
        WebParser {
            language: Language::JavaScript,
            ts_language: tree_sitter_javascript::LANGUAGE.into(),
            version: "tree-sitter-javascript/1",
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

    /// 从指定 node 起始位置的行，向上查找紧邻的注释（启发式文档检测）。
    fn has_adjacent_doc_comment(node: Node<'_>, content: &str) -> bool {
        let mut sibling = node;
        loop {
            match sibling.prev_sibling() {
                Some(prev) if prev.kind() == "comment" => {
                    // 仅当注释紧贴声明（间隔不超过 3 行）时视为文档。
                    let gap = node.start_position().row as i64 - prev.end_position().row as i64;
                    if gap <= 3 {
                        let text = Self::text(prev, content);
                        if text.trim_start().starts_with('/') {
                            return true;
                        }
                    }
                    return false;
                }
                Some(prev) => {
                    // 跳过 decorator 等修饰节点继续向前找。
                    if prev.kind().contains("decorator") {
                        sibling = prev;
                        continue;
                    }
                    return false;
                }
                None => return false,
            }
        }
    }

    fn visibility_of(node: Node<'_>, content: &str, in_class: bool) -> SymbolVisibility {
        let mut child = node.child(0);
        while let Some(current) = child {
            if current.kind() == "accessibility_modifier" {
                return match Self::text(current, content) {
                    "private" => SymbolVisibility::Private,
                    "protected" => SymbolVisibility::Protected,
                    _ => SymbolVisibility::Public,
                };
            }
            child = current.next_sibling();
        }
        // JS 的 # 私有字段 / # 私有方法。
        if Self::child_text(node, "name", content).is_some_and(|name| name.starts_with('#')) {
            return SymbolVisibility::Private;
        }
        if in_class {
            // TypeScript 类成员默认 public（语言真实语义）。
            SymbolVisibility::Public
        } else {
            SymbolVisibility::Unknown
        }
    }

    fn function_signature<'a>(node: Node<'a>, content: &'a str) -> Option<String> {
        let params = node.child_by_field_name("parameters")?;
        let mut signature = Self::text(params, content).to_string();
        if let Some(return_type) = node.child_by_field_name("return_type") {
            signature.push_str(&format!(": {}", Self::text(return_type, content)));
        }
        Some(signature)
    }
}

struct Collector<'a> {
    content: &'a str,
    file_id: &'a FileId,
    relative_path: &'a str,
    language: Language,
    symbols: Vec<Symbol>,
    diagnostics: Vec<ParseDiagnostic>,
    imports: Vec<ImportInfo>,
    cancel: &'a CancellationToken,
}

impl<'a> Collector<'a> {
    /// 限定名前缀：相对路径去掉最后一个扩展名（纲领示例：services/user.X）。
    fn qualified_root(relative_path: &str) -> String {
        match relative_path.rsplit_once('.') {
            // 只去掉“看起来像扩展名”的短后缀，避免误删路径中的点。
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
        in_class: bool,
        signature: Option<String>,
    ) -> Symbol {
        let mut symbol = Symbol::new(
            self.relative_path,
            qualified_path,
            kind,
            self.language,
            self.file_id.clone(),
            parent_id.cloned(),
            WebParser::visibility_of(node, self.content, in_class),
            WebParser::range(node),
        );
        symbol.signature = signature;
        symbol.documentation_present = WebParser::has_adjacent_doc_comment(node, self.content);
        self.symbols.push(symbol.clone());
        symbol
    }

    fn walk(&mut self, node: Node<'a>, parent_id: Option<&SymbolId>, prefix: &str, in_class: bool) {
        if self.cancel.is_cancelled() {
            return;
        }

        match node.kind() {
            "function_declaration" | "generator_function_declaration" => {
                if let Some(name) = WebParser::child_text(node, "name", self.content) {
                    let qualified = format!("{prefix}.{name}");
                    let signature = WebParser::function_signature(node, self.content);
                    let symbol = self.emit(
                        node,
                        SymbolKind::Function,
                        &qualified,
                        parent_id,
                        in_class,
                        signature,
                    );
                    self.walk_children(node, Some(&symbol.id), &qualified, false);
                }
            }
            "class_declaration" | "abstract_class_declaration" => {
                if let Some(name) = WebParser::child_text(node, "name", self.content) {
                    let qualified = format!("{prefix}.{name}");
                    let symbol = self.emit(
                        node,
                        SymbolKind::Class,
                        &qualified,
                        parent_id,
                        in_class,
                        None,
                    );
                    self.walk_children(node, Some(&symbol.id), &qualified, true);
                }
            }
            "interface_declaration" => {
                if let Some(name) = WebParser::child_text(node, "name", self.content) {
                    let qualified = format!("{prefix}.{name}");
                    let symbol = self.emit(
                        node,
                        SymbolKind::Interface,
                        &qualified,
                        parent_id,
                        in_class,
                        None,
                    );
                    self.walk_children(node, Some(&symbol.id), &qualified, true);
                }
            }
            "type_alias_declaration" => {
                if let Some(name) = WebParser::child_text(node, "name", self.content) {
                    let qualified = format!("{prefix}.{name}");
                    self.emit(
                        node,
                        SymbolKind::TypeAlias,
                        &qualified,
                        parent_id,
                        in_class,
                        None,
                    );
                }
            }
            "enum_declaration" => {
                if let Some(name) = WebParser::child_text(node, "name", self.content) {
                    let qualified = format!("{prefix}.{name}");
                    let symbol = self.emit(
                        node,
                        SymbolKind::Enum,
                        &qualified,
                        parent_id,
                        in_class,
                        None,
                    );
                    self.walk_children(node, Some(&symbol.id), &qualified, true);
                }
            }
            "namespace_declaration" | "internal_module" => {
                if let Some(name) = WebParser::child_text(node, "name", self.content) {
                    let qualified = format!("{prefix}.{name}");
                    let symbol = self.emit(
                        node,
                        SymbolKind::Namespace,
                        &qualified,
                        parent_id,
                        in_class,
                        None,
                    );
                    self.walk_children(node, Some(&symbol.id), &qualified, false);
                }
            }
            "method_definition" => {
                if let Some(name) = WebParser::child_text(node, "name", self.content) {
                    let kind = if name == "constructor" {
                        SymbolKind::Constructor
                    } else {
                        SymbolKind::Method
                    };
                    let qualified = format!("{prefix}.{name}");
                    let signature = WebParser::function_signature(node, self.content);
                    let symbol = self.emit(node, kind, &qualified, parent_id, true, signature);
                    self.walk_children(node, Some(&symbol.id), &qualified, true);
                }
            }
            "method_signature" | "abstract_method_signature" => {
                if let Some(name) = WebParser::child_text(node, "name", self.content) {
                    let qualified = format!("{prefix}.{name}");
                    let signature = WebParser::function_signature(node, self.content);
                    self.emit(
                        node,
                        SymbolKind::Method,
                        &qualified,
                        parent_id,
                        true,
                        signature,
                    );
                }
            }
            "public_field_definition" | "field_definition" => {
                if let Some(name) = WebParser::child_text(node, "name", self.content) {
                    let name = name.trim_start_matches('#');
                    let value_is_function =
                        node.child_by_field_name("value").is_some_and(|value| {
                            matches!(
                                value.kind(),
                                "arrow_function" | "function_expression" | "function"
                            )
                        });
                    let (kind, signature) = if value_is_function {
                        (
                            SymbolKind::Method,
                            WebParser::function_signature(node, self.content),
                        )
                    } else {
                        (SymbolKind::Property, None)
                    };
                    let qualified = format!("{prefix}.{name}");
                    self.emit(node, kind, &qualified, parent_id, true, signature);
                }
            }
            "lexical_declaration" | "variable_declaration" if !in_class => {
                // 顶层/命名空间变量；arrow/function 赋值识别为 Function。
                let mut child = node.child(0);
                while let Some(current) = child {
                    if current.kind() == "variable_declarator" {
                        if let Some(name) = WebParser::child_text(current, "name", self.content) {
                            let value_is_function =
                                current.child_by_field_name("value").is_some_and(|value| {
                                    matches!(
                                        value.kind(),
                                        "arrow_function" | "function_expression" | "function"
                                    )
                                });
                            let kind = if value_is_function {
                                SymbolKind::Function
                            } else {
                                SymbolKind::Variable
                            };
                            let qualified = format!("{prefix}.{name}");
                            let signature = if value_is_function {
                                current.child_by_field_name("value").and_then(|value| {
                                    WebParser::function_signature(value, self.content)
                                })
                            } else {
                                None
                            };
                            self.emit(current, kind, &qualified, parent_id, in_class, signature);
                        }
                    }
                    child = current.next_sibling();
                }
            }
            "enum_body" | "enum_assignment" => {
                self.walk_plain_children(node, parent_id, prefix, in_class, SymbolKind::Constant);
            }
            "export_statement" => {
                // 有声明子节点 → 透传（声明本身已捕获）；裸导出子句 → Export 符号。
                if node.child_by_field_name("declaration").is_some() {
                    // `export ... from "./x"`（声明形式的 re-export）→ 记录 ReExport。
                    if let Some(source) = node.child_by_field_name("source") {
                        let specifier = WebParser::text(source, self.content)
                            .trim_matches('"')
                            .trim_matches('\'')
                            .to_string();
                        self.imports.push(ImportInfo {
                            specifier,
                            kind: ImportKind::ReExport,
                            names: Vec::new(),
                            location: Some(WebParser::node_location(node, false)),
                        });
                    }
                    self.walk_children(node, parent_id, prefix, in_class);
                } else {
                    // `export { a, b } from "./x"` = re-export。
                    if let Some(source) = node.child_by_field_name("source") {
                        let specifier = WebParser::text(source, self.content)
                            .trim_matches('"')
                            .trim_matches('\'')
                            .to_string();
                        self.imports.push(ImportInfo {
                            specifier,
                            kind: ImportKind::ReExport,
                            names: Vec::new(),
                            location: Some(WebParser::node_location(node, false)),
                        });
                    }
                    let mut child = node.child(0);
                    while let Some(current) = child {
                        if current.kind() == "export_clause" {
                            let mut spec = current.child(0);
                            while let Some(specifier) = spec {
                                if specifier.kind() == "export_specifier" {
                                    if let Some(name) =
                                        WebParser::child_text(specifier, "name", self.content)
                                    {
                                        let qualified = format!("{prefix}.{name}");
                                        self.emit(
                                            specifier,
                                            SymbolKind::Export,
                                            &qualified,
                                            parent_id,
                                            in_class,
                                            None,
                                        );
                                    }
                                }
                                spec = specifier.next_sibling();
                            }
                        }
                        child = current.next_sibling();
                    }
                }
            }
            "import_statement" => {
                if let Some(source) = node.child_by_field_name("source") {
                    let raw_specifier = WebParser::text(source, self.content)
                        .trim_matches('"')
                        .trim_matches('\'')
                        .to_string();
                    let kind = if WebParser::text(node, self.content).starts_with("import type") {
                        ImportKind::TypeOnly
                    } else {
                        ImportKind::Plain
                    };
                    let mut names = Vec::new();
                    if let Some(clause) = node.child_by_field_name("imports") {
                        let mut spec = clause.child(0);
                        while let Some(current) = spec {
                            if current.kind() == "import_specifier" {
                                if let Some(name) =
                                    WebParser::child_text(current, "name", self.content)
                                {
                                    names.push(name.to_string());
                                }
                            }
                            spec = current.next_sibling();
                        }
                    }
                    self.imports.push(ImportInfo {
                        specifier: raw_specifier.clone(),
                        kind,
                        names,
                        location: Some(WebParser::node_location(node, false)),
                    });
                    let module = raw_specifier.trim_start_matches("./").to_string();
                    let qualified = format!("{prefix}.{module}");
                    self.emit(
                        node,
                        SymbolKind::Import,
                        &qualified,
                        parent_id,
                        in_class,
                        None,
                    );
                }
            }
            // ERROR 节点：受限恢复（规格 §24）——扫描 `class/interface/enum/function
            // <identifier>` 关键字序列合成符号，已恢复的结构化声明挂到其下；
            // 诊断本身在解析后统一收集。
            "ERROR" => self.walk_error(node, parent_id, prefix, in_class),
            _ => self.walk_children(node, parent_id, prefix, in_class),
        }
    }

    fn walk_plain_children(
        &mut self,
        node: Node<'a>,
        parent_id: Option<&SymbolId>,
        prefix: &str,
        in_class: bool,
        kind: SymbolKind,
    ) {
        let mut child = node.child(0);
        while let Some(current) = child {
            if current.kind() == "enum_member" || current.kind() == "property_identifier" {
                if let Some(name) =
                    WebParser::child_text(current, "name", self.content).or_else(|| {
                        let text = WebParser::text(current, self.content);
                        if text.is_empty() {
                            None
                        } else {
                            Some(text)
                        }
                    })
                {
                    let qualified = format!("{prefix}.{name}");
                    self.emit(current, kind, &qualified, parent_id, in_class, None);
                }
            }
            child = current.next_sibling();
        }
    }

    /// ERROR 节点内的受限恢复：不猜测、不伪造，只在关键字序列直接可见时合成。
    fn walk_error(
        &mut self,
        node: Node<'a>,
        parent_id: Option<&SymbolId>,
        prefix: &str,
        in_class: bool,
    ) {
        let mut pending: Option<SymbolKind> = None;
        // ERROR 内合成的“开放声明”（如 class 头），后续恢复出的成员挂到它下面。
        let mut inner: Option<(SymbolId, String)> = None;

        let mut child = node.child(0);
        while let Some(current) = child {
            if current.is_named() {
                let kind = current.kind();
                let looks_like_identifier = kind == "identifier"
                    || kind == "type_identifier"
                    || kind == "property_identifier";
                if looks_like_identifier && pending.is_some() {
                    let keyword_kind = pending.take().expect("checked above");
                    let raw_name = WebParser::text(current, self.content);
                    if !raw_name.is_empty() {
                        let name = raw_name;
                        let qualified = format!("{prefix}.{name}");
                        let symbol =
                            self.emit(current, keyword_kind, &qualified, parent_id, in_class, None);
                        // class/interface/enum 是“开放作用域”，后续成员归入。
                        inner = Some((symbol.id, qualified));
                    }
                } else if matches!(
                    kind,
                    "method_definition"
                        | "public_field_definition"
                        | "field_definition"
                        | "function_declaration"
                        | "class_declaration"
                        | "interface_declaration"
                        | "enum_declaration"
                        | "type_alias_declaration"
                        | "method_signature"
                ) {
                    // 已恢复的结构化声明：优先挂到 ERROR 内合成的开放声明下。
                    let (owner_id, owner_prefix) = match inner.as_ref() {
                        Some((id, qualified)) => (Some(id), qualified.as_str()),
                        None => (parent_id, prefix),
                    };
                    self.walk(current, owner_id, owner_prefix, true);
                }
                // 其他 named 节点（表达式碎片等）不再深入，避免噪音符号。
            } else {
                match current.kind() {
                    "class" => pending = Some(SymbolKind::Class),
                    "interface" => pending = Some(SymbolKind::Interface),
                    "enum" => pending = Some(SymbolKind::Enum),
                    "function" => pending = Some(SymbolKind::Function),
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
        in_class: bool,
    ) {
        let mut child = node.child(0);
        while let Some(current) = child {
            self.walk(current, parent_id, prefix, in_class);
            child = current.next_sibling();
        }
    }

    /// 收集语法错误诊断（ERROR 节点；MISSING 由 has_error 兜底）。
    /// 诊断顺序随后按 byte_offset 排序保证确定性。
    fn collect_error_diagnostics(&mut self, root: Node<'a>) {
        let mut stack = vec![root];
        let mut count = 0;
        while let Some(node) = stack.pop() {
            if self.cancel.is_cancelled() || count >= MAX_SYNTAX_DIAGNOSTICS {
                break;
            }
            if node.kind() == "ERROR" {
                self.diagnostics.push(ParseDiagnostic::error(
                    "syntax_error",
                    format!(
                        "syntax error near `{}`",
                        WebParser::text(node, self.content)
                            .chars()
                            .take(40)
                            .collect::<String>()
                    ),
                    Some(WebParser::node_location(node, false)),
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

impl LanguageParser for WebParser {
    fn language(&self) -> Language {
        self.language
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
            .set_language(&self.ts_language)
            .map_err(|err| PrismError::Parse {
                message: format!(
                    "failed to load grammar for {:?}: {err} (contract: {COORDINATE_CONTRACT})",
                    self.language
                ),
            })?;
        let tree: Tree = parser
            .parse(&request.content, None)
            .ok_or_else(|| PrismError::Parse {
                message: "tree-sitter returned no tree (encoding or cancellation)".to_string(),
            })?;
        cancellation.check()?;

        let qualified_root = Collector::qualified_root(&request.relative_path);
        let file_symbol = Symbol::new(
            &request.relative_path,
            &qualified_root,
            SymbolKind::File,
            self.language,
            request.file_id.clone(),
            None,
            SymbolVisibility::Unknown,
            SourceRange {
                start: SourceLocation::START,
                end: WebParser::node_location(tree.root_node(), true),
            },
        );

        let file_symbol_id = file_symbol.id.clone();
        let mut collector = Collector {
            content: &request.content,
            file_id: &request.file_id,
            relative_path: &request.relative_path,
            language: self.language,
            symbols: vec![file_symbol],
            diagnostics: Vec::new(),
            imports: Vec::new(),
            cancel: cancellation,
        };
        collector.walk_children(
            tree.root_node(),
            Some(&file_symbol_id),
            &qualified_root,
            false,
        );

        let had_error = tree.root_node().has_error();
        if had_error {
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
            language: Some(self.language),
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
