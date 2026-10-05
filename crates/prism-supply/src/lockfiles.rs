//! 锁文件解析（M29 §12）：resolved 侧事实的唯一来源。
//!
//! 范围按真实生态裁剪（§12：不为"完整"强上所有生态）——.NET
//! packages.lock.json（serde）+ Cargo.lock（行扫）。npm/pnpm/yarn 锁
//! 文件 v1 只记录存在（M22 先例），解析为后续增量。

use crate::model::{Ecosystem, ResolvedDependency};
use serde::Deserialize;
use std::collections::BTreeMap;

/// packages.lock.json 单条（obj/project JSON 格式，§12 第一优先）。
#[derive(Debug, Deserialize)]
struct PackageLockFile {
    #[serde(default)]
    dependencies: BTreeMap<String, PackageLockEntry>,
}

#[derive(Debug, Deserialize)]
struct PackageLockEntry {
    #[serde(default)]
    #[serde(rename = "type")]
    kind: Option<String>,
}

/// 解析 packages.lock.json：键形如 `Newtonsoft.Json/13.0.3`。
/// malformed ⇒ Err 诊断（调用方如实记录，不当作空锁 §56）。
pub fn parse_package_lock_json(
    content: &str,
    lockfile_path: &str,
) -> Result<Vec<ResolvedDependency>, String> {
    let parsed: PackageLockFile =
        serde_json::from_str(content).map_err(|err| format!("{lockfile_path}: {err}"))?;
    let mut out = Vec::new();
    for (key, entry) in &parsed.dependencies {
        let Some((package_id, version)) = key.rsplit_once('/') else {
            continue;
        };
        let direct = entry
            .kind
            .as_deref()
            .map(|k| k.eq_ignore_ascii_case("Direct"))
            .unwrap_or(false);
        out.push(ResolvedDependency {
            id: format!("{}:{package_id}", Ecosystem::NuGet.as_str()),
            ecosystem: Ecosystem::NuGet,
            package_id: package_id.to_string(),
            resolved_version: Some(version.to_string()),
            direct,
            from_lockfile: true,
            lockfile_path: lockfile_path.to_string(),
            evidence: Vec::new(),
        });
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(out)
}

/// 解析 Cargo.lock：`[[package]]` 段的 name/version 行扫。
/// （Cargo.lock 含根包自身；这里原样返回，调用方按声明对照。）
pub fn parse_cargo_lock(content: &str, lockfile_path: &str) -> Vec<ResolvedDependency> {
    let mut out = Vec::new();
    let mut current_name: Option<String> = None;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "[[package]]" {
            // 新段开始：丢弃上一个未配到 version 的孤儿 name。
            current_name = None;
            continue;
        }
        if let Some(value) = trimmed.strip_prefix("name = ") {
            current_name = Some(value.trim_matches('"').to_string());
        } else if let Some(value) = trimmed.strip_prefix("version = ") {
            if let Some(name) = current_name.take() {
                out.push(ResolvedDependency {
                    id: format!("{}:{name}", Ecosystem::Cargo.as_str()),
                    ecosystem: Ecosystem::Cargo,
                    package_id: name,
                    resolved_version: Some(value.trim_matches('"').to_string()),
                    direct: false,
                    from_lockfile: true,
                    lockfile_path: lockfile_path.to_string(),
                    evidence: Vec::new(),
                });
            }
        }
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    out.dedup_by(|a, b| a.id == b.id);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_lock_json_direct_and_transitive() {
        let content = r#"{
  "version": 1,
  "dependencies": {
    "Newtonsoft.Json/13.0.3": { "type": "Direct" },
    "Serilog/4.0.0": { "type": "Transitive" }
  },
  "frameworks": {}
}"#;
        let resolved = parse_package_lock_json(content, "src/packages.lock.json").unwrap();
        assert_eq!(resolved.len(), 2);
        assert_eq!(resolved[0].package_id, "Newtonsoft.Json");
        assert_eq!(resolved[0].resolved_version.as_deref(), Some("13.0.3"));
        assert!(resolved[0].direct, "type=Direct ⇒ direct");
        assert!(!resolved[1].direct, "Transitive ⇒ 非 direct");
        assert!(resolved.iter().all(|r| r.from_lockfile));
    }

    #[test]
    fn package_lock_json_malformed_is_error() {
        let err = parse_package_lock_json("{ not json", "x/packages.lock.json");
        assert!(err.is_err(), "malformed 锁 ⇒ 诊断，不是空结果");
    }

    #[test]
    fn cargo_lock_name_version_pairs() {
        let content = "version = 3\n[[package]]\nname = \"serde\"\nversion = \"1.0.210\"\n[[package]]\nname = \"serde\"\nversion = \"1.0.210\"\n[[package]]\nname = \"syn\"\nversion = \"2.0.87\"\n";
        let resolved = parse_cargo_lock(content, "Cargo.lock");
        assert_eq!(resolved.len(), 2, "同包同版本去重");
        assert_eq!(resolved[0].package_id, "serde");
        assert_eq!(resolved[1].package_id, "syn");
    }
}
