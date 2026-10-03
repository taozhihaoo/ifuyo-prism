//! PATH 采集与规范化（规格 M9a §19–§21/§108–§111/§163/§192/§314）。
//!
//! 只记录事实（duplicate/missing/shadowing 可能性），不轻率断言因果（§21）。

use crate::model::{PathEntry, PathFacts, PATH_ENTRY_CAP};
use prism_core::task::CancellationToken;
use std::path::Path;

/// 进程 PATH → 规范化事实（§163：v1 baseline 是 Process scope，§167）。
pub fn collect_path_facts(cancel: &CancellationToken) -> PathFacts {
    let raw = std::env::var("PATH").unwrap_or_default();
    path_facts_from_raw(&raw, cancel)
}

/// 从原始 PATH 字符串构建事实（测试可注入，不碰进程环境）。
pub fn path_facts_from_raw(raw: &str, cancel: &CancellationToken) -> PathFacts {
    let separator = if cfg!(windows) { ';' } else { ':' };
    let mut entries: Vec<PathEntry> = Vec::new();
    let mut duplicate_identities: Vec<String> = Vec::new();
    let mut seen: std::collections::BTreeSet<String> = Default::default();
    let mut truncated = false;
    for segment in raw.split(separator) {
        if cancel.is_cancelled() {
            break;
        }
        // 去引号/空白；空段跳过（§108 normalization）。
        let trimmed = segment.trim().trim_matches('"').trim();
        if trimmed.is_empty() {
            continue;
        }
        if entries.len() >= PATH_ENTRY_CAP {
            truncated = true;
            break;
        }
        let normalized = normalize_path(trimmed);
        let identity = path_identity(&normalized);
        // §19/§20：normalize 后按 identity 去重（保留首个 = 实际生效顺序），
        // 重复身份进 diagnostics（§21 duplicate PATH entries）。
        if !seen.insert(identity.clone()) {
            if !duplicate_identities.contains(&identity) {
                duplicate_identities.push(identity);
            }
            continue;
        }
        entries.push(PathEntry {
            raw: trimmed.to_string(),
            identity,
            normalized,
            exists: Path::new(trimmed).is_dir(),
        });
    }

    // §21 diagnostics：重复条目取首次出现的 raw 展示。
    let duplicates = duplicate_identities
        .into_iter()
        .map(|identity| {
            entries
                .iter()
                .find(|entry| entry.identity == identity)
                .map(|entry| entry.raw.clone())
                .unwrap_or(identity)
        })
        .collect();
    let missing = entries
        .iter()
        .filter(|entry| !entry.exists)
        .map(|entry| entry.raw.clone())
        .collect();

    PathFacts {
        scope: crate::model::FactScope::Process,
        entries,
        duplicates,
        missing,
        truncated,
    }
}

/// 平台规范化（§108/§109）：统一分隔符、折叠 `.` 与尾随分隔符；
/// 不做 symlink 解析（§109：保留 symlink/大小写差异，只规范化表示）。
pub fn normalize_path(path: &str) -> String {
    let unified = path.replace('/', std::path::MAIN_SEPARATOR_STR);
    let mut parts: Vec<&str> = Vec::new();
    for part in unified.split(std::path::MAIN_SEPARATOR) {
        match part {
            "" | "." => {}
            other => parts.push(other),
        }
    }
    // 盘符（"C:"）天然落在 parts[0]，无需单独前缀；只有绝对路径补前导分隔符。
    let prefix = if unified.starts_with(std::path::MAIN_SEPARATOR) {
        std::path::MAIN_SEPARATOR.to_string()
    } else {
        String::new()
    };
    format!("{prefix}{}", parts.join(std::path::MAIN_SEPARATOR_STR))
}

/// 路径身份键（§192）：Windows 大小写不敏感；其他平台原样。
pub fn path_identity(normalized: &str) -> String {
    if cfg!(windows) {
        normalized.to_ascii_lowercase()
    } else {
        normalized.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cancel() -> CancellationToken {
        CancellationToken::new()
    }

    #[test]
    fn parses_and_dedupes_entries_preserving_order() {
        let dir = std::env::temp_dir().join(format!(
            "prism-env-path-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.subsec_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let dir_str = dir.to_string_lossy().to_string();
        let missing = dir.join("definitely-missing").to_string_lossy().to_string();
        let sep = if cfg!(windows) { ';' } else { ':' };
        let raw = format!("{dir_str}{sep}{missing}{sep}{dir_str}{sep}");
        let facts = path_facts_from_raw(&raw, &cancel());

        assert_eq!(facts.entries.len(), 2, "dedupe by identity: {facts:?}");
        assert_eq!(facts.entries[0].raw, dir_str, "order preserved");
        assert!(facts.entries[0].exists);
        assert_eq!(facts.duplicates.len(), 1);
        assert_eq!(facts.missing, vec![missing]);
        assert!(!facts.truncated);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn windows_identity_is_case_insensitive() {
        if !cfg!(windows) {
            return;
        }
        assert_eq!(
            path_identity(&normalize_path("C:\\Node")),
            path_identity(&normalize_path("c:\\node"))
        );
    }

    #[test]
    fn normalizes_dot_and_duplicate_separators() {
        let sep = std::path::MAIN_SEPARATOR;
        assert_eq!(normalize_path("/a/./b//"), format!("{sep}a{sep}b"));
        assert_eq!(normalize_path("a/./b"), format!("a{sep}b"));
    }

    #[test]
    fn unicode_and_space_paths_survive_normalization() {
        // §193：CJK/空格/emoji 路径逐字符保留。
        let raw = "C:\\项目\\测试 dir\\📚";
        let normalized = normalize_path(raw);
        assert!(normalized.contains("项目"));
        assert!(normalized.contains("测试 dir"));
        assert!(normalized.contains("📚"));
    }

    #[test]
    fn caps_entries_and_flags_truncation() {
        let sep = if cfg!(windows) { ';' } else { ':' };
        let raw = (0..PATH_ENTRY_CAP + 10)
            .map(|index| format!("/dir-{index}"))
            .collect::<Vec<_>>()
            .join(&sep.to_string());
        let facts = path_facts_from_raw(&raw, &cancel());
        assert_eq!(facts.entries.len(), PATH_ENTRY_CAP);
        assert!(facts.truncated, "§314 bounded PATH");
    }
}
