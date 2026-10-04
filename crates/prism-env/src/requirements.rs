//! Requirement providers（规格 M9b §42–§45/§71–§77/§101–§103/§124–§125）。
//!
//! provider 只做 **项目事实 → 需求**（§125），不做机器探测。
//! 来源层级（§44/§102）：显式项目配置 > 生态元数据 > CI > 版本文件 >
//! 容器配置 > 文档 > 启发式；同主体多来源且期望不一致 ⇒ RequirementConflict
//! （§45/§103：不静默择一）。

use crate::model::{
    ConflictEntry, EnvironmentRequirement, RequirementConflict, RequirementKind,
    RequirementOperator, SourceKind,
};
use prism_core::Confidence;
use std::path::Path;

/// 一次 provider 运行的项目事实上下文（§152/§335：复用 M1，不重扫）。
pub struct ProjectFacts<'a> {
    pub root: &'a Path,
    /// M1 discovery 的相对路径清单（None ⇒ 只做单文件探测）。
    pub files: Option<&'a [String]>,
}

impl<'a> ProjectFacts<'a> {
    fn has_file(&self, relative: &str) -> bool {
        if let Some(files) = self.files {
            files.iter().any(|file| file == relative)
        } else {
            self.root.join(relative).is_file()
        }
    }

    fn read(&self, relative: &str) -> Option<String> {
        std::fs::read_to_string(self.root.join(relative)).ok()
    }
}

/// provider trait（§124）。
pub trait RequirementProvider {
    fn name(&self) -> &'static str;
    /// 返回 (requirements, raw-entries-for-conflict-detection)。
    fn provide(&self, facts: &ProjectFacts) -> Vec<EnvironmentRequirement>;
}

/// 收集全部需求（§124–§125）：跑所有 provider，再按 (kind, subject)
/// 做冲突检测（§45）——期望一致 ⇒ 合并保留最高优先级来源；
/// 不一致 ⇒ RequirementConflict + 全部条目保留进冲突记录。
pub fn collect_requirements(
    facts: &ProjectFacts,
) -> (Vec<EnvironmentRequirement>, Vec<RequirementConflict>) {
    let providers: Vec<Box<dyn RequirementProvider>> = vec![
        Box::new(NodeRequirementProvider),
        Box::new(PythonRequirementProvider),
        Box::new(DotnetRequirementProvider),
        Box::new(RustRequirementProvider),
        Box::new(GoRequirementProvider),
        Box::new(JavaRequirementProvider),
        Box::new(ContainerRequirementProvider),
        Box::new(DocumentationRequirementProvider),
    ];
    let mut all: Vec<EnvironmentRequirement> = Vec::new();
    for provider in &providers {
        all.extend(provider.provide(facts));
    }
    if let Some(requirement) = package_manager_requirement(facts.root) {
        all.push(requirement);
    }
    resolve_conflicts(all)
}

