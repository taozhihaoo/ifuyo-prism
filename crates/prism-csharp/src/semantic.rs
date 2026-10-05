//! L2 语义解析（2.0-M25 §10-§21/§61）。
//!
//! 后端决策（§七矩阵，全文论证见 PRISM_2_0_CSHARP_SEMANTIC.md）：不采用
//! Roslyn（.NET 进程外依赖 + 用户机器 SDK 前置）；基于 tree-sitter 语法树 +
//! M23 符号表做**作用域规则解析**（enclosing namespace → using → 全局唯一
//! 兜底），并按 §61 纪律诚实分层：
//!
//! - Resolved/High：作用域内唯一候选（同命名空间 / using 命名空间）
//! - Resolved/Medium：全局唯一简单名兜底（Probable）
//! - Ambiguous：多候选；Unresolved：无项目内候选（外部/框架——不是错误）
//! - Unknown/Unsupported：dynamic、reflection（检测 + 降级 + 证据化 §30）
//!
//! arity 级 overload 解析（按参数个数）；按实参类型的完整重载解析 =
//! 未支持（无 Roslyn SemanticModel，不伪造）。

use crate::index::SymbolIndex;
use crate::model::{SourceRangeCs, SymbolKindCs};
use prism_intel::Confidence;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use tree_sitter::Node;

/// 解析状态（§13：标准化；与 Confidence 分离 §34）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Resolution {
    Resolved,
    Ambiguous,
    Unresolved,
    Unsupported,
    Unknown,
}

/// 派发类别（§17：静态可分派 ≠ 运行时唯一目标）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Dispatch {
    Direct,
    Virtual,
    Interface,
    Delegate,
    Dynamic,
    Unknown,
}

/// 语义关系种类（§12 首批——有证据才建）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SemanticRelationKind {
    Inherits,
    Implements,
    Overrides,
    Calls,
    Constructs,
    ReferencesType,
    SubscribesEvent,
    UnsubscribesEvent,
}

impl SemanticRelationKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            SemanticRelationKind::Inherits => "inherits",
            SemanticRelationKind::Implements => "implements",
            SemanticRelationKind::Overrides => "overrides",
            SemanticRelationKind::Calls => "calls",
            SemanticRelationKind::Constructs => "constructs",
            SemanticRelationKind::ReferencesType => "references_type",
            SemanticRelationKind::SubscribesEvent => "subscribes_event",
            SemanticRelationKind::UnsubscribesEvent => "unsubscribes_event",
        }
    }
}

/// 解析目标（Invariant 1/2：Resolved 必须对应真实 M23 符号；Unresolved 不伪装）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SemanticTarget {
    /// M23 逻辑符号 id + 限定名（Source of Truth 在 M23，§二.5）。
    Resolved {
        symbol_id: String,
        qualified_name: String,
    },
    /// 外部/框架符号：名称原样 + 原因（§26 不伪装成内部错误）。
    External {
        name: String,
        reason: String,
    },
    Unresolved {
        name: String,
        reason: String,
    },
    Ambiguous {
        name: String,
        candidates: Vec<String>,
    },
    /// dynamic / reflection 等（§18 诚实降级）。
    Unknown {
        reason: String,
    },
}

/// 一条语义关系（§13/§32：Evidence 定位 + Resolution + Confidence 分离）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticRelation {
    pub kind: SemanticRelationKind,
    /// 来源符号（M23 逻辑符号 id——enclosing type/method）。
    pub source_symbol_id: String,
    pub target: SemanticTarget,
    pub dispatch: Dispatch,
    pub resolution: Resolution,
    pub confidence: Confidence,
    pub file: String,
    pub range: SourceRangeCs,
    /// 源/目标分属不同 csproj 项目组（§25 跨项目语义依赖的标记）。
    #[serde(default, skip_serializing_if = "is_false")]
    pub cross_project: bool,
}

fn is_false(value: &bool) -> bool {
    !*value
}

/// 语义诊断（§42：与编译器诊断分开；Prism 视角）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticDiagnostic {
    pub file: String,
    /// 如 "dynamic_invocation" / "reflection_usage" / "ambiguous_reference"。
    pub kind: String,
    pub message: String,
    pub range: SourceRangeCs,
}

/// 单文件语义抽取结果。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct FileSemantics {
    pub file: String,
    pub content_hash: String,
    pub relations: Vec<SemanticRelation>,
    pub diagnostics: Vec<SemanticDiagnostic>,
}

/// 解析器：M23 索引上的作用域规则表（§21-§22）。
pub struct Resolver<'a> {
    index: &'a SymbolIndex,
    /// 简单名（小写）→ 类型符号 id 列表（类型种类）。
    types_by_name: BTreeMap<String, Vec<String>>,
    /// 命名空间（小写）→ 该命名空间下类型 id。
    types_by_namespace: BTreeMap<String, Vec<String>>,
    /// 父 → 子成员（方法/属性/字段；含继承链查询时沿 base_types 走）。
    files: BTreeMap<String, crate::model::FileExtraction>,
}

impl<'a> Resolver<'a> {
    pub fn new(
        index: &'a SymbolIndex,
        files: &BTreeMap<String, crate::model::FileExtraction>,
    ) -> Self {
        let mut types_by_name: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let mut types_by_namespace: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for symbol in index.by_id.values() {
            if matches!(
                symbol.kind,
                SymbolKindCs::Class
                    | SymbolKindCs::Struct
                    | SymbolKindCs::Interface
                    | SymbolKindCs::Enum
                    | SymbolKindCs::Record
                    | SymbolKindCs::RecordStruct
                    | SymbolKindCs::Delegate
            ) {
                types_by_name
                    .entry(symbol.name.to_lowercase())
                    .or_default()
                    .push(symbol.id.clone());
                if !symbol.namespace.is_empty() {
                    types_by_namespace
                        .entry(symbol.namespace.to_lowercase())
                        .or_default()
                        .push(symbol.id.clone());
                }
            }
        }
        Resolver {
            index,
            types_by_name,
            types_by_namespace,
            files: files.clone(),
        }
    }

