//! SymbolMatcher：diff 行 → 符号归属的确定性、保守匹配（规格 M5 §19/§22–§26/§49–§51）。
//!
//! 分层策略（§51）：
//! 1. 稳定 ID 精确匹配（同路径 + qualified + kind）
//! 2. Git rename 文件对内的 local-identity 匹配
//! 3. 同文件 rename 证据匹配（kind + parent + 主体逐行一致）
//! 4. 兜底：孤儿侧按 Added/Deleted 记录；多候选 ⇒ Ambiguous，不硬选（§83）
//!
//! 只把与变更行有真实 range 重叠的符号记为变更（§23/§24）；
//! 文件级修改不外推符号修改。全程无 AI、有界、可测（§52）。

use crate::diagnostic::{ChangeDiagnostic, ChangeDiagnosticKind};
use crate::evidence::{ChangeEvidence, EvidenceSource};
use crate::file_change::FileChangeType;
use crate::model::{ChangedRange, LineRange, MappingStatus, MatchConfidence};
use crate::revision::FileModel;
use crate::symbol_change::{SymbolChange, SymbolChangeType, SymbolCharacteristic};
use prism_git::{GitDiffHunk, GitDiffLineKind};
use prism_parser::{SourceRange, Symbol, SymbolKind};
use std::collections::BTreeMap;

/// 不参与符号匹配的整文件占位符号（range 覆盖全文件，只会制造噪音）。
fn is_matchable(symbol: &Symbol) -> bool {
    symbol.kind != SymbolKind::File
}

/// 一次 diff 的变更行事实：old/new 两侧被增删的行号集合 + hunk 摘要。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChangedLines {
    pub old: std::collections::BTreeSet<u64>,
    pub new: std::collections::BTreeSet<u64>,
    /// 全部 hunk 的行区间摘要（证据用）。
    pub ranges: Vec<ChangedRange>,
}

impl ChangedLines {
    /// 从 hunks 提取：Deletion → old 侧；Addition → new 侧；context 忽略。
    pub fn from_hunks(hunks: &[GitDiffHunk]) -> Self {
        let mut changed = ChangedLines::default();
        for hunk in hunks {
            changed.ranges.push(ChangedRange {
                old_start: Some(hunk.old_start),
                old_lines: hunk.old_lines,
                new_start: Some(hunk.new_start),
                new_lines: hunk.new_lines,
            });
            for line in &hunk.lines {
                match line.kind {
                    GitDiffLineKind::Deletion => {
                        if let Some(old_line) = line.old_line {
                            changed.old.insert(old_line);
                        }
                    }
                    GitDiffLineKind::Addition => {
                        if let Some(new_line) = line.new_line {
                            changed.new.insert(new_line);
                        }
                    }
                    GitDiffLineKind::Context => {}
                }
            }
        }
        changed
    }

    pub fn hits_old(&self, range: &SourceRange) -> bool {
        self.old
            .iter()
            .any(|line| range_contains_line(range, *line))
    }

    pub fn hits_new(&self, range: &SourceRange) -> bool {
        self.new
            .iter()
            .any(|line| range_contains_line(range, *line))
    }

    pub fn is_empty(&self) -> bool {
        self.old.is_empty() && self.new.is_empty()
    }
}

/// 行号是否落在 SourceRange 内（坐标契约：line 1-based，end 位置独占；
/// end.column <= 1 时 end 行本身不含）。
pub fn range_contains_line(range: &SourceRange, line: u64) -> bool {
    let line_u32 = match u32::try_from(line) {
        Ok(value) => value,
        Err(_) => return false,
    };
    if line_u32 < range.start.line {
        return false;
    }
    if line_u32 > range.end.line {
        return false;
    }
    // end 位置独占：end.column <= 1 时 end 行本身不含（M2 坐标契约）。
    !(line_u32 == range.end.line && range.end.column <= 1)
}

/// 符号源文本切片（byte_offset 与解析内容同坐标系，M2 契约）。
pub fn slice_symbol<'a>(content: &'a str, range: &SourceRange) -> Option<&'a str> {
    content.get(range.start.byte_offset..range.end.byte_offset)
}

/// rename 证据：去掉声明行后的主体逐行一致（§49 保守：只认完全一致）。
fn body_minus_declaration(before: &str, after: &str) -> bool {
    let before_body = before.split_once('\n').map(|(_, rest)| rest);
    let after_body = after.split_once('\n').map(|(_, rest)| rest);
    match (before_body, after_body) {
        (Some(b), Some(a)) => b == a,
        // 单行符号没有可验证主体 ⇒ 不给 rename 证据（保守）。
        _ => false,
    }
}