/// 同 (kind, subject) 分组；期望等价（operator+expected 相同）⇒ 保留
/// 优先级最高的一条（evidence 合并）；否则 ⇒ RequirementConflict。
fn resolve_conflicts(
    all: Vec<EnvironmentRequirement>,
) -> (Vec<EnvironmentRequirement>, Vec<RequirementConflict>) {
    let mut groups: std::collections::BTreeMap<(String, String), Vec<EnvironmentRequirement>> =
        Default::default();
    for requirement in all {
        let key = (
            format!("{:?}", requirement.kind),
            requirement.subject.clone(),
        );
        groups.entry(key).or_default().push(requirement);
    }

    let mut requirements = Vec::new();
    let mut conflicts = Vec::new();
    for ((kind_key, subject), mut group) in groups {
        group.sort_by(|a, b| {
            a.source_kind
                .cmp(&b.source_kind)
                .then_with(|| a.source.cmp(&b.source))
        });
        let signatures: std::collections::BTreeSet<(RequirementOperator, String)> = group
            .iter()
            .map(|requirement| (requirement.operator, requirement.expected.clone()))
            .collect();
        if signatures.len() == 1 {
            // §45：不静默覆盖——保留最高优先级来源，证据合并列出全部来源。
            let mut primary = group.remove(0);
            let extra_sources: Vec<&str> = group.iter().map(|r| r.source.as_str()).collect();
            if !extra_sources.is_empty() {
                primary.evidence = format!(
                    "{}; also stated by {}",
                    primary.evidence,
                    extra_sources.join(", ")
                );
            }
            requirements.push(primary);
        } else {
            conflicts.push(RequirementConflict {
                kind: serde_kind(&kind_key),
                entries: group
                    .iter()
                    .map(|requirement| ConflictEntry {
                        source: requirement.source.clone(),
                        source_kind: requirement.source_kind,
                        expected: requirement.expected.clone(),
                        operator: requirement.operator,
                    })
                    .collect(),
                subject,
            });
            // §327：冲突主体不作为单一明确需求参与匹配——每条都保留、
            // 标记 not-applicable（matcher 对冲突主体输出 Unknown，见 matcher）。
            for mut requirement in group {
                requirement.applicable = false;
                requirements.push(requirement);
            }
        }
    }
    requirements.sort_by(|a, b| a.id.cmp(&b.id));
    conflicts.sort_by(|a, b| a.subject.cmp(&b.subject));
    (requirements, conflicts)
}

fn serde_kind(kind_key: &str) -> RequirementKind {
    match kind_key {
        "Runtime" => RequirementKind::Runtime,
        "Sdk" => RequirementKind::Sdk,
        "Tool" => RequirementKind::Tool,
        "PackageManager" => RequirementKind::PackageManager,
        "Container" => RequirementKind::Container,
        _ => RequirementKind::Tool,
    }
}

#[allow(clippy::too_many_arguments)]
fn requirement(
    kind: RequirementKind,
    subject: &str,
    operator: RequirementOperator,
    expected: &str,
    source: &str,
    source_kind: SourceKind,
    evidence: &str,
    confidence: Confidence,
) -> EnvironmentRequirement {
    EnvironmentRequirement {
        id: format!("req:{:?}:{}#{}", kind, subject, source),
        kind,
        subject: subject.to_string(),
        operator,
        expected: expected.to_string(),
        source: source.to_string(),
        source_kind,
        evidence: evidence.to_string(),
        confidence,
        applicable: true,
    }
}

/// 版本约束表达式 → operator（保守）：">=x" AtLeast、"<x"/"<=x" AtMost、
/// "x" / "=x" Equals；"^x"/"~x"/"x.y.z - y.z.w" 等区间 ⇒ AtLeast 下界
/// （保守放宽）+ 注明；无法解析 ⇒ None（不猜）。
pub fn parse_version_spec(spec: &str) -> Option<(RequirementOperator, String)> {
    let spec = spec.trim();
    if let Some(rest) = spec.strip_prefix(">=") {
        return numeric(rest).map(|version| (RequirementOperator::AtLeast, version));
    }
    if let Some(rest) = spec.strip_prefix('>') {
        return numeric(rest).map(|version| (RequirementOperator::AtLeast, version));
    }
    if let Some(rest) = spec.strip_prefix("<=") {
        return numeric(rest).map(|version| (RequirementOperator::AtMost, version));
    }
    if let Some(rest) = spec.strip_prefix('<') {
        return numeric(rest).map(|version| (RequirementOperator::AtMost, version));
    }
    if let Some(rest) = spec.strip_prefix('=') {
        return numeric(rest).map(|version| (RequirementOperator::Equals, version));
    }
    // ^x.y.z / ~x.y.z：保守取下界（AtLeast），不建模完整区间语义。
    if let Some(rest) = spec.strip_prefix('^').or_else(|| spec.strip_prefix('~')) {
        return numeric(rest).map(|version| (RequirementOperator::AtLeast, version));
    }
    if spec.ends_with(".x") || spec.ends_with(".*") {
        let base = spec.trim_end_matches(".x").trim_end_matches(".*");
        return numeric(base).map(|version| (RequirementOperator::AtLeast, version));
    }
    numeric(spec).map(|version| (RequirementOperator::Equals, version))
}

