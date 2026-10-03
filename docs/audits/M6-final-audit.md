# M6 Final Audit — Change Impact Analysis

日期：2026-10-03 ｜ 规格：`Spec/Prism M6 — Change Impact Analysis 完整开发提示词.md`

---

## A. Baseline（M0–M5）

- **FACT**：M3 `Dependency{source_id→target_id}`=import 语义（source 依赖 target）；`CallEdge{caller_id→callee_id}`（callee Some=Resolved）；Resolution 七态；cycles=SCC；`DependencyService::analyze(discovery, model)`。方向契约以真实代码核验，非字段名猜测（§11/§12）。
- **FACT**：M5 ChangeSet/SymbolChange（before/after 身份+confidence）/FileChange/ModuleChange/diagnostics 完备。
- **GAP → RESOLVED**：GitProvider 无历史源读取（M5 已补 read_blob/resolve_revision）；M3 `CallEdge.caller_id` 与 M2 SymbolId 格式不一致（缺 `#kind` 段）导致反向调用链第 2 跳断裂 —— **本里程碑修复**（WORSENED→RESOLVED，见 O）。

## B. Impact Architecture（§121/§124）

- **FACT**：新 crate `prism-impact`（core ← project/parser/analysis/graph/changes ← impact ← cli/tauri）。M3 拥有关系事实、M5 拥有变更事实、M6 只解释（§64）；发现可疑关系记 diagnostic 不私改。
- **FACT**：稳定服务边界 `ImpactAnalysisService::analyze(change_set, code_model, graph, options, cancel) -> ImpactAnalysis`（§124）；CLI/GUI 同源调用。模块拆分 model/traversal/service/render（§122），无投机抽象（§123）。

## C. Graph Semantics（§5/§11/§12/§35/§50/§80）

- **FACT**：反向索引一次构建（§62/§63，不复制 M3 模型）：resolved 模块反向依赖（target→sources）、resolved 调用反向（callee→callers）、未解析调用名字索引（deleted-symbol 证据专用 §20）、未解析模块依赖按 raw_target 仅计数。
- **FACT**：只遍历 Resolved 边；Unresolved/Ambiguous 计数+诊断，绝不伪造目标（§5/§102/§103 有测试）。
- **FACT**：visited-by-distance 保证环终止（§80 有测试）；自影响保持 DirectlyChanged（§50 有测试）；路径重建在 distance-0 停止 + 跳数上限，互依赖环不会走成无限路径。

## D. Impact Model（§7/§8/§30/§45/§79–§84）

- **FACT**：`ImpactableNode{impact_kinds[], distance(min), confidence(weakest 传播), reasons[], evidence_path_count, additional_paths}`；Changed 与 Impacted 集合严格分离（§48/§49 有测试）；distance=关系跳数、多路径取最短并保留替代前驱（上限 4，§34/§82/§109 有测试）；确定性排序全链路（§79/§113 有测试）。

## E. Symbol Impact（§13–§24/§96–§99）

- **FACT**：caller（CallsChangedSymbol）、传递 caller（TransitiveCaller）、deleted（名字索引精确限定名=HighConfidence/同名=Probable + CallsDeletedSymbol + DeletedReferencedSymbol 信号，§96 e2e）、rename 保守用 after 身份（§19/§99 e2e）。签名/可见性变更经 M5 characteristics 进入 PublicApiChanged 信号与常规 caller 查找（§22/§23）。
- **INFERENCE**：ReferencesChangedSymbol 语义位保留——当前 M3 的符号级关系即调用边，无独立"引用"边（以代码为准）。

## F. Module Impact（§17/§25/§51/§52）

- **FACT**：模块反向依赖 direct/transitive（§92/§93 有测试）；file-only 变更经 File→Module 兜底种子（§18）；workspace 项目引用=ProjectConsumer（§51）；外部包不扫描不下载（§52，无网络）。

## G. Test Candidates（§26/§27/§54/§55/§107）

- **FACT**：M2 管线纳入 Test 文件（M1 一等分类；最小接口扩展），test→production 关系可解析 ⇒ TestCandidate 按 distance 分类（direct/transitive 有 e2e）；措辞为 "Potentially affected tests / test candidate"，绝不出现 "will fail"（§27，UI 测试断言）。

## H. Documentation Candidates（§28/§29/§108）

- **FACT**：机制存在但当前 M3 无可解析的文档引用关系 ⇒ 产出 `NoDocumentationEvidence` 诊断而非猜测（§108 "无证据不猜"；§145 "where evidence exists"——证据目前结构性不存在，如实声明）。

## I. Risk Signals（§40–§43/§85/§86/§130）

- **FACT**：6 种信号全部为事实计数+有界证据（上限可配）：PublicApiChanged（可见性/导出特征）、DeletedReferencedSymbol、CrossModuleImpact、UnresolvedRelationship、AmbiguousSymbolMapping、ConfigurationFileChanged。**无阈值、无总分**（§85 优先"只输出原始计数"选项；§41 禁 0-100 分）。

## J. Limits（§36/§37/§78/§127）

