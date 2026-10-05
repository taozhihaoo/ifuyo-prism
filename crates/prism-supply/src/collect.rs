//! 供给事实汇总（M29 §103 Vertical Slice 的事实层）。
//!
//! 输入 = 调用方有界读取后的文件内容映射 + 真实观测的工具版本 +
//! 可选 advisory provider；本模块零 IO（§2/§100：不重扫文件系统）。

use crate::advisory::{match_advisories, AdvisoryProvider};
use crate::csproj::{parse_csproj, parse_directory_packages_props};
use crate::environment::{compare_declared_observed, EnvironmentDeclaration};
use crate::license::{LicenseFact, LicenseSource};
use crate::lockfiles::{parse_cargo_lock, parse_package_lock_json};
use crate::manifests::{
    parse_cargo_toml, parse_global_json, parse_package_json, parse_project_godot_features,
};
use crate::model::{
    CapabilityState, DependencyDeclaration, DependencyKind, DependencySource, Ecosystem,
    EnvironmentCapability, ResolvedDependency, SupplyReport, VersionSource, SUPPLY_SCHEMA_VERSION,
};
use crate::secrets::{scan_secrets, SecretScanLimits};
use prism_intel::meta::Origin;
use std::collections::BTreeMap;

/// 汇总输入（§4：全部来自调用方注入，不自扫描）。
pub struct SupplyInputs<'a> {
    /// 项目相对路径 → 有界读取后的文本内容（调用方决定扫描面，§67）。
    pub files: &'a BTreeMap<String, String>,
    /// 源码/测试文本文件（secret 扫描面；可与 files 重叠）。
    pub text_files: &'a [(String, String)],
    /// 真实观测的工具版本（name → raw version；探测失败⇒空串，§38）。
    pub observed_tools: &'a BTreeMap<String, String>,
    /// advisory provider（None ⇒ Unavailable，§29）。
    pub advisory_provider: Option<&'a dyn AdvisoryProvider>,
    pub secret_limits: &'a SecretScanLimits,
}

