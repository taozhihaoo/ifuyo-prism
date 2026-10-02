//! 通用命令执行抽象。
//!
//! M0 只建立抽象与标准库实现，不含任何 Git / Cargo / Node / Python 特定逻辑。
//!
//! 设计约束：
//! - [`CommandSpec`] 表达 program / args / working_directory / environment / timeout；
//! - 超时与取消都会终止子进程并返回错误（区分 [`PrismError::Command`] 与 [`PrismError::Cancelled`]）；
//! - 标准输出/错误由独立线程持续读取，避免管道缓冲区填满导致的死锁；
//! - 不经过 shell 启动（参数按原样传递），平台 shell 仅由调用方显式选择。

use crate::error::{PrismError, PrismResult};
use crate::task::CancellationToken;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io::Read;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

/// 一次外部命令执行的完整描述。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandSpec {
    pub program: String,
    pub args: Vec<String>,
    #[serde(default)]
    pub working_directory: Option<PathBuf>,
    /// 在继承环境之上追加的环境变量（键有序，保证序列化确定）。
    #[serde(default)]
    pub environment: BTreeMap<String, String>,
    /// 超时毫秒数；到时终止子进程并返回 Command 错误。
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
}

fn default_timeout_ms() -> u64 {
    30_000
}

impl CommandSpec {
    pub fn new(
        program: impl Into<String>,
        args: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        CommandSpec {
            program: program.into(),
            args: args.into_iter().map(Into::into).collect(),
            working_directory: None,
            environment: BTreeMap::new(),
            timeout_ms: default_timeout_ms(),
        }
    }

    pub fn with_working_directory(mut self, path: impl Into<PathBuf>) -> Self {
        self.working_directory = Some(path.into());
        self
    }

    pub fn with_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }
}

/// 一次外部命令执行的结果。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandOutput {
    /// 进程退出码；`None` 表示被信号终止等异常情况。
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
}

/// 命令执行器抽象。实现必须线程安全。
pub trait CommandRunner: Send + Sync {
    /// 执行命令。`cancel` 提供协作式取消；取消时终止子进程并返回 `Err(Cancelled)`。
    fn run(
        &self,
        spec: &CommandSpec,
        cancel: Option<&CancellationToken>,
    ) -> PrismResult<CommandOutput>;
}

/// 基于 `std::process::Command` 的默认实现。
#[derive(Debug, Default)]
pub struct StdCommandRunner;

impl StdCommandRunner {
    pub fn new() -> Self {
        Self
    }

    fn spawn(spec: &CommandSpec) -> PrismResult<Child> {
        let mut command = Command::new(&spec.program);
        command.args(&spec.args);
        if let Some(dir) = &spec.working_directory {
            command.current_dir(dir);
        }
        for (key, value) in &spec.environment {
            command.env(key, value);
        }
        command
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        // spawn 失败（程序不存在、权限不足等）属于命令层错误；
        // 不透传 io::ErrorKind::NotFound，避免与“路径类 NotFound”语义混淆。
        command.spawn().map_err(|err| PrismError::Command {
            message: format!("failed to spawn {:?}: {err}", spec.program),
        })
    }

    /// 在独立线程读取一个管道，避免子进程因缓冲区塞满而阻塞。
    fn drain(pipe: impl Read + Send + 'static) -> mpsc::Receiver<String> {
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let mut text = String::new();
            let mut pipe = pipe;
            // 读取失败按空输出处理；诊断信息通过退出码与 stderr 呈现。
            let _ = pipe.read_to_string(&mut text);
            let _ = sender.send(text);
        });
        receiver
    }

    fn finish(
        child: &mut Child,
        stdout_rx: mpsc::Receiver<String>,
        stderr_rx: mpsc::Receiver<String>,
        started: Instant,
    ) -> PrismResult<CommandOutput> {
        let status = child.wait()?;
        // 进程已退出，管道写端关闭；读取线程很快结束，超时仅作保险。
        let stdout = stdout_rx
            .recv_timeout(Duration::from_secs(10))
            .unwrap_or_default();
        let stderr = stderr_rx
            .recv_timeout(Duration::from_secs(10))
            .unwrap_or_default();
        Ok(CommandOutput {
            exit_code: status.code(),
            stdout,
            stderr,
            duration_ms: started.elapsed().as_millis() as u64,
        })
    }

    fn terminate(child: &mut Child) {
        // M0 只 kill 单进程；进程树终止（Windows job object 等）留给 M8 的 Verify 执行器。
        let _ = child.kill();
        let _ = child.wait();
    }
}

