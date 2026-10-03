//! ChangeSetService：把 Git diff + 双侧代码模型编排为 ChangeSet
//! （规格 M5 §3/§109 的分步实现；无千行 monolith，§110）。
//!
//! 流程：
//! ```text
//! discover → resolve revisions → git diff (+untracked for WorkingTree)
//!   → 双侧仅变更文件 SideModel（bounded 并发；binary 不读内容）
//!   → FileChange 归一化 → SymbolMatcher → ModuleChange 聚合
//!   → summary + diagnostics → 确定性排序 → ChangeSet
//! ```
//!
//! 部分成功（§34）：单文件解析/读取失败只产生 diagnostic，不击穿；
//! 只有 base/target revision 无法解析这类"无法构建输入"才整体失败（§80）。

use crate::diagnostic::{diagnostic_sort_key, ChangeDiagnostic, ChangeDiagnosticKind};
use crate::evidence::{ChangeEvidence, EvidenceSource};
use crate::file_change::{FileChange, FileChangeType};
use crate::matcher::{compute_symbol_changes, ChangedLines, SymbolMatchInput};
use crate::model::{
    ChangeScope, ChangeSet, ChangeSource, ChangeSummary, MappingStatus, CHANGESET_SCHEMA_VERSION,
};
use crate::module_change::{ModuleChange, ModuleChangeType};
use crate::revision::{
    build_side_model, CurrentWorkingTreeSource, FileModel, GitRevisionSource, RevisionSource,
    SideModel, SourceState,
};
use crate::symbol_change::{SymbolChange, SymbolChangeType};
use prism_core::task::CancellationToken;
use prism_core::{PrismError, PrismResult};
use prism_git::{GitChangeType, GitDiffFile, GitDiffScope, GitService};
use prism_graph::{file_module_id, namespace_module_id, ModuleId, ModuleKind};
use prism_project::FileKind;
use std::path::Path;
use std::sync::Arc;

/// 资源上限与并发（§78；复用 M1/M2 的有界风格）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChangeSetOptions {
    /// 文件数上限（与 M4 diff 一致；超出 truncated=true）。
    pub max_files: usize,
    /// 单文件读取字节上限。
    pub max_file_bytes: u64,
    pub max_workers: usize,
}

impl Default for ChangeSetOptions {
    fn default() -> Self {
        let workers = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1)
            .clamp(1, 4);
        ChangeSetOptions {
            max_files: 200,
            max_file_bytes: 1_000_000,
            max_workers: workers,
        }
    }
}

/// 双侧解析可用性（决定 FileChange.mapping_status 与诊断，§42）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SideParseState {
    Parsed,
    ParseFailed,
    UnsupportedLanguage,
    Generated,
    TooLarge,
    EncodingUnsupported,
    ReadUnavailable,
    NotNeeded,
}

#[derive(Clone)]
pub struct ChangeSetService {
    runner: Arc<dyn prism_core::CommandRunner>,
    git: GitService,
}

impl std::fmt::Debug for ChangeSetService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChangeSetService").finish()
    }
}

impl ChangeSetService {
    pub fn new(runner: Arc<dyn prism_core::CommandRunner>) -> Self {
        let git = GitService::new(runner.clone());
        ChangeSetService { runner, git }
    }

    /// 构建 ChangeSet；非 Git 项目返回 Ok(None)（不是错误，规格 §25）。
    #[allow(clippy::too_many_arguments)]
    pub fn build(
        &self,
        root: &Path,
        scope: ChangeScope,
        base: Option<&str>,
        target: Option<&str>,
        options: &ChangeSetOptions,
        cancel: &CancellationToken,
    ) -> PrismResult<Option<ChangeSet>> {
        let Some(repository) = self.git.discover(root, cancel)? else {
            return Ok(None);
        };
        let repository_name = Path::new(&repository.root_path)
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| "repository".to_string());

