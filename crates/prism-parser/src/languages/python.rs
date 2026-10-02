//! Python adapter（tree-sitter-python）。
//!
//! 抽取（规格 §33）：class / function / method / 顶层变量 / 类级字段 /
//! import / from-import / docstring 存在性；可见性按 Python 惯例
//! （前导下划线 = Private）。decorator 完整抽取属 P1，当前只解包
//! decorated_definition 让被装饰声明正常进入管线。

use crate::model::{
    SourceLocation, SourceRange, Symbol, SymbolKind, SymbolVisibility, COORDINATE_CONTRACT,
};
use crate::parse::{
    CallInfo, ImportInfo, ImportKind, ParseDiagnostic, ParseRequest, ParseResult, ParseStatus,
};
use crate::registry::LanguageParser;
use prism_core::ids::{FileId, SymbolId};
use prism_core::task::CancellationToken;
use prism_core::{PrismError, PrismResult};
use prism_project::Language;
use std::time::Instant;
use tree_sitter::{Node, Parser};

pub struct PythonParser {
    version: &'static str,
}

impl Default for PythonParser {
    fn default() -> Self {
        Self::new()
    }
}

impl PythonParser {
    pub fn new() -> Self {
        PythonParser {
            version: "tree-sitter-python/1",
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

    fn visibility_of(name: &str) -> SymbolVisibility {
        if name.starts_with('_') {
            SymbolVisibility::Private
        } else {
            SymbolVisibility::Public
        }
    }

    fn python_signature<'a>(node: Node<'a>, content: &'a str) -> Option<String> {
        let params = node.child_by_field_name("parameters")?;
        let mut signature = Self::text(params, content).to_string();
        if let Some(return_type) = node.child_by_field_name("return_type") {
            signature.push_str(&format!(" -> {}", Self::text(return_type, content)));
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
    calls: Vec<CallInfo>,
    cancel: &'a CancellationToken,
}

impl<'a> Collector<'a> {
    fn qualified_root(relative_path: &str) -> String {
        let without_ext = match relative_path.rsplit_once('.') {
            Some((prefix, ext)) if !ext.is_empty() && ext.len() <= 5 && !ext.contains('/') => {
                prefix.to_string()
            }
            _ => relative_path.to_string(),
        };
        // Python 惯例：路径分隔符 → 点（规格 §17 示例 app.services.user.X）。
        without_ext.replace('/', ".")
    }

    fn emit(
        &mut self,
        node: Node<'a>,
        kind: SymbolKind,
        qualified_path: &str,
        parent_id: Option<&SymbolId>,
        signature: Option<String>,
    ) -> Symbol {
        let name = qualified_path.rsplit('.').next().unwrap_or(qualified_path);
        let mut symbol = Symbol::new(
            self.relative_path,
            qualified_path,
            kind,
            Language::Python,
            self.file_id.clone(),
            parent_id.cloned(),
            PythonParser::visibility_of(name),
            PythonParser::range(node),
        );
        symbol.signature = signature;
        symbol.documentation_present = Self::has_docstring(node, self.content);
        self.symbols.push(symbol.clone());
        symbol
    }

    /// docstring 启发式：body 的第一条语句是字符串字面量。
    fn has_docstring(node: Node<'_>, content: &str) -> bool {
        let body = match node.child_by_field_name("body") {
            Some(body) => body,
            None => return false,
        };
        let first = match body.child(0) {
            Some(child) => child,
            None => return false,
        };
        if first.kind() != "expression_statement" {
            return false;
        }
        let inner = first.child(0);
        inner.is_some_and(|node| node.kind() == "string")
            && PythonParser::text(first, content)
                .trim_start()
                .starts_with('"')
            || PythonParser::text(first, content)
                .trim_start()
                .starts_with('\'')
    }

    fn walk(
        &mut self,
        node: Node<'a>,
        parent_id: Option<&SymbolId>,
        prefix: &str,
        in_class: bool,
        caller: Option<&str>,
    ) {
        if self.cancel.is_cancelled() {
            return;
        }

        match node.kind() {
            "function_definition" => {
                if let Some(name) = PythonParser::child_text(node, "name", self.content) {
                    let kind = if in_class {
                        SymbolKind::Method
                    } else {
                        SymbolKind::Function
                    };
                    let qualified = format!("{prefix}.{name}");
                    let signature = PythonParser::python_signature(node, self.content);
                    let symbol = self.emit(node, kind, &qualified, parent_id, signature);
                    // 函数体不是类作用域：内部赋值按 Variable 处理（且不深入
                    // 抽取局部变量属 P1，这里只保持作用域语义正确）。
                    self.walk_children(node, Some(&symbol.id), &qualified, false, Some(&qualified));
                }
            }
            "class_definition" => {
                if let Some(name) = PythonParser::child_text(node, "name", self.content) {
                    let qualified = format!("{prefix}.{name}");
                    let symbol = self.emit(node, SymbolKind::Class, &qualified, parent_id, None);
                    self.walk_children(node, Some(&symbol.id), &qualified, true, Some(&qualified));
                }
            }
            "decorated_definition" => {
                // P1 才抽取 decorator 本身；这里解包让声明正常进入管线。
                self.walk_children(node, parent_id, prefix, in_class, caller);
            }
            "import_statement" | "import_from_statement" => {
                let module = node
                    .child_by_field_name("module_name")
                    .or_else(|| {
                        node.children(&mut node.walk()).find(|child| {
                            matches!(
                                child.kind(),
                                "dotted_name" | "relative_import" | "wildcard_import"
                            )
                        })
                    })
                    .map(|child| PythonParser::text(child, self.content))
                    .unwrap_or("");
                if !module.is_empty() {
                    self.imports.push(ImportInfo {
                        specifier: module.to_string(),
                        kind: ImportKind::Plain,
                        names: Vec::new(),
                        location: Some(PythonParser::node_location(node, false)),
                    });
                    let qualified = format!("{prefix}.{module}");
                    self.emit(node, SymbolKind::Import, &qualified, parent_id, None);
                }
            }
            "call" => {
                // 规格 §19 可靠子集：直接调用，callee = function 字段文本。
                if let Some(caller) = caller {
                    if let Some(function) = node.child_by_field_name("function") {
                        self.calls.push(CallInfo {
                            caller_qualified: caller.to_string(),
                            callee_text: PythonParser::text(function, self.content).to_string(),
                            location: Some(PythonParser::node_location(node, false)),
                        });
                    }
                }
                self.walk_children(node, parent_id, prefix, in_class, caller);
            }
            "assignment" => {
                // 顶层 → Variable；类体 → Field（局部赋值不抽取，规格 §16）。
                if let Some(name) = PythonParser::child_text(node, "left", self.content) {
                    let kind = if in_class {
                        SymbolKind::Field
                    } else {
                        SymbolKind::Variable
                    };
                    let qualified = format!("{prefix}.{name}");
                    self.emit(node, kind, &qualified, parent_id, None);
                }
            }
            "ERROR" => self.walk_children(node, parent_id, prefix, in_class, caller),
            _ => self.walk_children(node, parent_id, prefix, in_class, caller),
        }
    }

    fn walk_children(
        &mut self,
        node: Node<'a>,
        parent_id: Option<&SymbolId>,
        prefix: &str,
        in_class: bool,
        caller: Option<&str>,
    ) {
        let mut child = node.child(0);
        while let Some(current) = child {
            // block / module 内的直接子语句才推进 in_class 语义；
            // 深层嵌套（函数体内的赋值）不抽取，由 _ 前缀规则之外忽略。
            match current.kind() {
                "block" => self.walk_children(current, parent_id, prefix, in_class, caller),
                _ => self.walk(current, parent_id, prefix, in_class, caller),
            }
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
                        PythonParser::text(node, self.content)
                            .chars()
                            .take(40)
                            .collect::<String>()
                    ),
                    Some(PythonParser::node_location(node, false)),
                ));
                count += 1;
            } else if node.is_missing() {
                // Python 语法错误常以 MISSING 节点表达（无 ERROR 包裹）。
                self.diagnostics.push(ParseDiagnostic::error(
                    "missing_syntax",
                    format!("missing `{}`", node.kind()),
                    Some(PythonParser::node_location(node, false)),
                ));
                count += 1;
            }
            let mut child = node.child(0);
            while let Some(current) = child {
                stack.push(current);
                child = current.next_sibling();
            }
        }
        // has_error 但未定位到具体节点时的兜底（不吞错误，规格 §122 精神）。
        if self.diagnostics.is_empty() {
            self.diagnostics.push(ParseDiagnostic::error(
                "syntax_error",
                "grammar reported an error without a locatable node",
                None,
            ));
        }
        self.diagnostics.sort_by(|a, b| {
            a.location
                .map(|l| l.byte_offset)
                .cmp(&b.location.map(|l| l.byte_offset))
        });
    }
}

