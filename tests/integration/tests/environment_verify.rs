//! M8 × M9 组合流（规格 M9b §379/§380：跨引擎一致性 + 无循环依赖证明）。
//!
//! §380："这是非常关键的集成测试" —— M9 的工具事实（ExecutableResolution/
//! probe）与 M8 的真实执行结果（Unavailable/Passed）必须互相一致；
//! §267：依赖方向 prism-verify → prism-env 合法，本测试同时消费两 crate，
//! 证明 workspace 无环。
//!
//! 机器无关性：断言的是**两引擎结论的一致性**，不预设工具存在与否——
//! 环境里 yarn/bun 缺失或存在都应通过。

use prism_core::config::{VerifyCheckConfig, VerifyConfig};
use prism_core::runtime::command::{CommandRunner, StdCommandRunner};
use prism_core::task::CancellationToken;
use prism_env::{EnvironmentScanOptions, EnvironmentService};
use prism_verify::{discover_checks, run_verification, CheckState};
use std::sync::Arc;

fn temp_project(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "prism-int-m89-{name}-{}-{}",
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

#[test]
fn env_facts_and_verify_execution_agree() {
    let dir = temp_project("agree");
    std::fs::write(dir.join("package.json"), r#"{ "name": "demo" }"#).unwrap();

    let runner: Arc<dyn CommandRunner> = Arc::new(StdCommandRunner::new());
    let cancel = CancellationToken::new();

    // ---- M9：环境事实（node profile：node/npm/pnpm/yarn/bun/corepack/git）----
    let discovery = prism_project::DiscoveryService::new()
        .discover(&dir, prism_project::ScanOptions::default(), &cancel, |_| {})
        .ok();
    let snapshot = EnvironmentService::scan(
        &dir,
        discovery.as_ref(),
        &EnvironmentScanOptions::default(),
        runner.clone(),
        &cancel,
    )
    .unwrap();

    // ---- M8：对同样工具执行真实 check（自定义 check 走 verify 引擎）----
    let mut config = VerifyConfig::default();
    for tool in ["node", "yarn"] {
        config.checks.insert(
            format!("{tool}-check"),
            VerifyCheckConfig {
                kind: "build".to_string(),
                program: tool.to_string(),
                args: vec!["--version".to_string()],
                working_directory: None,
                timeout_ms: Some(30_000),
                required: None,
                write_policy: Some("read_only".to_string()),
                depends_on: vec![],
            },
        );
    }
    let specs = discover_checks(&dir, &config);
    let report = run_verification(
        &specs,
        &dir,
        "demo",
        &prism_verify::VerifyOptions::default(),
        runner,
        2,
        true,
        None,
        &[],
        &cancel,
    )
    .unwrap();

    // ---- §380：两引擎结论必须一致 ----
    for tool in ["node", "yarn"] {
        let env_fact = snapshot
            .tools
            .iter()
            .find(|fact| fact.name == tool)
            .unwrap_or_else(|| panic!("M9 must probe {tool} in node profile"));
        let run = report
            .runs
            .iter()
            .find(|run| run.check_id == format!("{tool}-check"))
            .unwrap_or_else(|| panic!("M8 must execute {tool}-check"));

        match env_fact.probe {
            prism_env::ProbeOutcome::NotFound => {
                // §379/§380：M9 说缺失 ⇒ M8 必然 Unavailable（spawn 前置失败），
                // 而不是别的状态。
                assert_eq!(
                    run.state,
                    CheckState::Unavailable,
                    "M9 {tool}=not_found but M8 said {:?} — engines disagree",
                    run.state
                );
            }
            prism_env::ProbeOutcome::Succeeded => {
                assert!(
                    matches!(run.state, CheckState::Passed | CheckState::Failed),
                    "M9 {tool}=succeeded but M8 said {:?} — engines disagree",
                    run.state
                );
            }
            other => {
                // M9 无法判定（超时等）⇒ M8 自行执行，两引擎独立成立即可。
                let _ = (other, run);
            }
        }
    }

    // §267：两 crate 同时消费编译通过，即依赖方向无环的结构证明。
    let _ = snapshot.schema_version;
    let _ = report.schema_version;
    let _ = std::fs::remove_dir_all(&dir);
}