        // ---- 解析 revision 身份（§6/§80：解析失败=无法构建输入=整体失败）----
        let head_sha = self.git.resolve_revision(root, "HEAD", cancel)?;
        let (base_id, target_id) = match scope {
            ChangeScope::WorkingTree => (
                head_sha.clone().unwrap_or_else(|| "unborn".to_string()),
                "worktree".to_string(),
            ),
            ChangeScope::Staged => (
                head_sha.unwrap_or_else(|| "unborn".to_string()),
                "index".to_string(),
            ),
            ChangeScope::RevisionRange => {
                let base_raw = base.ok_or_else(|| PrismError::InvalidInput {
                    message: "revision range requires --from".to_string(),
                })?;
                let target_raw = target.ok_or_else(|| PrismError::InvalidInput {
                    message: "revision range requires --to".to_string(),
                })?;
                let base_resolved = self
                    .git
                    .resolve_revision(root, base_raw, cancel)?
                    .ok_or_else(|| PrismError::InvalidInput {
                        message: format!("invalid base revision: {base_raw}"),
                    })?;
                let target_resolved = self
                    .git
                    .resolve_revision(root, target_raw, cancel)?
                    .ok_or_else(|| PrismError::InvalidInput {
                        message: format!("invalid target revision: {target_raw}"),
                    })?;
                (base_resolved, target_resolved)
            }
        };

        // ---- Git diff（每侧一次批量查询，§72）----
        let git_scope = match scope {
            ChangeScope::WorkingTree => GitDiffScope::WorkingTree,
            ChangeScope::Staged => GitDiffScope::Staged,
            ChangeScope::RevisionRange => GitDiffScope::RevisionRange,
        };
        let diff = self.git.diff(root, git_scope, base, target, options.max_files, cancel)?;

        // ---- WorkingTree scope 合并 untracked（git diff 不含 untracked）----
        let mut diff_files = diff.files.clone();
        if scope == ChangeScope::WorkingTree {
            let status = self.git.status(root, false, cancel)?;
            for entry in status.entries.iter().filter(|entry| entry.is_untracked) {
                diff_files.push(untracked_as_added(entry.path.clone()));
            }
            diff_files.sort_by_path();
        }
        // 合并后的最终视图截断（untracked 合并可能超限；§35 如实标注）。
        let mut truncated = diff.truncated;
        if diff_files.len() > options.max_files {
            diff_files.truncate(options.max_files);
            truncated = true;
        }

        // ---- 双侧读取路径（rename 读旧路径；Added/Deleted/binary 不读缺失侧）----
        let needed = |file: &&GitDiffFile| !file.is_binary;
        let before_paths: Vec<String> = diff_files
            .iter()
            .filter(needed)
            .filter(|file| file.change_type != GitChangeType::Added)
            .filter_map(|file| file.old_path.clone().or_else(|| file.new_path.clone()))
            .collect();
        let after_paths: Vec<String> = diff_files
            .iter()
            .filter(needed)
            .filter(|file| file.change_type != GitChangeType::Deleted)
            .filter_map(|file| file.new_path.clone().or_else(|| file.old_path.clone()))
            .collect();

        let before_source: Box<dyn RevisionSource> = match scope {
            ChangeScope::WorkingTree | ChangeScope::Staged => Box::new(GitRevisionSource::new(
                self.runner.clone(),
                root,
                if scope == ChangeScope::Staged { ":0" } else { "HEAD" },
            )),
            ChangeScope::RevisionRange => Box::new(GitRevisionSource::new(
                self.runner.clone(),
                root,
                base.unwrap_or("HEAD"),
            )),
        };
        let after_source: Box<dyn RevisionSource> = match scope {
            ChangeScope::WorkingTree => Box::new(CurrentWorkingTreeSource::new(root)),
            ChangeScope::Staged => {
                Box::new(GitRevisionSource::new(self.runner.clone(), root, ":0"))
            }
            ChangeScope::RevisionRange => Box::new(GitRevisionSource::new(
                self.runner.clone(),
                root,
                target.unwrap_or("HEAD"),
            )),
        };

