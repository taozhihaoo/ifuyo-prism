//! 派生事实：一次计算、多规则消费（规格 M7 §91–§94）。
//!
//! - [`DerivedFacts::file_contents`]：每文件只读一次（MAINT/CODE/SEC 共享）
//! - [`CodeMetrics`]：每个符号的行数 / 分支关键词数 / 最大括号嵌套
//!
//! 复杂度定义（§37 明确算法，不冒充标准 McCabe）：
//! `1 + 函数体内分支关键词与短路/三元操作符的出现次数`（词法近似，
//! 按语言关键词表统计；字符串字面量中的关键词会计入——文档化的已知近似）。
//! 嵌套深度仅对花括号语言定义；Python 缩进块不计算（NotApplicable，§115）。

use prism_analysis::ProjectCodeModel;
use prism_graph::DependencyGraphResult;
use prism_parser::SymbolKind;
use prism_project::Language;
use std::collections::BTreeMap;

/// 单符号代码度量。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolMetrics {
    /// 符号体源码行数（start..=end 行，含）。
    pub line_count: u64,
    /// 分支关键词/操作符计数（见模块文档；语言感知）。
    pub branch_count: u64,
    /// 最大花括号嵌套深度（相对函数体起始；非花括号语言为 None）。
    pub max_brace_nesting: Option<u64>,
}

/// 每符号度量，键 = M2 SymbolId 字符串。
pub type SymbolMetricsMap = BTreeMap<String, SymbolMetrics>;

/// 文件内容缓存（键 = 项目相对路径）。
pub type FileContents = BTreeMap<String, String>;

/// 一次分析内共享的派生事实（§92；不复制 ProjectModel）。
#[derive(Debug, Default, Clone)]
pub struct DerivedFacts {
    pub file_contents: FileContents,
    pub symbol_metrics: SymbolMetricsMap,
    /// 模块级 fan-in / fan-out（来自 M3 resolved 依赖，键 = ModuleId 字符串）。
    pub fan_in: BTreeMap<String, u64>,
    pub fan_out: BTreeMap<String, u64>,
}

impl DerivedFacts {
    /// 构建派生事实（一次遍历；§91 不做每规则全量重扫）。
    ///
    /// 文件集 = 代码模型管线文件（Source/Test，含解析失败——文件级规则仍可用）
    /// ∪ discovery 的 Documentation/Config 文件（SEC 扫描面，§120）。
    /// 超 1MB 的文件跳过（有界，§103）。
    pub fn build(
        code_model: Option<&ProjectCodeModel>,
        discovery: Option<&prism_project::ProjectDiscovery>,
        graph: Option<&DependencyGraphResult>,
        root: &std::path::Path,
    ) -> Self {
        let mut facts = DerivedFacts::default();
        let mut wanted: BTreeMap<String, Option<Language>> = BTreeMap::new();

        if let Some(model) = code_model {
            for result in &model.parse_results {
                if result.status == prism_parser::ParseStatus::Skipped {
                    continue; // Generated 策略跳过 ⇒ 不读内容（§118/§49）
                }
                wanted.insert(
                    result.relative_path.clone(),
                    result.language,
                );
            }
        }
        if let Some(discovery) = discovery {
            for file in &discovery.files {
                // 安全扫描面（§120/§49）：除 Generated 外的全部已发现文件
                // ——secret 模式精确（键名+分隔符+长值/PEM 标记），宁宽收集、
                // 报告时以 [REDACTED] 与 confidence 约束噪声；非 UTF-8 与
                // 超大文件在读取防线被跳过。CODE/MAINT 规则仍只消费代码模型。
                if file.classification.kind != prism_project::FileKind::Generated {
                    wanted.entry(file.relative_path.clone()).or_insert(file.language);
                }
            }
        }

        let mut languages: BTreeMap<String, Language> = BTreeMap::new();
        for (path, language) in wanted {
            let full = root.join(&path);
            let Ok(metadata) = std::fs::metadata(&full) else {
                continue;
            };
            if metadata.len() > 1_000_000 {
                continue; // 有界（§103）：不读超大文件
            }
            if let Ok(content) = std::fs::read_to_string(&full) {
                if let Some(language) = language {
                    languages.insert(path.clone(), language);
                }
                facts.file_contents.insert(path, content);
            }
        }

        if let Some(model) = code_model {
            facts.symbol_metrics = Self::symbol_metrics(model, &facts.file_contents, &languages);
        }
        if let Some(graph) = graph {
            for dependency in &graph.dependencies {
                if dependency.resolution != prism_graph::ResolutionStatus::Resolved {
                    continue;
                }
                *facts.fan_out.entry(dependency.source_id.as_str().to_string()).or_insert(0) += 1;
                if let Some(target) = &dependency.target_id {
                    *facts.fan_in.entry(target.as_str().to_string()).or_insert(0) += 1;
                }
            }
        }
        facts
    }

