//! 运行时 / SDK / 包管理器 / 虚拟环境发现（规格 M9a §31–§37/§62–§70/§104–§107/§159）。
//!
//! 复用 ToolFact 的探测结果；额外的原生 SDK 命令（`dotnet --list-sdks`、
//! `rustup toolchain list`）也是 allowlist 形态（§33/§155）。项目关联
//! （lockfile / packageManager 字段）必须带证据（§36/§37）。

use crate::model::{
    PackageManagerAssociation, PackageManagerInfo, ProbeOutcome, RuntimeInfo, SdkInfo, ToolFact,
    ToolSource, VirtualEnvInfo,
};
use crate::probe::{run_probe, ProbeResult, ToolProbe};
use prism_core::runtime::command::CommandRunner;
use prism_core::task::CancellationToken;
use std::path::Path;

/// ToolFact → RuntimeInfo（§31：node→Node.js、python→Python、rustc→Rust…）。
pub fn runtimes_from_tools(tools: &[ToolFact]) -> Vec<RuntimeInfo> {
    let mut runtimes = Vec::new();
    let runtime_names: &[(&str, &str)] = &[
        ("node", "Node.js"),
        ("python", "Python"),
        ("python3", "Python"),
        ("dotnet", ".NET"),
        ("java", "JVM"),
        ("go", "Go"),
        ("rustc", "Rust"),
    ];
    for (tool_name, runtime_name) in runtime_names {
        if let Some(tool) = tools.iter().find(|tool| tool.name == *tool_name) {
            // 同名 runtime（python/python3）不重复列（§395 稳定去重）。
            if runtimes
                .iter()
                .any(|runtime: &RuntimeInfo| runtime.name == *runtime_name)
            {
                continue;
            }
            runtimes.push(RuntimeInfo {
                name: runtime_name.to_string(),
                version: tool.version.clone(),
                raw_version: tool.raw_version.clone(),
                path: tool.path.clone(),
                architecture: None, // v1 未探测 runtime 自身架构（不猜）
                source: tool.source,
            });
        }
    }
    runtimes
}

/// 原生 SDK 列表（§33–§34）：dotnet --list-sdks / rustup toolchain list。
/// 只在对应工具存在时执行（§120 lazy）；输出有界、失败隔离。
pub fn discover_sdks(
    tools: &[ToolFact],
    runner: &dyn CommandRunner,
    working_directory: &Path,
    cancel: &CancellationToken,
) -> Vec<SdkInfo> {
    let mut sdks = Vec::new();
    if tools
        .iter()
        .any(|tool| tool.name == "dotnet" && tool.probe == ProbeOutcome::Succeeded)
    {
        let probe = ToolProbe {
            program: "dotnet".to_string(),
            args: vec!["--list-sdks".to_string()],
            timeout_ms: crate::probe::DEFAULT_PROBE_TIMEOUT_MS,
            output_kind: crate::probe::ProbeOutputKind::LineList,
        };
        if let ProbeResult::Output { text, .. } =
            run_probe(&probe, runner, working_directory, cancel)
        {
            sdks.extend(parse_dotnet_sdks(&text));
        }
    }
    if tools
        .iter()
        .any(|tool| tool.name == "rustup" && tool.probe == ProbeOutcome::Succeeded)
    {
        let probe = ToolProbe {
            program: "rustup".to_string(),
            args: vec!["toolchain".to_string(), "list".to_string()],
            timeout_ms: crate::probe::DEFAULT_PROBE_TIMEOUT_MS,
            output_kind: crate::probe::ProbeOutputKind::LineList,
        };
        if let ProbeResult::Output { text, .. } =
            run_probe(&probe, runner, working_directory, cancel)
        {
            sdks.extend(parse_rustup_toolchains(&text));
        }
    }
    sdks
}

/// `dotnet --list-sdks`：行形如 `10.0.401 [C:\Program Files\dotnet\sdk]`。
pub fn parse_dotnet_sdks(output: &str) -> Vec<SdkInfo> {
    let mut sdks = Vec::new();
    for line in output.lines().filter_map(|line| {
        let line = line.trim();
        (!line.is_empty()).then_some(line)
    }) {
        let Some(version_str) = line.split(" [").next().map(str::trim) else {
            continue;
        };
        sdks.push(SdkInfo {
            name: ".NET SDK".to_string(),
            version: crate::version::parse_version_token(version_str),
            raw_version: Some(version_str.to_string()),
            source: ToolSource::Path,
        });
        if sdks.len() >= 20 {
            break; // §311 有界
        }
    }
    sdks
}

