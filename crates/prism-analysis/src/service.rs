//! CodeAnalysisService：源文件选择 → 有界并发解析 → ProjectCodeModel。
//!
//! 生命周期（规格 §28/§30/§31）：
//!
//! ```text
//! ProjectDiscovery → 选择 Source/Test&&analyzable 文件（Generated 策略性跳过；
//! M6 起纳入 Test 文件——测试是影响分析的一等事实，M1 已将其单独分类）
//!   → N 个 worker（各自 ParserRegistry）→ read once → parse once → index
//! ```
//!
//! - 取消：每个文件前检查 token；取消后保留已完成部分，status = Cancelled
//! - 单文件失败非致命（读文件/编码/解析均落 diagnostics）
//! - 确定性：结果按 relative_path 排序，与并发顺序无关

use crate::model::{ParseSummary, ProjectCodeModel, CODE_MODEL_SCHEMA_VERSION};
use prism_core::task::{CancellationToken, TaskState};
use prism_core::PrismResult;
use prism_parser::{ParseDiagnostic, ParseRequest, ParseResult, ParseStatus, ParserRegistry};
use prism_project::{DiscoveryService, FileInfo, FileKind, ProjectDiscovery};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

/// 分析选项。max_workers 有界并发（规格 §30，不无限 spawn）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CodeAnalysisOptions {
    pub max_workers: usize,
}

impl Default for CodeAnalysisOptions {
    fn default() -> Self {
        let available = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1);
        CodeAnalysisOptions {
            max_workers: available.clamp(1, 4),
        }
    }
}

/// 实时进度（真实计数，不伪造百分比）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CodeAnalysisProgress {
    pub files_total: u64,
    pub files_done: u64,
    pub symbols: u64,
    pub errors: u64,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct CodeAnalysisService;

impl CodeAnalysisService {
    pub fn new() -> Self {
        CodeAnalysisService
    }

    /// 便捷入口：先 discovery 再 analyze（CLI 单命令场景）。
    pub fn inspect(
        &self,
        root: &std::path::Path,
        options: CodeAnalysisOptions,
        cancel: &CancellationToken,
        on_progress: impl Fn(CodeAnalysisProgress) + Send + Sync + 'static,
    ) -> PrismResult<ProjectCodeModel> {
        let discovery = DiscoveryService::new().discover(
            root,
            prism_project::ScanOptions::default(),
            cancel,
            |_| {},
        )?;
        self.analyze(&discovery, options, cancel, on_progress)
    }

