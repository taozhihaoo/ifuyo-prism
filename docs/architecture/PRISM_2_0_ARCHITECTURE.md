# PRISM_2_0_ARCHITECTURE.md

> 2.0-M20 产物（M20 §45）。确定 2.0 目标架构与边界；以真实工程为底，不预设万能框架（§52）。

## 1. Product Boundary

Prism 2.0 = 本地优先的软件项目智能工作台：统一工程事实/证据/关系/变化/历史模型，服务理解、变更、健康、验证、上下文与（后续的）调试、可视化、部署。不做：AI Chat、云服务、企业 SaaS、遥测、默认联网。

## 2. Architecture Principles

1. Code is Source of Truth：文档漂移以代码 + 测试 + 运行结果为准。
2. Evidence First：结论携带证据/来源/置信度；FACT / INFERENCE / HYPOTHESIS 不得混淆。
3. 无虚假精确：输出计数与事实，不输出无公式的百分比/风险分。
4. Adapter 边界：语言/引擎特有逻辑（Godot/C#）只存在于 adapter，不进 Core。
5. 最大复用、最小破坏：1.0 域 crate 是资产，2.0 演进而非重写。
6. 本地优先：项目数据默认敏感；任何新 IO 面（MCP/Webhook/Deploy）单独威胁建模。

## 3. Layering（依赖方向：上→下，禁止反向）

```text
Interface Layer      GUI(React/Tauri) / CLI(clap) / MCP(待建) / JSON Contract
        ↓
Application Layer    src-tauri commands + CLI commands + Session 状态机
        ↓
Intelligence Core    Facts / Evidence / Relations / Findings / Confidence / Snapshot / History
        ↓            (M21 起逐步实体化；今天以各域模型的共性契约为形态)
Analysis Engines     L0 结构(prism-project/git/env) | L1 语法(prism-parser/analysis) | L2 语义(M25)
Project Adapters     Godot(M24) / C#(M23/M25) / TypeScript / Python / 未来
        ↓
Persistence          prism-store(SQLite) / fs / git(只读)
```

- FACT：1.0 现状已符合该方向——prism-core 不依赖 interface；域 crate 无环。
- DECISION：2.0 新增代码必须落入该分层；Intelligence Core 落地为 prism-core 内新模块或新 crate `prism-intel`（M21 开工时定，M20 不建）。

## 4. Core Domain

Core 拥有且仅拥有通用概念：Identity（实体规范 id）、Fact、Evidence、Relation、Finding、Confidence、Origin、Freshness、Snapshot、History。
Godot/Scene/Node/Signal 等 adapter 概念一律不得进入 Core 类型（对齐 2.0 提示词 M21 §51 的"禁止 Core 变成 Godot Core"）。adapter 特有数据进 metadata，收编为正式字段的判据：明确语义 + 查询需求 + 稳定契约 + 跨 adapter 共性。

## 5. Analysis Engine

- L0 Structural：文件/目录/项目/manifest/配置/git/日志/环境。不需要语言语义。1.0 已具备（RETAIN/EXTEND）。
- L1 Syntactic：AST/Symbol/Namespace/Class/Struct/Interface/Method/Function/Property/Field/Enum。现仅 Python；C# 为 M23 第一优先新建。
- L2 Semantic：Reference/Call/Type/Inheritance/Implementation/Generic/Project Reference。M25 起，以 Exact/Probable/Unresolved 置信度诚实交付，不做 IDE 级承诺。
- 边界规则：字符串搜索/文本匹配不得称为 semantic reference（M20 §十二）。

## 6. Project Adapter

契约见 PRISM_2_0_ADAPTER_CONTRACT.md。Core 通过 capability 查询适配器（supports_l1? supports_l2?），不硬编码语言分支。第一优先：Godot、C#。

## 7. Application Services

- FACT：现状 = src-tauri 7 个 command 模块（49 IPC）+ prism-cli 14 命令，两者同调 Domain Service（§475 同源已在 snapshot/env/verify/context 落实）。
- 规则：Application 层不得复制业务逻辑；GUI/CLI/MCP 三接口只做传输与呈现。
- 2.0 决策：M21 起新 Intelligence 能力必须以 Core Service 暴露，三接口同日接线，禁止"先 CLI 后 GUI 补"造成分叉。

## 8. Interface Layer

CLI（developer automation）、GUI（interactive exploration）、MCP（AI/external，read-only，待建）、JSON（稳定机器契约）。JSON schema 变更 = 契约变更，需版本化（沿用 SNAPSHOT_SCHEMA_VERSION 模式）。

## 9. Persistence

FACT：prism-store 独占 SQLite（4 表 + schema_migrations 顺序迁移）。2.0 原则：Intelligence Core 的持久化必须走 prism-store（新增表 = 新 migration，事务原子写沿用）；禁止旁路 SQLite 连接。快照/历史表保持向后兼容（详见 MIGRATION）。

## 10. Cache

- 现状 RETAIN：Session 内同源复用（discovery/code-model/graph）；快照指纹去重。
- 2.0 边界：新增分析层必须回答五问（M20 §34）：复用什么 / 重算什么 / 何时 stale / 如何 invalidate / 如何取消。cache ownership = 数据生产者（与 freshness 绑定）；禁止 consumer 私建缓存副本。

## 11. Snapshot

FACT：ProjectSnapshot 十域指纹 + state_fingerprint（fnv1a64）+ canonical JSON + 去重（§17）+ per-project 锁（§145）+ 域数据注入（M11b）。2.0 定位：**版本化项目智能快照**——Intelligence Core 的历史维度载体；M22+ 新域按既有 DomainRow 模式扩展，schema 走 migration。

## 12. Evidence

契约见 PRISM_2_0_EVIDENCE_CONTRACT.md。M20 冻结方向：统一 Evidence 结构（来源/位置/关系/置信度/origin），Impact/Health/Context/Snapshot/Adapter 共用；1.0 各域局部 Evidence（graph Evidence、health Finding.evidence、env 证据链、context TrustLevel）向其收敛（MIGRATE，策略见 MIGRATION）。

## 13. History

两层：Git history（prism-git 只读事实 + M211 演进智能）与 Snapshot history（prism-snapshot + timeline 派生）。2.0 不合并两者，而是让 Evolution（M211）同时消费。

## 14. Security Boundary（沿袭 1.0 + 新增面）

保持：Tauri capability 最小、IPC 白名单、前端无 fs/git/sqlite 直连、中央 privacy + 输出面脱敏、SQL 参数化、verify 命令白名单 + write_policy、Git 只读。
新增面（MCP / M213 Webhook / M214 Deploy）一律先威胁建模再实现：MCP 默认 read-only + path allowlist + size limit + 审计；Webhook 接收面默认 loopback + SSRF 防护；Deploy 凭据不落盘明文。禁止：自动上传源码、默认远程分析、隐式网络发送项目内容。

## 15. Dependency Direction

UI → Application → Domain/Core → Persistence。禁止：Core → UI / Tauri / Godot / 具体 adapter / 具体命令。FACT：现 workspace 无违反者；CI 期（M215 前）可用 cargo 依赖图断言固化（候选任务，非 M20 必须）。

## 16. Future Extension

- 语言扩展 = 新 adapter（Open Kind 注册），Core 不改。
- 域扩展 = 新 DomainRow + 域指纹（既有模式）。
- 接口扩展 = MCP/M213/M214 独立模块挂 Application 层。
- 明确不做（§52）：UniversalPluginFramework / UniversalAnalyzerFramework / UniversalWorkflowEngine 等万能框架。
