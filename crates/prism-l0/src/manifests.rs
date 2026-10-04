//! Manifest 检测与依赖声明抽取（2.0-M22 §26-§28/§76-§77）。
//!
//! 边界：只做 discovery + 基础结构解析 + 声明抽取；
//! declared ≠ resolved（§27/§77）——relation 一律指向 External 包实体，
//! 不宣称解析后的依赖图已建立。锁文件只记录存在。

use crate::options::L0Options;
use prism_intel::entity::{Entity, EntityLocation, EntityScope};
use prism_intel::evidence::{Evidence, EvidenceKind};
use prism_intel::fact::{Fact, FactType, FactValue};
use prism_intel::identity::EntityKind;
use prism_intel::meta::{Confidence, ConfidenceReason, Freshness, Origin, Provenance};
use prism_intel::relation::{Relation, RelationTarget, RelationType};
use prism_intel::IntelBundle;
use prism_project::FileInfo;
use std::path::Path;

/// 单条依赖声明（§28 模型）。
struct DeclaredDependency {
    name: String,
    version_constraint: Option<String>,
    scope: &'static str,
    /// 证据行号（Cargo.toml/requirements.txt 有；package.json 无）。
    line: Option<usize>,
}

/// 支持声明抽取的 manifest（其余仅记录存在）。
const DECLARATION_MANIFESTS: [&str; 3] = ["package.json", "Cargo.toml", "requirements.txt"];
/// 锁文件（§77：只记录存在，不宣称 resolved graph）。
const LOCK_FILES: [&str; 5] = [
    "package-lock.json",
    "pnpm-lock.yaml",
    "yarn.lock",
    "Cargo.lock",
    "packages.lock.json",
];

