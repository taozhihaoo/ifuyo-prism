//! `prism changes` — 输出归一化 ChangeSet 而非原始 Git diff（规格 M5 §64–§66）。
//!
//! 退出码沿用 M0/M4 契约：6 = 非 Git 项目；3 = 无效 revision；
//! 4 = 取消；1 = 其他失败。

use prism_changes::{
    render_markdown, render_text, ChangeScope, ChangeSetOptions, ChangeSetService,
};
use prism_core::runtime::command::{CommandRunner, StdCommandRunner};
use prism_core::task::CancellationToken;
use std::path::Path;
use std::sync::Arc;

pub fn run(
    path: &Path,
    staged: bool,
    from: Option<&str>,
    to: Option<&str>,
    format: crate::cli::ChangesFormatArg,
    max_files: usize,
) -> i32 {
    // 路径不存在 = 输入无效（与 scan/inspect 契约一致）。
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
    let options = ChangeSetOptions {
        max_files,
        ..ChangeSetOptions::default()
    };

    let service = ChangeSetService::new(runner);
    match service.build(path, scope, from, to, &options, &cancel) {
        Ok(Some(changeset)) => {
            match format {
                crate::cli::ChangesFormatArg::Json => {
                    match serde_json::to_string_pretty(&changeset) {
                        Ok(text) => println!("{text}"),
                        Err(err) => {
                            eprintln!("prism: failed to serialize changeset: {err}");
                            return crate::EXIT_ERROR;
                        }
                    }
                }
                crate::cli::ChangesFormatArg::Text => print!("{}", render_text(&changeset)),
                crate::cli::ChangesFormatArg::Markdown => print!("{}", render_markdown(&changeset)),
            }
            crate::EXIT_OK
        }
        // 规格 §25/§39：明确请求变更分析但非 Git 项目 → 专用退出码。
        Ok(None) => {
            eprintln!("not a git repository");
            crate::EXIT_NOT_GIT
        }
        Err(prism_core::PrismError::Cancelled) => {
            eprintln!("prism: changes cancelled");
            crate::EXIT_CANCELLED
        }
        Err(prism_core::PrismError::InvalidInput { message }) => {
            eprintln!("prism: {message}");
            crate::EXIT_INVALID_INPUT
        }
        Err(err) => {
            eprintln!("prism: changes failed: {err}");
            crate::EXIT_ERROR
        }
    }
}
