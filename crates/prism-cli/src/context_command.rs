//! `prism context <path>` — 确定性 Context 生成基础命令（规格 M10a §283/§347/§348）。
//!
//! stdout 只输出数据（§347）；M1–M6 事实由本命令现场构建（无 session 时
//! M7–M9 事实缺省为 None——collectors 如实降级，§260）；GUI/CLI 同源
//! ContextService（§475）。

use prism_analysis::{CodeAnalysisOptions, CodeAnalysisService};
use prism_changes::{ChangeScope, ChangeSetOptions, ChangeSetService};
use prism_context::model::{ContextBudget, ContextRequest, ContextScope};
use prism_core::runtime::command::{CommandRunner, StdCommandRunner};
use prism_core::task::CancellationToken;
use prism_graph::DependencyService;
use std::path::Path;
use std::sync::Arc;

#[allow(clippy::too_many_arguments)]
pub fn run(
    path: &Path,
    task: Option<String>,
    json: bool,
    scope: Option<String>,
    target: Option<String>,
    max_tokens: Option<u64>,
) -> i32 {
    if !path.exists() {
        eprintln!("prism: path does not exist: {}", path.to_string_lossy());
        return crate::EXIT_INVALID_INPUT;
    }
    let runner: Arc<dyn CommandRunner> = Arc::new(StdCommandRunner::new());
    let cancel = CancellationToken::new();

    // ---- M1/M2/M3 现场事实（§263：调用方负责提供；CLI 无 session）----
    let discovery = match prism_project::DiscoveryService::new().discover(
        path,
        prism_project::ScanOptions::default(),
        &cancel,
        |_| {},
    ) {
        Ok(discovery) => discovery,
        Err(err) => {
            eprintln!("prism: context failed building discovery: {err}");
            return crate::EXIT_ERROR;
        }
    };
    let code_model = match CodeAnalysisService::new().analyze(
        &discovery,
        CodeAnalysisOptions::default(),
        &cancel,
        |_| {},
    ) {
        Ok(model) => model,
        Err(err) => {
            eprintln!("prism: context failed building code model: {err}");
            return crate::EXIT_ERROR;
        }
    };
    let graph = DependencyService::new().analyze(&discovery, &code_model).ok();
    // M5/M6（git 仓库才有；失败 ⇒ None，collectors 降级 §260）。
    let changeset = ChangeSetService::new(runner.clone())
        .build(
            path,
            ChangeScope::WorkingTree,
            None,
            None,
            &ChangeSetOptions::default(),
            &cancel,
        )
        .ok()
        .flatten();
    let impact = changeset
        .as_ref()
        .zip(graph.as_ref())
        .and_then(|(changeset, graph)| {
            prism_impact::ImpactAnalysisService
                .analyze(
                    changeset,
                    &code_model,
                    graph,
                    &prism_impact::ImpactOptions::default(),
                    &cancel,
                )
                .ok()
        });

    let parsed_scope = match scope.as_deref() {
        None => ContextScope::Auto,
        Some("auto") => ContextScope::Auto,
        Some("current_change") | Some("current-change") => ContextScope::CurrentChange,
        Some("target") => ContextScope::Target,
        Some("full") => ContextScope::Full,
        Some(other) => {
            eprintln!("prism: unknown scope '{other}' (auto|current_change|target|full)");
            return crate::EXIT_INVALID_INPUT;
        }
    };

    let mut budget = ContextBudget::default();
    if let Some(max_tokens) = max_tokens {
        budget.max_tokens_estimate = max_tokens;
    }

    let request = ContextRequest {
        project_root: path.to_string_lossy().to_string(),
        task,
        scope: parsed_scope,
        target,
        budget,
    };

    let pack = match prism_context::ContextService::generate(
        request,
        Some(&discovery),
        Some(&code_model),
        changeset.as_ref(),
        impact.as_ref(),
        graph.as_ref(),
        None, // M7/M8/M9 session 事实 CLI 未持有 ⇒ None（collectors 降级）
        None,
        None,
        &cancel,
    ) {
        Ok(pack) => pack,
        Err(prism_core::PrismError::InvalidInput { message }) => {
            // §200：零预算等请求错误 ⇒ exit 3。
            eprintln!("prism: {message}");
            return crate::EXIT_INVALID_INPUT;
        }
        Err(err) => {
            eprintln!("prism: context generation failed: {err}");
            return crate::EXIT_ERROR;
        }
    };

    if json {
        match serde_json::to_string_pretty(&pack) {
            Ok(text) => println!("{text}"),
            Err(err) => {
                eprintln!("prism: failed to serialize context pack: {err}");
                return crate::EXIT_ERROR;
            }
        }
    } else {
        print!("{}", prism_context::render_text(&pack));
    }
    crate::EXIT_OK
}