impl CommandRunner for StdCommandRunner {
    fn run(
        &self,
        spec: &CommandSpec,
        cancel: Option<&CancellationToken>,
    ) -> PrismResult<CommandOutput> {
        let started = Instant::now();
        let deadline = started + Duration::from_millis(spec.timeout_ms);
        let mut child = Self::spawn(spec)?;

        // 静态保证已设置为 piped，expect 属于“明确不可达分支”。
        let stdout_rx = Self::drain(child.stdout.take().expect("stdout was piped"));
        let stderr_rx = Self::drain(child.stderr.take().expect("stderr was piped"));

        loop {
            if cancel.is_some_and(|token| token.is_cancelled()) {
                Self::terminate(&mut child);
                return Err(PrismError::Cancelled);
            }
            match child.try_wait()? {
                Some(_status) => {
                    return Self::finish(&mut child, stdout_rx, stderr_rx, started);
                }
                None if Instant::now() >= deadline => {
                    Self::terminate(&mut child);
                    return Err(PrismError::Command {
                        message: format!(
                            "command {:?} timed out after {}ms",
                            spec.program, spec.timeout_ms
                        ),
                    });
                }
                None => std::thread::sleep(Duration::from_millis(25)),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::PrismErrorCode;

    /// 跨平台执行一段 shell 脚本（仅测试使用；业务代码不得隐式经过 shell）。
    fn shell_command(script: &str) -> CommandSpec {
        if cfg!(target_os = "windows") {
            CommandSpec::new("cmd.exe", ["/C", script])
        } else {
            CommandSpec::new("/bin/sh", ["-c", script])
        }
    }

    /// 跨平台的“长时间运行”命令，用于超时/取消测试。
    fn long_running_command() -> CommandSpec {
        if cfg!(target_os = "windows") {
            // ping 是 Windows 上无需 PowerShell 启动开销的可靠 sleep。
            CommandSpec::new("ping.exe", ["-n", "30", "127.0.0.1"])
        } else {
            CommandSpec::new("/bin/sh", ["-c", "sleep 30"])
        }
    }

    #[test]
    fn spec_serializes_with_defaults() {
        let spec = CommandSpec::new("git", ["--version"]);
        let json = serde_json::to_value(&spec).unwrap();
        assert_eq!(json["timeout_ms"], serde_json::json!(30_000));
        assert_eq!(json["program"], "git");
        assert!(json["environment"].is_object());
    }

    #[test]
    fn executes_and_captures_stdout() {
        let runner = StdCommandRunner::new();
        let output = runner.run(&shell_command("echo prism-ok"), None).unwrap();
        assert_eq!(output.exit_code, Some(0));
        assert!(output.stdout.contains("prism-ok"));
    }

    #[test]
    fn captures_nonzero_exit_and_stderr() {
        let runner = StdCommandRunner::new();
        let spec = shell_command("echo boom-failed >&2; exit 3");
        // cmd.exe 不支持 `;` 分隔与 bash 风格重定向，使用其自身语法。
        let spec = if cfg!(target_os = "windows") {
            CommandSpec::new("cmd.exe", ["/C", "echo boom-failed 1>&2 & exit /b 3"])
        } else {
            spec
        };
        let output = runner.run(&spec, None).unwrap();
        assert_eq!(output.exit_code, Some(3));
        assert!(output.stderr.contains("boom-failed"));
    }

    #[test]
    fn passes_working_directory_and_environment() {
        let runner = StdCommandRunner::new();
        let mut spec = shell_command("echo %PRISM_TEST_VAR%");
        if !cfg!(target_os = "windows") {
            spec = CommandSpec::new("/bin/sh", ["-c", "echo $PRISM_TEST_VAR"]);
        }
        spec.environment
            .insert("PRISM_TEST_VAR".to_string(), "env-works".to_string());
        let output = runner.run(&spec, None).unwrap();
        assert!(output.stdout.contains("env-works"));
    }

    #[test]
    fn unknown_program_returns_command_error() {
        let runner = StdCommandRunner::new();
        let spec = CommandSpec::new(
            "definitely-not-a-real-program-xyz",
            std::iter::empty::<String>(),
        )
        .with_timeout_ms(5_000);
        let err = runner.run(&spec, None).expect_err("must fail to spawn");
        assert_eq!(err.code(), PrismErrorCode::Command);
    }

    #[test]
    fn timeout_kills_process_and_reports() {
        let runner = StdCommandRunner::new();
        let spec = long_running_command().with_timeout_ms(400);
        let started = Instant::now();
        let err = runner.run(&spec, None).expect_err("must time out");
        assert_eq!(err.code(), PrismErrorCode::Command);
        assert!(err.message().contains("timed out"));
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "timeout must not wait for the full command"
        );
    }

    #[test]
    fn cancellation_kills_process() {
        let runner = StdCommandRunner::new();
        let token = CancellationToken::new();
        token.cancel();
        let spec = long_running_command().with_timeout_ms(30_000);
        let err = runner.run(&spec, Some(&token)).expect_err("must cancel");
        assert_eq!(err.code(), PrismErrorCode::Cancelled);
    }
}
