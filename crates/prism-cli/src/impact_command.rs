//! `prism impact` — 输出 ImpactAnalysis（规格 M6 §87–§90）。
//!
//! 语义：当前 ChangeSet（默认 HEAD→worktree，--staged/--from/--to 对齐 M5）
//! × 项目代码模型 × M3 图 ⇒ 潜在影响 + 证据 + 局限。
//! 退出码沿用契约：3 输入无效 / 4 取消 / 6 非 Git；"valid but truncated"
//! 仍为 0（§90：truncated ≠ failed）。

use prism_analysis::{CodeAnalysisOptions, CodeAnalysisService};
use prism_changes::{ChangeScope, ChangeSetOptions, ChangeSetService};
use prism_core::runtime::command::{CommandRunner, StdCommandRunner};
use prism_core::task::CancellationToken;
use prism_graph::DependencyService;
use prism_impact::{
    render_markdown, render_text, ImpactAnalysisService, ImpactOptions,
};
use std::path::Path;
use std::sync::Arc;

#[allow(clippy::too_many_arguments)] // CLI 参数直传；结构体化只会转移同样的数量
pub fn run(
    path: &Path,
    staged: bool,
    from: Option<&str>,
    to: Option<&str>,
    format: crate::cli::ChangesFormatArg,
    max_files: usize,
    max_depth: u32,
    max_nodes: u64,
    include_tests: bool,
    include_docs: bool,
) -> i32 {
    // 路径不存在 = 输入无效（与 scan/inspect/changes 契约一致）。
    if !path.exists() {
        eprintln!("prism: path does not exist: {}", path.to_string_lossy());
        return crate::EXIT_INVALID_INPUT;
    }
    let runner: Arc<dyn CommandRunner> = Arc::new(StdCommandRunner::new());
    let cancel = CancellationToken::new();
    let scope = if staged {
        ChangeScope::Staged
    } else if from.is_some() || to.is_some() {
        ChangeScope::RevisionRange
    } else {
        ChangeScope::WorkingTree
    };

    // ---- M5 ChangeSet（非 Git ⇒ exit 6，与 prism changes 一致）----
    let changeset = match ChangeSetService::new(runner.clone()).build(
        path,
        scope,
        from,
        to,
        &ChangeSetOptions { max_files, ..ChangeSetOptions::default() },
        &cancel,
    ) {
        Ok(Some(changeset)) => changeset,
        Ok(None) => {
            eprintln!("not a git repository");
            return crate::EXIT_NOT_GIT;
        }
        Err(prism_core::PrismError::Cancelled) => {
            eprintln!("prism: impact cancelled");
            return crate::EXIT_CANCELLED;
        }
        Err(prism_core::PrismError::InvalidInput { message }) => {
            eprintln!("prism: {message}");
            return crate::EXIT_INVALID_INPUT;
        }
        Err(err) => {
            eprintln!("prism: impact failed building changeset: {err}");
            return crate::EXIT_ERROR;
        }
    };

    // ---- 当前代码模型 + M3 图（§138：impact 消费当前关系事实）----
    let (model, graph) = match build_facts(path, &cancel) {
        Ok(facts) => facts,
        Err(prism_core::PrismError::Cancelled) => {
            eprintln!("prism: impact cancelled");
            return crate::EXIT_CANCELLED;
        }
        Err(err) => {
            eprintln!("prism: impact failed building project facts: {err}");
            return crate::EXIT_ERROR;
        }
    };

    let options = ImpactOptions {
        max_depth,
        max_nodes,
        include_tests,
        include_docs,
        ..ImpactOptions::default()
    };
    match ImpactAnalysisService.analyze(&changeset, &model, &graph, &options, &cancel) {
        Ok(analysis) => {
            match format {
                crate::cli::ChangesFormatArg::Json => {
                    match serde_json::to_string_pretty(&analysis) {
                        Ok(text) => println!("{text}"),
                        Err(err) => {
                            eprintln!("prism: failed to serialize impact analysis: {err}");
                            return crate::EXIT_ERROR;
                        }
                    }
                }
                crate::cli::ChangesFormatArg::Text => print!("{}", render_text(&analysis)),
                crate::cli::ChangesFormatArg::Markdown => {
                    print!("{}", render_markdown(&analysis))
                }
            }
            // §90：valid but truncated 仍是成功（truncated 已在输出中如实标注）。
            crate::EXIT_OK
        }
        Err(prism_core::PrismError::Cancelled) => {
            eprintln!("prism: impact cancelled");
            crate::EXIT_CANCELLED
        }
        Err(err) => {
            eprintln!("prism: impact failed: {err}");
            crate::EXIT_ERROR
        }
    }
}

/// discovery → 代码模型 → 依赖图（与 prism graph 同一管线）。
fn build_facts(
    path: &Path,
    cancel: &CancellationToken,
) -> prism_core::PrismResult<(
    prism_analysis::ProjectCodeModel,
    prism_graph::DependencyGraphResult,
)> {
    let discovery = prism_project::DiscoveryService::new().discover(
        path,
        prism_project::ScanOptions::default(),
        cancel,
        |_| {},
    )?;
    let model = CodeAnalysisService::new().analyze(
        &discovery,
        CodeAnalysisOptions::default(),
        cancel,
        |_| {},
    )?;
    let graph = DependencyService::new().analyze(&discovery, &model)?;
    Ok((model, graph))
}
