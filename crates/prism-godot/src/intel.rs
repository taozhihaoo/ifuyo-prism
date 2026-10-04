//! Godot IR → M21 IntelBundle（2.0-M24 §27-§31/§41/§56）。
//!
//! Godot 种类全部经 `EntityKind::other("godot.*")` 开放注册——Core 零 Godot 知识。
//! res:// 只存在于 Adapter 层（显示名/External 引用），Core 身份用项目相对路径。

use crate::adapter::{GodotAnalysis, GodotIndex};
use prism_csharp::SymbolIndex;
use prism_intel::entity::{Entity, EntityLocation, EntityScope};
use prism_intel::evidence::{Evidence, EvidenceKind};
use prism_intel::fact::{Fact, FactType, FactValue};
use prism_intel::identity::EntityKind;
use prism_intel::meta::{Confidence, ConfidenceReason, Freshness, Origin, Provenance};
use prism_intel::relation::{Relation, RelationTarget, RelationType};
use prism_intel::{IntelBundle, IntelOutcome};
use std::collections::BTreeMap;

/// Godot 生命周期方法名（§12：只断言"名字是生命周期名"这一 FACT）。
const LIFECYCLE_METHODS: [&str; 8] = [
    "_Ready",
    "_Process",
    "_PhysicsProcess",
    "_EnterTree",
    "_ExitTree",
    "_Input",
    "_UnhandledInput",
    "_Notification",
];

/// Godot 专用 RelationType（§27：有证据才建，名称语义分明）。
pub mod relations {
    use prism_intel::RelationType;
    pub fn loads_autoload() -> RelationType {
        RelationType::other("loads")
    }
    pub fn attached_script() -> RelationType {
        RelationType::other("attached_script")
    }
    pub fn backed_by_symbol() -> RelationType {
        RelationType::other("backed_by")
    }
    pub fn references_scene() -> RelationType {
        RelationType::other("references_scene")
    }
    pub fn references_resource() -> RelationType {
        RelationType::other("references_resource")
    }
    pub fn signal_connects() -> RelationType {
        RelationType::other("signal_connects")
    }
    pub fn hosts_scene() -> RelationType {
        RelationType::other("hosts_scene")
    }
}

