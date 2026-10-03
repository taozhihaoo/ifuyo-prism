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
pub enum GraphKindArg {
    File,
    Module,
    Project,
}

impl GraphKindArg {
    pub const fn as_str(self) -> &'static str {
        match self {
            GraphKindArg::File => "file",
            GraphKindArg::Module => "module",
            GraphKindArg::Project => "project",
        }
    }
}

/// `prism changes` 独有格式（含 markdown，规格 M5 §66）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ChangesFormatArg {
    Text,
    Json,
    Markdown,
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

/// `prism health` 子命令（M7a 仅 rules；M7b 增加完整分析）。
#[derive(Debug, Subcommand)]
pub enum HealthCommand {
    /// 列出内建规则与合并配置后的最终开关状态
    Rules {
        /// JSON 输出
        #[arg(long)]
        json: bool,
    },
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
    /// 代码级变更集：Git diff → 文件/符号/模块变更（M5 可用）
    Changes {
        #[arg(default_value = ".")]
        path: std::path::PathBuf,
        /// 比较 index 与 HEAD（staged），默认 HEAD → 工作区
        #[arg(long)]
        staged: bool,
        /// RevisionRange 起点（与 --to 搭配，如 HEAD~1）
        #[arg(long)]
        from: Option<String>,
        /// RevisionRange 终点（默认 HEAD）
        #[arg(long)]
        to: Option<String>,
        /// 输出格式
        #[arg(long, value_enum, default_value_t = ChangesFormatArg::Text)]
        format: ChangesFormatArg,
        /// 文件数上限（超出截断并标注）
        #[arg(long, default_value_t = 200)]
        max_files: usize,
    },
    /// 影响分析：ChangeSet × 关系图 → 潜在波及 + 证据（M6 可用）
    Impact {
        #[arg(default_value = ".")]
        path: std::path::PathBuf,
        /// 比较 index 与 HEAD（staged），默认 HEAD → 工作区
        #[arg(long)]
        staged: bool,
        /// RevisionRange 起点（与 --to 搭配，如 HEAD~1）
        #[arg(long)]
        from: Option<String>,
        /// RevisionRange 终点（默认 HEAD）
        #[arg(long)]
        to: Option<String>,
        /// 输出格式
        #[arg(long, value_enum, default_value_t = ChangesFormatArg::Text)]
        format: ChangesFormatArg,
        /// 变更文件数上限（透传 ChangeSet）
        #[arg(long, default_value_t = 200)]
        max_files: usize,
        /// 反向遍历最大深度（关系跳数）
        #[arg(long, default_value_t = 10)]
        max_depth: u32,
        /// 受影响节点上限
        #[arg(long, default_value_t = 2000)]
        max_nodes: u64,
        /// 收集测试候选
        #[arg(long, default_value_t = true)]
        include_tests: bool,
        /// 收集文档候选（无证据则记诊断）
        #[arg(long, default_value_t = true)]
        include_docs: bool,
    },
    /// 依赖与架构图（M3 可用）
    Graph {
        #[arg(default_value = ".")]
        path: std::path::PathBuf,
        /// 图层级：file / module / project
        #[arg(long, value_enum, default_value_t = GraphKindArg::File)]
        kind: GraphKindArg,
        /// 输出格式
        #[arg(long, value_enum, default_value_t = FormatArg::Text)]
        format: FormatArg,
    },
    /// Git 事实（status/log/diff/refs，M4 只读）
    Git {
        #[arg(default_value = ".")]
        path: std::path::PathBuf,
        #[command(subcommand)]
        command: crate::git_command::GitCommand,
    },
    /// 项目健康（M7a：规则检视；完整分析在 M7b）
    Health {
        #[arg(default_value = ".")]
        path: std::path::PathBuf,
        #[command(subcommand)]
        command: HealthCommand,
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
            Command::Graph { .. } => "graph",
            Command::Git { .. } => "git",
            Command::Impact { .. } => "impact",
            Command::Changes { .. } => "changes",
            Command::Health { .. } => "health",
            Command::Verify { .. } => "verify",
            Command::Env { .. } => "env",
            Command::Context { .. } => "context",
            Command::Report { .. } => "report",
        }
    }
}
