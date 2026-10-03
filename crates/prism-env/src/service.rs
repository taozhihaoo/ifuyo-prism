//! EnvironmentService 编排（规格 M9a §118/§126/§268–§270/§309–§311/§392–§395）。
//!
//! 分阶段（§118 Host → Toolchain → Requirements → Variables → Matching）；
//! 每个采集器失败隔离（§392/§393）；probe 有界并发 + 总预算（§309/§310）；
//! 取消返回部分快照（§394）；输出顺序确定（§395）。

use crate::model::{
    EnvironmentCompleteness, EnvironmentDiagnostic, EnvironmentSnapshot, EnvironmentStatus,
    FactScope, PathFacts, ProbeOutcome, ScanOutcome, ToolFact, ENVIRONMENT_SCHEMA_VERSION,
};
use crate::requirements::collect_requirements;
use crate::requirement::match_requirements;
use crate::tools::{discover_tool, definition, EnvironmentProfile};
use prism_core::runtime::command::CommandRunner;
use prism_core::task::CancellationToken;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// 扫描选项（§269/§270：默认 project-relevant；重扫描显式开启）。
#[derive(Debug, Clone)]
pub struct EnvironmentScanOptions {
    /// §270 heavy：探测全部 allowlist 工具（默认只探测项目相关）。
    pub include_all_tools: bool,
    pub include_variables: bool,
    pub include_path: bool,
    pub include_resources: bool,
    /// §86/§81 动态事实（默认采集；§217 每次刷新）。
    pub include_processes: bool,
    pub include_ports: bool,
    /// §310 max_parallel_probes。
    pub max_parallel_probes: usize,
    /// §309 总预算；超时后剩余工具记 NotProbed + 诊断（不失败）。
    pub total_budget: Duration,
}

impl Default for EnvironmentScanOptions {
    fn default() -> Self {
        EnvironmentScanOptions {
            include_all_tools: false,
            include_variables: true,
            include_path: true,
            include_resources: true,
            include_processes: true,
            include_ports: true,
            max_parallel_probes: 4,
            total_budget: Duration::from_secs(30),
        }
    }
}

pub struct EnvironmentService;