    /// 在既有 Discovery 结果上执行代码分析（GUI 复用扫描结果的入口）。
    pub fn analyze(
        &self,
        discovery: &ProjectDiscovery,
        options: CodeAnalysisOptions,
        cancel: &CancellationToken,
        on_progress: impl Fn(CodeAnalysisProgress) + Send + Sync + 'static,
    ) -> PrismResult<ProjectCodeModel> {
        let started = Instant::now();
        let registry = Arc::new(ParserRegistry::with_defaults());

        // ---- 选择阶段 ----
        let mut pipeline: Vec<FileInfo> = Vec::new();
        let mut placeholders: Vec<ParseResult> = Vec::new();

        for file in &discovery.files {
            if file.is_symlink {
                continue;
            }
            if file.classification.kind == FileKind::Generated && file.analyzable {
                placeholders.push(Self::placeholder(file, ParseStatus::Skipped, "generated"));
                continue;
            }
            // M6：Test 文件进入分析管线（影响分析需要 test → production 关系）。
            // M1 已保证 Test 与 Source 分类互斥且证据充分。
            if file.classification.kind != FileKind::Source
                && file.classification.kind != FileKind::Test
            {
                continue;
            }
            if !file.analyzable {
                let reason = file
                    .non_analyzable_reason
                    .clone()
                    .unwrap_or_else(|| "not_analyzable".to_string());
                placeholders.push(Self::placeholder(file, ParseStatus::Skipped, &reason));
                continue;
            }
            let Some(language) = file.language else {
                continue;
            };
            if !registry.is_supported(language) {
                placeholders.push(Self::placeholder(
                    file,
                    ParseStatus::Unsupported,
                    "no_parser",
                ));
                continue;
            }
            pipeline.push(file.clone());
        }

        pipeline.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
        let files_total = (pipeline.len() + placeholders.len()) as u64;

        let root = discovery.project.root_path.clone();
        let counters = Arc::new(ProgressCounters::new(files_total));
        let cancelled = Arc::new(AtomicBool::new(false));
        let progress_cb = Arc::new(on_progress);
        let counters_for_workers = counters.clone();
        let cancelled_for_workers = cancelled.clone();
        let cancel_token = cancel.clone();

        // ---- 有界并发解析 ----
        let workers = options.max_workers.max(1).min(pipeline.len().max(1));
        let mut chunks: Vec<Vec<FileInfo>> = vec![Vec::new(); workers];
        for (index, file) in pipeline.iter().enumerate() {
            chunks[index % workers].push(file.clone());
        }

        let mut worker_results: Vec<Vec<ParseResult>> = std::thread::scope(|scope| {
            let handles: Vec<_> = chunks
                .into_iter()
                .map(|chunk| {
                    let counters = counters_for_workers.clone();
                    let cancelled = cancelled_for_workers.clone();
                    let cancel = cancel_token.clone();
                    let progress_cb = progress_cb.clone();
                    let root = root.clone();
                    scope.spawn(move || {
                        let worker_registry = ParserRegistry::with_defaults();
                        let mut results = Vec::new();
                        for file in chunk {
                            if cancel.is_cancelled() {
                                cancelled.store(true, Ordering::SeqCst);
                                break;
                            }
                            let result = Self::parse_one(&worker_registry, &root, &file, &cancel);
                            counters.files_done.fetch_add(1, Ordering::SeqCst);
                            counters
                                .symbols
                                .fetch_add(result.symbols.len() as u64, Ordering::SeqCst);
                            if result.has_errors() {
                                counters.errors.fetch_add(1, Ordering::SeqCst);
                            }
                            results.push(result);
                            progress_cb(counters.snapshot());
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

        let mut parse_results: Vec<ParseResult> = worker_results
            .drain(..)
            .reduce(|mut acc, mut chunk| {
                acc.append(&mut chunk);
                acc
            })
            .unwrap_or_default();
        parse_results.extend(placeholders);
        parse_results.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));

        let mut files: Vec<FileInfo> = parse_results
            .iter()
            .filter_map(|result| {
                discovery
                    .files
                    .iter()
                    .find(|file| file.id == result.file_id)
                    .cloned()
            })
            .collect();
        files.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));

        let progress = counters.snapshot();
        let summary = ParseSummary {
            files_total,
            files_parsed: parse_results
                .iter()
                .filter(|r| r.status == ParseStatus::Successful)
                .count() as u64,
            files_with_diagnostics: parse_results
                .iter()
                .filter(|r| r.status == ParseStatus::SuccessfulWithDiagnostics)
                .count() as u64,
            files_failed: parse_results
                .iter()
                .filter(|r| r.status == ParseStatus::Failed)
                .count() as u64,
            files_unsupported: parse_results
                .iter()
                .filter(|r| r.status == ParseStatus::Unsupported)
                .count() as u64,
            files_skipped: parse_results
                .iter()
                .filter(|r| r.status == ParseStatus::Skipped)
                .count() as u64,
            symbols_total: parse_results.iter().map(|r| r.symbols.len() as u64).sum(),
            error_count: parse_results
                .iter()
                .map(|r| {
                    r.diagnostics
                        .iter()
                        .filter(|d| d.severity == prism_parser::ParseDiagnosticSeverity::Error)
                        .count() as u64
                })
                .sum(),
            warning_count: parse_results
                .iter()
                .map(|r| {
                    r.diagnostics
                        .iter()
                        .filter(|d| d.severity == prism_parser::ParseDiagnosticSeverity::Warning)
                        .count() as u64
                })
                .sum(),
            duration_ms: started.elapsed().as_millis() as u64,
            status: if cancelled_for_workers.load(Ordering::SeqCst) {
                TaskState::Cancelled
            } else {
                TaskState::Completed
            },
        };

        let _ = progress; // 进度经回调暴露；保留计数器以防未来聚合

        Ok(ProjectCodeModel {
            schema_version: CODE_MODEL_SCHEMA_VERSION,
            project: discovery.project.clone(),
            files,
            parse_results,
            summary,
        })
    }

    /// 单文件：按需读取（read once）→ registry 分派 → 解析。
    /// 读文件失败为非致命 Failed（编码不支持 → unsupported_encoding，规格 §45）。
    fn parse_one(
        registry: &ParserRegistry,
        root: &std::path::Path,
        file: &FileInfo,
        cancel: &CancellationToken,
    ) -> ParseResult {
        let language = match file.language {
            Some(language) => language,
            None => {
                return Self::failed_result(file, "no_language", "file has no recognized language");
            }
        };
        let Some(parser) = registry.get(language) else {
            return Self::placeholder(file, ParseStatus::Unsupported, "no_parser");
        };

        let absolute = root.join(&file.relative_path);
        let content = match std::fs::read_to_string(&absolute) {
            Ok(content) => content,
            Err(err) => {
                let code = if err.kind() == std::io::ErrorKind::InvalidData {
                    "unsupported_encoding"
                } else {
                    "read_failed"
                };
                return Self::failed_result(file, code, &format!("failed to read file: {err}"));
            }
        };

        let request = ParseRequest {
            file_id: file.id.clone(),
            relative_path: file.relative_path.clone(),
            language,
            content,
        };
        match parser.parse(&request, cancel) {
            Ok(result) => result,
            // 解析中途取消：非致命，也不计入 failed。
            Err(prism_core::PrismError::Cancelled) => {
                Self::placeholder(file, ParseStatus::Skipped, "cancelled")
            }
            Err(err) => Self::failed_result(file, "parser_error", &err.to_string()),
        }
    }

    fn placeholder(file: &FileInfo, status: ParseStatus, reason: &str) -> ParseResult {
        ParseResult {
            file_id: file.id.clone(),
            relative_path: file.relative_path.clone(),
            language: file.language,
            status,
            symbols: vec![],
            diagnostics: vec![],
            duration_ms: 0,
            parser_version: "n/a".to_string(),
            reason: Some(reason.to_string()),
            imports: Vec::new(),
            calls: Vec::new(),
        }
    }

    fn failed_result(file: &FileInfo, code: &str, message: &str) -> ParseResult {
        ParseResult {
            file_id: file.id.clone(),
            relative_path: file.relative_path.clone(),
            language: file.language,
            status: ParseStatus::Failed,
            symbols: vec![],
            diagnostics: vec![ParseDiagnostic::error(code, message, None)],
            duration_ms: 0,
            parser_version: "n/a".to_string(),
            reason: None,
            imports: Vec::new(),
            calls: Vec::new(),
        }
    }
}

struct ProgressCounters {
    files_total: AtomicU64,
    files_done: AtomicU64,
    symbols: AtomicU64,
    errors: AtomicU64,
}

impl ProgressCounters {
    fn new(files_total: u64) -> Self {
        ProgressCounters {
            files_total: AtomicU64::new(files_total),
            files_done: AtomicU64::new(0),
            symbols: AtomicU64::new(0),
            errors: AtomicU64::new(0),
        }
    }

    fn snapshot(&self) -> CodeAnalysisProgress {
        CodeAnalysisProgress {
            files_total: self.files_total.load(Ordering::SeqCst),
            files_done: self.files_done.load(Ordering::SeqCst),
            symbols: self.symbols.load(Ordering::SeqCst),
            errors: self.errors.load(Ordering::SeqCst),
        }
    }
}
