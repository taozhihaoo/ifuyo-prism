//! 项目类型检测：Evidence → Rules → Candidates → Confidence。
//!
//! 现代项目常是多技术栈混合（Node + Rust + Tauri），因此返回多个候选，
//! 每个候选带 confidence 与 evidence；不使用 AI，不做唯一裁决。

use crate::directory::DirectoryInfo;
use crate::evidence::Evidence;
use crate::file::FileInfo;
use prism_core::Confidence;
use serde::{Deserialize, Serialize};

/// 一个项目类型候选。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectTypeCandidate {
    /// 展示名（如 "Rust" / "Node.js" / "Tauri"）。
    pub name: String,
    pub confidence: Confidence,
    pub evidence: Vec<Evidence>,
}

const SOURCE: &str = "project_type_detector";

fn has_file(files: &[FileInfo], relative_path: &str) -> bool {
    files.iter().any(|file| file.relative_path == relative_path)
}

fn has_file_ending_with(files: &[FileInfo], suffix: &str) -> bool {
    files
        .iter()
        .any(|file| file.relative_path.to_ascii_lowercase().ends_with(suffix))
}

fn has_file_starting_with(files: &[FileInfo], prefix: &str) -> bool {
    files
        .iter()
        .any(|file| file.relative_path.to_ascii_lowercase().starts_with(prefix))
}

fn has_directory(directories: &[DirectoryInfo], relative_path: &str) -> bool {
    directories
        .iter()
        .any(|directory| directory.relative_path == relative_path)
}

fn candidate(name: &str, confidence: Confidence, evidence: Vec<Evidence>) -> ProjectTypeCandidate {
    ProjectTypeCandidate {
        name: name.to_string(),
        confidence,
        evidence,
    }
}

