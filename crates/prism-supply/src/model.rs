//! 供给事实模型（2.0-M29 §7-§19）。
//!
//! 结构化身份（§14）：`{ecosystem}:{package_id}` 是内部 id；
//! `nuget:Newtonsoft.Json@13.0.3` 只是展示形式。声明与解析分离（§9）。

use prism_intel::evidence::EvidenceId;
use prism_intel::meta::Origin;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// M29 输出契约版本（§71/§111：变更须版本化）。
pub const SUPPLY_SCHEMA_VERSION: u32 = 1;

/// 包生态（开放集合；未知生态如实 Unknown，不假装支持）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Ecosystem {
    NuGet,
    Npm,
    Cargo,
    PyPi,
    Godot,
    Unknown,
}

impl Ecosystem {
    pub const fn as_str(self) -> &'static str {
        match self {
            Ecosystem::NuGet => "nuget",
            Ecosystem::Npm => "npm",
            Ecosystem::Cargo => "cargo",
            Ecosystem::PyPi => "pypi",
            Ecosystem::Godot => "godot",
            Ecosystem::Unknown => "unknown",
        }
    }
}

/// 依赖种类（§8：不把所有依赖压成一个字符串）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencyKind {
    ProjectReference,
    PackageReference,
    Tool,
    Runtime,
    Framework,
    Analyzer,
    BuildDependency,
    DevelopmentDependency,
    OptionalDependency,
    Unknown,
}

/// 依赖来源（§18：能回答"从哪里来"，不只显示包名）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencySource {
    NuGet,
    LocalPackage,
    Git,
    File,
    Project,
    Registry,
    Vendored,
    Generated,
    Unknown,
}

/// 解析状态（§17：解析失败 ≠ 没有依赖）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResolutionState {
    Resolved,
    Partial,
    Unresolved,
    Ambiguous,
    Unsupported,
    Unavailable,
    Unknown,
}

/// 版本漂移分类（§11：Declared != Resolved 不自动等于错误）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DriftState {
    /// 声明为范围/通配，解析落在范围内（如 `8.0.*` → 8.0.11）。
    NormalResolution,
    /// 声明与解析一致。
    Expected,
    /// 落在声明范围之外或无声明可对照。
    Drift,
    /// 明确冲突（同名多版本锁定等）。
    Mismatch,
    Conflict,
    Unresolved,
    Unknown,
}

/// 请求版本的来源（§13 CPM：PackageReference 无版本 ≠ 无版本事实）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VersionSource {
    /// csproj 内联 Version 属性。
    Manifest,
    /// Directory.Packages.props 中央版本。
    Central,
    Unspecified,
}

/// 依赖声明（§8：manifest 侧事实，Origin=Fact）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DependencyDeclaration {
    /// `{ecosystem}:{package_id}`（ProjectReference 为 `project:{相对路径}`）。
    pub id: String,
    pub ecosystem: Ecosystem,
    pub package_id: String,
    /// 原样约束（`8.0.*` / `^1.2.3` / `>= 13.0`）；无声明为 None。
    pub requested_version: Option<String>,
    pub version_is_range: bool,
    pub kind: DependencyKind,
    pub source: DependencySource,
    /// direct / development / central / tool 等（生态原样语义）。
    pub scope: String,
    /// 项目相对 manifest 路径（证据定位，非身份 §19）。
    pub manifest_path: String,
    pub line: Option<usize>,
    pub version_source: VersionSource,
    pub origin: Origin,
    pub evidence: Vec<EvidenceId>,
}

/// 解析后依赖（§9：lockfile 侧事实）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvedDependency {
    pub id: String,
    pub ecosystem: Ecosystem,
    pub package_id: String,
    /// 锁定版本原样；锁文件损坏/缺失 ⇒ None（不是 "0"，§17）。
    pub resolved_version: Option<String>,
    /// lockfile 标记为 Direct（npm lock type=Direct / csproj 声明存在）。
    pub direct: bool,
    pub from_lockfile: bool,
    pub lockfile_path: String,
    pub evidence: Vec<EvidenceId>,
}

