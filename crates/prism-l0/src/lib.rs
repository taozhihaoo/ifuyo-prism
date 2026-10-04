//! # Prism L0 Structural Analysis（2.0-M22）
//!
//! Core 的第一个主要 Fact Producer（§102）：把既有 M1 发现扫描 / M4 Git
//! / manifest 内容观察翻译为 M21 IntelBundle——filesystem、git 状态、
//! manifest 声明、配置存在性、平台常量。
//!
//! 边界：
//! - L0 = 结构/环境观察；不做 AST/symbol/语义（M23/M25）/Godot 图（M24）；
//! - declared dependency ≠ resolved dependency（§27）；
//! - GitIgnored ≠ PrismExcluded（§38：当前 M1 IgnoreEngine 合并默认规则与
//!   .gitignore，忽略数进统计事实；逐文件双维标注随需要扩展）；
//! - 单文件/单 manifest 失败 ⇒ 诊断 + 继续（§56）；根不可用 ⇒ Err；
//! - 不读全部文件内容（§59）：只读小体积 manifest（上限可配）；
//! - 无 shell/网络/遥测；不读配置内容（§30 secret safety——配置只记录存在）。
//!
//! 增量：每运行为确定性全量再观察（亚秒级），`delta::diff` 提供两次观察
//! 的正确差分（§52 规则的验证底座）；持久增量缓存待测量证据再建（§51）。

pub mod delta;
pub mod manifests;
pub mod options;
pub mod structural;

pub use delta::{diff, L0Delta};
pub use options::L0Options;
pub use structural::structural_analysis;

