use prism_analysis::{CodeAnalysisOptions, CodeAnalysisService};
use prism_core::task::CancellationToken;
use prism_project::{DiscoveryService, ScanOptions};

fn main() {
    let root = std::env::temp_dir().join("prism-analysis-probe2");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::create_dir_all(root.join("src/generated")).unwrap();
    std::fs::create_dir_all(root.join("vendor")).unwrap();
    std::fs::write(
        root.join("src/app.ts"),
        "export class App {\n    render(): string { return \"x\"; }\n}\n",
    )
    .unwrap();
    std::fs::write(
        root.join("src/service.py"),
        "class Service:\n    def run(self):\n        return 1\n",
    )
    .unwrap();
    std::fs::write(root.join("src/generated/old.ts"), "export class Old {}\n").unwrap();
    std::fs::write(root.join("vendor/lib.rs"), "struct RubyLike;\n").unwrap();
    std::fs::write(root.join("README.md"), "docs").unwrap();

    let discovery = DiscoveryService::new()
        .discover(
            &root,
            ScanOptions::default(),
            &CancellationToken::new(),
            |_| {},
        )
        .unwrap();
    println!("discovery: {}", discovery.files.len());
    for f in &discovery.files {
        println!(
            "  {:?} {:?} analyzable={} lang={:?}",
            f.relative_path, f.classification.kind, f.analyzable, f.language
        );
    }
    let model = CodeAnalysisService::new()
        .analyze(
            &discovery,
            CodeAnalysisOptions::default(),
            &CancellationToken::new(),
            |_| {},
        )
        .unwrap();
    println!("model: {}", model.parse_results.len());
    for r in &model.parse_results {
        println!("  {:?} {:?}", r.relative_path, r.status);
    }
}