/// `rustup toolchain list`：`1.90.0-x86_64-pc-windows-msvc (default)` /
/// `stable-x86_64-pc-windows-msvc`（非语义化 ⇒ 只保留 raw）。
pub fn parse_rustup_toolchains(output: &str) -> Vec<SdkInfo> {
    let mut sdks = Vec::new();
    for line in output.lines().filter_map(|line| {
        let line = line.trim();
        (!line.is_empty()).then_some(line)
    }) {
        let raw = line.split_whitespace().next().unwrap_or(line).to_string();
        sdks.push(SdkInfo {
            name: "rust-toolchain".to_string(),
            version: crate::version::parse_version_token(&raw),
            raw_version: Some(raw),
            source: ToolSource::VersionManager,
        });
        if sdks.len() >= 20 {
            break;
        }
    }
    sdks
}

/// lockfile → 包管理器关联（§104：只记录 exists + association；
/// 依赖语义分析仍归 M3/未来工具）。
pub const LOCKFILES: &[(&str, &str)] = &[
    ("package-lock.json", "npm"),
    ("pnpm-lock.yaml", "pnpm"),
    ("yarn.lock", "yarn"),
    ("bun.lockb", "bun"),
    ("bun.lock", "bun"),
    ("uv.lock", "uv"),
    ("poetry.lock", "poetry"),
    ("Cargo.lock", "cargo"),
    ("packages.lock.json", "nuget"),
];

/// 包管理器事实 = 机器上的 PM 工具 × 项目关联证据（§35–§37）。
pub fn discover_package_managers(
    tools: &[ToolFact],
    project_root: &Path,
) -> Vec<PackageManagerInfo> {
    let mut managers: Vec<PackageManagerInfo> = Vec::new();

    // 1) lockfile 存在 ⇒ 关联（§104）。逐个 stat 项目根（§334 不重扫）。
    for (lockfile, manager) in LOCKFILES {
        if project_root.join(lockfile).is_file() {
            upsert_manager(&mut managers, manager, |info| {
                info.project_association = Some(PackageManagerAssociation {
                    manager: manager.to_string(),
                    evidence: format!("lockfile {lockfile}"),
                    required_version: None,
                });
            });
        }
    }

    // 2) packageManager 字段（§106/§107）：优先级最高（§36 active 推断），
    //    携带版本要求与证据。
    if let Some((manager, required_version)) = package_manager_field(project_root) {
        upsert_manager(&mut managers, &manager, |info| {
            info.project_association = Some(PackageManagerAssociation {
                manager: manager.clone(),
                evidence: "package.json packageManager field".to_string(),
                required_version: required_version.clone(),
            });
        });
    }

    // 3) 机器上实际安装的 PM 工具事实（version/path/source）。
    for tool in tools {
        if is_package_manager(&tool.name) {
            upsert_manager(&mut managers, &tool.name, |info| {
                info.version = tool.version.clone();
                info.raw_version = tool.raw_version.clone();
                info.path = tool.path.clone();
                info.source = tool.source;
            });
        }
    }

    managers.sort_by(|a, b| a.name.cmp(&b.name));
    managers
}

fn is_package_manager(name: &str) -> bool {
    matches!(
        name,
        "npm" | "pnpm" | "yarn" | "bun" | "pip" | "uv" | "poetry" | "cargo" | "corepack"
    )
}

fn upsert_manager(
    managers: &mut Vec<PackageManagerInfo>,
    name: &str,
    apply: impl FnOnce(&mut PackageManagerInfo),
) {
    if let Some(existing) = managers.iter_mut().find(|info| info.name == name) {
        apply(existing);
    } else {
        let mut info = PackageManagerInfo {
            name: name.to_string(),
            version: None,
            raw_version: None,
            path: None,
            project_association: None,
            source: ToolSource::Unknown,
        };
        apply(&mut info);
        managers.push(info);
    }
}

/// 解析 package.json `packageManager` 字段（§106/§107）：
/// "pnpm@10.17.0" ⇒ ("pnpm", Some("10.17.0"))。
pub fn package_manager_field(project_root: &Path) -> Option<(String, Option<String>)> {
    let content = std::fs::read_to_string(project_root.join("package.json")).ok()?;
    let value = serde_json::from_str::<serde_json::Value>(&content).ok()?;
    let field = value.get("packageManager")?.as_str()?;
    let (manager, version) = field.split_once('@')?;
    let manager = manager.trim();
    if manager.is_empty() {
        return None;
    }
    let version = version
        .trim()
        .trim_start_matches('^')
        .trim_start_matches('~')
        .to_string();
    let version = (!version.is_empty()).then_some(version);
    Some((manager.to_string(), version))
}

