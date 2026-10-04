//! L0 结构分析主产出（2.0-M22 §82 垂直切片）。
//!
//! 管线（§84 迁移不重写）：既有 M1 DiscoveryService / M4 GitService
//! → 翻译为 M21 IntelBundle（Entity/Fact/Relation/Evidence/Provenance/Freshness）。
//! 本 crate 是 Core 的第一个主要 Fact Producer（§102），不重算、不另立真相源（§85）。

use crate::manifests;
use crate::options::L0Options;
use prism_core::PrismError;
use prism_git::GitService;
use prism_intel::entity::{Entity, EntityLocation, EntityScope};
use prism_intel::evidence::{Evidence, EvidenceKind};
use prism_intel::fact::{Fact, FactType, FactValue};
use prism_intel::identity::EntityKind;
use prism_intel::meta::{Confidence, ConfidenceReason, Freshness, Origin, Provenance};
use prism_intel::relation::{Relation, RelationTarget, RelationType};
use prism_intel::{AnalysisContext, IntelBundle, IntelOutcome, IntelStatus};
use prism_project::DiscoveryService;
use std::path::Path;
use std::sync::Arc;

/// 一次 L0 结构观察：真实项目 → Core 集合。
///
/// 失败隔离（§56）：单文件/单 manifest/单域失败 ⇒ 诊断 + 继续；
/// 仅项目根本身不可用 ⇒ Err。
pub fn structural_analysis(
    project_root: &Path,
    ctx: &AnalysisContext,
    options: &L0Options,
) -> IntelOutcome<IntelBundle> {
    if ctx.cancel.is_cancelled() {
        return IntelOutcome::cancelled(None, vec!["cancelled before l0 scan".into()]);
    }

    let mut warnings: Vec<String> = Vec::new();
    let provenance = Provenance {
        producer: "prism-l0::structural_analysis".to_string(),
        producer_version: prism_intel::INTEL_SCHEMA_VERSION.to_string(),
        observed_at: ctx.observed_at.clone(),
    };
    let mut bundle = IntelBundle::new(ctx.project.clone(), provenance.clone());
    let at = ctx.observed_at.clone();

    // ---- M1 发现扫描（既有 scanner；不跟随 symlink、默认忽略规则，§十一/§37）----
    let discovery = {
        let runner: Arc<dyn prism_core::runtime::command::CommandRunner> =
            Arc::new(prism_core::runtime::command::StdCommandRunner::new());
        let _ = runner;
        DiscoveryService::new().discover(
            project_root,
            prism_project::ScanOptions::default(),
            &ctx.cancel,
            |_| {},
        )
    };
    let discovery = match discovery {
        Ok(discovery) => discovery,
        Err(err) => return IntelOutcome::failed(vec![format!("l0 discovery failed: {err}")]),
    };

    // ---- 项目实体 ----
    let project_entity_id = bundle.entity_id(&EntityKind::project(), "");
    bundle.insert_entity(Entity::new(
        project_entity_id.clone(),
        EntityKind::project(),
        ctx.project.display_name.clone(),
        EntityLocation::FileOnly {
            path: String::new(),
        },
        EntityScope::Internal,
    ));

    // ---- 目录 + 文件实体与 contains 关系（§14/§15/§40；确定性：输入已按路径排序）----
    let mut truncated = false;
    let mut dir_ids: Vec<(String, prism_intel::EntityId)> = Vec::new();
    for directory in &discovery.directories {
        // 根目录由项目实体代表，不重复建模。
        if directory.relative_path.is_empty() {
            continue;
        }
        if bundle.entities.len() >= options.max_entities {
            truncated = true;
            break;
        }
        let id = bundle.entity_id(&EntityKind::directory(), &directory.relative_path);
        bundle.insert_entity(Entity::new(
            id.clone(),
            EntityKind::directory(),
            directory.name.clone(),
            EntityLocation::FileOnly {
                path: directory.relative_path.clone(),
            },
            EntityScope::Internal,
        ));
        dir_ids.push((directory.relative_path.clone(), id));
    }
    let mut file_ids: Vec<(String, prism_intel::EntityId)> = Vec::new();
    for file in &discovery.files {
        if bundle.entities.len() >= options.max_entities {
            truncated = true;
            break;
        }
        let id = bundle.entity_id(&EntityKind::file(), &file.relative_path);
        let mut entity = Entity::new(
            id.clone(),
            EntityKind::file(),
            file.file_name.clone(),
            EntityLocation::FileOnly {
                path: file.relative_path.clone(),
            },
            EntityScope::Internal,
        );
        entity
            .metadata
            .insert("size_bytes".to_string(), file.size_bytes.to_string());
        if let Some(language) = file.language {
            // §20：语言识别声明依据（当前为扩展名推断，非语法分析）。
            entity.metadata.insert(
                "language".to_string(),
                format!("{language:?}").to_lowercase(),
            );
            entity
                .metadata
                .insert("language_basis".to_string(), "extension".to_string());
        }
        entity.metadata.insert(
            "classification".to_string(),
            format!("{:?}", file.classification.kind).to_lowercase(),
        );
        if let Some(reason) = &file.non_analyzable_reason {
            entity
                .metadata
                .insert("non_analyzable_reason".to_string(), reason.clone());
        }
        bundle.insert_entity(entity);
        file_ids.push((file.relative_path.clone(), id));
    }
    if truncated {
        warnings.push(format!(
            "entity cap {} reached; inventory truncated (Partial)",
            options.max_entities
        ));
    }

    // contains 关系：project → 顶层目录/文件；目录 → 子目录/文件（§40）。
    for (path, id) in &dir_ids {
        let (parent, _) = split_parent(path);
        let source = if parent.is_empty() {
            project_entity_id.clone()
        } else {
            match dir_ids.iter().find(|(p, _)| *p == parent) {
                Some((_, pid)) => pid.clone(),
                None => continue,
            }
        };
        bundle.insert_relation(Relation::new(
            RelationType::contains(),
            source,
            RelationTarget::Resolved { entity: id.clone() },
            Origin::Fact,
            Confidence::High,
            vec![],
        ));
    }
    for (path, id) in &file_ids {
        let (parent, _) = split_parent(path);
        let source = if parent.is_empty() {
            project_entity_id.clone()
        } else {
            match dir_ids.iter().find(|(p, _)| *p == parent) {
                Some((_, pid)) => pid.clone(),
                None => continue,
            }
        };
        bundle.insert_relation(Relation::new(
            RelationType::contains(),
            source,
            RelationTarget::Resolved { entity: id.clone() },
            Origin::Fact,
            Confidence::High,
            vec![],
        ));
    }

    if ctx.cancel.is_cancelled() {
        return IntelOutcome::cancelled(Some(bundle), vec!["cancelled after inventory".into()]);
    }

    // ---- 聚合统计事实（§41 合格 FACT 形态）----
    let stats = &discovery.statistics;
    for (fact_type, value) in [
        (FactType::modules_total(), stats.total_files as i64),
        (
            FactType::other("directories_total"),
            stats.total_directories as i64,
        ),
        (
            FactType::other("files_ignored"),
            discovery.scan.files_ignored as i64,
        ),
        (
            FactType::other("total_size_bytes"),
            stats.total_size_bytes as i64,
        ),
    ] {
        bundle.insert_fact(Fact::new(
            fact_type,
            Some(project_entity_id.clone()),
            FactValue::Int(value),
            Origin::Fact,
            Confidence::High,
            ConfidenceReason::direct_observation(),
            Freshness::Fresh,
            vec![],
            at.clone(),
            provenance.clone(),
        ));
    }

    // ---- 项目类型推断（§42/§43：INFERENCE + 证据 + 置信度，非事实）----
    for candidate in discovery.project_types.iter().take(5) {
        let mut evidence_ids = Vec::new();
        for item in &candidate.evidence {
            let evidence = Evidence::new(
                EvidenceKind::Manifest,
                Some(item.path.clone()),
                None,
                Some(item.source.clone()),
                item.reason.clone(),
            );
            evidence_ids.push(evidence.id.clone());
            bundle.insert_evidence(evidence);
        }
        bundle.insert_fact(Fact::new(
            FactType::other("project_type_detected"),
            Some(project_entity_id.clone()),
            FactValue::Structured(serde_json::json!({
                "name": candidate.name,
                "confidence": format!("{:?}", candidate.confidence).to_lowercase(),
            })),
            Origin::Inference,
            Confidence::from_core(candidate.confidence),
            ConfidenceReason::multiple_compatible_facts(),
            Freshness::Fresh,
            evidence_ids,
            at.clone(),
            provenance.clone(),
        ));
    }

    // ---- 配置（§29：只记录存在与关系，不读内容——§30 secret safety）----
    // §85 单一真相：路径已被 file 实体覆盖时复用该实体，不建第二实体。
    for configuration in &discovery.configurations {
        let file_id = bundle.entity_id(&EntityKind::file(), &configuration.path);
        let target = if bundle.entities.contains_key(&file_id) {
            file_id
        } else {
            let id = bundle.entity_id(&EntityKind::configuration(), &configuration.path);
            bundle.insert_entity(Entity::new(
                id.clone(),
                EntityKind::configuration(),
                configuration.name.clone(),
                EntityLocation::FileOnly {
                    path: configuration.path.clone(),
                },
                EntityScope::Internal,
            ));
            id
        };
        bundle.insert_relation(Relation::new(
            RelationType::other("has_configuration"),
            project_entity_id.clone(),
            RelationTarget::Resolved { entity: target },
            Origin::Fact,
            Confidence::High,
            vec![],
        ));
    }

    // ---- Git 状态（§22/§24/§25：证据携带 observed revision）----
    match collect_git(
        &mut bundle,
        &project_entity_id,
        project_root,
        ctx,
        options,
        &at,
        &provenance,
    ) {
        Ok(git_warnings) => warnings.extend(git_warnings),
        Err(err) => warnings.push(format!("git state unavailable: {err}")),
    }

    if ctx.cancel.is_cancelled() {
        return IntelOutcome::cancelled(Some(bundle), vec!["cancelled after git".into()]);
    }

    // ---- Manifest 检测 + 依赖声明（§26-§28/§76/§77）----
    warnings.extend(manifests::collect_manifests(
        &mut bundle,
        &project_entity_id,
        &discovery.files,
        options,
        &at,
        &provenance,
    ));

    // ---- 平台常量（§31/§32：零探测、工程相关；工具探测归 prism-env/M9→M29）----
    if options.include_platform_facts {
        for (fact_type, value) in [
            (
                FactType::other("platform_os"),
                std::env::consts::OS.to_string(),
            ),
            (
                FactType::other("platform_arch"),
                std::env::consts::ARCH.to_string(),
            ),
        ] {
            bundle.insert_fact(Fact::new(
                fact_type,
                Some(project_entity_id.clone()),
                FactValue::Str(value),
                Origin::Fact,
                Confidence::High,
                ConfidenceReason::direct_observation(),
                Freshness::Fresh,
                vec![],
                at.clone(),
                provenance.clone(),
            ));
        }
    }

    // ---- 扫描非致命错误 → 诊断（§78）----
    const MAX_SCAN_ERROR_WARNINGS: usize = 20;
    for scan_error in discovery.errors.iter().take(MAX_SCAN_ERROR_WARNINGS) {
        warnings.push(format!("scan diagnostic: {scan_error:?}"));
    }
    if discovery.errors.len() > MAX_SCAN_ERROR_WARNINGS {
        warnings.push(format!(
            "…and {} more scan diagnostics",
            discovery.errors.len() - MAX_SCAN_ERROR_WARNINGS
        ));
    }
    if truncated {
        warnings.push("scan truncated by entity cap".to_string());
    }

    if truncated {
        // 截断的清单不得伪装成完整（§55）。
        IntelOutcome::partial(bundle, warnings)
    } else if warnings.is_empty() {
        IntelOutcome::complete(bundle)
    } else {
        // 诊断属项目问题（M21 §28 边界）；分析已完整完成 ⇒ Complete + warnings。
        IntelOutcome {
            status: IntelStatus::Complete,
            data: Some(bundle),
            warnings,
            errors: Vec::new(),
        }
    }
}

