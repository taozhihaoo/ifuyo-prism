//! ContextMaterializer（规格 M10a §166–§177/§195–§203/§254/§324–§329）。
//!
//! 懒物化（§202）：candidate → 估算 → 选择 → **现在才读文件** → 切片 →
//! 脱敏（复用中央 privacy，§346）→ 边界截断（§36/§37）→ ContextItem。
//! 全程只读（§156–§158）；取消在逐 item 间生效（§324）。

use crate::model::{ContextItem, FileContentLevel, Priority};
use prism_core::privacy::redact_text;
use prism_core::task::CancellationToken;
use prism_analysis::ProjectCodeModel;
use std::path::Path;

/// 巨型文件阈值（超过 ⇒ skeleton/metadata-only，§174）。
const HUGE_FILE_BYTES: u64 = 96 * 1024;

/// 文件特殊形态（§169–§173）：默认 metadata-only / 排除，不塞全文。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileSpecialness {
    Normal,
    Binary,
    Generated,
    Vendor,
    Lockfile,
    Minified,
}

/// 路径形态判定（保守后缀/目录规则，§169–§173）。
pub fn classify_path(path: &str) -> FileSpecialness {
    let lower = path.to_ascii_lowercase();
    let name = lower.rsplit('/').next().unwrap_or(&lower);
    const BINARY_EXT: [&str; 12] = [
        "png", "jpg", "jpeg", "gif", "ico", "pdf", "zip", "jar", "exe", "dll", "woff", "woff2",
    ];
    if let Some(ext) = name.rsplit('.').next() {
        if BINARY_EXT.contains(&ext) && name.contains('.') {
            return FileSpecialness::Binary;
        }
    }
    if name.ends_with(".min.js") || name.ends_with(".min.css") {
        return FileSpecialness::Minified;
    }
    const LOCKFILES: [&str; 6] = [
        "package-lock.json", "cargo.lock", "pnpm-lock.yaml", "yarn.lock", "uv.lock", "poetry.lock",
    ];
    if LOCKFILES.contains(&name) {
        return FileSpecialness::Lockfile;
    }
    if lower.contains("node_modules/")
        || lower.contains("/vendor/")
        || lower.starts_with("vendor/")
        || lower.contains("/target/")
        || lower.contains("/dist/")
    {
        return FileSpecialness::Vendor;
    }
    if name.contains(".generated.") || name.ends_with(".g.cs") || name.ends_with(".d.ts") {
        return FileSpecialness::Generated;
    }
    FileSpecialness::Normal
}

/// 物化结果汇总。
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct MaterializeReport {
    pub materialized: usize,
    pub metadata_only: usize,
    pub unavailable: usize,
    pub truncated: usize,
}