pub fn collect_manifests(
    bundle: &mut IntelBundle,
    project_entity_id: &prism_intel::EntityId,
    files: &[FileInfo],
    options: &L0Options,
    at: &str,
    provenance: &Provenance,
) -> Vec<String> {
    let mut warnings = Vec::new();
    let root = Path::new(&bundle.project.root).to_path_buf();

    let mut manifests_seen = 0usize;
    for file in files {
        let name = file.file_name.as_str();
        let is_manifest = DECLARATION_MANIFESTS.contains(&name)
            || name == "pyproject.toml"
            || LOCK_FILES.contains(&name);
        if !is_manifest {
            continue;
        }
        manifests_seen += 1;

        // has_manifest 关系（存在性 = FACT）；§85：路径已被 file 实体
        // 覆盖时复用该实体，不建第二实体。
        let file_id = bundle.entity_id(&EntityKind::file(), &file.relative_path);
        let target = if bundle.entities.contains_key(&file_id) {
            file_id
        } else {
            let manifest_id = bundle.entity_id(&EntityKind::configuration(), &file.relative_path);
            bundle.insert_entity(Entity::new(
                manifest_id.clone(),
                EntityKind::configuration(),
                name.to_string(),
                EntityLocation::FileOnly {
                    path: file.relative_path.clone(),
                },
                EntityScope::Internal,
            ));
            manifest_id
        };
        bundle.insert_relation(Relation::new(
            RelationType::other("has_manifest"),
            project_entity_id.clone(),
            RelationTarget::Resolved { entity: target },
            Origin::Fact,
            Confidence::High,
            vec![],
        ));

        if LOCK_FILES.contains(&name) {
            bundle.insert_fact(Fact::new(
                FactType::other("lock_file_present"),
                Some(project_entity_id.clone()),
                FactValue::Str(file.relative_path.clone()),
                Origin::Fact,
                Confidence::High,
                ConfidenceReason::direct_observation(),
                Freshness::Fresh,
                vec![],
                at,
                provenance.clone(),
            ));
            continue;
        }

        // 大小上限（§72）：超过只记录存在。
        if file.size_bytes > options.max_manifest_bytes {
            warnings.push(format!(
                "manifest {} too large ({} bytes); existence recorded only",
                file.relative_path, file.size_bytes
            ));
            continue;
        }

        let absolute = root.join(&file.relative_path);
        let content = match std::fs::read_to_string(&absolute) {
            Ok(content) => content,
            Err(err) => {
                // 失败隔离（§56）：单 manifest 读取失败 ⇒ 诊断 + 继续。
                warnings.push(format!("manifest {} unreadable: {err}", file.relative_path));
                continue;
            }
        };

        let declarations = match name {
            "package.json" => match parse_package_json(&content) {
                Ok(declarations) => declarations,
                Err(reason) => {
                    warnings.push(format!(
                        "manifest {} malformed ({reason}); existence recorded only",
                        file.relative_path
                    ));
                    continue;
                }
            },
            "Cargo.toml" => parse_cargo_toml(&content),
            "requirements.txt" => parse_requirements_txt(&content),
            _ => Vec::new(), // pyproject.toml：仅记录存在（TOML 数组解析留待专门阶段，§76）
        };

        let extracted = declarations.len();
        let truncated = extracted > options.max_dependencies_per_manifest;
        for declared in declarations
            .into_iter()
            .take(options.max_dependencies_per_manifest)
        {
            // 证据：Manifest 来源 + 文件路径（+ 行号若有）。
            let evidence = Evidence::new(
                EvidenceKind::Manifest,
                Some(file.relative_path.clone()),
                None,
                Some(name.to_string()),
                match declared.line {
                    Some(line) => format!("declares {} (line {line})", declared.name),
                    None => format!("declares {}", declared.name),
                },
            );
            let evidence_id = evidence.id.clone();
            bundle.insert_evidence(evidence);

            // External 包实体（§27：declared ≠ resolved ⇒ External scope）。
            let package_key = format!("pkg:{}", declared.name);
            let package_id = bundle.entity_id(&EntityKind::package(), &package_key);
            bundle.insert_entity(Entity::new(
                package_id.clone(),
                EntityKind::package(),
                declared.name.clone(),
                EntityLocation::External {
                    reference: declared.name.clone(),
                },
                EntityScope::External,
            ));
            bundle.insert_relation(Relation::new(
                RelationType::other("declares_dependency"),
                project_entity_id.clone(),
                RelationTarget::Resolved { entity: package_id },
                Origin::Fact,
                Confidence::High,
                vec![evidence_id.clone()],
            ));

            bundle.insert_fact(Fact::new(
                FactType::dependency_declared(),
                Some(project_entity_id.clone()),
                FactValue::Structured(serde_json::json!({
                    "name": declared.name,
                    "version_constraint": declared.version_constraint,
                    "scope": declared.scope,
                    "manifest": file.relative_path,
                })),
                Origin::Fact,
                Confidence::High,
                ConfidenceReason::direct_observation(),
                Freshness::Fresh,
                vec![evidence_id],
                at,
                provenance.clone(),
            ));
        }
        if truncated {
            warnings.push(format!(
                "manifest {} declarations capped at {}",
                file.relative_path, options.max_dependencies_per_manifest
            ));
        }
        let _ = extracted;
    }

    bundle.insert_fact(Fact::new(
        FactType::other("manifests_detected"),
        Some(project_entity_id.clone()),
        FactValue::Int(manifests_seen as i64),
        Origin::Fact,
        Confidence::High,
        ConfidenceReason::direct_observation(),
        Freshness::Fresh,
        vec![],
        at,
        provenance.clone(),
    ));
    warnings
}

/// package.json：dependencies（runtime）+ devDependencies（development）。
/// Err = JSON 畸形（§78 Malformed manifest 诊断的依据）。
fn parse_package_json(content: &str) -> Result<Vec<DeclaredDependency>, &'static str> {
    let mut out = Vec::new();
    let Ok(value) = serde_json::from_str::<serde_json::Value>(content) else {
        return Err("malformed JSON");
    };
    for (key, scope) in [
        ("dependencies", "runtime"),
        ("devDependencies", "development"),
    ] {
        let Some(map) = value.get(key).and_then(|v| v.as_object()) else {
            continue;
        };
        for (name, version) in map {
            out.push(DeclaredDependency {
                name: name.clone(),
                version_constraint: version.as_str().map(|v| v.to_string()),
                scope: match scope {
                    "runtime" => "runtime",
                    _ => "development",
                },
                line: None,
            });
        }
    }
    Ok(out)
}

