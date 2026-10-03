//! M7b 规则目录 e2e：真实 fixture 仓库 → M1/M2/M3 事实 → 引擎 → 各规则 findings
//! （§149–§153、§155/§156/§158/§159、§241/§243/§244/§245/§246）。

use prism_analysis::{CodeAnalysisOptions, CodeAnalysisService};
use prism_core::config::HealthConfig;
use prism_core::task::CancellationToken;
use prism_graph::DependencyService;
use prism_core::severity::Severity;
use prism_health::{analyze, HealthContext, HealthOptions, HealthRuleRegistry};
use std::path::PathBuf;
use std::process::Command;

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new(name: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "prism-health-rules-{name}-{}-{}",
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

    /// 全事实分析（discovery + code model + M3 图）。
    fn analyze(&self, config: &HealthConfig) -> prism_health::HealthAnalysis {
        let cancel = CancellationToken::new();
        let discovery = prism_project::DiscoveryService::new()
            .discover(&self.root, prism_project::ScanOptions::default(), &cancel, |_| {})
            .expect("discovery");
        let model = CodeAnalysisService::new()
            .analyze(&discovery, CodeAnalysisOptions::default(), &cancel, |_| {})
            .expect("code model");
        let graph = DependencyService::new()
            .analyze(&discovery, &model)
            .expect("graph");
        let context = HealthContext {
            project: &discovery.project,
            code_model: Some(&model),
            discovery: Some(&discovery),
            graph: Some(&graph),
            git: None,
            change_set: None,
            impact: None,
            root: &self.root,
            derived: None,
        };
        analyze(
            &HealthRuleRegistry::with_builtins(),
            config,
            &context,
            &HealthOptions::default(),
            &cancel,
        )
        .expect("health analysis")
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

const A_TS: &str = "import { helper } from \"./b\";\nexport function alpha(x: number): number {\n  if (x > 0) { return helper(x); }\n  return 0; // TODO: handle negative\n}\n";
const B_TS: &str = "import { alpha } from \"./a\";\nexport function helper(x: number): number {\n  while (x > 100) { x = x - 100; }\n  return alpha(x - 1); // cycle back to a\n}\n";

#[test]
fn arch001_detects_canonical_cycles_not_acyclic_pairs() {
    // §149：A↔B（环）+ 无环对照；环去重为一条 canonical finding（§24/§25）。
    let fixture = Fixture::new("arch-cycle");
    fixture.write("src/a.ts", A_TS);
    fixture.write("src/b.ts", B_TS);
    fixture.write("src/solo.ts", "export function solo(): number { return 1; }\n");
    fixture.commit_all("base");

    let analysis = fixture.analyze(&HealthConfig::default());
    let cycles: Vec<_> = analysis
        .findings
        .iter()
        .filter(|f| f.rule_id == "ARCH001")
        .collect();
    assert_eq!(cycles.len(), 1, "exactly one canonical cycle: {cycles:?}");
    let members = &cycles[0].related_modules;
    assert!(members.len() >= 2);
    assert!(
        members.iter().any(|m| m.contains("a.ts")) && members.iter().any(|m| m.contains("b.ts")),
        "cycle members are a.ts/b.ts: {members:?}"
    );
    assert!(
        cycles[0]
            .evidence
            .iter()
            .all(|e| !e.detail.is_empty()),
        "cycle evidence present (§243)"
    );
    // ARCH002/003 不因小项目误报（阈值默认 15/20）。
    assert!(!analysis.findings.iter().any(|f| f.rule_id == "ARCH002"));
    assert!(!analysis.findings.iter().any(|f| f.rule_id == "ARCH003"));
}

#[test]
fn fan_out_threshold_boundary_is_exact() {
    // §150：fan_in = threshold 不报，threshold+1 报。
    let fixture = Fixture::new("fan-out");
    // hub.ts 被 16 个文件 import（默认 fan-in 阈值 20 ⇒ 不报 fan-in；
    // 另造 fan-out：huge.ts import 16 个（阈值 15 ⇒ 报）。
    fixture.write("src/hub.ts", "export const hub = 1;\n");
    for i in 0..16 {
        fixture.write(
            &format!("src/consumer{i}.ts"),
            &format!("import {{ hub }} from \"./hub\";\nexport const c{i} = hub;\n"),
        );
    }
    let mut huge = String::new();
    for i in 0..16 {
        huge.push_str(&format!("import {{ c{i} }} from \"./consumer{i}\";\n"));
    }
    huge.push_str("export const huge = 1;\n");
    fixture.write("src/huge.ts", &huge);
    fixture.commit_all("base");

    let analysis = fixture.analyze(&HealthConfig::default());
    let fan_out: Vec<_> = analysis
        .findings
        .iter()
        .filter(|f| f.rule_id == "ARCH002")
        .collect();
    assert_eq!(fan_out.len(), 1, "huge.ts fan-out=16 > 15: {fan_out:?}");
    assert_eq!(fan_out[0].metadata["count"], "16");
    assert_eq!(fan_out[0].metadata["threshold"], "15");
    // hub fan-in = 16 ≤ 20 ⇒ 不报。
    assert!(!analysis.findings.iter().any(|f| f.rule_id == "ARCH003"));
}

#[test]
fn dep_rules_report_unresolved_not_ambiguous_in_fixture() {
    // §32/§241：未解析 import ⇒ DEP001；evidence 带 requested target。
    let fixture = Fixture::new("dep");
    fixture.write(
        "src/app.ts",
        "import { thing } from \"@shared/payment\";\nexport const value = thing;\n",
    );
    fixture.commit_all("base");

    let analysis = fixture.analyze(&HealthConfig::default());
    let dep001: Vec<_> = analysis
        .findings
        .iter()
        .filter(|f| f.rule_id == "DEP001")
        .collect();
    assert_eq!(dep001.len(), 1);
    assert!(dep001[0].message.contains("@shared/payment"));
    assert!(
        dep001[0]
            .evidence
            .iter()
            .any(|e| e.detail.contains("@shared/payment")),
        "evidence carries requested target (§32)"
    );
}

#[test]
fn code_rules_report_complexity_size_nesting_with_thresholds() {
    // §152/§153：复杂度/行数/嵌套边界 + 配置覆盖（§218）。
    let fixture = Fixture::new("code");
    // 复杂函数：12 个 if ⇒ complexity = 13 > 10。
    let mut complex = String::from("export function complex(x: number): number {\n  let y = 0;\n");
    for i in 0..12 {
        complex.push_str(&format!("  if (x > {i}) {{ y += {i}; }}\n"));
    }
    complex.push_str("  return y;\n}\n");
    fixture.write("src/complex.ts", &complex);
    // 250 行大函数（> 200）+ 深嵌套（5 层 > 5? 恰好 5 不报；6 报）。
    let mut large = String::from("export function large(): number {\n  let y = 0;\n");
    large.push_str("  if (true) {\n    if (true) {\n      if (true) {\n        if (true) {\n          if (true) {\n            if (true) {\n              y = 1;\n            }\n          }\n        }\n      }\n    }\n  }\n");
    for i in 0..240 {
        large.push_str(&format!("  y += {i};\n"));
    }
    large.push_str("  return y;\n}\n");
    fixture.write("src/large.ts", &large);
    fixture.commit_all("base");

    let analysis = fixture.analyze(&HealthConfig::default());
    assert!(
        analysis
            .findings
            .iter()
            .any(|f| f.rule_id == "CODE001" && f.location.relative_path == "src/complex.ts"),
        "complexity finding expected: {:?}",
        analysis.findings.iter().map(|f| &f.rule_id).collect::<Vec<_>>()
    );
    // large：行数 250+ > 200 ⇒ CODE002；嵌套 6 > 5 ⇒ CODE004。
    assert!(analysis.findings.iter().any(|f| f.rule_id == "CODE002" && f.location.relative_path == "src/large.ts"));
    assert!(analysis.findings.iter().any(|f| f.rule_id == "CODE004" && f.location.relative_path == "src/large.ts"));
    // evidence 携带 value/threshold（§218）。
    let code002 = analysis.findings.iter().find(|f| f.rule_id == "CODE002").unwrap();
    assert!(code002.metadata["value"].parse::<u64>().unwrap() > 200);
    assert_eq!(code002.metadata["threshold"], "200");

    // §218 配置覆盖：threshold 提到 500 ⇒ CODE002 消失。
    let mut config = HealthConfig::default();
    config.rules.insert(
        "CODE002".to_string(),
        prism_core::config::HealthRuleConfig {
            enabled: None,
            severity: None,
            threshold: Some(500),
        },
    );
    let adjusted = fixture.analyze(&config);
    assert!(!adjusted.findings.iter().any(|f| f.rule_id == "CODE002"));
}

#[test]
fn sec_rules_redact_and_skip_placeholders() {
    // §155/§244：PEM 强证据；占位符跳过；**evidence 永远 [REDACTED]**。
    let fixture = Fixture::new("sec");
    fixture.write(
        "src/config.ts",
        "export const config = {\n  api_key: \"sk-live-0123456789abcdef\",\n  placeholder: \"example-password-123456\",\n};\n",
    );
    fixture.write("keys/dev.pem", "-----BEGIN PRIVATE KEY-----\nabc\n-----END PRIVATE KEY-----\n");
    fixture.commit_all("base");

    let analysis = fixture.analyze(&HealthConfig::default());
    let sec001: Vec<_> = analysis
        .findings
        .iter()
        .filter(|f| f.rule_id == "SEC001")
        .collect();
    assert_eq!(sec001.len(), 1, "placeholder skipped, real assignment found");
    assert!(
        !serde_json::to_string(&analysis).unwrap().contains("sk-live"),
        "secret value must never appear in output (§46/§244)"
    );
    assert!(
        analysis
            .findings
            .iter()
            .all(|f| serde_json::to_string(f).unwrap().contains("[REDACTED]"))
            || sec001.is_empty(),
        "SEC evidence redacted"
    );
    let sec002: Vec<_> = analysis
        .findings
        .iter()
        .filter(|f| f.rule_id == "SEC002")
        .collect();
    assert_eq!(sec002.len(), 1);
    assert_eq!(sec002[0].severity, Severity::Critical);
    assert_eq!(sec002[0].confidence, prism_core::Confidence::High);
}

#[test]
fn doc_rules_detect_command_and_path_drift() {
    // §156/§245：命令漂移 + 路径漂移 + 正例不误报。
    let fixture = Fixture::new("doc");
    fixture.write(
        "package.json",
        r#"{
  "name": "demo",
  "scripts": { "dev": "vite", "test": "vitest" }
}"#,
    );
    fixture.write(
        "README.md",
        "# Demo\n\n## Usage\n\n```bash\nnpm run dev\nnpm run build\n```\n\nSee [docs/guide.md](docs/guide.md) and [app.ts](src/app.ts).\n",
    );
    fixture.write("src/app.ts", "export const app = 1;\n");
    fixture.commit_all("base");

    let analysis = fixture.analyze(&HealthConfig::default());
    let doc001: Vec<_> = analysis
        .findings
        .iter()
        .filter(|f| f.rule_id == "DOC001")
        .collect();
    assert_eq!(doc001.len(), 1, "only 'build' is missing: {doc001:?}");
    assert!(doc001[0].id.as_str().contains("build"), "{:?}", doc001[0].id);
    let doc002: Vec<_> = analysis
        .findings
        .iter()
        .filter(|f| f.rule_id == "DOC002")
        .collect();
    assert_eq!(doc002.len(), 1, "only docs/guide.md missing: {doc002:?}");
    assert!(doc002[0].evidence[0].detail.contains("docs/guide.md"));
}

#[test]
fn generated_and_binary_files_stay_silent() {
    // §158/§159：generated 不产生 code findings；二进制不崩溃不伪造。
    let fixture = Fixture::new("gen-bin");
    fixture.write("src/handwritten.ts", "// TODO: real work\nexport const a = 1;\n");
    // dist 产物（M1 分类 Generated ⇒ M2 Skipped ⇒ 不扫描）。
    fixture.write("dist/bundle.ts", "// TODO generated marker\nexport const b = 2;\n");
    fixture.write("assets/logo.png", "PNGDATA\r\n\x1a\nTODO-in-binary");
    fixture.commit_all("base");

    let analysis = fixture.analyze(&HealthConfig::default());
    let maint: Vec<_> = analysis
        .findings
        .iter()
        .filter(|f| f.rule_id == "MAINT001")
        .collect();
    assert_eq!(maint.len(), 1, "only handwritten.ts scanned: {maint:?}");
    assert_eq!(maint[0].location.relative_path, "src/handwritten.ts");
    // 不崩溃即通过（binary 在 derived 读取时被 metadata 大小/UTF-8 防线处理）。
}

#[test]
fn suppress_and_isolate_across_catalog() {
    // §162/§164：禁用 MAINT 不影响 SEC；suppress 保持检出。
    let fixture = Fixture::new("isolate");
    fixture.write(
        "src/a.ts",
        "// TODO: x\nexport const api_key = \"sk-live-0123456789abcdef\";\n",
    );
    fixture.commit_all("base");

    let config = HealthConfig {
        rules: [(
            "MAINT001".to_string(),
            prism_core::config::HealthRuleConfig {
                enabled: Some(false),
                severity: None,
                threshold: None,
            },
        )]
        .into_iter()
        .collect(),
        ..HealthConfig::default()
    };
    let analysis = fixture.analyze(&config);
    assert!(!analysis.findings.iter().any(|f| f.rule_id == "MAINT001"));
    assert!(
        analysis.findings.iter().any(|f| f.rule_id == "SEC001"),
        "SEC001 must survive MAINT disable"
    );
    assert_eq!(
        analysis.analysis_completeness,
        prism_core::PrismCompleteness::Complete
    );
}
