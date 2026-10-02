//! CLI 参数定义（clap derive）。
//!
//! 未来里程碑在此追加子命令的真实实现与参数；
//! M0 阶段除 `info` 外均为占位，明确输出未实现而不是伪造数据。

use clap::{Parser, Subcommand, ValueEnum};
use prism_core::config::OutputFormat;

#[derive(Debug, Parser)]
#[command(
    name = "prism",
    version,
    about = "ifuyo Prism — local-first project understanding workbench",
    long_about = "Prism helps you and AI understand a project: its structure, its changes, \
                  its risks, and whether it actually works.\n\n\
                  Prism is local-first and offline by default: no telemetry, no automatic \
                  network access, no AI dependency in core functionality."
)]
pub struct Cli {
    /// 输出调试日志（stderr）。
    #[arg(long, global = true)]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum FormatArg {
    Text,
    Json,
}

impl From<FormatArg> for OutputFormat {
    fn from(value: FormatArg) -> Self {
        match value {
            FormatArg::Text => OutputFormat::Text,
            FormatArg::Json => OutputFormat::Json,
        }
    }
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// 显示应用与运行时信息（真实数据，M0 可用）
    Info {
        /// 输出格式
        #[arg(long, value_enum, default_value_t = FormatArg::Text)]
        format: FormatArg,
    },
    /// 扫描项目结构并输出 Discovery 结果（M1 可用）
    Scan {
        /// 项目根目录
        #[arg(default_value = ".")]
        path: std::path::PathBuf,
        /// 输出格式
        #[arg(long, value_enum, default_value_t = FormatArg::Text)]
        format: FormatArg,
    },
    /// 检查代码符号（M2 可用）
    Inspect {
        #[arg(default_value = ".")]
        path: std::path::PathBuf,
        /// 输出格式
        #[arg(long, value_enum, default_value_t = FormatArg::Text)]
        format: FormatArg,
    },
    /// 查看变更与影响（M5/M6）
    Changes {
        #[arg(default_value = ".")]
        path: std::path::PathBuf,
    },
    /// 项目健康体检（M7）
    Health {
        #[arg(default_value = ".")]
        path: std::path::PathBuf,
    },
    /// 运行构建/测试/静态检查（M8）
    Verify {
        #[arg(default_value = ".")]
        path: std::path::PathBuf,
    },
    /// 检查开发环境（M9）
    Env {
        #[arg(default_value = ".")]
        path: std::path::PathBuf,
    },
    /// 生成 AI 上下文包（M10）
    Context {
        #[arg(default_value = ".")]
        path: std::path::PathBuf,
    },
    /// 生成报告（随各里程碑逐步提供）
    Report {
        #[arg(default_value = ".")]
        path: std::path::PathBuf,
    },
}

impl Command {
    pub fn name(&self) -> &'static str {
        match self {
            Command::Info { .. } => "info",
            Command::Scan { .. } => "scan",
            Command::Inspect { .. } => "inspect",
            Command::Changes { .. } => "changes",
            Command::Health { .. } => "health",
            Command::Verify { .. } => "verify",
            Command::Env { .. } => "env",
            Command::Context { .. } => "context",
            Command::Report { .. } => "report",
        }
    }
}
