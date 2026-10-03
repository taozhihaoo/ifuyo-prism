//! 可执行文件发现（规格 M9a §22–§27/§62–§70/§120–§123/§154/§196）。
//!
//! 策略（§24）：PATH lookup → known platform locations → ecosystem-specific；
//! **禁止全盘扫描**（§25）。发现范围按项目 profile 惰性选择（§120/§123），
//! 未知项目至少 git + 常见工具可用性（§122 Generic mode）。

use crate::model::{ProbeOutcome, ToolFact, ToolSource};
use crate::probe::{run_probe, version_line, ProbeResult, ToolProbe};
use prism_core::runtime::lookup::resolve_executable;
use prism_core::task::CancellationToken;
use prism_core::runtime::command::CommandRunner;
use std::path::{Path, PathBuf};

/// 工具定义（§23 allowlist；§112–§114 只允许这里的 version 命令）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToolDefinition {
    /// 稳定逻辑名（§23）。
    pub name: &'static str,
    /// 实际可执行程序名（多数与 name 相同）。
    pub program: &'static str,
    /// 版本 probe 参数（§112：只用 --version/-version/version 形态）。
    pub version_args: &'static [&'static str],
}

/// §23 allowlist（实际使用范围按项目 profile 决定，§23 "以真实项目为准"）。
pub const TOOL_ALLOWLIST: &[ToolDefinition] = &[
    ToolDefinition { name: "node", program: "node", version_args: &["--version"] },
    ToolDefinition { name: "npm", program: "npm", version_args: &["--version"] },
    ToolDefinition { name: "pnpm", program: "pnpm", version_args: &["--version"] },
    ToolDefinition { name: "yarn", program: "yarn", version_args: &["--version"] },
    ToolDefinition { name: "bun", program: "bun", version_args: &["--version"] },
    ToolDefinition { name: "python", program: "python", version_args: &["--version"] },
    ToolDefinition { name: "python3", program: "python3", version_args: &["--version"] },
    ToolDefinition { name: "pip", program: "pip", version_args: &["--version"] },
    ToolDefinition { name: "uv", program: "uv", version_args: &["--version"] },
    ToolDefinition { name: "poetry", program: "poetry", version_args: &["--version"] },
    ToolDefinition { name: "dotnet", program: "dotnet", version_args: &["--version"] },
    ToolDefinition { name: "rustc", program: "rustc", version_args: &["--version"] },
    ToolDefinition { name: "cargo", program: "cargo", version_args: &["--version"] },
    ToolDefinition { name: "rustup", program: "rustup", version_args: &["--version"] },
    ToolDefinition { name: "go", program: "go", version_args: &["version"] },
    ToolDefinition { name: "java", program: "java", version_args: &["-version"] },
    ToolDefinition { name: "git", program: "git", version_args: &["--version"] },
    ToolDefinition { name: "docker", program: "docker", version_args: &["--version"] },
    ToolDefinition { name: "corepack", program: "corepack", version_args: &["--version"] },
    ToolDefinition { name: "gcc", program: "gcc", version_args: &["--version"] },
    ToolDefinition { name: "cmake", program: "cmake", version_args: &["--version"] },
    ToolDefinition { name: "ninja", program: "ninja", version_args: &["--version"] },
    ToolDefinition { name: "ssh", program: "ssh", version_args: &["-V"] },
];

pub fn definition(name: &str) -> Option<&'static ToolDefinition> {
    TOOL_ALLOWLIST.iter().find(|definition| definition.name == name)
}

/// 项目 profile（§121）：由 M1 discovery 启发式推导，决定惰性发现范围。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnvironmentProfile {
    pub node: bool,
    pub python: bool,
    pub dotnet: bool,
    pub rust: bool,
    pub go: bool,
    pub container: bool,
}

impl EnvironmentProfile {
    /// §122 Generic：未知项目至少 git 可用，不失败。
    pub fn generic() -> Self {
        EnvironmentProfile {
            node: false,
            python: false,
            dotnet: false,
            rust: false,
            go: false,
            container: false,
        }
    }