        let before_model = build_side_model(
            &before_paths,
            before_source.as_ref(),
            options.max_file_bytes,
            options.max_workers,
            cancel,
        )?;
        let after_model = build_side_model(
            &after_paths,
            after_source.as_ref(),
            options.max_file_bytes,
            options.max_workers,
            cancel,
        )?;

        // ---- 逐文件归一化 ----
        let mut files: Vec<FileChange> = Vec::new();
        let mut symbols: Vec<SymbolChange> = Vec::new();
        let mut diagnostics: Vec<ChangeDiagnostic> = Vec::new();

        for git_file in &diff_files {
            let path = git_file
                .new_path
                .clone()
                .or_else(|| git_file.old_path.clone())
                .unwrap_or_default();
            let change_type = to_file_change_type(git_file.change_type);
            let file_change_id = FileChange::derive_id(&path);
            let changed = ChangedLines::from_hunks(&git_file.hunks);

            let before_read_path = git_file
                .old_path
                .clone()
                .or_else(|| git_file.new_path.clone())
                .unwrap_or_else(|| path.clone());
            let after_read_path = git_file
                .new_path
                .clone()
                .or_else(|| git_file.old_path.clone())
                .unwrap_or_else(|| path.clone());
            let before = before_model.get(&before_read_path);
            let after = after_model.get(&after_read_path);

            let before_state = side_state(before);
            let after_state = side_state(after);
            collect_side_diagnostics(Side::Before, &path, before_state, before, &mut diagnostics);
            collect_side_diagnostics(Side::After, &path, after_state, after, &mut diagnostics);

            let mut mapping_status = mapping_for_states(
                change_type,
                before_state,
                after_state,
                git_file.is_binary,
            );
            if git_file.is_binary {
                diagnostics.push(ChangeDiagnostic {
                    kind: ChangeDiagnosticKind::BinaryFile,
                    path: Some(path.clone()),
                    detail: "binary content; symbol mapping not applicable".to_string(),
                });
            }

            if !git_file.is_binary && (before.is_some() || after.is_some()) {
                {
                    let input = SymbolMatchInput {
                        file_change_id: file_change_id.clone(),
                        path: path.clone(),
                        file_change_type: change_type,
                        git_is_rename: git_file.is_rename,
                        before,
                        before_read_path: before_read_path.clone(),
                        after,
                        after_read_path: after_read_path.clone(),
                        changed: &changed,
                    };
                    let output = compute_symbol_changes(&input);
                    if output
                        .diagnostics
                        .iter()
                        .any(|d| d.kind == ChangeDiagnosticKind::SymbolMappingAmbiguous)
                    {
                        mapping_status = MappingStatus::Ambiguous;
                    }
                    symbols.extend(output.changes);
                    diagnostics.extend(output.diagnostics);
                }
            }

            files.push(FileChange {
                id: file_change_id.clone(),
                old_path: git_file.old_path.clone(),
                new_path: git_file.new_path.clone(),
                change_type,
                additions: git_file.additions,
                deletions: git_file.deletions,
                old_exists: git_file.change_type != GitChangeType::Added,
                new_exists: git_file.change_type != GitChangeType::Deleted,
                old_language: before.and_then(|model| model.language),
                new_language: after.and_then(|model| model.language),
                old_file_kind: before.map(|model| model.classification.kind),
                new_file_kind: after.map(|model| model.classification.kind),
                language_changed: FileChange::compute_language_changed(
                    before.and_then(|model| model.language),
                    after.and_then(|model| model.language),
                ),
                is_binary: git_file.is_binary,
                similarity: git_file.similarity,
                mapping_status,
                evidence: ChangeEvidence::new(
                    EvidenceSource::GitDiff,
                    format!(
                        "git reports {}{}",
                        change_type_label(change_type),
                        if git_file.is_binary { " (binary)" } else { "" }
                    ),
                )
                .with_file(file_change_id.clone())
                .with_ranges(changed.ranges.clone()),
            });
        }