/// L0 输出契约版本（与 INTEL_SCHEMA_VERSION 独立演化，M21 §61）。
pub const L0_SCHEMA_VERSION: u32 = 1;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::options::L0Options;
    use prism_core::task::CancellationToken;
    use prism_core::Severity;
    use prism_intel::evidence::EvidenceKind;
    use prism_intel::fact::{FactType, FactValue};
    use prism_intel::relation::RelationType;
    use prism_intel::{AnalysisContext, AnalysisScope, IntelStatus, ProjectIdentity};
    use std::path::Path;

    fn ctx_for(root: &Path) -> AnalysisContext {
        let project = prism_project::Project::from_root(root).unwrap();
        AnalysisContext::new(
            ProjectIdentity::from_project(&project),
            AnalysisScope::FullProject,
            vec!["l0_structural".into()],
            CancellationToken::new(),
        )
    }

    fn make_project(name: &str, layout: fn(&Path)) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "prism-l0-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.subsec_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        layout(&dir);
        dir
    }

    fn minimal(dir: &Path) {
        std::fs::write(dir.join("README.md"), "# demo\n").unwrap();
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(dir.join("src/main.py"), "print(1)\n").unwrap();
    }

    #[test]
    fn inventory_entities_relations_and_stats_are_correct() {
        let dir = make_project("basic", minimal);
        let outcome = structural_analysis(&dir, &ctx_for(&dir), &L0Options::default());
        assert_eq!(outcome.status, IntelStatus::Complete);
        let bundle = outcome.data.unwrap();

        // 1 project + 1 dir + 2 files；contains：project→README、project→src、src→main.py。
        assert_eq!(bundle.entities.len(), 4);
        let contains = bundle
            .relations
            .values()
            .filter(|r| r.relation_type == RelationType::contains())
            .count();
        assert_eq!(contains, 3);
        // 统计事实。
        let files_total = bundle
            .facts
            .values()
            .find(|f| f.fact_type == FactType::modules_total())
            .unwrap();
        assert_eq!(files_total.value, FactValue::Int(2));
        // 语言元数据标注 basis=extension（§20）。
        let main_file = bundle
            .entities
            .values()
            .find(|e| e.display_name == "main.py")
            .unwrap();
        assert_eq!(
            main_file.metadata.get("language_basis").map(String::as_str),
            Some("extension")
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn determinism_same_state_same_output() {
        let dir = make_project("det", minimal);
        // 同一项目身份（runtime id 共享）；仅观察时间不同。
        let project = prism_project::Project::from_root(&dir).unwrap();
        let identity = ProjectIdentity::from_project(&project);
        let make_ctx = || {
            AnalysisContext::new(
                identity.clone(),
                AnalysisScope::FullProject,
                vec!["l0_structural".into()],
                CancellationToken::new(),
            )
        };
        let a = structural_analysis(&dir, &make_ctx(), &L0Options::default());
        let b = structural_analysis(&dir, &make_ctx(), &L0Options::default());
        let strip = |outcome: &prism_intel::IntelOutcome<prism_intel::IntelBundle>| {
            let text = serde_json::to_string(&outcome).unwrap();
            // 各自剥离自己的两处观察时间（身份构造 + provenance；时间不是
            // 结构事实——M21 §30 determinism 指"同状态 ⇒ 同结构输出"）。
            let bundle = outcome.data.as_ref().unwrap();
            text.replace(&bundle.project.observed_at, "")
                .replace(&bundle.provenance.observed_at, "")
        };
        let (ta, tb) = (strip(&a), strip(&b));
        let divergence = ta
            .chars()
            .zip(tb.chars())
            .position(|(x, y)| x != y)
            .unwrap_or(0);
        assert_eq!(
            ta,
            tb,
            "same project state must be deterministic; divergence at {}
A: {}
B: {}",
            divergence,
            &ta[divergence.saturating_sub(60)..(divergence + 120).min(ta.len())],
            &tb[divergence.saturating_sub(60)..(divergence + 120).min(tb.len())],
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn git_facts_carry_revision_evidence() {
        let dir = make_project("git", |dir| {
            minimal(dir);
            let run = |args: &[&str]| {
                std::process::Command::new("git")
                    .args(args)
                    .current_dir(dir)
                    .output()
                    .expect("git available")
            };
            run(&["init", "-b", "main"]);
            run(&["config", "user.name", "Prism Test"]);
            run(&["config", "user.email", "t@prism.local"]);
            run(&["add", "-A"]);
            run(&["commit", "-m", "base"]);
        });
        let outcome = structural_analysis(&dir, &ctx_for(&dir), &L0Options::default());
        let bundle = outcome.data.unwrap();
        let branch = bundle
            .facts
            .values()
            .find(|f| f.fact_type.as_str() == "git_branch")
            .expect("git branch fact");
        assert_eq!(branch.value, FactValue::Str("main".into()));
        assert_eq!(branch.origin, prism_intel::Origin::Fact);
        // 证据可追溯且引用 revision（§24）。
        let evidence_id = &branch.evidence[0];
        let evidence = bundle.evidence.get(evidence_id).unwrap();
        assert!(matches!(evidence.kind, EvidenceKind::Git));
        assert_ne!(evidence.reference.as_deref(), Some("no-head"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn manifest_dependency_extraction_with_failure_isolation() {
        let dir = make_project("manifests", |dir| {
            minimal(dir);
            std::fs::write(
                dir.join("package.json"),
                r#"{"dependencies": {"left-pad": "^1.0"}, "devDependencies": {"vitest": "^2.0"}}"#,
            )
            .unwrap();
            std::fs::create_dir_all(dir.join("sub")).unwrap();
            std::fs::write(dir.join("sub/package.json"), "{ not json").unwrap();
            std::fs::write(dir.join("requirements.txt"), "requests>=2.0\n").unwrap();
        });
        let outcome = structural_analysis(&dir, &ctx_for(&dir), &L0Options::default());
        // 失败隔离：坏 manifest 是诊断，不拖垮扫描（§56）。
        assert_eq!(outcome.status, IntelStatus::Complete);
        assert!(outcome
            .warnings
            .iter()
            .any(|w| w.contains("sub/package.json")));
        let bundle = outcome.data.unwrap();

        let declared: Vec<_> = bundle
            .facts
            .values()
            .filter(|f| f.fact_type == FactType::dependency_declared())
            .collect();
        assert_eq!(declared.len(), 3, "package.json 2 + requirements.txt 1");
        // declared ≠ resolved：relation 指向 External 包实体（§27）。
        let external = bundle
            .entities
            .values()
            .filter(|e| e.scope == prism_intel::EntityScope::External)
            .count();
        assert!(external >= 3);
        // lock 文件缺席 ⇒ 无 lock 事实。
        assert!(!bundle
            .facts
            .values()
            .any(|f| f.fact_type.as_str() == "lock_file_present"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn non_git_project_records_absence_honestly() {
        let dir = make_project("nogit", minimal);
        let bundle = structural_analysis(&dir, &ctx_for(&dir), &L0Options::default())
            .data
            .unwrap();
        let present = bundle
            .facts
            .values()
            .find(|f| f.fact_type.as_str() == "git_repository_present")
            .expect("presence fact recorded either way");
        assert_eq!(present.value, FactValue::Bool(false));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn cancellation_returns_cancelled_not_complete() {
        let dir = make_project("cancel", minimal);
        let context = ctx_for(&dir);
        context.cancel.cancel();
        let outcome = structural_analysis(&dir, &context, &L0Options::default());
        assert_eq!(outcome.status, IntelStatus::Cancelled);
        assert!(outcome.data.is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn entity_cap_produces_partial_with_warning() {
        let dir = make_project("cap", minimal);
        let options = L0Options {
            max_entities: 2,
            ..L0Options::default()
        };
        let outcome = structural_analysis(&dir, &ctx_for(&dir), &options);
        assert_eq!(
            outcome.status,
            IntelStatus::Partial,
            "truncation must not fake completeness"
        );
        assert!(outcome.warnings.iter().any(|w| w.contains("truncated")));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn unicode_and_space_paths_survive() {
        let dir = make_project("unicode", |dir| {
            let nested = dir.join("docs 中文 (draft)");
            std::fs::create_dir_all(&nested).unwrap();
            std::fs::write(nested.join("说明 文档.md"), "内容\n").unwrap();
        });
        let outcome = structural_analysis(&dir, &ctx_for(&dir), &L0Options::default());
        let bundle = outcome.data.unwrap();
        assert!(
            bundle
                .entities
                .values()
                .any(|e| e.display_name == "说明 文档.md"),
            "unicode paths must inventory correctly"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn ignored_directory_stays_out_of_inventory() {
        let dir = make_project("ignored", |dir| {
            minimal(dir);
            std::fs::create_dir_all(dir.join("node_modules")).unwrap();
            std::fs::write(dir.join("node_modules/pkg.js"), "x").unwrap();
        });
        let bundle = structural_analysis(&dir, &ctx_for(&dir), &L0Options::default())
            .data
            .unwrap();
        assert!(
            !bundle
                .entities
                .values()
                .any(|e| e.display_name == "node_modules"),
            "default-excluded dirs must not become entities"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn structural_findings_stay_out_of_l0() {
        // §41/§79：L0 不产出评价性结论——bad design 类判断不在这里。
        let dir = make_project("nofinding", minimal);
        let bundle = structural_analysis(&dir, &ctx_for(&dir), &L0Options::default())
            .data
            .unwrap();
        for finding in bundle.findings.values() {
            assert_ne!(finding.severity, Severity::Critical);
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn core_contract_alignment_round_trip() {
        // §89：L0 产物满足 M21 契约（round-trip + 集合确定性）。
        let dir = make_project("contract", minimal);
        let bundle = structural_analysis(&dir, &ctx_for(&dir), &L0Options::default())
            .data
            .unwrap();
        let text = serde_json::to_string(&bundle).unwrap();
        let back: prism_intel::IntelBundle = serde_json::from_str(&text).unwrap();
        assert_eq!(back, bundle);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
