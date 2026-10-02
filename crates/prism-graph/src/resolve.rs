//! Import 解析（规格 §12–§16、§19）。
//!
//! 输出统一为 [`Dependency`] + [`ResolutionDiagnostic`]；无法可靠解析的
//! 关系显式标记（Unresolved/Ambiguous），禁止猜测（§19/§35：不做在线
//! package lookup，只基于项目文件与本地元数据）。
//!
//! 语言分派按 M1 语言枚举进行（集中在 [`resolve_import`]，规格 §12）。

use crate::model::{Dependency, DependencyKind, ResolutionDiagnostic, ResolutionStatus};
use prism_core::Confidence;
use prism_parser::parse::{ImportInfo, ImportKind};
use prism_project::Language;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// 解析所需的全部项目上下文（由调用方从 discovery + 配置文件构建）。
#[derive(Debug, Clone, Default)]
pub struct ResolutionContext {
    /// 项目内全部文件相对路径（`/` 分隔，来源 M1 discovery）。
    pub files: BTreeMap<String, Option<Language>>,
    /// 全部目录（`/` 分隔）。
    pub directories: BTreeSet<String>,
    /// tsconfig paths（pattern → 目标前缀列表；来自 tsconfig.json）。
    pub tsconfig_paths: Vec<(String, Vec<String>)>,
    /// tsconfig baseUrl（相对项目根）。
    pub tsconfig_base_url: Option<String>,
    /// package.json dependencies + devDependencies 的包名。
    pub package_dependencies: BTreeSet<String>,
    /// C# 命名空间 → 声明它的文件路径列表（来源 M2 符号）。
    pub csharp_namespaces: BTreeMap<String, Vec<String>>,
    /// (.csproj 路径, ProjectReference Include 原样值) 列表。
    pub project_references: Vec<(String, String)>,
}

/// 解析单个 import 事实为一条 Dependency（状态/证据齐备）。
pub fn resolve_import(
    ctx: &ResolutionContext,
    source_path: &str,
    import: &ImportInfo,
) -> Dependency {
    let language = ctx.files.get(source_path).copied().flatten();
    let source_id = prism_core::ids::ModuleId::from_string(format!("module:file:{source_path}"));
    let location = import.location;

    let (resolution, target_path, confidence, reason) = match language {
        Some(Language::TypeScript)
        | Some(Language::TypeScriptReact)
        | Some(Language::JavaScript) => resolve_web(ctx, source_path, &import.specifier),
        Some(Language::Python) => resolve_python(ctx, source_path, &import.specifier),
        Some(Language::CSharp) => resolve_csharp(ctx, &import.specifier),
        _ => (
            ResolutionStatus::Unresolved,
            None,
            Confidence::Low,
            "file language unknown".to_string(),
        ),
    };

    let kind = match import.kind {
        ImportKind::ReExport => DependencyKind::ReExport,
        _ => DependencyKind::Import,
    };

    Dependency {
        id: format!("dep:{source_path}#{}#{}", import.specifier, kind.as_str()),
        source_id,
        source_path: source_path.to_string(),
        raw_target: import.specifier.clone(),
        kind,
        resolution,
        confidence,
        target_id: target_path
            .as_ref()
            .map(|path| prism_core::ids::ModuleId::from_string(format!("module:file:{path}"))),
        target_path,
        imported_names: import.names.clone(),
        location,
        evidence: vec![prism_project::Evidence::new(
            "import_resolver",
            source_path,
            &reason,
        )],
    }
}

impl Dependency {
    /// 附带诊断构造（Unresolved/Ambiguous 时调用方使用）。
    pub fn with_diagnostic(
        mut self,
        resolver: &str,
        reason: &str,
    ) -> (Dependency, ResolutionDiagnostic) {
        let diagnostic = ResolutionDiagnostic {
            source_path: self.source_path.clone(),
            raw_target: self.raw_target.clone(),
            resolver: resolver.to_string(),
            status: self.resolution,
            reason: reason.to_string(),
            location: self.location,
        };
        self.evidence.push(prism_project::Evidence::new(
            "import_resolver",
            &self.raw_target,
            reason,
        ));
        (self, diagnostic)
    }
}

