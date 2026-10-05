//! # Prism Supply Intelligence（2.0-M29）
//!
//! Dependency / License / Security / SBOM / Environment 声明与扫描的事实
//! 供给层。边界（§2/§3/§98/§100）：
//!
//! - **不建第二套 Finding/Graph/Health/Policy**：Findings 由 M28
//!   `prism-health2` RuleRegistry 产出（rules 挂载在 health2 侧，
//!   本 crate 只提供 `SupplyReport` 事实）；Provider 不污染 Core。
//! - **Declared ≠ Resolved**（§9）：声明（manifest）与解析（lockfile）
//!   是两种事实，永不合并。
//! - **Unknown 是一等公民**（§105）：advisory 库不可用 ⇒ Vulnerability
//!   analysis = Unavailable，绝不等于"0 vulnerabilities"；依赖的依赖侧
//!   license 离线 = UNKNOWN，不等于"没有 license"。
//! - **Secret 安全**（§35/§36/§62）：候选只存 redacted 表示 + 内容寻址
//!   指纹（不含 secret 内容）；secret 原文不进任何下游。
//! - **不信任项目文件**（§113）：全部手写/serde 数据解析，零执行、
//!   零 shell、零网络（offline-first，§72）。

pub mod advisory;
pub mod collect;
pub mod csproj;
pub mod environment;
pub mod license;
pub mod lockfiles;
pub mod manifests;
pub mod model;
pub mod sbom;
pub mod secrets;
pub mod version;

pub use advisory::{Advisory, AdvisoryMatch, AdvisoryProvider, LocalAdvisoryDatabase};
pub use collect::{collect, SupplyInputs};
pub use csproj::{parse_csproj, parse_directory_packages_props, CsprojProject, PackageVersion};
pub use environment::{compare_declared_observed, EnvironmentDeclaration, ToolchainState};
pub use license::{LicenseFact, LicensePolicy, LicenseSource};
pub use lockfiles::{parse_cargo_lock, parse_package_lock_json};
pub use manifests::{
    parse_cargo_toml, parse_global_json, parse_package_json, parse_project_godot_features,
};
pub use model::{
    DependencyCycle, DependencyDeclaration, DependencyKind, DependencySource, DriftState,
    Ecosystem, EnvironmentCapability, ProviderCapability, ResolutionState, ResolvedDependency,
    SupplyReport, VersionComparison, SUPPLY_SCHEMA_VERSION,
};
pub use sbom::{SbomComponent, SbomDocument};
pub use secrets::{scan_secrets, SecretCandidate};
