//! 工具发现（规格 M8 §19–§21）：resolve executable / check availability。
//! 完整 toolchain model 留 M9（§19）。

use std::path::{Path, PathBuf};

/// 在 PATH（Windows 另含 PATHEXT）中解析可执行文件。
/// 带路径分隔符的 program 按相对/绝对路径直接解析。
pub fn resolve_executable(program: &str, working_directory: &Path) -> Option<PathBuf> {
    if program.contains('/') || program.contains('\\') {
        let direct = working_directory.join(program);
        if direct.is_file() {
            return Some(direct);
        }
        return Path::new(program).is_file().then(|| PathBuf::from(program));
    }
    let path_var = std::env::var_os("PATH")?;
    let extensions = path_extensions();
    for dir in std::env::split_paths(&path_var) {
        let base = dir.join(program);
        for extension in &extensions {
            let candidate = PathBuf::from(format!(
                "{}{}",
                base.to_string_lossy(),
                extension
            ));
            if candidate.is_file() {
                return Some(candidate);
            }
        }
        if base.is_file() {
            return Some(base);
        }
    }
    None
}

/// Windows 上附加 PATHEXT（.exe/.cmd/.bat…）；其他平台空后缀即可。
fn path_extensions() -> Vec<String> {
    if cfg!(windows) {
        let mut extensions = vec![".exe".to_string()];
        if let Some(path_ext) = std::env::var_os("PATHEXT") {
            for entry in std::env::split_paths(&path_ext) {
                if let Some(text) = entry.to_str() {
                    if !text.is_empty() {
                        extensions.push(text.to_ascii_lowercase());
                    }
                }
            }
        }
        extensions
    } else {
        vec![String::new()]
    }
}

/// 可执行文件是否可用（§20 availability check）。
pub fn executable_exists(program: &str, working_directory: &Path) -> bool {
    resolve_executable(program, working_directory).is_some()
}
