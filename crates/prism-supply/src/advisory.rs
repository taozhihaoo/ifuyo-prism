//! Advisory 数据源（M29 §28-§30/§73）。
//!
//! offline-first（§29/§72）：默认本地 JSON 数据库（显式加载）；
//! Provider 不可用 ⇒ Vulnerability analysis = Unavailable，
//! **绝不等于 0 vulnerabilities**（§105）。Core 不依赖 HTTP。

use crate::model::ProviderCapability;
use crate::version;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// 一条 advisory（§30：只存匹配所需最小字段，不大段复制描述）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Advisory {
    /// ADV/CVE/GHSA id 原样。
    pub id: String,
    /// 来源标识（如 "local-db" / 导入库名）。
    pub source: String,
    /// 生态（nuget/npm/cargo/...，与 Ecosystem::as_str 同词表）。
    pub ecosystem: String,
    pub package_id: String,
    /// 受影响范围原样（`>=1.0,<1.2.5` / `1.2.3` / `2.*`）。
    pub affected_ranges: Vec<String>,
    /// advisory 明确给出的 safe version（§80：有才推荐，不伪造）。
    pub fixed_version: Option<String>,
    /// advisory 侧严重度提示（映射规则在 finding 层，不直接当 Priority）。
    pub severity_hint: String,
}

/// advisory 匹配结论（§30 示例形态）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdvisoryMatch {
    pub advisory: Advisory,
    pub resolved_version: String,
    /// "Potentially affected — resolved version intersects affected range"。
    pub conclusion: String,
}

/// Advisory 数据源边界（§73：未来可加远程 provider，Core 不见 HTTP）。
pub trait AdvisoryProvider {
    fn capability(&self) -> ProviderCapability;
    fn database_version(&self) -> Option<String>;
    /// 该 (ecosystem, package) 的已知 advisory 列表。
    fn advisories_for(&self, ecosystem: &str, package_id: &str) -> Vec<Advisory>;
}

/// 本地 JSON advisory 库（§29 Imported/Cached advisory data）。
/// 文件形如：{ "version": "...", "advisories": [ { id, source, ecosystem,
/// package_id, affected_ranges, fixed_version, severity_hint } ] }。
#[derive(Debug, Deserialize)]
pub struct LocalAdvisoryDatabase {
    version: Option<String>,
    #[serde(default)]
    advisories: Vec<Advisory>,
    /// 运行期构建的倒排索引（不参与反序列化）。
    #[serde(skip)]
    index: BTreeMap<String, Vec<usize>>,
}

impl LocalAdvisoryDatabase {
    pub fn load(content: &str) -> Result<Self, String> {
        let mut parsed: LocalAdvisoryDatabase =
            serde_json::from_str(content).map_err(|err| format!("advisory database: {err}"))?;
        parsed.index = BTreeMap::new();
        for (position, advisory) in parsed.advisories.iter().enumerate() {
            parsed
                .index
                .entry(format!("{}:{}", advisory.ecosystem, advisory.package_id))
                .or_default()
                .push(position);
        }
        Ok(parsed)
    }
}

impl AdvisoryProvider for LocalAdvisoryDatabase {
    fn capability(&self) -> ProviderCapability {
        ProviderCapability::Supported
    }

    fn database_version(&self) -> Option<String> {
        self.version.clone()
    }