    /// 每符号度量（§93：一次计算，CODE 规则共享）。
    fn symbol_metrics(
        model: &ProjectCodeModel,
        contents: &FileContents,
        languages: &BTreeMap<String, Language>,
    ) -> SymbolMetricsMap {
        let mut metrics = SymbolMetricsMap::new();
        for result in &model.parse_results {
            let Some(content) = contents.get(&result.relative_path) else {
                continue;
            };
            let language = languages.get(&result.relative_path).copied();
            let lines: Vec<&str> = content.lines().collect();
            for symbol in &result.symbols {
                let callable_or_type = matches!(
                    symbol.kind,
                    SymbolKind::Function
                        | SymbolKind::Method
                        | SymbolKind::Constructor
                        | SymbolKind::Class
                        | SymbolKind::Struct
                        | SymbolKind::Interface
                        | SymbolKind::Enum
                );
                if !callable_or_type {
                    continue;
                }
                let start = symbol.range.start.line.saturating_sub(1) as usize;
                let end = (symbol.range.end.line as usize).min(lines.len());
                if start >= end {
                    continue;
                }
                let body = &lines[start..end];
                let branch_count = body
                    .iter()
                    .map(|line| count_branches(line, language))
                    .sum();
                let max_brace_nesting = max_brace_nesting(body);
                metrics.insert(
                    symbol.id.as_str().to_string(),
                    SymbolMetrics {
                        line_count: body.len() as u64,
                        branch_count,
                        max_brace_nesting,
                    },
                );
            }
        }
        metrics
    }
}

/// 分支关键词/操作符计数（整词匹配；语言感知，§115）。
fn count_branches(line: &str, language: Option<Language>) -> u64 {
    let keywords: &[&str] = match language {
        Some(Language::Python | Language::Shell) => {
            &["if", "elif", "for", "while", "except", "and", "or"]
        }
        Some(
            Language::TypeScript
            | Language::TypeScriptReact
            | Language::JavaScript
            | Language::JavaScriptReact
            | Language::CSharp
            | Language::Java
            | Language::C
            | Language::Cpp
            | Language::Rust
            | Language::Go,
        ) => &["if", "for", "while", "case", "catch", "&&", "||", "?"],
        _ => &[],
    };
    let mut count = 0u64;
    for keyword in keywords {
        if keyword.chars().all(|c| c.is_ascii_alphabetic()) {
            // 整词匹配（前后不是字母/数字/下划线）。
            let mut cursor = 0;
            while let Some(found) = line[cursor..].find(keyword) {
                let start = cursor + found;
                let end = start + keyword.len();
                let before_ok = line[..start]
                    .chars()
                    .next_back()
                    .is_none_or(|c| !(c.is_alphanumeric() || c == '_'));
                let after_ok = line[end..]
                    .chars()
                    .next()
                    .is_none_or(|c| !(c.is_alphanumeric() || c == '_'));
                if before_ok && after_ok {
                    count += 1;
                }
                cursor = end;
                if cursor >= line.len() {
                    break;
                }
            }
        } else {
            // 符号操作符直接计数。
            count += line.matches(keyword).count() as u64;
        }
    }
    count
}

/// 花括号嵌套深度（相对行组起始的净深度最大值；非花括号内容自然为 Some(0)）。
fn max_brace_nesting(body: &[&str]) -> Option<u64> {
    let mut depth: i64 = 0;
    let mut max: u64 = 0;
    for line in body {
        for character in line.chars() {
            match character {
                '{' => {
                    depth += 1;
                    max = max.max(depth as u64);
                }
                '}' => depth -= 1,
                _ => {}
            }
        }
    }
    Some(max)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn branch_counting_is_word_boundary_and_language_aware() {
        assert_eq!(count_branches("if (a && b) { return 1; }", Some(Language::TypeScript)), 2);
        assert_eq!(count_branches("verify(x)", Some(Language::TypeScript)), 0, "整词匹配");
        assert_eq!(count_branches("if x and y:", Some(Language::Python)), 2);
        assert_eq!(count_branches("match x:", Some(Language::TypeScript)), 0, "match 非 TS 关键词");
        assert_eq!(count_branches("anything", None), 0, "未知语言不计（NotApplicable）");
    }

    #[test]
    fn nesting_tracks_braces_only() {
        let body = vec![
            "function f() {",
            "  if (a) {",
            "    for (;;) {",
            "    }",
            "  }",
            "}",
        ];
        assert_eq!(max_brace_nesting(&body), Some(3));
    }
}
