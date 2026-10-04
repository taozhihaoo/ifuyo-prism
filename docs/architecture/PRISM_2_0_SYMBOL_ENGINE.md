# PRISM_2_0_SYMBOL_ENGINE.md

> 2.0-M23 产物（§74 允许合并：本文件覆盖 Parser Decision / Symbol Model / C# Support Matrix / Incremental；Benchmark 另见 PRISM_2_0_M23_BENCHMARK.md）。
> 实现 = 新 crate `prism-csharp`（15 项测试）；语言无关契约仍由 `prism-intel` 承载（§62 不造空抽象）。

## 1. Parser Decision（§四 评估矩阵实录）

| 候选 | 结论 | 依据 |
|---|---|---|
| Roslyn | **否**（留 M25 再议） | 需进程外 .NET 运行时（Prism 是 Rust）——新环境前置 + 跨平台负担；L1 声明抽取不值得该成本 |
| **tree-sitter-c-sharp 0.23.5 + tree-sitter 0.25** | **采用** | **本地 cargo 缓存已有**（离线可装）；MIT；C 编译内嵌无运行时依赖；错误恢复最佳（§30 硬要求）；精确 SourceRange；增量解析支持；跨平台 |
| 手写声明扫描器 | 备选未用 | 1.0 Python parser 先例可行，但错误恢复/range 精度明显弱于 tree-sitter，工作量更大 |
| 复用 1.0 prism-parser | 部分 | prism-parser 本就 tree-sitter-based（M2 Python/Ts 走它）；但其 Symbol 模型（M2 简化形态）不承载 partial/overload/attributes——M23 用独立 crate，避免污染 M2 既有契约（§66：KEEP 旧、新并行） |

Grammar 节点名以探针实测为准（开发期 probe 验证 record_declaration/file_scoped_namespace_declaration/enum_member_declaration_list 等），未臆测。

## 2. Symbol Model（§6-§9/§59）

- `RawSymbol`（单文件语法层）→ `SymbolIndex.Symbol`（跨文件逻辑聚合）。
- **Identity**：`cs:{qualified}#{kind}#{sig8}`——qualified 含 namespace+容器链；**sig 分量仅可调用成员**（method/ctor/operator/delegate/local function）→ overload 可区分、字段不受签名影响；**行号/文件不参与**（§7；code-movement 测试证明）。
- **Partial**（§9）：logical symbol 聚合全部 `DeclarationOccurrence{file, range, name_range}`——`Player.cs + Player.Inventory.cs ⇒ 1 逻辑符号 / 2 声明`（测试覆盖）。
- **Location**：declaration range + name range 一等公民（1-based）；Range 是证据非身份。
- Modifiers/type_parameters(arity)/constraints/base_types(语法)/attributes(名称原文——`[Export]` 语义归 M24)/parameters(TypeSyntax 原样)/doc_summary（`///` 剥 XML 标签纯文本）。

## 3. C# Support Matrix（§75 实测填写；Support Matrix 来源 = 测试）

| Feature | Support | Evidence / Test |
|---|---|---|
| Namespace（块） | Supported | extracts_types_members_and_hierarchy |
| File-scoped namespace | Supported | modern_syntax_coverage（兄弟作用域语义实测） |
| Class / Struct / Interface / Enum | Supported | 同上 |
| Record / Record struct | Supported | 同上（readonly record struct 区分） |
| Delegate | Supported | 同上 |
| Nested type | Supported | Player.Inner（walk 嵌套） |
| Generic + arity + constraints | Supported | fixture Player\<T\> where T : class |
| Method / Constructor / Destructor | Supported | extracts… |
| Property / Field（多 declarator）/ Event | Supported | 同上 |
| Indexer / Operator | Supported（声明级） | fixture |
| Enum member | Supported | modern_syntax_coverage |
| Overload | Supported | overloads_distinguished… |
| Partial | Supported（跨文件聚合） | partial_type_aggregates_across_files |
| Attributes | Supported（名称原文） | extracts…（Serializable） |
| Documentation（///） | Supported（剥标签摘要） | extracts…（"Player doc."） |
| Local function | Supported | fixture Inner 方法内嵌套（parent=方法） |
| Top-level statements | Supported（跳过语句不失败，§12） | scanner 设计 + 非 type 兄弟节点 walk_root 兜底 |
| Global using / using alias | Partial（跳过为 trivia；不做语义） | §12 边界 |
| Syntax recovery | Supported（ERROR 子树内声明仍抽取；qualified 可能降级——已测并记录） | syntax_error_does_not_kill_extraction |
| 显式接口实现/params/ref out in 修饰 | Partial（进 parameter 文本，不单独建模） | §16 边界 |

## 4. Incremental / Cache（§28-§30/§39-§40）

- **Source of Truth** = `SymbolEngine.extractions`（per-file：content_hash + 符号 + 诊断）。
- `update_file`：hash 未变 ⇒ 复用；变了 ⇒ 重析。`remove_file` ⇒ 集合移除。
- Index 每次从当前文件集合**全量重建**（BTreeMap 确定性；300 文件 317ms 含全部 IO——重建成本实测可接受，无第二套缓存，§39 红线）。
- Ghost symbols 结构性不可能（索引来自当前集合——测试覆盖 modify/delete）。
- 持久化：无（§38：无跨重启需求，SQLite 不落 AST；后续 Snapshot 域按需评估）。

## 5. M21/M22 Integration（§23-§26/§33-§34）

- L1 → Core：`symbols_to_intel` —— 符号=Entity(kind=symbol, metadata: kind/namespace/modifiers/generic_arity/attributes/doc)；`defines`（file→symbol）+ `contains`（parent→child）关系；每声明一条 SourceRange Evidence（cap 4/symbol）；聚合事实 symbols_total/symbols_{kind}；Origin=Fact、Freshness=Fresh、Provenance 带 parser/extraction 版本（§61 四版本分离）。
- M22→M23：`prism intel --domain symbols` 直接消费 L0 bundle 的文件清单（metadata.language=="csharp"）——**不重扫文件系统**（§三十四）。
- CLI：`prism intel --domain symbols [--search name] [--json]`——search 为**符号搜索**（只查声明名称，非全文搜索，§58）；GUI/MCP 维持既有消费面（§36/§37：无第二套，待对应入口需求）。

## 6. Security（§52/§一百零一）

纯内存解析（source bytes → tree → symbols）；无 shell/进程/网络；文件读取上限 4 MiB/文件；解析失败⇒诊断不崩溃；无秘密面（源码不脱敏进 bundle——源码本身即用户数据，输出契约沿中央 redaction 于传输层处理）。
