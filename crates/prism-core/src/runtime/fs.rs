//! 文件系统抽象。
//!
//! 只抽象未来需要“可测试、可替换、可 mock”的边界，不包装 std::fs 的每个 API。
//! 业务模块依赖 [`FileSystem`] trait；测试可注入内存实现。

use crate::error::PrismResult;
use crate::timestamp::Timestamp;
use std::path::{Path, PathBuf};

/// 文件元数据（trait 自有类型，不泄漏 std::fs::Metadata）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileMetadata {
    pub is_dir: bool,
    pub is_file: bool,
    pub is_symlink: bool,
    pub size_bytes: u64,
    /// 修改时间；不可得时为 None（不是错误）。
    pub modified: Option<Timestamp>,
}

/// 文件系统抽象。实现必须线程安全。
pub trait FileSystem: Send + Sync {
    fn exists(&self, path: &Path) -> bool;
    fn read_to_string(&self, path: &Path) -> PrismResult<String>;
    fn write(&self, path: &Path, contents: &str) -> PrismResult<()>;
    fn create_dir_all(&self, path: &Path) -> PrismResult<()>;
    /// 删除文件（不递归目录）。
    fn remove_file(&self, path: &Path) -> PrismResult<()>;
    /// 递归删除目录。
    fn remove_dir_all(&self, path: &Path) -> PrismResult<()>;
    /// 列出直接子项，按路径字典序排序（确定性输出）。
    fn list_dir(&self, path: &Path) -> PrismResult<Vec<PathBuf>>;
    fn metadata(&self, path: &Path) -> PrismResult<FileMetadata>;
    fn canonicalize(&self, path: &Path) -> PrismResult<PathBuf>;
}

/// 基于 `std::fs` 的默认实现。
#[derive(Debug, Default)]
pub struct StdFileSystem;

impl StdFileSystem {
    pub fn new() -> Self {
        Self
    }
}

impl FileSystem for StdFileSystem {
    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }

    fn read_to_string(&self, path: &Path) -> PrismResult<String> {
        Ok(std::fs::read_to_string(path)?)
    }

    fn write(&self, path: &Path, contents: &str) -> PrismResult<()> {
        Ok(std::fs::write(path, contents)?)
    }

    fn create_dir_all(&self, path: &Path) -> PrismResult<()> {
        Ok(std::fs::create_dir_all(path)?)
    }

    fn remove_file(&self, path: &Path) -> PrismResult<()> {
        Ok(std::fs::remove_file(path)?)
    }

    fn remove_dir_all(&self, path: &Path) -> PrismResult<()> {
        Ok(std::fs::remove_dir_all(path)?)
    }

    fn list_dir(&self, path: &Path) -> PrismResult<Vec<PathBuf>> {
        let mut entries: Vec<PathBuf> = std::fs::read_dir(path)?
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .map(|entry| entry.path())
            .collect();
        entries.sort();
        Ok(entries)
    }

    fn metadata(&self, path: &Path) -> PrismResult<FileMetadata> {
        let meta = std::fs::metadata(path)?;
        let file_type = meta.file_type();
        let modified = meta.modified().ok().map(|time| {
            let dt = time
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as i128)
                .unwrap_or(0);
            Timestamp::from_unix_millis(dt).unwrap_or_default()
        });
        Ok(FileMetadata {
            is_dir: file_type.is_dir(),
            is_file: file_type.is_file(),
            is_symlink: file_type.is_symlink(),
            size_bytes: meta.len(),
            modified,
        })
    }

    fn canonicalize(&self, path: &Path) -> PrismResult<PathBuf> {
        Ok(path.canonicalize()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("prism-core-fs-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn write_read_and_list_roundtrip() {
        let fs = StdFileSystem::new();
        let dir = temp_dir("roundtrip");
        fs.write(&dir.join("b.txt"), "beta").unwrap();
        fs.write(&dir.join("a.txt"), "alpha").unwrap();
        fs.create_dir_all(&dir.join("sub")).unwrap();

        assert_eq!(fs.read_to_string(&dir.join("a.txt")).unwrap(), "alpha");
        let entries = fs.list_dir(&dir).unwrap();
        let names: Vec<String> = entries
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        assert_eq!(names, vec!["a.txt", "b.txt", "sub"], "list must be sorted");

        fs.remove_dir_all(&dir).unwrap();
        assert!(!fs.exists(&dir));
    }

    #[test]
    fn metadata_reports_kind_and_size() {
        let fs = StdFileSystem::new();
        let dir = temp_dir("metadata");
        fs.write(&dir.join("file.txt"), "12345").unwrap();

        let file_meta = fs.metadata(&dir.join("file.txt")).unwrap();
        assert!(file_meta.is_file);
        assert_eq!(file_meta.size_bytes, 5);

        let dir_meta = fs.metadata(&dir).unwrap();
        assert!(dir_meta.is_dir);

        fs.remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn missing_path_maps_to_not_found() {
        let fs = StdFileSystem::new();
        let missing = Path::new("definitely-missing-prism-file.txt");
        let err = fs.read_to_string(missing).unwrap_err();
        assert_eq!(err.code(), crate::error::PrismErrorCode::NotFound);
    }

    #[test]
    fn remove_file_only_removes_files() {
        let fs = StdFileSystem::new();
        let dir = temp_dir("remove-file");
        fs.write(&dir.join("x.txt"), "x").unwrap();
        fs.remove_file(&dir.join("x.txt")).unwrap();
        assert!(!fs.exists(&dir.join("x.txt")));
        assert!(fs.exists(&dir));
        fs.remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn canonicalize_resolves_paths() {
        let fs = StdFileSystem::new();
        let dir = temp_dir("canonicalize");
        let canonical = fs.canonicalize(&dir).unwrap();
        assert!(canonical.is_absolute());
        fs.remove_dir_all(&dir).unwrap();
    }
}