/// 主入口：文件映射 → SupplyReport（确定性输出，§95）。
pub fn collect(inputs: &SupplyInputs) -> SupplyReport {
    let mut report = SupplyReport {
        schema_version: SUPPLY_SCHEMA_VERSION,
        ..Default::default()
    };
    let mut central_versions: BTreeMap<String, String> = BTreeMap::new();
    let mut declarations: Vec<DependencyDeclaration> = Vec::new();
    let mut resolved: Vec<ResolvedDependency> = Vec::new();
    let mut environment_declarations: Vec<EnvironmentDeclaration> = Vec::new();
    let mut licenses: Vec<LicenseFact> = Vec::new();

    for (path, content) in inputs.files {
        let lower = path.to_ascii_lowercase();
        if lower.ends_with(".csproj") {
            let project = parse_csproj(content, path);
            report.diagnostics.extend(project.diagnostics.clone());
            if let Some(framework) = &project.target_framework {
                environment_declarations.push(EnvironmentDeclaration {
                    tool: "dotnet".to_string(),
                    constraint: framework.clone(),
                    source_path: path.clone(),
                    line: None,
                });
            }
            for reference in &project.package_references {
                declarations.push(DependencyDeclaration {
                    id: format!("{}:{}", Ecosystem::NuGet.as_str(), reference.include),
                    ecosystem: Ecosystem::NuGet,
                    package_id: reference.include.clone(),
                    requested_version: reference.version.clone(),
                    version_is_range: reference
                        .version
                        .as_deref()
                        .map(|v| v.contains('*') || v.contains(','))
                        .unwrap_or(false),
                    kind: DependencyKind::PackageReference,
                    source: DependencySource::NuGet,
                    scope: "direct".to_string(),
                    manifest_path: path.clone(),
                    line: Some(reference.line),
                    version_source: if reference.version.is_some() {
                        VersionSource::Manifest
                    } else {
                        VersionSource::Unspecified
                    },
                    origin: Origin::Fact,
                    evidence: Vec::new(),
                });
            }
            for reference in &project.project_references {
                let normalized = normalize_project_path(&manifest_dir(path), &reference.include);
                declarations.push(DependencyDeclaration {
                    id: format!("project:{normalized}"),
                    ecosystem: Ecosystem::Unknown,
                    package_id: normalized.clone(),
                    requested_version: None,
                    version_is_range: false,
                    kind: DependencyKind::ProjectReference,
                    source: DependencySource::Project,
                    scope: "direct".to_string(),
                    manifest_path: path.clone(),
                    line: Some(reference.line),
                    version_source: VersionSource::Unspecified,
                    origin: Origin::Fact,
                    evidence: Vec::new(),
                });
                report
                    .project_references
                    .push(crate::model::ProjectReferenceEdge {
                        from_project: path.clone(),
                        to_project: normalized,
                        manifest_path: path.clone(),
                    });
            }
        } else if lower.ends_with("directory.packages.props") {
            let central = parse_directory_packages_props(content, path);
            report.diagnostics.extend(central.diagnostics.clone());
            if central.manage_centrally {
                for version in &central.package_versions {
                    central_versions.insert(version.include.clone(), version.version.clone());
                }
            } else if !central.package_versions.is_empty() {
                report.diagnostics.push(format!(
                    "{path}: PackageVersion entries present but ManagePackageVersionsCentrally is not true; not treating as central versions"
                ));
            }
        } else if lower.ends_with("global.json") {
            match parse_global_json(content) {
                Ok(sdk) => {
                    if let Some(version) = sdk.version {
                        environment_declarations.push(EnvironmentDeclaration {
                            tool: "dotnet".to_string(),
                            constraint: version,
                            source_path: path.clone(),
                            line: None,
                        });
                    }
                }
                Err(err) => report.diagnostics.push(err),
            }
        } else if lower.ends_with("project.godot") {
            if let Some((hint, line)) = parse_project_godot_features(content) {
                // §84：声明提示——机器版本必须来自真实观测。
                environment_declarations.push(EnvironmentDeclaration {
                    tool: "godot".to_string(),
                    constraint: hint,
                    source_path: path.clone(),
                    line: Some(line),
                });
            }
        } else if lower.ends_with("packages.lock.json") {
            match parse_package_lock_json(content, path) {
                Ok(mut locked) => resolved.append(&mut locked),
                Err(err) => report.diagnostics.push(err),
            }
        } else if lower.ends_with("cargo.lock") {
            resolved.append(&mut parse_cargo_lock(content, path));
        } else if lower.ends_with("cargo.toml") {
            for (name, version, is_dev) in parse_cargo_toml(content) {
                declarations.push(DependencyDeclaration {
                    id: format!("{}:{name}", Ecosystem::Cargo.as_str()),
                    ecosystem: Ecosystem::Cargo,
                    package_id: name.clone(),
                    requested_version: version,
                    version_is_range: false,
                    kind: if is_dev {
                        DependencyKind::DevelopmentDependency
                    } else {
                        DependencyKind::PackageReference
                    },
                    source: DependencySource::Registry,
                    scope: if is_dev {
                        "development".to_string()
                    } else {
                        "direct".to_string()
                    },
                    manifest_path: path.clone(),
                    line: None,
                    version_source: VersionSource::Manifest,
                    origin: Origin::Fact,
                    evidence: Vec::new(),
                });
            }
            if let Some(license) = extract_cargo_license(content) {
                licenses.push(LicenseFact {
                    subject: "project".to_string(),
                    expression: license,
                    spdx_known: false,
                    source: LicenseSource::Manifest,
                    origin: Origin::Fact,
                    evidence: Vec::new(),
                });
            }
        } else if lower.ends_with("package.json") {
            match parse_package_json(content) {
                Ok(parsed) => {
                    for (name, version, is_dev) in parsed.dependencies {
                        declarations.push(DependencyDeclaration {
                            id: format!("{}:{name}", Ecosystem::Npm.as_str()),
                            ecosystem: Ecosystem::Npm,
                            package_id: name.clone(),
                            requested_version: version,
                            version_is_range: true,
                            kind: if is_dev {
                                DependencyKind::DevelopmentDependency
                            } else {
                                DependencyKind::PackageReference
                            },
                            source: DependencySource::Registry,
                            scope: if is_dev {
                                "development".to_string()
                            } else {
                                "direct".to_string()
                            },
                            manifest_path: path.clone(),
                            line: None,
                            version_source: VersionSource::Manifest,
                            origin: Origin::Fact,
                            evidence: Vec::new(),
                        });
                    }
                    if let Some(license) = parsed.license {
                        licenses.push(LicenseFact {
                            subject: "project".to_string(),
                            expression: license,
                            // SPDX 判定统一在下方补全轮（表达式已定后）。
                            spdx_known: false,
                            source: LicenseSource::Manifest,
                            origin: Origin::Fact,
                            evidence: Vec::new(),
                        });
                    }
                }
                Err(err) => report.diagnostics.push(err),
            }
        }
    }

    // CPM（§13）：无内联版本的 PackageReference 用中央版本补全。
    for declaration in &mut declarations {
        if declaration.version_source == VersionSource::Unspecified {
            if let Some(central) = central_versions.get(&declaration.package_id) {
                declaration.requested_version = Some(central.clone());
                declaration.version_is_range = central.contains('*') || central.contains(',');
                declaration.version_source = VersionSource::Central;
            }
        }
    }

    // license SPDX 判定补全（前面内联时 subject 未定）。
    for license in &mut licenses {
        license.spdx_known = crate::license::is_known_spdx(&license.expression);
    }

    // secret 扫描（§67：扫描面由调用方决定）。
    let scan = scan_secrets(inputs.text_files, inputs.secret_limits);
    report.secret_scan_truncated = scan.truncated;
    report.diagnostics.extend(scan.diagnostics);
    report.secret_candidates = scan.candidates;

    // advisory 匹配（§29/§105：不可用 ⇒ Unavailable，不是零）。
    match inputs.advisory_provider {
        Some(provider) => {
            let (capability, database_version, matches) = match_advisories(provider, &resolved);
            report.advisory_capability = Some(capability);
            report.advisory_database_version = database_version;
            report.advisory_matches = matches;
        }
        None => {
            report.advisory_capability = Some(crate::model::ProviderCapability::Unavailable);
            report.diagnostics.push(
                "vulnerability analysis unavailable: no advisory database provided".to_string(),
            );
        }
    }

    // 去重排序（确定性 §95）。
    declarations.sort_by(|a, b| {
        a.id.cmp(&b.id)
            .then_with(|| a.manifest_path.cmp(&b.manifest_path))
    });
    declarations.dedup_by(|a, b| a.id == b.id && a.manifest_path == b.manifest_path);
    resolved.sort_by(|a, b| {
        a.id.cmp(&b.id)
            .then_with(|| a.lockfile_path.cmp(&b.lockfile_path))
    });
    resolved.dedup_by(|a, b| a.id == b.id && a.resolved_version == b.resolved_version);
    environment_declarations.sort_by(|a, b| {
        a.tool
            .cmp(&b.tool)
            .then_with(|| a.source_path.cmp(&b.source_path))
    });

    report.declarations = declarations;
    report.resolved = resolved;
    report.licenses = licenses;
    report.environment_declarations = environment_declarations;
    report.cycles = detect_cycles(&report.project_references);
    report.compare_versions();
    report.toolchain_comparisons =
        compare_declared_observed(&report.environment_declarations, inputs.observed_tools);
    report.capabilities = derive_capabilities(&report);
    report
}