fn numeric(text: &str) -> Option<String> {
    let trimmed = text.trim().trim_start_matches('v');
    let head: String = trimmed
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    if head.is_empty() || !head.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        return None;
    }
    // "1.22." / 尾点清理；多段保留前 3 段。
    let parts: Vec<&str> = head.split('.').filter(|p| !p.is_empty()).take(3).collect();
    (parts.iter().all(|p| p.parse::<u64>().is_ok()) && !parts.is_empty()).then(|| parts.join("."))
}

// ---------- Node（§71） ----------

pub struct NodeRequirementProvider;

impl RequirementProvider for NodeRequirementProvider {
    fn name(&self) -> &'static str {
        "node"
    }

    fn provide(&self, facts: &ProjectFacts) -> Vec<EnvironmentRequirement> {
        let mut out = Vec::new();
        // package.json engines（§71 ecosystem metadata）。
        if let Some(content) = facts.read("package.json") {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(engines) = value.get("engines").and_then(|engines| engines.as_object())
                {
                    for (subject, expected) in engines {
                        let Some(expected) = expected.as_str() else {
                            continue;
                        };
                        let Some((operator, version)) = parse_version_spec(expected) else {
                            continue;
                        };
                        let kind = match subject.as_str() {
                            "node" => RequirementKind::Runtime,
                            "npm" | "pnpm" | "yarn" => RequirementKind::PackageManager,
                            _ => RequirementKind::Tool,
                        };
                        out.push(requirement(
                            kind,
                            subject,
                            operator,
                            &version,
                            "package.json engines",
                            SourceKind::EcosystemMetadata,
                            "package.json: engines",
                            Confidence::High,
                        ));
                    }
                }
            }
        }
        // .nvmrc / .node-version（§71 版本文件）。
        for file in [".nvmrc", ".node-version"] {
            if let Some(content) = facts.read(file) {
                let Some((operator, version)) =
                    parse_version_spec(content.lines().next().unwrap_or_default())
                else {
                    continue; // "lts/*" 等非数字内容不猜。
                };
                out.push(requirement(
                    RequirementKind::Runtime,
                    "node",
                    operator,
                    &version,
                    file,
                    SourceKind::VersionFile,
                    file,
                    Confidence::High,
                ));
            }
        }
        out
    }
}

// ---------- Python（§72） ----------

pub struct PythonRequirementProvider;

impl RequirementProvider for PythonRequirementProvider {
    fn name(&self) -> &'static str {
        "python"
    }

    fn provide(&self, facts: &ProjectFacts) -> Vec<EnvironmentRequirement> {
        let mut out = Vec::new();
        if let Some(content) = facts.read("pyproject.toml") {
            for line in content.lines() {
                let line = line.trim();
                if let Some(rest) = line.strip_prefix("requires-python") {
                    if let Some((_, value)) = rest.split_once('=') {
                        let value = value.trim().trim_matches('"').trim_matches('\'');
                        if let Some((operator, version)) = parse_version_spec(value) {
                            out.push(requirement(
                                RequirementKind::Runtime,
                                "python",
                                operator,
                                &version,
                                "pyproject.toml requires-python",
                                SourceKind::EcosystemMetadata,
                                "pyproject.toml: requires-python",
                                Confidence::High,
                            ));
                        }
                    }
                    break;
                }
            }
        }
        for file in [".python-version", "runtime.txt"] {
            if let Some(content) = facts.read(file) {
                let first = content.lines().next().unwrap_or_default().trim();
                let version_text = first.strip_prefix("python-").unwrap_or(first);
                if let Some((operator, version)) = parse_version_spec(version_text) {
                    out.push(requirement(
                        RequirementKind::Runtime,
                        "python",
                        operator,
                        &version,
                        file,
                        SourceKind::VersionFile,
                        file,
                        Confidence::High,
                    ));
                }
            }
        }
        out
    }
}

