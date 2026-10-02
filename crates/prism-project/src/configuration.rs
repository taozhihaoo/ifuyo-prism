//! 配置文件发现：只做“存在性与基础识别”，不解析内容。
//!
//! 解释（如 package.json 依赖、tsconfig paths）属于后续里程碑。

use crate::file::FileInfo;
use prism_core::Confidence;
use serde::{Deserialize, Serialize};

/// 已知配置文件的角色类型。JSON 值为 snake_case 字符串，保持稳定。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigurationType {
    NodeProjectManifest,
    NodePackageLock,
    YarnLock,
    PnpmWorkspace,
    TypeScriptConfig,
    RustManifest,
    RustLock,
    DotnetProject,
    DotnetSolution,
    PythonProjectConfig,
    PythonRequirements,
    Pipfile,
    MavenPom,
    GradleBuild,
    GoModules,
    CMake,
    Make,
    Docker,
    DockerCompose,
    ViteConfig,
    WebpackConfig,
    NextConfig,
    TauriConfig,
    ProjectReadme,
    AgentInstructions,
    GitIgnore,
    EditorConfig,
    Env,
    PrismConfig,
}

impl ConfigurationType {
    /// 已知配置名 → 类型（精确匹配文件名，大小写不敏感）。
    fn from_exact_name(file_name: &str) -> Option<Self> {
        let lower = file_name.to_ascii_lowercase();
        let kind = match lower.as_str() {
            "package.json" => ConfigurationType::NodeProjectManifest,
            "package-lock.json" => ConfigurationType::NodePackageLock,
            "yarn.lock" => ConfigurationType::YarnLock,
            "pnpm-workspace.yaml" => ConfigurationType::PnpmWorkspace,
            "tsconfig.json" | "tsconfig.jsonc" | "jsconfig.json" => {
                ConfigurationType::TypeScriptConfig
            }
            "cargo.toml" => ConfigurationType::RustManifest,
            "cargo.lock" => ConfigurationType::RustLock,
            "pyproject.toml" => ConfigurationType::PythonProjectConfig,
            "requirements.txt" | "requirements-dev.txt" => ConfigurationType::PythonRequirements,
            "pipfile" => ConfigurationType::Pipfile,
            "pom.xml" => ConfigurationType::MavenPom,
            "build.gradle" | "build.gradle.kts" | "settings.gradle" | "settings.gradle.kts" => {
                ConfigurationType::GradleBuild
            }
            "go.mod" => ConfigurationType::GoModules,
            "cmakelists.txt" => ConfigurationType::CMake,
            "makefile" | "gnumakefile" => ConfigurationType::Make,
            "dockerfile" | "dockerfile.dev" => ConfigurationType::Docker,
            "docker-compose.yml" | "docker-compose.yaml" => ConfigurationType::DockerCompose,
            "readme.md" | "readme" | "readme.txt" => ConfigurationType::ProjectReadme,
            "agents.md" | "claude.md" => ConfigurationType::AgentInstructions,
            ".gitignore" => ConfigurationType::GitIgnore,
            ".editorconfig" => ConfigurationType::EditorConfig,
            ".env" | ".env.example" | ".env.local" => ConfigurationType::Env,
            "prism.config.json" => ConfigurationType::PrismConfig,
            _ => return None,
        };
        Some(kind)
    }

    /// 已知配置模式 → 类型（模式匹配，大小写不敏感）。
    fn from_pattern(file_name: &str) -> Option<Self> {
        let lower = file_name.to_ascii_lowercase();
        if lower.ends_with(".csproj") {
            Some(ConfigurationType::DotnetProject)
        } else if lower.ends_with(".sln") {
            Some(ConfigurationType::DotnetSolution)
        } else if lower.ends_with(".fsproj") {
            Some(ConfigurationType::DotnetProject)
        } else if lower.starts_with("vite.config.") {
            Some(ConfigurationType::ViteConfig)
        } else if lower.starts_with("webpack.config.") || lower.starts_with("webpackfile.") {
            Some(ConfigurationType::WebpackConfig)
        } else if lower.starts_with("next.config.") {
            Some(ConfigurationType::NextConfig)
        } else if lower == "tauri.conf.json" {
            Some(ConfigurationType::TauriConfig)
        } else {
            None
        }
    }
}

