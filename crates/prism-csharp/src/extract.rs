//! tree-sitter-c-sharp 抽取器（2.0-M23 Batch C）。
//!
//! 管线（§20）：source → tree-sitter syntax tree → declaration walk → RawSymbol。
//! 错误恢复（§30）：tree-sitter 增量恢复保证局部有效子树仍可抽取——
//! ERROR 节点只产生诊断，绝不让整个文件/项目失败。

use crate::model::{
    AttributeInfo, FileExtraction, ParamInfo, ParseDiagnostic, RawSymbol, SourceRangeCs,
    SymbolKindCs,
};
use tree_sitter::{Node, Tree};

/// 解析单个 C# 源文件。
pub fn parse(source: &str) -> (Tree, bool) {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_c_sharp::LANGUAGE.into())
        .expect("c-sharp grammar language");
    let tree = parser
        .parse(source, None)
        .expect("tree-sitter parser allocated");
    let has_error = tree.root_node().has_error();
    (tree, has_error)
}

/// 抽取单文件全部声明（§47 支持范围见 Support Matrix）。
pub fn extract(file: &str, source: &str) -> FileExtraction {
    let content_hash = crate::identity::hash16(source);
    let (tree, has_error) = parse(source);
    let mut diagnostics = Vec::new();
    if has_error {
        diagnostics.push(ParseDiagnostic {
            file: file.to_string(),
            message: "parse errors present; recovered declarations still extracted".to_string(),
            range: SourceRangeCs {
                start_line: 1,
                start_column: 1,
                end_line: 1,
                end_column: 1,
            },
            raw: None,
        });
        // 前 5 个 ERROR 节点的位置（§50：保留定位信息）。
        let mut stack = vec![tree.root_node()];
        let mut reported = 0;
        while let Some(node) = stack.pop() {
            if node.kind() == "ERROR" && reported < 5 {
                diagnostics.push(ParseDiagnostic {
                    file: file.to_string(),
                    message: "syntax error (recovered)".to_string(),
                    range: point_range(&node),
                    raw: None,
                });
                reported += 1;
            }
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                stack.push(child);
            }
        }
    }

    let mut context = ExtractContext {
        file: file.to_string(),
        source,
        symbols: Vec::new(),
        namespace: String::new(),
        type_stack: Vec::new(),
        doc_summary: None,
        usings: Vec::new(),
    };
    walk_root(tree.root_node(), &mut context);
    FileExtraction {
        file: file.to_string(),
        content_hash,
        symbols: context.symbols,
        usings: context.usings,
        diagnostics,
    }
}

struct ExtractContext<'a> {
    file: String,
    source: &'a str,
    symbols: Vec<RawSymbol>,
    namespace: String,
    type_stack: Vec<String>,
    /// 下一个声明可能携带的 `///` 文档摘要。
    doc_summary: Option<String>,
    /// using 指令限定名（§21：仅供引用解析作用域规则）。
    usings: Vec<String>,
}

fn walk_root(node: Node, ctx: &mut ExtractContext) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "namespace_declaration" => walk_namespace(child, ctx),
            // file-scoped（§11）：命名空间作用于**后续兄弟声明**——
            // 在此设置 ctx.namespace 且不还原（兄弟在同一循环内继续处理）。
            "file_scoped_namespace_declaration" => {
                let name = child_text(child, "name", ctx.source.as_bytes()).unwrap_or_default();
                let previous = ctx.namespace.clone();
                ctx.namespace = if previous.is_empty() {
                    name.clone()
                } else {
                    format!("{previous}.{name}")
                };
                ctx.symbols.push(RawSymbol {
                    kind: SymbolKindCs::Namespace,
                    name: name.clone(),
                    qualified_name: ctx.namespace.clone(),
                    namespace: previous.clone(),
                    modifiers: Vec::new(),
                    type_parameters: Vec::new(),
                    constraints: Vec::new(),
                    base_types: Vec::new(),
                    attributes: Vec::new(),
                    parameters: Vec::new(),
                    return_type_syntax: None,
                    is_partial: false,
                    doc_summary: None,
                    file: ctx.file.clone(),
                    range: point_range(&child),
                    name_range: name_range(child, "name"),
                });
                // 不还原：file-scoped 语义持续到文件尾。
                let _ = previous;
            }
            "comment" => note_doc_comment(child, ctx),
            "using_directive" => {
                // §21：记录限定名；alias 记 "alias:名字=目标" 原样。
                if let Some(name) = child.child_by_field_name("name") {
                    let text = name.utf8_text(ctx.source.as_bytes()).unwrap_or_default();
                    ctx.usings.push(text.to_string());
                } else {
                    for part in child.children(&mut child.walk()) {
                        if part.kind() == "name_equals" {
                            let text = part.utf8_text(ctx.source.as_bytes()).unwrap_or_default();
                            ctx.usings.push(format!("alias:{text}"));
                        }
                    }
                }
            }
            _ => {
                if is_declaration_kind(child.kind()) {
                    walk_declaration(child, ctx);
                } else {
                    // 顶层语句（top-level statements）等：跳过内容，不失败（§12）。
                    walk_root(child, ctx);
                }
            }
        }
    }
}