/// 翻译为 M21 Core。
///
/// `csharp`：M23 符号索引（Script→Symbol 连接的 Source of Truth，不复制符号）。
/// 不可用时传 None——Script 实体照建，backed_by 关系缺席（诚实缺省）。
pub fn godot_to_intel(
    analysis: &GodotAnalysis,
    index: &GodotIndex,
    csharp: Option<&SymbolIndex>,
    identity: &prism_intel::ProjectIdentity,
    observed_at: &str,
) -> IntelOutcome<IntelBundle> {
    let mut warnings = Vec::new();
    let provenance = Provenance {
        producer: "prism-godot::godot_to_intel".to_string(),
        producer_version: "1".to_string(),
        observed_at: observed_at.to_string(),
    };
    let mut bundle = IntelBundle::new(identity.clone(), provenance.clone());
    let at = observed_at.to_string();
    let project_entity_id = bundle.entity_id(&EntityKind::other("godot.project"), "");

    // Godot 项目实体 + 检测事实。
    bundle.insert_entity(Entity::new(
        project_entity_id.clone(),
        EntityKind::other("godot.project"),
        analysis
            .project
            .as_ref()
            .and_then(|p| p.project_name.clone())
            .unwrap_or_else(|| "Godot Project".to_string()),
        EntityLocation::FileOnly {
            path: "project.godot".to_string(),
        },
        EntityScope::Internal,
    ));
    if let Some(project) = &analysis.project {
        let evidence = Evidence::new(
            EvidenceKind::Config,
            Some("project.godot".to_string()),
            None,
            None,
            "project.godot application section".to_string(),
        );
        let evidence_id = evidence.id.clone();
        bundle.insert_evidence(evidence);
        bundle.insert_fact(Fact::new(
            FactType::other("godot_project_detected"),
            Some(project_entity_id.clone()),
            FactValue::Structured(serde_json::json!({
                "name": project.project_name,
                "engine_version": project.engine_version_hint,
                // §33：无证据 ⇒ unknown，不猜测。
                "version_source": "project.godot config/features",
            })),
            Origin::Fact,
            Confidence::High,
            ConfidenceReason::direct_observation(),
            Freshness::Fresh,
            vec![evidence_id],
            at.clone(),
            provenance.clone(),
        ));
    }

    // 场景 + 节点（Scene Graph，§14-§16）。
    for (scene_path, document) in &index.scenes {
        let scene_id = bundle.entity_id(&EntityKind::other("godot.scene"), scene_path);
        bundle.insert_entity(Entity::new(
            scene_id.clone(),
            EntityKind::other("godot.scene"),
            scene_path
                .rsplit('/')
                .next()
                .unwrap_or(scene_path)
                .to_string(),
            EntityLocation::FileOnly {
                path: scene_path.clone(),
            },
            EntityScope::Internal,
        ));
        bundle.insert_relation(Relation::new(
            relations::hosts_scene(),
            project_entity_id.clone(),
            RelationTarget::Resolved {
                entity: scene_id.clone(),
            },
            Origin::Fact,
            Confidence::High,
            vec![],
        ));

        let root_name = document
            .nodes
            .first()
            .filter(|n| n.parent.is_none())
            .map(|n| n.name.clone())
            .unwrap_or_default();
        let mut path_ids: BTreeMap<String, prism_intel::EntityId> = BTreeMap::new();
        for node in &document.nodes {
            let node_path = if node.parent.is_none() {
                format!("/{root_name}")
            } else {
                prism_godot_normalize(&root_name, node.parent.as_deref().unwrap_or("."))
                    + "/"
                    + node.name.as_str()
            };
            let node_entity_id = bundle.entity_id(
                &EntityKind::other("godot.node"),
                &format!("{scene_path}|{node_path}"),
            );
            let mut entity = Entity::new(
                node_entity_id.clone(),
                EntityKind::other("godot.node"),
                node.name.clone(),
                EntityLocation::Virtual {
                    key: format!("res://{scene_path}{node_path}"),
                },
                EntityScope::Internal,
            );
            if let Some(node_type) = &node.node_type {
                entity
                    .metadata
                    .insert("godot_type".to_string(), node_type.clone());
            }
            bundle.insert_entity(entity);
            path_ids.insert(node_path.clone(), node_entity_id.clone());

            // 节点树关系：父 NodePath 的实体为父（root 的父 = 场景）。
            let parent_entity: Option<prism_intel::EntityId> = if node.parent.is_none() {
                Some(scene_id.clone())
            } else {
                let parent_path =
                    prism_godot_normalize(&root_name, node.parent.as_deref().unwrap_or("."));
                path_ids.get(&parent_path).cloned()
            };
            if let Some(parent_entity) = parent_entity {
                bundle.insert_relation(Relation::new(
                    RelationType::contains(),
                    parent_entity,
                    RelationTarget::Resolved {
                        entity: node_entity_id.clone(),
                    },
                    Origin::Fact,
                    Confidence::High,
                    vec![],
                ));
            }

            // Script 挂接（§19）：node → attached_script → script 实体。
            let script_relative = index.script_for_node(&node_path).cloned();
            if let Some(script_relative) = script_relative {
                let script_id = ensure_script(&mut bundle, &script_relative);
                bundle.insert_relation(Relation::new(
                    relations::attached_script(),
                    node_entity_id.clone(),
                    RelationTarget::Resolved {
                        entity: script_id.clone(),
                    },
                    Origin::Fact,
                    Confidence::High,
                    vec![],
                ));
                // Script → C# Symbol（§11/§29：路径对应 ⇒ INFERENCE，不复制符号）。
                if let Some(csharp) = csharp {
                    match backing_class(csharp, &script_relative) {
                        Some((qualified, suffix)) => {
                            // 与 prism-csharp::intel 同口径的实体 id（merge 后同链）。
                            let canonical = format!("{qualified}#{suffix}");
                            let symbol_entity_id =
                                bundle.entity_id(&EntityKind::symbol(), &canonical);
                            bundle.insert_relation(Relation::new(
                                relations::backed_by_symbol(),
                                script_id.clone(),
                                RelationTarget::Resolved {
                                    entity: symbol_entity_id.clone(),
                                },
                                Origin::Inference,
                                Confidence::High,
                                vec![],
                            ));
                            // 生命周期方法名（§12：名字是生命周期名 = FACT；
                            // 运行时调用顺序不在 M24 断言范围）。
                            let class_index_id = prism_csharp::symbol_id(
                                prism_csharp::SymbolKindCs::Class,
                                &qualified,
                                &[],
                            );
                            for method in csharp.children(&class_index_id) {
                                if method.kind == prism_csharp::SymbolKindCs::Method
                                    && LIFECYCLE_METHODS.contains(&method.name.as_str())
                                {
                                    let method_suffix =
                                        method.id.rsplit('#').next().unwrap_or_default();
                                    let method_canonical =
                                        format!("{}#{}", method.qualified_name, method_suffix);
                                    let method_entity =
                                        bundle.entity_id(&EntityKind::symbol(), &method_canonical);
                                    bundle.insert_fact(Fact::new(
                                        FactType::other("godot_lifecycle_method"),
                                        Some(method_entity),
                                        FactValue::Str(method.name.clone()),
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
                        }
                        None => {
                            warnings.push(format!(
                                "script {script_relative} has no matching C# class symbol (unresolved backing)"
                            ));
                        }
                    }
                }
            }
        }

        // Signal connections（§23-§24）。
        for connection in &document.connections {
            let from_path = resolve_scene_path(&root_name, &connection.from);
            let to_path = resolve_scene_path(&root_name, &connection.to);
            let from_entity = path_ids.get(&from_path).cloned();
            let to_entity = path_ids.get(&to_path).cloned();
            let evidence = Evidence::new(
                EvidenceKind::File,
                Some(scene_path.clone()),
                None,
                None,
                format!(
                    "[connection signal={:?} from={:?} to={:?} method={:?}]",
                    connection.signal, connection.from, connection.to, connection.method
                ),
            );
            let evidence_id = evidence.id.clone();
            bundle.insert_evidence(evidence);
            match (from_entity, to_entity) {
                (Some(from), Some(to)) => {
                    let mut relation = Relation::new(
                        relations::signal_connects(),
                        from,
                        RelationTarget::Resolved { entity: to },
                        Origin::Fact,
                        Confidence::High,
                        vec![evidence_id],
                    );
                    if let Some(method) = &connection.method {
                        relation
                            .metadata
                            .insert("signal".to_string(), connection.signal.clone());
                        relation
                            .metadata
                            .insert("method".to_string(), method.clone());
                    }
                    bundle.insert_relation(relation);
                }
                _ => {
                    // §24 未验证：保留 raw method 名，不伪造关系。
                    bundle.insert_fact(Fact::new(
                        FactType::other("godot_unresolved_connection"),
                        Some(project_entity_id.clone()),
                        FactValue::Structured(serde_json::json!({
                            "scene": scene_path,
                            "signal": connection.signal,
                            "from": connection.from,
                            "to": connection.to,
                            "method": connection.method,
                        })),
                        Origin::Fact,
                        Confidence::High,
                        ConfidenceReason::direct_observation(),
                        Freshness::Fresh,
                        vec![evidence_id],
                        at.clone(),
                        provenance.clone(),
                    ));
                }
            }
        }
    }

    // 资源（.tres，§21）。
    for resource_path in index.resources.keys() {
        let id = bundle.entity_id(&EntityKind::other("godot.resource"), resource_path);
        bundle.insert_entity(Entity::new(
            id,
            EntityKind::other("godot.resource"),
            resource_path
                .rsplit('/')
                .next()
                .unwrap_or(resource_path)
                .to_string(),
            EntityLocation::FileOnly {
                path: resource_path.clone(),
            },
            EntityScope::Internal,
        ));
    }

    // Autoload（§9：一等 Godot Fact + LOADS 关系）。
    if let Some(project) = &analysis.project {
        for autoload in &project.autoloads {
            let autoload_id =
                bundle.entity_id(&EntityKind::other("godot.autoload"), &autoload.name);
            bundle.insert_entity(Entity::new(
                autoload_id.clone(),
                EntityKind::other("godot.autoload"),
                autoload.name.clone(),
                EntityLocation::FileOnly {
                    path: autoload.project_relative_path.clone(),
                },
                EntityScope::Internal,
            ));
            bundle.insert_relation(Relation::new(
                RelationType::other("has_autoload"),
                project_entity_id.clone(),
                RelationTarget::Resolved {
                    entity: autoload_id.clone(),
                },
                Origin::Fact,
                Confidence::High,
                vec![],
            ));
            let script_id = ensure_script(&mut bundle, &autoload.project_relative_path);
            bundle.insert_relation(Relation::new(
                relations::loads_autoload(),
                autoload_id,
                RelationTarget::Resolved { entity: script_id },
                Origin::Fact,
                Confidence::High,
                vec![],
            ));
        }
    }

    // 诊断进 warnings（§78）。
    warnings.extend(index.diagnostics.clone());
    if warnings.is_empty() {
        IntelOutcome::complete(bundle)
    } else {
        prism_intel::IntelOutcome {
            status: prism_intel::IntelStatus::Complete,
            data: Some(bundle),
            warnings,
            errors: Vec::new(),
        }
    }
}

fn ensure_script(bundle: &mut IntelBundle, script_relative: &str) -> prism_intel::EntityId {
    let script_id = bundle.entity_id(&EntityKind::other("godot.script"), script_relative);
    bundle.insert_entity(Entity::new(
        script_id.clone(),
        EntityKind::other("godot.script"),
        script_relative
            .rsplit('/')
            .next()
            .unwrap_or(script_relative)
            .to_string(),
        EntityLocation::FileOnly {
            path: script_relative.to_string(),
        },
        EntityScope::Internal,
    ));
    script_id
}

/// 脚本文件的宿主 class（M23 索引直读——不复制符号语义，只取定位字段）。
fn backing_class(index: &SymbolIndex, script_relative: &str) -> Option<(String, String)> {
    index
        .by_file(script_relative)
        .into_iter()
        .find(|s| s.kind == prism_csharp::SymbolKindCs::Class)
        .map(|symbol| {
            let suffix = symbol.id.rsplit('#').next().unwrap_or_default().to_string();
            (symbol.qualified_name.clone(), suffix)
        })
}

fn prism_godot_normalize(root: &str, relative: &str) -> String {
    crate::adapter::normalize_node_path(&format!("/{root}"), relative)
}

/// connection 的 from/to 归一化（"." = root；"UI" = root 下子路径）。
fn resolve_scene_path(root_name: &str, raw: &str) -> String {
    crate::adapter::normalize_node_path(&format!("/{root_name}"), raw)
}