- **FACT**：max_depth=10 / max_nodes=2000 / max_paths=10000 / 每信号证据 20 条（ImpactOptions，保守默认）；depth/nodes 超限 ⇒ TraversalDepthExceeded / ImpactNodeLimitReached + `analysis_incomplete=true` + completeness=Truncated；truncated ≠ failed（§90，CLI 测试断言 exit 0）；1000 节点扇出有界确定性（§110 有测试）；取消逐节点检查（§112 有测试）。超时未单独实现（取消已覆盖 GUI 场景；Deferred）。

## K. CLI（§87–§90）

- **FACT**：`prism impact [PATH] [--staged] [--from/--to] [--format text|json|markdown] [--max-files] [--max-depth] [--max-nodes] [--include-tests/--include-docs]`；退出码 0/1/3/4/6，truncated 仍 0；6 个契约测试（含 JSON schema 字段齐备、确定性、truncated-success、非 git 6、缺失路径 3）。

## L. GUI（§70–§76/§131/§136/§137）

- **FACT**：Changes 页在 ChangeSet 完成后自动链式运行影响分析（独立取消令牌，页面卸载取消 §112）；渲染 Impact summary（completeness/max distance/unresolved）、可过滤的受影响列表（All/Modules/Symbols/Tests/Direct/Transitive chips §71 + "View all" 展开 §137）、"Why impacted?" top paths（逐跳 relation+resolution §73）、事实性风险信号（§40）、常驻 Analysis limitations 声明（§74/§131）。14 个前端测试 + tsc + vite build 通过；tauri 命令镜像 M5 模式。

## M. Tests（§92–§113）

- **FACT**：全工作区 300+ 测试绿（33 套件，prism-impact 25 单测/合成图 + 5 e2e + prism-cli 6 impact 契约 + 前端 14）。合成图覆盖 §92–§113 全部拓扑（链/环/多路径/歧义/未解析/大扇出/深度/取消/确定性）；真实 git e2e 覆盖 §138（主场景）/§139（删除符号）/§104–§106（部分成功）/§100（纯 rename 无假影响）。

## N. Performance（§116 实测）

- **FACT**（Prism 自身，Windows debug 构建）：graph = 65 modules / 103 deps / 139 calls / 82 nodes / 217 edges；`prism impact --from HEAD~3 --to HEAD`（15 文件/27 符号变更）：**4.07s** 端到端（含 discovery+全量解析+图构建）；HEAD~1 区间 **0.74s**。real-ts（3 文件链）亚秒。内存未单独测量（无失控迹象；§115 "不失控"满足）。

## O. Security / Privacy（§118–§120/§142/§143）

- **FACT**：无网络/遥测/AI/embeddings/运行时 tracing/数据库；新增 git 调用为零（M6 只消费 M4/M5 产物）；无任何 mutating git 路径；输出仅项目相对路径。

## P. Git

- **FACT**：子任务分笔提交（model/traversal/service+e2e/CLI/GUI/修复/docs）；工作树仅含本审计与 docs；未 push。

## Q. Risks（仅 P0/P1）

- **P1（本里程碑发现并修复）**：M3 `CallEdge.caller_id` 缺 `#kind` 段与 M2 SymbolId 不一致 → 反向调用遍历第 2 跳断裂。已修复（resolve_calls 按 (path, qualified) 查表取完整 id，缺失时回退旧行为）；真实重构场景验证传递调用链恢复。无测试曾断言旧格式（安全修复）。
- **P1（遗留，不阻塞 M7）**：Python 绝对导入（`from pkg import x`）M3 不解析（相对导入可解析）——影响在未解析侧如实计数；M2 选择器纳入 Test 文件后 GUI inspect 的符号计数包含测试符号（行为变化已记录）；大仓库（万级节点）遍历未实测（有界+可取消已具备）。
- **无 P0**。

## R. Deferred

- **M7 Health Engine**：架构健康/规则/评分——M6 的 CrossModuleImpact 等仅为事实信号（§132 边界已测）。
- **M8 Verify**：TestCandidate → 实际执行/Pass/Fail（§133/§55 候选语义）。
- **M9 Environment / M10 Context / M11 Snapshot**：M6 输出（changed+impacted+paths+uncertainty）为 M10 ContextPack 与 M11 时序的直接输入（§134/§135）。
- **M6 内部推迟**：遍历超时（取消已覆盖）、before-graph 完整物化（deleted-symbol 已用名字索引覆盖核心场景）、GUI 影响图可视化（列表+路径已满足 §136 文字结构）。

---

## M6 Completion Gate（§145）

M0–M5 回归 ✅（300+ 测试）｜ChangeSet→ImpactAnalysis ✅｜direct/transitive/reverse ✅｜caller/reference/module/deleted/signature/visibility/rename ✅｜test/docs candidates（证据处）✅｜distance/paths/合并/环/不确定保持/部分/截断/取消/确定性 ✅｜blast radius + 事实信号 + 无总分 ✅｜CLI/JSON/GUI ✅（Markdown/HTML 经 render 模块，HTML 复用留 M12 前端）｜Prism 自分析 ✅｜TS/Python/C# 真实项目 ✅｜真实重构场景 ✅｜无 AI/网络/遥测/数据库/运行时/git 变更 ✅｜无 P0 ✅｜无阻塞 M7 的 P1 ✅
