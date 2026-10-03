//! M6 端到端测试：真实 git 仓库 → M5 ChangeSet → M3 graph → ImpactAnalysis
//! （规格 §104–§108、§113–§115、§138–§141 主验收场景）。

use prism_analysis::{CodeAnalysisOptions, CodeAnalysisService};
use prism_changes::{ChangeScope, ChangeSetOptions, ChangeSetService};
use prism_core::runtime::command::StdCommandRunner;
use prism_core::task::CancellationToken;
use prism_graph::DependencyService;
use prism_impact::{
    ImpactAnalysisService, ImpactCompleteness, ImpactKind, ImpactOptions, ImpactReason,
    NodeKind, RiskSignalKind,
};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new(name: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "prism-impact-e2e-{name}-{}-{}",
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

    fn git(root: &Path, args: &[&str]) {
        let output = Command::new("git")
            .args(args)
            .current_dir(root)
            .env("GIT_AUTHOR_DATE", "2026-01-01T00:00:00+00:00")
            .env("GIT_COMMITTER_DATE", "2026-01-01T00:00:00+00:00")
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

    fn rename(&self, from: &str, to: &str) {
        Self::git(&self.root, &["mv", from, to]);
    }

    fn commit_all(&self, message: &str) {
        Self::git(&self.root, &["add", "-A"]);
        Self::git(&self.root, &["commit", "-m", message]);
    }

    /// 全链路：ChangeSet（工作区）+ 代码模型 + M3 图 → ImpactAnalysis。
    fn analyze(&self, options: &ImpactOptions) -> prism_impact::ImpactAnalysis {
        let cancel = CancellationToken::new();
        let runner: Arc<dyn prism_core::CommandRunner> = Arc::new(StdCommandRunner::new());
        let changeset = ChangeSetService::new(runner.clone())
            .build(
                &self.root,
                ChangeScope::WorkingTree,
                None,
                None,
                &ChangeSetOptions::default(),
                &cancel,
            )
            .expect("changeset")
            .expect("git repo");
        let discovery = prism_project::DiscoveryService::new()
            .discover(&self.root, prism_project::ScanOptions::default(), &cancel, |_| {})
            .expect("discovery");
        let model = CodeAnalysisService::new()
            .analyze(&discovery, CodeAnalysisOptions::default(), &cancel, |_| {})
            .expect("code model");
        let graph = DependencyService::new()
            .analyze(&discovery, &model)
            .expect("graph");
        ImpactAnalysisService
            .analyze(&changeset, &model, &graph, options, &cancel)
            .expect("impact analysis")
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

const LIB: &str = "export function pay(amount: number): number {\n  return amount * 2;\n}\n";
const CHECKOUT: &str = "import { pay } from \"./lib\";\nexport function checkout(amount: number): number {\n  return pay(amount) + 1;\n}\n";
const TEST: &str = "import { checkout } from \"./checkout\";\nexport function testCheckout(): number {\n  return checkout(1);\n}\n";

/// §138 主场景：改 pay() ⇒ checkout=Caller(1)；测试模块=transitive TestCandidate。
#[test]
fn main_scenario_body_change_propagates_to_callers_and_tests() {
    let fixture = Fixture::new("main");
    fixture.write("lib.ts", LIB);
    fixture.write("checkout.ts", CHECKOUT);
    fixture.write("checkout.test.ts", TEST);
    fixture.commit_all("base");
    fixture.write("lib.ts", &LIB.replace("amount * 2", "amount * 3"));

    let analysis = fixture.analyze(&ImpactOptions::default());

    // Changed 集合（§48）：pay 是 DirectlyChanged，不出现在 impacted 里。
    assert!(analysis.changed_items.iter().any(|item| item.name == "pay"));
    assert!(
        !analysis
            .impacted_items
            .iter()
            .any(|item| item.node_id.contains("#pay#")),
        "changed symbol must not double as impacted (§49)"
    );

    // checkout() 调用 pay() ⇒ Caller distance 1。
    let checkout = analysis
        .impacted_items
        .iter()
        .find(|item| item.node_kind == NodeKind::Symbol && item.name == "checkout")
        .expect("checkout caller expected");
    assert_eq!(checkout.distance, 1);
    assert!(checkout.impact_kinds.contains(&ImpactKind::Caller));
    assert!(checkout.reasons.contains(&ImpactReason::CallsChangedSymbol));

    // 测试模块：checkout.test.ts 依赖 checkout.ts 依赖 lib.ts ⇒ distance 2 + TestCandidate。
    let test_module = analysis
        .impacted_items
        .iter()
        .find(|item| item.path == "checkout.test.ts")
        .expect("test module expected");
    assert_eq!(test_module.distance, 2);
    assert!(test_module.impact_kinds.contains(&ImpactKind::TestCandidate));
    assert!(test_module.reasons.contains(&ImpactReason::TransitiveDependency));

    // 汇总与信号（§38/§40）：事实计数 + 跨模块事实信号，无总分。
    assert!(analysis.summary.direct_impacted_symbols >= 1);
    assert!(analysis.summary.potentially_affected_tests >= 1);
    assert!(
        analysis
            .risk_signals
            .iter()
            .any(|signal| signal.kind == RiskSignalKind::CrossModuleImpact)
    );
    assert!(
        !analysis
            .risk_signals
            .iter()
            .any(|signal| signal.detail.to_lowercase().contains("score")),
        "no overall risk score (§41)"
    );
    assert_eq!(analysis.analysis_completeness, ImpactCompleteness::Complete);

    // 证据路径（§30）：pay → checkout 的 calls 跳可追溯。
    assert!(
        analysis
            .impact_paths
            .iter()
            .any(|path| path.target.contains("checkout") && path.distance == 1),
        "evidence path to caller expected: {:?}",
        analysis.impact_paths
    );
}

/// §139 核心验收：删除被调用符号 ⇒ 调用点经名字索引成为 PotentiallyAffected。
#[test]
fn deleted_symbol_reports_stale_callers() {
    let fixture = Fixture::new("deleted");
    fixture.write("lib.ts", LIB);
    fixture.write(
        "report.ts",
        "import { pay } from \"./lib\";\nexport function report(amount: number): number {\n  return pay(amount) - 1;\n}\n",
    );
    fixture.commit_all("base");
    // 删除 pay；report.ts 的调用保持不变（现在解析失败——正是要暴露的场景）。
    fixture.write("lib.ts", "export const version = 2;\n");

    let analysis = fixture.analyze(&ImpactOptions::default());

    let caller = analysis
        .impacted_items
        .iter()
        .find(|item| item.node_kind == NodeKind::Symbol && item.name == "report")
        .expect("stale caller expected (§139)");
    assert_eq!(caller.distance, 1);
    assert!(caller.reasons.contains(&ImpactReason::CallsDeletedSymbol));

    assert!(
        analysis
            .risk_signals
            .iter()
            .any(|signal| signal.kind == RiskSignalKind::DeletedReferencedSymbol),
        "DeletedReferencedSymbol signal expected (§130)"
    );
    assert!(
        analysis
            .diagnostics
            .iter()
            .any(|d| matches!(
                d.kind,
                prism_impact::ImpactDiagnosticKind::UnresolvedRelationship
            )),
        "unresolved relationship must be visible (§44/§131)"
    );
}

/// §104/§105/§106：部分 ChangeSet（含不支持语言/二进制）⇒ Partial + 有效分析。
#[test]
fn partial_changeset_yields_partial_but_valid_analysis() {
    let fixture = Fixture::new("partial");
    fixture.write("lib.ts", LIB);
    fixture.write("checkout.ts", CHECKOUT);
    fixture.commit_all("base");
    fixture.write("lib.ts", &LIB.replace("amount * 2", "amount * 7"));
    fixture.write("data.xyz", "opaque v2\n");
    fixture.write_bytes("assets/logo.png", &[0x89u8, 0x50, 0x4e, 0x47, 0x00, 0x02]);

    let analysis = fixture.analyze(&ImpactOptions::default());
    assert_eq!(
        analysis.analysis_completeness,
        ImpactCompleteness::Partial,
        "unsupported/binary changes limit completeness (§127)"
    );
    // 不支持语言不伪造符号图（§105）；二进制符号影响 NotApplicable（§106）。
    assert!(
        !analysis
            .impacted_items
            .iter()
            .any(|item| item.path.ends_with(".xyz") || item.path.ends_with(".png")),
        "no fake symbol impact for unsupported/binary files"
    );
    // 仍然给出 lib 的调用者影响（部分成功，§104）。
    assert!(
        analysis
            .impacted_items
            .iter()
            .any(|item| item.name == "checkout"),
        "valid part of analysis must still work"
    );
}

/// §113：确定性——同输入连续两次分析 JSON 一致。
#[test]
fn analysis_is_deterministic() {
    let fixture = Fixture::new("determinism");
    fixture.write("lib.ts", LIB);
    fixture.write("checkout.ts", CHECKOUT);
    fixture.write("checkout.test.ts", TEST);
    fixture.commit_all("base");
    fixture.write("lib.ts", &LIB.replace("amount * 2", "amount * 5"));

    let first = fixture.analyze(&ImpactOptions::default());
    let second = fixture.analyze(&ImpactOptions::default());
    assert_eq!(first.id, second.id);
    assert_eq!(
        serde_json::to_string(&first).unwrap(),
        serde_json::to_string(&second).unwrap(),
        "business output must be byte-identical (§113)"
    );
}

/// §100：纯文件 rename（内容不变）不产生虚假的全量依赖影响。
#[test]
fn pure_file_rename_avoids_fake_impact() {
    let fixture = Fixture::new("rename");
    fixture.write("lib.ts", LIB);
    fixture.commit_all("base");
    fixture.rename("lib.ts", "renamed.ts");
    fixture.commit_all("rename");

    let analysis = fixture.analyze(&ImpactOptions::default());
    // 没有调用者/依赖者 ⇒ 不存在伪造的波及（§100）。
    assert!(
        analysis.impacted_items.is_empty(),
        "no fake dependency impact expected: {:?}",
        analysis.impacted_items
    );
    assert_eq!(analysis.analysis_completeness, ImpactCompleteness::Complete);
}

// Fixture 上的便捷写字节方法（避免上方占位行残留）。
impl Fixture {
    fn write_bytes(&self, relative: &str, content: &[u8]) {
        let path = self.root.join(relative);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, content).unwrap();
    }
}
