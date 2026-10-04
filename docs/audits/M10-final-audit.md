# Prism M10 Final Context Engine Audit

日期：2026-10-03 · 范围：M10a（Selection Engine）+ M10b（Health / Privacy / Delivery）
复核文档：`docs/audits/M10a-gate-check.md`
基线：`cargo test --workspace` **498 passed / 0 failed**（60 个测试二进制）+ vitest **45 passed**，clippy **0 warning**。

## A. Baseline

M0–M9 全部 FACT/CONFIRMED；M10 新增 prism-context 未改动既有 crate（M8 的
redact/lookup 已在 M9 下沉 prism-core）。无 WORSENED；P0 无。

## B. Architecture

M1–M9 facts → candidate collectors → normalization → deterministic selection →
budget → materialization → privacy（中央层）→ render。依赖方向：prism-context →
{core, project, analysis, graph, changes, impact, health, verify, env}，无环；
CLI 与 GUI 均只经 ContextService（§475 同源，context_commands.rs 复用同一
generate 流程）。

## C. Context Model

ContextRequest/Target(Resolved|Ambiguous|NotFound|None)/Candidate/Item/Pack/
SelectionTrace/Manifest/Budget/Health 全部存在且 serde 稳定（schema_version=1，
§246/§247；§248 domain model 不含 React/UI 字段）。

## D. Task / Target

14 TaskKind 中英关键词分类（多 kind，§13）；Unknown ⇒ Generic + 明确声明
（§18 测试）。Target：唯一 ⇒ Resolved；多候选 ⇒ Ambiguous 全列出（§16 测试）；
无 ⇒ NotFound/None。

## E. Collectors / Dedup / Priority

11 collector 只产候选（§89）；归一化身份 (source_type, path|source_id)（§93–
§97 沿用 M2/M3/M7/M8 id）；§92 多理由保留测试。优先级带 Critical→Low 规则化
（§144），review diff Critical（§99）、failed verification 对 bug/test 任务
Critical；稳定 tie-break (band, source_type, path, location)（§140）；
reproducibility CLI 测试两次运行 ids/bytes 全等（§138/§220/§478）。

## F. Recipes

Auto/Review/Debug（§291 有界）；Review ⇒ CurrentChange 先（§124 测试）；
Debug ⇒ Verification 先（§118）；Auto 按 focus 适配 CI（§121 测试）/
Documentation（§123）；服务排序 recipe 驱动（§116 policy）。

## G. Budget

max_items/max_source_files/max_bytes/max_tokens_estimate 全部生效；Critical
预留（超预算 Critical 仍入选，§197/§198 测试）；零预算 ⇒ exit 3（§200 测试）；
极小预算 ⇒ minimal context + "N candidate(s) excluded by budget" limitation
（§199 测试）；估算 tokens = bytes/4，恒标 estimated（§32/§388）。

## H. Materialization / Truncation

懒物化只读被选文件（§167/§168）；binary/minified/lockfile ⇒ metadata-only、
generated/vendor ⇒ skeleton、huge ⇒ skeleton（§169–§174 测试）；symbol range
切片（§37）+ 行边界 + markdown heading 回退（§36/§176 测试）；truncated 恒带
original/included/reason（§35 测试）。

## I. Privacy

- 变量/密钥值结构性的不采集、materializer 中央 redact（[REDACTED] 保留行，
  §377–§379 测试）、content_hash 对 redacted 后内容（§380）
- `.env` 只 metadata（M9 §58-§60 事实，§375）
- HTML/markdown/terminal 转义：render escape_markdown + strip_control +
  fenced 动态长度（§242–§245）
- GUI 无"显示原始 secret"路径（§382）；用户任务不绕过（§383 结构保证）
- 无第二套 redaction engine（§257：复用 prism_core::privacy）

## J. Prompt-injection boundary

instructions/docs 在 pack 中标 Documentation/ProjectInstruction 信任层
（§188–§190 never flatten）；collector evidence 声明 "data, never executed"；
注入行不是命令候选（doc_health 测试 §459）；渲染含 limitations/risk signals
让下游看到边界（§312–§315 schema 保留 trust/provenance）。

## K. Context Health