// ---------- .NET（§73） ----------

pub struct DotnetRequirementProvider;

impl RequirementProvider for DotnetRequirementProvider {
    fn name(&self) -> &'static str {
        "dotnet"
    }

    fn provide(&self, facts: &ProjectFacts) -> Vec<EnvironmentRequirement> {
        let mut out = Vec::new();
        if let Some(content) = facts.read("global.json") {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(version) = value
                    .get("sdk")
                    .and_then(|sdk| sdk.get("version"))
                    .and_then(|version| version.as_str())
                {
                    if let Some((operator, version)) = parse_version_spec(version) {
                        out.push(requirement(
                            RequirementKind::Sdk,
                            "dotnet",
                            operator,
                            &version,
                            "global.json sdk.version",
                            SourceKind::ProjectConfig,
                            "global.json: sdk.version",
                            Confidence::High,
                        ));
                    }
                }
            }
        }
        out
    }
}

// ---------- Rust（§74） ----------

pub struct RustRequirementProvider;

impl RequirementProvider for RustRequirementProvider {
    fn name(&self) -> &'static str {
        "rust"
    }

    fn provide(&self, facts: &ProjectFacts) -> Vec<EnvironmentRequirement> {
        let mut out = Vec::new();
        // rust-toolchain.toml：channel = "1.85"（数字）或 "stable"（不猜）。
        if let Some(content) = facts.read("rust-toolchain.toml") {
            for line in content.lines() {
                let line = line.trim();
                if let Some(rest) = line.strip_prefix("channel") {
                    if let Some((_, value)) = rest.split_once('=') {
                        let value = value.trim().trim_matches('"');
                        if let Some((operator, version)) = parse_version_spec(value) {
                            out.push(requirement(
                                RequirementKind::Tool,
                                "rustc",
                                operator,
                                &version,
                                "rust-toolchain.toml channel",
                                SourceKind::ProjectConfig,
                                "rust-toolchain.toml: channel",
                                Confidence::High,
                            ));
                        }
                    }
                    break;
                }
            }
        }
        if let Some(content) = facts.read("rust-toolchain") {
            let first = content.lines().next().unwrap_or_default().trim();
            if let Some((operator, version)) = parse_version_spec(first) {
                out.push(requirement(
                    RequirementKind::Tool,
                    "rustc",
                    operator,
                    &version,
                    "rust-toolchain",
                    SourceKind::ProjectConfig,
                    "rust-toolchain",
                    Confidence::High,
                ));
            }
        }
        out
    }
}

// ---------- Go（§76） ----------

pub struct GoRequirementProvider;

impl RequirementProvider for GoRequirementProvider {
    fn name(&self) -> &'static str {
        "go"
    }

    fn provide(&self, facts: &ProjectFacts) -> Vec<EnvironmentRequirement> {
        let mut out = Vec::new();
        if let Some(content) = facts.read("go.mod") {
            for line in content.lines() {
                let line = line.trim();
                if let Some(rest) = line.strip_prefix("go ") {
                    if let Some(version) = numeric(rest) {
                        out.push(requirement(
                            RequirementKind::Runtime,
                            "go",
                            RequirementOperator::AtLeast,
                            &version,
                            "go.mod go directive",
                            SourceKind::ProjectConfig,
                            "go.mod: go directive",
                            Confidence::High,
                        ));
                    }
                    break;
                }
            }
        }
        out
    }
}

// ---------- Java（§75，轻量） ----------

pub struct JavaRequirementProvider;