/// 逐 item 物化（§202/§167：只读被选中的文件/范围）。
/// `remaining_bytes` 随物化递减（selector 已按优先级排序 ⇒ Critical 先吃预算）。
pub fn materialize(
    items: &mut [ContextItem],
    project_root: &Path,
    code_model: Option<&ProjectCodeModel>,
    budget: &crate::model::ContextBudget,
    cancel: &CancellationToken,
) -> MaterializeReport {
    let mut remaining_bytes = budget.max_bytes.saturating_sub(
        items
            .iter()
            .map(|item| item.included_size.unwrap_or(0))
            .sum::<u64>(),
    );
    let mut report = MaterializeReport::default();
    for item in items.iter_mut() {
        if cancel.is_cancelled() {
            // §324：取消 ⇒ 未物化的 item 保持 metadata-only（不假装完整）。
            break;
        }
        let Some(path) = item.source.path.clone() else {
            // 无文件来源（CheckRun/Finding/概览等）：metadata 即内容。
            item.content = Some(compose_metadata_content(item));
            item.content_level = FileContentLevel::MetadataOnly;
            report.metadata_only += 1;
            continue;
        };
        let specialness = classify_path(&path);
        let full_path = project_root.join(&path);
        if !full_path.is_file() {
            // §326：文件消失 ⇒ unavailable，不整体失败。
            item.unavailable_reason = Some("Current file no longer exists".to_string());
            item.content_level = FileContentLevel::MetadataOnly;
            item.content = Some(compose_metadata_content(item));
            report.unavailable += 1;
            continue;
        }
        let size = std::fs::metadata(&full_path).map(|meta| meta.len()).unwrap_or(0);
        let special_only = matches!(
            specialness,
            FileSpecialness::Binary | FileSpecialness::Minified | FileSpecialness::Lockfile
        ) || size > HUGE_FILE_BYTES && item.priority != Priority::Critical;
        if special_only || matches!(specialness, FileSpecialness::Generated | FileSpecialness::Vendor)
        {
            // §169–§173/§38：metadata/skeleton，不搬运全文。
            item.content = Some(match specialness {
                FileSpecialness::Binary => format!("[binary file omitted: {path}]"),
                FileSpecialness::Minified => format!("[minified file omitted: {path}]"),
                FileSpecialness::Lockfile => format!(
                    "[lockfile metadata only: {path}] (full content excluded by policy, §172)"
                ),
                _ => skeleton_for(&path, code_model)
                    .unwrap_or_else(|| format!("[generated/vendor file omitted: {path}]")),
            });
            item.content_level = if specialness == FileSpecialness::Lockfile
                || specialness == FileSpecialness::Binary
                || specialness == FileSpecialness::Minified
            {
                FileContentLevel::MetadataOnly
            } else {
                FileContentLevel::Skeleton
            };
            item.included_size = item.content.as_ref().map(|content| content.len() as u64);
            report.metadata_only += 1;
            continue;
        }

        // ---- 正常读取 + 切片（§168/§203）----
        let Ok(raw) = std::fs::read_to_string(&full_path) else {
            item.unavailable_reason = Some("file could not be read (permission/encoding)".to_string());
            item.content_level = FileContentLevel::MetadataOnly;
            item.content = Some(compose_metadata_content(item));
            report.unavailable += 1;
            continue;
        };
        let total_lines = raw.lines().count() as u64;
        let (start, end) = slice_range(item, total_lines);
        let selected: String = raw
            .lines()
            .skip(start.saturating_sub(1) as usize)
            .take((end.saturating_sub(start).saturating_add(1)) as usize)
            .collect::<Vec<_>>()
            .join("\n");
        // 剩余预算截断：沿行边界（§35/§36），Critical 也如实标 truncated（§195）。
        let content = truncate_at_line_boundary(&selected, remaining_bytes.max(512));
        let truncated = content.len() < selected.len();
        let redacted = redact_text(&content);
        item.content = Some(redacted.clone());
        item.content_level = if truncated || (start, end) != (1, total_lines) {
            FileContentLevel::RelevantSymbols
        } else {
            FileContentLevel::Full
        };
        item.truncated = truncated;
        item.original_size = Some(selected.len() as u64);
        item.included_size = Some(redacted.len() as u64);
        if truncated {
            item.truncation_reason = Some(
                "budget exceeded — truncated at line boundary (no mid-token cut, §36)".to_string(),
            );
            report.truncated += 1;
        }
        item.content_hash = Some(fnv1a(&redacted));
        item.ranges = vec![crate::model::ContextRange {
            start,
            end,
            reason: item
                .reasons
                .first()
                .copied()
                .unwrap_or(crate::model::SelectionReason::TaskMatch),
            symbol_id: Some(item.source.source_id.clone()),
            symbol_name: Some(item.title.clone()),
        }];
        remaining_bytes = remaining_bytes.saturating_sub(redacted.len() as u64);
        report.materialized += 1;
    }
    report
}

/// 切片范围（§203）：有 symbol_range 用之；否则全文。
fn slice_range(item: &ContextItem, total_lines: u64) -> (u64, u64) {
    match item.ranges.first() {
        Some(range) => (range.start.max(1), range.end.min(total_lines).max(range.start)),
        None => (1, total_lines),
    }
}

/// 行边界截断（§36：不从中间随机切）。
fn truncate_at_line_boundary(content: &str, max_bytes: u64) -> String {
    if (content.len() as u64) <= max_bytes {
        return content.to_string();
    }
    let mut taken = 0u64;
    let mut lines = Vec::new();
    for line in content.lines() {
        let line_len = line.len() as u64 + 1;
        if taken + line_len > max_bytes {
            break;
        }
        taken += line_len;
        lines.push(line);
    }
    lines.join("\n")
}

/// §39/§422：结构骨架（class + 成员签名），高价值低 token。
fn skeleton_for(path: &str, code_model: Option<&ProjectCodeModel>) -> Option<String> {
    let model = code_model?;
    for result in &model.parse_results {
        if result.relative_path != path {
            continue;
        }
        if result.symbols.is_empty() {
            return None;
        }
        let mut skeleton = String::new();
        for symbol in &result.symbols {
            let signature = symbol
                .signature
                .clone()
                .unwrap_or_else(|| format!("{}(...)", symbol.name));
            skeleton.push_str(&format!(
                "{} {} · {} · L{}-L{}\n",
                format!("{:?}", symbol.kind).to_lowercase(),
                symbol.name,
                signature,
                symbol.range.start.line,
                symbol.range.end.line
            ));
        }
        return Some(format!("[skeleton: {path}]\n{skeleton}"));
    }
    None
}

