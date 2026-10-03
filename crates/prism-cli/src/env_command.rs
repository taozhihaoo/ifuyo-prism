//! `prism env <path>` — 环境扫描基础命令（规格 M9a §240–§242/§268/§295–§297）。
//!
//! stdout 只输出数据（JSON/文本报告，§295）；人类进度不打印（本命令足够快）。
//! exit：0 = 无 Unsatisfied；1 = 有 Unsatisfied（或 --strict 下有 Unknown）；
//! 3 = 路径不存在等输入错误。Unknown ≠ Unsatisfied（§242 默认宽松）。

use prism_core::runtime::command::{CommandRunner, StdCommandRunner};
use prism_core::task::CancellationToken;
use prism_env::{EnvironmentScanOptions, EnvironmentService};
use std::path::Path;
use std::sync::Arc;

pub fn run(path: &Path, json: bool, strict: bool, all_tools: bool) -> i32 {
    if !path.exists() {
        eprintln!("prism: path does not exist: {}", path.to_string_lossy());
        return crate::EXIT_INVALID_INPUT;
    }
    let runner: Arc<dyn CommandRunner> = Arc::new(StdCommandRunner::new());
    let cancel = CancellationToken::new();

    // §152/§334：复用一次 M1 discovery（失败 ⇒ generic profile，§122 不失败）。
    let discovery = prism_project::DiscoveryService::new()
        .discover(path, prism_project::ScanOptions::default(), &cancel, |_| {})
        .ok();
    let project = match prism_project::Project::from_root(path) {
        Ok(project) => project,
        Err(err) => {
            eprintln!("prism: cannot open project: {err}");
            return crate::EXIT_INVALID_INPUT;
        }
    };

    let options = EnvironmentScanOptions {
        include_all_tools: all_tools,
        ..Default::default()
    };
    let mut snapshot = match EnvironmentService::scan(
        path,
        discovery.as_ref(),
        &options,
        runner.clone(),
        &cancel,
    ) {
        Ok(snapshot) => snapshot,
        Err(err) => {
            eprintln!("prism: environment scan failed: {err}");
            return crate::EXIT_ERROR;
        }
    };
    snapshot.project_id = project.id.as_str().to_string();

    if json {
        match serde_json::to_string_pretty(&snapshot) {
            Ok(text) => println!("{text}"),
            Err(err) => {
                eprintln!("prism: failed to serialize environment snapshot: {err}");
                return crate::EXIT_ERROR;
            }
        }
    } else {
        print!("{}", prism_env::render_text(&snapshot));
    }

    // §240/§242：Unsatisfied ⇒ 非零；Unknown 默认不算失败；--strict 才算（§241）。
    let mut unsatisfied = 0usize;
    let mut unknown = 0usize;
    for matched in &snapshot.matches {
        match matched.result {
            prism_env::MatchResult::Unsatisfied => unsatisfied += 1,
            prism_env::MatchResult::Unknown => unknown += 1,
            _ => {}
        }
    }
    if unsatisfied > 0 {
        eprintln!(
            "prism: {unsatisfied} environment requirement(s) unsatisfied"
        );
        return crate::EXIT_ERROR;
    }
    if strict && unknown > 0 {
        eprintln!(
            "prism: --strict: {unknown} environment requirement(s) unknown (treated as failure)"
        );
        return crate::EXIT_ERROR;
    }
    crate::EXIT_OK
}
