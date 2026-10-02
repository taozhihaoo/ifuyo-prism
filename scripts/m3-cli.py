# -*- coding: utf-8 -*-
"""M3 CLI wiring: add `prism graph` subcommand."""

import io

p = "crates/prism-cli/Cargo.toml"
s = io.open(p, encoding="utf-8", newline="\n").read()
if "prism-graph.workspace = true" not in s:
    s = s.replace(
        "prism-analysis.workspace = true",
        "prism-analysis.workspace = true\nprism-graph.workspace = true",
    )
io.open(p, "w", encoding="utf-8", newline="\n").write(s)

p = "crates/prism-cli/src/cli.rs"
s = io.open(p, encoding="utf-8", newline="\n").read()

# 1) GraphKindArg before FormatArg
if "GraphKindArg" not in s:
    s = s.replace(
        """#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum FormatArg {""",
        """#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum FormatArg {""",
    )

# 2) Graph subcommand before Health
if "Graph {" not in s:
    s = s.replace(
        """    /// 项目健康体检（M7）
    Health {""",
        """    /// 依赖与架构图（M3 可用）
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
    /// 项目健康体检（M7）
    Health {""",
    )

# 3) name()
if 'Command::Graph { .. } => "graph"' not in s:
    s = s.replace(
        '            Command::Inspect { .. } => "inspect",',
        '            Command::Inspect { .. } => "inspect",\n            Command::Graph { .. } => "graph",',
    )

io.open(p, "w", encoding="utf-8", newline="\n").write(s)
print("cli wiring done")
