//! RevisionSource：任一侧（before/after）源内容抽象（规格 M5 §76）。
//!
//! ```text
//! RevisionSource → per-file content → M2 parser → FileModel
//! ```
//!
//! - [`CurrentWorkingTreeSource`]：直接读磁盘（read-only）
//! - [`GitRevisionSource`]：`git cat-file`（HEAD / `:0` index / 任意 revision），
//!   绝不改写工作区/index/HEAD（§38/§77）
//!
//! 内容状态显式建模（§42/§78）：缺失、超限、非 UTF-8 都不是"空文件"。

use prism_core::task::CancellationToken;
use prism_core::{PrismError, PrismResult};
use prism_git::{CliGitProvider, GitService, RevisionBlob};
use prism_parser::{ParseRequest, ParseResult, ParserRegistry};
use prism_project::{FileClassifier, FileKind};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// 一侧某文件的源内容状态。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceState {
    /// 文本内容可用。
    Available(String),
    /// 该侧不存在此路径（Added 的 before / Deleted 的 after）——不是失败。
    Absent,
    /// 超过字节上限（未读取内容，§78）。
    TooLarge { size_bytes: u64 },
    /// 非 UTF-8：不假装解析（§42 关键正确性要求）。
    EncodingUnsupported,
    /// 读取失败（IO/git 错误；detail 不含源码内容）。
    Unavailable(String),
}

/// 一个变更文件的归一化事实（源 + 分类 + 解析结果）。
#[derive(Debug, Clone)]
pub struct FileModel {
    pub relative_path: String,
    pub language: Option<prism_project::Language>,
    pub classification: prism_project::FileClassification,
    pub source: SourceState,
    /// 解析结果（内容可用且语言受支持时才存在；status=Failed 表示解析失败）。
    pub parse: Option<ParseResult>,
}

impl FileModel {
    /// 该侧符号事实是否可用（解析成功或带诊断成功；Failed/缺失都不可用）。
    pub fn symbols_available(&self) -> bool {
        self.parse
            .as_ref()
            .is_some_and(|result| parse_succeeded(result.status))
    }
}

/// 解析出了符号事实（含带诊断的部分成功）。
pub fn parse_succeeded(status: prism_parser::ParseStatus) -> bool {
    matches!(
        status,
        prism_parser::ParseStatus::Successful
            | prism_parser::ParseStatus::SuccessfulWithDiagnostics
    )
}

/// 一侧（before/after）全部变更文件的模型（按 relative_path 排序）。
#[derive(Debug, Clone, Default)]
pub struct SideModel {
    pub files: Vec<FileModel>,
}

impl SideModel {
    pub fn get(&self, relative_path: &str) -> Option<&FileModel> {
        self.files
            .iter()
            .find(|file| file.relative_path == relative_path)
    }
}

/// 一侧的源抽象（规格 M5 §76）。
pub trait RevisionSource: Send + Sync {
    fn label(&self) -> &'static str;
    fn read(
        &self,
        relative_path: &str,
        max_bytes: u64,
        cancel: &CancellationToken,
    ) -> PrismResult<SourceState>;
}

/// 工作区侧：直接读磁盘（只读）。
pub struct CurrentWorkingTreeSource {
    root: PathBuf,
}

impl CurrentWorkingTreeSource {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        CurrentWorkingTreeSource { root: root.into() }
    }
}

impl RevisionSource for CurrentWorkingTreeSource {
    fn label(&self) -> &'static str {
        "working_tree"
    }

    fn read(
        &self,
        relative_path: &str,
        max_bytes: u64,
        _cancel: &CancellationToken,
    ) -> PrismResult<SourceState> {
        let path = self.root.join(relative_path);
        let metadata = std::fs::metadata(&path).map_err(|err| PrismError::Io {
            message: err.to_string(),
        })?;
        if metadata.len() > max_bytes {
            return Ok(SourceState::TooLarge {
                size_bytes: metadata.len(),
            });
        }
        match std::fs::read(&path) {
            Ok(bytes) => Ok(match String::from_utf8(bytes) {
                Ok(content) => SourceState::Available(content),
                Err(_) => SourceState::EncodingUnsupported,
            }),
            Err(err) => Err(PrismError::Io {
                message: err.to_string(),
            }),
        }
    }
}