fn derive_capabilities(report: &SupplyReport) -> Vec<EnvironmentCapability> {
    let lock_count = report.resolved.len();
    let has_dotnet_project = report
        .declarations
        .iter()
        .any(|d| d.ecosystem == Ecosystem::NuGet || d.kind == DependencyKind::ProjectReference);
    vec![
        EnvironmentCapability {
            name: "CanParse".to_string(),
            state: if report.diagnostics.is_empty() {
                CapabilityState::Available
            } else {
                CapabilityState::Partial
            },
            reason: format!(
                "manifest parsing completed with {} diagnostic(s)",
                report.diagnostics.len()
            ),
        },
        EnvironmentCapability {
            name: "CanResolveDependencies".to_string(),
            state: if lock_count > 0 {
                CapabilityState::Available
            } else if has_dotnet_project {
                CapabilityState::Unavailable
            } else {
                CapabilityState::Unknown
            },
            reason: format!("{lock_count} resolved entries from parsed lockfiles"),
        },
        EnvironmentCapability {
            name: "CanCompile".to_string(),
            state: CapabilityState::Unavailable,
            reason: "no build integration in this Prism version; capability is honest, not assumed"
                .to_string(),
        },
        EnvironmentCapability {
            name: "CanGenerateSBOM".to_string(),
            state: if lock_count > 0 {
                CapabilityState::Available
            } else {
                CapabilityState::Unknown
            },
            reason: format!("{lock_count} components available for SBOM"),
        },
        EnvironmentCapability {
            name: "VulnerabilityAnalysis".to_string(),
            state: match report.advisory_capability {
                Some(crate::model::ProviderCapability::Supported) => CapabilityState::Available,
                Some(crate::model::ProviderCapability::Unavailable) => CapabilityState::Unavailable,
                _ => CapabilityState::Unknown,
            },
            reason: report
                .advisory_database_version
                .clone()
                .unwrap_or_else(|| "no advisory database provided".to_string()),
        },
    ]
}