/// ERROR 节点内的有效子树仍可抽取（§30 错误恢复核心）。
fn walk_error_container(node: Node, ctx: &mut ExtractContext) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "comment" => note_doc_comment(child, ctx),
            "ERROR" => walk_error_container(child, ctx),
            _ => {
                if is_declaration_kind(child.kind()) {
                    walk_declaration(child, ctx);
                } else if child.child_count() > 0 {
                    walk_error_container(child, ctx);
                }
            }
        }
    }
}

fn is_declaration_kind(kind: &str) -> bool {
    matches!(
        kind,
        "class_declaration"
            | "struct_declaration"
            | "interface_declaration"
            | "enum_declaration"
            | "record_declaration"
            | "delegate_declaration"
            | "method_declaration"
            | "constructor_declaration"
            | "destructor_declaration"
            | "property_declaration"
            | "field_declaration"
            | "event_field_declaration"
            | "event_declaration"
            | "indexer_declaration"
            | "operator_declaration"
            | "conversion_operator_declaration"
    )
}

fn walk_namespace(node: Node, ctx: &mut ExtractContext) {
    let name = child_text(node, "name", ctx.source.as_bytes()).unwrap_or_default();
    let previous = ctx.namespace.clone();
    ctx.namespace = if previous.is_empty() {
        name.clone()
    } else {
        format!("{previous}.{name}")
    };
    ctx.symbols.push(RawSymbol {
        kind: SymbolKindCs::Namespace,
        name: name.clone(),
        qualified_name: ctx.namespace.clone(),
        namespace: previous.clone(),
        modifiers: Vec::new(),
        type_parameters: Vec::new(),
        constraints: Vec::new(),
        base_types: Vec::new(),
        attributes: Vec::new(),
        parameters: Vec::new(),
        return_type_syntax: None,
        is_partial: false,
        doc_summary: ctx.doc_summary.take(),
        file: ctx.file.clone(),
        range: point_range(&node),
        name_range: name_range(node, "name"),
    });
    if let Some(body) = node.child_by_field_name("body") {
        let mut cursor = body.walk();
        for child in body.children(&mut cursor) {
            match child.kind() {
                "namespace_declaration" => walk_namespace(child, ctx),
                "file_scoped_namespace_declaration" => walk_file_scoped_namespace(child, ctx),
                "comment" => note_doc_comment(child, ctx),
                "ERROR" => walk_error_container(child, ctx),
                _ => {
                    if is_declaration_kind(child.kind()) {
                        walk_declaration(child, ctx);
                    }
                }
            }
        }
    }
    ctx.namespace = previous;
}

fn walk_file_scoped_namespace(node: Node, ctx: &mut ExtractContext) {
    let name = child_text(node, "name", ctx.source.as_bytes()).unwrap_or_default();
    let previous = ctx.namespace.clone();
    ctx.namespace = if previous.is_empty() {
        name.clone()
    } else {
        format!("{previous}.{name}")
    };
    ctx.symbols.push(RawSymbol {
        kind: SymbolKindCs::Namespace,
        name: name.clone(),
        qualified_name: ctx.namespace.clone(),
        namespace: previous.clone(),
        modifiers: Vec::new(),
        type_parameters: Vec::new(),
        constraints: Vec::new(),
        base_types: Vec::new(),
        attributes: Vec::new(),
        parameters: Vec::new(),
        return_type_syntax: None,
        is_partial: false,
        doc_summary: ctx.doc_summary.take(),
        file: ctx.file.clone(),
        range: point_range(&node),
        name_range: name_range(node, "name"),
    });
    // file-scoped：其余声明是本节点的兄弟节点——由上层 walk_root 处理。
    ctx.namespace = previous;
}

