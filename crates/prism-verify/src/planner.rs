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
    let dependencies = resolved_dependencies(specs.as_slice());
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
                    ordered.push(planned_check_resolved(spec, &dependencies));
                }
                continue;
            }
            break;
        }
        for id in &ready {
            placed.insert(id.clone());
            if let Some(spec) = specs.iter().find(|spec| &spec.id == id) {
                ordered.push(planned_check_resolved(spec, &dependencies));
            }
        }
        for id in &ready {
            remaining.remove(id);
        }
    }

    VerifyPlan { checks: ordered }
}

/// 计划项携带**解析后**的依赖（含约定依赖），而非 spec 原始声明。
fn planned_check_resolved(
    spec: &CheckSpec,
    dependencies: &std::collections::BTreeMap<String, Vec<String>>,
) -> PlannedCheck {
    let mut planned = planned_check(spec);
    if let Some(deps) = dependencies.get(&spec.id) {
        planned.depends_on = deps.clone();
    }
    planned
}

/// 显式 depends_on（限已知 id、去自环）+ 约定依赖（§27：同 toolchain 的
/// Build 失败 ⇒ 后续 Blocked）的统一解析。计划与执行共用，保证二者一致。
pub fn resolved_dependencies(
    specs: &[CheckSpec],
) -> BTreeMap<String, Vec<String>> {
    let ids: BTreeSet<String> = specs.iter().map(|spec| spec.id.clone()).collect();
    let has_build = specs.iter().any(|spec| spec.kind == CheckKind::Build);
    let mut dependencies: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for spec in specs {
        let mut deps: Vec<String> = spec
            .depends_on
            .iter()
            .filter(|dep| ids.contains(*dep) && *dep != &spec.id)
            .cloned()
            .collect();
        if has_build && spec.kind != CheckKind::Build {
            // 约定：Build 失败 ⇒ 后续 Blocked（§27）。限定同一 toolchain
            // （program 相同）：npm test 依赖 npm build，cargo-test 依赖
            // cargo-build——前端构建失败不应 Blocked 掉 cargo 测试。
            if let Some(build) = specs
                .iter()
                .find(|candidate| {
                    candidate.kind == CheckKind::Build
                        && candidate.program == spec.program
                })
            {
                if build.id != spec.id && !deps.contains(&build.id) {
                    deps.push(build.id.clone());
                }
            }
        }
        deps.sort();
        deps.dedup();
        dependencies.insert(spec.id.clone(), deps);
    }
    dependencies
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
