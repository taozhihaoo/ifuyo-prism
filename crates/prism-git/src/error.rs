//! Git 错误模型（规格 §26）。
//!
//! 每种错误可区分、可诊断、可映射到 PrismErrorDto 的稳定 code；
//! GitUnavailable 不包装成 Internal；MalformedGitOutput 保留原始细节
//! （已脱敏：不含文件内容，只含结构性信息）。

use std::fmt;

#[derive(Debug, Clone)]
pub enum GitError {
    NotGitRepository {
        path: String,
    },
    GitUnavailable {
        message: String,
    },
    InvalidRepository {
        message: String,
    },
    PermissionDenied {
        message: String,
    },
    CommandFailed {
        command: String,
        exit_code: i32,
        stderr: String,
    },
    MalformedGitOutput {
        command: String,
        detail: String,
    },
    Timeout {
        command: String,
    },
    Cancelled,
    Unsupported {
        message: String,
    },
    Unknown {
        message: String,
    },
}

pub type GitResult<T> = Result<T, GitError>;

impl GitError {
    /// 稳定的对外错误码（PrismErrorDto.code / CLI exit 语义共用）。
    pub fn kind(&self) -> &'static str {
        match self {
            GitError::NotGitRepository { .. } => "not_git_repository",
            GitError::GitUnavailable { .. } => "git_unavailable",
            GitError::InvalidRepository { .. } => "invalid_repository",
            GitError::PermissionDenied { .. } => "permission_denied",
            GitError::CommandFailed { .. } => "command_failed",
            GitError::MalformedGitOutput { .. } => "malformed_git_output",
            GitError::Timeout { .. } => "timeout",
            GitError::Cancelled => "cancelled",
            GitError::Unsupported { .. } => "unsupported",
            GitError::Unknown { .. } => "unknown",
        }
    }

    pub fn message(&self) -> String {
        match self {
            GitError::NotGitRepository { path } => {
                format!("not a git repository: {path}")
            }
            GitError::GitUnavailable { message } => message.clone(),
            GitError::InvalidRepository { message } => message.clone(),
            GitError::PermissionDenied { message } => message.clone(),
            GitError::CommandFailed {
                command,
                exit_code,
                stderr,
            } => {
                // stderr 截断且按行折叠，避免泄漏多行内容。
                let stderr = stderr
                    .lines()
                    .next()
                    .unwrap_or("")
                    .chars()
                    .take(200)
                    .collect::<String>();
                format!("git {command} failed with exit code {exit_code}: {stderr}")
            }
            GitError::MalformedGitOutput { command, detail } => {
                format!("malformed output from git {command}: {detail}")
            }
            GitError::Timeout { command } => format!("git {command} timed out"),
            GitError::Cancelled => "operation cancelled".to_string(),
            GitError::Unsupported { message } => message.clone(),
            GitError::Unknown { message } => message.clone(),
        }
    }
}

impl fmt::Display for GitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.message(), self.kind())
    }
}

impl std::error::Error for GitError {}

impl From<GitError> for prism_core::PrismErrorDto {
    fn from(err: GitError) -> Self {
        prism_core::PrismErrorDto::new(err.kind(), err.message())
    }
}

/// 上层（M5 ChangeSet 等）统一用 PrismError；Cancelled 语义原样保留。
impl From<GitError> for prism_core::PrismError {
    fn from(err: GitError) -> Self {
        match err {
            GitError::Cancelled => prism_core::PrismError::Cancelled,
            GitError::NotGitRepository { .. } | GitError::InvalidRepository { .. } => {
                prism_core::PrismError::Git {
                    message: err.message(),
                }
            }
            other => prism_core::PrismError::Command {
                message: other.message(),
            },
        }
    }
}

impl From<prism_core::PrismError> for GitError {
    fn from(err: prism_core::PrismError) -> Self {
        use prism_core::PrismError;
        match err {
            PrismError::Cancelled => GitError::Cancelled,
            PrismError::Command { message } => GitError::CommandFailed {
                command: "git".to_string(),
                exit_code: -1,
                stderr: message,
            },
            other => GitError::Unknown {
                message: other.to_string(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_are_stable() {
        assert_eq!(
            GitError::NotGitRepository { path: "p".into() }.kind(),
            "not_git_repository"
        );
        assert_eq!(
            GitError::GitUnavailable {
                message: "m".into()
            }
            .kind(),
            "git_unavailable"
        );
    }

    #[test]
    fn command_failed_message_is_sanitized() {
        let err = GitError::CommandFailed {
            command: "status".into(),
            exit_code: 128,
            stderr: "first line\nsecond line with potential secret".into(),
        };
        let message = err.message();
        assert!(message.contains("128"));
        assert!(message.contains("first line"));
        assert!(!message.contains("second line"));
    }
}