    /// 由 M1 discovery 推导（§121/§152–§153：复用项目事实，不重扫）。
    pub fn from_discovery(discovery: &prism_project::ProjectDiscovery) -> Self {
        let has = |relative: &str| {
            discovery
                .files
                .iter()
                .any(|file| file.relative_path == relative)
        };
        let has_suffix = |suffix: &str| {
            discovery
                .files
                .iter()
                .any(|file| file.relative_path.ends_with(suffix))
        };
        EnvironmentProfile {
            node: has("package.json"),
            python: has("pyproject.toml")
                || has("requirements.txt")
                || has("setup.py")
                || has("pytest.ini"),
            dotnet: has_suffix(".csproj") || has_suffix(".sln") || has("global.json"),
            rust: has("Cargo.toml"),
            go: has("go.mod"),
            container: has("Dockerfile") || has("docker-compose.yml"),
        }
    }

    /// 项目相关工具（§120/§123：project-relevant discovery）。
    pub fn tool_names(&self) -> Vec<&'static str> {
        let mut names = vec!["git"];
        if self.node {
            names.extend(["node", "npm", "pnpm", "yarn", "bun", "corepack"]);
        }
        if self.python {
            names.extend(["python", "python3", "pip", "uv", "poetry"]);
        }
        if self.dotnet {
            names.push("dotnet");
        }
        if self.rust {
            names.extend(["rustc", "cargo", "rustup"]);
        }
        if self.go {
            names.push("go");
        }
        if self.container {
            names.push("docker");
        }
        names.sort();
        names.dedup();
        names
    }
}

/// 发现一个工具（§22/§24/§26/§65/§66）：解析 active 路径 + 同名候选，
/// 再按 allowlist probe 版本。
pub fn discover_tool(
    definition: &ToolDefinition,
    runner: &dyn CommandRunner,
    working_directory: &Path,
    cancel: &CancellationToken,
) -> ToolFact {
    let (resolved, additional) = resolve_on_path(definition.program, working_directory);
    let Some(active_path) = resolved else {
        // §209：只有 lookup 确认 not found 才算 NotFound。
        return ToolFact {
            name: definition.name.to_string(),
            path: None,
            additional_paths: Vec::new(),
            version: None,
            raw_version: None,
            source: ToolSource::Unknown,
            probe: ProbeOutcome::NotFound,
        };
    };

    // probe（§112–§115）：allowlist 命令 + 有界超时 + 取消传播。
    let probe_def = ToolProbe::version(definition.program, definition.version_args);
    let (version, raw_version, probe_outcome) = match run_probe(&probe_def, runner, working_directory, cancel) {
        ProbeResult::Output { text, output_truncated: _ } => {
            let raw = version_line(&text, "");
            let parsed = raw.as_deref().and_then(crate::version::parse_version_output);
            if parsed.is_some() {
                (parsed, raw, ProbeOutcome::Succeeded)
            } else {
                // 输出成功但解析不出版本 ⇒ version unknown（§29 保守）。
                (None, raw, ProbeOutcome::Succeeded)
            }
        }
        ProbeResult::Failed { .. } | ProbeResult::TimedOut | ProbeResult::SpawnFailed { .. } => {
            // §157：exists = true，version unknown，probe failed。
            (None, None, ProbeOutcome::Failed)
        }
        ProbeResult::Cancelled => (None, None, ProbeOutcome::Failed),
    };

    ToolFact {
        name: definition.name.to_string(),
        path: Some(active_path.to_string_lossy().to_string()),
        additional_paths: additional,
        version,
        raw_version: raw_version.map(|line| {
            prism_core::privacy::redact_text(&line).trim().to_string()
        }),
        source: ToolSource::Path,
        probe: probe_outcome,
    }
}