/// Git revision 侧：`git cat-file`（HEAD、`:0` = index stage 0、任意 SHA）。
/// 只读；经 M0 CommandRunner。
pub struct GitRevisionSource {
    service: GitService,
    root: PathBuf,
    revision: String,
}

impl GitRevisionSource {
    /// `revision` 为任意 rev 表达式；index 侧传 ":0"。
    pub fn new(
        runner: Arc<dyn prism_core::CommandRunner>,
        root: impl Into<PathBuf>,
        revision: impl Into<String>,
    ) -> Self {
        GitRevisionSource {
            service: GitService::with_provider(Arc::new(CliGitProvider::new(runner))),
            root: root.into(),
            revision: revision.into(),
        }
    }
}

impl RevisionSource for GitRevisionSource {
    fn label(&self) -> &'static str {
        "git_revision"
    }

    fn read(
        &self,
        relative_path: &str,
        max_bytes: u64,
        cancel: &CancellationToken,
    ) -> PrismResult<SourceState> {
        match self
            .service
            .read_blob(&self.root, &self.revision, relative_path, max_bytes, cancel)
        {
            Ok(RevisionBlob::Text { content }) => Ok(SourceState::Available(content)),
            Ok(RevisionBlob::Missing) => Ok(SourceState::Absent),
            Ok(RevisionBlob::TooLarge { size_bytes }) => Ok(SourceState::TooLarge { size_bytes }),
            Ok(RevisionBlob::EncodingUnsupported) => Ok(SourceState::EncodingUnsupported),
            Err(err) => Err(PrismError::Git {
                message: err.to_string(),
            }),
        }
    }
}

/// 把一侧的路径集合归一化为 FileModel（bounded 并发 + 取消 + 确定性，§71/§72）。
///
/// paths 为该侧应读取的项目相对路径（rename 时 before 侧读 old_path、
/// after 侧读 new_path）。输出按 relative_path 排序，与并发顺序无关。
pub fn build_side_model(
    paths: &[String],
    source: &dyn RevisionSource,
    max_bytes: u64,
    max_workers: usize,
    cancel: &CancellationToken,
) -> PrismResult<SideModel> {
    let mut models: Vec<FileModel> = Vec::with_capacity(paths.len());

    let workers = max_workers.max(1).min(paths.len().max(1));
    let mut chunks: Vec<Vec<String>> = vec![Vec::new(); workers];
    for (index, path) in paths.iter().enumerate() {
        chunks[index % workers].push(path.clone());
    }

    let worker_results: Vec<Vec<FileModel>> = std::thread::scope(|scope| {
        let handles: Vec<_> = chunks
            .into_iter()
            .map(|chunk| {
                scope.spawn(move || {
                    let registry = ParserRegistry::with_defaults();
                    let classifier = FileClassifier::new();
                    let mut results = Vec::new();
                    for path in chunk {
                        if cancel.is_cancelled() {
                            break;
                        }
                        results.push(build_file_model(
                            &classifier,
                            &registry,
                            &path,
                            source,
                            max_bytes,
                            cancel,
                        ));
                    }
                    results
                })
            })
            .collect();
        handles
            .into_iter()
            .filter_map(|handle| handle.join().ok())
            .collect::<Vec<_>>()
    });

    for chunk in worker_results {
        models.extend(chunk);
    }
    if cancel.is_cancelled() {
        return Err(PrismError::Cancelled);
    }
    models.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
    Ok(SideModel { files: models })
}

/// 单文件：分类（M1 语义）→ 读源 → 解析（M2 管线）。任何单文件失败非致命。
pub fn build_file_model(
    classifier: &FileClassifier,
    registry: &ParserRegistry,
    relative_path: &str,
    source: &dyn RevisionSource,
    max_bytes: u64,
    cancel: &CancellationToken,
) -> FileModel {
    let file_name = Path::new(relative_path)
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| relative_path.to_string());
    let (classification, _analyzable, _reason) =
        classifier.assess(relative_path, &file_name, false);
    let language = classifier.detect_language(&file_name);

    let content = match source.read(relative_path, max_bytes, cancel) {
        Ok(state) => state,
        Err(err) => SourceState::Unavailable(err.to_string()),
    };

    let parse = match (&content, language) {
        (SourceState::Available(text), Some(language)) if registry.is_supported(language) => {
            let request = ParseRequest {
                file_id: prism_core::ids::FileId::from_string(format!("file:{relative_path}")),
                relative_path: relative_path.to_string(),
                language,
                content: text.clone(),
            };
            match registry.get(language) {
                Some(parser) => parser.parse(&request, cancel).ok(),
                None => None,
            }
        }
        _ => None,
    };

    FileModel {
        relative_path: relative_path.to_string(),
        language,
        classification,
        source: content,
        parse,
    }
}

