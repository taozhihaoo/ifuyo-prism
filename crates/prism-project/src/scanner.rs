//! 文件系统扫描器：递归目录遍历 → IgnoreEngine → metadata → 分类。
//!
//! 设计约束（纲领 §52/§53/§54/§57）：
//! - 确定性：每层目录条目按名字典序处理，最终结果再按相对路径排序；
//! - 不一次性读文件内容，只用 metadata（禁止 read_to_string 整个项目）；
//! - 取消：每个条目检查 CancellationToken，取消后返回已完成部分并标记
//!   status = Cancelled（取消不是故障）；
//! - 非致命错误累积为 ScanError，不击穿整个扫描（纲领 §18/§19）；
//! - 不跟随符号链接（目录符号链接记录为条目但不递归，纲领 §17）；
//! - 只访问项目 root 内部（纲领 §57）。

use crate::directory::DirectoryInfo;
use crate::discovery::{LanguageCount, ProjectDiscovery, ScanInfo, SCHEMA_VERSION};
use crate::file::{build_file_info, FileClassifier};
use crate::ignore::{IgnoreEngine, ScanOptions};
use crate::project::Project;
use crate::statistics::compute_statistics;
use prism_core::ids::TaskId;
use prism_core::task::{CancellationToken, TaskState};
use prism_core::timestamp::Timestamp;
use prism_core::{PrismError, PrismResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

/// 实时扫描计数（进度展示用，都是真实计数，不伪造百分比）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ScanProgress {
    pub directories: u64,
    pub files: u64,
    pub ignored: u64,
    pub errors: u64,
}

/// 非致命扫描错误：可追踪到 path / operation / message（纲领 §77）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScanError {
    /// 项目相对路径（根级问题为 "."）。
    pub path: String,
    pub kind: ScanErrorKind,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScanErrorKind {
    ReadDir,
    Metadata,
    Symlink,
    Other,
}

/// 扫描器。
#[derive(Debug, Clone)]
pub struct Scanner {
    options: ScanOptions,
}

struct MutableWalk {
    files: Vec<crate::file::FileInfo>,
    directories: Vec<DirectoryInfo>,
    errors: Vec<ScanError>,
    progress: ScanProgress,
}

impl Scanner {
    pub fn new(options: ScanOptions) -> Self {
        Scanner { options }
    }

