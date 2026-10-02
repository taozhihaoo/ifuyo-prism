//! Parser contract tests（规格 §52）：每个语言 adapter 必须通过同一组契约。
//! 新语言加入时在 `LANGUAGES` 中加一行，而不是重写测试体系。

use prism_core::task::CancellationToken;
use prism_parser::{
    CSharpParser, LanguageParser, ParseRequest, ParseStatus, ParserRegistry, PythonParser,
    WebParser,
};
use prism_project::Language;
use std::sync::Arc;

use prism_parser::SymbolKind;

struct Fixture {
    name: &'static str,
    parser: Arc<dyn LanguageParser>,
    /// 含 import + class + method + function 的有效源码。
    valid: &'static str,
    /// 语法损坏但应可恢复的源码。
    invalid: &'static str,
    /// 定位符号名（用于 location mapping 断言）。
    locate_name: &'static str,
    expect_line: u32,
    relative_path: &'static str,
    expect_kinds: &'static [SymbolKind],
}

fn request(fixture: &Fixture, content: &str) -> ParseRequest {
    ParseRequest {
        file_id: prism_core::ids::FileId::from_string(format!("file:{}", fixture.relative_path)),
        relative_path: fixture.relative_path.to_string(),
        language: fixture.parser.language(),
        content: content.to_string(),
    }
}

fn run_contract_suite(fixture: &Fixture) {
    let parser = &fixture.parser;
    let tag = fixture.name;

    // 1. supported_language（由编译期类型保证，这里验证 trait 通路）
    let _ = parser.language();

    // 2/6. parse_valid_file + symbol_extraction
    let result = parser
        .parse(&request(fixture, fixture.valid), &CancellationToken::new())
        .unwrap();
    assert!(
        matches!(
            result.status,
            ParseStatus::Successful | ParseStatus::SuccessfulWithDiagnostics
        ),
        "[{tag}] valid file must parse, got {:?} diagnostics: {:?}",
        result.status,
        result.diagnostics
    );
    let kinds: Vec<_> = result.symbols.iter().map(|s| s.kind).collect();
    for expected in fixture.expect_kinds {
        assert!(
            kinds.contains(expected),
            "[{tag}] missing {expected:?} in {kinds:?}"
        );
    }

    // 3. location_mapping（坐标契约：UTF-8 字节列）
    let located = result
        .symbols
        .iter()
        .find(|s| s.name == fixture.locate_name)
        .unwrap_or_else(|| panic!("[{tag}] symbol {} must exist", fixture.locate_name));
    assert_eq!(
        located.range.start.line, fixture.expect_line,
        "[{tag}] location mapping broken for {}",
        fixture.locate_name
    );
    assert!(located.range.end.line >= located.range.start.line);
    assert!(located.range.start.byte_offset < located.range.end.byte_offset);

    // 3.5 imports 语法事实必须被捕获（M3 关系解析输入）。
    assert!(
        !result.imports.is_empty(),
        "[{tag}] valid fixture imports an external/local module"
    );
    assert!(result.imports.iter().all(|i| !i.specifier.is_empty()));

    // 3.7 直接调用语法事实（M3 调用图输入）：fixture 内 render/create 调用
    // helper / 方法体里的调用必须被捕获且 caller 为宿主限定路径。
    assert!(
        !result.calls.is_empty(),
        "[{tag}] fixture contains calls inside methods"
    );
    assert!(result
        .calls
        .iter()
        .all(|c| !c.caller_qualified.is_empty() && !c.callee_text.is_empty()));

    // 4. hierarchy：方法挂在类下
    let class = result
        .symbols
        .iter()
        .find(|s| s.kind == SymbolKind::Class)
        .unwrap_or_else(|| panic!("[{tag}] fixture must contain a class"));
    let method = result
        .symbols
        .iter()
        .find(|s| s.kind == SymbolKind::Method)
        .unwrap_or_else(|| panic!("[{tag}] fixture must contain a method"));
    assert_eq!(
        method.parent_id.as_ref(),
        Some(&class.id),
        "[{tag}] method parent must be the class"
    );

    // 5. parse_invalid_file + diagnostics：单文件失败不致命，且尽量保留符号
    let broken = parser
        .parse(
            &request(fixture, fixture.invalid),
            &CancellationToken::new(),
        )
        .unwrap();
    assert!(
        matches!(
            broken.status,
            ParseStatus::SuccessfulWithDiagnostics | ParseStatus::Failed
        ),
        "[{tag}] broken file must carry diagnostics, got {:?}",
        broken.status
    );
    assert!(broken.has_errors() || !broken.diagnostics.is_empty());
    assert!(
        broken.symbols.iter().any(|s| s.kind == SymbolKind::Class),
        "[{tag}] recovery should retain the class symbol"
    );

    // 7. determinism：同输入两次解析，符号与诊断完全一致
    let again = parser
        .parse(&request(fixture, fixture.valid), &CancellationToken::new())
        .unwrap();
    assert_eq!(
        result.symbols, again.symbols,
        "[{tag}] symbols must be deterministic"
    );
    assert_eq!(
        result.diagnostics, again.diagnostics,
        "[{tag}] diagnostics must be deterministic"
    );

    // 8. cancellation：预取消的 token 必须立即生效
    let token = CancellationToken::new();
    token.cancel();
    let err = parser
        .parse(&request(fixture, fixture.valid), &token)
        .unwrap_err();
    assert_eq!(err.code(), prism_core::PrismErrorCode::Cancelled);
}

const TS_VALID: &str = r#"import { helper } from "./helper";

export class Widget {
    private ready = false;

    render(): string {
        return helper(this.name);
    }
}

function createWidget(name: string): Widget {
    return new Widget();
}
"#;