/// 虚拟环境识别（§69/§70）：.venv/venv/virtualenv/conda 目录特征；
/// interpreter 存在才 probe 版本（allowlist python --version 形态）。
pub fn discover_virtualenvs(
    project_root: &Path,
    runner: &dyn CommandRunner,
    cancel: &CancellationToken,
) -> Vec<VirtualEnvInfo> {
    let mut venvs = Vec::new();
    for (dir_name, kind) in [
        (".venv", "venv"),
        ("venv", "venv"),
        ("virtualenv", "virtualenv"),
        (".conda", "conda"),
    ] {
        if cancel.is_cancelled() {
            break;
        }
        let dir = project_root.join(dir_name);
        if !dir.is_dir() {
            continue;
        }
        let interpreter = venv_interpreter(&dir);
        let (version, raw_version) = interpreter
            .as_ref()
            .map(|interpreter| probe_venv_version(interpreter, runner, project_root, cancel))
            .unwrap_or((None, None));
        let active = std::env::var_os("VIRTUAL_ENV")
            .map(|value| {
                std::path::PathBuf::from(&value)
                    .canonicalize()
                    .ok()
                    .or(Some(std::path::PathBuf::from(value)))
                    .is_some_and(|active| active == dir.canonicalize().unwrap_or(dir.clone()))
            })
            .unwrap_or(false);
        venvs.push(VirtualEnvInfo {
            kind: kind.to_string(),
            path: dir.to_string_lossy().to_string(),
            interpreter: interpreter
                .as_ref()
                .map(|path| path.to_string_lossy().to_string()),
            version,
            raw_version,
            active,
        });
    }
    venvs
}

