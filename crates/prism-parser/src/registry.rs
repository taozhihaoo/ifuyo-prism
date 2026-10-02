//! LanguageParser trait 与 ParserRegistry。
//!
//! 语言分派集中在 Registry（规格 §7），业务代码不得散布
//! `match language { TypeScript => .. }`。

use crate::parse::{ParseRequest, ParseResult};
use prism_core::task::CancellationToken;
use prism_core::PrismResult;
use prism_project::Language;
use std::collections::BTreeMap;
use std::sync::Arc;

/// 语言 parser 抽象。实现必须线程安全（多文件并发解析共用）。
pub trait LanguageParser: Send + Sync {
    /// 本 parser 负责的语言。
    fn language(&self) -> Language;

    /// parser 实现版本（随 grammar/抽取逻辑变化递增）。
    fn parser_version(&self) -> &str {
        "1"
    }

    /// 解析单个文件。取消时返回 `Err(Cancelled)`。
    fn parse(
        &self,
        request: &ParseRequest,
        cancellation: &CancellationToken,
    ) -> PrismResult<ParseResult>;
}

/// 语言 → parser 注册表。注册/查询线程安全。
#[derive(Default)]
pub struct ParserRegistry {
    parsers: BTreeMap<Language, Arc<dyn LanguageParser>>,
}

impl ParserRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// M2 全量默认注册表：TypeScript / TSX / JavaScript / Python / C#。
    pub fn with_defaults() -> Self {
        let mut registry = ParserRegistry::new();
        registry.register(Arc::new(crate::languages::WebParser::typescript()));
        registry.register(Arc::new(crate::languages::WebParser::tsx()));
        registry.register(Arc::new(crate::languages::WebParser::javascript()));
        registry.register(Arc::new(crate::languages::PythonParser::new()));
        registry.register(Arc::new(crate::languages::CSharpParser::new()));
        registry
    }

    pub fn register(&mut self, parser: Arc<dyn LanguageParser>) {
        self.parsers.insert(parser.language(), parser);
    }

    pub fn get(&self, language: Language) -> Option<Arc<dyn LanguageParser>> {
        self.parsers.get(&language).cloned()
    }

    pub fn is_supported(&self, language: Language) -> bool {
        self.parsers.contains_key(&language)
    }

    /// 受支持语言（按枚举定义排序，确定性）。
    pub fn supported_languages(&self) -> Vec<Language> {
        self.parsers.keys().copied().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Symbol;
    use prism_core::ids::FileId;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct FakeParser {
        calls: AtomicUsize,
    }

    impl LanguageParser for FakeParser {
        fn language(&self) -> Language {
            Language::Python
        }

        fn parse(
            &self,
            request: &ParseRequest,
            cancellation: &CancellationToken,
        ) -> PrismResult<ParseResult> {
            cancellation.check()?;
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(ParseResult {
                file_id: request.file_id.clone(),
                relative_path: request.relative_path.clone(),
                language: Some(self.language()),
                status: crate::parse::ParseStatus::Successful,
                symbols: Vec::<Symbol>::new(),
                diagnostics: vec![],
                duration_ms: 0,
                parser_version: self.parser_version().to_string(),
                reason: None,
                imports: Vec::new(),
            })
        }
    }

    #[test]
    fn registry_lookup_and_unsupported_detection() {
        let mut registry = ParserRegistry::new();
        assert!(!registry.is_supported(Language::Python));
        assert!(registry.supported_languages().is_empty());

        registry.register(Arc::new(FakeParser {
            calls: AtomicUsize::new(0),
        }));
        assert!(registry.is_supported(Language::Python));
        assert!(!registry.is_supported(Language::TypeScript));
        assert_eq!(registry.supported_languages(), vec![Language::Python]);
        assert!(registry.get(Language::Python).is_some());
        assert!(registry.get(Language::TypeScript).is_none());
    }

    #[test]
    fn parser_via_registry_honours_cancellation() {
        let mut registry = ParserRegistry::new();
        registry.register(Arc::new(FakeParser {
            calls: AtomicUsize::new(0),
        }));
        let parser = registry.get(Language::Python).unwrap();
        let token = CancellationToken::new();
        token.cancel();

        let request = ParseRequest {
            file_id: FileId::from_string("file:a.py"),
            relative_path: "a.py".to_string(),
            language: Language::Python,
            content: "x = 1".to_string(),
        };
        let err = parser.parse(&request, &token).unwrap_err();
        assert_eq!(err.code(), prism_core::PrismErrorCode::Cancelled);
    }

    #[test]
    fn re_registration_replaces_parser_for_same_language() {
        let mut registry = ParserRegistry::new();
        registry.register(Arc::new(FakeParser {
            calls: AtomicUsize::new(0),
        }));
        registry.register(Arc::new(FakeParser {
            calls: AtomicUsize::new(0),
        }));
        assert_eq!(registry.supported_languages().len(), 1);
    }
}
