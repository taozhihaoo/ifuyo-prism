//! `--changed` 集成（规格 M8b §32-§34/§247/§307）：
//! M5 ChangeSet → M6 ImpactAnalysis → TestCandidate。
//!
//! 只产出**潜在**受影响测试候选（Potential，§247）；绝不推断具体测试命令、
//! 绝不冒充 selective verification（§33/§34）。图不可建（语言不支持等）
//! ⇒ 退回 M1 Discovery 的测试文件分类——仍是候选清单，只是更粗。

use prism_core::runtime::command::CommandRunner;
use prism_core::task::CancellationToken;
use std::path::Path;
use std::sync::Arc;

/// 受影响测试候选路径（去重排序；Potential，不冒充已验证集合）。
pub fn impacted_test_candidates(
    root: &Path,
    runner: Arc<dyn CommandRunner>,
    cancel: &CancellationToken,
) -> Vec<String> {
    let discovery = match prism_project::DiscoveryService::new().discover(
        root,
        prism_project::ScanOptions::default(),
        cancel,
        |_| {},
    ) {
        Ok(discovery) => discovery,
        Err(_) => return Vec::new(),
    };

    let fallback: Vec<String> = discovery
        .files
        .iter()
        .filter(|file| file.classification.kind == prism_project::FileKind::Test)
        .map(|file| file.relative_path.clone())
        .collect();

    let chain = (|| {
        let model = prism_analysis::CodeAnalysisService::new()
            .analyze(
                &discovery,
                prism_analysis::CodeAnalysisOptions::default(),
                cancel,
                |_| {},
            )
            .ok()?;
        let graph = prism_graph::DependencyService::new()
            .analyze(&discovery, &model)
            .ok()?;
        let change_set = prism_changes::ChangeSetService::new(runner)
            .build(
                root,
                prism_changes::ChangeScope::WorkingTree,
                None,
                None,
                &prism_changes::ChangeSetOptions::default(),
                cancel,
            )
            .ok()
            .flatten()?;
        let impact = prism_impact::ImpactAnalysisService
            .analyze(
                &change_set,
                &model,
                &graph,
                &prism_impact::ImpactOptions::default(),
                cancel,
            )
            .ok()?;
        let mut candidates: Vec<String> = impact
            .impacted_items
            .iter()
            .filter(|node| {
                node.impact_kinds
                    .contains(&prism_impact::ImpactKind::TestCandidate)
            })
            .map(|node| node.path.clone())
            .filter(|path| !path.is_empty())
            .collect();
        candidates.sort();
        candidates.dedup();
        Some(candidates)
    })();

    chain.unwrap_or(fallback)
}
