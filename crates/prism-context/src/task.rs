//! 确定性任务分类与目标解析（规格 M10a §5/§11–§18/§148–§149）。
//!
//! 无 LLM（§5/§253）：关键词表 + 形态规则；"不要假装理解所有自然语言"
//! （§17）——匹配不到 ⇒ Unknown/Generic 并如实声明（§18）。

use crate::model::{
    ContextFocus, ContextTarget, TargetResolution, TaskClassification, TaskKind,
    TaskSpecificity,
};
use prism_analysis::ProjectCodeModel;

/// 任务关键词表（§12/§13；中英并列，保守匹配——宁缺勿滥，匹配不到即 Unknown）。
const KEYWORDS: &[(TaskKind, &[&str])] = &[
    (
        TaskKind::BugInvestigation,
        &["bug", "fix", "error", "崩溃", "修复", "错误", "调查", "investigate", "500", "失败", "fail"],
    ),
    (TaskKind::Test, &["test", "测试", "failing"]),
    (TaskKind::Ci, &["ci", "github actions", "workflow", "pipeline", "流水线", "构建流程"]),
    (TaskKind::Refactor, &["refactor", "重构"]),
    (TaskKind::Feature, &["feature", "新增", "添加", "实现", "implement"]),
    (TaskKind::Change, &["diff", "变更", "改动"]),
    (TaskKind::Review, &["review", "审查", "评审"]),
    (TaskKind::Security, &["security", "安全", "漏洞", "credential"]),
    (TaskKind::Documentation, &["docs", "documentation", "文档", "readme"]),
    (TaskKind::Release, &["release", "发布"]),
    (TaskKind::Environment, &["environment", "环境"]),
    (TaskKind::Build, &["build", "编译", "构建失败"]),
    (TaskKind::Architecture, &["architecture", "架构", "模块结构"]),
];

/// kind → 关注面（§14；多对多并集，排序去重）。
fn focus_for(kind: TaskKind) -> &'static [ContextFocus] {
    match kind {
        TaskKind::BugInvestigation => &[
            ContextFocus::Code,
            ContextFocus::Health,
            ContextFocus::Verification,
            ContextFocus::Changes,
        ],
        TaskKind::Test => &[ContextFocus::Tests, ContextFocus::Code],
        TaskKind::Ci => &[ContextFocus::Ci, ContextFocus::Verification],
        TaskKind::Refactor => &[ContextFocus::Code, ContextFocus::Architecture, ContextFocus::Impact],
        TaskKind::Feature => &[ContextFocus::Code, ContextFocus::Tests],
        TaskKind::Change => &[ContextFocus::Changes, ContextFocus::Git],
        TaskKind::Review => &[ContextFocus::Changes, ContextFocus::Code, ContextFocus::Health],
        TaskKind::Security => &[ContextFocus::Code, ContextFocus::Health],
        TaskKind::Documentation => &[ContextFocus::Documentation],
        TaskKind::Release => &[ContextFocus::Changes, ContextFocus::Verification],
        TaskKind::Environment => &[ContextFocus::Environment],
        TaskKind::Build => &[ContextFocus::Verification, ContextFocus::Configuration],
        TaskKind::Architecture => &[ContextFocus::Architecture, ContextFocus::Impact],
        TaskKind::Unknown => &[
            ContextFocus::Code,
            ContextFocus::Changes,
            ContextFocus::Architecture,
            ContextFocus::Health,
            ContextFocus::Verification,
        ],
    }
}

/// 确定性分类（§12/§13：多 kind 并存；全部命中按枚举名稳定排序）。
pub fn classify(task: Option<&str>) -> TaskClassification {
    let Some(text) = task.map(str::trim).filter(|text| !text.is_empty()) else {
        // §18：无任务/空任务 ⇒ Unknown + Generic overview（focus 由 Unknown 分支给）。
        return TaskClassification {
            kinds: vec![TaskKind::Unknown],
            focus: focus_for(TaskKind::Unknown).to_vec(),
            specificity: TaskSpecificity::Generic,
            raw_task: task.map(str::to_string),
        };
    };
    let lower = text.to_lowercase();
    let mut kinds: Vec<TaskKind> = Vec::new();
    for (kind, keywords) in KEYWORDS {
        if keywords.iter().any(|keyword| lower.contains(keyword))
            && !kinds.contains(kind)
        {
            kinds.push(*kind);
        }
    }
    if kinds.is_empty() {
        return TaskClassification {
            kinds: vec![TaskKind::Unknown],
            focus: focus_for(TaskKind::Unknown).to_vec(),
            specificity: TaskSpecificity::Generic,
            raw_task: Some(text.to_string()),
        };
    }
    kinds.sort_by(|a, b| a.as_str().cmp(b.as_str()));
    let mut focus: Vec<ContextFocus> = kinds
        .iter()
        .flat_map(|kind| focus_for(*kind).iter().copied())
        .collect();
    focus.sort_by(|a, b| format!("{a:?}").cmp(&format!("{b:?}")));
    focus.dedup();
    TaskClassification {
        kinds,
        focus,
        specificity: TaskSpecificity::Specific,
        raw_task: Some(text.to_string()),
    }
}