impl RequirementProvider for JavaRequirementProvider {
    fn name(&self) -> &'static str {
        "java"
    }

    fn provide(&self, facts: &ProjectFacts) -> Vec<EnvironmentRequirement> {
        let mut out = Vec::new();
        if let Some(content) = facts.read(".java-version") {
            let first = content.lines().next().unwrap_or_default().trim();
            if let Some((operator, version)) = parse_version_spec(first) {
                out.push(requirement(
                    RequirementKind::Runtime,
                    "java",
                    operator,
                    &version,
                    ".java-version",
                    SourceKind::VersionFile,
                    ".java-version",
                    Confidence::High,
                ));
            }
        }
        // .tool-versions 的 java 行（asdf，§71/§75 版本文件）。
        if let Some(content) = facts.read(".tool-versions") {
            for line in content.lines() {
                let mut parts = line.split_whitespace();
                if parts.next() == Some("java") {
                    if let Some(version) = parts.next().and_then(numeric) {
                        out.push(requirement(
                            RequirementKind::Runtime,
                            "java",
                            RequirementOperator::Equals,
                            &version,
                            ".tool-versions",
                            SourceKind::VersionFile,
                            ".tool-versions: java",
                            Confidence::High,
                        ));
                    }
                    break;
                }
            }
        }
        out
    }
}

// ---------- Container（§78/§101 容器配置来源） ----------

pub struct ContainerRequirementProvider;

impl RequirementProvider for ContainerRequirementProvider {
    fn name(&self) -> &'static str {
        "container"
    }

    fn provide(&self, facts: &ProjectFacts) -> Vec<EnvironmentRequirement> {
        let mut out = Vec::new();
        let has_container = ["Dockerfile", "docker-compose.yml", "compose.yaml"]
            .iter()
            .any(|file| facts.has_file(file));
        if has_container {
            out.push(requirement(
                RequirementKind::Tool,
                "docker",
                RequirementOperator::Exists,
                "",
                "Dockerfile/compose presence",
                SourceKind::ContainerConfig,
                "container config file exists",
                Confidence::High,
            ));
        }
        out
    }
}

// ---------- 文档（§43：README 是 Documentation 层，Probable） ----------

pub struct DocumentationRequirementProvider;

impl RequirementProvider for DocumentationRequirementProvider {
    fn name(&self) -> &'static str {
        "documentation"
    }

    fn provide(&self, facts: &ProjectFacts) -> Vec<EnvironmentRequirement> {
        let mut out = Vec::new();
        let Some(content) = facts.read("README.md").or_else(|| facts.read("readme.md")) else {
            return out;
        };
        for (subject, keywords) in [
            ("node", vec!["node.js", "nodejs", "node"]),
            ("python", vec!["python"]),
            ("go", vec!["golang", "go"]),
        ] {
            if let Some((operator, version)) = scan_documentation_version(&content, &keywords) {
                out.push(requirement(
                    RequirementKind::Runtime,
                    subject,
                    operator,
                    &version,
                    "README",
                    SourceKind::Documentation,
                    "README mentions version",
                    Confidence::Medium,
                ));
            }
        }
        out
    }
}

/// 文档行扫描（保守）：仅识别 "Node 20" / "node >= 22" / "Python 3.12+" 形态；
/// 每个工具只取第一处命中。
fn scan_documentation_version(
    content: &str,
    keywords: &[&str],
) -> Option<(RequirementOperator, String)> {
    for line in content.lines() {
        let lower = line.to_ascii_lowercase();
        if !keywords.iter().any(|keyword| lower.contains(keyword)) {
            continue;
        }
        // 行内找 ">= 22" / "20+" / "3.12" 形态。
        for segment in line.split([' ', '(', ')', ',', ':']) {
            let segment = segment.trim().trim_end_matches('+');
            // Documentation 层（§43 Probable）：裸版本按"最低版本"取 AtLeast
            // ——README "Node 20" 语义是"需要 Node 20+"，不是精确 pin。
            if let Some(version) = numeric(segment) {
                return Some((RequirementOperator::AtLeast, version));
            }
        }
    }
    None
}

