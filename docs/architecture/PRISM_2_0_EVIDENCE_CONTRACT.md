# PRISM_2_0_EVIDENCE_CONTRACT.md

> 2.0-M20 产物（M20 §48/§十三/§十四）。冻结 2.0 Evidence 基础协议的**方向与契约**；实现属 M21。M20 只在必要时允许最小类型/interface（§十三），本里程碑未新增代码类型——契约先行。

## 1. 术语与不可混淆原则

- **FACT**：代码/测试/Git/配置/运行结果可直接证明的陈述。携带可直接指认的来源（文件/symbol/位置/命令输出）。
- **INFERENCE**：由 ≥2 个 FACT 按确定规则推出的结论（例：A→B 且 C→A ⇒ C 间接受 B 影响）。必须可追溯其前提 FACT。
- **HYPOTHESIS**：尚未被项目事实充分证明的可能性（例：可能存在运行时动态调用）。必须显式标注为假设，禁止伪装成事实。
- 置信度（Confidence）不是"AI 感觉"，必须来自可指认的来源分级（下 §4）。

## 2. 契约概念（M21 落地的最小类型集）

```text
FactId        稳定 id（fnv1a/uuid 语义沿用 1.0 既有模式）
Origin        FACT | INFERENCE | HYPOTHESIS
Evidence      { source: SourceRef, location?: LocationRef, relation?: RelationRef, note }
SourceRef     git blob | 文件+行 | 命令输出（verify/env） | store 记录 | adapter 声明
Confidence    直接观测 > 强推断 > 弱推断 > 未验证（枚举对齐 1.0 core Confidence Low/Medium/High 映射）
Relation      { from, to, kind, origin, evidence[], confidence }
Finding       { id, rule_id, severity, status, confidence, location, evidence[], origin }
```

- 规则 1：每个重要分析结论（Finding/Impact/Context 项）必须能回答：证据是什么？来源在哪？为什么支持结论？哪个是 FACT 哪个是推断？
- 规则 2：Evidence 一等公民化——可序列化、可随 Snapshot 持久化、可跨接口（CLI/GUI/MCP）无损呈现。
- 规则 3：禁止无 Evidence 的"看起来危险"类输出；禁止凭空百分比（沿用 §六 无虚假精确）。

## 3. 与 1.0 局部模型的收敛方向（MIGRATE 策略）

| 1.0 现状（FACT） | 收敛方式 |
|---|---|
| prism-graph `Evidence`（依赖解析证据） | 直接映射为 SourceRef+relation evidence |
| prism-health `Finding.evidence[]`（What/Evidence/Why） | Finding 结构的超集，字段映射后收编 |
| prism-env 证据链（requirement source / probe / 观测） | SourceRef 增加探测类来源 |
| prism-context TrustLevel 6 层 + Freshness | Confidence 来源分级 + Freshness 契约对齐（§5） |
| prism-snapshot finding_fingerprint（无行号稳定指纹 §37） | FindingId 稳定性规则的既成先例，直接沿用 |

收敛在 M21 完成每域替换/适配；过渡期兼容层允许双形态并存，禁止一次性破坏各域 CLI JSON（§三十八 数据兼容）。

## 4. Confidence 分级（对齐 1.0 词汇）

```text
DirectlyObserved   工具/解析器直接产出，来源可复跑
StronglyInferred   确定性规则推导，前提全为 FACT
WeaklyInferred     启发式推导，存在已知误报面（如字符串匹配的"引用"）
Unverified         尚无足够证据（= HYPOTHESIS 侧）
```

映射：DirectlyObserved/StronglyInferred → core High；WeaklyInferred → Medium；Unverified → Low（沿用 core Confidence 三级，不新增枚举，M21 细化时再议）。

## 5. Freshness（与 Evidence 绑定）

沿袭 M10 ContextFreshness / M11 DomainFreshness 的既有语义，统一四态：`fresh / stale / invalid / unknown`。
判定来源 = 数据所有者的输入指纹（1.0 已有 input_fingerprint/content_fingerprint 先例）。任何新分析层必须声明：何时必须重算、何时可复用、何时增量、谁负责 invalidate（M20 §33/§34 契约，实现随域落 M21+）。

## 6. 验收对齐（供 M21 引用）

- FACT fixture / INFERENCE fixture / HYPOTHESIS fixture 三类测试样例（2.0 提示词 §四十一）从 M21 起强制。
- 任何输出面（CLI/GUI/MCP/JSON）渲染 Evidence 时不得丢弃 origin。
