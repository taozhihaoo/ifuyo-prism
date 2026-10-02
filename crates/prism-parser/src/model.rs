//! 归一化 Symbol 模型与坐标契约。
//!
//! ## 坐标契约（全项目唯一语义，禁止各处自定义）
//!
//! [`COORDINATE_CONTRACT`]：`line` 1-based；`column` 1-based、按行内 UTF-8
//! 字节计数；`byte_offset` 0-based、相对文件起点的绝对 UTF-8 字节。
//! 语言 parser 的原始坐标系不同（如 tree-sitter 的 byte 列）时，
//! 必须在 adapter 内转换为契约坐标。
//!
//! ## 身份契约
//!
//! [`Symbol::new`] 以 `symbol:{relative_path}#{qualified_path}#{kind}` 派生
//! 稳定 [`SymbolId`]：代码结构不变 ⇒ 同一 Symbol ⇒ 同一 ID（禁随机 UUID）。
//!
//! ## 层级
//!
//! Symbol 只存 `parent_id`；children 关系由 prism-analysis 的 SymbolIndex
//! 提供，避免双份簿记不一致。

use prism_core::ids::{FileId, SymbolId};
use prism_project::Language;
use serde::{Deserialize, Serialize};

/// 坐标契约的人类可读形式（文档/审计引用）。
pub const COORDINATE_CONTRACT: &str =
    "line is 1-based; column is 1-based and counted in UTF-8 bytes within the line; \
     byte_offset is 0-based absolute UTF-8 bytes from the start of the file";

/// 一个源码位置。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SourceLocation {
    pub line: u32,
    pub column: u32,
    pub byte_offset: usize,
}

impl SourceLocation {
    pub const START: SourceLocation = SourceLocation {
        line: 1,
        column: 1,
        byte_offset: 0,
    };
}

/// 一个闭区间（含 start、不含 end）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SourceRange {
    pub start: SourceLocation,
    pub end: SourceLocation,
}

/// Symbol 种类（规格 §12）。语言 adapter 只映射能可靠识别的种类，不伪造。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SymbolKind {
    File,
    Module,
    Namespace,
    Class,
    Struct,
    Interface,
    Enum,
    Function,
    Method,
    Constructor,
    Property,
    Field,
    Variable,
    Constant,
    Parameter,
    TypeAlias,
    Import,
    Export,
    Decorator,
    Unknown,
}

/// 可见性（规格 §13）。语言没有对应概念时用 Unknown，不制造虚假语义。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SymbolVisibility {
    Public,
    Private,
    Protected,
    Internal,
    Package,
    Module,
    Unknown,
}

/// 归一化后的 Symbol（不含 children；临时源码/AST 不驻留）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Symbol {
    pub id: SymbolId,
    pub name: String,
    /// 限定名（各语言语义不同，原样呈现，不做跨语言统一假象）。
    pub qualified_name: String,
    pub kind: SymbolKind,
    pub language: Language,
    pub file_id: FileId,
    pub parent_id: Option<SymbolId>,
    pub visibility: SymbolVisibility,
    pub range: SourceRange,
    /// 结构化签名（如 `(name: string): User`）；不适用的语言为 None。
    pub signature: Option<String>,
    /// 只记录“是否存在文档”，质量分析属于 M7+。
    pub documentation_present: bool,
}

impl Symbol {
    /// 构造 Symbol 并派生稳定 ID。
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        relative_path: &str,
        qualified_path: &str,
        kind: SymbolKind,
        language: Language,
        file_id: FileId,
        parent_id: Option<SymbolId>,
        visibility: SymbolVisibility,
        range: SourceRange,
    ) -> Self {
        let name = qualified_path
            .rsplit('.')
            .next()
            .unwrap_or(qualified_path)
            .to_string();
        Symbol {
            id: Self::derive_id(relative_path, qualified_path, kind),
            name,
            qualified_name: qualified_path.to_string(),
            kind,
            language,
            file_id,
            parent_id,
            visibility,
            range,
            signature: None,
            documentation_present: false,
        }
    }

    /// 稳定 ID：project 相对路径 + 限定路径 + kind（规格 §11）。
    pub fn derive_id(relative_path: &str, qualified_path: &str, kind: SymbolKind) -> SymbolId {
        SymbolId::from_string(format!(
            "symbol:{relative_path}#{qualified_path}#{}",
            kind_serialized(kind)
        ))
    }
}