/// qualified 去掉最后一段（parent 语义比较用）。
fn parent_qualified(qualified: &str) -> &str {
    qualified
        .rsplit_once('.')
        .map(|(parent, _)| parent)
        .unwrap_or("")
}

/// 文件路径对应的 qualified 前缀（与 M2/M3 同规则：去最后一个短扩展名）。
pub fn qualified_root(relative_path: &str) -> &str {
    match relative_path.rsplit_once('.') {
        Some((prefix, ext)) if !ext.is_empty() && ext.len() <= 5 && !ext.contains('/') => prefix,
        _ => relative_path,
    }
}

/// rename 文件对内的 local identity：qualified 去掉路径前缀。
fn local_qualified<'a>(symbol: &'a Symbol, read_path: &str) -> &'a str {
    let root = qualified_root(read_path);
    symbol
        .qualified_name
        .strip_prefix(root)
        .and_then(|rest| rest.strip_prefix('.'))
        .unwrap_or(&symbol.qualified_name)
}

/// 匹配输入（全部为已归一化事实）。
pub struct SymbolMatchInput<'a> {
    pub file_change_id: String,
    pub path: String,
    pub file_change_type: FileChangeType,
    /// Git 明确给出的 rename 事实。
    pub git_is_rename: bool,
    pub before: Option<&'a FileModel>,
    /// before 侧读取路径（rename 时 = old_path，qualified 前缀计算用）。
    pub before_read_path: String,
    pub after: Option<&'a FileModel>,
    pub after_read_path: String,
    pub changed: &'a ChangedLines,
}

#[derive(Debug)]
pub struct SymbolMatchOutput {
    pub changes: Vec<SymbolChange>,
    pub diagnostics: Vec<ChangeDiagnostic>,
}

/// 主入口：为一个 FileChange 计算符号级变更。
pub fn compute_symbol_changes(input: &SymbolMatchInput) -> SymbolMatchOutput {
    let mut output = SymbolMatchOutput {
        changes: Vec::new(),
        diagnostics: Vec::new(),
    };

    match input.file_change_type {
        FileChangeType::Added => {
            emit_side_symbols(input, Side::After, SymbolChangeType::Added, &mut output);
        }
        FileChangeType::Deleted => {
            emit_side_symbols(input, Side::Before, SymbolChangeType::Deleted, &mut output);
        }
        _ => match_modified_pair(input, &mut output),
    }
    output
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Side {
    Before,
    After,
}

/// Added/Deleted 文件：整侧符号全部记录（§10/§11/§40/§41）。
fn emit_side_symbols(
    input: &SymbolMatchInput,
    side: Side,
    change_type: SymbolChangeType,
    output: &mut SymbolMatchOutput,
) {
    let model = match side {
        Side::Before => input.before,
        Side::After => input.after,
    };
    let Some(model) = model else {
        // Added 的 before / Deleted 的 after 天然缺失：NotApplicable，不是诊断。
        return;
    };
    let Some(parse) = &model.parse else {
        // 有内容但没解析（不支持语言/非 UTF-8/超限）→ 由 service 层诊断，这里跳过。
        return;
    };
    if !crate::revision::parse_succeeded(parse.status) {
        // 解析失败：绝不伪造"全部删除/全部新增"（§42/§84）。
        return;
    }
    for symbol in parse.symbols.iter().filter(|symbol| is_matchable(symbol)) {
        let evidence = ChangeEvidence::new(
            EvidenceSource::ParseResult,
            format!(
                "file is {} at this scope; symbol from {} parse",
                match side {
                    Side::Before => "absent after",
                    Side::After => "absent before",
                },
                source_label(model)
            ),
        )
        .with_file(input.file_change_id.clone());
        output.changes.push(SymbolChange {
            file_change_id: input.file_change_id.clone(),
            path: input.path.clone(),
            before_symbol_id: (side == Side::Before).then(|| symbol.id.clone()),
            after_symbol_id: (side == Side::After).then(|| symbol.id.clone()),
            before_qualified_name: (side == Side::Before)
                .then(|| symbol.qualified_name.clone()),
            after_qualified_name: (side == Side::After).then(|| symbol.qualified_name.clone()),
            name: symbol.name.clone(),
            kind: symbol.kind,
            change_type,
            characteristics: Vec::new(),
            location_before: (side == Side::Before).then(|| display_range(&symbol.range)),
            location_after: (side == Side::After).then(|| display_range(&symbol.range)),
            changed_ranges: Vec::new(),
            mapping_status: MappingStatus::NotApplicable,
            confidence: MatchConfidence::Unmatched,
            evidence,
        });
    }
}

fn source_label(model: &FileModel) -> &'static str {
    if model.symbols_available() {
        "successful"
    } else {
        "unavailable"
    }
}