/// Cargo.toml `[package] license = "..."` 行扫（无该字段 ⇒ None，
/// §21：不等于"没有 license"，只是无此事实）。
fn extract_cargo_license(content: &str) -> Option<String> {
    let mut in_package = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_package = trimmed == "[package]";
            continue;
        }
        if in_package {
            if let Some(rest) = trimmed.strip_prefix("license ") {
                let rest = rest.trim_start();
                if let Some(value) = rest.strip_prefix('=') {
                    let value = value.trim().trim_matches('"');
                    if !value.is_empty() {
                        return Some(value.to_string());
                    }
                }
            }
        }
    }
    None
}

/// manifest 目录（'/' 分隔）。
fn manifest_dir(path: &str) -> String {
    match path.rfind('/') {
        Some(at) => path[..at].to_string(),
        None => String::new(),
    }
}

/// include 路径（可能 `\` 分隔、含 `..`）⇒ 项目相对规范路径。
fn normalize_project_path(directory: &str, include: &str) -> String {
    let mut parts: Vec<&str> = if directory.is_empty() {
        Vec::new()
    } else {
        directory.split('/').collect()
    };
    for segment in include.split(['\\', '/']) {
        match segment {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            other => parts.push(other),
        }
    }
    parts.join("/")
}

/// 项目引用环检测（§16：DFS + 路径证据；cap 10 个环 / 深度 32 防爆）。
fn detect_cycles(
    edges: &[crate::model::ProjectReferenceEdge],
) -> Vec<crate::model::DependencyCycle> {
    let mut adjacency: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for edge in edges {
        adjacency
            .entry(edge.from_project.as_str())
            .or_default()
            .push(edge.to_project.as_str());
    }
    let mut cycles = Vec::new();
    let mut remaining = 10usize;
    for root in adjacency.keys().copied().collect::<Vec<_>>() {
        if remaining == 0 {
            break;
        }
        let mut path: Vec<&str> = vec![root];
        let mut on_path: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
        on_path.insert(root);
        collect_cycles(
            root,
            &adjacency,
            &mut path,
            &mut on_path,
            &mut cycles,
            &mut remaining,
        );
    }
    // 环的旋转等价归一（同一环从不同起点发现 ⇒ 旋转到最小元素开头）。
    for cycle in &mut cycles {
        let len = cycle.path.len();
        if len < 2 {
            continue;
        }
        let min_at = (0..len - 1)
            .min_by(|ia, ib| cycle.path[*ia].cmp(&cycle.path[*ib]).then(ia.cmp(ib)))
            .unwrap_or(0);
        if min_at > 0 {
            // 旋转到最小元素开头后，闭合节点 = 旋转后的首元素（环语义）。
            let mut body: Vec<String> = cycle.path[..len - 1].to_vec();
            body.rotate_left(min_at);
            let closing = body[0].clone();
            body.push(closing);
            cycle.path = body;
        }
    }
    cycles.sort_by(|a, b| a.path.cmp(&b.path));
    cycles.dedup();
    cycles
}

