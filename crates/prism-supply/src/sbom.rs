//! SBOM 模型与导出（M29 §23-§25）。
//!
//! v1 = 明确版本化的自有格式 `prism-sbom/1`：确定性（无时间戳/UUID 进
//! 内容，§24）、可追溯（组件 ⇒ ResolvedDependency ⇒ lockfile ⇒ evidence，
//! §25）。SPDX / CycloneDX 明确 DEFER（§24：不因标准名存在就假设完成）。

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// SBOM 导出格式版本（导出内容 hash 的稳定性契约）。
pub const SBOM_FORMAT: &str = "prism-sbom";
pub const SBOM_FORMAT_VERSION: u32 = 1;

/// 组件（§23 内部表达：Component/Identity/Version/Source/License/关系）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SbomComponent {
    /// `{ecosystem}:{package_id}@{resolved_version}`（展示/引用形式；
    /// 结构化字段仍然分开，§14）。
    pub name: String,
    pub ecosystem: String,
    pub package_id: String,
    pub version: String,
    /// direct（声明存在）或 transitive。
    pub relationship: String,
    pub source: String,
    /// license：已知表达式或 "UNKNOWN"（§21 一等公民）。
    pub license: String,
    /// lockfile 路径（可追溯 §25）。
    pub evidence_ref: String,
}

/// SBOM 文档（BTreeMap + 排序数组 ⇒ 同输入同输出，§24/§95）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SbomDocument {
    pub format: String,
    pub format_version: u32,
    /// 生成器标识（不含时间戳——确定性内容，§24）。
    pub generator: String,
    /// 组件按 name 排序。
    pub components: Vec<SbomComponent>,
    /// 无 license 事实的组件数（UNKNOWN 是一等公民，不藏）。
    pub unknown_license_count: usize,
}

impl SbomDocument {
    /// 从解析态依赖 + license 事实构建（§25 追溯链）。
    pub fn from_resolved(
        resolved: &[crate::model::ResolvedDependency],
        licenses: &[crate::license::LicenseFact],
        generator: &str,
    ) -> Self {
        let license_index: BTreeMap<&str, &crate::license::LicenseFact> = licenses
            .iter()
            .map(|fact| (fact.subject.as_str(), fact))
            .collect();
        let mut components: Vec<SbomComponent> = Vec::new();
        let mut unknown_license_count = 0usize;
        for dependency in resolved {
            let Some(version) = &dependency.resolved_version else {
                continue;
            };
            let license = license_index.get(dependency.id.as_str());
            let license_expression = license
                .map(|fact| fact.expression.clone())
                .filter(|expression| !expression.is_empty())
                .unwrap_or_else(|| {
                    unknown_license_count += 1;
                    "UNKNOWN".to_string()
                });
            let package_id = dependency.package_id.as_str();
            let ecosystem_str = dependency.ecosystem.as_str();
            components.push(SbomComponent {
                name: format!("{ecosystem_str}:{package_id}@{version}"),
                ecosystem: dependency.ecosystem.as_str().to_string(),
                package_id: dependency.package_id.clone(),
                version: version.clone(),
                relationship: if dependency.direct {
                    "direct".into()
                } else {
                    "transitive".into()
                },
                source: format!("{:?}", dependency.ecosystem),
                license: license_expression,
                evidence_ref: dependency.lockfile_path.clone(),
            });
        }
        components.sort_by(|a, b| a.name.cmp(&b.name));
        SbomDocument {
            format: SBOM_FORMAT.to_string(),
            format_version: SBOM_FORMAT_VERSION,
            generator: generator.to_string(),
            components,
            unknown_license_count,
        }
    }

    /// 确定性 JSON 导出（同输入同输出，逐字节稳定）。
    pub fn to_deterministic_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dependency(package: &str, version: &str, direct: bool) -> crate::model::ResolvedDependency {
        crate::model::ResolvedDependency {
            id: format!("nuget:{package}"),
            ecosystem: crate::model::Ecosystem::NuGet,
            package_id: package.to_string(),
            resolved_version: Some(version.to_string()),
            direct,
            from_lockfile: true,
            lockfile_path: "packages.lock.json".to_string(),
            evidence: Vec::new(),
        }
    }

    #[test]
    fn deterministic_and_traceable_export() {
        let licenses = vec![crate::license::LicenseFact {
            subject: "nuget:Known.Pkg".to_string(),
            expression: "MIT".to_string(),
            spdx_known: true,
            source: crate::license::LicenseSource::Unknown,
            origin: prism_intel::meta::Origin::Fact,
            evidence: Vec::new(),
        }];
        let document = SbomDocument::from_resolved(
            &[
                dependency("Zeta.Pkg", "2.0.0", false),
                dependency("Known.Pkg", "1.0.0", true),
            ],
            &licenses,
            "prism-supply test",
        );
        assert_eq!(document.components.len(), 2);
        assert_eq!(
            document.components[0].name, "nuget:Known.Pkg@1.0.0",
            "按 name 排序"
        );
        assert_eq!(document.components[0].license, "MIT");
        assert_eq!(document.components[0].relationship, "direct");
        assert_eq!(
            document.components[0].evidence_ref, "packages.lock.json",
            "可追溯"
        );
        assert_eq!(document.unknown_license_count, 1, "UNKNOWN 计数可见");
        let json_a = document.to_deterministic_json();
        let json_b = document.to_deterministic_json();
        assert_eq!(json_a, json_b);
        assert!(!json_a.contains("2026-"), "无时间戳进内容（§24）");
    }
}
