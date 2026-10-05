//! 其余 manifest 的声明抽取（M29 §12：package.json / Cargo.toml /
//! global.json / project.godot features——均与 M22 边界一致：只解析数据）。

use prism_intel::meta::Origin;
use serde::{Deserialize, Serialize};

/// package.json 依赖声明（§12 次优先；serde 解析）。
#[derive(Debug, Deserialize)]
struct PackageJsonFile {
    #[serde(default)]
    dependencies: serde_json::Map<String, serde_json::Value>,
    #[serde(default, rename = "devDependencies")]
    dev_dependencies: serde_json::Map<String, serde_json::Value>,
    #[serde(default)]
    license: Option<serde_json::Value>,
}

pub struct PackageJsonDeps {
    /// (name, requested_version, is_dev)
    pub dependencies: Vec<(String, Option<String>, bool)>,
    /// 项目自身 license 声明（原样字符串；缺失 ⇒ None，不等于"无"）。
    pub license: Option<String>,
}

/// 解析 package.json（malformed ⇒ Err 诊断）。
pub fn parse_package_json(content: &str) -> Result<PackageJsonDeps, String> {
    let parsed: PackageJsonFile =
        serde_json::from_str(content).map_err(|err| format!("package.json: {err}"))?;
    let mut dependencies = Vec::new();
    for (name, value) in parsed.dependencies {
        dependencies.push((name, value.as_str().map(str::to_string), false));
    }
    for (name, value) in parsed.dev_dependencies {
        dependencies.push((name, value.as_str().map(str::to_string), true));
    }
    dependencies.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(PackageJsonDeps {
        dependencies,
        license: parsed.license.and_then(|v| v.as_str().map(str::to_string)),
    })
}

/// Cargo.toml 依赖声明（行扫 `[dependencies]`/`[dev-dependencies]` 段；
/// `name = "ver"` 与 `name = { version = "ver" }` 两形）。
pub fn parse_cargo_toml(content: &str) -> Vec<(String, Option<String>, bool)> {
    let mut out = Vec::new();
    let mut section: Option<(&str, bool)> = None;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            let name = trimmed.trim_start_matches('[').trim_end_matches(']');
            section = match name {
                "dependencies" => Some(("dependencies", false)),
                "dev-dependencies" | "build-dependencies" => Some((name, true)),
                _ => None,
            };
            continue;
        }
        let Some((section_name, is_dev)) = section else {
            continue;
        };
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let Some((name, rest)) = trimmed.split_once('=') else {
            continue;
        };
        let name = name.trim().to_string();
        let rest = rest.trim();
        let version = if let Some(stripped) = rest.strip_prefix('{') {
            // inline table：找 version = "..."。
            stripped.split(',').find_map(|part| {
                let part = part.trim();
                part.strip_prefix("version")
                    .and_then(|v| v.trim().strip_prefix('='))
                    .map(|v| v.trim().trim_matches('"').to_string())
            })
        } else {
            Some(rest.trim_matches('"').to_string())
        };
        let _ = section_name;
        out.push((name, version, is_dev));
    }
    out
}

/// global.json 的 SDK 声明（§38 Declared Environment）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SdkDeclaration {
    pub version: Option<String>,
    pub allow_prerelease: Option<bool>,
    pub roll_forward: Option<String>,
    pub origin: Origin,
}

/// 解析 global.json（malformed ⇒ Err）。
pub fn parse_global_json(content: &str) -> Result<SdkDeclaration, String> {
    #[derive(Deserialize)]
    struct GlobalJson {
        #[serde(default)]
        sdk: Option<SdkSection>,
    }
    #[derive(Deserialize)]
    struct SdkSection {
        #[serde(default)]
        version: Option<String>,
        #[serde(default, rename = "allowPrerelease")]
        allow_prerelease: Option<bool>,
        #[serde(default, rename = "rollForward")]
        roll_forward: Option<String>,
    }
    let parsed: GlobalJson =
        serde_json::from_str(content).map_err(|err| format!("global.json: {err}"))?;
    Ok(match parsed.sdk {
        Some(sdk) => SdkDeclaration {
            version: sdk.version,
            allow_prerelease: sdk.allow_prerelease,
            roll_forward: sdk.roll_forward,
            origin: Origin::Fact,
        },
        None => SdkDeclaration {
            version: None,
            allow_prerelease: None,
            roll_forward: None,
            origin: Origin::Fact,
        },
    })
}

/// project.godot 的 `config/features` 首行版本提示（§84：这只是项目
/// 声明提示，**不是**当前机器 Godot 版本——观测版本必须来自真实探测）。
pub fn parse_project_godot_features(content: &str) -> Option<(String, usize)> {
    for (index, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("config/features") {
            // PackedStringArray("4.3", "C#") → 第一个引号段 = 版本提示。
            let first_quoted = rest.split('"').nth(1);
            if let Some(hint) = first_quoted {
                return Some((hint.to_string(), index + 1));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_json_deps_and_license() {
        let content = r#"{
  "name": "app",
  "version": "1.0.0",
  "license": "MIT",
  "dependencies": { "react": "^18.2.0" },
  "devDependencies": { "vitest": "^2.0.0" }
}"#;
        let parsed = parse_package_json(content).unwrap();
        assert_eq!(parsed.dependencies.len(), 2);
        assert_eq!(parsed.dependencies[0].0, "react");
        assert_eq!(parsed.dependencies[0].1.as_deref(), Some("^18.2.0"));
        assert!(!parsed.dependencies[0].2);
        assert!(parsed.dependencies[1].2, "devDependencies ⇒ development");
        assert_eq!(parsed.license.as_deref(), Some("MIT"));
    }

    #[test]
    fn cargo_toml_sections_and_inline_table() {
        let content = "[dependencies]\nserde = \"1.0.210\"\nclap = { version = \"4.5.0\", features = [\"derive\"] }\n\n[dev-dependencies]\ntempfile = \"3.0\"\n";
        let parsed = parse_cargo_toml(content);
        assert_eq!(parsed.len(), 3);
        assert_eq!(parsed[0], ("serde".into(), Some("1.0.210".into()), false));
        assert_eq!(parsed[1], ("clap".into(), Some("4.5.0".into()), false));
        assert_eq!(parsed[2], ("tempfile".into(), Some("3.0".into()), true));
    }

    #[test]
    fn global_json_sdk_declaration() {
        let parsed = parse_global_json(
            r#"{ "sdk": { "version": "8.0.100", "rollForward": "latestFeature" } }"#,
        )
        .unwrap();
        assert_eq!(parsed.version.as_deref(), Some("8.0.100"));
        assert_eq!(parsed.roll_forward.as_deref(), Some("latestFeature"));
    }

    #[test]
    fn godot_features_is_declaration_hint_only() {
        let content = "config_version=5\nconfig/features=PackedStringArray(\"4.3\", \"C#\", \"Forward Plus\")\n";
        let (hint, line) = parse_project_godot_features(content).unwrap();
        assert_eq!(hint, "4.3");
        assert_eq!(line, 2);
        assert!(parse_project_godot_features("config_version=5\n").is_none());
    }
}