impl LanguageParser for PythonParser {
    fn language(&self) -> Language {
        Language::Python
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
            .set_language(&tree_sitter_python::LANGUAGE.into())
            .map_err(|err| PrismError::Parse {
                message: format!(
                    "failed to load python grammar: {err} (contract: {COORDINATE_CONTRACT})"
                ),
            })?;
        let tree = parser
            .parse(&request.content, None)
            .ok_or_else(|| PrismError::Parse {
                message: "tree-sitter returned no tree".to_string(),
            })?;
        cancellation.check()?;

        let qualified_root = Collector::qualified_root(&request.relative_path);
        let file_symbol = Symbol::new(
            &request.relative_path,
            &qualified_root,
            SymbolKind::File,
            Language::Python,
            request.file_id.clone(),
            None,
            SymbolVisibility::Unknown,
            SourceRange {
                start: SourceLocation::START,
                end: PythonParser::node_location(tree.root_node(), true),
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
            calls: Vec::new(),
            cancel: cancellation,
        };
        collector.walk_children(
            tree.root_node(),
            Some(&file_symbol_id),
            &qualified_root,
            false,
            None,
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
            language: Some(Language::Python),
            status,
            symbols: collector.symbols,
            diagnostics: collector.diagnostics,
            duration_ms: started.elapsed().as_millis() as u64,
            parser_version: self.version.to_string(),
            reason: None,
            imports: collector.imports,
            calls: collector.calls,
        })
    }
}