/// Cargo.toml：行级扫描 [dependencies]/[dev-dependencies]/[build-dependencies]
/// 段内的 `name = "ver"` 与 `name = { version = "..." }`（确定性、无 TOML 依赖）。
fn parse_cargo_toml(content: &str) -> Vec<DeclaredDependency> {
    let mut out = Vec::new();
    let mut scope: Option<&'static str> = None;
    for (index, raw_line) in content.lines().enumerate() {
        let line = raw_line.trim();
        if line.starts_with('[') {
            scope = match line {
                "[dependencies]" => Some("runtime"),
                "[dev-dependencies]" => Some("development"),
                "[build-dependencies]" => Some("build"),
                _ => None,
            };
            continue;
        }
        let Some(current_scope) = scope else {
            continue;
        };
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((name, rest)) = line.split_once('=') else {
            continue;
        };
        let name = name.trim().trim_matches('"').to_string();
        if name.is_empty() {
            continue;
        }
        // workspace 继承形态（name.workspace = true）：展示名去后缀，
        // 原始键保留在约束字段由调用方原样记录。
        let name = name.strip_suffix(".workspace").unwrap_or(&name).to_string();
        // version = "x.y" 优先；否则整段值去引号后原样保留（workspace = true 等）。
        let version_constraint =
            extract_toml_version(rest).or_else(|| Some(rest.trim().trim_matches('"').to_string()));
        out.push(DeclaredDependency {
            name,
            version_constraint,
            scope: match current_scope {
                "runtime" => "runtime",
                "development" => "development",
                _ => "build",
            },
            line: Some(index + 1),
        });
    }
    out
}

fn extract_toml_version(value: &str) -> Option<String> {
    let key_pos = value.find("version")?;
    let after = &value[key_pos + "version".len()..];
    let eq_pos = after.find('=')?;
    let tail = after[eq_pos + 1..].trim();
    let quoted = tail.strip_prefix('"')?;
    let end = quoted.find('"')?;
    Some(quoted[..end].to_string())
}

/// requirements.txt：每条非注释行为一条声明（整行作为约束）。
fn parse_requirements_txt(content: &str) -> Vec<DeclaredDependency> {
    content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#') && !line.starts_with('-'))
        .map(|line| {
            let name = line
                .split(['=', '>', '<', '~', '!', ';', ' ', '['])
                .next()
                .unwrap_or(line)
                .to_string();
            DeclaredDependency {
                name,
                version_constraint: Some(line.to_string()),
                scope: "runtime",
                line: None,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_json_parses_both_scopes() {
        let deps = parse_package_json(
            r#"{"name":"x","dependencies":{"a":"^1.0"},"devDependencies":{"b":"~2.1"}}"#,
        )
        .unwrap();
        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0].name, "a");
        assert_eq!(deps[0].scope, "runtime");
        assert_eq!(deps[1].scope, "development");
    }

    #[test]
    fn malformed_package_json_is_an_error_not_panic() {
        assert!(
            parse_package_json("{ not json ").is_err(),
            "malformed manifest must surface as Err"
        );
    }

    #[test]
    fn cargo_toml_scopes_and_versions() {
        let deps = parse_cargo_toml(
            r#"[package]
name = "demo"

[dependencies]
serde = "1.0"
local = { path = "../local" }

[dev-dependencies]
tempfile = "3"
"#,
        );
        assert_eq!(deps.len(), 3);
        assert_eq!(deps[0].name, "serde");
        assert_eq!(deps[0].version_constraint.as_deref(), Some("1.0"));
        assert_eq!(deps[1].scope, "runtime");
        assert_eq!(deps[2].scope, "development");
    }

    #[test]
    fn requirements_txt_lines() {
        let deps = parse_requirements_txt("# comment\nrequests>=2.0\n\nflask\n-e .\n");
        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0].name, "requests");
        assert_eq!(deps[1].name, "flask");
    }
}