/// (parent_path, file_name) —— `/` 分隔的项目相对路径。
fn split_parent(path: &str) -> (String, &str) {
    match path.rsplit_once('/') {
        Some((parent, name)) => (parent.to_string(), name),
        None => (String::new(), path),
    }
}

/// Git 状态 → 实体 + 事实（失败返回诊断而非致命，§56）。
fn collect_git(
    bundle: &mut IntelBundle,
    project_entity_id: &prism_intel::EntityId,
    project_root: &Path,
    ctx: &AnalysisContext,
    options: &L0Options,
    at: &str,
    provenance: &Provenance,
) -> Result<Vec<String>, PrismError> {
    let mut warnings = Vec::new();
    let git = GitService::new(Arc::new(
        prism_core::runtime::command::StdCommandRunner::new(),
    ));
    let status = git.status(project_root, false, &ctx.cancel)?;

    let Some(repository) = status.repository else {
        bundle.insert_fact(Fact::new(
            FactType::other("git_repository_present"),
            Some(project_entity_id.clone()),
            FactValue::Bool(false),
            Origin::Fact,
            Confidence::High,
            ConfidenceReason::direct_observation(),
            Freshness::Fresh,
            vec![],
            at,
            provenance.clone(),
        ));
        return Ok(warnings);
    };

    // 仓库实体（location 用 Virtual：gitdir 可能不在项目树下，worktree 场景尤甚）。
    let repo_id = bundle.entity_id(&EntityKind::repository(), &repository.git_dir);
    bundle.insert_entity(Entity::new(
        repo_id.clone(),
        EntityKind::repository(),
        "git",
        EntityLocation::Virtual {
            key: repository.git_dir.clone(),
        },
        EntityScope::Internal,
    ));
    bundle.insert_relation(Relation::new(
        RelationType::other("hosts_project"),
        repo_id.clone(),
        RelationTarget::Resolved {
            entity: project_entity_id.clone(),
        },
        Origin::Fact,
        Confidence::High,
        vec![],
    ));
    bundle.insert_fact(Fact::new(
        FactType::other("git_repository_present"),
        Some(project_entity_id.clone()),
        FactValue::Bool(true),
        Origin::Fact,
        Confidence::High,
        ConfidenceReason::direct_observation(),
        Freshness::Fresh,
        vec![],
        at,
        provenance.clone(),
    ));

    // 证据引用 observed revision（§24：Git 数据必须有 Evidence）。
    let observed_revision = status
        .head
        .as_ref()
        .and_then(|head| head.commit_id.clone())
        .unwrap_or_else(|| "no-head".to_string());
    let git_evidence = Evidence::new(
        EvidenceKind::Git,
        None,
        None,
        Some(observed_revision.clone()),
        "git status --porcelain at observed revision".to_string(),
    );
    bundle.insert_evidence(git_evidence.clone());

    if let Some(head) = &status.head {
        if let Some(branch) = &head.branch_name {
            bundle.insert_fact(Fact::new(
                FactType::other("git_branch"),
                Some(project_entity_id.clone()),
                FactValue::Str(branch.clone()),
                Origin::Fact,
                Confidence::High,
                ConfidenceReason::direct_observation(),
                Freshness::Fresh,
                vec![git_evidence.id.clone()],
                at,
                provenance.clone(),
            ));
        }
        if let Some(commit) = &head.commit_id {
            bundle.insert_fact(Fact::new(
                FactType::other("git_head_commit"),
                Some(project_entity_id.clone()),
                FactValue::Str(commit.clone()),
                Origin::Fact,
                Confidence::High,
                ConfidenceReason::direct_observation(),
                Freshness::Fresh,
                vec![git_evidence.id.clone()],
                at,
                provenance.clone(),
            ));
        }
    }

    let dirty = status.entries.len();
    bundle.insert_fact(Fact::new(
        FactType::other("git_working_tree_changed_files"),
        Some(project_entity_id.clone()),
        FactValue::Int(dirty as i64),
        Origin::Fact,
        Confidence::High,
        ConfidenceReason::direct_observation(),
        Freshness::Fresh,
        vec![git_evidence.id.clone()],
        at,
        provenance.clone(),
    ));

    // 逐文件状态事实（cap；rename 证据显式保留 original_path，§16）。
    for entry in status.entries.iter().take(options.max_git_status_facts) {
        let file_id = bundle.entity_id(&EntityKind::file(), &entry.path);
        bundle.insert_fact(Fact::new(
            FactType::other("git_file_status"),
            Some(file_id),
            FactValue::Structured(serde_json::json!({
                "index": format!("{:?}", entry.index_state).to_lowercase(),
                "worktree": format!("{:?}", entry.working_tree_state).to_lowercase(),
                "untracked": entry.is_untracked,
                "renamed_from": entry.original_path,
            })),
            Origin::Fact,
            Confidence::High,
            ConfidenceReason::direct_observation(),
            // 工作树随时可变（§25）：观察即刻新鲜，易 stale。
            Freshness::Fresh,
            vec![git_evidence.id.clone()],
            at,
            provenance.clone(),
        ));
    }
    if status.entries.len() > options.max_git_status_facts {
        warnings.push(format!(
            "git status facts capped at {} ({} changed files)",
            options.max_git_status_facts,
            status.entries.len()
        ));
    }
    Ok(warnings)
}
