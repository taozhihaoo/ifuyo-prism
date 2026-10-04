//! ARCH004 Forbidden Dependency 配置规则测试（规格 M7 §28/§113/§133/§151/§246）。

use prism_core::config::{ForbiddenDependencyConfig, HealthConfig};
use prism_core::ids::ModuleId;
use prism_core::task::CancellationToken;
use prism_graph::{
    Dependency, DependencyGraphResult, DependencyKind, GraphMetrics, Module, ModuleKind,
    ResolutionStatus,
};
use prism_health::{analyze, HealthContext, HealthOptions, HealthRuleRegistry};

fn file_module(path: &str) -> Module {
    Module {
        id: ModuleId::from_string(format!("module:file:{path}")),
        name: path.to_string(),
        kind: ModuleKind::FileModule,
        file_ids: vec![],
        parent_id: None,
        language: None,
        path: Some(path.to_string()),
        evidence: vec![],
    }
}

fn dependency(source: &str, target: &str) -> Dependency {
    Dependency {
        id: format!("dep:{source}#{target}#import"),
        source_id: ModuleId::from_string(format!("module:file:{source}")),
        source_path: source.to_string(),
        raw_target: target.to_string(),
        kind: DependencyKind::Import,
        resolution: ResolutionStatus::Resolved,
        confidence: prism_core::Confidence::High,
        target_id: Some(ModuleId::from_string(format!("module:file:{target}"))),
        target_path: Some(target.to_string()),
        imported_names: vec![],
        location: None,
        evidence: vec![],
    }
}

fn graph_with(deps: Vec<Dependency>) -> DependencyGraphResult {
    DependencyGraphResult {
        schema_version: 1,
        project_name: "demo".to_string(),
        modules: vec![
            file_module("src/ui/page.ts"),
            file_module("src/database/db.ts"),
        ],
        dependencies: deps,
        nodes: vec![],
        edges: vec![],
        calls: vec![],
        cycles: vec![],
        metrics: GraphMetrics::default(),
        diagnostics: vec![],
    }
}

fn config_with_forbidden(from: &str, to: &str) -> HealthConfig {
    HealthConfig {
        architecture: prism_core::config::ArchitectureConfig {
            forbidden_dependencies: vec![ForbiddenDependencyConfig {
                from: from.to_string(),
                to: to.to_string(),
            }],
        },
        ..HealthConfig::default()
    }
}

fn run(config: &HealthConfig, graph: &DependencyGraphResult) -> prism_health::HealthAnalysis {
    let project = prism_project::Project {
        id: prism_core::ids::ProjectId::from_string("proj"),
        name: "demo".to_string(),
        root_path: std::path::PathBuf::from("."),
        discovered_at: prism_core::Timestamp::default(),
    };
    let context = HealthContext {
        project: &project,
        code_model: None,
        discovery: None,
        graph: Some(graph),
        git: None,
        change_set: None,
        impact: None,
        root: std::path::Path::new("."),
        derived: None,
        health_config: config,
    };
    analyze(
        &HealthRuleRegistry::with_builtins(),
        config,
        &context,
        &HealthOptions::default(),
        &CancellationToken::new(),
    )
    .unwrap()
}

/// §28/§246：配置 ui→database 禁止，观测 ui→database ⇒ ARCH004；
/// evidence 含 observed relationship 与 configured restriction。
#[test]
fn forbidden_dependency_violation_reported_with_dual_evidence() {
    let config = config_with_forbidden("ui", "database");
    let graph = graph_with(vec![dependency("src/ui/page.ts", "src/database/db.ts")]);
    let analysis = run(&config, &graph);

    let arch004: Vec<_> = analysis
        .findings
        .iter()
        .filter(|f| f.rule_id == "ARCH004")
        .collect();
    assert_eq!(arch004.len(), 1);
    let finding = arch004[0];
    assert_eq!(finding.severity, prism_core::Severity::Warning);
    // §28 双证据：observed + configured restriction。
    assert!(
        finding.evidence.iter().any(|e| e
            .detail
            .contains("observed relationship: src/ui/page.ts -> src/database/db.ts")),
        "observed relationship evidence expected"
    );
    assert!(
        finding
            .evidence
            .iter()
            .any(|e| e.detail.contains("configured restriction")),
        "configured restriction evidence expected"
    );
    // §140 措辞：violates configured rule，不是 "bad architecture"。
    assert!(finding.title.contains("configured"));
}

/// §246 后半：删除配置 ⇒ ARCH004 消失（finding 来自项目规则而非臆测）。
#[test]
fn removing_configuration_removes_finding() {
    let graph = graph_with(vec![dependency("src/ui/page.ts", "src/database/db.ts")]);
    let analysis = run(&HealthConfig::default(), &graph);
    // §113/§207：未配置 ⇒ 无 finding、无诊断噪音（规则可执行但安静跳过）。
    assert!(!analysis.findings.iter().any(|f| f.rule_id == "ARCH004"));
    assert!(analysis.diagnostics.is_empty());
}

/// §151：allowed（未命中 forbidden 对）不误报。
#[test]
fn non_matching_dependency_is_silent() {
    let config = config_with_forbidden("ui", "database");
    // ui → api（api 不匹配 database）⇒ 无 finding。
    let graph = graph_with(vec![dependency("src/ui/page.ts", "src/api/client.ts")]);
    let analysis = run(&config, &graph);
    assert!(!analysis.findings.iter().any(|f| f.rule_id == "ARCH004"));
}

/// §151：规则禁用 ⇒ 不运行（isolation）。
#[test]
fn disabled_arch004_does_not_run() {
    let mut config = config_with_forbidden("ui", "database");
    config.rules.insert(
        "ARCH004".to_string(),
        prism_core::config::HealthRuleConfig {
            enabled: Some(false),
            severity: None,
            threshold: None,
        },
    );
    let graph = graph_with(vec![dependency("src/ui/page.ts", "src/database/db.ts")]);
    let analysis = run(&config, &graph);
    assert!(!analysis.findings.iter().any(|f| f.rule_id == "ARCH004"));
    assert!(analysis.disabled_rules.contains(&"ARCH004".to_string()));
}

/// 确定性（§95）：同配置同图两次运行输出一致。
#[test]
fn arch004_is_deterministic() {
    let config = config_with_forbidden("ui", "database");
    let graph = graph_with(vec![dependency("src/ui/page.ts", "src/database/db.ts")]);
    let first = run(&config, &graph);
    let second = run(&config, &graph);
    assert_eq!(
        serde_json::to_string(&first).unwrap(),
        serde_json::to_string(&second).unwrap()
    );
}