    /// 执行扫描。根目录不存在/不可读是 Fatal（返回 Err）；其余错误非致命。
    /// 取消时返回 Ok，status = Cancelled（保留已完成的部分事实）。
    pub fn scan(
        &self,
        project: &Project,
        cancel: &CancellationToken,
        mut on_progress: impl FnMut(ScanProgress),
    ) -> PrismResult<ProjectDiscovery> {
        let started_at = Timestamp::now_utc();
        let started_instant = Instant::now();
        let classifier = FileClassifier::new();

        let (engine, engine_warnings) = IgnoreEngine::build(&project.root_path, &self.options);
        let mut walk = MutableWalk {
            files: Vec::new(),
            directories: Vec::new(),
            errors: engine_warnings
                .iter()
                .map(|message| ScanError {
                    path: ".gitignore".to_string(),
                    kind: ScanErrorKind::Other,
                    message: message.clone(),
                })
                .collect(),
            progress: ScanProgress::default(),
        };

        // 迭代 DFS（避免深目录递归爆栈），每层条目按名字典序处理。
        let mut stack: Vec<(PathBuf, String, u32)> =
            vec![(project.root_path.clone(), String::new(), 0)];
        let mut cancelled = false;

        'outer: while let Some((abs_dir, rel_dir, depth)) = stack.pop() {
            if cancel.is_cancelled() {
                cancelled = true;
                break 'outer;
            }

            let mut entries: Vec<(String, PathBuf, bool, bool)> = Vec::new(); // (name, path, is_dir, is_symlink)
            match std::fs::read_dir(&abs_dir) {
                Ok(read_dir) => {
                    for entry in read_dir {
                        match entry {
                            Ok(entry) => {
                                let name = entry.file_name().to_string_lossy().to_string();
                                match entry.file_type() {
                                    Ok(file_type) => entries.push((
                                        name,
                                        entry.path(),
                                        file_type.is_dir(),
                                        file_type.is_symlink(),
                                    )),
                                    Err(err) => Self::record_error(
                                        &mut walk,
                                        &rel_dir,
                                        ScanErrorKind::Metadata,
                                        format!("failed to stat entry: {err}"),
                                    ),
                                }
                            }
                            Err(err) => Self::record_error(
                                &mut walk,
                                &rel_dir,
                                ScanErrorKind::ReadDir,
                                format!("failed to read directory entry: {err}"),
                            ),
                        }
                    }
                }
                Err(err) => {
                    if rel_dir.is_empty() {
                        // Fatal：项目根不可读。
                        return Err(PrismError::Io {
                            message: format!("project root is not readable: {err}"),
                        });
                    }
                    Self::record_error(
                        &mut walk,
                        &rel_dir,
                        ScanErrorKind::ReadDir,
                        format!("failed to read directory: {err}"),
                    );
                    continue;
                }
            }

            entries.sort_by(|a, b| a.0.cmp(&b.0));

            for (name, abs_path, is_dir, is_symlink) in entries {
                if cancel.is_cancelled() {
                    cancelled = true;
                    break 'outer;
                }

                let rel_path = if rel_dir.is_empty() {
                    name.clone()
                } else {
                    format!("{rel_dir}/{name}")
                };
                let is_hidden = name.starts_with('.');

                if engine.ignored(Path::new(&rel_path), &name, is_dir) {
                    walk.progress.ignored += 1;
                    on_progress(walk.progress);
                    continue;
                }

                if is_dir && !is_symlink {
                    walk.directories.push(DirectoryInfo {
                        id: prism_core::ids::FileId::from_string(format!("dir:{rel_path}")),
                        relative_path: rel_path.clone(),
                        name: name.clone(),
                        depth: depth + 1,
                        is_hidden,
                    });
                    walk.progress.directories += 1;
                    stack.push((abs_path, rel_path, depth + 1));
                    on_progress(walk.progress);
                    continue;
                }

                // 文件或符号链接（链接一律不跟随，按条目记录）。
                let meta = match std::fs::symlink_metadata(&abs_path) {
                    Ok(meta) => meta,
                    Err(err) => {
                        Self::record_error(
                            &mut walk,
                            &rel_path,
                            ScanErrorKind::Metadata,
                            format!("failed to read metadata: {err}"),
                        );
                        continue;
                    }
                };
                let modified_at = meta.modified().ok().map(|time| {
                    Timestamp::from_unix_millis(
                        time.duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_millis() as i128)
                            .unwrap_or(0),
                    )
                    .unwrap_or_default()
                });

                match build_file_info(
                    &classifier,
                    &rel_path,
                    name.clone(),
                    meta.len(),
                    modified_at,
                    is_hidden,
                    is_symlink,
                    self.options.max_file_size,
                ) {
                    Ok(info) => {
                        walk.files.push(info);
                        walk.progress.files += 1;
                        on_progress(walk.progress);
                    }
                    Err(err) => Self::record_error(
                        &mut walk,
                        &rel_path,
                        ScanErrorKind::Other,
                        format!("failed to build file info: {err}"),
                    ),
                }
            }
        }

        // 确定性排序：文件/目录按相对路径字典序。
        walk.files
            .sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
        walk.directories
            .sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
        walk.errors
            .sort_by(|a, b| a.path.cmp(&b.path).then_with(|| a.message.cmp(&b.message)));

        let configurations = crate::configuration::detect_configurations(&walk.files);
        let project_types =
            crate::project_type::detect_project_types(&walk.files, &walk.directories);
        let entry_points = crate::entry_point::detect_entry_points(&walk.files);
        let languages = Self::language_counts(&walk.files);
        let statistics = compute_statistics(
            &walk.files,
            walk.directories.len() as u64,
            walk.progress.ignored,
            walk.errors.len() as u64,
        );

        let finished_at = Timestamp::now_utc();
        let scan = ScanInfo {
            scan_id: TaskId::new(),
            root_path: project.root_path.clone(),
            started_at,
            finished_at: Some(finished_at),
            duration_ms: started_instant.elapsed().as_millis() as u64,
            files_scanned: walk.files.len() as u64,
            directories_scanned: walk.directories.len() as u64,
            files_ignored: walk.progress.ignored,
            errors_count: walk.errors.len() as u64,
            status: if cancelled {
                TaskState::Cancelled
            } else {
                TaskState::Completed
            },
        };

        Ok(ProjectDiscovery {
            schema_version: SCHEMA_VERSION,
            project: project.clone(),
            scan,
            files: walk.files,
            directories: walk.directories,
            languages,
            project_types,
            configurations,
            entry_points,
            statistics,
            errors: walk.errors,
        })
    }

    /// 语言 → 文件数（数量降序，同数量按语言名字典序）。
    fn language_counts(files: &[crate::file::FileInfo]) -> Vec<LanguageCount> {
        let mut counts: HashMap<crate::language::Language, u64> = HashMap::new();
        for file in files.iter().filter_map(|file| file.language) {
            *counts.entry(file).or_insert(0) += 1;
        }
        let mut list: Vec<LanguageCount> = counts
            .into_iter()
            .map(|(language, count)| LanguageCount { language, count })
            .collect();
        list.sort_by(|a, b| {
            b.count
                .cmp(&a.count)
                .then_with(|| a.language.cmp(&b.language))
        });
        list
    }

    fn record_error(walk: &mut MutableWalk, path: &str, kind: ScanErrorKind, message: String) {
        walk.errors.push(ScanError {
            path: path.to_string(),
            kind,
            message,
        });
        walk.progress.errors += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn unique_root(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "prism-scanner-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.subsec_nanos())
                .unwrap_or(0)
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn scan(root: &Path) -> ProjectDiscovery {
        let project = Project::from_root(root).unwrap();
        Scanner::new(ScanOptions::default())
            .scan(&project, &CancellationToken::new(), |_| {})
            .unwrap()
    }

    #[test]
    fn scans_nested_tree_with_counts_and_ordering() {
        let root = unique_root("nested");
        fs::create_dir_all(root.join("src/components")).unwrap();
        fs::create_dir_all(root.join("tests")).unwrap();
        fs::write(root.join("src/main.ts"), "let x = 1;").unwrap();
        fs::write(root.join("src/components/Button.tsx"), "x").unwrap();
        fs::write(root.join("tests/app.test.ts"), "x").unwrap();
        fs::write(root.join("README.md"), "doc").unwrap();
        fs::write(root.join("package.json"), "{}").unwrap();

        let discovery = scan(&root);
        assert_eq!(discovery.scan.status, TaskState::Completed);
        assert_eq!(discovery.scan.files_scanned, 5);
        // src、src/components、tests 共 3 个目录。
        assert_eq!(discovery.scan.directories_scanned, 3);
        let paths: Vec<&str> = discovery
            .files
            .iter()
            .map(|f| f.relative_path.as_str())
            .collect();
        assert_eq!(
            paths,
            vec![
                "README.md",
                "package.json",
                "src/components/Button.tsx",
                "src/main.ts",
                "tests/app.test.ts"
            ]
        );
        let langs: Vec<&str> = discovery
            .languages
            .iter()
            .map(|l| l.language.display_name())
            .collect();
        assert_eq!(langs.first().copied(), Some("TypeScript"));
        assert!(!discovery.entry_points.is_empty());
        assert!(discovery.project_types.iter().any(|t| t.name == "Node.js"));
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn empty_project_scans_clean() {
        let root = unique_root("empty");
        let discovery = scan(&root);
        assert_eq!(discovery.files.len(), 0);
        assert_eq!(discovery.directories.len(), 0);
        assert_eq!(discovery.scan.status, TaskState::Completed);
        assert_eq!(discovery.statistics.total_files, 0);
        assert!(discovery.errors.is_empty());
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn default_ignores_exclude_build_directories() {
        let root = unique_root("ignore");
        fs::create_dir_all(root.join("node_modules/pkg")).unwrap();
        fs::create_dir_all(root.join("target/debug")).unwrap();
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("node_modules/pkg/index.js"), "x").unwrap();
        fs::write(root.join("target/debug/app.exe"), "x").unwrap();
        fs::write(root.join("src/main.ts"), "x").unwrap();

        let discovery = scan(&root);
        assert_eq!(discovery.files.len(), 1);
        assert_eq!(discovery.files[0].relative_path, "src/main.ts");
        // 只计第一层的 node_modules 与 target（walker 不进入被忽略目录，
        // 其子条目不会被枚举）。
        assert_eq!(discovery.scan.files_ignored, 2);
        assert!(discovery
            .directories
            .iter()
            .all(|d| d.name != "node_modules"));
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn gitignore_is_respected_with_negation() {
        let root = unique_root("gitignore");
        fs::write(
            root.join(".gitignore"),
            "generated/\n*.tmp\n!important.tmp\n",
        )
        .unwrap();
        fs::create_dir_all(root.join("generated")).unwrap();
        fs::write(root.join("generated/out.ts"), "x").unwrap();
        fs::write(root.join("scratch.tmp"), "x").unwrap();
        fs::write(root.join("important.tmp"), "x").unwrap();
        fs::write(root.join("main.ts"), "x").unwrap();

        let discovery = scan(&root);
        let paths: Vec<&str> = discovery
            .files
            .iter()
            .map(|f| f.relative_path.as_str())
            .collect();
        assert_eq!(paths, vec![".gitignore", "important.tmp", "main.ts"]);
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn cancellation_preserves_partial_results_with_status() {
        let root = unique_root("cancel");
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("src/main.ts"), "x").unwrap();

        let token = CancellationToken::new();
        token.cancel();
        let project = Project::from_root(&root).unwrap();
        let discovery = Scanner::new(ScanOptions::default())
            .scan(&project, &token, |_| {})
            .unwrap();
        assert_eq!(discovery.scan.status, TaskState::Cancelled);
    }

    #[test]
    fn same_project_scans_deterministically() {
        let root = unique_root("determinism");
        fs::create_dir_all(root.join("a/b")).unwrap();
        fs::create_dir_all(root.join("z")).unwrap();
        for name in ["a/one.ts", "a/b/two.ts", "z/three.ts", "readme.md"] {
            fs::write(root.join(name), "x").unwrap();
        }
        let first = scan(&root);
        let second = scan(&root);
        let a = serde_json::to_string(&first).unwrap();
        let b = serde_json::to_string(&second).unwrap();
        // scan_id / 时间戳是运行期事实；比较结构性内容。
        let strip = |json: &str, discovery: &ProjectDiscovery| {
            let mut value: serde_json::Value = serde_json::from_str(json).unwrap();
            value["scan"]["scan_id"] = serde_json::json!("<id>");
            value["scan"]["started_at"] = serde_json::json!("<t>");
            value["scan"]["finished_at"] = serde_json::json!("<t>");
            value["scan"]["duration_ms"] = serde_json::json!(0);
            value["project"]["id"] = serde_json::json!("<id>");
            value["project"]["discovered_at"] = serde_json::json!("<t>");
            let _ = discovery;
            value
        };
        assert_eq!(
            strip(&a, &first),
            strip(&b, &second),
            "structural result must be deterministic"
        );
    }

    #[test]
    fn missing_root_is_fatal_error() {
        let missing = unique_root("missing-root-placeholder");
        let missing = missing.join("does-not-exist");
        let err = Project::from_root(&missing).unwrap_err();
        assert_eq!(err.code(), prism_core::PrismErrorCode::NotFound);
    }

    #[test]
    fn symlinked_directories_are_not_followed() {
        let root = unique_root("symlink");
        fs::create_dir_all(root.join("real")).unwrap();
        fs::write(root.join("real/inside.ts"), "x").unwrap();
        fs::write(root.join("top.ts"), "x").unwrap();

        let link_path = root.join("loop");
        #[cfg(unix)]
        let created = std::os::unix::fs::symlink(&root, &link_path).is_ok();
        #[cfg(windows)]
        let created = std::os::windows::fs::symlink_dir(&root, &link_path).is_ok();

        let discovery = scan(&root);
        // 无论 symlink 是否创建成功（Windows 需要权限），扫描都不能进入循环。
        assert!(discovery.scan.status == TaskState::Completed);
        if created {
            let link_entry = discovery
                .files
                .iter()
                .find(|f| f.relative_path == "loop")
                .expect("symlink must be recorded as an entry");
            assert!(link_entry.is_symlink);
            assert!(!link_entry.analyzable);
            // real/inside.ts 恰好出现一次（没有经由链接的重复遍历）。
            assert_eq!(
                discovery
                    .files
                    .iter()
                    .filter(|f| f.file_name == "inside.ts")
                    .count(),
                1
            );
        }
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn progress_callback_reports_real_counts() {
        let root = unique_root("progress");
        fs::write(root.join("a.ts"), "x").unwrap();
        fs::write(root.join("b.ts"), "x").unwrap();

        let project = Project::from_root(&root).unwrap();
        let mut last = ScanProgress::default();
        let mut calls = 0usize;
        Scanner::new(ScanOptions::default())
            .scan(&project, &CancellationToken::new(), |progress| {
                last = progress;
                calls += 1;
            })
            .unwrap();
        assert!(calls >= 2);
        assert_eq!(last.files, 2);
        assert_eq!(last.ignored, 0);
    }
}
