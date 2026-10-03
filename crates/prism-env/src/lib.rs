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

pub mod host;
pub mod model;
pub mod paths;
pub mod probe;
pub mod requirement;
pub mod runtime;
pub mod variables;
pub mod tools;
pub mod version;

pub use host::{collect_host, collect_resources, collect_shell};
pub use model::{
    Architecture, EnvironmentCompleteness, EnvironmentDiagnostic, EnvironmentRequirement,
    EnvironmentSnapshot, FactScope, HostInfo, MatchResult, OsFamily, PackageManagerAssociation,
    PackageManagerInfo, PathEntry, PathFacts, ProbeOutcome, RequirementKind, RequirementMatch,
    RequirementOperator, ResourceFacts, RuntimeInfo, SdkInfo, ScanOutcome, ShellInfo, ShellKind,
    ToolFact, ToolSource, VariableFact, VariableSource, VirtualEnvInfo,
    ENVIRONMENT_SCHEMA_VERSION, PATH_ENTRY_CAP, VARIABLE_CAP,
};
pub use runtime::{
    discover_package_managers, discover_sdks, discover_virtualenvs,
    package_manager_field, parse_dotnet_sdks, parse_rustup_toolchains, resolve_project_local,
    runtimes_from_tools, LOCKFILES,
};
pub use paths::{collect_path_facts, normalize_path, path_facts_from_raw, path_identity};
pub use probe::{run_probe, version_line, ProbeOutputKind, ProbeResult, ToolProbe, DEFAULT_PROBE_TIMEOUT_MS};
pub use tools::{
    definition, discover_tool, EnvironmentProfile, ToolDefinition, TOOL_ALLOWLIST,
};
pub use requirement::{base_requirements, match_requirements};
pub use variables::{collect_variables, is_possibly_sensitive};
pub use version::{
    compare_versions, parse_version_output, parse_version_token, satisfies, ToolVersion,
};