/// 类型与成员声明（class/struct/interface/enum/record/delegate + 全部成员）。
fn walk_declaration(node: Node, ctx: &mut ExtractContext) {
    let kind = match node.kind() {
        "class_declaration" => SymbolKindCs::Class,
        "struct_declaration" => SymbolKindCs::Struct,
        "interface_declaration" => SymbolKindCs::Interface,
        "enum_declaration" => SymbolKindCs::Enum,
        "record_declaration" => {
            // readonly record struct ⇒ RecordStruct（readonly/struct 以 modifier/keyword 出现）。
            let has_struct_keyword = node
                .children(&mut node.walk())
                .any(|c| c.kind() == "struct");
            let readonly = modifiers_of(node, ctx.source.as_bytes())
                .iter()
                .any(|m| m == "readonly");
            if has_struct_keyword
                || readonly
                    && node.child_by_field_name("type_parameters").is_none()
                    && has_struct_token(node)
            {
                SymbolKindCs::RecordStruct
            } else {
                SymbolKindCs::Record
            }
        }
        "delegate_declaration" => SymbolKindCs::Delegate,
        "method_declaration" => SymbolKindCs::Method,
        "constructor_declaration" => SymbolKindCs::Constructor,
        "destructor_declaration" => SymbolKindCs::Destructor,
        "property_declaration" => SymbolKindCs::Property,
        "field_declaration" => SymbolKindCs::Field,
        "event_field_declaration" | "event_declaration" => SymbolKindCs::Event,
        "indexer_declaration" => SymbolKindCs::Indexer,
        "operator_declaration" | "conversion_operator_declaration" => SymbolKindCs::Operator,
        _ => return,
    };

    let modifiers = modifiers_of(node, ctx.source.as_bytes());
    let is_partial = modifiers.iter().any(|m| m == "partial");
    let attributes = attributes_of(node, ctx.source.as_bytes());
    let name_node = node.child_by_field_name("name");
    let name = name_node
        .map(|n| {
            n.utf8_text(ctx.source.as_bytes())
                .unwrap_or_default()
                .to_string()
        })
        .unwrap_or_default();

    let type_parameters: Vec<String> = node
        .child_by_field_name("type_parameters")
        .map(|list| {
            let mut out = Vec::new();
            let mut cursor = list.walk();
            for child in list.children(&mut cursor) {
                if child.kind() == "type_parameter" {
                    out.push(
                        child
                            .utf8_text(ctx.source.as_bytes())
                            .unwrap_or_default()
                            .to_string(),
                    );
                }
            }
            out
        })
        .unwrap_or_default();
    let constraints: Vec<String> = node
        .children(&mut node.walk())
        .filter(|c| c.kind() == "type_parameter_constraints_clause")
        .map(|c| {
            c.utf8_text(ctx.source.as_bytes())
                .unwrap_or_default()
                .to_string()
        })
        .collect();
    // grammar 证实：base_list 不是 field（class_declaration fields 只有
    // body/name）——按 kind 扫描子节点。
    let base_types: Vec<String> = node
        .children(&mut node.walk())
        .find(|c| c.kind() == "base_list")
        .map(|list| {
            let mut out = Vec::new();
            let mut cursor = list.walk();
            for child in list.children(&mut cursor) {
                if child.kind() == ":" {
                    continue;
                }
                let text = child
                    .utf8_text(ctx.source.as_bytes())
                    .unwrap_or_default()
                    .to_string();
                if !text.is_empty() {
                    out.push(text);
                }
            }
            out
        })
        .unwrap_or_default();

    let parameters: Vec<ParamInfo> = node
        .child_by_field_name("parameters")
        .map(|list| parameters_of(list, ctx.source.as_bytes()))
        .unwrap_or_default();
    let return_type_syntax = node.child_by_field_name("returns").map(|n| {
        n.utf8_text(ctx.source.as_bytes())
            .unwrap_or_default()
            .to_string()
    });

    let qualified = compose_qualified(&ctx.namespace, &ctx.type_stack, &name);
    ctx.symbols.push(RawSymbol {
        kind,
        name: name.clone(),
        qualified_name: qualified,
        namespace: ctx.namespace.clone(),
        modifiers: modifiers.clone(),
        type_parameters,
        constraints,
        base_types,
        attributes: attributes.clone(),
        parameters,
        return_type_syntax,
        is_partial,
        doc_summary: ctx.doc_summary.take(),
        file: ctx.file.clone(),
        range: point_range(&node),
        name_range: name_node
            .map(|n| point_range(&n))
            .unwrap_or(point_range(&node)),
    });

    // 嵌套处理：类型入栈继续下钻；成员在 declaration_list / 参数列表内。
    match kind {
        SymbolKindCs::Class
        | SymbolKindCs::Struct
        | SymbolKindCs::Interface
        | SymbolKindCs::Record
        | SymbolKindCs::RecordStruct => {
            ctx.type_stack.push(name);
            if let Some(body) = node.child_by_field_name("body") {
                let mut cursor = body.walk();
                for child in body.children(&mut cursor) {
                    match child.kind() {
                        "comment" => note_doc_comment(child, ctx),
                        "ERROR" => walk_error_container(child, ctx),
                        _ => {
                            if is_declaration_kind(child.kind()) {
                                walk_declaration(child, ctx);
                            }
                        }
                    }
                }
            }
            ctx.type_stack.pop();
        }
        SymbolKindCs::Method => {
            ctx.type_stack.push(name);
            // 局部函数（§47 local function）：父为方法。
            if let Some(body) = node.child_by_field_name("body") {
                let mut cursor = body.walk();
                for child in body.children(&mut cursor) {
                    if child.kind() == "local_function_statement" {
                        let local_name = child
                            .child_by_field_name("name")
                            .map(|n| {
                                n.utf8_text(ctx.source.as_bytes())
                                    .unwrap_or_default()
                                    .to_string()
                            })
                            .unwrap_or_default();
                        let qualified =
                            compose_qualified(&ctx.namespace, &ctx.type_stack, &local_name);
                        ctx.symbols.push(RawSymbol {
                            kind: SymbolKindCs::LocalFunction,
                            name: local_name,
                            qualified_name: qualified,
                            namespace: ctx.namespace.clone(),
                            modifiers: modifiers_of(child, ctx.source.as_bytes()),
                            type_parameters: Vec::new(),
                            constraints: Vec::new(),
                            base_types: Vec::new(),
                            attributes: Vec::new(),
                            parameters: child
                                .child_by_field_name("parameters")
                                .map(|list| parameters_of(list, ctx.source.as_bytes()))
                                .unwrap_or_default(),
                            return_type_syntax: child.child_by_field_name("type").map(|n| {
                                n.utf8_text(ctx.source.as_bytes())
                                    .unwrap_or_default()
                                    .to_string()
                            }),
                            is_partial: false,
                            doc_summary: None,
                            file: ctx.file.clone(),
                            range: point_range(&child),
                            name_range: name_range(child, "name"),
                        });
                    }
                }
            }
            ctx.type_stack.pop();
        }
        // field/event-field：多个 declarator 共享一个类型声明（§15）。
        SymbolKindCs::Field | SymbolKindCs::Event => {
            let attributes = attributes.clone();
            let variable_declaration = node
                .children(&mut node.walk())
                .find(|c| c.kind() == "variable_declaration");
            if let Some(declaration) = variable_declaration {
                let mut cursor = declaration.walk();
                for declarator in declaration.children(&mut cursor) {
                    if declarator.kind() != "variable_declarator" {
                        continue;
                    }
                    let declarator_name = declarator.child_by_field_name("name").map(|n| {
                        n.utf8_text(ctx.source.as_bytes())
                            .unwrap_or_default()
                            .to_string()
                    });
                    let Some(declarator_name) = declarator_name else {
                        continue;
                    };
                    let qualified =
                        compose_qualified(&ctx.namespace, &ctx.type_stack, &declarator_name);
                    ctx.symbols.push(RawSymbol {
                        kind,
                        name: declarator_name,
                        qualified_name: qualified,
                        namespace: ctx.namespace.clone(),
                        modifiers: modifiers.clone(),
                        type_parameters: Vec::new(),
                        constraints: Vec::new(),
                        base_types: Vec::new(),
                        attributes: attributes.clone(),
                        parameters: Vec::new(),
                        return_type_syntax: declaration.child_by_field_name("type").map(|n| {
                            n.utf8_text(ctx.source.as_bytes())
                                .unwrap_or_default()
                                .to_string()
                        }),
                        is_partial: false,
                        doc_summary: None,
                        file: ctx.file.clone(),
                        range: point_range(&declarator),
                        name_range: name_range(declarator, "name"),
                    });
                }
            }
        }
        SymbolKindCs::Enum => {
            // 枚举成员（§15）：enum_member_declaration_list 的每个名字。
            ctx.type_stack.push(name);
            let mut outer = node.walk();
            for list in node.children(&mut outer) {
                if list.kind() != "enum_member_declaration_list" {
                    continue;
                }
                let mut inner = list.walk();
                for member in list.children(&mut inner) {
                    if member.kind() != "enum_member_declaration" {
                        continue;
                    }
                    let member_name = member
                        .child_by_field_name("name")
                        .map(|n| {
                            n.utf8_text(ctx.source.as_bytes())
                                .unwrap_or_default()
                                .to_string()
                        })
                        .unwrap_or_default();
                    let qualified =
                        compose_qualified(&ctx.namespace, &ctx.type_stack, &member_name);
                    ctx.symbols.push(RawSymbol {
                        kind: SymbolKindCs::EnumMember,
                        name: member_name,
                        qualified_name: qualified,
                        namespace: ctx.namespace.clone(),
                        modifiers: Vec::new(),
                        type_parameters: Vec::new(),
                        constraints: Vec::new(),
                        base_types: Vec::new(),
                        attributes: Vec::new(),
                        parameters: Vec::new(),
                        return_type_syntax: None,
                        is_partial: false,
                        doc_summary: None,
                        file: ctx.file.clone(),
                        range: point_range(&member),
                        name_range: name_range(member, "name"),
                    });
                }
            }
            ctx.type_stack.pop();
        }
        // 其余成员（ctor/property/indexer/operator/event-with-accessors/delegate）无嵌套声明下钻。
        _ => {}
    }
}