/// 从任务文本提取显式目标（§15/§17）：优先 file path 形态，其次
/// symbol 形态（snake_case / CamelCase 标识符）。保守——宁缺勿滥。
pub fn extract_target(task: Option<&str>) -> Option<String> {
    let text = task?;
    for token in text.split(|c: char| c.is_whitespace() || matches!(c, ',' | '，' | '"' | '“' | '”' | '「' | '」')) {
        let token = token
            .trim_matches(|c: char| matches!(c, '.' | ':' | '(' | ')' | '（' | '）'));
        if token.is_empty() {
            continue;
        }
        // 显式路径：含分隔符，或带扩展名。
        if token.contains('/') || token.contains('\\') {
            return Some(token.to_string());
        }
        let mut parts = token.rsplit('.');
        let (maybe_ext, stem) = (parts.next(), parts.next());
        if let (Some(ext), Some(stem)) = (maybe_ext, stem) {
            if (1..=4).contains(&ext.len()) && ext.chars().all(|c| c.is_ascii_alphanumeric())
                && !stem.is_empty()
                && stem.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
            {
                return Some(token.to_string());
            }
        }
        // Symbol：snake_case / CamelCase / 限定名（BattleManager.resolve）；
        // 文件形态已在上面分支处理，这里的 '.' 属限定名分隔。
        if token.len() >= 4
            && token
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
            && (token.contains('_')
                || token.chars().any(|c| c.is_uppercase())
                || token.chars().any(|c| c.is_ascii_digit()))
        {
            return Some(token.to_string());
        }
    }
    None
}

/// 目标解析（§16：唯一 ⇒ Resolved；多候选 ⇒ Ambiguous 全列出；无 ⇒ NotFound）。
/// 文件按 discovery 的项目相对路径后缀匹配；符号按精确名匹配 M2 code model。
pub fn resolve_target(
    target: Option<&str>,
    discovery: Option<&prism_project::ProjectDiscovery>,
    code_model: Option<&ProjectCodeModel>,
) -> ContextTarget {
    let Some(raw) = target.map(str::to_string) else {
        return ContextTarget {
            raw: String::new(),
            resolution: TargetResolution::None,
        };
    };
    let looks_like_path = raw.contains('/') || raw.contains('\\') || raw.contains('.');

    // ---- 文件匹配（M1 paths，§152 复用）----
    let path_hits: Vec<String> = discovery
        .map(|discovery| {
            discovery
                .files
                .iter()
                .map(|file| file.relative_path.clone())
                .filter(|path| path_ends_with(path, &raw))
                .collect()
        })
        .unwrap_or_default();
    let _ = looks_like_path;

    // ---- 符号匹配（M2 精确名，§149）----
    let mut symbol_hits: Vec<(String, String)> = Vec::new(); // (SymbolId, file)
    if let Some(model) = code_model {
        for result in &model.parse_results {
            for symbol in &result.symbols {
                if symbol.name == raw || symbol.qualified_name.ends_with(&raw) {
                    symbol_hits.push((
                        symbol.id.as_str().to_string(),
                        result.relative_path.clone(),
                    ));
                }
            }
        }
    }

    let mut paths: Vec<String> = path_hits.clone();
    for (symbol_id, file) in &symbol_hits {
        if !paths.contains(file) {
            paths.push(file.clone());
        }
    }
    paths.sort();
    paths.dedup();

    let symbol_ids: Vec<String> = symbol_hits.iter().map(|(id, _)| id.clone()).collect();
    // 唯一文件 ⇒ Resolved（同文件内多命中只是 file-module + 符号双层记录）。
    if paths.len() == 1 {
        return ContextTarget {
            raw,
            resolution: TargetResolution::Resolved {
                paths,
                symbols: symbol_ids,
            },
        };
    }
    if paths.is_empty() && symbol_ids.is_empty() {
        return ContextTarget {
            raw,
            resolution: TargetResolution::NotFound,
        };
    }
    // 多文件/多符号 ⇒ Ambiguous（§16：不要猜）。
    let mut candidates = paths;
    for (symbol_id, file) in symbol_hits {
        candidates.push(format!("{file}#{symbol_id}"));
    }
    candidates.sort();
    candidates.dedup();
    candidates.truncate(8);
    ContextTarget {
        raw,
        resolution: TargetResolution::Ambiguous { candidates },
    }
}

