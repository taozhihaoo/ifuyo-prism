//! CheckDiscovery（规格 M8 §22–§24/§235–§241）：
//! 回答"这个项目可能有哪些验证入口？"——与真正执行严格分离（§24）。
//!
//! 优先级（§235）：显式 Prism config > 项目原生配置 > 生态约定 > none。
//! 同 id 冲突：configured wins 并保留 source 标记（§59 不静默覆盖）。
//! 只有 Exact/HighConfidence 的 check 才进入自动执行（§238）。

use crate::model::{CheckConfidence, CheckKind, CheckSource, CheckSpec, WritePolicy};
use prism_core::config::VerifyConfig;
use std::path::Path;

/// 默认超时（§90：configurable + documented；product default 10 分钟）。
pub const DEFAULT_TIMEOUT_MS: u64 = 600_000;

/// 项目验证入口发现（§24）。`root` 为项目根。
pub fn discover_checks(root: &Path, verify_config: &VerifyConfig) -> Vec<CheckSpec> {
    let mut specs: Vec<CheckSpec> = Vec::new();

    // ---- Node 生态：package.json scripts（§22：candidate ≠ enabled）----
    let package_json = root.join("package.json");
    if package_json.is_file() {
        if let Ok(content) = std::fs::read_to_string(&package_json) {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&content) {
                let manager = package_manager(root);
                let script_names: Vec<String> = value
                    .get("scripts")
                    .and_then(|scripts| scripts.as_object())
                    .map(|map| map.keys().cloned().collect())
                    .unwrap_or_default();
                for name in &script_names {
                    let Some((kind, confidence)) = script_kind(name) else {
                        continue;
                    };
                    specs.push(CheckSpec {
                        id: kind.as_str().to_string(),
                        kind,
                        name: format!("{} ({})", kind.as_str(), name),
                        program: manager.program.to_string(),
                        args: manager.run_args(name),
                        working_directory: None,
                        timeout_ms: DEFAULT_TIMEOUT_MS,
                        required: false,
                        write_policy: discovered_write_policy(kind),
                        depends_on: Vec::new(),
                        source: CheckSource::PackageScript,
                        confidence,
                        detected_from: format!("package.json scripts.{name}"),
                    });
                }
            }
        }
    }

    // ---- Rust 生态：Cargo.toml 存在 ⇒ cargo build/test（约定，§235 第三优先）----
    if root.join("Cargo.toml").is_file() {
        for (kind, program, args, name) in [
            (
                CheckKind::Build,
                "cargo",
                vec!["build".to_string()],
                "cargo build",
            ),
            (
                CheckKind::Test,
                "cargo",
                vec!["test".to_string()],
                "cargo test",
            ),
        ] {
            specs.push(CheckSpec {
                id: kind.as_str().to_string(),
                kind,
                name: name.to_string(),
                program: program.to_string(),
                args,
                working_directory: None,
                timeout_ms: DEFAULT_TIMEOUT_MS,
                required: false,
                write_policy: WritePolicy::BuildArtifacts,
                depends_on: Vec::new(),
                source: CheckSource::Cargo,
                confidence: CheckConfidence::HighConfidence,
                detected_from: format!("Cargo.toml ({name})"),
            });
        }
    }

    // ---- Python 生态：pyproject 含 pytest 配置 ⇒ pytest（§23 不发明命令）----
    let pyproject = root.join("pyproject.toml");
    if pyproject.is_file() {
        if let Ok(content) = std::fs::read_to_string(&pyproject) {
            if content.contains("[tool.pytest") {
                specs.push(CheckSpec {
                    id: "test".to_string(),
                    kind: CheckKind::Test,
                    name: "pytest".to_string(),
                    program: "python".to_string(),
                    args: vec!["-m".to_string(), "pytest".to_string()],
                    working_directory: None,
                    timeout_ms: DEFAULT_TIMEOUT_MS,
                    required: false,
                    write_policy: WritePolicy::BuildArtifacts,
                    depends_on: Vec::new(),
                    source: CheckSource::Pyproject,
                    confidence: CheckConfidence::HighConfidence,
                    detected_from: "pyproject.toml [tool.pytest]".to_string(),
                });
            }
        }
    }

    // ---- 显式配置（§235 第一优先）：同 id 覆盖 detected（§59）----
    for (id, check) in &verify_config.checks {
        let kind = CheckKind::parse(&check.kind).unwrap_or(CheckKind::Custom);
        let spec = CheckSpec {
            id: id.clone(),
            kind,
            name: id.clone(),
            program: check.program.clone(),
            args: check.args.clone(),
            working_directory: check.working_directory.clone(),
            timeout_ms: check.timeout_ms.unwrap_or(DEFAULT_TIMEOUT_MS),
            required: check.required.unwrap_or(false),
            write_policy: check
                .write_policy
                .as_deref()
                .and_then(WritePolicy::parse)
                .unwrap_or(WritePolicy::ReadOnly),
            depends_on: check.depends_on.clone(),
            source: CheckSource::Configured,
            confidence: CheckConfidence::Exact,
            detected_from: "prism.config.json verify.checks".to_string(),
        };
        // §59：configured wins——同 id 的 detected check 移除，不偷偷执行两个。
        specs.retain(|existing| existing.id != id);
        specs.push(spec);
    }

    specs.sort_by(|a, b| a.id.cmp(&b.id));
    specs
}