/// PATH 解析（§66 ExecutableResolution）：active = 第一个命中（模拟
/// 用户 shell 的解析顺序），同名候选记录（§65 shadowing 事实，不判对错）。
fn resolve_on_path(program: &str, working_directory: &Path) -> (Option<PathBuf>, Vec<String>) {
    let active = resolve_executable(program, working_directory);
    let mut additional = Vec::new();
    if let Some(path_var) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path_var) {
            let mut candidates: Vec<PathBuf> = vec![dir.join(program)];
            if cfg!(windows) {
                for extension in prism_core::runtime::lookup::path_extensions() {
                    candidates.push(PathBuf::from(format!(
                        "{}.{}",
                        dir.join(program).to_string_lossy(),
                        extension
                    )));
                }
            }
            for candidate in candidates {
                if candidate.is_file() {
                    match &active {
                        Some(active_path) if candidate != *active_path => {
                            if additional.len() < 5 {
                                additional.push(candidate.to_string_lossy().to_string());
                            }
                        }
                        Some(_) => {}
                        None => {}
                    }
                }
            }
        }
    }
    (active, additional)
}

#[cfg(test)]
mod tests {
    use super::*;
    use prism_core::runtime::command::StdCommandRunner;

    #[test]
    fn allowlist_covers_key_ecosystems() {
        for name in ["node", "npm", "pnpm", "python", "dotnet", "cargo", "git", "docker"] {
            assert!(definition(name).is_some(), "{name} must be in allowlist");
        }
        // §23：每个定义的 probe 都是版本形态。
        for definition in TOOL_ALLOWLIST {
            assert!(
                definition
                    .version_args
                    .iter()
                    .all(|arg| arg.starts_with('-') || *arg == "version"),
                "{} probe must be version-shaped: {:?}",
                definition.name,
                definition.version_args
            );
        }
    }

    #[test]
    fn generic_profile_covers_git_only() {
        let profile = EnvironmentProfile::generic();
        assert_eq!(profile.tool_names(), vec!["git"]);
    }

    #[test]
    fn profile_from_discovery_is_lazy() {
        let dir = std::env::temp_dir().join(format!(
            "prism-env-profile-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.subsec_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("package.json"), "{}").unwrap();
        std::fs::write(dir.join("Cargo.toml"), "[package]\nname=\"x\"\n").unwrap();
        let cancel = CancellationToken::new();
        let discovery = prism_project::DiscoveryService::new()
            .discover(&dir, prism_project::ScanOptions::default(), &cancel, |_| {})
            .unwrap();
        let profile = EnvironmentProfile::from_discovery(&discovery);
        assert!(profile.node && profile.rust);
        assert!(!profile.python && !profile.dotnet);
        let names = profile.tool_names();
        assert!(names.contains(&"node") && names.contains(&"cargo") && names.contains(&"git"));
        assert!(!names.contains(&"dotnet"), "§120 lazy: not relevant");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_tool_is_not_found_not_failed() {
        // §209：lookup 确认不存在 ⇒ NotFound（而非 probe failed）。
        let probe_definition = ToolDefinition {
            name: "definitely-not-a-real-tool-42",
            program: "definitely-not-a-real-tool-42",
            version_args: &["--version"],
        };
        let fact = discover_tool(
            &probe_definition,
            &StdCommandRunner::new(),
            Path::new("."),
            &CancellationToken::new(),
        );
        assert_eq!(fact.probe, ProbeOutcome::NotFound);
        assert!(fact.path.is_none());
    }

    #[test]
    fn found_tool_reports_version_or_failed_probe() {
        // git 在测试环境几乎必然存在；探测结果二选一且语义正确。
        let Some(git) = definition("git") else {
            return;
        };
        let fact = discover_tool(
            git,
            &StdCommandRunner::new(),
            Path::new("."),
            &CancellationToken::new(),
        );
        match fact.probe {
            ProbeOutcome::Succeeded => {
                assert!(fact.version.is_some(), "git --version must parse: {fact:?}");
                assert!(fact.path.is_some());
                assert_eq!(fact.source, ToolSource::Path);
            }
            ProbeOutcome::Failed => {
                // 环境异常（git 存在但被限制）——仍必须是"exists but probe failed"。
                assert!(fact.path.is_some(), "§157: found but probe failed");
                assert!(fact.version.is_none());
            }
            other => panic!("unexpected probe outcome: {other:?}"),
        }
    }
}