impl EnvironmentService {
    /// 完整扫描（§268 discover）。`discovery` 传 M1 结果避免重扫（§152/§334）；
    /// 传 None ⇒ generic profile（§122：未知项目不失败）。
    pub fn scan(
        project_root: &Path,
        discovery: Option<&prism_project::ProjectDiscovery>,
        options: &EnvironmentScanOptions,
        runner: Arc<dyn CommandRunner>,
        cancel: &CancellationToken,
    ) -> prism_core::PrismResult<EnvironmentSnapshot> {
        let started = Instant::now();
        let mut diagnostics: Vec<EnvironmentDiagnostic> = Vec::new();

        // ---- Host / Shell（§118 第一阶段；§14–§18）----
        let host = crate::host::collect_host();
        let shell = crate::host::collect_shell();

        // ---- Toolchain（§118 第二阶段；§120 lazy / §122 generic）----
        let profile = discovery
            .map(EnvironmentProfile::from_discovery)
            .unwrap_or_else(EnvironmentProfile::generic);
        let tool_names: Vec<&str> = if options.include_all_tools {
            crate::tools::TOOL_ALLOWLIST.iter().map(|d| d.name).collect()
        } else {
            profile.tool_names()
        };

        // 有界并发 probe（§117/§310）；按名字排序输出（§395）。
        let definitions: Vec<&crate::tools::ToolDefinition> = tool_names
            .iter()
            .filter_map(|name| definition(name))
            .collect();
        let tools = probe_tools_concurrent(
            &definitions,
            &runner,
            project_root,
            cancel,
            options.max_parallel_probes,
            started,
            options.total_budget,
            &mut diagnostics,
        );

        // ---- 生态事实（§31–§37/§62–§70；各自失败隔离，§392/§393）----
        let runtimes = crate::runtime::runtimes_from_tools(&tools);
        let sdks = crate::runtime::discover_sdks(&tools, runner.as_ref(), project_root, cancel);
        if cancel.is_cancelled() {
            return Ok(cancelled_snapshot(project_root, host, shell, tools, runtimes, sdks, diagnostics));
        }
        let package_managers = crate::runtime::discover_package_managers(&tools, project_root);
        let virtualenvs =
            crate::runtime::discover_virtualenvs(project_root, runner.as_ref(), cancel);

        // ---- PATH / Variables / Resources（§19–§21/§54/§140）----
        let paths = if options.include_path {
            crate::paths::collect_path_facts(cancel)
        } else {
            PathFacts {
                scope: FactScope::Unknown,
                entries: Vec::new(),
                duplicates: Vec::new(),
                missing: Vec::new(),
                truncated: false,
            }
        };
        // ---- Ports / Processes（§81–§94 动态事实，§211 observed_at）----
        let (ports, processes, expected_ports) = if cancel.is_cancelled() {
            (Vec::new(), Vec::new(), Vec::new())
        } else {
            let observed_at = prism_core::Timestamp::now_utc().to_string();
            let collected = if options.include_ports {
                crate::ports::collect_ports(&runner, cancel, &observed_at)
            } else {
                Vec::new()
            };
            let processes = if options.include_processes {
                crate::ports::collect_processes(&collected, &runner, cancel, &observed_at)
            } else {
                Vec::new()
            };
            let expected = crate::ports::expected_ports(project_root);
            (collected, processes, expected)
        };

        // ---- .env 元数据（§58–§60/§59：M4 tracked 事实，只读）----
        let env_files = if options.include_variables {
            let git = prism_git::GitService::new(runner.clone());
            crate::envfiles::collect_env_files(project_root, Some(&git), cancel)
        } else {
            Vec::new()
        };

        let variables = if options.include_variables {
            crate::variables::collect_variables(cancel)
        } else {
            Vec::new()
        };
        let resources = if options.include_resources {
            crate::host::collect_resources(project_root)
        } else {
            Default::default()
        };

        // ---- Requirements + Matching（§42–§45/§46/§124–§127/§258）----
        let discovery_files: Option<Vec<String>> = discovery.map(|d| {
            d.files
                .iter()
                .map(|file| file.relative_path.clone())
                .collect()
        });
        let facts = crate::requirements::ProjectFacts {
            root: project_root,
            files: discovery_files.as_deref(),
        };
        let (requirements, conflicts) = collect_requirements(&facts);
        let matches = match_requirements(
            &requirements,
            &conflicts,
            &tools,
            &package_managers,
            &variables,
            &ports,
            project_root,
        );

        // ---- 状态（§337/§338/§138）----

        let (scan_outcome, completeness) = if cancel.is_cancelled() {
            (ScanOutcome::Cancelled, EnvironmentCompleteness::Partial)
        } else {
            let degraded = discovery.is_none()
                || tools
                    .iter()
                    .any(|tool| tool.probe == ProbeOutcome::NotProbed);
            let outcome = if degraded {
                ScanOutcome::Partial
            } else {
                ScanOutcome::Completed
            };
            let completeness = if discovery.is_none() {
                EnvironmentCompleteness::Partial
            } else {
                EnvironmentCompleteness::Complete
            };
            (outcome, completeness)
        };

        let mut snapshot = EnvironmentSnapshot {
            schema_version: ENVIRONMENT_SCHEMA_VERSION,
            id: uuid::Uuid::new_v4().to_string(),
            project_id: String::new(), // 由调用方填（Project::id）
            project_root: project_root.to_string_lossy().to_string(),
            captured_at: prism_core::Timestamp::now_utc().to_string(),
            host,
            shell,
            paths,
            tools,
            virtualenvs,
            runtimes,
            sdks,
            package_managers,
            variables,
            env_files,
            ports,
            processes,
            expected_ports,
            resources,
            requirements,
            conflicts,
            matches,
            findings: Vec::new(),
            status: EnvironmentStatus::default(),
            diagnostics,
            scan_outcome,
            completeness,
        };
        // ---- Findings + 聚合状态（§128/§354–§358，从既有事实派生）----
        let findings = crate::findings::derive_findings(&snapshot);
        let status = crate::findings::aggregate_status(&snapshot, &findings);
        snapshot.findings = findings;
        snapshot.status = status;
        Ok(snapshot)
    }

