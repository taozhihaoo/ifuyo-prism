//! 模块层级构建（规格 §6/§17）。
//!
//! ```text
//! Project（module:project:{name}）
//!   ├─ Package（module:package:{dir}，Python 含 __init__.py 的目录）
//!   ├─ Namespace（module:namespace:{qualified}，C#/TS namespace 符号）
//!   └─ FileModule（module:file:{relative_path}，每个被解析的源文件）
//! ```
//!
//! 没有证据不创造虚拟结构（规格 §17）：无 __init__.py 就没有 Package，
//! 无 namespace 符号就没有 Namespace 模块。

use crate::model::{Module, ModuleId, ModuleKind};
use prism_analysis::ProjectCodeModel;
use prism_parser::{ParseStatus, SymbolKind};
use prism_project::{Evidence, Language, ProjectDiscovery};
use std::collections::BTreeMap;

/// 从 discovery + code model 构建模块层级（输出按 id 排序，确定性）。
pub fn build_modules(discovery: &ProjectDiscovery, model: &ProjectCodeModel) -> Vec<Module> {
    let mut modules: BTreeMap<String, Module> = BTreeMap::new();
    let project_id = ModuleId::from_string(format!("module:project:{}", discovery.project.name));
    modules.insert(
        project_id.as_str().to_string(),
        Module {
            id: project_id.clone(),
            name: discovery.project.name.clone(),
            kind: ModuleKind::Project,
            file_ids: vec![],
            parent_id: None,
            language: None,
            path: None,
            evidence: vec![Evidence::new(
                "module_builder",
                ".",
                "project root discovered by M1",
            )],
        },
    );

    // ---- Python 包：含 __init__.py 的目录（证据驱动，规格 §17）----
    let mut package_dirs: BTreeMap<String, ModuleId> = BTreeMap::new();
    for file in &model.files {
        if file.language == Some(Language::Python) && file.relative_path.ends_with("__init__.py") {
            let dir = std::path::Path::new(&file.relative_path)
                .parent()
                .map(|p| p.to_string_lossy().replace('\\', "/"))
                .unwrap_or_default();
            if dir.is_empty() {
                continue;
            }
            let id = ModuleId::from_string(format!("module:package:{dir}"));
            let name = dir.rsplit('/').next().unwrap_or(&dir).to_string();
            modules
                .entry(id.as_str().to_string())
                .or_insert_with(|| Module {
                    id: id.clone(),
                    name,
                    kind: ModuleKind::Package,
                    file_ids: vec![],
                    parent_id: Some(project_id.clone()),
                    language: Some(Language::Python),
                    path: Some(dir.clone()),
                    evidence: vec![Evidence::new(
                        "module_builder",
                        format!("{dir}/__init__.py").as_str(),
                        "package marker __init__.py found",
                    )],
                });
            package_dirs.insert(dir, id);
        }
    }

    // ---- Namespace 模块：来自 code model 的 namespace 符号 ----
    let mut namespace_modules: BTreeMap<String, ModuleId> = BTreeMap::new();
    for result in &model.parse_results {
        for symbol in &result.symbols {
            if symbol.kind != SymbolKind::Namespace {
                continue;
            }
            // 去掉文件路径根前缀，取真实命名空间段（root = 去扩展名路径 + "."）。
            let root_len = result
                .relative_path
                .rsplit_once('.')
                .map_or(0, |(p, _)| p.chars().count() + 1);
            let Some(ns_name) = symbol.qualified_name.get(root_len..) else {
                continue;
            };
            if ns_name.is_empty() {
                continue;
            }
            let id = ModuleId::from_string(format!("module:namespace:{ns_name}"));
            if namespace_modules.contains_key(ns_name) {
                continue;
            }
            let name = ns_name.rsplit('.').next().unwrap_or(ns_name).to_string();
            let module = Module {
                id: id.clone(),
                name,
                kind: ModuleKind::Namespace,
                file_ids: vec![],
                parent_id: Some(project_id.clone()),
                language: Some(symbol.language),
                path: None,
                evidence: vec![Evidence::new(
                    "module_builder",
                    &result.relative_path,
                    "namespace declaration found",
                )],
            };
            modules.insert(id.as_str().to_string(), module);
            namespace_modules.insert(ns_name.to_string(), id);
        }
    }

    // ---- FileModule：每个成功进入管线的解析结果 ----
    for result in &model.parse_results {
        if matches!(
            result.status,
            ParseStatus::Unsupported | ParseStatus::Skipped
        ) {
            // 无代码事实的占位结果不构成 FileModule。
            continue;
        }
        let path = result.relative_path.clone();
        let id = ModuleId::from_string(format!("module:file:{path}"));

        // 父级判定：Python → 最近包目录；其他语言 → 文件声明的命名空间（最长匹配）。
        let mut parent = Some(project_id.clone());
        let mut parent_str = project_id.as_str().to_string();
        if result.language == Some(Language::Python) {
            let dir = std::path::Path::new(&path)
                .parent()
                .map(|p| p.to_string_lossy().replace('\\', "/"))
                .unwrap_or_default();
            if let Some(package_id) = package_dirs.get(&dir) {
                parent = Some(package_id.clone());
                parent_str = package_id.as_str().to_string();
            }
        } else {
            let declared: Vec<&str> = result
                .symbols
                .iter()
                .filter(|s| s.kind == SymbolKind::Namespace)
                .filter_map(|s| {
                    let root_len = result
                        .relative_path
                        .rsplit_once('.')
                        .map_or(0, |(p, _)| p.chars().count() + 1);
                    s.qualified_name.get(root_len..)
                })
                .collect();
            // 取最长声明的命名空间作为父容器（子命名空间优先于父）。
            if let Some(ns) = declared.iter().max_by_key(|ns| ns.len()) {
                if let Some(ns_id) = namespace_modules.get(*ns) {
                    parent = Some(ns_id.clone());
                    parent_str = ns_id.as_str().to_string();
                }
            }
        }

        let name = std::path::Path::new(&path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.clone());
        let module = Module {
            id: id.clone(),
            name,
            kind: ModuleKind::FileModule,
            file_ids: vec![result.file_id.clone()],
            parent_id: parent.clone(),
            language: result.language,
            path: Some(path.clone()),
            evidence: vec![Evidence::new(
                "module_builder",
                &path,
                "parsed source file from M2",
            )],
        };
        modules.insert(id.as_str().to_string(), module);
        // FileModule 归属父容器（Project/Package/Namespace 聚合 file_ids）。
        if let Some(container) = modules.get_mut(&parent_str) {
            container.file_ids.push(result.file_id.clone());
        }
    }

    let mut list: Vec<Module> = modules.into_values().collect();
    list.sort_by(|a, b| a.id.cmp(&b.id));
    list
}
