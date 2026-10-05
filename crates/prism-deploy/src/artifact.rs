//! Artifact 构建（M214 §五/§六/§七）：目录/文件 → 稳定身份 + Manifest。
//!
//! 身份 = SHA-256（目录 = 排序 manifest 哈希）；Git linkage 从注入的
//! commit 而来（§十六：不猜 Git 来源）；零 IO 设计之外仅读取调用方
//! 指定的 artifact 目录（这是 §五 Artifact Source 的真实读入）。

use crate::model::{ArtifactIdentity, ArtifactKind, ArtifactSource};
pub use crate::model::{ArtifactManifest, FileEntry, Sha256Hex};
use prism_core::task::CancellationToken;
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::Path;

/// 有界读取（§四十四 Resource Limits）。
#[derive(Debug, Clone, Copy)]
pub struct ArtifactLimits {
    pub max_files: usize,
    pub max_total_bytes: u64,
    pub max_file_bytes: u64,
}

impl Default for ArtifactLimits {
    fn default() -> Self {
        ArtifactLimits {
            max_files: 20_000,
            max_total_bytes: 2 * 1024 * 1024 * 1024,
            max_file_bytes: 512 * 1024 * 1024,
        }
    }
}

/// SHA-256 文件哈希（流式，bounded）。
pub fn sha256_file(path: &Path, max_bytes: u64) -> std::io::Result<Sha256Hex> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 65536];
    let mut total = 0u64;
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        total += count as u64;
        if total > max_bytes {
            return Err(std::io::Error::new(
                std::io::ErrorKind::FileTooLarge,
                format!("file exceeds {max_bytes} bytes"),
            ));
        }
        hasher.update(&buffer[..count]);
    }
    Ok(Sha256Hex(hex(&hasher.finalize())))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// 从目录或文件构建 Artifact Manifest（§七）。
///
/// * `git_commit` — 注入的 Git linkage（§十六：None ⇒ None，不猜）。
/// - 失败/超限 ⇒ Err（不静默，§四十四）。
pub fn build_manifest(
    artifact_path: &Path,
    name: &str,
    source: ArtifactSource,
    git_commit: Option<String>,
    limits: &ArtifactLimits,
    cancel: &CancellationToken,
) -> std::io::Result<ArtifactManifest> {
    let metadata = std::fs::metadata(artifact_path)?;
    if metadata.is_file() {
        let sha = sha256_file(artifact_path, limits.max_file_bytes)?;
        let size = metadata.len();
        let identity = ArtifactIdentity {
            artifact_id: format!(
                "artifact:{}",
                prism_intel::identity::fnv1a_hex16(&format!("file|{}", sha.0))
            ),
            kind: ArtifactKind::File,
            sha256: sha.clone(),
            git_commit,
        };
        return Ok(ArtifactManifest {
            artifact: identity,
            name: name.to_string(),
            source,
            files: vec![FileEntry {
                relative_path: name.to_string(),
                size_bytes: size,
                sha256: sha,
            }],
            total_bytes: size,
            generated_at: prism_core::Timestamp::now_utc().to_string(),
            diagnostics: Vec::new(),
        });
    }
    // 目录：递归收集（确定性排序）。
    let mut files: Vec<FileEntry> = Vec::new();
    let mut diagnostics = Vec::new();
    collect_dir(
        artifact_path,
        "",
        &mut files,
        &mut diagnostics,
        limits,
        cancel,
    )?;
    if files.is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "artifact directory is empty",
        ));
    }
    files.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
    let total_bytes = files.iter().map(|f| f.size_bytes).sum();
    // 目录哈希 = 排序 (path|size|hash) 串联的整体 SHA-256（§七 manifest hash）。
    let mut hasher = Sha256::new();
    for file in &files {
        hasher.update(file.relative_path.as_bytes());
        hasher.update(b"|");
        hasher.update(file.size_bytes.to_le_bytes());
        hasher.update(b"|");
        hasher.update(file.sha256.0.as_bytes());
        hasher.update(b"\n");
    }
    let sha = Sha256Hex(hex(&hasher.finalize()));
    let identity = ArtifactIdentity {
        artifact_id: format!(
            "artifact:{}",
            prism_intel::identity::fnv1a_hex16(&format!("dir|{}", sha.0))
        ),
        kind: ArtifactKind::Directory,
        sha256: sha.clone(),
        git_commit,
    };
    Ok(ArtifactManifest {
        artifact: identity,
        name: name.to_string(),
        source,
        files,
        total_bytes,
        generated_at: prism_core::Timestamp::now_utc().to_string(),
        diagnostics,
    })
}

