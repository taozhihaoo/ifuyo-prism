//! ChangeSet 的 CLI 文本渲染（text / markdown，规格 M5 §64–§66）。
//!
//! 只渲染 ChangeSet 已有事实；不新增判断，不显示风险/影响（§69）。

use crate::file_change::FileChangeType;
use crate::model::ChangeSet;
use crate::module_change::ModuleChangeType;
use crate::symbol_change::SymbolChangeType;

/// 纯文本渲染（人类终端阅读）。
pub fn render_text(changeset: &ChangeSet) -> String {
    let mut out = String::new();
    let summary = &changeset.summary;
    out.push_str(&format!("ChangeSet: {}\n", changeset.id));
    out.push_str(&format!(
        "Scope: {}  Base: {}  Target: {}\n",
        changeset.scope.as_str(),
        changeset.base.as_deref().unwrap_or("-"),
        changeset.target.as_deref().unwrap_or("-")
    ));
    if changeset.truncated {
        out.push_str("(truncated: file limit exceeded)\n");
    }
    out.push_str(&format!(
        "Files: {} ({} added, {} modified, {} deleted, {} renamed)\n",
        summary.files_total,
        summary.files_added,
        summary.files_modified,
        summary.files_deleted,
        summary.files_renamed
    ));
    for file in &changeset.files {
        out.push_str(&format!(
            "  {} {} (+{} -{}){}\n",
            file.path(),
            change_label(file.change_type),
            file.additions.unwrap_or(0),
            file.deletions.unwrap_or(0),
            if file.is_binary { " binary" } else { "" },
        ));
    }
    out.push_str(&format!(
        "Symbols: {} ({} added, {} modified, {} deleted, {} renamed, {} moved)\n",
        summary.symbols_total,
        summary.symbols_added,
        summary.symbols_modified,
        summary.symbols_deleted,
        summary.symbols_renamed,
        summary.symbols_moved
    ));
    for symbol in &changeset.symbols {
        let characteristics = if symbol.characteristics.is_empty() {
            String::new()
        } else {
            let names: Vec<String> = symbol
                .characteristics
                .iter()
                .map(|c| format!("{c:?}").to_ascii_lowercase())
                .collect();
            format!(" [{}]", names.join(", "))
        };
        let location = symbol
            .location_after
            .or(symbol.location_before)
            .map(|range| format!(" L{}-{}", range.start_line, range.end_line))
            .unwrap_or_default();
        out.push_str(&format!(
            "  ~ {} {}{}{}\n",
            symbol.name,
            symbol_label(symbol.change_type),
            characteristics,
            location
        ));
    }
    out.push_str(&format!("Modules changed: {}\n", summary.modules_changed));
    for module in &changeset.modules {
        out.push_str(&format!(
            "  {} {} (files: {}; symbols: {})\n",
            module.module_id.as_str(),
            module_label(module.change_type),
            module.changed_files.join(", "),
            module.changed_symbol_count
        ));
    }
    out.push_str(&format!("Diagnostics: {}\n", summary.diagnostics_total));
    for diagnostic in &changeset.diagnostics {
        out.push_str(&format!(
            "  [{}] {} {}\n",
            diagnostic.kind.as_str(),
            diagnostic.path.as_deref().unwrap_or("-"),
            diagnostic.detail
        ));
    }
    out
}