fn has_struct_token(node: Node) -> bool {
    node.children(&mut node.walk())
        .any(|c| c.kind() == "struct")
}

fn compose_qualified(namespace: &str, type_stack: &[String], name: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    if !namespace.is_empty() {
        parts.push(namespace);
    }
    for part in type_stack {
        parts.push(part);
    }
    parts.push(name);
    parts.join(".")
}

fn modifiers_of(node: Node, source: &[u8]) -> Vec<String> {
    node.children(&mut node.walk())
        .filter(|c| c.kind() == "modifier")
        .map(|c| c.utf8_text(source).unwrap_or_default().to_string())
        .collect()
}

fn attributes_of(node: Node, source: &[u8]) -> Vec<AttributeInfo> {
    let mut out = Vec::new();
    let mut outer = node.walk();
    for list in node.children(&mut outer) {
        if list.kind() != "attribute_list" {
            continue;
        }
        let mut inner = list.walk();
        for attribute in list.children(&mut inner) {
            if attribute.kind() != "attribute" {
                continue;
            }
            out.push(AttributeInfo {
                name: attribute
                    .child_by_field_name("name")
                    .map(|n| n.utf8_text(source).unwrap_or_default().to_string())
                    .unwrap_or_default(),
            });
        }
    }
    out
}