/// metadata-only 内容（CheckRun/Finding/概览等无文件来源）。
fn compose_metadata_content(item: &ContextItem) -> String {
    let mut content = String::new();
    content.push_str(&item.title);
    content.push('\n');
    for reason in &item.reasons {
        content.push_str(&format!("reason: {}\n", reason.as_str()));
    }
    for evidence in &item.evidence {
        content.push_str(&format!("evidence: {evidence}\n"));
    }
    redact_text(&content)
}

/// FNV-1a（§218/§219：对 redacted 内容 hash，不引入外部依赖）。
fn fnv1a(text: &str) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in text.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ContextSource, SourceType};

    fn item_with(path: Option<&str>, content: Option<&str>) -> ContextItem {
        ContextItem {
            id: format!("ctx:test:{path:?}"),
            source: ContextSource {
                source_type: path.map_or(SourceType::CheckRun, |_| SourceType::File),
                source_id: format!("test:{path:?}"),
                path: path.map(str::to_string),
                location: None,
                provenance: "test".to_string(),
            },
            group: crate::model::ContextGroup::TargetCode,
            title: format!("item {path:?}"),
            content: content.map(str::to_string),
            location: None,
            priority: Priority::High,
            reasons: vec![crate::model::SelectionReason::DirectlyChanged],
            evidence: vec![],
            language: None,
            change_state: None,
            impact_distance: None,
            truncated: false,
            original_size: None,
            included_size: None,
            truncation_reason: None,
            content_level: FileContentLevel::MetadataOnly,
            detail_level: crate::model::DetailLevel::Focused,
            ranges: vec![],
            unavailable_reason: None,
            content_hash: None,
        }
    }

    fn temp(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "prism-ctx-mat-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.subsec_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn special_files_become_metadata_or_skeleton() {
        assert_eq!(classify_path("logo.png"), FileSpecialness::Binary);
        assert_eq!(classify_path("app.min.js"), FileSpecialness::Minified);
        assert_eq!(classify_path("package-lock.json"), FileSpecialness::Lockfile);
        assert_eq!(classify_path("node_modules/x/y.js"), FileSpecialness::Vendor);
        assert_eq!(classify_path("src/bundle.generated.ts"), FileSpecialness::Generated);
        assert_eq!(classify_path("src/app.ts"), FileSpecialness::Normal);
    }

    #[test]
    fn missing_file_marks_unavailable_not_failure() {
        let dir = temp("missing");
        let mut items = vec![item_with(Some("src/gone.ts"), None)];
        let report = materialize(
            &mut items,
            &dir,
            None,
            &crate::model::ContextBudget::default(),
            &CancellationToken::new(),
        );
        assert_eq!(report.unavailable, 1);
        assert_eq!(
            items[0].unavailable_reason.as_deref(),
            Some("Current file no longer exists")
        );
    }

    #[test]
    fn truncation_respects_line_boundary_and_records_sizes() {
        let dir = temp("truncate");
        // 200 行 × 100 字节 = 20KB；预算 4KB ⇒ 沿行边界截断。
        let body: String = (0..200)
            .map(|index| format!("line-{index:04} {}", "x".repeat(80)))
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(dir.join("big.txt"), &body).unwrap();
        let mut items = vec![item_with(Some("big.txt"), None)];
        let budget = crate::model::ContextBudget {
            max_items: 8,
            max_source_files: 8,
            max_bytes: 4 * 1024,
            max_tokens_estimate: 1024,
        };
        let report = materialize(&mut items, &dir, None, &budget, &CancellationToken::new());
        assert_eq!(report.truncated, 1);
        assert!(items[0].truncated);
        assert!(items[0].truncation_reason.as_deref().unwrap().contains("line boundary"));
        let original = items[0].original_size.unwrap();
        let included = items[0].included_size.unwrap();
        assert!(included < original);
        // 边界完整：内容以完整行结尾（无半行）。
        let content = items[0].content.as_deref().unwrap();
        assert!(content.starts_with("line-0000"), "head-first, line-boundary cut");
    }

    #[test]
    fn secrets_redacted_during_materialization() {
        let dir = temp("secret");
        std::fs::write(dir.join("cfg.txt"), "API_TOKEN=super-secret-value\nnormal=line\n").unwrap();
        let mut items = vec![item_with(Some("cfg.txt"), None)];
        materialize(
            &mut items,
            &dir,
            None,
            &crate::model::ContextBudget::default(),
            &CancellationToken::new(),
        );
        let content = items[0].content.as_deref().unwrap();
        assert!(!content.contains("super-secret-value"), "§ secret never leaks");
        assert!(content.contains("[REDACTED]"));
    }

    #[test]
    fn metadata_only_items_compose_content_from_facts() {
        let mut items = vec![item_with(None, None)];
        materialize(
            &mut items,
            Path::new("."),
            None,
            &crate::model::ContextBudget::default(),
            &CancellationToken::new(),
        );
        let content = items[0].content.as_deref().unwrap();
        assert!(content.contains("reason: directly_changed"));
    }
}