与 M7 分离（§62/§274 页面/UI 均声明）；CTX005 verification stale、CTX007
target ambiguous、CTX008 candidate unavailable、CTX009 critical truncated、
CTX011 instruction scope overlap 已实现（evidence-first，core Severity §341、
Confidence §342）；coverage 分项无总分（§129/§277/§462）。CTX002/003 路径与
命令漂移由 doc_health 提供（§65–§69/§178–§184：仅事实，不判对错）。CTX001/
CTX006/CTX010/CTX012 保守未启用（无可靠证据源——记录为 P1）。

## L. Freshness

ContextStateReference（head/branch/dirty/generated_at，§136/§137）+ 
ContextFreshness（§333）+ VerificationStale 风险信号与 CTX005（§131–§132：
M8 DirtyWorkspace ⇒ StaleForCurrentWorkspace）；M9 observation timestamp 
透传（§134）。§337 no fake fresh：无"新=正确"声明。

## M. CLI

实测：`prism context .`、`--task`、`--json`、`--markdown`（§349）、`--dry-run`
（§285/§350：只 plan 不物化，测试断言无源码正文）、`--explain`（§287）、
`--check-health`（§284）、`--scope`、`--target`、`--max-tokens/--max-items/
--max-bytes`（§288）、`--profile auto|review|debug`（§290/§291）；exit 0/1/3
（§351：incomplete 不失败——实测 Partial/complete 均 exit 0）。

## N. GUI

Context 页真实挂载（占位页移除）：task 输入 + scope + Generate（§293/§294）、
生成摘要（§295）、分组 items + why/trust + 展开正文（§296–§298）、excluded
（§299）、health + §274 边界声明（§300）、budget 计数（§301）、Copy Markdown/
Text + Export IPC（§302/§303/§311 用户显式、无自动上传 §308–§310）。4 vitest。

## O. M1–M9 reuse / No duplication

FactsInputs 全由调用方注入；无第二套 parser/graph/git/health/verify/env
（§463–§468）；integration crate 的 M8×M9 组合测试已在 M9b 建立。

## P. Real validation（§408–§417）

- Prism 自身 review 任务：18 items、changeset-first、§416 ratio 记录
  （396 files → 0 file items——clean tree 无变更候选，诚实）；limitation
  "16 excluded by budget — minimal, not comprehensive"
- CI 任务：kinds=[build,ci]、focus 含 ci/configuration/verification
- 歧义目标："refactor the project service layer" ⇒ target none ⇒ generic
  context（不猜）
- 真实 TS（ctxdemo）/Python（pydemo）/C#（csdemo）项目 exit 0

## Q. Tests

498 Rust（context 26 lib + 11 context_cli 契约 + 1 M8×M9 组合）+ 45 vitest；
覆盖 selection/dedup/budget(§400)/truncation(§402)/freshness(§403 部分)/
doc health(§404)/privacy(§405)/injection(§406 部分 fence+command)/CLI 契约/
GUI 状态机。

## R. Git

工作树干净；无生成物入库。

## S. Risks

P0 无。P1（非阻塞）：
1. CTX001/006/010/012 未启用（缺可靠证据源）；CTX002/003 保守事实化。
2. GUI per-item 展开为全文 `<pre>`，未分页（§363 pack 上限已有 max_bytes）。
3. §289 --include/--exclude 粒度过滤未做（安全不受影响——redaction 在
   materializer 不可绕过）。
4. §335 动态环境独立 freshness 阈值未分档（统一 Unknown 保守）。
5. §150–§152 相邻 range 合并、§203 DocumentSection 检索（M10a P1 沿袭）。
6. §361 GUI IPC 大小：pack max_bytes 默认 512KiB 上界，未分块。

## T. Deferred

M11 Snapshot/history、M12 UI polish、M13 hardening；LLM/AI adapter/
agent loop/RAG/embeddings/vector/MCP/cloud/自动上传/自动修改（§481.Y）——
ContextConsumer/RelevanceProvider 只留概念，未实现 stub。

## §482 Completion Gate（摘录核对）

全部 [x]（M0–M9 回归、模型/分类/收集/去重/优先级/recipe/多 awareness/budget/
truncation/provenance/freshness/doc 路径与命令/CTX/隐私四件套/三渲染/CLI/GUI/
dry-run/export/copy/同源 parity/M1–M9 复用/无重复引擎/无 AI/无网络/无 DB/
无 P0 无阻塞 P1）——逐项证据见上文 A–R 与 M10a-gate-check.md。

结论：**M10（M10a + M10b）验收通过**，按 §483 执行最终提交。