    fn advisories_for(&self, ecosystem: &str, package_id: &str) -> Vec<Advisory> {
        self.index
            .get(&format!("{ecosystem}:{package_id}"))
            .map(|positions| {
                positions
                    .iter()
                    .map(|&p| self.advisories[p].clone())
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// Provider 不可用时的统一表达（§48：规则前置缺失 ⇒ 诚实跳过）。
pub struct UnavailableAdvisoryProvider {
    pub reason: String,
}

impl AdvisoryProvider for UnavailableAdvisoryProvider {
    fn capability(&self) -> ProviderCapability {
        ProviderCapability::Unavailable
    }
    fn database_version(&self) -> Option<String> {
        None
    }
    fn advisories_for(&self, _ecosystem: &str, _package_id: &str) -> Vec<Advisory> {
        Vec::new()
    }
}

/// 解析态依赖 × advisory 匹配（§30：范围相交 ⇒ Potentially affected）。
pub fn match_advisories(
    provider: &dyn AdvisoryProvider,
    resolved: &[crate::model::ResolvedDependency],
) -> (ProviderCapability, Option<String>, Vec<AdvisoryMatch>) {
    let capability = provider.capability();
    let database_version = provider.database_version();
    if capability != ProviderCapability::Supported {
        return (capability, database_version, Vec::new());
    }
    let mut matches = Vec::new();
    for dependency in resolved {
        let Some(resolved_version) = &dependency.resolved_version else {
            continue;
        };
        for advisory in
            provider.advisories_for(dependency.ecosystem.as_str(), &dependency.package_id)
        {
            if advisory.affected_ranges.is_empty() {
                continue;
            }
            let affected = advisory
                .affected_ranges
                .iter()
                .any(|range| version::version_satisfies(resolved_version, range));
            if affected {
                matches.push(AdvisoryMatch {
                    conclusion: "Potentially affected — resolved version intersects affected range"
                        .to_string(),
                    advisory,
                    resolved_version: resolved_version.clone(),
                });
            }
        }
    }
    matches.sort_by(|a, b| a.advisory.id.cmp(&b.advisory.id));
    (capability, database_version, matches)
}

#[cfg(test)]
mod tests {
    use super::*;

    const DB: &str = r#"{
  "version": "test-db-1",
  "advisories": [
    {
      "id": "ADV-0001",
      "source": "test-db",
      "ecosystem": "nuget",
      "package_id": "Vulnerable.Pkg",
      "affected_ranges": [">=1.0.0,<1.2.5"],
      "fixed_version": "1.2.5",
      "severity_hint": "high"
    },
    {
      "id": "ADV-0002",
      "source": "test-db",
      "ecosystem": "nuget",
      "package_id": "Other.Pkg",
      "affected_ranges": ["2.*"],
      "fixed_version": null,
      "severity_hint": "medium"
    }
  ]
}"#;

    fn resolved(id: &str, package: &str, version: &str) -> crate::model::ResolvedDependency {
        crate::model::ResolvedDependency {
            id: format!("nuget:{id}"),
            ecosystem: crate::model::Ecosystem::NuGet,
            package_id: package.to_string(),
            resolved_version: Some(version.to_string()),
            direct: true,
            from_lockfile: true,
            lockfile_path: "packages.lock.json".to_string(),
            evidence: Vec::new(),
        }
    }

    #[test]
    fn affected_range_intersects_to_match() {
        let database = LocalAdvisoryDatabase::load(DB).unwrap();
        let (capability, version, matches) = match_advisories(
            &database,
            &[resolved("Vulnerable.Pkg", "Vulnerable.Pkg", "1.2.3")],
        );
        assert_eq!(capability, ProviderCapability::Supported);
        assert_eq!(version.as_deref(), Some("test-db-1"));
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].advisory.id, "ADV-0001");
        assert!(matches[0].conclusion.contains("Potentially affected"));
    }

    #[test]
    fn fixed_version_outside_range_does_not_match() {
        let database = LocalAdvisoryDatabase::load(DB).unwrap();
        let (_, _, matches) = match_advisories(
            &database,
            &[resolved("Vulnerable.Pkg", "Vulnerable.Pkg", "1.2.5")],
        );
        assert!(matches.is_empty(), "已修复版本不命中");
    }

    #[test]
    fn unavailable_provider_is_not_zero_vulnerabilities() {
        // §105：库不可用 ⇒ capability Unavailable（规则层据此跳过并记诊断），
        // 绝不产出"0 vulnerabilities"。
        let unavailable = UnavailableAdvisoryProvider {
            reason: "no advisory db provided".into(),
        };
        let (capability, version, matches) = match_advisories(&unavailable, &[]);
        assert_eq!(capability, ProviderCapability::Unavailable);
        assert!(version.is_none());
        assert!(matches.is_empty());
    }
}
