# M3 Final Audit — Dependency & Architecture

- 日期：2026-10-03
- 依据：`Spec/ifuyo Prism — M3 Dependency & Architecture 开发提示词.md` §41–§43
- 基线：M2 HEAD `4b29e2f` → M3 HEAD（本次最终 commit），工作树 clean
- 结论标记：FACT / HYPOTHESIS / INFERENCE

## 1. Baseline

```text
branch: main   M2 HEAD: 4b29e2f（clean）   本次 6 个子任务 commit
新增: crates/prism-graph（模型/模块构建/四语言 resolver/图构建/服务）、
      dependency-graph fixture、M3 集成测试套件、GUI Architecture 视图
```

## 2. Architecture（§43.2）

FACT：依赖方向单向
`prism-cli / src-tauri → prism-analysis / prism-graph → prism-parser → prism-project → prism-core`。
prism-graph 不依赖 Tauri/前端；GUI 经 IPC 读取 DependencyGraphResult。
状态：CONFIRMED（cargo tree + 代码审查）。

## 3. Module Model（§43.3）

FACT：Module（id/name/kind/file_ids/parent/language/path/evidence）与
File（FileInfo）、Symbol 三者清晰分离（§6）。ModuleId =
`module:{kind}:{identity}`（相对路径/包名/命名空间），确定性派生（测试锁定）。

FACT — 层级证据驱动（§17）：Package 仅当 `__init__.py` 存在；Namespace 仅当
声明符号存在（最长匹配父级）；无证据不造虚拟结构。
INFERENCE：ProjectId 维度暂缺（M1 运行期 UUID），跨项目唯一性由 M11 定。

## 4. Dependency Model（§43.4）

FACT：Dependency（id/source/raw_target/kind/resolution/confidence/target_id/
target_path/imported_names/location/evidence）+ 七态 ResolutionStatus +
DependencyKind（Import/ReExport/NamespaceReference/ProjectReference/
PackageReference/ModuleReference/Unknown）。Edge（结构依赖）与 Call 分离——
调用图属 P1 deferred（规格 §19 可靠子集：M2 未捕获调用语法，未实现，
如实报告）。

## 5. Resolution（§43.5）

自扫描（Prism 自己，`prism graph .`）FACT：

```text
resolved 71 / external 14 / builtin 6 / unresolved 0 / ambiguous 0
```

fixture 矩阵（dependency-graph）FACT：relative 链 resolved、tsconfig paths
(@/helpers/*) resolved、react external（package.json 声明）、fs builtin、
./ghost + ghost unresolved（含诊断）。C# exact/child/ambiguous/builtin/
unresolved 五路由单元测试覆盖。

## 6. Graph（§43.6）

FACT：自扫描 Nodes 33 / Edges 112 / Cycles 0 / Components 16；
Contains/Imports/ReExports/DependsOn/ProjectReferences 边种类；环 = SCC
（Tarjan 迭代，>1 节点），cycle fixture（x↔y2）检出且两次运行 ID 一致；
指标与 nodes/edges 数组一致性测试通过。

## 7. Call Graph（§43.7）

FACT：未实现（P1 deferred）。规格 §19 要求"可靠子集"，M2 未捕获调用语法
事实；M3 不猜测（handler() 类动态调用必须 Unresolved 而非指向）。
架构已预留：Dependency/CallEdge 模型分离。

## 8. CLI（§43.8）

```text
prism graph .                → 文本（计数/cycles/fan-in-out/diagnostics）exit 0
prism graph . --format json  → DependencyGraphResult（schema_version=1）PASS
prism graph . --kind module  → 层级过滤视图 PASS
prism graph <不存在路径>      → exit 3 PASS
```

## 9. GUI（§43.9）

FACT：`tauri dev` 启动成功；Understand 页 Architecture 区新增——客观指标卡、
无第三方依赖的 SVG 文件级图（确定性环形布局、60 节点上限如实说明、
节点点击显示 fan-in/out）、cycles 列表（标注"事实非健康判断"）、unresolved
诊断卡。IPC：start_graph/get_graph_status/get_dependency_graph，后台线程 +
轮询；scan → inspect → graph 在 ProjectStore 中自动链式执行（单管线，§29）。
HYPOTHESIS：SVG 交互的人工确认待做（同 M1/M2 项）。

## 10. Tests（§43 H）

```text
Rust：core 54 + project 48 + parser 19(12 单测+7 契约) + analysis 5 +
      graph 13(3 模块+5 resolver+5 图) + cli 17 + M0 集成 10 + M1 集成 9 +
      M2 集成 10 + M3 集成 6 = 191 全部通过
前端：8/8；typecheck/build 通过；clippy 0；fmt 通过
回归（§38）：M0/M1/M2 测试全部保留并通过（cargo test --workspace 单命令）
确定性（§37）：graph JSON 两次运行全等（YES）；同结构 → 同 ModuleId/边 ID
```

## 11. Performance（§43 I）

```text
small fixture（dependency-graph）：< 0.1s
Prism self-scan：70 源文件 → 33 节点 / 112 边，CLI < 2s（含 cargo run）
```

INFERENCE：万文件级项目 resolver 输入索引（files BTreeMap）足够；更深优化
（增量 resolution fingerprint，§28）已预留接口未实现。

## 12. Privacy / Security（§43 J）

FACT：无网络/无 telemetry/无 AI；external 判定仅基于本地 package.json 与
内置表（§35 禁止在线 lookup ✓）；graph JSON 自扫描零绝对路径泄漏
（程序化验证 = 0）；无 secret/源码转储。

## 13. Git（§43 K）

```text
branch: main   工作树 clean   未 push
M3 提交（6 个，子任务粒度）：
  3a4468d feat: capture import syntax facts in parsers (M3 input)
  6791780 feat: capture import syntax facts（修正版——见下）
  f95e0b9 feat: add prism-graph crate with relationship model and module hierarchy
  687e7e2 feat: add prism graph CLI
  faea967 feat: add dependency architecture view in the GUI
  dec9a68 test: add dependency-graph fixture and M3 integration suite
  1b67310 docs: record M3 graph layer…
注意：ImportInfo 提交曾因一次 amend 失误被拆成 3a4468d（不完整）+ 6791780
（完整修正版）；graph 模型/resolver 因同类失误并入 f95e0b9 单提交。内容均
完整（每次 amend 前后 tree 均通过全部测试），历史未改写，如实记录。
```

## 14. Risks

### P0
- 无

### P1
- 调用图未实现（P1 deferred，规格 §19 允许；M6 Impact 前需补）
- GUI SVG 交互人工确认待做；大图只渲染 60 节点（CLI JSON 为完整数据）
- Python stdlib 表为保守子集（未命中 → Unresolved，不误报）
- C# ProjectReference 解析为 naive Include 扫描（csproj 注释场景不处理）

## 15. Deferred

调用图可靠子集（P1）、Rust/Go/Java/C/C++ resolver、增量 resolution
fingerprint（接口已留）、M4 Git / M5 ChangeSet / M6 Impact / M7 Health /
M8 Verify / M9 Environment / M10 Context / M11 Snapshot——均未提前实现。

## 结论

INFERENCE：M3 = COMPLETE（§41 清单中 Core/Resolver/Graph/CLI/Tests/Safety
全部满足；GUI 的"graph"以文件级 SVG + 完整 JSON/诊断呈现，交互打磨属
M12）。关系层（实体 + 证据 + 解析状态 + 指标）可作为 M6/M7 的可信输入。
