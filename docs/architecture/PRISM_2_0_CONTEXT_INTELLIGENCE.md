# PRISM_2_0_CONTEXT_INTELLIGENCE.md

> 2.0-M27 产物（§73 允许合并：Context Intelligence/Model/Policy/Selection/Budget/Evidence 六主题并入本文；Test Matrix 见 §6）。实现 = `prism-context` 增量（evidence collector + generate_with_evidence）+ CLI `--impact-aware`（commit 927b79b）。

## 1. Context Pack 数据模型（§7-§11）

沿 M10 既有模型（ContextPack/ContextItem/ContextSection/ContextCandidate/SelectionReason/ExcludedCandidate——Source of Truth，未建第二套 §66）。M27 增量：Candidate 的新来源 = M26 Evidence Graph 关系。

## 2. Selection（§14/§17）

Candidate 收集与 Selection/Budget 分离（M10 既有架构保持）。新增 **evidence collector**：
- 输入 = M26 关系边（calls/inherits/...）+ 变更实体集（symbol_id）。
- 规则：target ∈ 变更集的边 ⇒ source 成为候选；resolved ⇒ `DirectlyImpacted`（高优先）；unresolved ⇒ `ReverseDependency`（§63 低可信，不混淆 §34）。
- Evidence = 关系 file:line；provenance = "evidence_graph"。
- 同实体同关系去重；无输入 ⇒ 静默跳过（零伪造）。

## 3. Task/Change/Impact-aware（§19-§21）

- Task-aware：M10 recipe 分类保持（auto/review/debug）。
- Change-aware：M26 change_mapping 的变更符号集 = 匹配目标。
- Impact-aware：M26 关系即 Impact 的 Evidence 面——受影响候选直接携带 CALLS/INHERITS 关系原文。
- Budget/Freshness：沿 M10（budget 剪枝、verification_stale）。

## 4. Explanation（§34/§58）

ContextItem.reasons = DirectlyImpacted/ReverseDependency（机器可读）；evidence = "file:line (relation)"。WhyAffected 数据基础 = 候选 reasons + evidence + 关系原文。

## 5. CLI（§42/§六十七）

`prism context <path> --impact-aware [--json]`——工作区 git 变更 ⇒ 符号集；M23/M25 引擎跑全项目 .cs ⇒ Evidence 关系；`generate_with_evidence` 生成。1.0 `prism context`（无旗标）行为零变化（薄包装兼容 §65）。

## 6. Test Matrix（§55 摘要）

| 用例 | 断言 |
|---|---|
| resolved 边 + target ∈ 变更集 | DirectlyImpacted 候选 + file:line evidence ✓ |
| unresolved 边 | ReverseDependency（低可信）✓ |
| 无关 target | 跳过 ✓ |
| 回归 | 1.0 context 测试全绿（23→26 项）✓ |

## 7. 边界（§五/§80-§82）

Context 不复制 M26 影响计算（只消费其关系）；不做 Prompt Packaging（M27 后续/M212）；风险评分归 M28；无 LLM。