/// Node.js/bun 内置模块名（本地静态表，不做在线查询，规格 §35）。
const NODE_BUILTINS: &[&str] = &[
    "assert",
    "buffer",
    "child_process",
    "cluster",
    "console",
    "constants",
    "crypto",
    "dgram",
    "dns",
    "domain",
    "events",
    "fs",
    "http",
    "http2",
    "https",
    "module",
    "net",
    "os",
    "path",
    "perf_hooks",
    "process",
    "punycode",
    "querystring",
    "readline",
    "repl",
    "stream",
    "string_decoder",
    "timers",
    "tls",
    "tty",
    "url",
    "util",
    "v8",
    "vm",
    "worker_threads",
    "zlib",
];

/// Python 标准库常用模块（保守子集；未命中不等于不存在，走 Unresolved）。
const PYTHON_STDLIB: &[&str] = &[
    "abc",
    "argparse",
    "asyncio",
    "base64",
    "collections",
    "contextlib",
    "copy",
    "csv",
    "ctypes",
    "dataclasses",
    "datetime",
    "decimal",
    "difflib",
    "enum",
    "functools",
    "glob",
    "hashlib",
    "heapq",
    "http",
    "importlib",
    "inspect",
    "io",
    "itertools",
    "json",
    "logging",
    "math",
    "multiprocessing",
    "os",
    "pathlib",
    "pickle",
    "platform",
    "pprint",
    "queue",
    "random",
    "re",
    "shutil",
    "signal",
    "socket",
    "sqlite3",
    "ssl",
    "stat",
    "statistics",
    "string",
    "subprocess",
    "sys",
    "tempfile",
    "textwrap",
    "threading",
    "time",
    "tomllib",
    "traceback",
    "types",
    "typing",
    "unittest",
    "urllib",
    "uuid",
    "warnings",
    "weakref",
    "xml",
    "zipfile",
];

/// C# 常见框架命名空间前缀（保守子集）。
const CSHARP_BUILTIN_PREFIXES: &[&str] = &["System", "Microsoft", "internal."];

fn resolve_web(
    ctx: &ResolutionContext,
    source_path: &str,
    specifier: &str,
) -> (ResolutionStatus, Option<String>, Confidence, String) {
    // 1) 相对导入：基于源文件目录展开扩展名候选（规格 §14）。
    if specifier.starts_with("./") || specifier.starts_with("../") {
        let base = Path::new(source_path)
            .parent()
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_default();
        let joined = normalize_join(&base, specifier);
        if let Some(target) = resolve_file_candidates(ctx, &joined) {
            return (
                ResolutionStatus::Resolved,
                Some(target),
                Confidence::High,
                "relative import resolved to project file".to_string(),
            );
        }
        return (
            ResolutionStatus::Unresolved,
            None,
            Confidence::High,
            "relative specifier matches no project file".to_string(),
        );
    }

    // 2) tsconfig paths（精确键优先，其次最长通配前缀，规格 §14）。
    if !ctx.tsconfig_paths.is_empty() {
        if let Some((target, how)) = try_tsconfig_paths(ctx, specifier) {
            return (
                ResolutionStatus::Resolved,
                Some(target),
                Confidence::Medium,
                how,
            );
        }
    }

    // 3) node builtin（含 node: 前缀）。
    let root_segment = specifier.split('/').next().unwrap_or(specifier);
    if specifier.starts_with("node:") || NODE_BUILTINS.contains(&root_segment) {
        return (
            ResolutionStatus::Builtin,
            None,
            Confidence::High,
            "node builtin module".to_string(),
        );
    }

    // 4) package.json 声明的依赖 → External（本地元数据，不联网）。
    let package_name = if specifier.starts_with('@') {
        // @scope/name 取两段。
        specifier
            .splitn(3, '/')
            .take(2)
            .collect::<Vec<_>>()
            .join("/")
    } else {
        root_segment.to_string()
    };
    if ctx.package_dependencies.contains(&package_name) {
        return (
            ResolutionStatus::External,
            None,
            Confidence::High,
            "declared in package.json dependencies".to_string(),
        );
    }

    // 5) 项目内裸路径兜底：某些项目直接以根相对路径引用（如 "src/x"）。
    if let Some(target) = resolve_file_candidates(ctx, specifier) {
        return (
            ResolutionStatus::Resolved,
            Some(target),
            Confidence::Low,
            "bare specifier matched a project file".to_string(),
        );
    }

    (
        ResolutionStatus::Unresolved,
        None,
        Confidence::Low,
        "not relative, not a declared dependency, matches no project file".to_string(),
    )
}

