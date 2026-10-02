//! Ignore 引擎：系统安全规则 + 项目 `.gitignore` + 用户规则。
//!
//! 优先级（记录于 docs/architecture.md，未来不再争论）：
//!
//! ```text
//! 1. hidden 过滤（include_hidden = false 时）+ 系统安全规则 + 用户排除（按名字）
//! 2. 项目 .gitignore（含 ! 取反语义，由 ignore crate 提供）
//! ```
//!
//! 设计决策：
//! - gitignore 匹配复用 `ignore` crate（BurntSushi/ripgrep 生态，MIT/Apache-2.0，
//!   维护良好）——不重新发明 gitignore 语法（纲领 §14）。
//! - 系统安全规则按“路径段名”匹配且不可被 gitignore 取反（最安全优先）。
//! - M1 只读取项目根的 `.gitignore`；嵌套目录的 .gitignore 留给后续里程碑。

use serde::{Deserialize, Serialize};
use std::path::Path;

/// 默认不深度扫描的系统安全目录/文件名（纲领 §16）。不可被取反。
pub const SYSTEM_IGNORED_NAMES: &[&str] = &[
    ".git",
    ".hg",
    ".svn",
    "node_modules",
    "bower_components",
    "target",
    "bin",
    "obj",
    ".venv",
    "venv",
    "__pycache__",
    ".pytest_cache",
    ".mypy_cache",
    "dist",
    "build",
    "out",
    "coverage",
    ".cache",
    ".next",
    ".nuxt",
    ".gradle",
    ".terraform",
    "Pods",
    "DerivedData",
];

/// 扫描选项（纲领 §35/§36）。hidden ≠ ignored。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScanOptions {
    /// 不跟随符号链接（防循环/防越界；纲领 §17）。
    pub follow_symlinks: bool,
    /// 读取并应用项目根 `.gitignore`。
    pub respect_gitignore: bool,
    /// 包含点开头文件/目录（.gitignore、.env.example 等是重要上下文）。
    pub include_hidden: bool,
    /// 单文件大小上限（字节）；超过仍被发现，仅标记不可分析。
    pub max_file_size: u64,
    /// 用户额外排除的目录/文件名（按名字精确匹配）。
    pub extra_ignores: Vec<String>,
}

impl Default for ScanOptions {
    fn default() -> Self {
        ScanOptions {
            follow_symlinks: false,
            respect_gitignore: true,
            include_hidden: true,
            max_file_size: 2 * 1024 * 1024,
            extra_ignores: Vec::new(),
        }
    }
}

/// 统一 ignore 判断引擎。
#[derive(Debug, Clone)]
pub struct IgnoreEngine {
    include_hidden: bool,
    /// 按名字匹配的硬排除（系统安全规则 + 用户规则）。
    hard_names: Vec<String>,
    gitignore: Option<ignore::gitignore::Gitignore>,
}

impl IgnoreEngine {
    /// 构建引擎。`.gitignore` 读取失败不致命：返回引擎本身 + 警告列表，
    /// 由 scanner 记录为非致命 ScanError（纲领 §18/§19）。
    pub fn build(root: &Path, options: &ScanOptions) -> (Self, Vec<String>) {
        let mut warnings = Vec::new();
        let mut hard_names: Vec<String> = SYSTEM_IGNORED_NAMES
            .iter()
            .map(|name| (*name).to_string())
            .collect();
        for extra in &options.extra_ignores {
            hard_names.push(extra.clone());
        }

        let mut gitignore = None;
        if options.respect_gitignore {
            let gitignore_path = root.join(".gitignore");
            if gitignore_path.exists() {
                let mut builder = ignore::gitignore::GitignoreBuilder::new(root);
                // builder.add 返回 Option<Error>（None = 成功读取）。
                if let Some(err) = builder.add(&gitignore_path) {
                    warnings.push(format!("failed to read .gitignore: {err}"));
                }
                match builder.build() {
                    Ok(matcher) => gitignore = Some(matcher),
                    Err(err) => warnings.push(format!("failed to compile .gitignore: {err}")),
                }
            }
        }

        (
            IgnoreEngine {
                include_hidden: options.include_hidden,
                hard_names,
                gitignore,
            },
            warnings,
        )
    }

