//! 模块层级构建集成测试。

use prism_analysis::{CodeAnalysisOptions, CodeAnalysisService};
use prism_core::task::CancellationToken;
use prism_graph::{build_modules, ModuleKind};
use prism_project::{DiscoveryService, ScanOptions};
use std::path::PathBuf;

fn temp_root(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "prism-graph-{name}-{}-{}",
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

fn modules_of(root: &std::path::Path) -> Vec<prism_graph::Module> {
    let discovery = DiscoveryService::new()
        .discover(
            root,
            ScanOptions::default(),
            &CancellationToken::new(),
            |_| {},
        )
        .unwrap();
    let model = CodeAnalysisService::new()
        .analyze(
            &discovery,
            CodeAnalysisOptions::default(),
            &CancellationToken::new(),
            |_| {},
        )
        .unwrap();
    build_modules(&discovery, &model)
}

#[test]
fn builds_project_file_and_package_hierarchy() {
    let root = temp_root("packages");
    std::fs::create_dir_all(root.join("app")).unwrap();
    std::fs::write(root.join("app/__init__.py"), "").unwrap();
    std::fs::write(root.join("app/main.py"), "def main():\n    pass\n").unwrap();
    std::fs::write(root.join("app/__init__.py"), "").unwrap();
    std::fs::write(root.join("top.py"), "x = 1\n").unwrap();

    let modules = modules_of(&root);
    let kinds: Vec<(&str, ModuleKind)> = modules
        .iter()
        .map(|m| (m.path.as_deref().unwrap_or(m.name.as_str()), m.kind))
        .collect();

    assert!(kinds.contains(&("top.py", ModuleKind::FileModule)));
    assert!(kinds.contains(&("app", ModuleKind::Package)));
    assert!(modules.iter().any(|m| m.kind == ModuleKind::Project));

    // app/main.py 的父级是 app 包。
    let main = modules
        .iter()
        .find(|m| m.path.as_deref() == Some("app/main.py"))
        .unwrap();
    let package = modules
        .iter()
        .find(|m| m.kind == ModuleKind::Package && m.path.as_deref() == Some("app"))
        .unwrap();
    assert_eq!(main.parent_id.as_ref(), Some(&package.id));
    // 包聚合其文件。
    assert!(package.file_ids.contains(&main.file_ids[0]));
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn csharp_namespaces_become_namespace_modules() {
    let root = temp_root("namespaces");
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(
        root.join("src/UserService.cs"),
        "namespace App.Services\n{\n    public class UserService\n    {\n    }\n}\n",
    )
    .unwrap();

    let modules = modules_of(&root);
    let ns = modules
        .iter()
        .find(|m| m.kind == ModuleKind::Namespace)
        .expect("namespace module must exist");
    // name = 最后一段；完整命名空间在 id 中（module:namespace:App.Services）。
    assert_eq!(ns.name, "Services");
    assert!(ns.id.as_str().ends_with("App.Services"));

    let file = modules
        .iter()
        .find(|m| m.path.as_deref() == Some("src/UserService.cs"))
        .unwrap();
    assert_eq!(file.parent_id.as_ref(), Some(&ns.id));
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn module_ids_are_deterministic_across_runs() {
    let root = temp_root("ids");
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(root.join("src/a.ts"), "export class A {}\n").unwrap();
    let ids = || {
        modules_of(&root)
            .iter()
            .map(|m| m.id.to_string())
            .collect::<Vec<_>>()
    };
    let first = ids();
    let second = ids();
    std::fs::remove_dir_all(&root).ok();
    assert_eq!(first, second, "same structure must yield same module ids");
}