/// 声明 × 解析 对照结论（§11）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionComparison {
    pub declaration_id: String,
    pub package_id: String,
    pub state: DriftState,
    pub detail: String,
}

/// 项目引用边（§15：正式 Relation 的事实侧；谁引用谁）。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ProjectReferenceEdge {
    pub from_project: String,
    pub to_project: String,
    pub manifest_path: String,
}

/// 项目引用环（§16：必须带证据路径 A→B→…→A）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DependencyCycle {
    /// 环路径，首尾相同（如 ["A","B","C","A"]）。
    pub path: Vec<String>,
}

/// Provider 能力状态（§102：UI/MCP 必须能看见）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderCapability {
    Supported,
    Partial,
    Unavailable,
    Unsupported,
    Failed,
}

/// 供给报告：一次分析的完整事实产物（M28 规则/健康层的注入对象）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct SupplyReport {
    pub schema_version: u32,
    /// 按 id 排序（确定性 §95）。
    pub declarations: Vec<DependencyDeclaration>,
    pub resolved: Vec<ResolvedDependency>,
    pub version_comparisons: Vec<VersionComparison>,
    pub project_references: Vec<ProjectReferenceEdge>,
    pub cycles: Vec<DependencyCycle>,
    pub licenses: Vec<crate::license::LicenseFact>,
    /// 已应用的 license policy 判定（§65：policy 是 M28 Gate 的数据）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license_policy: Option<crate::license::LicensePolicy>,
    pub license_policy_violations: Vec<crate::license::LicensePolicyOutcome>,
    pub secret_candidates: Vec<crate::secrets::SecretCandidate>,
    /// 候选达到 cap 被截断（§69：报告 truncated，不假装扫完）。
    pub secret_scan_truncated: bool,
    pub advisory_capability: Option<ProviderCapability>,
    pub advisory_database_version: Option<String>,
    pub advisory_matches: Vec<crate::advisory::AdvisoryMatch>,
    pub environment_declarations: Vec<crate::environment::EnvironmentDeclaration>,
    pub toolchain_comparisons: Vec<crate::environment::ToolchainComparison>,
    pub capabilities: Vec<EnvironmentCapability>,
    /// 解析/扫描诊断（malformed/unsupported/unavailable，§37/§56）。
    pub diagnostics: Vec<String>,
}

/// 环境能力（§39：直接影响 M25/M29 分析完整性表达）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnvironmentCapability {
    pub name: String,
    pub state: CapabilityState,
    pub reason: String,
}

/// 能力状态（§39/§102）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityState {
    Available,
    Unavailable,
    Unsupported,
    Partial,
    Failed,
    Unknown,
}

