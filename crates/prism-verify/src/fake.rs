//! FakeCommandRunner 测试基座（规格 M8 §180–§181）。
//!
//! 脚本化响应：按 `program` 匹配返回预置 `CommandOutput` 或错误；
//! 未命中走默认行为（exit 0 / 空输出）。不真实 spawn 进程。

use prism_core::runtime::command::{CommandOutput, CommandSpec, CommandRunner};
use prism_core::task::CancellationToken;
use prism_core::PrismError;
use prism_core::PrismResult;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub enum FakeResponse {
    /// exit_code + stdout + stderr（§181 success/non-zero/stderr/stdout）。
    Output {
        exit_code: i32,
        stdout: String,
        stderr: String,
    },
    /// spawn 失败（§181 not found / permission denied）。
    SpawnFailed { message: String },
    /// 超时（§181 timeout）。
    TimedOut,
    /// 取消（§181 cancelled）。
    Cancelled,
}

/// 内部共享状态（Clone 共享同一脚本与调用记录——Arc 语义符合测试需要）。
#[derive(Default, Clone)]
struct FakeState {
    scripted: BTreeMap<String, FakeResponse>,
    received: Vec<CommandSpec>,
}

#[derive(Default, Clone)]
pub struct FakeCommandRunner {
    state: Arc<Mutex<FakeState>>,
}

impl FakeCommandRunner {
    pub fn new() -> Self {
        Self::default()
    }

    /// 为某 program 脚本化响应。
    pub fn script(&mut self, program: &str, response: FakeResponse) -> &mut Self {
        self.state
            .lock()
            .unwrap()
            .scripted
            .insert(program.to_string(), response);
        self
    }

    pub fn calls(&self) -> Vec<CommandSpec> {
        self.state.lock().unwrap().received.clone()
    }
}

impl CommandRunner for FakeCommandRunner {
    fn run(
        &self,
        spec: &CommandSpec,
        cancel: Option<&CancellationToken>,
    ) -> PrismResult<CommandOutput> {
        self.state.lock().unwrap().received.push(spec.clone());
        if let Some(cancel) = cancel {
            if cancel.is_cancelled() {
                return Err(PrismError::Cancelled);
            }
        }
        let response = self
            .state
            .lock()
            .unwrap()
            .scripted
            .get(&spec.program)
            .cloned();
        match response {
            Some(FakeResponse::Output { exit_code, stdout, stderr }) => {
                Ok(CommandOutput {
                    exit_code: Some(exit_code),
                    stdout,
                    stderr,
                    duration_ms: 5,
                    output_truncated: false,
                })
            }
            Some(FakeResponse::SpawnFailed { message }) => Err(PrismError::Command { message }),
            Some(FakeResponse::TimedOut) => Err(PrismError::Command {
                message: format!("command {:?} timed out after {}ms", spec.program, spec.timeout_ms),
            }),
            Some(FakeResponse::Cancelled) => Err(PrismError::Cancelled),
            None => Ok(CommandOutput {
                exit_code: Some(0),
                stdout: String::new(),
                stderr: String::new(),
                duration_ms: 1,
                output_truncated: false,
            }),
        }
    }
}