/// 路径后缀匹配（"BattleManager.cs" 命中 "src/Battle/BattleManager.cs"；
/// 分量边界：".env" 不命中 "xenv"）。
fn path_ends_with(path: &str, suffix: &str) -> bool {
    let suffix = suffix.replace('\\', "/");
    let path_normalized = path.replace('\\', "/");
    path_normalized == suffix
        || path_normalized.ends_with(&format!("/{suffix}"))
        || (suffix.contains('/') && path_normalized.ends_with(&suffix))
}

#[cfg(test)]
mod tests {
    use super::*;

    // §12/§13：多 kind 并存（"为什么 CI 在 Windows 上失败"）。
    #[test]
    fn classification_allows_multiple_kinds() {
        let classification = classify(Some("为什么 CI 在 Windows 上失败？"));
        assert!(classification.kinds.contains(&TaskKind::Ci));
        assert!(classification.kinds.contains(&TaskKind::BugInvestigation));
        assert_eq!(classification.specificity, TaskSpecificity::Specific);
    }

    // §18：Unknown 任务 ⇒ Generic + 明确声明。
    #[test]
    fn unknown_task_is_generic() {
        let classification = classify(Some("帮我把这个做得更好"));
        assert_eq!(classification.kinds, vec![TaskKind::Unknown]);
        assert_eq!(classification.specificity, TaskSpecificity::Generic);
        assert!(classification.raw_task.as_deref() == Some("帮我把这个做得更好"));
    }

    #[test]
    fn empty_task_is_generic_overview() {
        let classification = classify(None);
        assert_eq!(classification.specificity, TaskSpecificity::Generic);
    }

    // §15/§17：显式路径优先于 symbol。
    #[test]
    fn target_extraction_prefers_paths() {
        assert_eq!(
            extract_target(Some("重构 src/Battle/BattleManager.cs")),
            Some("src/Battle/BattleManager.cs".to_string())
        );
        assert_eq!(
            extract_target(Some("修复 BattleManager.resolve 的 bug")),
            Some("BattleManager.resolve".to_string())
        );
        assert_eq!(extract_target(Some("做得更好")), None);
    }

    // §16：唯一 ⇒ Resolved；多候选 ⇒ Ambiguous；无 ⇒ NotFound。
    #[test]
    fn target_resolution_does_not_guess() {
        let dir = std::env::temp_dir().join(format!(
            "prism-ctx-target-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.subsec_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("src/Battle")).unwrap();
        std::fs::create_dir_all(dir.join("src/Legacy")).unwrap();
        std::fs::write(dir.join("src/Battle/BattleManager.ts"), "class BattleManager {}\n").unwrap();
        std::fs::write(dir.join("src/Legacy/BattleManager.ts"), "class BattleManager {}\n").unwrap();
        std::fs::write(dir.join("src/UniqueResolver.ts"), "class UniqueResolver {}\n").unwrap();

        let cancel = CancellationToken::new();
        let discovery = prism_project::DiscoveryService::new()
            .discover(&dir, prism_project::ScanOptions::default(), &cancel, |_| {})
            .unwrap();
        let model = prism_analysis::CodeAnalysisService::new()
            .analyze(&discovery, prism_analysis::CodeAnalysisOptions::default(), &cancel, |_| {})
            .unwrap();

        // 完整唯一路径 ⇒ Resolved。
        let resolved = resolve_target(Some("src/Battle/BattleManager.ts"), Some(&discovery), Some(&model));
        match resolved.resolution {
            TargetResolution::Resolved { paths, .. } => {
                assert_eq!(paths, vec!["src/Battle/BattleManager.ts".to_string()]);
            }
            other => panic!("expected resolved, got {other:?}"),
        }

        // 同名符号两处 ⇒ Ambiguous（不猜，§16）。
        let ambiguous = resolve_target(Some("BattleManager"), Some(&discovery), Some(&model));
        match ambiguous.resolution {
            TargetResolution::Ambiguous { candidates } => {
                assert!(candidates.len() >= 2, "both locations should be candidates: {candidates:?}");
            }
            other => panic!("expected ambiguous, got {other:?}"),
        }

        // 唯一符号名 ⇒ Resolved（symbol + path）。
        let unique = resolve_target(Some("UniqueResolver"), Some(&discovery), Some(&model));
        match unique.resolution {
            TargetResolution::Resolved { paths, symbols } => {
                assert_eq!(paths, vec!["src/UniqueResolver.ts".to_string()]);
                assert!(
                    symbols.len() >= 1,
                    "file-module + class symbols both count: {symbols:?}"
                );
            }
            other => panic!("expected resolved, got {other:?}"),
        }

        // 完全未知 ⇒ NotFound。
        let missing = resolve_target(Some("DoesNotExist.ts"), Some(&discovery), Some(&model));
        assert_eq!(missing.resolution, TargetResolution::NotFound);

        let _ = std::fs::remove_dir_all(&dir);
    }

    use prism_core::task::CancellationToken;
}
