//! 静态调用图（规格 §19 可靠子集）。
//!
//! 解析策略（保守，不猜测）：
//!
//! 1. 同文件：callee 最后一段与本文件 Function/Method/Constructor 名匹配
//!    → Resolved（High）。
//! 2. 已解析 import 的目标文件：该文件内同名符号 → Resolved（Medium）。
//! 3. 其余（动态分发、反射、未导入名字）→ Unresolved（Low），不指向任何
//!    符号（规格 §19：`handler()` 无法确定时保持 Unresolved）。
//!
//! 输出按稳定 ID 排序（规格 §27）。

use crate::model::{CallEdge, ResolutionStatus};
use prism_analysis::ProjectCodeModel;
use prism_core::ids::SymbolId;
use prism_core::Confidence;
use prism_parser::parse::CallInfo;
use prism_parser::SymbolKind;
use std::collections::BTreeMap;

/// 从 Code Model + 已解析依赖解析全部调用事实为 CallEdge。
pub fn resolve_calls(
    model: &ProjectCodeModel,
    dependencies: &[crate::model::Dependency],
) -> Vec<CallEdge> {
    // ---- 名字索引：name → [(文件路径, 符号 ID, 限定名)]（仅可调用种类）----
    let mut name_index: BTreeMap<String, Vec<(String, SymbolId, String)>> = BTreeMap::new();
    // ---- 限定名 → 完整 SymbolId（caller_id 必须与 M2 身份一致，含 #kind 段；
    // 否则 M6 反向调用遍历在第 2 跳断链）----
    let mut qualified_ids: BTreeMap<(String, String), SymbolId> = BTreeMap::new();
    for result in &model.parse_results {
        for symbol in &result.symbols {
            if matches!(
                symbol.kind,
                SymbolKind::Function | SymbolKind::Method | SymbolKind::Constructor
            ) {
                name_index.entry(symbol.name.clone()).or_default().push((
                    result.relative_path.clone(),
                    symbol.id.clone(),
                    symbol.qualified_name.clone(),
                ));
            }
            qualified_ids
                .entry((result.relative_path.clone(), symbol.qualified_name.clone()))
                .or_insert_with(|| symbol.id.clone());
        }
    }

    // ---- 已解析 import：源文件 → 目标文件集合 ----
    let mut import_targets: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for dependency in dependencies {
        if dependency.resolution != ResolutionStatus::Resolved {
            continue;
        }
        if matches!(
            dependency.kind,
            crate::model::DependencyKind::Import | crate::model::DependencyKind::ReExport
        ) {
            if let Some(target) = &dependency.target_path {
                import_targets
                    .entry(dependency.source_path.clone())
                    .or_default()
                    .push(target.clone());
            }
        }
    }

    // ---- 逐调用解析 ----
    let mut calls: Vec<CallEdge> = Vec::new();
    for result in &model.parse_results {
        for call in &result.calls {
            calls.push(CallEdge::resolve_one(
                &result.relative_path,
                call,
                &name_index,
                &qualified_ids,
                import_targets.get(&result.relative_path),
            ));
        }
    }

    calls.sort_by(|a, b| {
        a.caller_qualified
            .cmp(&b.caller_qualified)
            .then_with(|| a.callee_text.cmp(&b.callee_text))
            .then_with(|| {
                a.location
                    .map(|l| (l.line, l.column))
                    .cmp(&b.location.map(|l| (l.line, l.column)))
            })
    });
    calls
}

impl CallEdge {
    fn resolve_one(
        source_path: &str,
        call: &CallInfo,
        name_index: &BTreeMap<String, Vec<(String, SymbolId, String)>>,
        qualified_ids: &BTreeMap<(String, String), SymbolId>,
        import_targets: Option<&Vec<String>>,
    ) -> CallEdge {
        let base = call
            .callee_text
            .rsplit('.')
            .next()
            .unwrap_or(&call.callee_text)
            .to_string();

        // 1) 同文件同名可调用符号。
        let same_file = name_index
            .get(&base)
            .into_iter()
            .flatten()
            .find(|(path, _, _)| path == source_path);
        // 2) 已解析 import 的目标文件内同名符号。
        let via_import = if same_file.is_none() {
            import_targets.and_then(|targets| {
                targets.iter().find_map(|target| {
                    name_index
                        .get(&base)
                        .into_iter()
                        .flatten()
                        .find(|(path, _, _)| path == target)
                })
            })
        } else {
            None
        };

        let (resolution, confidence, target) = if let Some((path, id, qualified)) = same_file {
            (
                ResolutionStatus::Resolved,
                Confidence::High,
                Some((path.clone(), id.clone(), qualified.clone())),
            )
        } else if let Some((path, id, qualified)) = via_import {
            (
                ResolutionStatus::Resolved,
                Confidence::Medium,
                Some((path.clone(), id.clone(), qualified.clone())),
            )
        } else {
            (ResolutionStatus::Unresolved, Confidence::Low, None)
        };

        CallEdge {
            id: format!(
                "call:{}#{}#{}",
                call.caller_qualified,
                call.callee_text,
                call.location
                    .map(|l| format!("{}:{}", l.line, l.column))
                    .unwrap_or_else(|| "0:0".to_string())
            ),
            caller_id: qualified_ids
                .get(&(source_path.to_string(), call.caller_qualified.clone()))
                .cloned()
                .unwrap_or_else(|| {
                    // 回退：宿主符号不在模型中（理论不可达——calls 只在
                    // 符号体内捕获）；派生保持旧行为，不 panic。
                    SymbolId::from_string(format!(
                        "symbol:{}#{}",
                        source_path, call.caller_qualified
                    ))
                }),
            caller_qualified: call.caller_qualified.clone(),
            callee_text: call.callee_text.clone(),
            callee_name: base,
            callee_id: target.as_ref().map(|(_, id, _)| id.clone()),
            callee_qualified: target.as_ref().map(|(_, _, qualified)| qualified.clone()),
            resolution,
            confidence,
            location: call.location,
        }
    }
}