/// packageManager 字段 → PackageManager 需求（§106）；字段不存在 ⇒ None。
pub fn package_manager_requirement(root: &Path) -> Option<EnvironmentRequirement> {
    let (manager, version) = crate::runtime::package_manager_field(root)?;
    let (operator, expected) = match version {
        Some(version) => (RequirementOperator::Equals, version),
        None => (RequirementOperator::Exists, String::new()),
    };
    Some(requirement(
        RequirementKind::PackageManager,
        &manager,
        operator,
        &expected,
        "package.json packageManager field",
        SourceKind::EcosystemMetadata,
        "package.json packageManager field",
        Confidence::High,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts_with<'a>(
        root: &'a Path,
        files: &[&str],
        storage: &'a mut Vec<String>,
    ) -> ProjectFacts<'a> {
        *storage = files.iter().map(|file| file.to_string()).collect();
        ProjectFacts {
            root,
            files: Some(storage),
        }
    }

    fn temp(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "prism-env-req-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.subsec_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    // §197 requirement tests：>= <= = unknown conflict invalid。
    #[test]
    fn version_spec_parsing() {
        assert_eq!(
            parse_version_spec(">=22"),
            Some((RequirementOperator::AtLeast, "22".to_string()))
        );
        assert_eq!(
            parse_version_spec("^20.9.0"),
            Some((RequirementOperator::AtLeast, "20.9.0".to_string()))
        );
        assert_eq!(
            parse_version_spec("22.x"),
            Some((RequirementOperator::AtLeast, "22".to_string()))
        );
        assert_eq!(
            parse_version_spec("<=3.12"),
            Some((RequirementOperator::AtMost, "3.12".to_string()))
        );
        assert_eq!(
            parse_version_spec("1.85"),
            Some((RequirementOperator::Equals, "1.85".to_string()))
        );
        // invalid ⇒ None（不猜，§197 invalid requirement）。
        assert_eq!(parse_version_spec("lts/*"), None);
        assert_eq!(parse_version_spec("beta"), None);
    }

    #[test]
    fn node_engines_and_version_files() {
        let dir = temp("node");
        std::fs::write(
            dir.join("package.json"),
            r#"{ "engines": { "node": ">=22" } }"#,
        )
        .unwrap();
        std::fs::write(dir.join(".nvmrc"), "20.11.0\n").unwrap();
        let mut file_storage: Vec<String> = Vec::new();
        let facts = facts_with(&dir, &["package.json", ".nvmrc"], &mut file_storage);
        let (requirements, conflicts) = collect_requirements(&facts);
        // engines 与 .nvmrc 期望不一致 ⇒ §103/§327 冲突，不静默择一。
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].subject, "node");
        assert_eq!(conflicts[0].entries.len(), 2);
        // 冲突主体的所有 requirement 标记 not applicable。
        assert!(requirements
            .iter()
            .filter(|r| r.subject == "node")
            .all(|r| !r.applicable));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn consistent_sources_merge_without_conflict() {
        let dir = temp("consistent");
        std::fs::write(
            dir.join("package.json"),
            r#"{ "engines": { "node": ">=22" } }"#,
        )
        .unwrap();
        std::fs::write(dir.join(".nvmrc"), ">=22\n").unwrap();
        let mut file_storage: Vec<String> = Vec::new();
        let facts = facts_with(&dir, &["package.json", ".nvmrc"], &mut file_storage);
        let (requirements, conflicts) = collect_requirements(&facts);
        assert!(conflicts.is_empty());
        let node = requirements
            .iter()
            .find(|r| r.subject == "node")
            .expect("node requirement");
        assert_eq!(
            node.source, "package.json engines",
            "§44 priority: ecosystem_metadata outranks version_file"
        );
        assert!(node.evidence.contains("also stated by"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn python_requires_python_parsed() {
        let dir = temp("py");
        std::fs::write(
            dir.join("pyproject.toml"),
            "[project]\nname=\"x\"\nrequires-python = \">=3.12\"\n",
        )
        .unwrap();
        let mut file_storage: Vec<String> = Vec::new();
        let facts = facts_with(&dir, &["pyproject.toml"], &mut file_storage);
        let (requirements, conflicts) = collect_requirements(&facts);
        assert!(conflicts.is_empty());
        let python = requirements
            .iter()
            .find(|r| r.subject == "python" && r.source.contains("requires-python"))
            .expect("python requirement");
        assert_eq!(python.operator, RequirementOperator::AtLeast);
        assert_eq!(python.expected, "3.12");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn dotnet_global_json_and_rust_toolchain() {
        let dir = temp("multi");
        std::fs::write(
            dir.join("global.json"),
            r#"{ "sdk": { "version": "9.0.200" } }"#,
        )
        .unwrap();
        std::fs::write(
            dir.join("rust-toolchain.toml"),
            "[toolchain]\nchannel = \"1.85\"\n",
        )
        .unwrap();
        let mut file_storage: Vec<String> = Vec::new();
        let facts = facts_with(
            &dir,
            &["global.json", "rust-toolchain.toml"],
            &mut file_storage,
        );
        let (requirements, _) = collect_requirements(&facts);
        let dotnet = requirements
            .iter()
            .find(|r| r.kind == RequirementKind::Sdk)
            .unwrap();
        assert_eq!(dotnet.expected, "9.0.200");
        assert_eq!(dotnet.source_kind, SourceKind::ProjectConfig);
        let rustc = requirements.iter().find(|r| r.subject == "rustc").unwrap();
        assert_eq!(rustc.expected, "1.85");
        let _ = std::fs::remove_dir_all(&dir);
    }

    // §364 golden path #5：README vs package.json 冲突。
    #[test]
    fn readme_conflicts_with_package_json() {
        let dir = temp("readme");
        std::fs::write(
            dir.join("package.json"),
            r#"{ "engines": { "node": ">=22" } }"#,
        )
        .unwrap();
        std::fs::write(
            dir.join("README.md"),
            "# Demo\n\nRequires Node 20 or later.\n",
        )
        .unwrap();
        let mut file_storage: Vec<String> = Vec::new();
        let facts = facts_with(&dir, &["package.json", "README.md"], &mut file_storage);
        let (requirements, conflicts) = collect_requirements(&facts);
        assert_eq!(
            conflicts.len(),
            1,
            "README Node 20 vs engines >=22 ⇒ conflict"
        );
        assert_eq!(conflicts[0].entries.len(), 2);
        let readme_entry = conflicts[0]
            .entries
            .iter()
            .find(|entry| entry.source == "README")
            .unwrap();
        assert_eq!(readme_entry.source_kind, SourceKind::Documentation);
        assert_eq!(readme_entry.expected, "20");
        assert_eq!(readme_entry.operator, RequirementOperator::AtLeast);
        // §43/§364：两条都保留。
        assert_eq!(
            requirements.iter().filter(|r| r.subject == "node").count(),
            2
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn container_presence_requires_docker_tool() {
        let dir = temp("container");
        std::fs::write(dir.join("Dockerfile"), "FROM node:22\n").unwrap();
        let mut file_storage: Vec<String> = Vec::new();
        let facts = facts_with(&dir, &["Dockerfile"], &mut file_storage);
        let (requirements, _) = collect_requirements(&facts);
        let docker = requirements.iter().find(|r| r.subject == "docker").unwrap();
        assert_eq!(docker.operator, RequirementOperator::Exists);
        assert_eq!(docker.source_kind, SourceKind::ContainerConfig);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