        // ---- 模块聚合（复用 M3 FileModule 事实，§28–§31）----
        let mut modules = aggregate_modules(&files, &symbols, &before_model, &after_model);

        let summary = ChangeSummary {
            files_total: files.len() as u64,
            files_added: count_type(&files, FileChangeType::Added),
            files_modified: count_type(&files, FileChangeType::Modified),
            files_deleted: count_type(&files, FileChangeType::Deleted),
            files_renamed: count_type(&files, FileChangeType::Renamed),
            files_copied: count_type(&files, FileChangeType::Copied),
            files_type_changed: count_type(&files, FileChangeType::TypeChanged),
            files_other: count_type(&files, FileChangeType::Unmerged)
                + count_type(&files, FileChangeType::Unknown),
            symbols_total: symbols.len() as u64,
            symbols_added: count_symbol(&symbols, SymbolChangeType::Added),
            symbols_deleted: count_symbol(&symbols, SymbolChangeType::Deleted),
            symbols_modified: count_symbol(&symbols, SymbolChangeType::Modified),
            symbols_renamed: count_symbol(&symbols, SymbolChangeType::Renamed),
            symbols_moved: count_symbol(&symbols, SymbolChangeType::Moved),
            modules_changed: modules.len() as u64,
            files_with_ambiguous_symbols: diagnostics
                .iter()
                .filter(|d| d.kind == ChangeDiagnosticKind::SymbolMappingAmbiguous)
                .filter_map(|d| d.path.clone())
                .collect::<std::collections::BTreeSet<_>>()
                .len() as u64,
            test_files_changed: files
                .iter()
                .filter(|file| effective_kind(file) == FileKind::Test)
                .count() as u64,
            documentation_files_changed: files
                .iter()
                .filter(|file| effective_kind(file) == FileKind::Documentation)
                .count() as u64,
            diagnostics_total: diagnostics.len() as u64,
        };

        symbols.sort_by(|a, b| a.sort_key().cmp(&b.sort_key()));
        files.sort_by(|a, b| a.path().cmp(b.path()));
        modules.sort_by(|a, b| a.module_id.cmp(&b.module_id));
        diagnostics.sort_by(|a, b| diagnostic_sort_key(a).cmp(&diagnostic_sort_key(b)));