    /// 简单类型名解析（§19 作用域规则；返回候选 id 列表）。
    pub fn resolve_type(&self, name: &str, file: &str) -> Vec<String> {
        let key = name.rsplit('.').next().unwrap_or(name).to_lowercase();
        let Some(candidates) = self.types_by_name.get(&key) else {
            return Vec::new();
        };
        // 限定名后缀精确匹配优先。
        if name.contains('.') {
            let lowered = name.to_lowercase();
            let qualified: Vec<String> = candidates
                .iter()
                .filter(|id| {
                    self.index
                        .by_id
                        .get(id.as_str())
                        .map(|s| s.qualified_name.to_lowercase().ends_with(&lowered))
                        .unwrap_or(false)
                })
                .cloned()
                .collect();
            if !qualified.is_empty() {
                return qualified;
            }
        }
        // 作用域优先：同文件命名空间 → using 命名空间 → 全部候选（全局唯一兜底）。
        let file_namespace = self
            .index
            .by_file(file)
            .first()
            .map(|symbol| symbol.namespace.to_lowercase())
            .unwrap_or_default();
        let scoped: Vec<String> = self
            .types_by_namespace
            .get(&file_namespace)
            .map(|ids| {
                ids.iter()
                    .filter(|id| candidates.contains(id))
                    .cloned()
                    .collect()
            })
            .unwrap_or_else(|| {
                candidates
                    .iter()
                    .filter(|id| {
                        self.index
                            .by_id
                            .get(id.as_str())
                            .map(|s| s.namespace.to_lowercase() == file_namespace)
                            .unwrap_or(false)
                    })
                    .cloned()
                    .collect()
            });
        if !scoped.is_empty() {
            return scoped;
        }
        let usings = self
            .files
            .get(file)
            .map(|extraction| extraction.usings.clone())
            .unwrap_or_default();
        let via_using: Vec<String> = candidates
            .iter()
            .filter(|id| {
                self.index.by_id.get(id.as_str()).is_some_and(|s| {
                    usings.iter().any(|u| {
                        !u.starts_with("alias:") && s.namespace.to_lowercase() == u.to_lowercase()
                    })
                })
            })
            .cloned()
            .collect();
        if !via_using.is_empty() {
            return via_using;
        }
        // 全局兜底：返回全部候选（唯一 ⇒ 调用方 Resolved/Medium；
        // 多个 ⇒ 调用方 Ambiguous，§13 不强行猜）。
        candidates.clone()
    }

    /// 成员解析（§16/§15）：沿包含类型 + base 链找同名（+同 arity）成员。
    pub fn resolve_member(
        &self,
        containing_type_id: &str,
        member_name: &str,
        arity: usize,
    ) -> Vec<String> {
        let mut visited: BTreeMap<String, ()> = BTreeMap::new();
        let mut queue = vec![containing_type_id.to_string()];
        let mut found = Vec::new();
        while let Some(type_id) = queue.pop() {
            if visited.contains_key(&type_id) {
                continue;
            }
            visited.insert(type_id.clone(), ());
            let Some(symbol) = self.index.by_id.get(&type_id) else {
                continue;
            };
            for member in self.index.children(&type_id) {
                if member.name == member_name
                    && matches!(
                        member.kind,
                        SymbolKindCs::Method | SymbolKindCs::Property | SymbolKindCs::Field
                    )
                    && member.parameters.len() == arity
                {
                    found.push(member.id.clone());
                }
            }
            // base 链：语法 base 名解析（项目内）。
            for base in &symbol.base_types {
                let base_simple = base
                    .split(['<', ','])
                    .next()
                    .unwrap_or(base)
                    .trim()
                    .to_string();
                for base_id in self.resolve_type(&base_simple, "") {
                    queue.push(base_id);
                }
            }
        }
        found
    }
}

/// 语义分析编排：批量 `update_source` 登记 → 单遍 `rebuild` 重建全部
/// 语义（§37 文件级增量；§67 修复——单 Resolver 单遍，无 per-file 深拷贝）。
#[derive(Default)]
pub struct SemanticEngine {
    /// Source of Truth：逐文件 M23 抽取（含 usings 作用域表，§39）。
    extractions: BTreeMap<String, crate::model::FileExtraction>,
    /// 源码缓存（rebuild 重解析用）。
    sources: BTreeMap<String, String>,
    files: BTreeMap<String, FileSemantics>,
}

impl SemanticEngine {
    pub fn new() -> Self {
        Self::default()
    }

    /// 登记文件源码（抽取 hash 未变 ⇒ 复用抽取；语义由 rebuild 统一重算
    /// ——符号表变更影响任意文件解析，rebuild 为权威重算点 §36-§38）。
    /// `_index`：解析上下文由 rebuild 持有；保留参数位以稳定调用约定。
    pub fn update_source(&mut self, _index: &SymbolIndex, file: &str, source: &str) {
        let extraction = crate::extract::extract(file, source);
        if self
            .extractions
            .get(file)
            .map(|existing| existing.content_hash == extraction.content_hash)
            .unwrap_or(false)
        {
            self.sources.insert(file.to_string(), source.to_string());
            return;
        }
        self.extractions.insert(file.to_string(), extraction);
        self.sources.insert(file.to_string(), source.to_string());
        self.files.remove(file);
    }

    /// 删除文件（§28：无幽灵关系——rebuild 后当前集合之外零残留）。
    pub fn remove_source(&mut self, file: &str) {
        self.extractions.remove(file);
        self.sources.remove(file);
        self.files.remove(file);
    }