const TS_INVALID: &str = r#"export class UserService {
    create() {}

    validate(   // syntax error
"#;

const TSX_VALID: &str = r#"import React from "react";

export class Panel {
    render(): string {
        return mount();
    }
}

function mount(): string {
    return "panel";
}

function mountPanel(): void {}
"#;

const TSX_INVALID: &str = r#"export class Broken {
    mount(   // syntax error
"#;

const JS_VALID: &str = r#"import { helper } from "./util";

export class Store {
    constructor() {
        this.ready = true;
    }

    get(key) {
        return helper(key);
    }
}

function createStore(name) {
    return new Store();
}

const version = "1.0";
"#;

const JS_INVALID: &str = r#"export class Cache {
    put(   // syntax error
"#;

const PY_VALID: &str = r#"import os
from collections import OrderedDict

class UserService:
    """用户服务：中文 docstring 与注释。"""

    def create(self, 名称):
        # 中文注释
        return 名称

def create_service(name):
    _internal = 1
    return UserService()
"#;

const PY_INVALID: &str = r#"class UserService:
    def create(self):
        return 1
    def broken(self,:
        pass
"#;

const CS_VALID: &str = r#"using System;

namespace App.Services
{
    public class UserService
    {
        private bool ready;

        public User Create(string name)
        {
            var text = Convert.ToString(name);
            return new User(text);
        }
    }
}
"#;

const CS_INVALID: &str = r#"public class BrokenService
{
    public void Run(   // syntax error
"#;

static WEB_KINDS: &[SymbolKind] = &[
    SymbolKind::File,
    SymbolKind::Class,
    SymbolKind::Method,
    SymbolKind::Function,
    SymbolKind::Import,
];
static PY_KINDS: &[SymbolKind] = &[
    SymbolKind::File,
    SymbolKind::Class,
    SymbolKind::Method,
    SymbolKind::Function,
    SymbolKind::Import,
    SymbolKind::Variable,
];
static CS_KINDS: &[SymbolKind] = &[
    SymbolKind::File,
    SymbolKind::Namespace,
    SymbolKind::Class,
    SymbolKind::Method,
    SymbolKind::Field,
    SymbolKind::Import,
];

#[test]
fn typescript_contract() {
    run_contract_suite(&Fixture {
        name: "typescript",
        parser: Arc::new(WebParser::typescript()),
        valid: TS_VALID,
        invalid: TS_INVALID,
        locate_name: "createWidget",
        expect_line: 11,
        relative_path: "src/widget.ts",
        expect_kinds: WEB_KINDS,
    });
}

#[test]
fn tsx_contract() {
    run_contract_suite(&Fixture {
        name: "tsx",
        parser: Arc::new(WebParser::tsx()),
        valid: TSX_VALID,
        invalid: TSX_INVALID,
        locate_name: "mountPanel",
        expect_line: 13,
        relative_path: "src/panel.tsx",
        expect_kinds: WEB_KINDS,
    });
}

#[test]
fn javascript_contract() {
    run_contract_suite(&Fixture {
        name: "javascript",
        parser: Arc::new(WebParser::javascript()),
        valid: JS_VALID,
        invalid: JS_INVALID,
        locate_name: "createStore",
        expect_line: 13,
        relative_path: "src/store.js",
        expect_kinds: WEB_KINDS,
    });
}

#[test]
fn python_contract() {
    run_contract_suite(&Fixture {
        name: "python",
        parser: Arc::new(PythonParser::new()),
        valid: PY_VALID,
        invalid: PY_INVALID,
        locate_name: "create_service",
        expect_line: 11,
        relative_path: "src/services/user_service.py",
        expect_kinds: PY_KINDS,
    });
}

#[test]
fn csharp_contract() {
    run_contract_suite(&Fixture {
        name: "csharp",
        parser: Arc::new(CSharpParser::new()),
        valid: CS_VALID,
        invalid: CS_INVALID,
        locate_name: "Create",
        expect_line: 9,
        relative_path: "src/Services/UserService.cs",
        expect_kinds: CS_KINDS,
    });
}

#[test]
fn defaults_registry_supports_all_m2_languages() {
    let registry = ParserRegistry::with_defaults();
    for language in [
        Language::TypeScript,
        Language::TypeScriptReact,
        Language::JavaScript,
        Language::Python,
        Language::CSharp,
    ] {
        assert!(
            registry.is_supported(language),
            "{language:?} must be supported"
        );
    }
    assert!(
        !registry.is_supported(Language::Rust),
        "rust parser is not in M2 scope"
    );
    assert_eq!(registry.supported_languages().len(), 5);
}

#[test]
fn python_unicode_locations_are_byte_based_and_consistent() {
    // 规格 §15：中文标识符不得造成 byte/char 错位；坐标契约按 UTF-8 字节。
    let source = "def 计算价格(数量):\n    return 数量 * 2\n";
    let parser = PythonParser::new();
    let request = ParseRequest {
        file_id: prism_core::ids::FileId::from_string("file:src/价格.py"),
        relative_path: "src/价格.py".to_string(),
        language: Language::Python,
        content: source.to_string(),
    };
    let result = parser.parse(&request, &CancellationToken::new()).unwrap();
    let function = result
        .symbols
        .iter()
        .find(|s| s.kind == SymbolKind::Function)
        .expect("unicode function must be extracted");
    assert_eq!(function.name, "计算价格");
    assert_eq!(function.range.start.line, 1);
    // Symbol range 覆盖整条 `def ...`：起始 = 行首（列 1，字节 0）。
    assert_eq!(function.range.start.column, 1);
    assert_eq!(function.range.start.byte_offset, 0);

    let again = parser.parse(&request, &CancellationToken::new()).unwrap();
    assert_eq!(result.symbols, again.symbols);
}