fn try_tsconfig_paths(ctx: &ResolutionContext, specifier: &str) -> Option<(String, String)> {
    // 精确键优先。
    for (pattern, targets) in &ctx.tsconfig_paths {
        if !pattern.contains('*') && pattern == specifier {
            for target_prefix in targets {
                if let Some(target) = resolve_file_candidates(ctx, target_prefix) {
                    return Some((target, format!("tsconfig paths {pattern}")));
                }
            }
        }
    }
    // 通配键：取最长前缀匹配（tsconfig 语义：更具体的 pattern 优先）。
    let mut candidates: Vec<(usize, &String, &Vec<String>, String)> = Vec::new();
    for (pattern, targets) in &ctx.tsconfig_paths {
        let Some(star) = pattern.find('*') else {
            continue;
        };
        let prefix = &pattern[..star];
        let suffix = &pattern[star + 1..];
        if specifier.len() < prefix.len() + suffix.len()
            || !specifier.starts_with(prefix)
            || !specifier.ends_with(suffix)
        {
            continue;
        }
        let captured = &specifier[prefix.len()..specifier.len() - suffix.len()];
        candidates.push((prefix.len(), pattern, targets, captured.to_string()));
    }
    candidates.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(b.1)));
    for (_, pattern, targets, captured) in candidates {
        for target_prefix in targets {
            let base = target_prefix
                .replace('*', &captured)
                .trim_start_matches("./")
                .to_string();
            if let Some(target) = resolve_file_candidates(ctx, &base) {
                return Some((target, format!("tsconfig paths pattern {pattern}")));
            }
        }
    }
    None
}

fn resolve_python(
    ctx: &ResolutionContext,
    source_path: &str,
    specifier: &str,
) -> (ResolutionStatus, Option<String>, Confidence, String) {
    // 1) 相对导入：点数决定上溯层级（规格 §15）。
    let dots = specifier.chars().take_while(|c| *c == '.').count();
    if dots > 0 {
        let base_parts: Vec<&str> = source_path.split('/').collect();
        // base_parts 含文件名；源目录 = base_parts[..len-1]。
        let dir_len = base_parts.len().saturating_sub(1);
        let up = dir_len.saturating_sub(dots - 1);
        let mut parts: Vec<&str> = base_parts[..up.min(dir_len)].to_vec();
        let module_part = &specifier[dots..];
        if !module_part.is_empty() {
            parts.extend(module_part.split('.'));
        }
        let joined = parts.join("/");
        if let Some(target) = python_candidates(ctx, &joined) {
            return (
                ResolutionStatus::Resolved,
                Some(target),
                Confidence::High,
                "relative import resolved to project module".to_string(),
            );
        }
        return (
            ResolutionStatus::Unresolved,
            None,
            Confidence::Medium,
            "relative import matches no project module".to_string(),
        );
    }

    // 2) 绝对导入：项目文件优先（项目可遮蔽标准库）。
    let joined = specifier.replace('.', "/");
    if let Some(target) = python_candidates(ctx, &joined) {
        return (
            ResolutionStatus::Resolved,
            Some(target),
            Confidence::High,
            "absolute import resolved to project module".to_string(),
        );
    }

    // 3) 标准库（保守子表）。
    let first = specifier.split('.').next().unwrap_or(specifier);
    if PYTHON_STDLIB.contains(&first) {
        return (
            ResolutionStatus::Builtin,
            None,
            Confidence::Medium,
            "python standard library module".to_string(),
        );
    }

    (
        ResolutionStatus::Unresolved,
        None,
        Confidence::Medium,
        "matches no project module or known stdlib entry (runtime resolution depends on sys.path)"
            .to_string(),
    )
}