fn venv_interpreter(dir: &Path) -> Option<std::path::PathBuf> {
    let candidates = if cfg!(windows) {
        [vec!["Scripts", "python.exe"], vec!["bin", "python.exe"]]
    } else {
        [vec!["bin", "python"], vec!["Scripts", "python"]]
    };
    for parts in candidates {
        let candidate: std::path::PathBuf = parts.iter().fold(dir.to_path_buf(), |base, part| base.join(part));
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

fn probe_venv_version(
    interpreter: &Path,
    runner: &dyn CommandRunner,
    working_directory: &Path,
    cancel: &CancellationToken,
) -> (Option<crate::version::ToolVersion>, Option<String>) {
    // §112–§113：仍是最小 version probe，只是 program 为项目内解释器。
    let probe = ToolProbe {
        program: interpreter.to_string_lossy().to_string(),
        args: vec!["--version".to_string()],
        timeout_ms: crate::probe::DEFAULT_PROBE_TIMEOUT_MS,
        output_kind: crate::probe::ProbeOutputKind::VersionLine,
    };
    match run_probe(&probe, runner, working_directory, cancel) {
        ProbeResult::Output { text, .. } => {
            let raw = crate::probe::version_line(&text, "");
            let version = raw.as_deref().and_then(crate::version::parse_version_output);
            (version, raw)
        }
        _ => (None, None),
    }
}

/// 项目本地工具解析（§62/§68：node_modules/.bin、.venv）——
/// 供调用方在 PATH 之前尝试；只做文件存在性检查，不执行。
pub fn resolve_project_local(name: &str, project_root: &Path) -> Option<std::path::PathBuf> {
    let mut candidates: Vec<std::path::PathBuf> = Vec::new();
    let bin_dirs = [
        project_root.join("node_modules").join(".bin"),
        project_root.join(".venv").join(if cfg!(windows) { "Scripts" } else { "bin" }),
        project_root.join("venv").join(if cfg!(windows) { "Scripts" } else { "bin" }),
    ];
    for dir in bin_dirs {
        candidates.push(dir.join(name));
        if cfg!(windows) {
            for extension in [".exe", ".cmd", ".bat"] {
                candidates.push(dir.join(format!("{name}{extension}")));
            }
        }
    }
    candidates.into_iter().find(|candidate| candidate.is_file())
}

/// 确保 allowlist 引用不变成死代码（§23 定义与 PM 名单一致性）。
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_dotnet_sdk_lines() {
        let sdks = parse_dotnet_sdks(
            "10.0.401 [C:\\Program Files\\dotnet\\sdk]\n8.0.100 [C:\\Program Files\\dotnet\\sdk]\n",
        );
        assert_eq!(sdks.len(), 2);
        assert_eq!(sdks[0].name, ".NET SDK");
        assert_eq!(sdks[0].version.as_ref().unwrap().major, 10);
        assert_eq!(sdks[0].raw_version.as_deref(), Some("10.0.401"));
    }

    #[test]
    fn parse_rustup_toolchain_lines() {
        let sdks = parse_rustup_toolchains(
            "1.90.0-x86_64-pc-windows-msvc (default)\nstable-x86_64-pc-windows-msvc\n",
        );
        assert_eq!(sdks.len(), 2);
        assert_eq!(sdks[0].version.as_ref().unwrap().major, 1);
        assert_eq!(sdks[0].source, ToolSource::VersionManager);
        // 非语义化名字 ⇒ raw only，不猜（§29）。
        assert!(sdks[1].version.is_none());
        assert_eq!(sdks[1].raw_version.as_deref(), Some("stable-x86_64-pc-windows-msvc"));
    }

    #[test]
    fn package_manager_field_parsed_with_version() {
        let dir = std::env::temp_dir().join(format!(
            "prism-env-pmfield-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.subsec_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("package.json"),
            r#"{ "name": "x", "packageManager": "pnpm@10.17.0" }"#,
        )
        .unwrap();
        let (manager, version) = package_manager_field(&dir).unwrap();
        assert_eq!(manager, "pnpm");
        assert_eq!(version.as_deref(), Some("10.17.0"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn lockfile_association_with_provenance() {
        let dir = std::env::temp_dir().join(format!(
            "prism-env-lock-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.subsec_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("pnpm-lock.yaml"), "").unwrap();
        let managers = discover_package_managers(&[], &dir);
        let pnpm = managers.iter().find(|info| info.name == "pnpm").unwrap();
        let association = pnpm.project_association.as_ref().unwrap();
        assert_eq!(association.evidence, "lockfile pnpm-lock.yaml");
        // §36/§37：为什么是 pnpm —— 证据即答案；机器上没有 pnpm 工具也如实列出。
        assert!(pnpm.path.is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn package_manager_field_wins_over_lockfile() {
        let dir = std::env::temp_dir().join(format!(
            "prism-env-pmwin-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.subsec_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("package-lock.json"), "").unwrap();
        std::fs::write(
            dir.join("package.json"),
            r#"{ "packageManager": "pnpm@9.0.0" }"#,
        )
        .unwrap();
        let managers = discover_package_managers(&[], &dir);
        let pnpm = managers.iter().find(|info| info.name == "pnpm").unwrap();
        assert_eq!(
            pnpm.project_association.as_ref().unwrap().evidence,
            "package.json packageManager field"
        );
        assert_eq!(
            pnpm.project_association.as_ref().unwrap().required_version.as_deref(),
            Some("9.0.0")
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn venv_discovery_reports_interpreter_and_active_flag() {
        let dir = std::env::temp_dir().join(format!(
            "prism-env-venv-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.subsec_nanos())
                .unwrap_or(0)
        ));
        let scripts = if cfg!(windows) { "Scripts" } else { "bin" };
        let bin = dir.join(".venv").join(scripts);
        std::fs::create_dir_all(&bin).unwrap();
        let interpreter_name = if cfg!(windows) { "python.exe" } else { "python" };
        let interpreter = bin.join(interpreter_name);
        std::fs::write(&interpreter, "").unwrap(); // 探测会失败 ⇒ version None，§157 语义
        let cancel = CancellationToken::new();
        let runner = prism_core::runtime::command::StdCommandRunner::new();
        let venvs = discover_virtualenvs(&dir, &runner, &cancel);
        assert_eq!(venvs.len(), 1);
        assert_eq!(venvs[0].kind, "venv");
        assert!(venvs[0].interpreter.is_some());
        assert!(!venvs[0].active, "not activated in test process");
        assert!(venvs[0].version.is_none(), "fake interpreter ⇒ version unknown");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn project_local_resolution_finds_venv_tool() {
        let dir = std::env::temp_dir().join(format!(
            "prism-env-local-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.subsec_nanos())
                .unwrap_or(0)
        ));
        let scripts = if cfg!(windows) { "Scripts" } else { "bin" };
        let bin = dir.join(".venv").join(scripts);
        std::fs::create_dir_all(&bin).unwrap();
        let tool_name = if cfg!(windows) { "pytest.exe" } else { "pytest" };
        std::fs::write(bin.join(tool_name), "").unwrap();
        assert!(resolve_project_local("pytest", &dir).is_some());
        assert!(resolve_project_local("eslint", &dir).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn runtimes_dedupe_python_aliases() {
        let tools = vec![
            ToolFact {
                name: "python".to_string(),
                path: Some("python".to_string()),
                additional_paths: vec![],
                version: Some(crate::version::ToolVersion::new(3, Some(13), None)),
                raw_version: Some("Python 3.13.15".to_string()),
                source: ToolSource::Path,
                probe: ProbeOutcome::Succeeded,
            },
            ToolFact {
                name: "python3".to_string(),
                path: None,
                additional_paths: vec![],
                version: None,
                raw_version: None,
                source: ToolSource::Path,
                probe: ProbeOutcome::NotFound,
            },
        ];
        let runtimes = runtimes_from_tools(&tools);
        assert_eq!(runtimes.len(), 1);
        assert_eq!(runtimes[0].name, "Python");
    }
}