fn kind_serialized(kind: SymbolKind) -> String {
    serde_json::to_value(kind)
        .ok()
        .and_then(|value| value.as_str().map(str::to_string))
        .unwrap_or_else(|| "unknown".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn loc(line: u32, column: u32, byte_offset: usize) -> SourceLocation {
        SourceLocation {
            line,
            column,
            byte_offset,
        }
    }

    #[test]
    fn coordinate_contract_is_documented() {
        assert!(COORDINATE_CONTRACT.contains("UTF-8 bytes"));
        assert!(COORDINATE_CONTRACT.contains("1-based"));
    }

    #[test]
    fn symbol_id_is_deterministic_and_kind_sensitive() {
        let a = Symbol::derive_id("src/user.ts", "UserService.create", SymbolKind::Method);
        let b = Symbol::derive_id("src/user.ts", "UserService.create", SymbolKind::Method);
        let c = Symbol::derive_id("src/user.ts", "UserService.create", SymbolKind::Function);
        let d = Symbol::derive_id("src/other.ts", "UserService.create", SymbolKind::Method);
        assert_eq!(a, b);
        assert_ne!(a, c, "kind must be part of the identity");
        assert_ne!(a, d, "file must be part of the identity");
    }

    #[test]
    fn symbol_name_is_last_segment_of_qualified_path() {
        let symbol = Symbol::new(
            "src/services/user.ts",
            "services/user.UserService.create",
            SymbolKind::Method,
            Language::TypeScript,
            FileId::from_string("file:src/services/user.ts"),
            None,
            SymbolVisibility::Unknown,
            SourceRange {
                start: loc(1, 1, 0),
                end: loc(2, 1, 20),
            },
        );
        assert_eq!(symbol.name, "create");
        assert_eq!(symbol.qualified_name, "services/user.UserService.create");
        assert_eq!(
            symbol.id.as_str(),
            "symbol:src/services/user.ts#services/user.UserService.create#method"
        );
    }

    #[test]
    fn symbol_serializes_with_snake_case_fields() {
        let symbol = Symbol::new(
            "a.py",
            "app.services.UserService",
            SymbolKind::Class,
            Language::Python,
            FileId::from_string("file:a.py"),
            None,
            SymbolVisibility::Public,
            SourceRange {
                start: SourceLocation::START,
                end: SourceLocation::START,
            },
        );
        let json = serde_json::to_value(&symbol).unwrap();
        assert!(json.get("qualified_name").is_some());
        assert!(json.get("parent_id").is_some());
        assert!(json.get("documentation_present").is_some());
        assert_eq!(json["kind"], "class");
    }

    #[test]
    fn symbol_kinds_cover_m2_required_set() {
        // 规格 §12 的 P0 集合必须存在（编译期即校验，这里防止误删）。
        let kinds = [
            SymbolKind::File,
            SymbolKind::Module,
            SymbolKind::Namespace,
            SymbolKind::Class,
            SymbolKind::Struct,
            SymbolKind::Interface,
            SymbolKind::Enum,
            SymbolKind::Function,
            SymbolKind::Method,
            SymbolKind::Constructor,
            SymbolKind::Property,
            SymbolKind::Field,
            SymbolKind::Import,
            SymbolKind::Export,
            SymbolKind::Unknown,
        ];
        assert_eq!(kinds.len(), 15);
    }

    #[test]
    fn visibility_serializes_snake_case() {
        assert_eq!(
            serde_json::to_string(&SymbolVisibility::Unknown).unwrap(),
            "\"unknown\""
        );
    }
}
