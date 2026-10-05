# PRISM_2_0_INTELLIGENCE_CORE.md

> 2.0-M21 产物（M21 §66/§67）。实现 = 新 crate `prism-intel`（commit b834aaf）。
> 上游契约：PRISM_2_0_EVIDENCE_CONTRACT / ADAPTER_CONTRACT / ARCHITECTURE（M20）——本文不重复，只记录落地形态与差异。

## 1. Core 的职责

统一承载工程智能的**表达层**：Entity / Fact / Evidence / Relation / Finding + Origin / Confidence / Provenance / Freshness + AnalysisContext + IntelOutcome（部分结果诚实表达）+ IntelBundle（确定性集合）。垂直切片：依赖域（M21）+ L0 结构域（M22，第一个主要 Fact Producer）+ C# 符号域（M23）+ Godot 域（M24）+ L2 语义域（M25，`prism-csharp::semantic_to_intel`）+ Impact 域（M26，`prism-intel::impact::analyze`——EvidenceGraph 反向遍历，Direct/Transitive/Potential/Unknown）。

## 2. Core 的非职责

- **不产生事实**——分析器（L0/L1/L2/adapter/health…）产出，Core 定义表达（M21 §四）。
- **不持久化**——SQLite 归 prism-store；Snapshot 归 prism-snapshot；AnalysisContext 是 transient 运行期对象（§42）。
- **不缓存**——"是什么"（Core）与"可否复用"（cache）分离（§39）。
- **不做 AI**——无 LLM/embedding/向量；未来 AI 只是 Consumer（§68/§69：AI can interpret, cannot silently redefine facts）。
- **不执行**——无 shell/进程/网络/文件写入；secret 沿中央 redaction，Core 序列化不新增泄漏面（§48/§49）。
- **不知道 Godot/C#/任何 adapter**——EntityKind 开放注册，Core 内置种类仅跨 adapter 通用项（§51）。

## 3–7. Entity / Fact / Evidence / Relation / Finding（落地形态）

| 概念 | 类型 | 身份 | 关键点 |
|---|---|---|---|
| Entity | `Entity{id,kind,display_name,location,scope,metadata}` | `EntityId = "{kind}:{fnv1a16(stable_key|kind|canonical_key)}"` 项目 scope 唯一、跨运行稳定；display name 非身份 | Location 六态（Unknown/FileOnly/Range/Generated/External/Virtual）；res:// 由 adapter 映射，不进 Core |
| Fact | `Fact{fact_type,subject?,value,origin,confidence+reason,freshness,evidence,observed_at,provenance}` | 内容寻址 `fact:{hash16}` | FactValue 结构化（Bool/Int/Float/Str/Json）；项目级聚合事实 subject 显式 None |
| Evidence | `Evidence{kind,path?,range?,reference?,note}` | 内容寻址 `ev:{hash16}`，同证据自然去重 | 12 种 + Other；`Evidence::unavailable(reason)` 显式表达无证据（§58） |
| Relation | `Relation{relation_type,source,target,origin,confidence,evidence,metadata}` | 内容寻址 `rel:{hash16}`（覆盖方向） | 方向显式（A→B≠B→A）；target = Resolved(EntityId) \| Unresolved{specifier,reason?}——禁造假实体（§59） |
| Finding | `Finding{finding_type,severity,status,title,description,origin,confidence,target?,evidence,remediation?}` | 无行号稳定 id `find:{hash16}`（沿 1.0 finding_fingerprint 先例） | Severity/Confidence/Status 三轴分离；复用 prism-core::Severity；remediation 可选 |

## 8–10. Origin / Confidence / Provenance

- Origin = `fact | inference | hypothesis`（serde snake_case，稳定输出；类型系统层面不可混用）。
- Confidence = `high|medium|low|unknown` + **ConfidenceReason**（direct_observation / multiple_compatible_facts / heuristic_inference / insufficient_evidence / custom）——禁止无来源赋值；`from_core()` 映射 1.0 三级。
- Provenance = `{producer, producer_version, observed_at}`（analyzer schema 版本，非 crate 版本）。

## 11. Freshness

