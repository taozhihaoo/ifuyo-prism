//! 工具发现辅助（规格 M8 §19–§21）：完整实现在 prism-core
//! `runtime::lookup`（M9 §66 复用要求）；本模块仅保留兼容再导出。

pub use prism_core::runtime::lookup::{
    executable_exists, needs_windows_script_wrap, path_extensions, resolve_executable,
};