/// 从已发现的文件与目录推断项目类型候选。
/// 结果排序：confidence 降序，同置信度按名字典序（纲领 §52）。
pub fn detect_project_types(
    files: &[FileInfo],
    directories: &[DirectoryInfo],
) -> Vec<ProjectTypeCandidate> {
    let mut candidates: Vec<ProjectTypeCandidate> = Vec::new();

    let mut push = |candidate: ProjectTypeCandidate| {
        if !candidates
            .iter()
            .any(|existing| existing.name == candidate.name)
        {
            candidates.push(candidate);
        }
    };

    // 根级或嵌套 workspace（如 src-tauri/Cargo.toml）的 Rust manifest 均可作证据。
    let rust_manifest = if has_file(files, "Cargo.toml") {
        Some("Cargo.toml".to_string())
    } else {
        files
            .iter()
            .map(|file| file.relative_path.clone())
            .find(|path| path.ends_with("/Cargo.toml"))
    };
    if let Some(manifest) = rust_manifest {
        push(candidate(
            "Rust",
            Confidence::High,
            vec![Evidence::new(SOURCE, &manifest, "Rust manifest found")],
        ));
    }
    if has_file(files, "package.json") {
        push(candidate(
            "Node.js",
            Confidence::High,
            vec![Evidence::new(
                SOURCE,
                "package.json",
                "Node package manifest found",
            )],
        ));
    }
    if has_file(files, "pyproject.toml") {
        push(candidate(
            "Python",
            Confidence::High,
            vec![Evidence::new(
                SOURCE,
                "pyproject.toml",
                "Python project config found",
            )],
        ));
    } else if has_file(files, "requirements.txt") || has_file(files, "Pipfile") {
        let path = if has_file(files, "requirements.txt") {
            "requirements.txt"
        } else {
            "Pipfile"
        };
        push(candidate(
            "Python",
            Confidence::High,
            vec![Evidence::new(SOURCE, path, "Python dependency file found")],
        ));
    }
    if has_file_ending_with(files, ".csproj")
        || has_file_ending_with(files, ".fsproj")
        || has_file_ending_with(files, ".sln")
    {
        let path = files
            .iter()
            .map(|file| file.relative_path.clone())
            .find(|path| {
                path.to_ascii_lowercase().ends_with(".csproj")
                    || path.to_ascii_lowercase().ends_with(".fsproj")
                    || path.to_ascii_lowercase().ends_with(".sln")
            })
            .unwrap_or_default();
        push(candidate(
            ".NET",
            Confidence::High,
            vec![Evidence::new(
                SOURCE,
                &path,
                ".NET project/solution file found",
            )],
        ));
    }
    if has_file(files, "go.mod") {
        push(candidate(
            "Go",
            Confidence::High,
            vec![Evidence::new(SOURCE, "go.mod", "Go modules file found")],
        ));
    }
    if has_file(files, "pom.xml") {
        push(candidate(
            "Java Maven",
            Confidence::High,
            vec![Evidence::new(SOURCE, "pom.xml", "Maven POM found")],
        ));
    }
    if has_file(files, "build.gradle") || has_file(files, "build.gradle.kts") {
        let path = if has_file(files, "build.gradle") {
            "build.gradle"
        } else {
            "build.gradle.kts"
        };
        push(candidate(
            "Java Gradle",
            Confidence::Medium,
            vec![Evidence::new(SOURCE, path, "Gradle build file found")],
        ));
    }
    if has_file(files, "CMakeLists.txt") || has_file(files, "Makefile") {
        let path = if has_file(files, "CMakeLists.txt") {
            "CMakeLists.txt"
        } else {
            "Makefile"
        };
        push(candidate(
            "C/C++",
            Confidence::Medium,
            vec![Evidence::new(SOURCE, path, "C/C++ build system file found")],
        ));
    }
    if has_directory(directories, "src-tauri") && has_file(files, "src-tauri/tauri.conf.json") {
        push(candidate(
            "Tauri",
            Confidence::High,
            vec![
                Evidence::new(SOURCE, "src-tauri", "Tauri source directory found"),
                Evidence::new(
                    SOURCE,
                    "src-tauri/tauri.conf.json",
                    "Tauri configuration found",
                ),
            ],
        ));
    }
    if has_file(files, "tsconfig.json") {
        push(candidate(
            "TypeScript",
            Confidence::High,
            vec![Evidence::new(
                SOURCE,
                "tsconfig.json",
                "TypeScript configuration found",
            )],
        ));
    }
    if files.iter().any(|file| {
        matches!(file.extension.as_deref(), Some("tsx") | Some("jsx"))
            && file.classification.kind == crate::file::FileKind::Source
    }) {
        let path = files
            .iter()
            .find(|file| matches!(file.extension.as_deref(), Some("tsx") | Some("jsx")))
            .map(|file| file.relative_path.clone())
            .unwrap_or_default();
        push(candidate(
            "React",
            Confidence::Medium,
            vec![Evidence::new(
                SOURCE,
                &path,
                "TSX/JSX component source found",
            )],
        ));
    }
    if has_file_starting_with(files, "vite.config.") {
        push(candidate(
            "Vite",
            Confidence::Medium,
            vec![Evidence::new(
                SOURCE,
                "vite.config.ts",
                "Vite configuration found",
            )],
        ));
    }
    if has_file_starting_with(files, "next.config.") {
        push(candidate(
            "Next.js",
            Confidence::Medium,
            vec![Evidence::new(
                SOURCE,
                "next.config.ts",
                "Next.js configuration found",
            )],
        ));
    }
    if has_file_starting_with(files, "webpack.config.") {
        push(candidate(
            "Webpack",
            Confidence::Medium,
            vec![Evidence::new(
                SOURCE,
                "webpack.config.js",
                "Webpack configuration found",
            )],
        ));
    }

    candidates.sort_by(|a, b| {
        b.confidence
            .cmp(&a.confidence)
            .then_with(|| a.name.cmp(&b.name))
    });
    candidates
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file::{FileClassification, FileKind};

    fn file(relative_path: &str, extension: Option<&str>, kind: FileKind) -> FileInfo {
        let name = std::path::Path::new(relative_path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| relative_path.to_string());
        FileInfo {
            id: prism_core::ids::FileId::from_string(format!("file:{relative_path}")),
            relative_path: relative_path.to_string(),
            file_name: name,
            extension: extension.map(|e| e.to_string()),
            size_bytes: 10,
            language: None,
            classification: FileClassification {
                kind,
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

    fn dir(relative_path: &str) -> DirectoryInfo {
        let name = std::path::Path::new(relative_path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| relative_path.to_string());
        DirectoryInfo {
            id: prism_core::ids::FileId::from_string(format!("dir:{relative_path}")),
            relative_path: relative_path.to_string(),
            name,
            depth: relative_path.split('/').count() as u32,
            is_hidden: false,
        }
    }

    #[test]
    fn empty_project_has_no_candidates() {
        assert!(detect_project_types(&[], &[]).is_empty());
    }

    #[test]
    fn rust_project_detected_with_evidence() {
        let files = vec![file("Cargo.toml", Some("toml"), FileKind::Config)];
        let candidates = detect_project_types(&files, &[]);
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].name, "Rust");
        assert_eq!(candidates[0].confidence, Confidence::High);
        assert_eq!(candidates[0].evidence[0].path, "Cargo.toml");
    }

    #[test]
    fn tauri_requires_directory_and_config() {
        // 只有 src-tauri 目录、没有 tauri.conf.json → 不判定 Tauri。
        let dirs = vec![dir("src-tauri")];
        assert!(detect_project_types(&[], &dirs)
            .iter()
            .all(|candidate| candidate.name != "Tauri"));

        let files = vec![file(
            "src-tauri/tauri.conf.json",
            Some("json"),
            FileKind::Config,
        )];
        let candidates = detect_project_types(&files, &dirs);
        let tauri = candidates.iter().find(|c| c.name == "Tauri").unwrap();
        assert_eq!(tauri.confidence, Confidence::High);
        assert_eq!(tauri.evidence.len(), 2);
    }

    #[test]
    fn mixed_stack_yields_multiple_candidates_sorted() {
        let files = vec![
            file("package.json", Some("json"), FileKind::Config),
            file("Cargo.toml", Some("toml"), FileKind::Config),
            file("tsconfig.json", Some("json"), FileKind::Config),
            file("src/App.tsx", Some("tsx"), FileKind::Source),
            file("vite.config.ts", Some("ts"), FileKind::Config),
        ];
        let dirs = vec![dir("src-tauri"), dir("src")];
        // 补上 tauri.conf.json 使 Tauri 命中
        let files = {
            let mut files = files;
            files.push(file(
                "src-tauri/tauri.conf.json",
                Some("json"),
                FileKind::Config,
            ));
            files
        };
        let candidates = detect_project_types(&files, &dirs);
        let names: Vec<&str> = candidates.iter().map(|c| c.name.as_str()).collect();
        assert!(names.contains(&"Rust"));
        assert!(names.contains(&"Node.js"));
        assert!(names.contains(&"Tauri"));
        assert!(names.contains(&"TypeScript"));
        assert!(names.contains(&"React"));
        assert!(names.contains(&"Vite"));
        // 排序：High 在前，同置信度按名字典序。
        assert_eq!(candidates[0].confidence, Confidence::High);
        for pair in candidates.windows(2) {
            assert!(
                pair[0].confidence > pair[1].confidence
                    || (pair[0].confidence == pair[1].confidence && pair[0].name <= pair[1].name)
            );
        }
    }

    #[test]
    fn dotnet_detected_by_pattern() {
        let files = vec![file("src/App/App.csproj", Some("csproj"), FileKind::Config)];
        let candidates = detect_project_types(&files, &[]);
        assert_eq!(candidates[0].name, ".NET");
        assert_eq!(candidates[0].evidence[0].path, "src/App/App.csproj");
    }
}
