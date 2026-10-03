//! 工具发现辅助（规格 M8 §19–§21）：完整实现在 prism-core
//! `runtime::lookup`（M9 §66 复用要求）；本模块保留兼容再导出 +
//! verify 特有的 Windows `.cmd` 包装判定。

use std::path::Path;

pub use prism_core::runtime::lookup::{executable_exists, path_extensions, resolve_executable};

/// Windows 上 npm/pnpm/yarn/npx 等是 `.cmd`/`.bat` shim：`CreateProcess`
/// 无法直接执行（std 自 BatBadBut 修复后也不再隐式经 cmd 转发）。
/// 返回 true ⇒ 需要经 `cmd.exe /C <program> <args…>` 运行——这是显式
/// argv 传递（std 对 cmd.exe 参数做专用转义），不是 shell 字符串拼接（§13）。
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
            "prism-tool-wrap-{}-{}",
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