/// Modified/Renamed 文件对：分层匹配。
fn match_modified_pair(input: &SymbolMatchInput, output: &mut SymbolMatchOutput) {
    let before_symbols = input
        .before
        .and_then(|model| model.parse.as_ref())
        .filter(|parse| crate::revision::parse_succeeded(parse.status))
        .map(|parse| {
            parse
                .symbols
                .iter()
                .filter(|symbol| is_matchable(symbol))
                .cloned()
                .collect::<Vec<_>>()
        });
    let after_symbols = input
        .after
        .and_then(|model| model.parse.as_ref())
        .filter(|parse| crate::revision::parse_succeeded(parse.status))
        .map(|parse| {
            parse
                .symbols
                .iter()
                .filter(|symbol| is_matchable(symbol))
                .cloned()
                .collect::<Vec<_>>()
        });

    let (Some(before_symbols), Some(after_symbols)) = (before_symbols, after_symbols) else {
        // 至少一侧符号事实不可用：无法匹配（§42），不伪造任何方向。
        return;
    };

    let mut matched_before_ids: std::collections::BTreeSet<String> = Default::default();
    let mut matched_after_ids: std::collections::BTreeSet<String> = Default::default();

    // ---- Stage 1：稳定 ID 精确匹配（同文件 + qualified + kind）----
    let after_by_id: BTreeMap<String, &Symbol> = after_symbols
        .iter()
        .map(|symbol| (symbol.id.as_str().to_string(), symbol))
        .collect();
    for before in &before_symbols {
        let Some(after) = after_by_id.get(before.id.as_str()) else {
            continue;
        };
        let hits_before = input.changed.hits_old(&before.range);
        let hits_after = input.changed.hits_new(&after.range);
        if !hits_before && !hits_after {
            // ID 存活且范围无变更行 ⇒ 符号未变（§23 反例）。
            matched_before_ids.insert(before.id.as_str().to_string());
            matched_after_ids.insert(after.id.as_str().to_string());
            continue;
        }
        matched_before_ids.insert(before.id.as_str().to_string());
        matched_after_ids.insert(after.id.as_str().to_string());
        output
            .changes
            .push(modified_change(input, before, after, hits_before, hits_after));
    }

    // ---- Stage 2/3：孤儿侧匹配（rename 文件 local identity / 同文件 rename）----
    let mut before_only: Vec<&Symbol> = before_symbols
        .iter()
        .filter(|symbol| !matched_before_ids.contains(symbol.id.as_str()))
        .collect();
    let mut after_only: Vec<&Symbol> = after_symbols
        .iter()
        .filter(|symbol| !matched_after_ids.contains(symbol.id.as_str()))
        .collect();

    // Stage 2：git rename 文件对内按 local identity（qualified 去路径前缀）+ kind。
    if input.git_is_rename {
        let mut used_after: std::collections::BTreeSet<String> = Default::default();
        let mut staged: Vec<(usize, usize)> = Vec::new();
        for (b_index, before) in before_only.iter().enumerate() {
            let before_local = local_qualified(before, &input.before_read_path);
            for (a_index, after) in after_only.iter().enumerate() {
                if used_after.contains(after.id.as_str()) {
                    continue;
                }
                let after_local = local_qualified(after, &input.after_read_path);
                if before.kind == after.kind && before_local == after_local {
                    staged.push((b_index, a_index));
                    used_after.insert(after.id.as_str().to_string());
                    break;
                }
            }
        }
        for (b_index, a_index) in staged {
            let before = before_only[b_index];
            let after = after_only[a_index];
            matched_before_ids.insert(before.id.as_str().to_string());
            matched_after_ids.insert(after.id.as_str().to_string());
            let hits_before = input.changed.hits_old(&before.range);
            let hits_after = input.changed.hits_new(&after.range);
            output.changes.push(renamed_or_moved_change(
                input, before, after, hits_before, hits_after,
            ));
        }
        before_only.retain(|symbol| !matched_before_ids.contains(symbol.id.as_str()));
        after_only.retain(|symbol| !matched_after_ids.contains(symbol.id.as_str()));
    }

    // Stage 3：同文件 rename 证据（kind + parent + 主体一致）。唯一候选才成立。
    {
        let mut consumed_after: std::collections::BTreeSet<String> = Default::default();
        let mut staged: Vec<(usize, usize)> = Vec::new();
        let mut ambiguous: Vec<(usize, Vec<usize>)> = Vec::new();
        for (b_index, before) in before_only.iter().enumerate() {
            let mut candidates: Vec<usize> = Vec::new();
            for (a_index, after) in after_only.iter().enumerate() {
                if consumed_after.contains(after.id.as_str()) {
                    continue;
                }
                if rename_evidence(input, before, after) {
                    candidates.push(a_index);
                }
            }
            match candidates.len() {
                0 => {}
                1 => {
                    staged.push((b_index, candidates[0]));
                    consumed_after.insert(after_only[candidates[0]].id.as_str().to_string());
                }
                _ => ambiguous.push((b_index, candidates)),
            }
        }
        for (b_index, a_index) in staged {
            let before = before_only[b_index];
            let after = after_only[a_index];
            matched_before_ids.insert(before.id.as_str().to_string());
            matched_after_ids.insert(after.id.as_str().to_string());
            let hits_before = input.changed.hits_old(&before.range);
            let hits_after = input.changed.hits_new(&after.range);
            output.changes.push(renamed_or_moved_change(
                input, before, after, hits_before, hits_after,
            ));
        }
        for (b_index, candidates) in ambiguous {
            // §83：多候选不做硬选择 —— 双侧如实记为 Deleted/Added + Ambiguous。
            let before = before_only[b_index];
            output.diagnostics.push(ChangeDiagnostic {
                kind: ChangeDiagnosticKind::SymbolMappingAmbiguous,
                path: Some(input.path.clone()),
                detail: format!(
                    "symbol '{}' matches {} candidates after rename; recorded conservatively",
                    before.qualified_name,
                    candidates.len()
                ),
            });
        }
    }

    // ---- Stage 4：剩余孤儿 ⇒ Deleted / Added（需变更行证据，§23）----
    // 保守抑制（§42/§84）：对侧解析带 error 诊断时，符号缺失可能是恢复缺失，
    // 不得据此断言 Deleted/Added —— 记 SymbolMappingUnavailable 诊断。
    let after_has_errors = input
        .after
        .and_then(|model| model.parse.as_ref())
        .is_some_and(|parse| parse.has_errors());
    let before_has_errors = input
        .before
        .and_then(|model| model.parse.as_ref())
        .is_some_and(|parse| parse.has_errors());

    let mut suppressed_deleted = false;
    for before in &before_only {
        if !matched_before_ids.contains(before.id.as_str()) && input.changed.hits_old(&before.range)
        {
            if after_has_errors {
                suppressed_deleted = true;
                continue;
            }
            output.changes.push(orphan_change(
                input, Side::Before, before, SymbolChangeType::Deleted,
            ));
        }
    }
    let mut suppressed_added = false;
    for after in &after_only {
        if !matched_after_ids.contains(after.id.as_str()) && input.changed.hits_new(&after.range) {
            if before_has_errors {
                suppressed_added = true;
                continue;
            }
            output.changes.push(orphan_change(
                input, Side::After, after, SymbolChangeType::Added,
            ));
        }
    }
    if suppressed_deleted {
        output.diagnostics.push(ChangeDiagnostic {
            kind: ChangeDiagnosticKind::SymbolMappingUnavailable,
            path: Some(input.path.clone()),
            detail: "after parse has error diagnostics; deletions suppressed as possibly unrecovered"
                .to_string(),
        });
    }
    if suppressed_added {
        output.diagnostics.push(ChangeDiagnostic {
            kind: ChangeDiagnosticKind::SymbolMappingUnavailable,
            path: Some(input.path.clone()),
            detail: "before parse has error diagnostics; additions suppressed as possibly unrecovered"
                .to_string(),
        });
    }
}