fn resolve_csharp(
    ctx: &ResolutionContext,
    specifier: &str,
) -> (ResolutionStatus, Option<String>, Confidence, String) {
    // 1) 精确命名空间命中。
    match ctx.csharp_namespaces.get(specifier) {
        Some(files) if files.len() == 1 => {
            return (
                ResolutionStatus::Resolved,
                Some(files[0].clone()),
                Confidence::High,
                "exact namespace match".to_string(),
            );
        }
        Some(files) => {
            return (
                ResolutionStatus::Ambiguous,
                None,
                Confidence::Medium,
                format!("namespace declared in {} files", files.len()),
            );
        }
        None => {}
    }

    // 2) 子命名空间前缀（using Foo.Bar 可服务 Foo.Bar.Baz 的类型）。
    let children: Vec<&String> = ctx
        .csharp_namespaces
        .keys()
        .filter(|ns| ns.starts_with(&format!("{specifier}.")))
        .collect();
    if let Some(first) = children.first() {
        let file = ctx.csharp_namespaces.get(*first).and_then(|v| v.first());
        if let Some(file) = file {
            return (
                ResolutionStatus::Resolved,
                Some(file.clone()),
                Confidence::Medium,
                "child namespace match".to_string(),
            );
        }
    }

    // 3) 框架前缀。
    if CSHARP_BUILTIN_PREFIXES
        .iter()
        .any(|prefix| specifier == *prefix || specifier.starts_with(&format!("{prefix}.")))
    {
        return (
            ResolutionStatus::Builtin,
            None,
            Confidence::Medium,
            "framework namespace".to_string(),
        );
    }

    (
        ResolutionStatus::Unresolved,
        None,
        Confidence::Low,
        "matches no project namespace or framework prefix".to_string(),
    )
}

/// 相对目录 + specifier 的文件候选展开（TS/JS 习惯，规格 §14）。
fn resolve_file_candidates(ctx: &ResolutionContext, base: &str) -> Option<String> {
    let mut candidates = vec![base.to_string()];
    for ext in ["ts", "tsx", "js", "jsx", "mjs", "cjs"] {
        candidates.push(format!("{base}.{ext}"));
        candidates.push(format!("{base}/index.{ext}"));
    }
    candidates
        .into_iter()
        .find(|candidate| ctx.files.contains_key(candidate))
}

/// Python 模块候选：`path` → path.py / path/__init__.py。
fn python_candidates(ctx: &ResolutionContext, base: &str) -> Option<String> {
    [format!("{base}.py"), format!("{base}/__init__.py")]
        .into_iter()
        .find(|candidate| ctx.files.contains_key(candidate))
}

fn normalize_join(base: &str, specifier: &str) -> String {
    let mut parts: Vec<&str> = if base.is_empty() {
        Vec::new()
    } else {
        base.split('/').collect()
    };
    for segment in specifier.split('/') {
        match segment {
            "." => {}
            ".." => {
                parts.pop();
            }
            other => parts.push(other),
        }
    }
    parts.join("/")
}

impl ResolutionContext {
    /// 从 discovery + code model + 项目配置文件构建上下文。
    ///
    /// 读取（均为项目根内的本地配置文件，规格 §35）：
    /// - `tsconfig.json`（compilerOptions.paths/baseUrl；容忍注释与 BOM）
    /// - `package.json`（dependencies/devDependencies 包名）
    /// - `*.csproj`（ProjectReference Include）
    pub fn build(
        root: &std::path::Path,
        discovery: &prism_project::ProjectDiscovery,
        model: &prism_analysis::ProjectCodeModel,
    ) -> Self {
        let mut ctx = ResolutionContext::default();

        for file in &discovery.files {
            ctx.files.insert(file.relative_path.clone(), file.language);
            if let Some(dir) = std::path::Path::new(&file.relative_path).parent() {
                let dir = dir.to_string_lossy().replace('\\', "/");
                if !dir.is_empty() {
                    ctx.directories.insert(dir);
                }
            }
        }

        // M2 符号中的 C# 命名空间 → 声明文件列表。
        for result in &model.parse_results {
            for symbol in &result.symbols {
                if symbol.kind != prism_parser::SymbolKind::Namespace {
                    continue;
                }
                let root_len = result
                    .relative_path
                    .rsplit_once('.')
                    .map_or(0, |(p, _)| p.chars().count() + 1);
                if let Some(ns) = symbol.qualified_name.get(root_len..) {
                    if !ns.is_empty() {
                        ctx.csharp_namespaces
                            .entry(ns.to_string())
                            .or_default()
                            .push(result.relative_path.clone());
                    }
                }
            }
        }

        // tsconfig.json
        if let Ok(text) = std::fs::read_to_string(root.join("tsconfig.json")) {
            let text = text.trim_start_matches('\u{feff}');
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(text)
                .or_else(|_| serde_json::from_str::<serde_json::Value>(&strip_comments(text)))
            {
                if let Some(options) = value.get("compilerOptions") {
                    if let Some(base) = options.get("baseUrl").and_then(|v| v.as_str()) {
                        ctx.tsconfig_base_url = Some(base.to_string());
                    }
                    if let Some(paths) = options.get("paths").and_then(|v| v.as_object()) {
                        for (pattern, targets) in paths {
                            let targets: Vec<String> = targets
                                .as_array()
                                .map(|array| {
                                    array
                                        .iter()
                                        .filter_map(|t| t.as_str().map(str::to_string))
                                        .collect()
                                })
                                .unwrap_or_default();
                            if !targets.is_empty() {
                                ctx.tsconfig_paths.push((pattern.clone(), targets));
                            }
                        }
                    }
                }
            }
        }

        // package.json dependencies
        if let Ok(text) = std::fs::read_to_string(root.join("package.json")) {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) {
                for section in ["dependencies", "devDependencies"] {
                    if let Some(deps) = value.get(section).and_then(|v| v.as_object()) {
                        for name in deps.keys() {
                            ctx.package_dependencies.insert(name.clone());
                        }
                    }
                }
            }
        }

