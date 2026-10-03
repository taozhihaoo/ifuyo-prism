//! prism-env — Environment Facts 层（M9a：Environment Facts and Matching）。
//!
//! 定位（§0）：发现、规范化、展示和比较"项目所处的本地工程环境事实"。
//! 只做 **Observe, Compare, Explain**，不做 Install/Repair/Mutate（§1）。
//!
//! ```text
//! Host → Toolchain(lazy) → Requirements → Matching → Snapshot → Report
//! ```
//!
//! 边界：全部 probe 只读、allowlist、有界超时、可取消（§112–§117/§176）；
//! 敏感变量只记录 name + classification（§54）；Unknown ≠ Missing ≠ Broken
//! （§49/§157–§158/§209）；无网络 / 无遥测 / 无数据库（§253–§255/§384–§386）。

pub mod findings;
pub mod host;
pub mod model;
pub mod paths;
pub mod ports;
pub mod probe;
pub mod requirement;
pub mod requirements;
pub mod render;
pub mod report;
pub mod service;
pub mod runtime;
pub mod variables;
pub mod tools;
pub mod version;

pub use findings::{aggregate_status, derive_findings};
pub use host::{collect_host, collect_resources, collect_shell};
pub use model::{
    Architecture, ConflictEntry, EnvironmentCapability, EnvironmentCompleteness, EnvironmentDiagnostic, EnvironmentRequirement,
    EnvironmentSnapshot, FactScope, HostInfo, MatchResult, OsFamily, PackageManagerAssociation,
    PackageManagerInfo, PathEntry, PathFacts, ProbeOutcome, RequirementKind, RequirementMatch,
    PortInfo, PortProtocol, PortState, ProcessInfo, ProcessKind, RequirementOperator, ResourceFacts, RuntimeInfo, SdkInfo, ScanOutcome, ShellInfo, ShellKind,
    EnvironmentFinding, EnvironmentFindingKind, EnvironmentStatus, ExpectedPort,
    FindingCategory, FindingConfidence, SourceKind, ToolFact, ToolSource, VariableFact, VariableSource,
    VirtualEnvInfo,
    ENVIRONMENT_SCHEMA_VERSION, PATH_ENTRY_CAP, VARIABLE_CAP,
};
pub use runtime::{
    discover_package_managers, discover_sdks, discover_virtualenvs,
    package_manager_field, parse_dotnet_sdks, parse_rustup_toolchains, resolve_project_local,
    runtimes_from_tools, LOCKFILES,
};
pub use ports::{classify, collect_ports, collect_processes, expected_ports, parse_netstat, parse_ps, parse_ss, parse_tasklist};
pub use paths::{collect_path_facts, normalize_path, path_facts_from_raw, path_identity};
pub use probe::{run_probe, version_line, ProbeOutputKind, ProbeResult, ToolProbe, DEFAULT_PROBE_TIMEOUT_MS};
pub use tools::{
    definition, discover_tool, EnvironmentProfile, ToolDefinition, TOOL_ALLOWLIST,
};
pub use requirement::match_requirements;
pub use requirements::{collect_requirements, package_manager_requirement, parse_version_spec, ProjectFacts, RequirementProvider};
pub use variables::{collect_variables, is_possibly_sensitive};
pub use render::{render_text, sanitize};
pub use report::{escape_html, render_html, render_markdown, shorten_home_paths};
pub use service::{EnvironmentScanOptions, EnvironmentService};
pub use version::{
    compare_versions, parse_version_output, parse_version_token, satisfies, ToolVersion,
};
