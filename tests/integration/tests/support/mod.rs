//! 集成测试共享的小工具。

use prism_core::runtime::command::CommandSpec;
use std::path::PathBuf;

/// prism-test-helper 可执行文件的路径（cargo 自动构建并注入）。
/// 本模块被多个测试目标各自编译；未用到 helper 的目标会报 dead_code，按共享支持代码放行。
#[allow(dead_code)]
pub fn helper() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_prism-test-helper"))
}

/// 构造一个调用 helper 子命令的 CommandSpec。
#[allow(dead_code)]
pub fn helper_command(args: &[&str]) -> CommandSpec {
    let mut spec = CommandSpec::new(
        helper().to_string_lossy().to_string(),
        args.iter().map(|s| s.to_string()),
    );
    spec.timeout_ms = 15_000;
    spec
}

/// 在系统临时目录下创建一个唯一的测试目录。
pub fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "prism-integration-{name}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}