/// rename 证据（§49）：kind 相同 + parent 相同 + 主体（去声明行）逐行一致。
fn rename_evidence(input: &SymbolMatchInput, before: &Symbol, after: &Symbol) -> bool {
    if before.kind != after.kind || before.name == after.name {
        return false;
    }
    if parent_qualified(&before.qualified_name) != parent_qualified(&after.qualified_name) {
        return false;
    }
    let (Some(before_model), Some(after_model)) = (input.before, input.after) else {
        return false;
    };
    let (Some(before_text), Some(after_text)) = (model_content(before_model), model_content(after_model))
    else {
        return false;
    };
    let (Some(before_slice), Some(after_slice)) = (
        slice_symbol(before_text, &before.range),
        slice_symbol(after_text, &after.range),
    ) else {
        return false;
    };
    body_minus_declaration(before_slice, after_slice)
}

fn model_content(model: &FileModel) -> Option<&str> {
    match &model.source {
        crate::revision::SourceState::Available(content) => Some(content.as_str()),
        _ => None,
    }
}

/// 符号展示区间（end 独占 → 两端含入）。
fn display_range(range: &SourceRange) -> LineRange {
    SymbolChange::line_range(range.start.line, range.end.line, range.end.column)
}

/// Stage 1 命中的 Modified 符号变更（primary + evidence-backed characteristics，§17）。
fn modified_change(
    input: &SymbolMatchInput,
    before: &Symbol,
    after: &Symbol,
    hits_before: bool,
    hits_after: bool,
) -> SymbolChange {
    let mut characteristics: Vec<SymbolCharacteristic> = Vec::new();
    if hits_before && input.changed.old.contains(&(before.range.start.line as u64)) {
        characteristics.push(SymbolCharacteristic::DeclarationChanged);
    }
    if hits_after {
        let decl = after.range.start.line as u64;
        if input.changed.new.contains(&decl) {
            characteristics.push(SymbolCharacteristic::DeclarationChanged);
        }
    }
    if before.signature != after.signature {
        characteristics.push(SymbolCharacteristic::SignatureChanged);
    }
    if before.visibility != after.visibility {
        characteristics.push(SymbolCharacteristic::VisibilityChanged);
    }
    if before.documentation_present != after.documentation_present {
        characteristics.push(SymbolCharacteristic::DocumentationChanged);
    }
    // 主体证据：decl 行之外的变更行落在范围内。
    let body_hit = (hits_before && body_lines_hit(&input.changed.old, &before.range, true))
        || (hits_after && body_lines_hit(&input.changed.new, &after.range, true));
    if body_hit {
        match after.kind {
            SymbolKind::Function
            | SymbolKind::Method
            | SymbolKind::Constructor => {
                characteristics.push(SymbolCharacteristic::BodyChanged);
            }
            SymbolKind::Import => characteristics.push(SymbolCharacteristic::ImportChanged),
            SymbolKind::Export => characteristics.push(SymbolCharacteristic::ExportChanged),
            _ => {
                // 容器（class/namespace 等）：结构事实由 primary Modified 表达（§25）。
            }
        }
    }
    if characteristics.is_empty() {
        characteristics.push(SymbolCharacteristic::MetadataChanged);
    }
    characteristics.sort();
    characteristics.dedup();

    let ranges = overlapping_ranges(&input.changed.ranges, before, after);
    let detail = format!(
        "changed line(s) overlap symbol range (before {}-{}, after {}-{})",
        before.range.start.line,
        before.range.end.line,
        after.range.start.line,
        after.range.end.line
    );
    SymbolChange {
        file_change_id: input.file_change_id.clone(),
        path: input.path.clone(),
        before_symbol_id: Some(before.id.clone()),
        after_symbol_id: Some(after.id.clone()),
        before_qualified_name: Some(before.qualified_name.clone()),
        after_qualified_name: Some(after.qualified_name.clone()),
        name: after.name.clone(),
        kind: after.kind,
        change_type: SymbolChangeType::Modified,
        characteristics,
        location_before: Some(display_range(&before.range)),
        location_after: Some(display_range(&after.range)),
        changed_ranges: ranges.clone(),
        mapping_status: MappingStatus::Mapped,
        confidence: MatchConfidence::Exact,
        evidence: ChangeEvidence::new(EvidenceSource::SymbolRangeOverlap, detail)
            .with_file(input.file_change_id.clone())
            .with_ranges(ranges),
    }
}