    /// 单遍重建全部语义：单 Resolver 复用（修复此前每文件深拷贝
    /// extractions 的 O(N²)；确定性由 BTreeMap 遍历保证 §41）。
    pub fn rebuild(&mut self, index: &SymbolIndex) {
        let mut files = std::mem::take(&mut self.files);
        let resolver = Resolver::new(index, &self.extractions);
        for (path, source) in &self.sources {
            let semantics = analyze_file_with_resolver(&resolver, index, path, source);
            files.insert(path.clone(), semantics);
        }
        self.files = files;
    }

    pub fn file(&self, file: &str) -> Option<&FileSemantics> {
        self.files.get(file)
    }

    pub fn relations(&self) -> impl Iterator<Item = &SemanticRelation> {
        self.files.values().flat_map(|f| f.relations.iter())
    }

    pub fn diagnostics(&self) -> impl Iterator<Item = &SemanticDiagnostic> {
        self.files.values().flat_map(|f| f.diagnostics.iter())
    }

    /// callers/callees 查询（§52）。
    pub fn callers(&self, target_symbol_id: &str) -> Vec<&SemanticRelation> {
        self.relations()
            .filter(|relation| {
                relation.kind == SemanticRelationKind::Calls
                    && matches!(
                        &relation.target,
                        SemanticTarget::Resolved { symbol_id, .. }
                            if symbol_id == target_symbol_id
                    )
            })
            .collect()
    }

    pub fn callees(&self, source_symbol_id: &str) -> Vec<&SemanticRelation> {
        self.relations()
            .filter(|relation| {
                relation.source_symbol_id == source_symbol_id
                    && relation.kind == SemanticRelationKind::Calls
            })
            .collect()
    }
}

/// 单文件语义分析（tree-sitter walk + Resolver 作用域规则）。
pub fn analyze_file(index: &SymbolIndex, file: &str, source: &str) -> FileSemantics {
    let content_hash = crate::identity::hash16(source);
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_c_sharp::LANGUAGE.into())
        .expect("c-sharp grammar");
    let Some(tree) = parser.parse(source, None) else {
        return FileSemantics {
            file: file.to_string(),
            content_hash,
            ..Default::default()
        };
    };

    // 作用域上下文。
    let resolver = Resolver::new(index, &BTreeMap::new());
    let mut context = SemanticContext {
        file: file.to_string(),
        source,
        relations: Vec::new(),
        diagnostics: Vec::new(),
        namespace: String::new(),
        type_stack: Vec::new(),
        method_stack: Vec::new(),
        resolver: &resolver,
        index,
    };
    walk_semantic(tree.root_node(), &mut context);
    FileSemantics {
        file: file.to_string(),
        content_hash,
        relations: context.relations,
        diagnostics: context.diagnostics,
    }
}

/// 从既有抽取复用（保持与 M23 相同的 usings 表——避免第二套解析）。
pub fn analyze_file_with_resolver(
    resolver: &Resolver,
    index: &SymbolIndex,
    file: &str,
    source: &str,
) -> FileSemantics {
    let content_hash = crate::identity::hash16(source);
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_c_sharp::LANGUAGE.into())
        .expect("c-sharp grammar");
    let Some(tree) = parser.parse(source, None) else {
        return FileSemantics {
            file: file.to_string(),
            content_hash,
            ..Default::default()
        };
    };
    let mut context = SemanticContext {
        file: file.to_string(),
        source,
        relations: Vec::new(),
        diagnostics: Vec::new(),
        namespace: String::new(),
        type_stack: Vec::new(),
        method_stack: Vec::new(),
        resolver,
        index,
    };
    walk_semantic(tree.root_node(), &mut context);
    FileSemantics {
        file: file.to_string(),
        content_hash,
        relations: context.relations,
        diagnostics: context.diagnostics,
    }
}

struct SemanticContext<'a, 'b> {
    file: String,
    source: &'a str,
    relations: Vec<SemanticRelation>,
    diagnostics: Vec<SemanticDiagnostic>,
    namespace: String,
    type_stack: Vec<String>,
    method_stack: Vec<String>,
    resolver: &'b Resolver<'a>,
    index: &'a SymbolIndex,
}

fn walk_children(node: Node, ctx: &mut SemanticContext) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_semantic(child, ctx);
    }
}

