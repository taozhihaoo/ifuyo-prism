//! M7a 端到端：真实文件 → M1/M2 管线 → HealthContext → 引擎 → MAINT001 findings
//! （规格 §148/§154/§157/§161/§164/§116/§117）。

use prism_analysis::{CodeAnalysisOptions, CodeAnalysisService};
use prism_core::config::{HealthConfig, HealthRuleConfig};
use prism_core::runtime::command::StdCommandRunner;
use prism_core::task::CancellationToken;
use prism_health::{
    analyze, FindingStatus, HealthContext, HealthOptions, HealthRuleRegistry,
};
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new(name: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "prism-health-e2e-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.subsec_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        Self::git(&root, &["init", "-b", "main"]);
        Self::git(&root, &["config", "user.name", "Prism Test"]);
        Self::git(&root, &["config", "user.email", "test@prism.local"]);
        Fixture { root }
    }

    fn git(root: &std::path::Path, args: &[&str]) {
        let output = Command::new("git")
            .args(args)
            .current_dir(root)
            .output()
            .expect("git must be available");
        assert!(
            output.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn write(&self, relative: &str, content: &str) {
        let path = self.root.join(relative);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, content).unwrap();
    }

    fn commit_all(&self, message: &str) {
        Self::git(&self.root, &["add", "-A"]);
        Self::git(&self.root, &["commit", "-m", message]);
    }

    fn analyze(&self, config: &HealthConfig, options: &HealthOptions) -> prism_health::HealthAnalysis {
        let cancel = CancellationToken::new();
        let discovery = prism_project::DiscoveryService::new()
            .discover(&self.root, prism_project::ScanOptions::default(), &cancel, |_| {})
            .expect("discovery");
        let model = CodeAnalysisService::new()
            .analyze(&discovery, CodeAnalysisOptions::default(), &cancel, |_| {})
            .expect("code model");
        let context = HealthContext {
            project: &discovery.project,
            code_model: Some(&model),
            discovery: Some(&discovery),
            graph: None,
            git: None,
            change_set: None,
            impact: None,
            root: &self.root,
        };
        analyze(&HealthRuleRegistry::with_builtins(), config, &context, options, &cancel)
            .expect("health analysis")
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

#[test]
fn maint_rule_finds_markers_and_avoids_false_positives() {
    let fixture = Fixture::new("maint");
    fixture.write(
        "src/app.ts",
        "// TODO: extract helper\nexport function app(): number {\n  const label = \"TODO: not a comment\";\n  // see http://example.com/TODO\n  return 1; // FIXME later\n}\n",
    );
    fixture.write("src/notes.ts", "// TODOS tracked elsewhere\n");
    // 不支持语言：词法规则不扫描（未知扩展名整行跳过，§117）。
    fixture.write("data.xyz", "// TODO in unknown language\n");
    // 解析失败文件：文件级规则仍运行（§116/§157）。
    fixture.write("src/broken.ts", "// FIXME broken marker\nexport function broken() {\n  return (((\n");
    fixture.commit_all("base");

    let analysis = fixture.analyze(&HealthConfig::default(), &HealthOptions::default());

    let markers: Vec<&str> = analysis
        .findings
        .iter()
        .filter(|f| f.rule_id == "MAINT001")
        .map(|f| f.metadata.get("marker").map(String::as_str).unwrap_or(""))
        .collect();
    assert_eq!(markers.len(), 3, "TODO + FIXME + broken FIXME: {markers:?}");
    assert!(markers.contains(&"TODO"));
    assert_eq!(markers.iter().filter(|m| **m == "FIXME").count(), 2);

    // 全部 Info/High（词法精确匹配），id 稳定含行号。
    assert!(
        analysis
            .findings
            .iter()
            .all(|f| f.rule_id != "MAINT001" || f.id.as_str().starts_with("finding:MAINT001:src/"))
    );
    // 状态默认 Active；无诊断噪音（§208）。
    assert!(
        analysis
            .findings
            .iter()
            .all(|f| f.status == FindingStatus::Active)
    );
    assert_eq!(analysis.analysis_completeness, prism_core::PrismCompleteness::Complete);
    assert!(analysis.checked_rules.contains(&"MAINT001".to_string()));

    // §161 确定性：再次运行输出一致。
    let again = fixture.analyze(&HealthConfig::default(), &HealthOptions::default());
    assert_eq!(
        serde_json::to_string(&analysis).unwrap(),
        serde_json::to_string(&again).unwrap()
    );
}

#[test]
fn suppression_config_marks_detected_findings() {
    let fixture = Fixture::new("suppress");
    fixture.write("src/a.ts", "// TODO one\n// TODO two\n");
    fixture.commit_all("base");

    let first = fixture.analyze(&HealthConfig::default(), &HealthOptions::default());
    assert_eq!(first.stats.findings_active, 2);
    let target_id = first.findings[0].id.as_str().to_string();

    let mut config = HealthConfig::default();
    config.suppressed_findings = vec![target_id.clone()];
    let second = fixture.analyze(&config, &HealthOptions::default());
    assert_eq!(second.stats.findings_detected, 2, "still detected (§67)");
    assert_eq!(second.stats.findings_suppressed, 1);
    assert_eq!(second.stats.findings_active, 1);
    let suppressed = second
        .findings
        .iter()
        .find(|f| f.id.as_str() == target_id)
        .expect("suppressed finding still present");
    assert_eq!(suppressed.status, FindingStatus::Suppressed);
}

#[test]
fn rule_disable_removes_findings_and_lists_rule() {
    let fixture = Fixture::new("disable");
    fixture.write("src/a.ts", "// TODO one\n");
    fixture.commit_all("base");

    let config = HealthConfig {
        rules: [(
            "MAINT001".to_string(),
            HealthRuleConfig {
                enabled: Some(false),
                severity: None,
                threshold: None,
            },
        )]
        .into_iter()
        .collect(),
        ..HealthConfig::default()
    };
    let analysis = fixture.analyze(&config, &HealthOptions::default());
    assert!(analysis.findings.is_empty());
    assert!(analysis.disabled_rules.contains(&"MAINT001".to_string()));
    assert!(!analysis.checked_rules.contains(&"MAINT001".to_string()));
}
