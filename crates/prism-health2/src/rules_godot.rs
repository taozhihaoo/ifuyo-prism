//! Godot 规则（2.0-M28 §D Scene/Runtime Wiring Health）。
//!
//! 输入 = 注入的 M24 `GodotAnalysis`（IR 层事实）；规则只做引用一致性
//! 检查（ext_resource 悬挂 / 场景坏行），不重新解析场景文件（§2）。

use crate::model::Metric;
use crate::registry::{RuleContext, RuleOutcome};
use prism_intel::evidence::{Evidence, EvidenceKind};
use prism_intel::finding::{Finding, FindingStatus};
use prism_intel::identity::{EntityId, EntityKind};
use prism_intel::{Confidence, Origin};

fn metric(key: &str, value: u64, definition: &str) -> Metric {
    Metric {
        key: key.to_string(),
        value,
        unit: "count".to_string(),
        definition: definition.to_string(),
        producer: "prism-health2::rules_godot".to_string(),
    }
}

fn node_path(node: &prism_godot::NodeRecord) -> String {
    match &node.parent {
        Some(parent) if parent != "." => format!("{parent}/{}", node.name),
        _ => node.name.clone(),
    }
}

/// Godot 规则（§D）：每场景引用一致性——
/// - `GODOT/MISSING_SCRIPT`：node.script 引用的 ext_resource id 不存在
///   （运行时脚本为 null 的接线断裂，Warning）；
/// - `GODOT/MISSING_INSTANCE`：instance 引用的 ext_resource id 不存在
///   （子场景缺失，Warning）；
/// - `GODOT/MALFORMED_SCENE`：场景存在坏行（Info，数据如实降级）。
pub fn scene_wiring_rule(ctx: &RuleContext) -> RuleOutcome {
    let mut outcome = RuleOutcome::default();
    let Some(analysis) = ctx.godot else {
        outcome
            .diagnostics
            .push("no Godot analysis injected; rule skipped".to_string());
        return outcome;
    };
    let mut scenes_total = 0u64;
    let mut nodes_total = 0u64;
    for (scene_path, document) in &analysis.scenes {
        scenes_total += 1;
        nodes_total += document.nodes.len() as u64;
        if ctx.cancel.is_cancelled() {
            outcome.diagnostics.push("cancelled mid-rule".to_string());
            break;
        }
        let scene_entity = || EntityId::new("health2", &EntityKind::file(), scene_path);
        if !document.well_formed {
            let evidence = Evidence::new(
                EvidenceKind::File,
                Some(scene_path.clone()),
                None,
                None,
                "scene file contains unparseable lines (M24 parser)",
            );
            let finding = Finding::new(
                "GODOT/MALFORMED_SCENE",
                prism_core::Severity::Info,
                FindingStatus::Open,
                format!("Malformed scene file {scene_path}"),
                "M24 parser reported unparseable lines; facts are partial".to_string(),
                Origin::Fact,
                Confidence::High,
                Some(scene_entity()),
                vec![evidence.id.clone()],
            );
            outcome.evidence.push(evidence);
            outcome.findings.push(finding);
        }
        for node in &document.nodes {
            let mut check = |finding_type: &'static str, label: &str, id: &str| {
                let evidence = Evidence::new(
                    EvidenceKind::File,
                    Some(scene_path.clone()),
                    None,
                    None,
                    format!(
                        "node `{}` references missing ext_resource `{id}`",
                        node.name
                    ),
                );
                let finding = Finding::new(
                    finding_type,
                    prism_core::Severity::Warning,
                    FindingStatus::Open,
                    format!("Node '{0}' {label} ext_resource '{1}' not found", node_path(node), id),
                    format!(
                        "scene {scene_path} node `{}` references ext_resource id `{id}` that is not declared",
                        node.name
                    ),
                    Origin::Fact,
                    Confidence::High,
                    Some(scene_entity()),
                    vec![evidence.id.clone()],
                );
                outcome.evidence.push(evidence);
                outcome.findings.push(finding);
            };
            if let Some(script_id) = &node.script_ext_resource_id {
                if !document.ext_resources.iter().any(|e| &e.id == script_id) {
                    check("GODOT/MISSING_SCRIPT", "script", script_id);
                }
            }
            if let Some(instance_id) = &node.instance_ext_resource_id {
                if !document.ext_resources.iter().any(|e| &e.id == instance_id) {
                    check("GODOT/MISSING_INSTANCE", "instance", instance_id);
                }
            }
        }
    }
    outcome.metrics.push(metric(
        "godot_scenes_total",
        scenes_total,
        "scene documents in injected Godot analysis",
    ));
    outcome.metrics.push(metric(
        "godot_nodes_total",
        nodes_total,
        "scene nodes across injected Godot analysis",
    ));
    outcome
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::RuleContext;
    use crate::rules::SourceFileInput;
    use crate::RuleThresholds;
    use prism_core::task::CancellationToken;
    use prism_godot::ir::{
        ConnectionRecord, ExtResourceRecord, NodeRecord, SubResourceRecord, TscnDocument,
    };
    use prism_godot::GodotAnalysis;
    use std::collections::BTreeMap;

    fn context<'a>(
        files: &'a [SourceFileInput],
        analysis: &'a GodotAnalysis,
        cancel: &'a CancellationToken,
        thresholds: &'a RuleThresholds,
    ) -> RuleContext<'a> {
        RuleContext {
            files,
            thresholds,
            observed_at: "t",
            csharp: None,
            godot: Some(analysis),
            supply: None,
            debug: None,
            evolution: None,
            api: None,
            cancel,
        }
    }

    fn document(
        nodes: Vec<NodeRecord>,
        ext: Vec<ExtResourceRecord>,
        well_formed: bool,
    ) -> TscnDocument {
        TscnDocument {
            format: Some("3".to_string()),
            ext_resources: ext,
            sub_resources: Vec::<SubResourceRecord>::new(),
            nodes,
            connections: Vec::<ConnectionRecord>::new(),
            well_formed,
            is_resource: false,
        }
    }

    fn node(
        name: &str,
        parent: Option<&str>,
        script: Option<&str>,
        instance: Option<&str>,
    ) -> NodeRecord {
        NodeRecord {
            name: name.to_string(),
            node_type: Some("Node".to_string()),
            parent: parent.map(str::to_string),
            instance_ext_resource_id: instance.map(str::to_string),
            script_ext_resource_id: script.map(str::to_string),
            line: 1,
        }
    }

    fn ext(id: &str, path: &str) -> ExtResourceRecord {
        ExtResourceRecord {
            resource_type: "Script".to_string(),
            path: format!("res://{path}"),
            id: id.to_string(),
        }
    }

    #[test]
    fn missing_script_reference_produces_warning_finding() {
        let cancel = CancellationToken::new();
        let files: Vec<SourceFileInput> = Vec::new();
        let thresholds = RuleThresholds::default();
        let mut scenes = BTreeMap::new();
        scenes.insert(
            "scenes/main.tscn".to_string(),
            document(
                vec![node("Player", Some("."), Some("missing_id"), None)],
                vec![ext("real_id", "player.cs")],
                true,
            ),
        );
        let analysis = GodotAnalysis {
            project: None,
            scenes,
            resources: BTreeMap::new(),
            diagnostics: Vec::new(),
        };
        let ctx = context(&files, &analysis, &cancel, &thresholds);
        let outcome = scene_wiring_rule(&ctx);
        assert_eq!(outcome.findings.len(), 1);
        assert_eq!(outcome.findings[0].finding_type, "GODOT/MISSING_SCRIPT");
        assert_eq!(outcome.findings[0].severity, prism_core::Severity::Warning);
        assert_eq!(outcome.findings[0].evidence.len(), 1);
        assert_eq!(
            outcome
                .metrics
                .iter()
                .find(|m| m.key == "godot_nodes_total")
                .unwrap()
                .value,
            1
        );
    }

    #[test]
    fn wired_scene_and_malformed_scene() {
        let cancel = CancellationToken::new();
        let files: Vec<SourceFileInput> = Vec::new();
        let thresholds = RuleThresholds::default();
        let mut scenes = BTreeMap::new();
        // 接线完好 ⇒ 无 finding；坏行 ⇒ MALFORMED_SCENE。
        scenes.insert(
            "scenes/ok.tscn".to_string(),
            document(
                vec![node("Player", Some("."), Some("ok_id"), None)],
                vec![ext("ok_id", "player.cs")],
                true,
            ),
        );
        scenes.insert(
            "scenes/bad.tscn".to_string(),
            document(vec![], vec![], false),
        );
        let analysis = GodotAnalysis {
            project: None,
            scenes,
            resources: BTreeMap::new(),
            diagnostics: Vec::new(),
        };
        let ctx = context(&files, &analysis, &cancel, &thresholds);
        let outcome = scene_wiring_rule(&ctx);
        assert_eq!(outcome.findings.len(), 1);
        assert_eq!(outcome.findings[0].finding_type, "GODOT/MALFORMED_SCENE");
        assert_eq!(outcome.findings[0].severity, prism_core::Severity::Info);
    }

    #[test]
    fn no_godot_facts_is_diagnostic_not_finding() {
        let cancel = CancellationToken::new();
        let files: Vec<SourceFileInput> = Vec::new();
        let thresholds = RuleThresholds::default();
        let ctx = RuleContext {
            files: &files,
            thresholds: &thresholds,
            observed_at: "t",
            csharp: None,
            godot: None,
            supply: None,
            debug: None,
            evolution: None,
            api: None,
            cancel: &cancel,
        };
        let outcome = scene_wiring_rule(&ctx);
        assert!(outcome.findings.is_empty());
        assert!(!outcome.diagnostics.is_empty(), "如实记录跳过");
    }
}