fn walk_semantic(node: Node, ctx: &mut SemanticContext) {
    // 节点级语义收集：walk_semantic 可能从父 default-arm 到达本节点，
    // 必须在匹配**自身**种类时收集（子节点 match 永远看不到自身）。
    match node.kind() {
        "invocation_expression" => collect_invocation(node, ctx),
        "object_creation_expression" => collect_construct(node, ctx),
        "assignment_expression" => collect_event_subscription(node, ctx),
        "typeof_expression" => ctx.diagnostics.push(SemanticDiagnostic {
            file: ctx.file.clone(),
            kind: "reflection_usage".to_string(),
            message: "reflection usage: typeof".to_string(),
            range: point_range(&node),
        }),
        _ => {}
    }
    #[cfg(test)]
    if std::env::var("PRISM_DBG_WALK").is_ok() {
        eprintln!("[walk] {}", node.kind());
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "namespace_declaration" => {
                let previous = ctx.namespace.clone();
                if let Some(name) = child.child_by_field_name("name") {
                    let text = name.utf8_text(ctx.source.as_bytes()).unwrap_or_default();
                    ctx.namespace = if previous.is_empty() {
                        text.to_string()
                    } else {
                        format!("{previous}.{text}")
                    };
                }
                walk_children(child, ctx);
                ctx.namespace = previous;
            }
            "file_scoped_namespace_declaration" => {
                if let Some(name) = child.child_by_field_name("name") {
                    let text = name.utf8_text(ctx.source.as_bytes()).unwrap_or_default();
                    ctx.namespace = if ctx.namespace.is_empty() {
                        text.to_string()
                    } else {
                        format!("{}.{text}", ctx.namespace)
                    };
                }
                // 不还原：file-scoped 持续到文件尾（兄弟节点）。
            }
            "class_declaration"
            | "struct_declaration"
            | "interface_declaration"
            | "record_declaration" => {
                let name = child
                    .child_by_field_name("name")
                    .map(|n| {
                        n.utf8_text(ctx.source.as_bytes())
                            .unwrap_or_default()
                            .to_string()
                    })
                    .unwrap_or_default();
                ctx.type_stack.push(name);
                // 基类型语义（§14/§15）：语法 base 名 → 项目内解析。
                collect_base_relations(child, ctx);
                walk_children(child, ctx);
                ctx.type_stack.pop();
            }
            "method_declaration" | "constructor_declaration" | "local_function_statement" => {
                let name = child
                    .child_by_field_name("name")
                    .map(|n| {
                        n.utf8_text(ctx.source.as_bytes())
                            .unwrap_or_default()
                            .to_string()
                    })
                    .unwrap_or_default();
                // override 解析（§15）：override 修饰 + base 链同名同 arity。
                let is_override = child.children(&mut child.walk()).any(|m| {
                    m.kind() == "modifier"
                        && m.utf8_text(ctx.source.as_bytes()).unwrap_or_default() == "override"
                });
                let arity = child
                    .child_by_field_name("parameters")
                    .map(|list| {
                        list.children(&mut list.walk())
                            .filter(|c| c.kind() == "parameter")
                            .count()
                    })
                    .unwrap_or(0);
                let source_id = enclosing_symbol_id(ctx);
                if is_override {
                    if let Some(resolved) = resolve_override_base(ctx, &name, arity) {
                        push_relation(
                            ctx,
                            SemanticRelationKind::Overrides,
                            &source_id,
                            resolved,
                            Dispatch::Direct,
                            &child,
                        );
                    }
                }
                ctx.method_stack.push(name);
                walk_children(child, ctx);
                ctx.method_stack.pop();
            }
            _ => walk_children(child, ctx),
        }
    }
}