/// 除声明行外是否仍有变更行落在符号范围内（body 证据）。
fn body_lines_hit(lines: &std::collections::BTreeSet<u64>, range: &SourceRange, skip_decl: bool) -> bool {
    lines.iter().any(|line| {
        if !range_contains_line(range, *line) {
            return false;
        }
        if skip_decl && *line == range.start.line as u64 {
            return false;
        }
        true
    })
}

/// 与符号范围有交集的 hunk 摘要（证据范围）。
fn overlapping_ranges(
    ranges: &[ChangedRange],
    before: &Symbol,
    after: &Symbol,
) -> Vec<ChangedRange> {
    ranges
        .iter()
        .filter(|range| {
            let hits_old = range.old_start.is_some_and(|start| {
                (start..start + range.old_lines.max(1))
                    .any(|line| range_contains_line(&before.range, line))
            });
            let hits_new = range.new_start.is_some_and(|start| {
                (start..start + range.new_lines.max(1))
                    .any(|line| range_contains_line(&after.range, line))
            });
            hits_old || hits_new
        })
        .copied()
        .collect()
}

/// Stage 2/3 命中：rename/move（git 文件 rename ⇒ Moved；同文件改名 ⇒ Renamed，§49/§50）。
fn renamed_or_moved_change(
    input: &SymbolMatchInput,
    before: &Symbol,
    after: &Symbol,
    hits_before: bool,
    hits_after: bool,
) -> SymbolChange {
    let mut change = modified_change(input, before, after, hits_before, hits_after);
    let file_renamed = input.git_is_rename
        && input.before_read_path != input.after_read_path;
    change.change_type = if file_renamed {
        SymbolChangeType::Moved
    } else {
        SymbolChangeType::Renamed
    };
    change.confidence = MatchConfidence::HighConfidence;
    change.evidence = if file_renamed {
        ChangeEvidence::new(
            EvidenceSource::GitRename,
            format!(
                "git reports rename {} -> {}; local identity '{}' matched",
                input.before_read_path, input.after_read_path, after.name
            ),
        )
    } else {
        ChangeEvidence::new(
            EvidenceSource::SymbolRangeOverlap,
            format!(
                "same parent + kind + identical body for '{}' -> '{}'",
                before.name, after.name
            ),
        )
    }
    .with_file(input.file_change_id.clone())
    .with_ranges(change.changed_ranges.clone());
    change
}

