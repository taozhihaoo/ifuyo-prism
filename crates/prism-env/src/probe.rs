//! ToolProbe：allowlist 化的版本/发现命令（规格 M9a §112–§117/§155/§308–§311）。
//!
//! 红线（§113）：绝不因 README/配置里的字符串执行任意命令；probe 只来自
//! 本模块静态定义的 allowlist。全部 probe 经 M0 CommandRunner（§2），
//! 带 per-probe 超时（§115）与取消（§116）；只读（§176）。

use prism_core::runtime::command::{CommandRunner, CommandSpec};
use prism_core::task::CancellationToken;
use std::time::Duration;

/// 默认 per-probe 超时（§115：极短 + bounded）。
pub const DEFAULT_PROBE_TIMEOUT_MS: u64 = 5_000;

/// probe 定义（§114 ToolProbe：program + args + timeout + expected kind）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolProbe {
    pub program: String,
    pub args: Vec<String>,
    pub timeout_ms: u64,
    pub output_kind: ProbeOutputKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeOutputKind {
    /// 首个非空行是版本串。
    VersionLine,
    /// 多行列表（如 `dotnet --list-sdks`）。
    LineList,
}

impl ToolProbe {
    pub fn version(program: &str, args: &[&str]) -> Self {
        ToolProbe {
            program: program.to_string(),
            args: args.iter().map(|arg| arg.to_string()).collect(),
            timeout_ms: DEFAULT_PROBE_TIMEOUT_MS,
            output_kind: ProbeOutputKind::VersionLine,
        }
    }
}

/// probe 结果（§156–§157：失败语义显式，不把 probe 失败当成 missing）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProbeResult {
    /// 成功；携带（可能被截断的）输出。
    Output { text: String, output_truncated: bool },
    /// 非零退出（§156：exists 但 probe failed）。
    Failed { exit_code: Option<i32>, stderr_tail: String },
    /// 超时（§308 ProbeTimedOut）。
    TimedOut,
    /// 扫描取消（§116）。
    Cancelled,
    /// 无法 spawn（程序消失等）。
    SpawnFailed { message: String },
}

/// 执行一次 allowlist probe。
pub fn run_probe(
    probe: &ToolProbe,
    runner: &dyn CommandRunner,
    working_directory: &std::path::Path,
    cancel: &CancellationToken,
) -> ProbeResult {
    // Windows `.cmd`/`.bat` shim（npm/pnpm/corepack…）经 cmd.exe /C
    // 显式 argv 运行（§112 安全边界不变：program 仍来自 allowlist）。
    let (program, args) =
        if prism_core::runtime::lookup::needs_windows_script_wrap(&probe.program, working_directory)
        {
            let mut wrapped = vec!["/C".to_string(), probe.program.clone()];
            wrapped.extend(probe.args.iter().cloned());
            ("cmd.exe".to_string(), wrapped)
        } else {
            (probe.program.clone(), probe.args.clone())
        };

    let mut command_spec = CommandSpec::new(program, args)
        .with_working_directory(working_directory.to_path_buf())
        .with_timeout_ms(probe.timeout_ms);
    // probe 输出有界（§311 max_output）：64 KiB 足够版本行/SDK 列表。
    command_spec.max_output_bytes = Some(64 * 1024);

    match runner.run(&command_spec, Some(cancel)) {
        Ok(output) => {
            if output.exit_code == Some(0) {
                ProbeResult::Output {
                    text: output.stdout,
                    output_truncated: output.output_truncated,
                }
            } else {
                ProbeResult::Failed {
                    exit_code: output.exit_code,
                    stderr_tail: tail(&output.stderr, 400),
                }
            }
        }
        Err(prism_core::PrismError::Cancelled) => ProbeResult::Cancelled,
        Err(prism_core::PrismError::Command { message }) if message.contains("timed out") => {
            ProbeResult::TimedOut
        }
        Err(prism_core::PrismError::Command { message }) => {
            ProbeResult::SpawnFailed { message }
        }
        Err(err) => ProbeResult::SpawnFailed {
            message: err.to_string(),
        },
    }
}

fn tail(text: &str, max_chars: usize) -> String {
    let trimmed = text.trim();
    if trimmed.len() <= max_chars {
        trimmed.to_string()
    } else {
        trimmed[trimmed.len() - max_chars..].to_string()
    }
}

/// 从 probe 输出提取版本行（VersionLine 语义：首个非空行；stdout 优先，
/// stderr 兜底——`ssh -V` 等把版本打到 stderr）。
pub fn version_line(stdout: &str, stderr: &str) -> Option<String> {
    stdout
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .or_else(|| stderr.lines().map(str::trim).find(|line| !line.is_empty()))
        .map(str::to_string)
}

/// probe 超时常量导出（Duration 形态，供调度器做总预算核算，§309）。
pub fn probe_timeout(probe: &ToolProbe) -> Duration {
    Duration::from_millis(probe.timeout_ms)
}