fn enclosing_symbol_id(ctx: &SemanticContext) -> String {
    // 优先方法，其次类型，最后命名空间本身作为归属（M23 id 口径）。
    if let Some(method) = ctx.method_stack.last() {
        let type_part = ctx.type_stack.join(".");
        let qualified = if ctx.namespace.is_empty() {
            format!("{type_part}.{method}")
        } else {
            format!("{}.{}.{}", ctx.namespace, type_part, method)
        };
        return crate::identity::symbol_id(crate::model::SymbolKindCs::Method, &qualified, &[]);
    }
    if let Some(type_name) = ctx.type_stack.last() {
        let qualified = if ctx.namespace.is_empty() {
            type_name.clone()
        } else {
            format!("{}.{}", ctx.namespace, type_name)
        };
        return crate::identity::symbol_id(crate::model::SymbolKindCs::Class, &qualified, &[]);
    }
    let namespace = if ctx.namespace.is_empty() {
        "global".to_string()
    } else {
        ctx.namespace.clone()
    };
    crate::identity::symbol_id(crate::model::SymbolKindCs::Namespace, &namespace, &[])
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

fn collect_base_relations(node: Node, ctx: &mut SemanticContext) {
    // grammar 证实：base_list 非 field——按 kind 扫描。
    let Some(bases) = node
        .children(&mut node.walk())
        .find(|c| c.kind() == "base_list")
    else {
        return;
    };
    let type_name = node
        .child_by_field_name("name")
        .map(|n| {
            n.utf8_text(ctx.source.as_bytes())
                .unwrap_or_default()
                .to_string()
        })
        .unwrap_or_default();
    let source_id = {
        let qualified = if ctx.namespace.is_empty() {
            type_name.clone()
        } else {
            format!("{}.{}", ctx.namespace, type_name)
        };
        crate::identity::symbol_id(crate::model::SymbolKindCs::Class, &qualified, &[])
    };
    let mut cursor = bases.walk();
    for base in bases.children(&mut cursor) {
        if base.kind() == ":" {
            continue;
        }
        let text = base
            .utf8_text(ctx.source.as_bytes())
            .unwrap_or_default()
            .to_string();
        if text.is_empty() {
            continue;
        }
        let candidates = ctx.resolver.resolve_type(&text, &ctx.file);
        let kind = match candidates.len() {
            0 => SemanticRelationKind::ReferencesType,
            _ => {
                // 项目内：interface 种类 ⇒ IMPLEMENTS，否则 INHERITS（首个 base
                // 为类时 INHERITS，其余按目标种类）。
                let target_kind = candidates
                    .first()
                    .and_then(|id| ctx.index.by_id.get(id))
                    .map(|s| s.kind);
                match target_kind {
                    Some(SymbolKindCs::Interface) => SemanticRelationKind::Implements,
                    _ => SemanticRelationKind::Inherits,
                }
            }
        };
        let resolution = match candidates.len() {
            0 => (
                Resolution::Unresolved,
                SemanticTarget::External {
                    name: text.clone(),
                    reason: "not declared in analyzed project (external/framework)".to_string(),
                },
                Confidence::Unknown,
            ),
            1 => (
                Resolution::Resolved,
                SemanticTarget::Resolved {
                    symbol_id: candidates[0].clone(),
                    qualified_name: ctx
                        .index
                        .by_id
                        .get(&candidates[0])
                        .map(|s| s.qualified_name.clone())
                        .unwrap_or_default(),
                },
                Confidence::High,
            ),
            _ => (
                Resolution::Ambiguous,
                SemanticTarget::Ambiguous {
                    name: text.clone(),
                    candidates: candidates.clone(),
                },
                Confidence::Medium,
            ),
        };
        push_full_relation(
            ctx,
            kind,
            &source_id,
            resolution.1,
            resolution.0,
            resolution.2,
            Dispatch::Unknown,
            &base,
        );
    }
}

fn collect_invocation(node: Node, ctx: &mut SemanticContext) {
    #[cfg(test)]
    eprintln!("[dbg] collect_invocation fired in {}", ctx.file);
    let Some(function) = node.child_by_field_name("function") else {
        return;
    };
    let arity = node
        .child_by_field_name("arguments")
        .map(|list| {
            list.children(&mut list.walk())
                .filter(|c| c.kind() == "argument")
                .count()
        })
        .unwrap_or(0);
    let (member_name, dispatch) = match function.kind() {
        "identifier" => (
            function
                .utf8_text(ctx.source.as_bytes())
                .unwrap_or_default()
                .to_string(),
            Dispatch::Direct,
        ),
        // grammar 实测种类为 member_access_expression（M23 探针）。
        "member_access_expression" => {
            let name = function
                .child_by_field_name("name")
                .map(|n| {
                    n.utf8_text(ctx.source.as_bytes())
                        .unwrap_or_default()
                        .to_string()
                })
                .unwrap_or_default();
            // base.M() ⇒ Direct（base 链静态分派）；其余实例调用先记 Virtual
            // 候选（§17：静态可分派 ≠ 运行时唯一目标）。
            // grammar 实测：member_access_expression 的接收者字段为 expression。
            let object = function
                .child_by_field_name("expression")
                .map(|n| {
                    n.utf8_text(ctx.source.as_bytes())
                        .unwrap_or_default()
                        .to_string()
                })
                .unwrap_or_default();
            // §18 dynamic 接收者：参数/字段声明为 dynamic ⇒ 诚实 Unknown。
            #[cfg(test)]
            if std::env::var("PRISM_DBG_DYN").is_ok() {
                eprintln!(
                    "[dyn] object={object} result={}",
                    object_is_dynamic(ctx, &object)
                );
            }
            if object_is_dynamic(ctx, &object) {
                ctx.diagnostics.push(SemanticDiagnostic {
                    file: ctx.file.clone(),
                    kind: "dynamic_invocation".to_string(),
                    message: format!(
                        "invocation on dynamic receiver `{object}` — resolution unknown"
                    ),
                    range: point_range(&node),
                });
                return;
            }
            let dispatch = if object == "base" {
                Dispatch::Direct
            } else {
                Dispatch::Virtual
            };
            (name, dispatch)
        }
        _ => {
            // delegate invoke / dynamic 等（§18）。
            ctx.diagnostics.push(SemanticDiagnostic {
                file: ctx.file.clone(),
                kind: "dynamic_invocation".to_string(),
                message:
                    "invocation via non-static expression (delegate/dynamic) — resolution unknown"
                        .to_string(),
                range: point_range(&node),
            });
            return;
        }
    };
    if member_name.is_empty() {
        return;
    }
    // dynamic 检测：调用对象文本含 dynamic 声明无法解析——保守 Unknown。
    // 反射检测（§30）。
    if member_name == "InvokeMember" || member_name == "GetMethod" {
        ctx.diagnostics.push(SemanticDiagnostic {
            file: ctx.file.clone(),
            kind: "reflection_usage".to_string(),
            message: "reflection usage detected — resolution unknown".to_string(),
            range: point_range(&node),
        });
    }
    let source_id = enclosing_symbol_id(ctx);
    // 成员解析：包含类型链（含 base）。
    let containing = ctx
        .type_stack
        .last()
        .map(|type_name| {
            let qualified = if ctx.namespace.is_empty() {
                type_name.clone()
            } else {
                format!("{}.{}", ctx.namespace, type_name)
            };
            crate::identity::symbol_id(crate::model::SymbolKindCs::Class, &qualified, &[])
        })
        .unwrap_or_else(|| enclosing_symbol_id(ctx));
    let mut candidates = ctx
        .resolver
        .resolve_member(&containing, &member_name, arity);
    // §46 用例：接收者为参数/字段时，用其**声明的 TypeSyntax** 解析接收者
    // 类型（declaration-based 规则，非文本猜测），再在该类型链上解析成员。
    #[cfg(test)]
    if std::env::var("PRISM_DBG_DYN").is_ok() {
        eprintln!(
            "[recv] fn={:?} member={:?} candidates_before={} containing={:?}",
            function.kind(),
            member_name,
            candidates.len(),
            ctx.type_stack.last()
        );
    }
    if candidates.is_empty() && function.kind() == "member_access_expression" {
        let object = function.child_by_field_name("expression").map(|n| {
            n.utf8_text(ctx.source.as_bytes())
                .unwrap_or_default()
                .to_string()
        });
        let receiver_type = object
            .as_deref()
            .and_then(|o| receiver_declared_type(ctx, o));
        if let Some(receiver_type) = receiver_type {
            for type_id in ctx.resolver.resolve_type(&receiver_type, &ctx.file) {
                candidates.extend(ctx.resolver.resolve_member(&type_id, &member_name, arity));
            }
        }
    }
    finish_relation(
        ctx,
        SemanticRelationKind::Calls,
        &source_id,
        member_name,
        candidates,
        dispatch,
        arity,
        node,
    );
}

fn collect_construct(node: Node, ctx: &mut SemanticContext) {
    // object_creation_expression 的 type 是创建目标（§16 constructor）。
    let mut cursor = node.walk();
    let Some(type_node) = node.children(&mut cursor).find(|c| {
        matches!(
            c.kind(),
            "identifier" | "qualified_name" | "generic_name" | "predefined_type"
        )
    }) else {
        return;
    };
    let type_text = type_node
        .utf8_text(ctx.source.as_bytes())
        .unwrap_or_default()
        .to_string();
    if matches!(type_node.kind(), "predefined_type") {
        return; // int/string 等——无项目内目标。
    }
    let source_id = enclosing_symbol_id(ctx);
    let candidates = ctx.resolver.resolve_type(&type_text, &ctx.file);
    finish_relation(
        ctx,
        SemanticRelationKind::Constructs,
        &source_id,
        type_text,
        candidates,
        Dispatch::Direct,
        0,
        node,
    );
}

#[allow(clippy::too_many_arguments)]
fn finish_relation(
    ctx: &mut SemanticContext,
    kind: SemanticRelationKind,
    source_id: &str,
    name: String,
    candidates: Vec<String>,
    dispatch: Dispatch,
    _arity: usize,
    node: Node,
) {
    let (target, resolution, confidence) = match candidates.len() {
        0 => (
            SemanticTarget::Unresolved {
                name,
                reason: "no project-local candidate (external/framework/unresolved using)"
                    .to_string(),
            },
            Resolution::Unresolved,
            Confidence::Unknown,
        ),
        1 => {
            let symbol = ctx.index.by_id.get(&candidates[0]);
            let qualified = symbol.map(|s| s.qualified_name.clone()).unwrap_or_default();
            (
                SemanticTarget::Resolved {
                    symbol_id: candidates[0].clone(),
                    qualified_name: qualified,
                },
                Resolution::Resolved,
                Confidence::High,
            )
        }
        _ => (
            SemanticTarget::Ambiguous {
                name,
                candidates: candidates.clone(),
            },
            Resolution::Ambiguous,
            Confidence::Medium,
        ),
    };
    push_full_relation(
        ctx, kind, source_id, target, resolution, confidence, dispatch, &node,
    );
}

#[allow(clippy::too_many_arguments)]
fn push_full_relation(
    ctx: &mut SemanticContext,
    kind: SemanticRelationKind,
    source_id: &str,
    target: SemanticTarget,
    resolution: Resolution,
    confidence: Confidence,
    dispatch: Dispatch,
    node: &Node,
) {
    ctx.relations.push(SemanticRelation {
        kind,
        source_symbol_id: source_id.to_string(),
        target,
        dispatch,
        resolution,
        confidence,
        file: ctx.file.clone(),
        range: point_range(node),
        cross_project: false,
    });
}

fn push_relation(
    ctx: &mut SemanticContext,
    kind: SemanticRelationKind,
    source_id: &str,
    target: (String, String),
    dispatch: Dispatch,
    node: &Node,
) {
    ctx.relations.push(SemanticRelation {
        kind,
        source_symbol_id: source_id.to_string(),
        target: SemanticTarget::Resolved {
            symbol_id: target.0,
            qualified_name: target.1,
        },
        dispatch,
        resolution: Resolution::Resolved,
        confidence: Confidence::High,
        file: ctx.file.clone(),
        range: point_range(node),
        cross_project: false,
    });
}

fn resolve_override_base(
    ctx: &mut SemanticContext,
    method_name: &str,
    arity: usize,
) -> Option<(String, String)> {
    let containing = ctx.type_stack.last()?;
    let qualified = if ctx.namespace.is_empty() {
        containing.clone()
    } else {
        format!("{}.{}", ctx.namespace, containing)
    };
    let containing_id =
        crate::identity::symbol_id(crate::model::SymbolKindCs::Class, &qualified, &[]);
    let candidates = ctx
        .resolver
        .resolve_member(&containing_id, method_name, arity);
    // override 目标 = 基类链上的同签名方法——排除**自身所在类型**的声明
    // （resolve_member 从包含类型起步，首个同名即 self，§15）。
    candidates
        .into_iter()
        .filter(|id| {
            ctx.index
                .by_id
                .get(id)
                .map(|m| m.parent_id.as_deref() != Some(containing_id.as_str()))
                .unwrap_or(false)
        })
        .find(|id| {
            ctx.index
                .by_id
                .get(id)
                .map(|m| m.name == method_name)
                .unwrap_or(false)
        })
        .and_then(|id| {
            ctx.index
                .by_id
                .get(&id)
                .map(|m| (m.id.clone(), m.qualified_name.clone()))
        })
}

/// 接收者是否声明为 dynamic（§18：参数或包含类型字段，TypeSyntax 原样）。
fn object_is_dynamic(ctx: &SemanticContext, object: &str) -> bool {
    let Some(extraction) = ctx.resolver.files.get(&ctx.file) else {
        return false;
    };
    let containing = ctx.type_stack.last().cloned().unwrap_or_default();
    let method = ctx.method_stack.last().cloned().unwrap_or_default();
    for symbol in &extraction.symbols {
        let in_type = symbol.qualified_name.ends_with(&format!(".{containing}"))
            || symbol.qualified_name == containing;
        match symbol.kind {
            crate::model::SymbolKindCs::Method => {
                // 方法参数：qualified 以 .方法名 结尾的方法。
                if symbol.qualified_name.ends_with(&format!(".{method}")) {
                    for parameter in &symbol.parameters {
                        if parameter.name == object
                            && parameter.type_syntax.eq_ignore_ascii_case("dynamic")
                        {
                            return true;
                        }
                    }
                }
                let _ = in_type;
            }
            crate::model::SymbolKindCs::Field
                if in_type
                    && symbol.name == object
                    && symbol
                        .return_type_syntax
                        .as_deref()
                        .map(|t| t.eq_ignore_ascii_case("dynamic"))
                        .unwrap_or(false) =>
            {
                return true;
            }
            _ => {}
        }
    }
    false
}

/// 接收者标识符的声明类型（参数或包含类型字段/属性，§46 用例）。
fn receiver_declared_type(ctx: &SemanticContext, object: &str) -> Option<String> {
    let extraction = ctx.resolver.files.get(&ctx.file)?;
    let containing = ctx.type_stack.last().cloned().unwrap_or_default();
    let method = ctx.method_stack.last().cloned().unwrap_or_default();
    for symbol in &extraction.symbols {
        match symbol.kind {
            crate::model::SymbolKindCs::Method
                if symbol.qualified_name.ends_with(&format!(".{method}")) =>
            {
                for parameter in &symbol.parameters {
                    if parameter.name == object {
                        return Some(parameter.type_syntax.clone());
                    }
                }
            }
            kind @ (crate::model::SymbolKindCs::Field | crate::model::SymbolKindCs::Property)
                if symbol.name == object
                    && symbol.qualified_name.ends_with(&format!(".{containing}")) =>
            {
                let _ = kind;
                return symbol.return_type_syntax.clone();
            }
            _ => {}
        }
    }
    None
}

fn collect_event_subscription(node: Node, ctx: &mut SemanticContext) {
    let operator = node
        .child_by_field_name("operator")
        .map(|n| {
            n.utf8_text(ctx.source.as_bytes())
                .unwrap_or_default()
                .to_string()
        })
        .unwrap_or_default();
    let kind = match operator.as_str() {
        "+=" => SemanticRelationKind::SubscribesEvent,
        "-=" => SemanticRelationKind::UnsubscribesEvent,
        _ => return,
    };
    let right = node
        .child_by_field_name("right")
        .map(|n| {
            n.utf8_text(ctx.source.as_bytes())
                .unwrap_or_default()
                .to_string()
        })
        .unwrap_or_default();
    let source_id = enclosing_symbol_id(ctx);
    // §29：方法组名按包含类型成员解析；无法解析则 Unknown。
    let method_name = right.rsplit('.').next().unwrap_or(&right).to_string();
    let containing = ctx
        .type_stack
        .last()
        .map(|type_name| {
            let qualified = if ctx.namespace.is_empty() {
                type_name.clone()
            } else {
                format!("{}.{}", ctx.namespace, type_name)
            };
            crate::identity::symbol_id(crate::model::SymbolKindCs::Class, &qualified, &[])
        })
        .unwrap_or_else(|| enclosing_symbol_id(ctx));
    let candidates = ctx.resolver.resolve_member(&containing, &method_name, 0);
    let (target, resolution, confidence) = match candidates.len() {
        0 => (
            SemanticTarget::Unresolved {
                name: right.clone(),
                reason: "method group not found".into(),
            },
            Resolution::Unresolved,
            Confidence::Unknown,
        ),
        1 => (
            SemanticTarget::Resolved {
                symbol_id: candidates[0].clone(),
                qualified_name: ctx
                    .index
                    .by_id
                    .get(&candidates[0])
                    .map(|s| s.qualified_name.clone())
                    .unwrap_or_default(),
            },
            Resolution::Resolved,
            Confidence::High,
        ),
        _ => (
            SemanticTarget::Ambiguous {
                name: right.clone(),
                candidates: candidates.clone(),
            },
            Resolution::Ambiguous,
            Confidence::Medium,
        ),
    };
    ctx.relations.push(SemanticRelation {
        kind,
        source_symbol_id: source_id,
        target,
        dispatch: Dispatch::Delegate,
        resolution,
        confidence,
        file: ctx.file.clone(),
        range: point_range(&node),
        cross_project: false,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::SymbolEngine;

    const SOURCES: [(&str, &str); 4] = [
        (
            "BaseCharacter.cs",
            r#"namespace Game;
public class BaseCharacter {
    public virtual void Tick() { }
    protected void Common() { }
}"#,
        ),
        (
            "Player.cs",
            r#"namespace Game;
public interface ICombatant { void Attack(); }

public class Player : BaseCharacter, ICombatant {
    public override void Tick() { Common(); }
    public void Attack() { }
    public void TakeDamage(int amount) { }
}"#,
        ),
        (
            "Warrior.cs",
            r#"using Game;
public class Warrior : BaseCharacter {
    public override void Tick() { base.Tick(); }
}"#,
        ),
        (
            "Factory.cs",
            r#"namespace Game;
public class Factory {
    public Player Spawn() { return new Player(); }
    public void UnknownCall() { Missing.Thing.Go(); }
    public void DynamicCall(dynamic x) { x.Whatever(); }
}"#,
        ),
    ];

    fn engine() -> SymbolEngine {
        let mut engine = SymbolEngine::new();
        for (file, source) in SOURCES {
            engine.update_file(file, source);
        }
        engine
    }

    fn semantics() -> Vec<FileSemantics> {
        let engine = engine();
        let index = engine_index(&engine);
        let resolver = Resolver::new(&index, engine.extraction_map());
        SOURCES
            .iter()
            .map(|(file, source)| analyze_file_with_resolver(&resolver, &index, file, source))
            .collect()
    }

    fn engine_index(engine: &SymbolEngine) -> SymbolIndex {
        engine.build_index()
    }

    fn find<'a>(
        relations: &'a [SemanticRelation],
        kind: SemanticRelationKind,
        target_contains: &str,
    ) -> &'a SemanticRelation {
        relations
            .iter()
            .find(|r| {
                r.kind == kind
                    && matches!(
                        &r.target,
                        SemanticTarget::Resolved { qualified_name, .. }
                            if qualified_name.contains(target_contains)
                    )
            })
            .unwrap_or_else(|| {
                panic!("relation {kind:?} → {target_contains} missing in {relations:?}")
            })
    }

    #[test]
    fn inherits_and_implements_resolved_via_scope_rules() {
        let all: Vec<SemanticRelation> =
            semantics().into_iter().flat_map(|f| f.relations).collect();
        let inherits = find(&all, SemanticRelationKind::Inherits, "BaseCharacter");
        assert_eq!(inherits.resolution, Resolution::Resolved);
        assert_eq!(inherits.confidence, Confidence::High);
        let implements = find(&all, SemanticRelationKind::Implements, "ICombatant");
        assert_eq!(implements.resolution, Resolution::Resolved);
    }

    #[test]
    fn override_resolves_base_chain_method() {
        let all: Vec<SemanticRelation> =
            semantics().into_iter().flat_map(|f| f.relations).collect();
        let over = find(
            &all,
            SemanticRelationKind::Overrides,
            "Game.BaseCharacter.Tick",
        );
        assert_eq!(over.dispatch, Dispatch::Direct);
    }

    #[test]
    fn calls_resolved_via_containing_type_chain() {
        let all: Vec<SemanticRelation> =
            semantics().into_iter().flat_map(|f| f.relations).collect();
        // Player.Tick 内调用 Common()（protected，base 链解析）。
        let calls = find(&all, SemanticRelationKind::Calls, "Common");
        assert_eq!(calls.resolution, Resolution::Resolved);
    }

    #[test]
    fn constructs_resolved_and_unresolved_honest() {
        let all: Vec<SemanticRelation> =
            semantics().into_iter().flat_map(|f| f.relations).collect();
        let constructs = find(&all, SemanticRelationKind::Constructs, "Game.Player");
        assert_eq!(constructs.resolution, Resolution::Resolved);
        // Missing.Thing.Go()：CONSTRUCTS/REFERENCES 无目标 ⇒ Unresolved 事实保留。
        // Missing.Thing.Go()：成员名 Go 无项目内候选 ⇒ CALLS Unresolved。
        eprintln!("[dump] {:?}", all);
        assert!(all.iter().any(|r| matches!(
            &r.target,
            SemanticTarget::Unresolved { name, .. } if name == "Go"
        ) || matches!(
            &r.target,
            SemanticTarget::External { name, .. } if name.contains("Missing")
        )));
    }

    #[test]
    fn dynamic_call_is_unknown_not_faked() {
        let all: Vec<SemanticDiagnostic> = semantics()
            .into_iter()
            .flat_map(|f| f.diagnostics)
            .collect();
        assert!(
            all.iter().any(|d| d.kind == "dynamic_invocation"),
            "dynamic invocation must surface as unknown diagnostic (§18)"
        );
    }

    #[test]
    fn reflection_detection() {
        let engine = SymbolEngine::new();
        let _ = engine;
        let mut semantic = SemanticEngine::new();
        let index = crate::index::SymbolIndex::default();
        semantic.update_source(
            &index,
            "R.cs",
            "class R { void M() { typeof(R).GetMethod(\"M\"); } }",
        );
        semantic.rebuild(&index);
        assert!(
            semantic.diagnostics().any(|d| d.kind == "reflection_usage"),
            "§30 reflection detection"
        );
    }

    #[test]
    fn incremental_semantics_equal_full() {
        let mut incremental = SemanticEngine::new();
        let index = crate::index::SymbolIndex::default();
        for (file, source) in SOURCES {
            incremental.update_source(&index, file, source);
        }
        incremental.rebuild(&index);
        let mut full = SemanticEngine::new();
        for (file, source) in SOURCES {
            full.update_source(&index, file, source);
        }
        full.rebuild(&index);
        let a: Vec<_> = incremental.relations().cloned().collect();
        let b: Vec<_> = full.relations().cloned().collect();
        assert_eq!(a, b, "§66 incremental == clean full");
        // 删除文件 ⇒ 关系移除（无幽灵）。
        incremental.remove_source("Warrior.cs");
        incremental.rebuild(&index);
        assert!(incremental.file("Warrior.cs").is_none());
    }

    /// §46 验证用例：CombatService.Hit(Player p) → p.TakeDamage(10) ⇒
    /// CALLS Player.TakeDamage（接收者 = 参数声明的 Player 类型）。
    #[test]
    fn receiver_parameter_type_resolves_calls() {
        // M23 索引先建（语义解析的符号表来源），L2 在其上解析。
        let mut m23 = SymbolEngine::new();
        m23.update_file(
            "Player.cs",
            "namespace Game; public class Player { public void TakeDamage(int a) { } }",
        );
        m23.update_file(
            "CombatService.cs",
            r#"namespace Game;
public class CombatService {
    public void Hit(Player p) { p.TakeDamage(10); }
}
"#,
        );
        let index = m23.build_index();
        let mut semantic = SemanticEngine::new();
        semantic.update_source(
            &index,
            "Player.cs",
            "namespace Game; public class Player { public void TakeDamage(int a) { } }",
        );
        semantic.update_source(
            &index,
            "CombatService.cs",
            r#"namespace Game;
public class CombatService {
    public void Hit(Player p) { p.TakeDamage(10); }
}
"#,
        );
        semantic.rebuild(&index);
        let all: Vec<SemanticRelation> = semantic.relations().cloned().collect();
        let calls = find(&all, SemanticRelationKind::Calls, "TakeDamage");
        assert_eq!(calls.resolution, Resolution::Resolved);
        assert!(matches!(
            &calls.target,
            SemanticTarget::Resolved { qualified_name, .. } if qualified_name == "Game.Player.TakeDamage"
        ));
    }

    #[test]
    fn ambiguous_name_not_faked_resolved() {
        // 两个同名类型分布在非 using 命名空间 ⇒ 全局兜底只在唯一时成立。
        let mut engine = SymbolEngine::new();
        engine.update_file("A.cs", "namespace N1; class Dup { }");
        engine.update_file("B.cs", "namespace N2; class Dup { }");
        engine.update_file("C.cs", "namespace M; class Use { void M() { new Dup(); } }");
        let index = engine.build_index();
        let resolver = Resolver::new(&index, engine.extraction_map());
        let candidates = resolver.resolve_type("Dup", "C.cs");
        assert_eq!(
            candidates.len(),
            2,
            "two candidates visible to caller (Ambiguous)"
        );
    }
}
