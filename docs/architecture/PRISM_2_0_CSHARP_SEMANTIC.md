# PRISM_2_0_CSHARP_SEMANTIC.md

> 2.0-M25 产物（§64 允许合并：本文件覆盖 Parser Decision / Semantic Relations / Compilation(invalidation) / Support Matrix；Benchmark 另见 PRISM_2_0_M25_BENCHMARK.md）。
> 实现 = `prism-csharp` 新增 `semantic.rs` + `intel.rs::semantic_to_intel`（commit 4d3f727）。

## 1. Parser/Backend Decision（§七评估矩阵实录）

| 候选 | 结论 | 依据 |
|---|---|---|
| Roslyn | **否**（Foundation 阶段） | .NET 进程外 helper + 用户机器 SDK 前置 + DesignTime Build 复杂度（§七硬约束）——Foundation 的作用域规则解析已覆盖核心关系；Roslyn 升级路径保留（接口不变，换 Resolution=Exact 后端即可） |
| tree-sitter + 符号表作用域规则 | **采用** | 零运行时依赖；语法树 + 精确 range；M23 符号表提供作用域事实；诚实三级置信 |

**诚实声明（§61/§六十二）**：本实现是 **declaration/scope-based resolution**，不是 Roslyn 级 SemanticModel——by-argument-type 的 overload 精确解析、泛型替换、extension method 解析 = Unsupported（关系如实标 Unresolved/Unsupported，不伪造）。正确性 > 覆盖率（§六十二）。

## 2. Semantic Model

`SemanticRelation{kind, source_symbol_id, target, dispatch, resolution, confidence, file, range, cross_project}`：
- **Resolution 五态**（§13）：Resolved / Ambiguous / Unresolved / Unsupported / Unknown——与 Confidence 分离（§34）。
- **Dispatch 六类**（§17）：Direct/Virtual/Interface/Delegate/Dynamic/Unknown——静态可分派 ≠ 运行时唯一目标。
- **Target**：Resolved{symbol_id=M23 id, qualified_name} | External{name,reason} | Unresolved{name,reason} | Ambiguous{name,candidates} | Unknown{reason}。
- 诊断：SemanticDiagnostic{kind: dynamic_invocation / reflection_usage, ...}——与编译器诊断分离（§42）。

## 3. Resolution 算法（§61 合规声明）

类型名解析（作用域规则，三级）：① 同命名空间唯一 → Resolved/High；② using 命名空间命中 → Resolved/High；③ 全局唯一 → Resolved/Medium（Probable）；多候选 → Ambiguous；零 → Unresolved。
成员/调用解析：包含类型链 + base 链同名同 **arity** → Resolved（按 M23 modifier: override/virtual ⇒ Dispatch=Virtual else Direct）；接收者为参数/字段 ⇒ 用其声明 TypeSyntax 解析接收者类型（§46 用例）再沿链解析；链上未命中 ⇒ Unresolved（外部/框架）。
**边界（§61 禁令合规）**：无字符串匹配建边；using 本身不产生语义依赖；by-argument-type overload / generics 替换 / extension method = Unsupported 如实标注。

## 4. Relations（§12 首批八种）

inherits / implements / overrides / calls / constructs / references_type / subscribes_event / unsubscribes_event——全部 RelationType 开放注册（inherits/implements/calls/references 复用 M21 内置，其余 other()）；Resolution/Dispatch 进 metadata。

## 5. Compilation / Invalidation（§八/§36-§39）

无 Roslyn Compilation——"Compilation Configuration" 当前 = 单一隐式配置（无 csproj 解析；.csproj 的 ProjectReference 声明在 M22 manifest 层记录）。失效规则：content_hash 变 ⇒ 重抽取+重语义（SemanticEngine per-file）；索引/关系由当前文件集合重建（无幽灵，Invariant 7/8 有测试）。多配置（Debug/Release）隔离 = deferred（§24 记录）。

## 6. Support Matrix（§65 实测填写）

| 能力 | 状态 | 证据 |
|---|---|---|
| CALLS（同名/含 base 链/参数声明类型接收者） | Supported | receiver_parameter_type_resolves_calls + calls_resolved_via_containing_type_chain |
| CONSTRUCTS | Supported | constructs_resolved_and_unresolved_honest |
| INHERITS / IMPLEMENTS | Supported | inherits_and_implements_resolved_via_scope_rules |
| OVERRIDES | Supported（排除自身；base 链解析） | override_resolves_base_chain_method |
| ReferencesType（base 面） | Supported（含 External 如实记录） | 同上 |
| by-argument-type overload | Unsupported | §八 决策；arity 级区分保留（M23 identity） |
| Generic 替换 | Unsupported（声明面在 M23） | §20 边界 |
| Extension method | Unsupported | §61 |
| Delegate/Event 基础 | Partially（subscribes/unsubscribes 关系 + 方法组解析） | collect_event_subscription |
| dynamic / reflection | 检测+降级（诊断 + Unknown） | dynamic_call_is_unknown_not_faked / reflection_detection |
| Project-to-Project 语义依赖 | Partially（cross_project 标记就绪；csproj 分组待 M26 用例） | §25 |
| Conditional compilation | Unsupported（单配置，§24 deferred） | — |
| Incremental semantic analysis | Supported（hash 复用 + ==full 测试） | incremental_semantics_equal_full |
| Cross-project resolution | Partially（cross_project 标记；多项目分组 deferred） | — |

## 7. Invalidation / Tests / Security

- Invalidation：per-file hash（M23 extraction 共享，无第二套解析缓存 §39）+ 当前集合重建（Invariant 7/8）。
- Tests：M25 新增 10 项（inherits/override/calls/constructs/dynamic/reflection/ambiguous/incremental/receiver-parameter/§46 集成）；§47 最小语义集覆盖度见上表。
- Security：纯内存解析；无 shell/dotnet 执行（§55 —— Roslyn 路线免除了进程面）；文件读取 4MiB 上限沿 CLI 层。
