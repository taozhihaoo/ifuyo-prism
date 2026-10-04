//! 可执行文件查找（规格 M8 §19–§21；M9 §24/§66–§67 复用）。
//!
//! 在 PATH（Windows 另含 PATHEXT）中解析可执行文件；带路径分隔符的
//! program 按相对/绝对路径直接解析。verify 与 env 共用，避免两套查找逻辑。

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
            let candidate = PathBuf::from(format!("{}{}", base.to_string_lossy(), extension));
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
pub fn path_extensions() -> Vec<String> {
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

/// Windows 上 npm/pnpm/yarn/npx/corepack 等是 `.cmd`/`.bat` shim：
/// `CreateProcess` 无法直接执行（std 自 BatBadBut 修复后不再隐式经 cmd
/// 转发）。返回 true ⇒ 需经 `cmd.exe /C <program> <args…>` 显式 argv 运行
/// （std 对 cmd.exe 参数做专用转义；不是 shell 字符串拼接）。
pub fn needs_windows_script_wrap(program: &str, working_directory: &Path) -> bool {
    if !cfg!(windows) {
        return false;
    }
    let Some(resolved) = resolve_executable(program, working_directory) else {
        return false;
    };
    resolved
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            extension.eq_ignore_ascii_case("cmd") || extension.eq_ignore_ascii_case("bat")
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cmd_shim_needs_wrap_exe_does_not() {
        let dir = std::env::temp_dir().join(format!(
            "prism-core-wrap-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.subsec_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("tool.cmd"), "@echo off\r\n").unwrap();
        let _ = std::fs::write(dir.join("tool.exe"), b"mz");

        // 带路径分隔符 ⇒ 相对 working_directory 解析（PATH 扫描不适用）。
        assert_eq!(
            needs_windows_script_wrap(".\\tool.cmd", &dir),
            cfg!(windows),
            ".cmd shim wraps only on Windows"
        );
        assert!(
            !needs_windows_script_wrap(".\\tool.exe", &dir),
            ".exe never needs the cmd wrapper"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