impl SupplyReport {
    /// 版本对照（§11）：声明（含 CPM 解析出的版本）× 锁定版本。
    pub fn compare_versions(&mut self) {
        let mut lock_index: BTreeMap<&str, Vec<&ResolvedDependency>> = BTreeMap::new();
        for resolved in &self.resolved {
            lock_index
                .entry(resolved.id.as_str())
                .or_default()
                .push(resolved);
        }
        let mut comparisons = Vec::new();
        for declaration in &self.declarations {
            if declaration.kind == DependencyKind::ProjectReference {
                continue;
            }
            let Some(lockeds) = lock_index.get(declaration.id.as_str()) else {
                comparisons.push(VersionComparison {
                    declaration_id: declaration.id.clone(),
                    package_id: declaration.package_id.clone(),
                    state: DriftState::Unresolved,
                    detail: "declared but not present in any parsed lockfile".to_string(),
                });
                continue;
            };
            let Some(requested) = &declaration.requested_version else {
                comparisons.push(VersionComparison {
                    declaration_id: declaration.id.clone(),
                    package_id: declaration.package_id.clone(),
                    state: DriftState::Unknown,
                    detail: "no declared version; locked version(s) stand alone".to_string(),
                });
                continue;
            };
            for locked in lockeds {
                let Some(resolved_version) = &locked.resolved_version else {
                    continue;
                };
                let state = if crate::version::version_satisfies(resolved_version, requested) {
                    if declaration.version_is_range {
                        DriftState::NormalResolution
                    } else {
                        DriftState::Expected
                    }
                } else {
                    DriftState::Drift
                };
                comparisons.push(VersionComparison {
                    declaration_id: declaration.id.clone(),
                    package_id: declaration.package_id.clone(),
                    state,
                    detail: format!("declared `{requested}` vs resolved `{resolved_version}`"),
                });
            }
        }
        comparisons.sort_by(|a, b| {
            a.declaration_id
                .cmp(&b.declaration_id)
                .then_with(|| a.detail.cmp(&b.detail))
        });
        self.version_comparisons = comparisons;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn declaration(
        id: &str,
        ecosystem: Ecosystem,
        package: &str,
        version: Option<&str>,
        range: bool,
    ) -> DependencyDeclaration {
        DependencyDeclaration {
            id: id.to_string(),
            ecosystem,
            package_id: package.to_string(),
            requested_version: version.map(str::to_string),
            version_is_range: range,
            kind: DependencyKind::PackageReference,
            source: DependencySource::NuGet,
            scope: "direct".to_string(),
            manifest_path: "src/A.csproj".to_string(),
            line: Some(1),
            version_source: VersionSource::Manifest,
            origin: Origin::Fact,
            evidence: Vec::new(),
        }
    }

    fn resolved(
        id: &str,
        ecosystem: Ecosystem,
        package: &str,
        version: &str,
    ) -> ResolvedDependency {
        ResolvedDependency {
            id: id.to_string(),
            ecosystem,
            package_id: package.to_string(),
            resolved_version: Some(version.to_string()),
            direct: true,
            from_lockfile: true,
            lockfile_path: "src/packages.lock.json".to_string(),
            evidence: Vec::new(),
        }
    }

    #[test]
    fn range_resolution_is_normal_not_error() {
        // §11 示例：8.0.* → 8.0.11 是正常 range resolution。
        let mut report = SupplyReport {
            schema_version: 1,
            declarations: vec![declaration(
                "nuget:P",
                Ecosystem::NuGet,
                "P",
                Some("8.0.*"),
                true,
            )],
            resolved: vec![resolved("nuget:P", Ecosystem::NuGet, "P", "8.0.11")],
            ..SupplyReport::default()
        };
        report.compare_versions();
        assert_eq!(
            report.version_comparisons[0].state,
            DriftState::NormalResolution
        );
    }

    #[test]
    fn declared_but_unresolved_is_unresolved_not_zero() {
        let mut report = SupplyReport {
            schema_version: 1,
            declarations: vec![declaration(
                "nuget:Q",
                Ecosystem::NuGet,
                "Q",
                Some("1.0.0"),
                false,
            )],
            resolved: vec![],
            ..SupplyReport::default()
        };
        report.compare_versions();
        assert_eq!(report.version_comparisons[0].state, DriftState::Unresolved);
        assert_eq!(report.version_comparisons[0].package_id, "Q");
    }

    #[test]
    fn drift_when_outside_declaration() {
        let mut report = SupplyReport {
            schema_version: 1,
            declarations: vec![declaration(
                "nuget:R",
                Ecosystem::NuGet,
                "R",
                Some("1.0.0"),
                false,
            )],
            resolved: vec![resolved("nuget:R", Ecosystem::NuGet, "R", "2.0.0")],
            ..SupplyReport::default()
        };
        report.compare_versions();
        assert_eq!(report.version_comparisons[0].state, DriftState::Drift);
    }
}
