//! 统一错误模型。
//!
//! 所有业务路径必须返回 `PrismResult<T>`；禁止 `unwrap()` / `expect()` / `panic!()`
//! 出现在业务路径中（测试与静态常量初始化除外）。
//!
//! 错误必须：可区分（[`PrismErrorCode`]）、可记录、可传给 CLI、可安全传给 GUI。

use std::fmt;

/// 错误类别码。对外 JSON 契约的一部分，字符串值必须保持稳定。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PrismErrorCode {
    InvalidInput,
    NotFound,
    Io,
    Parse,
    Git,
    Command,
    Config,
    Serialization,
    Cancelled,
    Internal,
    Unsupported,
}

impl PrismErrorCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            PrismErrorCode::InvalidInput => "invalid_input",
            PrismErrorCode::NotFound => "not_found",
            PrismErrorCode::Io => "io",
            PrismErrorCode::Parse => "parse",
            PrismErrorCode::Git => "git",
            PrismErrorCode::Command => "command",
            PrismErrorCode::Config => "config",
            PrismErrorCode::Serialization => "serialization",
            PrismErrorCode::Cancelled => "cancelled",
            PrismErrorCode::Internal => "internal",
            PrismErrorCode::Unsupported => "unsupported",
        }
    }
}

/// Prism 统一错误。
#[derive(Debug, Clone)]
pub enum PrismError {
    InvalidInput {
        message: String,
    },
    NotFound {
        message: String,
    },
    Io {
        message: String,
    },
    Parse {
        message: String,
    },
    Git {
        message: String,
    },
    Command {
        message: String,
    },
    Config {
        message: String,
    },
    Serialization {
        message: String,
    },
    /// 操作被用户或调用方取消；这不是故障，不应当作错误上报给用户。
    Cancelled,
    /// Prism 自身缺陷或不可达分支；发生即视为 bug。
    Internal {
        message: String,
    },
    /// 当前里程碑 / 当前平台尚未支持的能力。
    Unsupported {
        message: String,
    },
}

impl PrismError {
    pub fn code(&self) -> PrismErrorCode {
        match self {
            PrismError::InvalidInput { .. } => PrismErrorCode::InvalidInput,
            PrismError::NotFound { .. } => PrismErrorCode::NotFound,
            PrismError::Io { .. } => PrismErrorCode::Io,
            PrismError::Parse { .. } => PrismErrorCode::Parse,
            PrismError::Git { .. } => PrismErrorCode::Git,
            PrismError::Command { .. } => PrismErrorCode::Command,
            PrismError::Config { .. } => PrismErrorCode::Config,
            PrismError::Serialization { .. } => PrismErrorCode::Serialization,
            PrismError::Cancelled => PrismErrorCode::Cancelled,
            PrismError::Internal { .. } => PrismErrorCode::Internal,
            PrismError::Unsupported { .. } => PrismErrorCode::Unsupported,
        }
    }

    pub fn message(&self) -> &str {
        match self {
            PrismError::Cancelled => "operation cancelled",
            PrismError::InvalidInput { message }
            | PrismError::NotFound { message }
            | PrismError::Io { message }
            | PrismError::Parse { message }
            | PrismError::Git { message }
            | PrismError::Command { message }
            | PrismError::Config { message }
            | PrismError::Serialization { message }
            | PrismError::Internal { message }
            | PrismError::Unsupported { message } => message,
        }
    }

    pub fn invalid_input(message: impl Into<String>) -> Self {
        PrismError::InvalidInput {
            message: message.into(),
        }
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        PrismError::NotFound {
            message: message.into(),
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        PrismError::Internal {
            message: message.into(),
        }
    }

    pub fn unsupported(message: impl Into<String>) -> Self {
        PrismError::Unsupported {
            message: message.into(),
        }
    }
}

impl fmt::Display for PrismError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.message(), self.code().as_str())
    }
}

impl std::error::Error for PrismError {}

impl From<std::io::Error> for PrismError {
    fn from(err: std::io::Error) -> Self {
        if err.kind() == std::io::ErrorKind::NotFound {
            PrismError::NotFound {
                message: err.to_string(),
            }
        } else {
            PrismError::Io {
                message: err.to_string(),
            }
        }
    }
}

impl From<serde_json::Error> for PrismError {
    fn from(err: serde_json::Error) -> Self {
        PrismError::Serialization {
            message: err.to_string(),
        }
    }
}

/// 全项目统一 `Result` 别名。
pub type PrismResult<T> = Result<T, PrismError>;

/// 错误的对外 JSON 形态（IPC / CLI JSON 输出）。字段稳定，禁止随意增删。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PrismErrorDto {
    pub code: String,
    pub message: String,
}

impl PrismErrorDto {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        PrismErrorDto {
            code: code.into(),
            message: message.into(),
        }
    }
}

impl From<&PrismError> for PrismErrorDto {
    fn from(err: &PrismError) -> Self {
        PrismErrorDto {
            code: err.code().as_str().to_string(),
            message: err.message().to_string(),
        }
    }
}

impl From<PrismError> for PrismErrorDto {
    fn from(err: PrismError) -> Self {
        PrismErrorDto::from(&err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_strings_are_stable() {
        assert_eq!(PrismErrorCode::InvalidInput.as_str(), "invalid_input");
        assert_eq!(PrismErrorCode::Cancelled.as_str(), "cancelled");
        assert_eq!(PrismErrorCode::Unsupported.as_str(), "unsupported");
    }

    #[test]
    fn display_contains_message_and_code() {
        let err = PrismError::invalid_input("root path is empty");
        let text = err.to_string();
        assert!(text.contains("root path is empty"));
        assert!(text.contains("invalid_input"));
    }

    #[test]
    fn io_error_kind_not_found_maps_to_not_found() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "missing file");
        let err = PrismError::from(io_err);
        assert_eq!(err.code(), PrismErrorCode::NotFound);
    }

    #[test]
    fn dto_conversion_preserves_code_and_message() {
        let err = PrismError::unsupported("scan is coming in M1");
        let dto = PrismErrorDto::from(&err);
        assert_eq!(dto.code, "unsupported");
        assert_eq!(dto.message, "scan is coming in M1");

        let json = serde_json::to_string(&dto).unwrap();
        let back: PrismErrorDto = serde_json::from_str(&json).unwrap();
        assert_eq!(back, dto);
    }

    #[test]
    fn cancelled_has_no_payload() {
        let err = PrismError::Cancelled;
        assert_eq!(err.code(), PrismErrorCode::Cancelled);
        assert_eq!(err.message(), "operation cancelled");
    }
}