四态 `fresh|stale|invalid|unknown`，是 Fact 的显式字段；"有缓存" ≠ "有效"（§24）。所有权=数据生产者（M20 §33 契约的落地形态）。

## 12. AnalysisContext

`{project: ProjectIdentity, scope: FullProject|Paths|Domain, requested_capabilities, observed_at, cancel: CancellationToken}`。运行期对象、不序列化；长操作接受取消（§44），取消 ⇒ IntelStatus::Cancelled + 已完成部分（不伪装 Complete，有测试）。

## 13. Partial Result / Error Model

`IntelOutcome<T>{status: complete|partial|failed|cancelled|unknown, data?, warnings, errors}`：
- partial 不得伪装 complete；cancelled 保留已完成部分（均有测试）。
- **Error（Prism 自己失败）** → outcome.errors / prism_core::PrismError（复用 1.0 错误分类，Core Error → CLI 退出码映射在 prism-cli）。
- **Diagnostic（项目的问题）** → bundle 的 Finding / outcome.warnings——依赖解析诊断进 warnings 且状态保持 Complete（边界有测试与 CLI 实测佐证）。

## 14. Identity

见上表；禁止 display name/行号/绝对路径作通用身份（有测试：同名不同文件 ≠ 同实体；改名不塌缩）。stable_key = canonical root 的 fnv1a16（与 M20 §32 canonicalize 口径一致）。

## 15. Project Isolation

ProjectIdentity 进 bundle、EntityId 派生含 stable_key——不同项目的同 key 实体必然不同 id（有测试 `project_isolation_different_roots_different_ids`）。跨项目数据结构上无法意外混用。

## 16. Serialization

serde；字段名稳定（snake_case）；枚举 snake_case 字符串；集合 BTreeMap 按 id 确定性遍历；未知字段容忍（无 deny_unknown_fields，有测试）；`INTEL_SCHEMA_VERSION = 1`（模型版本，与 analyzer 版本分离 §61）。round-trip 有测试。

## 17. Legacy compatibility

策略 = **additive first**（M21 §32/§33）：Core 是增量视图；`prism graph` 旧 JSON 契约零改动；无 Legacy DTO 删除。后续域（health finding→Core Finding、context→Fact/Evidence）按同一模式逐域迁移（落点见 MILESTONE_MATRIX：M26/M28）。

## 18. 第一条垂直切片（§63/§34）

`prism intel <path> [--json]`：`DependencyService::inspect_graph`（既有 analyzer）→ `dependencies_to_intel` → IntelOutcome/IntelBundle → CLI。**Prism 仓库自举实测**：82 实体 / 175 关系 / 4 聚合事实 / 176 证据 / 71 条未解析导入诊断进 warnings（Complete 状态——项目问题≠分析失败）。Core 被生产代码真实使用，非"没人用的类库"。

## 19. GUI / CLI / MCP Compatibility

- CLI：既有命令零改动（diff 仅新增 Command::Intel 变体 + dispatch）。
- GUI：不感知 Core（无新 IPC）；快照/上下文等能力无回归（全量测试绿）。
- MCP：不存在（FACT，M20 记录）⇒ 本节 N/A；Core 契约已按 read-only 边界设计（无写通道、无执行面），未来 MCP 建成时直接消费 bundle 序列化（§37 机器可消费已满足）。

## 20. Future Adapter integration

Adapter（M23 C# / M24 Godot）→ 产出 Entity/Relation/Evidence（EntityKind::other("godot.scene") 等开放注册）→ IntelBundle。Core 契约不预设任何 adapter 细节；Capability 调度（ADAPTER_CONTRACT §2）在 M24 接入时经 AnalysisContext.requested_capabilities 落地。

## 与 M20 契约的差异说明（代码为准）

1. EVIDENCE_CONTRACT 提出的 `EvidenceAnchor` 已定义但切片未用到（保留，供未来 relation→多主体证据引用）。
2. Confidence 的 `DirectlyObserved/...` 四级建议落地为 **Confidence 枚举四级 + ConfidenceReason 来源说明**——语义等价，枚举更利于确定性序列化。
3. M20 ARCHITECTURE 预留的 crate 命名定为 `prism-intel`（另一候选 prism-core 内模块被否：保持 core 精简、依赖方向更清晰）。