fn collect_dir(
    root: &Path,
    prefix: &str,
    files: &mut Vec<FileEntry>,
    diagnostics: &mut Vec<String>,
    limits: &ArtifactLimits,
    cancel: &CancellationToken,
) -> std::io::Result<()> {
    if files.len() >= limits.max_files {
        diagnostics.push(format!(
            "file cap {} reached; remaining files skipped",
            limits.max_files
        ));
        return Ok(());
    }
    for entry in std::fs::read_dir(root)? {
        if cancel.is_cancelled() {
            return Err(std::io::Error::other("cancelled"));
        }
        let entry = entry?;
        let file_type = entry.file_type()?;
        let relative = if prefix.is_empty() {
            entry.file_name().to_string_lossy().to_string()
        } else {
            format!("{prefix}/{}", entry.file_name().to_string_lossy())
        };
        if file_type.is_symlink() {
            // §四十六 symlink 逃逸拒绝：不入 manifest，如实诊断。
            diagnostics.push(format!("symlink skipped: {relative}"));
            continue;
        }
        if file_type.is_dir() {
            collect_dir(&entry.path(), &relative, files, diagnostics, limits, cancel)?;
        } else {
            let size = entry.metadata()?.len();
            if size > limits.max_file_bytes {
                diagnostics.push(format!("file skipped (too large): {relative}"));
                continue;
            }
            let sha = sha256_file(&entry.path(), limits.max_file_bytes)?;
            files.push(FileEntry {
                relative_path: relative,
                size_bytes: size,
                sha256: sha,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct DirFixture {
        root: std::path::PathBuf,
    }

    impl DirFixture {
        fn new(name: &str) -> Self {
            let root = std::env::temp_dir().join(format!(
                "prism-deploy-fixture-{name}-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.subsec_nanos())
                    .unwrap_or(0)
            ));
            std::fs::create_dir_all(&root).unwrap();
            DirFixture { root }
        }

        fn write(&self, relative: &str, content: &str) {
            let path = self.root.join(relative);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, content).unwrap();
        }
    }

    impl Drop for DirFixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn directory_manifest_is_deterministic_and_sorted() {
        let fixture = DirFixture::new("manifest");
        fixture.write("b.txt", "bbb");
        fixture.write("a.txt", "aa");
        fixture.write("sub/c.txt", "c");
        let manifest = build_manifest(
            &fixture.root,
            "demo",
            ArtifactSource::Build,
            Some("abc123".to_string()),
            &ArtifactLimits::default(),
            &CancellationToken::new(),
        )
        .unwrap();
        assert_eq!(manifest.artifact.kind, ArtifactKind::Directory);
        assert_eq!(
            manifest.artifact.git_commit.as_deref(),
            Some("abc123"),
            "§16 Git linkage"
        );
        let paths: Vec<&str> = manifest
            .files
            .iter()
            .map(|f| f.relative_path.as_str())
            .collect();
        assert_eq!(paths, vec!["a.txt", "b.txt", "sub/c.txt"], "排序稳定 §43");
        assert_eq!(manifest.total_bytes, 6);
        // 同输入同 manifest（确定性 §四十三）。
        let again = build_manifest(
            &fixture.root,
            "demo",
            ArtifactSource::Build,
            Some("abc123".to_string()),
            &ArtifactLimits::default(),
            &CancellationToken::new(),
        )
        .unwrap();
        assert_eq!(manifest.artifact.sha256, again.artifact.sha256);
        assert_eq!(manifest.artifact.artifact_id, again.artifact.artifact_id);
    }

    #[test]
    fn content_change_changes_identity() {
        let fixture = DirFixture::new("identity");
        fixture.write("app.bin", "v1");
        let first = build_manifest(
            &fixture.root,
            "app",
            ArtifactSource::Build,
            None,
            &ArtifactLimits::default(),
            &CancellationToken::new(),
        )
        .unwrap();
        fixture.write("app.bin", "v2");
        let second = build_manifest(
            &fixture.root,
            "app",
            ArtifactSource::Build,
            None,
            &ArtifactLimits::default(),
            &CancellationToken::new(),
        )
        .unwrap();
        assert_ne!(
            first.artifact.sha256, second.artifact.sha256,
            "§六 内容变化 => 身份变化"
        );
    }

    #[test]
    fn symlink_is_skipped_with_diagnostic() {
        let fixture = DirFixture::new("symlink");
        fixture.write("real.txt", "data");
        #[cfg(unix)]
        std::os::unix::fs::symlink("/etc/passwd", fixture.root.join("escape.txt")).unwrap();
        #[cfg(windows)]
        {
            // Windows symlink 需要特权——降级为普通文件跳过路径无法触发；
            // 改为验证诊断字段存在的结构性（Unix-only 逻辑）。
        }
        let manifest = build_manifest(
            &fixture.root,
            "app",
            ArtifactSource::Build,
            None,
            &ArtifactLimits::default(),
            &CancellationToken::new(),
        );
        // Unix: escape.txt 是 symlink => skipped + diagnostic；Windows: 普通文件。
        #[cfg(unix)]
        {
            let manifest = manifest.unwrap();
            assert!(
                manifest
                    .diagnostics
                    .iter()
                    .any(|d| d.contains("symlink skipped")),
                "§46 symlink 逃逸拒绝"
            );
            assert!(manifest
                .files
                .iter()
                .all(|f| f.relative_path != "escape.txt"));
        }
        #[cfg(windows)]
        let _ = manifest;
    }

    #[test]
    fn file_artifact_single_entry() {
        let fixture = DirFixture::new("single");
        fixture.write("app.zip", "PK");
        let manifest = build_manifest(
            fixture.root.join("app.zip").as_path(),
            "app.zip",
            ArtifactSource::Publish,
            None,
            &ArtifactLimits::default(),
            &CancellationToken::new(),
        )
        .unwrap();
        assert_eq!(manifest.artifact.kind, ArtifactKind::File);
        assert_eq!(manifest.files.len(), 1);
        assert_eq!(manifest.files[0].relative_path, "app.zip");
    }
}
