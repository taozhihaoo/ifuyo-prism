//! M9b golden paths + 真实扫描集成（规格 §360–§370/§206–§208/§332）。
//!
//! 真实执行 EnvironmentService（含真实 probe）；机器相关的结论只断言
//! 事实形态（发现/证据/无 panic），不断言具体版本。

use prism_core::runtime::command::StdCommandRunner;
use prism_core::task::CancellationToken;
use prism_env::{EnvironmentScanOptions, EnvironmentService, MatchResult};
use std::path::PathBuf;
use std::sync::Arc;

fn temp_project(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "prism-env-golden-{name}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn scan(dir: &std::path::Path) -> prism_env::EnvironmentSnapshot {
    let cancel = CancellationToken::new();
    let discovery = prism_project::DiscoveryService::new()
        .discover(dir, prism_project::ScanOptions::default(), &cancel, |_| {})
        .ok();
    EnvironmentService::scan(
        dir,
        discovery.as_ref(),
        &EnvironmentScanOptions::default(),
        Arc::new(StdCommandRunner::new()),
        &cancel,
    )
    .unwrap()
}

// §360 final golden path：完整链路 Open → Detect → Match → Findings → Report。
#[test]
fn full_chain_on_node_project_produces_coherent_facts() {
    let dir = temp_project("golden1");
    std::fs::write(
        dir.join("package.json"),
        r#"{ "name": "demo", "engines": { "node": ">=18" } }"#,
    )
    .unwrap();

    let snapshot = scan(&dir);
    // 需求被发现且带来源证据（§258）。
    let node = snapshot
        .requirements
        .iter()
        .find(|requirement| requirement.subject == "node")
        .expect("node requirement from engines");
    assert_eq!(node.source, "package.json engines");
    let matched = snapshot
        .matches
        .iter()
        .find(|matched| matched.subject == "node")
        .expect("match");
    // 机器相关：有 node ⇒ Satisfied；无 ⇒ Unsatisfied（lookup 确认）——两者都合法，
    // 但绝不能是捏造结果：evidence 必须非空。
    assert!(!matched.evidence.is_empty(), "§258 evidence-first");
    // findings 与 matches 一致（§128）。
    let finding_count = snapshot
        .findings
        .iter()
        .filter(|finding| finding.subject == "node")
        .count();
    assert_eq!(
        finding_count,
        usize::from(matched.result != MatchResult::Satisfied),
        "satisfied ⇒ no finding; otherwise exactly one"
    );
    // 报告可渲染（§360 最后一步）。
    let markdown = prism_env::render_markdown(&snapshot);
    assert!(markdown.contains("## Requirements"));
    let _ = std::fs::remove_dir_all(&dir);
}

// §364 golden path #5：engines 与 .nvmrc 冲突 ⇒ Conflict 状态，不偷偷择一。
#[test]
fn conflicting_sources_yield_conflict_status() {
    let dir = temp_project("golden5");
    std::fs::write(
        dir.join("package.json"),
        r#"{ "engines": { "node": ">=22" } }"#,
    )
    .unwrap();
    std::fs::write(dir.join(".nvmrc"), "18\n").unwrap();

    let snapshot = scan(&dir);
    assert_eq!(snapshot.conflicts.len(), 1);
    assert_eq!(snapshot.conflicts[0].subject, "node");
    assert_eq!(snapshot.conflicts[0].entries.len(), 2);
    assert_eq!(
        snapshot.status,
        prism_env::EnvironmentStatus::Conflict,
        "§358"
    );
    let node_matches: Vec<_> = snapshot
        .matches
        .iter()
        .filter(|matched| matched.subject == "node")
        .collect();
    assert!(
        node_matches
            .iter()
            .all(|matched| matched.result == MatchResult::Unknown),
        "§327: conflict ⇒ Unknown with sources listed"
    );
    assert!(node_matches.iter().all(|matched| matched
        .evidence
        .contains("cannot be interpreted as a single unambiguous requirement")));
    let _ = std::fs::remove_dir_all(&dir);
}

// §209/§49 golden path：不可探测的主体 ⇒ Unknown，不是 Unsatisfied。
#[test]
fn unknown_subject_yields_unknown_not_unsatisfied() {
    let dir = temp_project("golden-unknown");
    std::fs::write(
        dir.join("package.json"),
        r#"{ "engines": { "definitely-not-a-known-runtime": ">=1" } }"#,
    )
    .unwrap();

    let snapshot = scan(&dir);
    let matched = snapshot
        .matches
        .iter()
        .find(|matched| matched.subject == "definitely-not-a-known-runtime")
        .expect("requirement is discovered even for unknown tools");
    assert_eq!(
        matched.result,
        MatchResult::Unknown,
        "未探测过的工具名 ⇒ Unknown（没有 lookup 事实就不能说缺失）"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

// §78/§370 golden path #11：容器项目 + docker 可用性事实。
#[test]
fn container_project_reports_docker_fact() {
    let dir = temp_project("golden-docker");
    std::fs::write(dir.join("Dockerfile"), "FROM node:22\n").unwrap();

    let snapshot = scan(&dir);
    // 容器配置 ⇒ docker requirement（ContainerConfig 来源）。
    let docker = snapshot
        .requirements
        .iter()
        .find(|requirement| requirement.subject == "docker")
        .expect("docker requirement from container config");
    assert_eq!(docker.source_kind, prism_env::SourceKind::ContainerConfig);
    // 机器相关：docker 存在 ⇒ Satisfied；不存在 ⇒ Unsatisfied + ToolMissing finding。
    let matched = snapshot
        .matches
        .iter()
        .find(|matched| matched.subject == "docker")
        .expect("docker match");
    let has_missing_finding = snapshot.findings.iter().any(|finding| {
        finding.kind == prism_env::EnvironmentFindingKind::ToolMissing
            && finding.subject == "docker"
    });
    assert_eq!(
        has_missing_finding,
        matched.result == MatchResult::Unsatisfied,
        "ToolMissing finding 与 Unsatisfied 一致"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

// §401/§400：大规模 PATH 的有界降级（配合 paths 单测；此处跑真实扫描不炸）。
#[test]
fn scan_with_many_requirements_stays_bounded() {
    let dir = temp_project("golden-many");
    std::fs::write(
        dir.join("package.json"),
        r#"{ "engines": { "node": ">=18", "npm": ">=9" } }"#,
    )
    .unwrap();
    std::fs::write(
        dir.join("pyproject.toml"),
        "[project]\nname=\"x\"\nrequires-python = \">=3.10\"\n",
    )
    .unwrap();

    let started = std::time::Instant::now();
    let snapshot = scan(&dir);
    assert!(snapshot.requirements.len() >= 3);
    // §332：普通项目秒级内（宽松上界，防回归性劣化）。
    assert!(
        started.elapsed().as_secs() < 30,
        "scan took {:?} — probe budget should cap this",
        started.elapsed()
    );
    let _ = std::fs::remove_dir_all(&dir);
}