        Ok(Some(ChangeSet {
            schema_version: CHANGESET_SCHEMA_VERSION,
            id: ChangeSet::derive_id(Some(&repository_name), &base_id, &target_id, scope),
            source: ChangeSource::Git,
            scope,
            base: Some(base_id),
            target: Some(target_id),
            repository: Some(repository_name),
            files,
            symbols,
            modules,
            summary,
            diagnostics,
            truncated,
        }))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Side {
    Before,
    After,
}

fn untracked_as_added(path: String) -> GitDiffFile {
    GitDiffFile {
        old_path: None,
        new_path: Some(path),
        change_type: GitChangeType::Added,
        old_mode: None,
        new_mode: None,
        old_blob: None,
        new_blob: None,
        is_binary: false,
        is_rename: false,
        similarity: None,
        additions: None,
        deletions: None,
        hunks: Vec::new(),
    }
}

trait SortByPath {
    fn sort_by_path(&mut self);
}

impl SortByPath for Vec<GitDiffFile> {
    fn sort_by_path(&mut self) {
        self.sort_by(|a, b| display_path(a).cmp(display_path(b)));
    }
}

fn display_path(file: &GitDiffFile) -> &str {
    file.new_path
        .as_deref()
        .or(file.old_path.as_deref())
        .unwrap_or("")
}

fn to_file_change_type(change: GitChangeType) -> FileChangeType {
    match change {
        GitChangeType::Added => FileChangeType::Added,
        GitChangeType::Modified => FileChangeType::Modified,
        GitChangeType::Deleted => FileChangeType::Deleted,
        GitChangeType::Renamed => FileChangeType::Renamed,
        GitChangeType::Copied => FileChangeType::Copied,
        GitChangeType::TypeChanged => FileChangeType::TypeChanged,
        GitChangeType::Unmerged => FileChangeType::Unmerged,
        GitChangeType::Unknown => FileChangeType::Unknown,
    }
}

fn change_type_label(change: FileChangeType) -> &'static str {
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

fn count_type(files: &[FileChange], change_type: FileChangeType) -> u64 {
    files
        .iter()
        .filter(|file| file.change_type == change_type)
        .count() as u64
}

fn count_symbol(symbols: &[SymbolChange], change_type: SymbolChangeType) -> u64 {
    symbols
        .iter()
        .filter(|symbol| symbol.change_type == change_type)
        .count() as u64
}

/// 展示用文件种类（new 侧优先，Deleted 用 old 侧）。
fn effective_kind(file: &FileChange) -> FileKind {
    file.new_file_kind
        .or(file.old_file_kind)
        .unwrap_or(FileKind::Unknown)
}

fn side_state(model: Option<&FileModel>) -> SideParseState {
    let Some(model) = model else {
        return SideParseState::NotNeeded;
    };
    if model.classification.kind == FileKind::Generated {
        return SideParseState::Generated;
    }
    match &model.source {
        SourceState::Available(_) => {
            let Some(parse) = &model.parse else {
                return SideParseState::UnsupportedLanguage;
            };
            if crate::revision::parse_succeeded(parse.status) {
                SideParseState::Parsed
            } else if parse.status == prism_parser::ParseStatus::Failed {
                SideParseState::ParseFailed
            } else {
                SideParseState::UnsupportedLanguage
            }
        }
        SourceState::Absent => SideParseState::NotNeeded,
        SourceState::TooLarge { .. } => SideParseState::TooLarge,
        SourceState::EncodingUnsupported => SideParseState::EncodingUnsupported,
        SourceState::Unavailable(_) => SideParseState::ReadUnavailable,
    }
}

/// mapping_status 判定（§42/§44/§45）：
/// binary/generated/不支持语言 ⇒ NotApplicable；
/// 参与侧解析失败/超限/编码/读取失败 ⇒ Unmapped（诊断另记）；
/// 其余 Mapped。Added/Deleted 只看参与侧。
fn mapping_for_states(
    change_type: FileChangeType,
    before: SideParseState,
    after: SideParseState,
    is_binary: bool,
) -> MappingStatus {
    if is_binary {
        return MappingStatus::NotApplicable;
    }
    let mut status = MappingStatus::Mapped;
    let participants: [(bool, SideParseState); 2] = [
        (change_type != FileChangeType::Added, before),
        (change_type != FileChangeType::Deleted, after),
    ];
    for (participates, state) in participants {
        if !participates {
            continue;
        }
        match state {
            SideParseState::Generated | SideParseState::UnsupportedLanguage => {
                status = MappingStatus::NotApplicable;
            }
            SideParseState::ParseFailed
            | SideParseState::TooLarge
            | SideParseState::EncodingUnsupported
            | SideParseState::ReadUnavailable => {
                if status != MappingStatus::NotApplicable {
                    status = MappingStatus::Unmapped;
                }
            }
            SideParseState::Parsed | SideParseState::NotNeeded => {}
        }
    }
    status
}

/// 单侧状态 → 诊断（有内容但解析/读取失败的都要可解释，§33/§80）。
fn collect_side_diagnostics(
    side: Side,
    path: &str,
    state: SideParseState,
    model: Option<&FileModel>,
    diagnostics: &mut Vec<ChangeDiagnostic>,
) {
    let label = match side {
        Side::Before => "before",
        Side::After => "after",
    };
    let mut push = |kind: ChangeDiagnosticKind, detail: String| {
        diagnostics.push(ChangeDiagnostic {
            kind,
            path: Some(path.to_string()),
            detail,
        });
    };
    match state {
        SideParseState::Parsed | SideParseState::NotNeeded => {}
        SideParseState::ParseFailed => push(
            ChangeDiagnosticKind::ParseFailed,
            format!("{label} side failed to parse; symbol mapping skipped (not 'all deleted')"),
        ),
        SideParseState::UnsupportedLanguage => push(
            ChangeDiagnosticKind::UnsupportedLanguage,
            format!("{label} side has no parser for this language"),
        ),
        SideParseState::Generated => push(
            ChangeDiagnosticKind::GeneratedFile,
            format!("{label} side is classified generated; not symbol-mapped"),
        ),
        SideParseState::TooLarge => push(
            ChangeDiagnosticKind::FileTooLarge,
            format!("{label} side exceeds size limit"),
        ),
        SideParseState::EncodingUnsupported => push(
            ChangeDiagnosticKind::EncodingUnsupported,
            format!("{label} side is not valid UTF-8"),
        ),
        SideParseState::ReadUnavailable => {
            let kind = match side {
                Side::Before => ChangeDiagnosticKind::HistoricalModelUnavailable,
                Side::After => ChangeDiagnosticKind::GitQueryFailed,
            };
            let detail = model
                .and_then(|model| match &model.source {
                    SourceState::Unavailable(reason) => Some(reason.clone()),
                    _ => None,
                })
                .unwrap_or_default();
            push(kind, format!("{label} side content unavailable: {detail}"));
        }
    }
}

/// 一个待聚合模块条目。
#[derive(Default)]
struct ModuleAccumulator {
    change_type: Option<ModuleChangeType>,
    changed_files: Vec<String>,
    changed_symbols: u64,
    details: Vec<String>,
}

fn record_module(
    modules: &mut std::collections::BTreeMap<String, ModuleAccumulator>,
    module_id: &ModuleId,
    change_type: ModuleChangeType,
    path: &str,
    symbol_count: u64,
    detail: String,
) {
    let entry = modules.entry(module_id.as_str().to_string()).or_default();
    entry.change_type = Some(change_type);
    if !entry.changed_files.iter().any(|existing| existing == path) {
        entry.changed_files.push(path.to_string());
        entry.changed_files.sort();
    }
    entry.changed_symbols += symbol_count;
    if !entry.details.contains(&detail) {
        entry.details.push(detail);
    }
}

/// 模块聚合（§28–§31）：
/// - FileModule 生命周期由双侧模块事实决定（before 解析过 + 文件已删 ⇒ Deleted）
/// - rename ⇒ 旧 FileModule Deleted + 新 FileModule Added（模块事实，不假装同模块）
/// - namespace 容器：仅当该文件自身两侧都声明同一 namespace 时记 Modified
/// - before 基线缺失时不伪造模块生命周期
fn aggregate_modules(
    files: &[FileChange],
    symbols: &[SymbolChange],
    before: &SideModel,
    after: &SideModel,
) -> Vec<ModuleChange> {
    let mut modules: std::collections::BTreeMap<String, ModuleAccumulator> =
        std::collections::BTreeMap::new();

    let mut before_modules: std::collections::BTreeSet<String> = Default::default();
    for model in &before.files {
        if model.symbols_available() {
            before_modules.insert(file_module_id(&model.relative_path).as_str().to_string());
        }
    }

    for file in files {
        let symbol_count = symbols
            .iter()
            .filter(|symbol| symbol.file_change_id == file.id)
            .count() as u64;
        match file.change_type {
            FileChangeType::Added => {
                if let Some(new_path) = &file.new_path {
                    record_module(
                        &mut modules,
                        &file_module_id(new_path),
                        ModuleChangeType::Added,
                        new_path,
                        symbol_count,
                        format!("file added => file module created ({})", file.id),
                    );
                }
            }
            FileChangeType::Deleted => {
                if let Some(old_path) = &file.old_path {
                    let module_id = file_module_id(old_path);
                    if before_modules.contains(module_id.as_str()) {
                        record_module(
                            &mut modules,
                            &module_id,
                            ModuleChangeType::Deleted,
                            old_path,
                            symbol_count,
                            format!("file deleted and parsed before => file module removed ({})", file.id),
                        );
                    } else {
                        record_module(
                            &mut modules,
                            &module_id,
                            ModuleChangeType::Modified,
                            old_path,
                            0,
                            format!("file deleted; before module fact unavailable ({})", file.id),
                        );
                    }
                }
            }
            FileChangeType::Renamed => {
                if let (Some(old_path), Some(new_path)) = (&file.old_path, &file.new_path) {
                    record_module(
                        &mut modules,
                        &file_module_id(old_path),
                        ModuleChangeType::Deleted,
                        old_path,
                        0,
                        format!("file renamed away ({})", file.id),
                    );
                    record_module(
                        &mut modules,
                        &file_module_id(new_path),
                        ModuleChangeType::Added,
                        new_path,
                        symbol_count,
                        format!("file renamed in ({})", file.id),
                    );
                }
            }
            _ => {
                let path = file.path().to_string();
                record_module(
                    &mut modules,
                    &file_module_id(&path),
                    ModuleChangeType::Modified,
                    &path,
                    symbol_count,
                    format!("file changed ({})", file.id),
                );
                // namespace 容器（两侧都声明同一 namespace 才记，保守 §26/§30）。
                if let (Some(before_model), Some(after_model)) =
                    (before.get(&path), after.get(&path))
                {
                    for ns in shared_namespaces(before_model, after_model) {
                        record_module(
                            &mut modules,
                            &namespace_module_id(&ns),
                            ModuleChangeType::Modified,
                            &path,
                            0,
                            format!("contains changed file; namespace declared on both sides ({})", file.id),
                        );
                    }
                }
            }
        }
    }

    modules
        .into_iter()
        .map(|(module_key, accumulator)| ModuleChange {
            id: format!("mc:{module_key}"),
            module_id: ModuleId::from_string(module_key.clone()),
            module_name: module_key
                .rsplit("module:")
                .next()
                .unwrap_or(&module_key)
                .to_string(),
            module_kind: if module_key.starts_with("module:namespace:") {
                ModuleKind::Namespace
            } else if module_key.starts_with("module:package:") {
                ModuleKind::Package
            } else if module_key.starts_with("module:project:") {
                ModuleKind::Project
            } else {
                ModuleKind::FileModule
            },
            change_type: accumulator.change_type.unwrap_or(ModuleChangeType::Modified),
            changed_files: accumulator.changed_files,
            changed_symbol_count: accumulator.changed_symbols,
            mapping_status: MappingStatus::Mapped,
            evidence: ChangeEvidence::new(
                EvidenceSource::ModuleFacts,
                accumulator.details.join("; "),
            ),
        })
        .collect()
}

/// 两侧文件自身声明的、去路径前缀后相同的 namespace 集合。
fn shared_namespaces(before: &FileModel, after: &FileModel) -> Vec<String> {
    let local = |model: &FileModel| -> std::collections::BTreeSet<String> {
        let Some(parse) = &model.parse else {
            return Default::default();
        };
        let root_prefix = format!(
            "{}.",
            crate::matcher::qualified_root(&model.relative_path)
        );
        parse
            .symbols
            .iter()
            .filter(|symbol| symbol.kind == prism_parser::SymbolKind::Namespace)
            .filter_map(|symbol| symbol.qualified_name.strip_prefix(root_prefix.as_str()))
            .filter(|ns| !ns.is_empty())
            .map(|ns| ns.to_string())
            .collect()
    };
    local(before).intersection(&local(after)).cloned().collect()
}