/// Markdown 渲染（规格 M5 §66；结构对齐 §67 的 GUI 分区）。
pub fn render_markdown(changeset: &ChangeSet) -> String {
    let mut out = String::new();
    let summary = &changeset.summary;
    out.push_str(&format!("# ChangeSet `{}`\n\n", changeset.id));
    out.push_str(&format!(
        "- Scope: `{}`\n- Base: `{}`\n- Target: `{}`\n- Repository: `{}`\n\n",
        changeset.scope.as_str(),
        changeset.base.as_deref().unwrap_or("-"),
        changeset.target.as_deref().unwrap_or("-"),
        changeset.repository.as_deref().unwrap_or("-")
    ));
    if changeset.truncated {
        out.push_str("> truncated: file limit exceeded\n\n");
    }

    out.push_str("## Change Summary\n\n");
    out.push_str(&format!(
        "| files | added | modified | deleted | renamed | symbols | modules |\n|---|---|---|---|---|---|---|\n| {} | {} | {} | {} | {} | {} | {} |\n\n",
        summary.files_total,
        summary.files_added,
        summary.files_modified,
        summary.files_deleted,
        summary.files_renamed,
        summary.symbols_total,
        summary.modules_changed
    ));

    out.push_str("## Files\n\n");
    for file in &changeset.files {
        out.push_str(&format!(
            "- `{}` — {} (+{} / -{}){}\n",
            file.path(),
            change_label(file.change_type),
            file.additions.unwrap_or(0),
            file.deletions.unwrap_or(0),
            if file.is_binary { " (binary)" } else { "" }
        ));
    }
    out.push('\n');

    out.push_str("## Symbols\n\n");
    if changeset.symbols.is_empty() {
        out.push_str("_no symbol-level changes resolved_\n\n");
    }
    for symbol in &changeset.symbols {
        let characteristics: Vec<String> = symbol
            .characteristics
            .iter()
            .map(|c| format!("{c:?}").to_ascii_lowercase())
            .collect();
        out.push_str(&format!(
            "- `{}` ({}) — {}{} `{}`\n",
            symbol.name,
            symbol.kind.as_str(),
            symbol_label(symbol.change_type),
            if characteristics.is_empty() {
                String::new()
            } else {
                format!(" [{}]", characteristics.join(", "))
            },
            symbol.path
        ));
    }
    out.push('\n');

    out.push_str("## Modules\n\n");
    for module in &changeset.modules {
        out.push_str(&format!(
            "- `{}` — {} ({} file(s), {} symbol(s))\n",
            module.module_id.as_str(),
            module_label(module.change_type),
            module.changed_files.len(),
            module.changed_symbol_count
        ));
    }
    out.push('\n');

    out.push_str("## Diagnostics\n\n");
    if changeset.diagnostics.is_empty() {
        out.push_str("_none_\n");
    } else {
        for diagnostic in &changeset.diagnostics {
            out.push_str(&format!(
                "- `{}` {} — {}\n",
                diagnostic.kind.as_str(),
                diagnostic.path.as_deref().unwrap_or("-"),
                diagnostic.detail
            ));
        }
    }
    out
}

fn change_label(change: FileChangeType) -> &'static str {
    match change {
        FileChangeType::Added => "added",
        FileChangeType::Modified => "modified",
        FileChangeType::Deleted => "deleted",
        FileChangeType::Renamed => "renamed",
        FileChangeType::Copied => "copied",
        FileChangeType::TypeChanged => "type_changed",
        FileChangeType::Unmerged => "unmerged",
        FileChangeType::Unknown => "unknown",
    }
}

fn symbol_label(change: SymbolChangeType) -> &'static str {
    match change {
        SymbolChangeType::Added => "added",
        SymbolChangeType::Deleted => "deleted",
        SymbolChangeType::Modified => "modified",
        SymbolChangeType::Renamed => "renamed",
        SymbolChangeType::Moved => "moved",
        SymbolChangeType::Unknown => "unknown",
    }
}

fn module_label(change: ModuleChangeType) -> &'static str {
    match change {
        ModuleChangeType::Added => "added",
        ModuleChangeType::Deleted => "deleted",
        ModuleChangeType::Modified => "modified",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ChangeScope, ChangeSet, ChangeSource, ChangeSummary};

    #[test]
    fn text_and_markdown_render_empty_changeset() {
        let changeset = ChangeSet {
            schema_version: crate::model::CHANGESET_SCHEMA_VERSION,
            id: "changeset:demo#aaa#worktree#working_tree".to_string(),
            source: ChangeSource::Git,
            scope: ChangeScope::WorkingTree,
            base: Some("aaa".to_string()),
            target: Some("worktree".to_string()),
            repository: Some("demo".to_string()),
            files: vec![],
            symbols: vec![],
            modules: vec![],
            summary: ChangeSummary::default(),
            diagnostics: vec![],
            truncated: false,
        };
        let text = render_text(&changeset);
        assert!(text.contains("ChangeSet: changeset:demo"));
        assert!(text.contains("Files: 0"));
        let markdown = render_markdown(&changeset);
        assert!(markdown.contains("# ChangeSet"));
        assert!(markdown.contains("_none_"));
    }
}
