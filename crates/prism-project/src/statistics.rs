//! 统计：项目规模与语言计数（文件数量统计，不做代码行占比——LOC 推迟到 M2+）。

use crate::file::{FileInfo, FileKind};
use serde::{Deserialize, Serialize};

/// 项目规模统计。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ProjectStatistics {
    pub total_files: u64,
    pub total_directories: u64,
    pub total_size_bytes: u64,
    pub source_files: u64,
    pub test_files: u64,
    pub config_files: u64,
    pub documentation_files: u64,
    pub asset_files: u64,
    pub binary_files: u64,
    pub generated_files: u64,
    pub unknown_files: u64,
    /// 被 ignore 引擎跳过的条目数（文件与目录合并计数）。
    pub ignored_files: u64,
    pub error_count: u64,
}

/// 从发现结果计算统计（结果由输入决定，天然确定）。
pub fn compute_statistics(
    files: &[FileInfo],
    total_directories: u64,
    ignored: u64,
    error_count: u64,
) -> ProjectStatistics {
    let mut stats = ProjectStatistics {
        total_files: files.len() as u64,
        total_directories,
        total_size_bytes: files.iter().map(|file| file.size_bytes).sum(),
        ignored_files: ignored,
        error_count,
        ..ProjectStatistics::default()
    };
    for file in files {
        match file.classification.kind {
            FileKind::Source => stats.source_files += 1,
            FileKind::Test => stats.test_files += 1,
            FileKind::Config => stats.config_files += 1,
            FileKind::Documentation => stats.documentation_files += 1,
            FileKind::Asset => stats.asset_files += 1,
            FileKind::Binary => stats.binary_files += 1,
            FileKind::Generated => stats.generated_files += 1,
            FileKind::Unknown => stats.unknown_files += 1,
        }
    }
    stats
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file::{FileClassification, FileInfo};
    use prism_core::ids::FileId;
    use prism_core::Confidence;

    fn file_with_kind(relative_path: &str, kind: FileKind, size: u64) -> FileInfo {
        let name = std::path::Path::new(relative_path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| relative_path.to_string());
        FileInfo {
            id: FileId::from_string(format!("file:{relative_path}")),
            relative_path: relative_path.to_string(),
            file_name: name,
            extension: None,
            size_bytes: size,
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

    #[test]
    fn counts_by_kind_and_totals() {
        let files = vec![
            file_with_kind("src/a.ts", FileKind::Source, 10),
            file_with_kind("src/b.test.ts", FileKind::Test, 20),
            file_with_kind("package.json", FileKind::Config, 30),
            file_with_kind("README.md", FileKind::Documentation, 40),
            file_with_kind("logo.png", FileKind::Asset, 50),
            file_with_kind("lib.dll", FileKind::Binary, 60),
        ];
        let stats = compute_statistics(&files, 3, 7, 2);
        assert_eq!(stats.total_files, 6);
        assert_eq!(stats.total_directories, 3);
        assert_eq!(stats.total_size_bytes, 210);
        assert_eq!(stats.source_files, 1);
        assert_eq!(stats.test_files, 1);
        assert_eq!(stats.config_files, 1);
        assert_eq!(stats.documentation_files, 1);
        assert_eq!(stats.asset_files, 1);
        assert_eq!(stats.binary_files, 1);
        assert_eq!(stats.ignored_files, 7);
        assert_eq!(stats.error_count, 2);
    }

    #[test]
    fn empty_input_gives_zeroed_statistics() {
        let stats = compute_statistics(&[], 0, 0, 0);
        assert_eq!(stats, ProjectStatistics::default());
    }
}