fn collect_cycles<'a>(
    node: &'a str,
    adjacency: &BTreeMap<&'a str, Vec<&'a str>>,
    path: &mut Vec<&'a str>,
    on_path: &mut std::collections::BTreeSet<&'a str>,
    cycles: &mut Vec<crate::model::DependencyCycle>,
    remaining: &mut usize,
) {
    if *remaining == 0 || path.len() > 32 {
        return;
    }
    if let Some(neighbors) = adjacency.get(node) {
        for &next in neighbors {
            if on_path.contains(next) {
                // 找到环：截取 path 从 next 起点到 node，再闭合到 next。
                if let Some(start) = path.iter().position(|p| *p == next) {
                    let mut cycle_path: Vec<String> =
                        path[start..].iter().map(|s| (*s).to_string()).collect();
                    cycle_path.push(next.to_string());
                    cycles.push(crate::model::DependencyCycle { path: cycle_path });
                    *remaining -= 1;
                }
                continue;
            }
            path.push(next);
            on_path.insert(next);
            collect_cycles(next, adjacency, path, on_path, cycles, remaining);
            on_path.remove(next);
            path.pop();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::environment::ToolchainState;
    use crate::model::DriftState;
    use crate::secrets::SecretScanLimits;

    fn map(entries: &[(&str, String)]) -> BTreeMap<String, String> {
        entries
            .iter()
            .map(|(k, v)| ((*k).to_string(), v.clone()))
            .collect()
    }

    fn inputs<'a>(
        files: &'a BTreeMap<String, String>,
        observed: &'a BTreeMap<String, String>,
    ) -> SupplyInputs<'a> {
        SupplyInputs {
            files,
            text_files: &[],
            observed_tools: observed,
            advisory_provider: None,
            secret_limits: Box::leak(Box::new(SecretScanLimits::default())),
        }
    }

    #[test]
    fn godot_csharp_vertical_slice_facts() {
        // §83 Godot+C# 链路：csproj + CPM + lockfile + global.json + project.godot。
        let files = map(&[
            (
                "src/App/App.csproj",
                "<Project Sdk=\"Microsoft.NET.Sdk\">\n<TargetFramework>net8.0</TargetFramework>\n<PackageReference Include=\"Newtonsoft.Json\" />\n<PackageReference Include=\"Vulnerable.Pkg\" />\n<ProjectReference Include=\"..\\Lib\\Lib.csproj\" />\n</Project>".to_string(),
            ),
            (
                "Directory.Packages.props",
                "<Project><PropertyGroup><ManagePackageVersionsCentrally>true</ManagePackageVersionsCentrally></PropertyGroup><ItemGroup><PackageVersion Include=\"Newtonsoft.Json\" Version=\"13.0.3\" /></ItemGroup></Project>".to_string(),
            ),
            (
                "src/Lib/packages.lock.json",
                r#"{"version":1,"dependencies":{"Newtonsoft.Json/13.0.3":{"type":"Direct"},"Vulnerable.Pkg/1.2.3":{"type":"Direct"}}}"#.to_string(),
            ),
            ("global.json", r#"{ "sdk": { "version": "8.0.100" } }"#.to_string()),
            ("project.godot", "config/features=PackedStringArray(\"4.3\", \"C#\")\n".to_string()),
        ]);
        let mut observed = BTreeMap::new();
        observed.insert("dotnet".to_string(), "8.0.404".to_string());
        let report = collect(&inputs(&files, &observed));

        assert_eq!(
            report.declarations.len(),
            3,
            "2 PackageReference + 1 ProjectReference"
        );
        let newtonsoft = report
            .declarations
            .iter()
            .find(|d| d.package_id == "Newtonsoft.Json")
            .unwrap();
        assert_eq!(
            newtonsoft.requested_version.as_deref(),
            Some("13.0.3"),
            "CPM 补全"
        );
        assert_eq!(newtonsoft.version_source, VersionSource::Central);
        assert_eq!(report.resolved.len(), 2);
        assert_eq!(report.project_references.len(), 1);
        assert_eq!(
            report.project_references[0].to_project, "src/Lib/Lib.csproj",
            "include ..\\ 路径规范化"
        );
        // 声明 × 解析：CPM 13.0.3 vs 锁定 13.0.3 ⇒ Expected。
        let comparison = report
            .version_comparisons
            .iter()
            .find(|c| c.package_id == "Newtonsoft.Json")
            .unwrap();
        assert_eq!(comparison.state, DriftState::Expected);
        // 声明环境：global.json 8.0.100 + csproj net8.0 + godot 4.3。
        assert_eq!(report.environment_declarations.len(), 3);
        let godot = report
            .environment_declarations
            .iter()
            .find(|d| d.tool == "godot")
            .unwrap();
        assert_eq!(godot.constraint, "4.3");
        // dotnet 观测 8.0.404 满足声明 8.0.100（同 major ⇒ Match/RangeSatisfied）。
        let dotnet = report
            .toolchain_comparisons
            .iter()
            .find(|c| c.tool == "dotnet")
            .unwrap();
        assert!(
            dotnet.state == ToolchainState::Match || dotnet.state == ToolchainState::RangeSatisfied,
            "8.0.100 声明 vs 8.0.404 观测 ⇒ 满足，实际 {:?}",
            dotnet.state
        );
        // advisory 缺省 ⇒ Unavailable + 诊断（不伪造 0）。
        assert_eq!(
            report.advisory_capability,
            Some(crate::model::ProviderCapability::Unavailable)
        );
        assert!(report
            .diagnostics
            .iter()
            .any(|d| d.contains("vulnerability analysis unavailable")));
    }

    #[test]
    fn project_reference_cycle_detected_with_path() {
        let files = map(&[
            (
                "A/A.csproj",
                "<Project><ProjectReference Include=\"..\\B\\B.csproj\" /></Project>".to_string(),
            ),
            (
                "B/B.csproj",
                "<Project><ProjectReference Include=\"..\\C\\C.csproj\" /></Project>".to_string(),
            ),
            (
                "C/C.csproj",
                "<Project><ProjectReference Include=\"..\\A\\A.csproj\" /></Project>".to_string(),
            ),
        ]);
        let observed = BTreeMap::new();
        let report = collect(&inputs(&files, &observed));
        assert_eq!(report.cycles.len(), 1, "A→B→C→A");
        let cycle = &report.cycles[0].path;
        assert_eq!(cycle.first(), cycle.last(), "环路径首尾相同");
        assert_eq!(cycle.len(), 4, "A,B,C,A");
    }

    #[test]
    fn secret_scan_integrated_with_redaction() {
        let files = BTreeMap::new();
        let observed = BTreeMap::new();
        let text_files = vec![(
            "src/creds.cs".to_string(),
            "var k = \"AKIAABCDEFGHIJKLMNOP\";".to_string(),
        )];
        let mut supply_inputs = inputs(&files, &observed);
        supply_inputs.text_files = &text_files;
        let report = collect(&supply_inputs);
        assert_eq!(report.secret_candidates.len(), 1);
        assert!(
            !report.secret_candidates[0]
                .redacted_context
                .contains("ABCDEFGHIJKLMNOP"),
            "候选不含原文（§35）"
        );
    }

    #[test]
    fn npm_and_cargo_declarations_with_project_license() {
        let files = map(&[
            ("package.json", r#"{"name":"a","license":"MIT","dependencies":{"react":"^18.2.0"}}"#.to_string()),
            ("Cargo.toml", "[package]\nlicense = \"Apache-2.0\"\n\n[dependencies]\nserde = \"1.0\"\n[dev-dependencies]\ntempfile = \"3.0\"\n".to_string()),
        ]);
        let observed = BTreeMap::new();
        let report = collect(&inputs(&files, &observed));
        assert_eq!(
            report
                .declarations
                .iter()
                .filter(|d| d.ecosystem == Ecosystem::Npm)
                .count(),
            1
        );
        assert_eq!(
            report
                .declarations
                .iter()
                .filter(|d| d.kind == DependencyKind::DevelopmentDependency)
                .count(),
            1,
            "cargo dev（npm fixture 无 devDependencies）"
        );
        // Cargo.toml 与 package.json 都声明项目 license ⇒ 2 条事实（同 subject）。
        assert_eq!(report.licenses.len(), 2);
        assert!(report.licenses.iter().all(|l| l.spdx_known));
    }

    #[test]
    fn resolution_state_unresolved_when_no_lockfile() {
        let files = map(&[(
            "App/App.csproj",
            "<Project><PackageReference Include=\"Newtonsoft.Json\" Version=\"13.0.3\" /></Project>".to_string(),
        )]);
        let observed = BTreeMap::new();
        let report = collect(&inputs(&files, &observed));
        assert_eq!(report.resolved.len(), 0);
        let comparison = &report.version_comparisons[0];
        assert_eq!(comparison.state, DriftState::Unresolved);
        // 报告级解析状态：无锁 ⇒ 解析能力 Unavailable（不是 Resolved）。
        assert_eq!(
            report
                .capabilities
                .iter()
                .find(|c| c.name == "CanResolveDependencies")
                .unwrap()
                .state,
            CapabilityState::Unavailable
        );
    }
}
