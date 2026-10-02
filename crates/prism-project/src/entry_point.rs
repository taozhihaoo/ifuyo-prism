//! 候选入口点检测。
//!
//! M1 输出的是 Candidate Entry Points，不是绝对正确的 True Entry Point；
//! 一个项目可以有多个入口候选，不强行选择唯一入口（纲领 §28-§30）。

use crate::evidence::Evidence;
use crate::file::FileInfo;
use prism_core::Confidence;
use serde::{Deserialize, Serialize};

/// 入口角色：区分二进制入口与库入口等（不统一叫 main entry，纲领 §29）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntryPointKind {
    RustBinary,
    RustLibrary,
    Frontend,
    Python,
    Dotnet,
    Go,
}

/// 一个候选入口点。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntryPoint {
    /// 项目相对路径（`/` 分隔）。
    pub path: String,
    pub kind: EntryPointKind,
    pub confidence: Confidence,
    pub evidence: Vec<Evidence>,
}

const SOURCE: &str = "entry_point_detector";

fn evidence(path: &str, reason: &str) -> Vec<Evidence> {
    vec![Evidence::new(SOURCE, path, reason)]
}

/// 从已发现的文件推断候选入口点。
/// 结果排序：confidence 降序，同置信度按路径字典序（纲领 §52）。
pub fn detect_entry_points(files: &[FileInfo]) -> Vec<EntryPoint> {
    let mut entries: Vec<EntryPoint> = Vec::new();

    let mut push = |path: &str, kind: EntryPointKind, confidence: Confidence, reason: &str| {
        // 按文件存在判断（path 必须在发现结果中）。
        if files.iter().any(|file| file.relative_path == path)
            && !entries.iter().any(|e| e.path == path)
        {
            entries.push(EntryPoint {
                path: path.to_string(),
                kind,
                confidence,
                evidence: evidence(path, reason),
            });
        }
    };

    push(
        "src/main.rs",
        EntryPointKind::RustBinary,
        Confidence::High,
        "conventional Rust binary entry",
    );
    push(
        "main.rs",
        EntryPointKind::RustBinary,
        Confidence::High,
        "conventional Rust binary entry",
    );
    push(
        "src/lib.rs",
        EntryPointKind::RustLibrary,
        Confidence::High,
        "conventional Rust library entry",
    );
    push(
        "lib.rs",
        EntryPointKind::RustLibrary,
        Confidence::High,
        "conventional Rust library entry",
    );

    for extension in ["ts", "tsx", "js", "jsx"] {
        push(
            &format!("src/main.{extension}"),
            EntryPointKind::Frontend,
            Confidence::High,
            "conventional frontend entry",
        );
    }
    for extension in ["ts", "tsx", "js", "jsx"] {
        push(
            &format!("src/index.{extension}"),
            EntryPointKind::Frontend,
            Confidence::Medium,
            "common frontend module entry",
        );
    }
    push(
        "index.html",
        EntryPointKind::Frontend,
        Confidence::Medium,
        "web app entry page (Vite-style)",
    );

    push(
        "main.py",
        EntryPointKind::Python,
        Confidence::High,
        "conventional Python entry",
    );
    push(
        "app.py",
        EntryPointKind::Python,
        Confidence::Medium,
        "common Python application entry",
    );
    push(
        "__main__.py",
        EntryPointKind::Python,
        Confidence::Medium,
        "python -m package entry",
    );

    push(
        "Program.cs",
        EntryPointKind::Dotnet,
        Confidence::Medium,
        "conventional C# entry",
    );
    push(
        "Main.cs",
        EntryPointKind::Dotnet,
        Confidence::Low,
        "possible C# entry",
    );

    push(
        "main.go",
        EntryPointKind::Go,
        Confidence::High,
        "conventional Go entry",
    );
    push(
        "cmd/main.go",
        EntryPointKind::Go,
        Confidence::Medium,
        "possible Go entry",
    );

    entries.sort_by(|a, b| {
        b.confidence
            .cmp(&a.confidence)
            .then_with(|| a.path.cmp(&b.path))
    });
    entries
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file::{FileClassification, FileKind};

    fn file(relative_path: &str) -> FileInfo {
        let name = std::path::Path::new(relative_path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| relative_path.to_string());
        FileInfo {
            id: prism_core::ids::FileId::from_string(format!("file:{relative_path}")),
            relative_path: relative_path.to_string(),
            file_name: name,
            extension: None,
            size_bytes: 10,
            language: None,
            classification: FileClassification {
                kind: FileKind::Source,
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
    fn no_entry_points_in_empty_project() {
        assert!(detect_entry_points(&[]).is_empty());
    }

    #[test]
    fn detects_rust_binary_and_library_separately() {
        let files = vec![file("src/main.rs"), file("src/lib.rs")];
        let entries = detect_entry_points(&files);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].path, "src/lib.rs"); // 同为 High，按路径排序
        assert_eq!(entries[0].kind, EntryPointKind::RustLibrary);
        assert_eq!(entries[1].kind, EntryPointKind::RustBinary);
    }

    #[test]
    fn frontend_entries_ranked_by_confidence() {
        let files = vec![
            file("src/main.tsx"),
            file("src/index.ts"),
            file("index.html"),
        ];
        let entries = detect_entry_points(&files);
        assert_eq!(entries[0].path, "src/main.tsx");
        assert_eq!(entries[0].confidence, Confidence::High);
        assert_eq!(entries[0].kind, EntryPointKind::Frontend);
        // 剩余两个为 Medium，按路径排序。
        assert_eq!(entries[1].path, "index.html");
        assert_eq!(entries[2].path, "src/index.ts");
        for entry in &entries {
            assert!(!entry.evidence.is_empty(), "entry must carry evidence");
        }
    }

    #[test]
    fn missing_files_produce_no_entries() {
        // 规则只在文件真实存在时命中。
        let files = vec![file("README.md")];
        assert!(detect_entry_points(&files).is_empty());
    }

    #[test]
    fn python_entry_detection() {
        let files = vec![file("main.py"), file("__main__.py")];
        let entries = detect_entry_points(&files);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].path, "main.py");
        assert_eq!(entries[0].confidence, Confidence::High);
        assert_eq!(entries[1].path, "__main__.py");
        assert_eq!(entries[1].confidence, Confidence::Medium);
    }
}
