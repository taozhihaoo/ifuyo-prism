# PRISM_2_0_MILESTONE_MATRIX.md

> 2.0-M20 产物（M20 §28）。2.0 里程碑依赖矩阵。命名约定：文档文件名 M20–M215；审计/提交/与 1.0 并提时写 `2.0-Mx`。

## 1. 主链（依赖顺序）

```text
2.0-M20 Baseline（本里程碑，✅ 完成）
  ↓ 提供架构基线/契约五件套/遗留台账
2.0-M21 Intelligence Core（✅ 2026-10-05 完成——prism-intel crate + `prism intel` 切片）
  ↓ 依赖：Evidence/Identity/Freshness 契约（本文档套件）；不依赖具体实现
2.0-M22 L0 Structural（✅ 2026-10-05 完成——prism-l0 crate + `prism intel --domain structural|all`）
  ↓ 依赖：M21 Core 模型；复用 prism-project/git/env
2.0-M23 L1 Symbol Engine（✅ 2026-10-05 完成——prism-csharp（tree-sitter-c-sharp）+ `prism intel --domain symbols`；Python 1.0 管线 KEEP 并行）
  ↓ 依赖：M21；M22 的 File Identity
2.0-M24 Godot Adapter（✅ 2026-10-05 完成——prism-godot crate + `prism intel --domain godot`）
  ↓ 依赖：M21 契约 + M22 L0 + M23 的 Symbol 接口（Script→Symbol 层可后接）
2.0-M25 C# L2 Semantic（✅ 2026-10-05 完成——prism-csharp semantic.rs 作用域规则解析 + `prism intel --domain semantic`）
  ↓ 依赖：M23 L1；后端选型（Roslyn 环境依赖须显式决策）
2.0-M26 Evidence Graph & Impact Intelligence（✅ 2026-10-05 完成——prism-intel graph/impact + `prism intel --domain impact`）
  ↓ 依赖：M21 Core + M23/M24 关系源
2.0-M27 Context Intelligence 2.0（✅ 2026-10-05 完成——evidence collector + `prism context --impact-aware`）
  ↓ 依赖：M26 关系图；MCP 候选承载点
2.0-M28 Health Findings 2.0（✅ 2026-10-05 完成——prism-health2 crate + RuleRegistry 四类规则 + `prism health2 [--baseline]`；契约见 PRISM_2_0_HEALTH2.md）
  ↓ 依赖：M21 Finding/Evidence 契约（达成）；M23-M25/M24 事实注入（达成）
2.0-M29 Dependency/Security/Environment（✅ 2026-10-06 完成——prism-supply crate + 6 条供给规则 + `prism supply [--advisories] [--policy] [--sbom]`；契约见 PRISM_2_0_M29.md）
  ↓ 依赖：M21 契约（达成）+ M28 Registry（达成）+ M9 env 探测（达成）
2.0-M210 Debug Intelligence（✅ 2026-10-06 完成——prism-debug crate + 3 条 Debug 规则 + `prism debug [--source] [--baseline]`；契约见 PRISM_2_0_DEBUG.md）
  ↓ 依赖：M27（达成——Symbol 链实为 M23/M24 直连）+ M28 Registry（达成）+ M29 redaction（达成）
2.0-M211 Git Intelligence & Evolution（✅ 2026-10-06 完成——prism-evolution crate + prism-git 4 只读扩展 + git 子命令族 + commit-impact 切片；契约见 PRISM_2_0_GIT.md）
  ↓ 依赖：M21/M22（达成）+ M23/M26 change_mapping/impact（达成）+ M28 生命周期（达成）
2.0-M212 Visualization（✅ 2026-10-06 完成——prism-viz 投影 crate + get_visualization_graph Tauri + VizGraphPanel GUI + `prism viz`；契约见 PRISM_2_0_VIZ.md）
  ↓ 依赖：M26/M27/M211 只读查询面（达成——纯消费，未反向改 Core）
2.0-M213 API/Webhook Debugging（✅ 2026-10-06 完成——prism-api crate + CLI api 命令族 + health2 Api 规则；契约见 PRISM_2_0_M213_API_WEBHOOK.md）
  ↓ 依赖：M21 数据模型（达成）+ M29 redaction（达成）+ 威胁建模（SSRF guard + localhost-first 已实测）
2.0-M214 Developer Deploy（✅ 2026-10-06 完成——prism-deploy crate + CLI deploy 命令族 + 垂直切片实测；契约见 PRISM_2_0_DEPLOYMENT.md）
  ↓ 依赖：M213 前置（达成——M213 API 可为 health-check provider）；威胁建模（SSH=Unsupported 如实；Local transport 真实路径；路径逃逸/凭证边界测试锁定）
2.0-M215 Final 2.0 Audit（✅ 2026-10-06 完成——全量回归 804 Rust+61 vitest/clippy 0/fmt 0/release 二进制 E2E 实测/断链审计无孤儿/Final Gate 清单核对/Release Decision=READY_WITH_KNOWN_LIMITATIONS；报告见 docs/audits/2.0-M215-final-audit.md）
  ↓ 依赖：全部（达成——M20-M214 全部 Gate PASS）

**2.0 主线完成。** 后续候选（2.1）：MCP stdio server/2.0 域 SQLite 化/SFTP transport/GUI Deploy View/https 观测/增量缓存等价。
```

## 2. 依赖纪律

1. **契约依赖 > 实现依赖**：M24/M26/M27 等只能依赖 M21 冻结的契约（EVIDENCE/ADAPTER/Identity），不得依赖其内部实现细节——实现可重构，契约需版本化。
2. **禁止消费者反向驱动 Core**：M212 Visualization 发现需要新数据 ⇒ 提需求给 Core 迭代，不得在 Visualization 内自建图计算（§三十）。
3. **可并行组**：M28/M29/M211 互不依赖，可在 M21+M22 落地后并行推进。
4. **可裁剪组**：M213/M214 不在 §五十三 完成定义的关键路径上；排期紧张时最后做或砍。

## 3. 每阶段完成定义（对齐纲领 §48）

每个 2.0-Mx 结束必须产出：Final Report（38 节模板，M20 已示范）+ **Next Milestone Preconditions** 节 + Acceptance Gate PASS 证据（命令实测，禁止"文档写完=完成"）。

## 4. M21 进入条件（M20 §60 交付）

1. 本契约套件五份（BASELINE/ARCHITECTURE/DATA_FLOW/MIGRATION/EVIDENCE+ADAPTER_CONTRACT）入库并被接受；
2. 1.0 遗留台账清零 P0/P1（FACT：现存 P0=0、P1=0，见 BASELINE §15）；
3. 全量回归绿（529 Rust + 55 vitest、clippy 0、fmt 0，M20 末复测）；
4. 性能无未解释回退（snapshot 回退已定位为域捕获有意成本）；
5. M21 的核心交付边界：统一 Entity/Fact/Relation/Evidence/Finding/Confidence 模型 + 第一条 End-to-End slice + 三面同源回归——**不得提前实现 M22+ 的具体分析能力**。