/// 孤儿符号（无法配对）⇒ Deleted/Added；git rename 不确定时附诊断由调用方补。
fn orphan_change(
    input: &SymbolMatchInput,
    side: Side,
    symbol: &Symbol,
    change_type: SymbolChangeType,
) -> SymbolChange {
    let (symbol_id, qualified, location) = match side {
        Side::Before => (
            Some(symbol.id.clone()),
            Some(symbol.qualified_name.clone()),
            Some(display_range(&symbol.range)),
        ),
        Side::After => (
            None,
            None,
            None,
        ),
    };
    let (before_symbol_id, after_symbol_id) = match side {
        Side::Before => (symbol_id, None),
        Side::After => (None, Some(symbol.id.clone())),
    };
    let (before_qualified_name, after_qualified_name) = match side {
        Side::Before => (qualified, None),
        Side::After => (None, Some(symbol.qualified_name.clone())),
    };
    let location_before = if side == Side::Before {
        location
    } else {
        None
    };
    let location_after = if side == Side::After {
        Some(display_range(&symbol.range))
    } else {
        None
    };
    SymbolChange {
        file_change_id: input.file_change_id.clone(),
        path: input.path.clone(),
        before_symbol_id,
        after_symbol_id,
        before_qualified_name,
        after_qualified_name,
        name: symbol.name.clone(),
        kind: symbol.kind,
        change_type,
        characteristics: Vec::new(),
        location_before,
        location_after,
        changed_ranges: Vec::new(),
        mapping_status: MappingStatus::Mapped,
        confidence: MatchConfidence::Unmatched,
        evidence: ChangeEvidence::new(
            EvidenceSource::SymbolRangeOverlap,
            format!(
                "symbol only exists on {} side and its range intersects changed line(s)",
                match side {
                    Side::Before => "before",
                    Side::After => "after",
                }
            ),
        )
        .with_file(input.file_change_id.clone()),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::revision::{FileModel, SourceState};
    use prism_core::ids::FileId;
    use prism_core::task::CancellationToken;
    use prism_parser::{ParseRequest, ParserRegistry};
    use prism_core::Confidence;
    use prism_project::{FileClassification, FileKind};

    fn file_model(path: &str, content: &str) -> FileModel {
        let classifier = prism_project::FileClassifier::new();
        let registry = ParserRegistry::with_defaults();
        let file_name = std::path::Path::new(path)
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();
        let language = classifier.detect_language(&file_name).expect("ts test fixture");
        let request = ParseRequest {
            file_id: FileId::from_string(format!("file:{path}")),
            relative_path: path.to_string(),
            language,
            content: content.to_string(),
        };
        let parse = registry
            .get(language)
            .unwrap()
            .parse(&request, &CancellationToken::new())
            .unwrap();
        FileModel {
            relative_path: path.to_string(),
            language: Some(language),
            classification: FileClassification {
                kind: FileKind::Source,
                confidence: Confidence::High,
                reason: "test fixture".to_string(),
            },
            source: SourceState::Available(content.to_string()),
            parse: Some(parse),
        }
    }

    fn changed(old: &[u64], new: &[u64]) -> ChangedLines {
        let mut changed = ChangedLines::default();
        changed.old.extend(old.iter().copied());
        changed.new.extend(new.iter().copied());
        changed
    }

    fn run<'a>(
        before: Option<&'a FileModel>,
        after: Option<&'a FileModel>,
        changed_lines: &'a ChangedLines,
        file_change_type: FileChangeType,
    ) -> SymbolMatchOutput {
        let input = SymbolMatchInput {
            file_change_id: "fc:src/a.ts".to_string(),
            path: "src/a.ts".to_string(),
            file_change_type,
            git_is_rename: false,
            before,
            before_read_path: "src/a.ts".to_string(),
            after,
            after_read_path: "src/a.ts".to_string(),
            changed: changed_lines,
        };
        compute_symbol_changes(&input)
    }

    fn find<'a>(output: &'a SymbolMatchOutput, name: &str) -> &'a SymbolChange {
        output
            .changes
            .iter()
            .find(|change| change.name == name)
            .unwrap_or_else(|| panic!("symbol change '{name}' not found"))
    }

    const BEFORE_SRC: &str = "\
export function alpha() {
  return 1;
}