    /// §268 relevant_tools：项目相关工具名（供 M8/未来复用）。
    pub fn relevant_tools(discovery: &prism_project::ProjectDiscovery) -> Vec<&'static str> {
        EnvironmentProfile::from_discovery(discovery).tool_names()
    }
}

/// 有界并发 probe（§117/§310）：同时在飞 ≤ max_parallel；
/// 预算耗尽后不再派发（§309），未探测的记 NotProbed + 诊断；
/// 取消时未派发的记 NotProbed 并收割在飞结果（§394）。
#[allow(clippy::too_many_arguments)]
fn probe_tools_concurrent(
    definitions: &[&'static crate::tools::ToolDefinition],
    runner: &Arc<dyn CommandRunner>,
    working_directory: &Path,
    cancel: &CancellationToken,
    max_parallel: usize,
    started: Instant,
    total_budget: Duration,
    diagnostics: &mut Vec<EnvironmentDiagnostic>,
) -> Vec<ToolFact> {
    let max_parallel = max_parallel.max(1);
    let (sender, receiver) = std::sync::mpsc::channel::<ToolFact>();
    let mut queue: Vec<&'static crate::tools::ToolDefinition> = definitions.to_vec();
    let mut facts: Vec<ToolFact> = Vec::new();
    let mut in_flight = 0usize;

    while !queue.is_empty() || in_flight > 0 {
        if cancel.is_cancelled() {
            for definition in queue.drain(..) {
                facts.push(not_probed(definition.name));
            }
            break;
        }
        let budget_exhausted = started.elapsed() >= total_budget;
        if budget_exhausted && in_flight == 0 {
            let remaining = queue.len();
            for definition in queue.drain(..) {
                facts.push(not_probed(definition.name));
            }
            if remaining > 0 {
                diagnostics.push(EnvironmentDiagnostic {
                    kind: "scan_budget_exhausted".to_string(),
                    subject: None,
                    detail: format!(
                        "total scan budget ({total_budget:?}) reached;                          {remaining} tool(s) not probed"
                    ),
                });
            }
            break;
        }
        // 并发满 / 预算耗尽 / 队列已空 ⇒ 只收割在飞结果。
        if in_flight >= max_parallel || budget_exhausted || queue.is_empty() {
            if let Ok(fact) = receiver.recv_timeout(Duration::from_secs(10)) {
                facts.push(fact);
                in_flight -= 1;
            }
            continue;
        }
        let Some(definition) = queue.pop() else {
            continue;
        };
        let runner = runner.clone();
        let working_directory = working_directory.to_path_buf();
        let cancel = cancel.clone();
        let definition = *definition;
        let sender = sender.clone();
        in_flight += 1;
        std::thread::spawn(move || {
            let fact = discover_tool(&definition, runner.as_ref(), &working_directory, &cancel);
            let _ = sender.send(fact);
        });
    }
    drop(sender);
    facts.sort_by(|a, b| a.name.cmp(&b.name));
    facts
}

fn not_probed(name: &str) -> ToolFact {
    ToolFact {
        name: name.to_string(),
        path: None,
        additional_paths: vec![],
        version: None,
        raw_version: None,
        source: crate::model::ToolSource::Unknown,
        probe: ProbeOutcome::NotProbed,
    }
}

fn cancelled_snapshot(
    project_root: &Path,
    host: crate::model::HostInfo,
    shell: crate::model::ShellInfo,
    tools: Vec<ToolFact>,
    runtimes: Vec<crate::model::RuntimeInfo>,
    sdks: Vec<crate::model::SdkInfo>,
    diagnostics: Vec<EnvironmentDiagnostic>,
) -> EnvironmentSnapshot {
    EnvironmentSnapshot {
        schema_version: ENVIRONMENT_SCHEMA_VERSION,
        id: uuid::Uuid::new_v4().to_string(),
        project_id: String::new(),
        project_root: project_root.to_string_lossy().to_string(),
        captured_at: prism_core::Timestamp::now_utc().to_string(),
        host,
        shell,
        paths: PathFacts {
            scope: FactScope::Unknown,
            entries: vec![],
            duplicates: vec![],
            missing: vec![],
            truncated: false,
        },
        tools,
        virtualenvs: vec![],
        runtimes,
        sdks,
        package_managers: vec![],
        variables: vec![],
        env_files: vec![],
        ports: vec![],
        processes: vec![],
        expected_ports: vec![],
        resources: Default::default(),
        requirements: vec![],
        conflicts: vec![],
        matches: vec![],
        findings: vec![],
        status: EnvironmentStatus::Partial,
        diagnostics,
        scan_outcome: ScanOutcome::Cancelled,
        completeness: EnvironmentCompleteness::Partial,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use prism_core::runtime::command::StdCommandRunner;

    fn temp_project(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "prism-env-svc-{name}-{}-{}",
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
    fn scan_is_project_relevant_and_isolated() {
        let dir = temp_project("node-rust");
        std::fs::write(dir.join("package.json"), "{}").unwrap();
        std::fs::write(dir.join("Cargo.toml"), "[package]\nname=\"x\"\nversion=\"0.1.0\"\n").unwrap();

        let cancel = CancellationToken::new();
        let discovery = prism_project::DiscoveryService::new()
            .discover(&dir, prism_project::ScanOptions::default(), &cancel, |_| {})
            .unwrap();
        let snapshot = EnvironmentService::scan(
            &dir,
            Some(&discovery),
            &EnvironmentScanOptions::default(),
            Arc::new(StdCommandRunner::new()),
            &cancel,
        )
        .unwrap();

        assert_eq!(snapshot.schema_version, ENVIRONMENT_SCHEMA_VERSION);
        // §120 lazy：node + rust profile，不含 dotnet/go/docker。
        let names: Vec<&str> = snapshot.tools.iter().map(|tool| tool.name.as_str()).collect();
        assert!(names.contains(&"git") && names.contains(&"node") && names.contains(&"cargo"));
        assert!(!names.contains(&"dotnet") && !names.contains(&"docker"));
        // §395：稳定排序。
        let mut sorted = names.clone();
        sorted.sort_unstable();
        assert_eq!(names, sorted);
        // 变量只含分类，不含值（结构保证 + §54）。
        for fact in &snapshot.variables {
            assert!(fact.present);
        }
        // host 事实完整（Windows 上含版本）。
        assert!(!snapshot.host.os_name.is_empty());
        assert_ne!(snapshot.scan_outcome, ScanOutcome::Cancelled);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn generic_scan_without_discovery_never_fails() {
        let dir = temp_project("generic");
        let cancel = CancellationToken::new();
        let snapshot = EnvironmentService::scan(
            &dir,
            None,
            &EnvironmentScanOptions {
                include_variables: false,
                ..Default::default()
            },
            Arc::new(StdCommandRunner::new()),
            &cancel,
        )
        .unwrap();
        // §122：未知项目至少 OS/arch/git，不失败。
        let names: Vec<&str> = snapshot.tools.iter().map(|tool| tool.name.as_str()).collect();
        assert_eq!(names, vec!["git"]);
        assert_eq!(snapshot.variables.len(), 0, "include_variables=false ⇒ not collected");
        assert_eq!(snapshot.completeness, EnvironmentCompleteness::Partial);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn cancelled_scan_returns_partial_snapshot() {
        let dir = temp_project("cancel");
        std::fs::write(dir.join("package.json"), "{}").unwrap();
        let cancel = CancellationToken::new();
        cancel.cancel();
        let snapshot = EnvironmentService::scan(
            &dir,
            None,
            &EnvironmentScanOptions::default(),
            Arc::new(StdCommandRunner::new()),
            &cancel,
        )
        .unwrap();
        assert_eq!(snapshot.scan_outcome, ScanOutcome::Cancelled, "§394 partial, no panic");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