        // *.csproj ProjectReference（naive 内容扫描；csproj 通常无注释干扰）。
        for file in &discovery.files {
            if file.relative_path.ends_with(".csproj") {
                let text =
                    std::fs::read_to_string(root.join(&file.relative_path)).unwrap_or_default();
                let mut rest = text.as_str();
                while let Some(pos) = rest.find("ProjectReference") {
                    rest = &rest[pos + "ProjectReference".len()..];
                    if let Some(start) = rest.find("Include=\"") {
                        let after = &rest[start + "Include=\"".len()..];
                        if let Some(end) = after.find('"') {
                            ctx.project_references
                                .push((file.relative_path.clone(), after[..end].to_string()));
                        }
                    }
                }
            }
        }

        ctx
    }
}

/// naive JSONC 注释剥离（tsconfig 常含注释；字符串内注释属罕见场景，接受局限）。
fn strip_comments(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    let mut in_string = false;
    let mut chars = text.chars().peekable();
    while let Some(current) = chars.next() {
        if in_string {
            output.push(current);
            if current == '\\' {
                if let Some(next) = chars.next() {
                    output.push(next);
                }
            } else if current == '"' {
                in_string = false;
            }
            continue;
        }
        match current {
            '"' => {
                in_string = true;
                output.push(current);
            }
            '/' if chars.peek() == Some(&'/') => {
                for next in chars.by_ref() {
                    if next == '\n' {
                        output.push(next);
                        break;
                    }
                }
            }
            '/' if chars.peek() == Some(&'*') => {
                chars.next();
                while let Some(next) = chars.next() {
                    if next == '*' && chars.peek() == Some(&'/') {
                        chars.next();
                        break;
                    }
                }
            }
            other => output.push(other),
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use prism_parser::parse::{ImportInfo, ImportKind};

    fn ctx() -> ResolutionContext {
        let mut ctx = ResolutionContext::default();
        for path in [
            ("src/main.ts", Language::TypeScript),
            ("src/main.tsx", Language::TypeScriptReact),
            ("src/main.cs", Language::CSharp),
            ("src/utils.ts", Language::TypeScript),
            ("src/utils/index.ts", Language::TypeScript),
            ("src/components/Button.tsx", Language::TypeScriptReact),
            ("src/main.py", Language::Python),
            ("app/main.py", Language::Python),
            ("app/services/user.py", Language::Python),
            ("app/services/__init__.py", Language::Python),
            ("app/__init__.py", Language::Python),
            ("src/Services/UserService.cs", Language::CSharp),
        ] {
            ctx.files.insert(path.0.to_string(), Some(path.1));
        }
        ctx.tsconfig_paths
            .push(("@/*".to_string(), vec!["src/*".to_string()]));
        ctx.package_dependencies.insert("react".to_string());
        ctx.csharp_namespaces.insert(
            "App.Services".to_string(),
            vec!["src/Services/UserService.cs".to_string()],
        );
        ctx
    }

    fn dep(
        ctx: &ResolutionContext,
        source: &str,
        _language: Language,
        specifier: &str,
    ) -> Dependency {
        resolve_import(
            ctx,
            source,
            &ImportInfo {
                specifier: specifier.to_string(),
                kind: ImportKind::Plain,
                names: vec![],
                location: None,
            },
        )
    }

    #[test]
    fn web_relative_resolves_with_extension_candidates() {
        let ctx = ctx();
        let d = dep(&ctx, "src/main.ts", Language::TypeScript, "./utils");
        assert_eq!(d.resolution, ResolutionStatus::Resolved);
        assert_eq!(d.target_path.as_deref(), Some("src/utils.ts"));

        let d = dep(
            &ctx,
            "src/main.tsx",
            Language::TypeScriptReact,
            "../src/components/Button.tsx",
        );
        assert_eq!(d.resolution, ResolutionStatus::Resolved);
        assert_eq!(d.target_path.as_deref(), Some("src/components/Button.tsx"));
    }

    #[test]
    fn web_index_and_paths_and_builtin_and_external() {
        let ctx = ctx();
        // index 候选
        let d = dep(&ctx, "src/main.ts", Language::TypeScript, "./utils/index");
        assert_eq!(d.target_path.as_deref(), Some("src/utils/index.ts"));
        // tsconfig paths 通配
        let d = dep(
            &ctx,
            "src/main.ts",
            Language::TypeScript,
            "@/components/Button",
        );
        assert_eq!(d.resolution, ResolutionStatus::Resolved);
        assert_eq!(d.target_path.as_deref(), Some("src/components/Button.tsx"));
        // builtin
        let d = dep(&ctx, "src/main.ts", Language::TypeScript, "fs");
        assert_eq!(d.resolution, ResolutionStatus::Builtin);
        // external
        let d = dep(&ctx, "src/main.ts", Language::TypeScript, "react");
        assert_eq!(d.resolution, ResolutionStatus::External);
        // unresolved
        let d = dep(&ctx, "src/main.ts", Language::TypeScript, "totally-missing");
        assert_eq!(d.resolution, ResolutionStatus::Unresolved);
    }

    #[test]
    fn python_relative_absolute_stdlib_unresolved() {
        let ctx = ctx();
        let d = dep(&ctx, "app/services/user.py", Language::Python, ".");
        assert_eq!(d.resolution, ResolutionStatus::Resolved);
        assert_eq!(d.target_path.as_deref(), Some("app/services/__init__.py"));

        let d = dep(&ctx, "app/main.py", Language::Python, "app.services.user");
        assert_eq!(d.resolution, ResolutionStatus::Resolved);
        assert_eq!(d.target_path.as_deref(), Some("app/services/user.py"));

        let d = dep(&ctx, "app/main.py", Language::Python, "os.path");
        assert_eq!(d.resolution, ResolutionStatus::Builtin);

        let d = dep(&ctx, "app/main.py", Language::Python, "external_pkg.mod");
        assert_eq!(d.resolution, ResolutionStatus::Unresolved);
    }

    #[test]
    fn csharp_exact_child_ambiguous_builtin_unresolved() {
        let mut ctx = ctx();
        let d = dep(&ctx, "src/main.cs", Language::CSharp, "App.Services");
        assert_eq!(d.resolution, ResolutionStatus::Resolved);
        assert_eq!(
            d.target_path.as_deref(),
            Some("src/Services/UserService.cs")
        );

        // 子命名空间前缀
        ctx.csharp_namespaces.insert(
            "App.Services.Internal".to_string(),
            vec!["src/Services/Internal.cs".to_string()],
        );
        let d = dep(&ctx, "src/main.cs", Language::CSharp, "App.Services");
        assert_eq!(d.resolution, ResolutionStatus::Resolved);

        // 歧义：两个文件声明同一命名空间
        ctx.csharp_namespaces
            .get_mut("App.Services")
            .unwrap()
            .push("src/Other.cs".to_string());
        let d = dep(&ctx, "src/main.cs", Language::CSharp, "App.Services");
        assert_eq!(d.resolution, ResolutionStatus::Ambiguous);

        let d = dep(
            &ctx,
            "src/main.cs",
            Language::CSharp,
            "System.Collections.Generic",
        );
        assert_eq!(d.resolution, ResolutionStatus::Builtin);

        let d = dep(&ctx, "src/main.cs", Language::CSharp, "Totally.Unknown");
        assert_eq!(d.resolution, ResolutionStatus::Unresolved);
    }

    #[test]
    fn re_export_kind_is_preserved() {
        let ctx = ctx();
        let d = resolve_import(
            &ctx,
            "src/main.ts",
            &ImportInfo {
                specifier: "./utils".to_string(),
                kind: ImportKind::ReExport,
                names: vec!["helper".to_string()],
                location: None,
            },
        );
        assert_eq!(d.kind, DependencyKind::ReExport);
        assert_eq!(d.resolution, ResolutionStatus::Resolved);
        assert_eq!(d.imported_names, vec!["helper".to_string()]);
    }
}
