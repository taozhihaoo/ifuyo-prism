//! 语言检测：扩展名 → 语言。
//!
//! M1 只做扩展名映射，不解析内容。Registry 可扩展；
//! 语言不与 FileKind 混淆（package.json 是 JSON 语言、Config 类型）。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Prism 支持识别的语言（M1 覆盖总纲领的 18 种要求）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Language {
    TypeScript,
    #[serde(rename = "typescript_react")]
    TypeScriptReact,
    JavaScript,
    #[serde(rename = "javascript_react")]
    JavaScriptReact,
    Python,
    #[serde(rename = "c_sharp")]
    CSharp,
    Rust,
    Java,
    C,
    Cpp,
    Go,
    Html,
    Css,
    Scss,
    Json,
    Yaml,
    Toml,
    Markdown,
    Sql,
    Shell,
}

impl Language {
    /// 对外展示名（统计/报表使用）。
    pub fn display_name(self) -> &'static str {
        match self {
            Language::TypeScript => "TypeScript",
            Language::TypeScriptReact => "TypeScript React",
            Language::JavaScript => "JavaScript",
            Language::JavaScriptReact => "JavaScript React",
            Language::Python => "Python",
            Language::CSharp => "C#",
            Language::Rust => "Rust",
            Language::Java => "Java",
            Language::C => "C",
            Language::Cpp => "C++",
            Language::Go => "Go",
            Language::Html => "HTML",
            Language::Css => "CSS",
            Language::Scss => "SCSS",
            Language::Json => "JSON",
            Language::Yaml => "YAML",
            Language::Toml => "TOML",
            Language::Markdown => "Markdown",
            Language::Sql => "SQL",
            Language::Shell => "Shell",
        }
    }

    fn default_extensions(self) -> &'static [&'static str] {
        match self {
            Language::TypeScript => &["ts", "mts", "cts"],
            Language::TypeScriptReact => &["tsx"],
            Language::JavaScript => &["js", "mjs", "cjs"],
            Language::JavaScriptReact => &["jsx"],
            Language::Python => &["py", "pyi"],
            Language::CSharp => &["cs"],
            Language::Rust => &["rs"],
            Language::Java => &["java"],
            Language::C => &["c", "h"],
            Language::Cpp => &["cpp", "cc", "cxx", "hpp", "hh"],
            Language::Go => &["go"],
            Language::Html => &["html", "htm"],
            Language::Css => &["css"],
            Language::Scss => &["scss", "sass"],
            Language::Json => &["json", "json5", "jsonc"],
            Language::Yaml => &["yaml", "yml"],
            Language::Toml => &["toml"],
            Language::Markdown => &["md", "markdown"],
            Language::Sql => &["sql"],
            Language::Shell => &["sh", "bash", "zsh"],
        }
    }
}

/// 扩展名 → 语言注册表（小写扩展名，不含点）。
#[derive(Debug, Clone, Default)]
pub struct LanguageRegistry {
    extensions: HashMap<String, Language>,
}

impl LanguageRegistry {
    /// 内置 M1 全部支持语言的注册表。
    pub fn with_defaults() -> Self {
        let mut registry = LanguageRegistry::default();
        for language in [
            Language::TypeScript,
            Language::TypeScriptReact,
            Language::JavaScript,
            Language::JavaScriptReact,
            Language::Python,
            Language::CSharp,
            Language::Rust,
            Language::Java,
            Language::C,
            Language::Cpp,
            Language::Go,
            Language::Html,
            Language::Css,
            Language::Scss,
            Language::Json,
            Language::Yaml,
            Language::Toml,
            Language::Markdown,
            Language::Sql,
            Language::Shell,
        ] {
            for extension in language.default_extensions() {
                registry.register(extension, language);
            }
        }
        registry
    }

    /// 注册/覆盖一个扩展名（扩展名统一小写存储）。
    pub fn register(&mut self, extension: &str, language: Language) {
        self.extensions
            .insert(extension.to_ascii_lowercase(), language);
    }

    /// 从文件名检测语言（大小写不敏感）。无扩展名或未知扩展名返回 None。
    pub fn detect(&self, file_name: &str) -> Option<Language> {
        let extension = file_name.rsplit_once('.')?.1;
        if extension.is_empty() {
            return None;
        }
        self.extensions
            .get(&extension.to_ascii_lowercase())
            .copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_m1_required_languages() {
        let registry = LanguageRegistry::with_defaults();
        let cases = [
            ("main.ts", Language::TypeScript),
            ("app.tsx", Language::TypeScriptReact),
            ("index.js", Language::JavaScript),
            ("widget.jsx", Language::JavaScriptReact),
            ("server.py", Language::Python),
            ("Lib.cs", Language::CSharp),
            ("main.rs", Language::Rust),
            ("Main.java", Language::Java),
            ("a.c", Language::C),
            ("b.cpp", Language::Cpp),
            ("main.go", Language::Go),
            ("page.html", Language::Html),
            ("style.css", Language::Css),
            ("theme.scss", Language::Scss),
            ("data.json", Language::Json),
            ("config.yaml", Language::Yaml),
            ("Cargo.toml", Language::Toml),
            ("README.md", Language::Markdown),
            ("query.sql", Language::Sql),
            ("run.sh", Language::Shell),
        ];
        for (file, expected) in cases {
            assert_eq!(registry.detect(file), Some(expected), "file: {file}");
        }
    }

    #[test]
    fn unknown_and_extensionless_files_return_none() {
        let registry = LanguageRegistry::with_defaults();
        assert_eq!(registry.detect("Dockerfile"), None);
        assert_eq!(registry.detect("Makefile"), None);
        assert_eq!(registry.detect("LICENSE"), None);
        assert_eq!(registry.detect("archive.unknownext"), None);
        assert_eq!(registry.detect("dotfile."), None);
    }

    #[test]
    fn detection_is_case_insensitive() {
        let registry = LanguageRegistry::with_defaults();
        assert_eq!(registry.detect("MAIN.TS"), Some(Language::TypeScript));
        assert_eq!(registry.detect("README.MD"), Some(Language::Markdown));
    }

    #[test]
    fn registry_is_extensible() {
        let mut registry = LanguageRegistry::with_defaults();
        registry.register("swift", Language::JavaScript); // 示例：自定义映射
        assert_eq!(registry.detect("app.swift"), Some(Language::JavaScript));
    }

    #[test]
    fn serializes_snake_case() {
        assert_eq!(
            serde_json::to_string(&Language::TypeScriptReact).unwrap(),
            "\"typescript_react\""
        );
        assert_eq!(
            serde_json::to_string(&Language::CSharp).unwrap(),
            "\"c_sharp\""
        );
    }
}