export function beta(x: number) {
  return x + 1;
}

export function gamma() {
  return 3;
}
";

    #[test]
    fn changed_lines_from_hunks_extracts_both_sides() {
        let hunk = GitDiffHunk {
            old_start: 2,
            old_lines: 4,
            new_start: 2,
            new_lines: 5,
            header: String::new(),
            lines: vec![
                prism_git::GitDiffLine {
                    kind: GitDiffLineKind::Context,
                    old_line: Some(2),
                    new_line: Some(2),
                    content: " ctx".to_string(),
                },
                prism_git::GitDiffLine {
                    kind: GitDiffLineKind::Deletion,
                    old_line: Some(3),
                    new_line: None,
                    content: "-old".to_string(),
                },
                prism_git::GitDiffLine {
                    kind: GitDiffLineKind::Addition,
                    old_line: None,
                    new_line: Some(4),
                    content: "+new".to_string(),
                },
            ],
        };
        let changed_lines = ChangedLines::from_hunks(&[hunk]);
        assert_eq!(changed_lines.old.iter().copied().collect::<Vec<_>>(), vec![3]);
        assert_eq!(changed_lines.new.iter().copied().collect::<Vec<_>>(), vec![4]);
        assert_eq!(changed_lines.ranges.len(), 1);
    }

    #[test]
    fn body_change_yields_modified_with_body_changed() {
        let before = file_model("src/a.ts", BEFORE_SRC);
        let after_src = BEFORE_SRC.replace("return 3;", "return 42;");
        let after = file_model("src/a.ts", &after_src);
        let changed_lines = changed(&[10], &[10]);
        let output = run(Some(&before), Some(&after), &changed_lines, FileChangeType::Modified);

        let gamma = find(&output, "gamma");
        assert_eq!(gamma.change_type, SymbolChangeType::Modified);
        assert!(gamma.characteristics.contains(&SymbolCharacteristic::BodyChanged));
        assert_eq!(gamma.confidence, MatchConfidence::Exact);
        assert_eq!(gamma.mapping_status, MappingStatus::Mapped);
        // 未触及的符号不得出现（§23：不靠文件级修改外推）。
        assert!(output.changes.iter().all(|change| change.name != "alpha"));
        assert!(output.changes.iter().all(|change| change.name != "beta"));
    }

    #[test]
    fn signature_change_yields_signature_changed() {
        let before = file_model("src/a.ts", BEFORE_SRC);
        let after_src = BEFORE_SRC.replace("beta(x: number)", "beta(x: number, y: number)");
        let after = file_model("src/a.ts", &after_src);
        let changed_lines = changed(&[5], &[5]);
        let output = run(Some(&before), Some(&after), &changed_lines, FileChangeType::Modified);
        let beta = find(&output, "beta");
        assert!(beta.characteristics.contains(&SymbolCharacteristic::SignatureChanged));
        assert_eq!(beta.change_type, SymbolChangeType::Modified);
    }

    #[test]
    fn same_file_rename_detected_conservatively() {
        let before = file_model("src/a.ts", BEFORE_SRC);
        let after_src = BEFORE_SRC.replace("gamma", "gamma_renamed");
        let after = file_model("src/a.ts", &after_src);
        let changed_lines = changed(&[9], &[9]);
        let output = run(Some(&before), Some(&after), &changed_lines, FileChangeType::Modified);
        let renamed = find(&output, "gamma_renamed");
        assert_eq!(renamed.change_type, SymbolChangeType::Renamed);
        assert_eq!(renamed.confidence, MatchConfidence::HighConfidence);
        assert!(renamed.before_qualified_name.is_some());
        assert!(output.diagnostics.is_empty(), "unique candidate: {output:?}");
        // 不应同时出现 Deleted gamma + Added gamma_renamed。
        assert!(output.changes.iter().all(|change| change.change_type != SymbolChangeType::Deleted));
    }

    #[test]
    fn duplicate_symbols_stay_ambiguous() {
        // zeta 与两个 after 候选主体一致 ⇒ Ambiguous，不硬选（§83）。
        let before_text = "export function helper() {\n  return 2;\n}\n\nexport function zeta() {\n  return 2;\n}\n";
        let after_text = "export function a1() {\n  return 2;\n}\n\nexport function a2() {\n  return 2;\n}\n";
        let before = file_model("src/a.ts", before_text);
        let after = file_model("src/a.ts", after_text);
        let changed_lines = changed(&[1, 5], &[1, 5]);
        let output = run(Some(&before), Some(&after), &changed_lines, FileChangeType::Modified);
        assert!(
            output
                .diagnostics
                .iter()
                .any(|d| d.kind == ChangeDiagnosticKind::SymbolMappingAmbiguous),
            "expected ambiguous diagnostic: {output:?}"
        );
    }

    #[test]
    fn nested_symbol_change_marks_both_levels() {
        let before_src = "export class Widget {\n  render() {\n    return 1;\n  }\n}\n";
        let after_src = "export class Widget {\n  render() {\n    return 2;\n  }\n}\n";
        let before = file_model("src/a.ts", before_src);
        let after = file_model("src/a.ts", after_src);
        let changed_lines = changed(&[3], &[3]);
        let output = run(Some(&before), Some(&after), &changed_lines, FileChangeType::Modified);
        let render = find(&output, "render");
        assert_eq!(render.change_type, SymbolChangeType::Modified);
        assert!(render.characteristics.contains(&SymbolCharacteristic::BodyChanged));
        let widget = find(&output, "Widget");
        assert_eq!(widget.change_type, SymbolChangeType::Modified);
        assert!(
            !widget.characteristics.contains(&SymbolCharacteristic::BodyChanged),
            "container is structurally modified, not body-changed (§25)"
        );
    }

    #[test]
    fn multi_symbol_hunk_covers_all_overlapped_symbols() {
        let before_src = "export function one() {\n  return 1;\n}\n\nexport function two() {\n  return 2;\n}\n";
        let after_src = "export function one() {\n  return 11;\n}\n\nexport function two() {\n  return 22;\n}\n";
        let before = file_model("src/a.ts", before_src);
        let after = file_model("src/a.ts", after_src);
        // 一个大 hunk 覆盖两个符号（§24：全部实际覆盖者都要出现）。
        let changed_lines = changed(&[2, 6], &[2, 6]);
        let output = run(Some(&before), Some(&after), &changed_lines, FileChangeType::Modified);
        assert!(output.changes.iter().any(|change| change.name == "one"));
        assert!(output.changes.iter().any(|change| change.name == "two"));
    }

    #[test]
    fn added_and_deleted_files_emit_whole_side() {
        let after = file_model("src/new.ts", "export function fresh() {\n  return 1;\n}\n");
        let changed_lines = changed(&[], &[]);
        let output = run(None, Some(&after), &changed_lines, FileChangeType::Added);
        let added = find(&output, "fresh");
        assert_eq!(added.change_type, SymbolChangeType::Added);
        assert_eq!(added.mapping_status, MappingStatus::NotApplicable);
        assert!(added.before_symbol_id.is_none());
        assert!(added.after_symbol_id.is_some());

        let before = file_model("src/old.ts", "export function stale() {\n  return 1;\n}\n");
        let output = run(Some(&before), None, &changed_lines, FileChangeType::Deleted);
        let deleted = find(&output, "stale");
        assert_eq!(deleted.change_type, SymbolChangeType::Deleted);
        assert!(deleted.before_symbol_id.is_some());
    }

    #[test]
    fn file_symbol_kind_is_filtered_from_matching() {
        let before = file_model("src/a.ts", BEFORE_SRC);
        let after_src = BEFORE_SRC.replace("return 3;", "return 33;");
        let after = file_model("src/a.ts", &after_src);
        let changed_lines = changed(&[10], &[10]);
        let output = run(Some(&before), Some(&after), &changed_lines, FileChangeType::Modified);
        assert!(
            output.changes.iter().all(|change| change.kind != SymbolKind::File),
            "whole-file symbol must not swallow changes"
        );
    }

    #[test]
    fn range_contains_line_honors_exclusive_end() {
        let range = SourceRange {
            start: prism_parser::SourceLocation { line: 20, column: 1, byte_offset: 0 },
            end: prism_parser::SourceLocation { line: 46, column: 1, byte_offset: 100 },
        };
        assert!(range_contains_line(&range, 20));
        assert!(range_contains_line(&range, 45));
        assert!(!range_contains_line(&range, 46), "end.column=1 excludes end line");
        assert!(!range_contains_line(&range, 19));
    }
}