/// 按文件名判断是否为已知配置文件（供分类器复用）。
pub fn known_configuration(file_name: &str) -> Option<ConfigurationType> {
    ConfigurationType::from_exact_name(file_name)
        .or_else(|| ConfigurationType::from_pattern(file_name))
}

/// 一个被识别出的配置文件。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigurationInfo {
    /// 配置的逻辑名（如 "package.json"）。
    pub name: String,
    /// 项目相对路径（`/` 分隔）。
    pub path: String,
    pub kind: ConfigurationType,
    pub confidence: Confidence,
}

/// 从已发现的文件中识别配置文件（结果按路径排序，确定性）。
pub fn detect_configurations(files: &[FileInfo]) -> Vec<ConfigurationInfo> {
    let mut configs: Vec<ConfigurationInfo> = files
        .iter()
        .filter_map(|file| {
            known_configuration(&file.file_name).map(|kind| ConfigurationInfo {
                name: file.file_name.clone(),
                path: file.relative_path.clone(),
                kind,
                // 根级已知配置为 High；嵌套目录里的同名文件仍是 High，
                // 但模式匹配（*.csproj / vite.config.*）记 Medium。
                confidence: if ConfigurationType::from_exact_name(&file.file_name).is_some() {
                    Confidence::High
                } else {
                    Confidence::Medium
                },
            })
        })
        .collect();
    configs.sort_by(|a, b| a.path.cmp(&b.path));
    configs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file::{FileClassification, FileKind};
    use prism_core::ids::FileId;
    use std::path::PathBuf;

    fn file(relative_path: &str) -> FileInfo {
        let name = std::path::Path::new(relative_path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| relative_path.to_string());
        FileInfo {
            id: FileId::from_string(format!("file:{relative_path}")),
            relative_path: relative_path.to_string(),
            file_name: name,
            extension: None,
            size_bytes: 10,
            language: None,
            classification: FileClassification {
                kind: FileKind::Config,
                confidence: Confidence::High,
                reason: "test".to_string(),
            },
            analyzable: true,
            non_analyzable_reason: None,
            modified_at: None,
            is_hidden: false,
            is_symlink: false,
        }
    }

    #[test]
    fn recognizes_known_configuration_names() {
        assert_eq!(
            known_configuration("package.json"),
            Some(ConfigurationType::NodeProjectManifest)
        );
        assert_eq!(
            known_configuration("Cargo.toml"),
            Some(ConfigurationType::RustManifest)
        );
        assert_eq!(
            known_configuration("tauri-app.csproj"),
            Some(ConfigurationType::DotnetProject)
        );
        assert_eq!(
            known_configuration("vite.config.ts"),
            Some(ConfigurationType::ViteConfig)
        );
        assert_eq!(
            known_configuration("AGENTS.md"),
            Some(ConfigurationType::AgentInstructions)
        );
        assert_eq!(known_configuration("random.txt"), None);
    }

    #[test]
    fn detection_is_case_insensitive() {
        assert_eq!(
            known_configuration("PACKAGE.JSON"),
            Some(ConfigurationType::NodeProjectManifest)
        );
    }

    #[test]
    fn detect_configurations_sorts_by_path() {
        let files = vec![
            file("src/other.json"),
            file("package.json"),
            file("Cargo.toml"),
        ];
        let configs = detect_configurations(&files);
        // package.json / Cargo.toml 命中已知名；src/other.json 不命中。
        assert_eq!(configs.len(), 2);
        assert_eq!(configs[0].path, "Cargo.toml");
        assert_eq!(configs[1].path, "package.json");
        assert_eq!(configs[1].confidence, Confidence::High);
    }
}