/// 已发现的默认写策略（§87：format 类命令默认按可能写源码处理）。
fn discovered_write_policy(kind: CheckKind) -> WritePolicy {
    match kind {
        CheckKind::Format => WritePolicy::ArbitraryWrite,
        _ => WritePolicy::BuildArtifacts,
    }
}

/// Node 包管理器探测（lockfile 优先，§235 项目原生配置）。
fn package_manager(root: &Path) -> PackageManager {
    if root.join("pnpm-lock.yaml").is_file() {
        PackageManager { program: "pnpm", run_prefix: vec!["run".to_string()] }
    } else if root.join("yarn.lock").is_file() {
        PackageManager { program: "yarn", run_prefix: vec![] }
    } else {
        PackageManager { program: "npm", run_prefix: vec!["run".to_string()] }
    }
}

struct PackageManager {
    program: &'static str,
    run_prefix: Vec<&'static str>,
}

impl PackageManager {
    /// npm test 特例：test script 直接 `npm test`；其余 `npm run <name>`。
    fn run_args(&self, script: &str) -> Vec<String> {
        let mut args: Vec<String> = self.run_prefix.iter().map(|value| value.to_string()).collect();
        if self.program == "npm" && self.run_prefix.iter().any(|value| value == "run") {
            args = vec![];
        }
        if script == "test" && self.program == "npm" {
            args.clear();
        }
        args.push(script.to_string());
        args
    }
}

/// script 名 → (kind, confidence)；未知 script 名不猜测（§23）。
fn script_kind(name: &str) -> Option<(CheckKind, CheckConfidence)> {
    let lower = name.to_ascii_lowercase();
    if lower == "test" || lower.ends_with(":test") || lower.ends_with("-test") {
        return Some((CheckKind::Test, CheckConfidence::Exact));
    }
    if lower == "build" || lower.ends_with(":build") {
        return Some((CheckKind::Build, CheckConfidence::Exact));
    }
    if lower == "lint" || lower.starts_with("lint") {
        return Some((CheckKind::Lint, CheckConfidence::Exact));
    }
    if lower == "typecheck" || lower.starts_with("typecheck") || lower == "tsc" || lower == "check" {
        return Some((CheckKind::Typecheck, CheckConfidence::Exact));
    }
    if lower == "format" || lower == "fmt" {
        return Some((CheckKind::Format, CheckConfidence::Exact));
    }
    None
}
