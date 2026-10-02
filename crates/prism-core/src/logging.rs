//! 统一日志。
//!
//! - 统一格式：`RFC3339 时间戳 LEVEL target — message`，单行（换行转义）。
//! - 输出到 stderr（GUI 开发环境与 CLI 均可见），可选同时写日志文件。
//! - 隐私约束：不记录完整文件内容、API Key、密码、token；本 crate 的日志
//!   只承载简短诊断信息——该规则同时约束后续所有模块（见 docs/development.md）。

use crate::error::PrismResult;
use log::{Level, Log, Metadata, Record};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// 日志文件名（位于数据目录 `logs/` 下）。
pub const LOG_FILE_NAME: &str = "prism.log";

pub struct PrismLogger {
    level: Level,
    file: Option<Mutex<File>>,
}

impl PrismLogger {
    fn format_record(&self, record: &Record<'_>) -> String {
        let timestamp = crate::Timestamp::now_utc();
        let message = record
            .args()
            .to_string()
            .replace('\r', "\\r")
            .replace('\n', "\\n");
        format!(
            "{} {:<5} {} — {}",
            timestamp.as_rfc3339(),
            record.level().to_string(),
            record.target(),
            message
        )
    }
}

impl Log for PrismLogger {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        metadata.level() <= self.level
    }

    fn log(&self, record: &Record<'_>) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let line = self.format_record(record);
        // stderr 是日志通道；写入失败时静默放弃，绝不 panic。
        let _ = writeln!(std::io::stderr(), "{line}");
        if let Some(file) = &self.file {
            if let Ok(mut handle) = file.lock() {
                let _ = writeln!(handle, "{line}");
            }
        }
    }

    fn flush(&self) {
        let _ = std::io::stderr().flush();
        if let Some(file) = &self.file {
            if let Ok(mut handle) = file.lock() {
                let _ = handle.flush();
            }
        }
    }
}

/// 安装全局日志器。重复初始化是幂等的（保留首个 logger）。
///
/// `log_file` 为 `Some(path)` 时以追加模式写入该文件（自动创建父目录）。
pub fn init(level: Level, log_file: Option<&Path>) -> PrismResult<()> {
    let file = match log_file {
        Some(path) => {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let handle = OpenOptions::new().create(true).append(true).open(path)?;
            Some(Mutex::new(handle))
        }
        None => None,
    };
    let logger = PrismLogger { level, file };
    match log::set_boxed_logger(Box::new(logger)) {
        Ok(()) => {}
        // 已有 logger（例如宿主先初始化过）时保持现状，不视为错误。
        Err(_) => {}
    }
    log::set_max_level(level.to_level_filter());
    Ok(())
}

/// 解析日志文件的标准位置：`<data_dir>/logs/prism.log`。
pub fn default_log_file(data_dir: &Path) -> PathBuf {
    data_dir.join("logs").join(LOG_FILE_NAME)
}

#[cfg(test)]
mod tests {
    use super::*;

    // 说明：log crate 全局只允许一个 logger，因此文件写入与幂等性
    // 合并在一个测试中：先安装带文件的 logger，再验证重复 init 不报错。
    #[test]
    fn init_installs_logger_writes_file_and_is_idempotent() {
        let dir = std::env::temp_dir().join(format!("prism-core-logging-{}", std::process::id()));
        let file_path = dir.join("test.log");
        init(Level::Debug, Some(&file_path)).unwrap();
        // 重复 init 必须保持幂等（保留首个 logger，不返回错误）。
        init(Level::Info, None).unwrap();
        log::info!("first line");
        log::warn!("multi\nline\nmessage");

        let contents = std::fs::read_to_string(&file_path).unwrap();
        let lines: Vec<&str> = contents.lines().collect();
        assert!(lines.iter().any(|l| l.contains("first line")));
        let multi = lines.iter().find(|l| l.contains("multi")).unwrap();
        assert!(multi.contains("\\n"), "newlines must be escaped: {multi}");
        assert_eq!(
            lines.iter().filter(|l| l.contains("multi")).count(),
            1,
            "one record = one line"
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn log_file_lives_under_data_dir() {
        let file = default_log_file(Path::new("/data"));
        assert_eq!(file, PathBuf::from("/data/logs/prism.log"));
    }
}