/// 便捷判定：FileKind 是否值得尝试符号解析（Source/Test；Generated 记事实不解析）。
pub fn is_symbol_capable_kind(kind: FileKind) -> bool {
    matches!(kind, FileKind::Source | FileKind::Test)
}

#[cfg(test)]
mod tests {
    use super::*;
    use prism_parser::ParseStatus;
    use std::path::PathBuf;

    /// 测试用临时目录守卫（Drop 清理）。
    struct TempDirGuard(PathBuf);

    impl TempDirGuard {
        fn new(name: &str) -> Self {
            let root = std::env::temp_dir().join(format!(
                "prism-changes-rev-{name}-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.subsec_nanos())
                    .unwrap_or(0)
            ));
            let _ = std::fs::remove_dir_all(&root);
            std::fs::create_dir_all(&root).unwrap();
            TempDirGuard(root)
        }
    }

    impl Drop for TempDirGuard {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn workspace_with(content: &str) -> (TempDirGuard, CurrentWorkingTreeSource) {
        let guard = TempDirGuard::new("wt");
        std::fs::write(guard.0.join("a.ts"), content).unwrap();
        let source = CurrentWorkingTreeSource::new(&guard.0);
        (guard, source)
    }

    #[test]
    fn working_tree_source_reads_and_reports_states() {
        let (_guard, source) = workspace_with("export const a = 1;\n");
        let cancel = CancellationToken::new();
        assert!(matches!(
            source.read("a.ts", 1024, &cancel).unwrap(),
            SourceState::Available(_)
        ));
        assert!(source.read("missing.ts", 1024, &cancel).is_err());
        assert!(matches!(
            source.read("a.ts", 2, &cancel).unwrap(),
            SourceState::TooLarge { .. }
        ));
    }

    #[test]
    fn non_utf8_reports_encoding_unsupported_not_empty_text() {
        let guard = TempDirGuard::new("latin1");
        std::fs::write(guard.0.join("data.cs"), [0x43, 0xE9, 0x6C, 0x69]).unwrap();
        let source = CurrentWorkingTreeSource::new(&guard.0);
        let cancel = CancellationToken::new();
        assert_eq!(
            source.read("data.cs", 1024, &cancel).unwrap(),
            SourceState::EncodingUnsupported
        );
    }

    #[test]
    fn file_model_parses_supported_language_and_flags_unsupported() {
        let (_guard, source) = workspace_with("export function foo() {\n  return 1;\n}\n");
        let cancel = CancellationToken::new();
        let classifier = FileClassifier::new();
        let registry = ParserRegistry::with_defaults();
        let model = build_file_model(&classifier, &registry, "a.ts", &source, 1024, &cancel);
        assert_eq!(model.language, Some(prism_project::Language::TypeScript));
        assert_eq!(model.classification.kind, FileKind::Source);
        assert!(model.symbols_available());
        let parse = model.parse.unwrap();
        assert_eq!(parse.status, ParseStatus::Successful);
        assert!(parse.symbols.iter().any(|symbol| symbol.name == "foo"));
    }

    #[test]
    fn side_model_is_sorted_and_deterministic() {
        let guard = TempDirGuard::new("side");
        std::fs::write(guard.0.join("b.ts"), "export const b = 2;\n").unwrap();
        std::fs::write(guard.0.join("a.ts"), "export const a = 1;\n").unwrap();
        let source = CurrentWorkingTreeSource::new(&guard.0);
        let cancel = CancellationToken::new();
        let side = build_side_model(
            &["b.ts".to_string(), "a.ts".to_string()],
            &source,
            1024,
            4,
            &cancel,
        )
        .unwrap();
        let paths: Vec<&str> = side
            .files
            .iter()
            .map(|model| model.relative_path.as_str())
            .collect();
        assert_eq!(paths, vec!["a.ts", "b.ts"]);
    }
}