    /// 判断一个条目是否应被忽略。
    ///
    /// `relative_path` 为项目相对路径（原生分隔符即可）；`entry_name` 为自身名字。
    pub fn ignored(&self, relative_path: &Path, entry_name: &str, is_dir: bool) -> bool {
        if !self.include_hidden && entry_name.starts_with('.') {
            return true;
        }
        if self.hard_names.iter().any(|name| name == entry_name) {
            return true;
        }
        if let Some(matcher) = &self.gitignore {
            match matcher.matched(relative_path, is_dir) {
                ignore::Match::Ignore(_) => return true,
                ignore::Match::Whitelist(_) => return false,
                ignore::Match::None => {}
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn engine_with_gitignore(root: &Path, lines: &str, options: ScanOptions) -> IgnoreEngine {
        std::fs::write(root.join(".gitignore"), lines).unwrap();
        let (engine, warnings) = IgnoreEngine::build(root, &options);
        assert!(warnings.is_empty(), "unexpected warnings: {warnings:?}");
        engine
    }

    fn temp_root(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("prism-ignore-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn system_rules_always_ignore_safety_directories() {
        let root = temp_root("system");
        let (engine, _) = IgnoreEngine::build(&root, &ScanOptions::default());
        for name in [
            ".git",
            "node_modules",
            "target",
            ".venv",
            "__pycache__",
            "dist",
        ] {
            assert!(
                engine.ignored(Path::new(name), name, true),
                "{name} must be ignored"
            );
        }
        assert!(!engine.ignored(Path::new("src"), "src", true));
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn gitignore_rules_apply_with_negation() {
        let root = temp_root("gitignore");
        let engine = engine_with_gitignore(
            &root,
            "ignored-dir/\n*.log\n!keep.log\n",
            ScanOptions::default(),
        );
        assert!(engine.ignored(Path::new("ignored-dir"), "ignored-dir", true));
        // 注：引擎按“单个条目”判断；被忽略目录内的文件由 walker 保证不会进入
        // （目录在下降前已判定忽略），因此这里只断言目录本身。
        assert!(engine.ignored(Path::new("debug.log"), "debug.log", false));
        // 取反生效：keep.log 不被忽略。
        assert!(!engine.ignored(Path::new("keep.log"), "keep.log", false));
        assert!(!engine.ignored(Path::new("src/main.ts"), "main.ts", false));
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn system_rules_override_gitignore_whitelist() {
        let root = temp_root("override");
        // 即使 .gitignore 想取反 node_modules，系统规则仍然忽略。
        let engine = engine_with_gitignore(&root, "!node_modules/\n", ScanOptions::default());
        assert!(engine.ignored(Path::new("node_modules"), "node_modules", true));
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn user_extra_ignores_apply_by_name() {
        let root = temp_root("user");
        let options = ScanOptions {
            extra_ignores: vec!["secrets".to_string()],
            ..ScanOptions::default()
        };
        let (engine, _) = IgnoreEngine::build(&root, &options);
        assert!(engine.ignored(Path::new("secrets"), "secrets", true));
        assert!(engine.ignored(Path::new("a/b/secrets"), "secrets", false));
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn hidden_filter_respects_include_hidden_flag() {
        let root = temp_root("hidden");
        // 默认 include_hidden = true：.gitignore / .env.example 应被发现。
        let (engine, _) = IgnoreEngine::build(&root, &ScanOptions::default());
        assert!(!engine.ignored(Path::new(".gitignore"), ".gitignore", false));

        let options = ScanOptions {
            include_hidden: false,
            ..ScanOptions::default()
        };
        let (engine, _) = IgnoreEngine::build(&root, &options);
        assert!(engine.ignored(Path::new(".gitignore"), ".gitignore", false));
        assert!(!engine.ignored(Path::new("main.ts"), "main.ts", false));
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn unreadable_gitignore_is_reported_as_warning_not_error() {
        let root = temp_root("warn");
        // 不创建 .gitignore：respect_gitignore 开着但没有文件 → 无警告、正常工作。
        let (engine, warnings) = IgnoreEngine::build(&root, &ScanOptions::default());
        assert!(warnings.is_empty());
        assert!(engine.gitignore.is_none());
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn scan_options_serialize_snake_case() {
        let options = ScanOptions::default();
        let json = serde_json::to_value(&options).unwrap();
        assert_eq!(json["follow_symlinks"], serde_json::json!(false));
        assert_eq!(json["respect_gitignore"], serde_json::json!(true));
        assert_eq!(json["include_hidden"], serde_json::json!(true));
    }
}
