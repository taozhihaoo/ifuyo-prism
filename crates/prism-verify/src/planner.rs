//! VerifyPlanner（规格 M8 §25–§28/§269）：
//! 依赖拓扑排序 + Build→后续 的约定依赖 + Blocked 传播基础。

use crate::model::{CheckKind, CheckSpec, PlannedCheck, VerifyPlan};
use std::collections::{BTreeMap, BTreeSet};

/// 由 CheckSpec 列表构建执行计划（拓扑层序展开；§269 计划不可变）。
///
/// 约定依赖（§25 示例 / §27）：存在 Build check 时，Test/Typecheck/Lint/
/// Format/Custom 依赖 Build（可用显式 depends_on 覆盖；两者取并集去重）。
pub fn plan_checks(specs: &[CheckSpec]) -> VerifyPlan {
    let mut specs = specs.to_vec();
    specs.sort_by(|a, b| a.id.cmp(&b.id));

    let ids: BTreeSet<String> = specs.iter().map(|spec| spec.id.clone()).collect();
    let has_build = specs.iter().any(|spec| spec.kind == CheckKind::Build);

    // —— 依赖解析：显式 depends_on（限已知 id）+ 约定依赖 ——
    let mut dependencies: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for spec in &specs {
        let mut deps: Vec<String> = spec
            .depends_on
            .iter()
            .filter(|dep| ids.contains(*dep) && *dep != &spec.id)
            .cloned()
            .collect();
        if has_build && spec.kind != CheckKind::Build && spec.id != "build" {
            // 约定：Build 失败 ⇒ 后续 Blocked（§27）。
            if let Some(build) = specs.iter().find(|spec| spec.kind == CheckKind::Build) {
                if build.id != spec.id && !deps.contains(&build.id) {
                    deps.push(build.id.clone());
                }
            }
        }
        deps.sort();
        deps.dedup();
        dependencies.insert(spec.id.clone(), deps);
    }

    // —— 拓扑层序（Kahn；环内节点按 id 顺序落到可达层，§35 有界保证）——
    let mut remaining: BTreeSet<String> = ids.clone();
    let mut placed: BTreeSet<String> = BTreeSet::new();
    let mut ordered: Vec<PlannedCheck> = Vec::new();
    loop {
        let ready: Vec<String> = remaining
            .iter()
            .filter(|id| {
                dependencies
                    .get(*id)
                    .map(|deps| deps.iter().all(|dep| placed.contains(dep)))
                    .unwrap_or(true)
            })
            .cloned()
            .collect();
        if ready.is_empty() {
            // 剩余的是依赖环内节点：按 id 顺序强制落位（保证终止，§35 环）。
            if let Some(next) = remaining.iter().next() {
                placed.insert(next.clone());
                if let Some(spec) = specs.iter().find(|spec| &spec.id == next) {
                    ordered.push(planned_check(spec));
                }
                continue;
            }
            break;
        }
        for id in &ready {
            placed.insert(id.clone());
            if let Some(spec) = specs.iter().find(|spec| &spec.id == id) {
                ordered.push(planned_check(spec));
            }
        }
        for id in &ready {
            remaining.remove(id);
        }
    }

    VerifyPlan { checks: ordered }
}

pub fn planned_check(spec: &CheckSpec) -> PlannedCheck {
    PlannedCheck {
        id: spec.id.clone(),
        kind: spec.kind,
        name: spec.name.clone(),
        program: spec.program.clone(),
        args: spec.args.clone(),
        required: spec.required,
        source: spec.source,
        detected_from: spec.detected_from.clone(),
        depends_on: spec.depends_on.clone(),
    }
}