fn parameters_of(list: Node, source: &[u8]) -> Vec<ParamInfo> {
    list.children(&mut list.walk())
        .filter(|c| c.kind() == "parameter")
        .map(|parameter| {
            let name = parameter
                .child_by_field_name("name")
                .map(|n| n.utf8_text(source).unwrap_or_default().to_string())
                .unwrap_or_default();
            let type_syntax = parameter
                .child_by_field_name("type")
                .map(|n| n.utf8_text(source).unwrap_or_default().to_string())
                .unwrap_or_default();
            ParamInfo { name, type_syntax }
        })
        .collect()
}

fn child_text(node: Node, field: &str, source: &[u8]) -> Option<String> {
    node.child_by_field_name(field)
        .map(|n| n.utf8_text(source).unwrap_or_default().to_string())
}

fn point_range(node: &Node) -> SourceRangeCs {
    let start = node.start_position();
    let end = node.end_position();
    SourceRangeCs {
        start_line: start.row as u32 + 1,
        start_column: start.column as u32 + 1,
        end_line: end.row as u32 + 1,
        end_column: end.column as u32 + 1,
    }
}

fn name_range(node: Node, field: &str) -> SourceRangeCs {
    node.child_by_field_name(field)
        .map(|n| point_range(&n))
        .unwrap_or(point_range(&node))
}

/// `///` 文档注释（§18：首行摘要；其余注释忽略）。
fn note_doc_comment(node: Node, ctx: &mut ExtractContext) {
    let text = node.utf8_text(ctx.source.as_bytes()).unwrap_or_default();
    let trimmed = text.trim_start();
    if let Some(doc) = trimmed.strip_prefix("///") {
        // XML 文档（§18）：剥标签取纯文本摘要，不做自然语言分析。
        let mut summary = String::new();
        let mut in_tag = false;
        for ch in doc.trim().chars() {
            match ch {
                '<' => in_tag = true,
                '>' => in_tag = false,
                c if !in_tag => summary.push(c),
                _ => {}
            }
        }
        let summary = summary.trim().to_string();
        if !summary.is_empty() {
            ctx.doc_summary = Some(summary);
        }
    }
}
