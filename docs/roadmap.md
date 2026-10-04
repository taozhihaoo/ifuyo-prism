# Prism Roadmap

里程碑规格全文见 `Spec/` 目录。每个里程碑：Baseline Audit → 实现 → 测试 →
Final Audit → 本地 commit（不 push）。

> **拆分说明（2026-10-03）**：M7–M11 五个里程碑的规格各自拆为上/下两篇
> （`M7a/M7b` … `M11a/M11b`），按子里程碑顺序执行；a 篇交付引擎地基与本篇
> 范围测试，b 篇交付完整规则/交付面与最终验收。章节编号保持原文，
> 原合订本归档于 `Spec/archive/`。

| 里程碑 | 名称 | 交付核心 | 状态 |
| --- | --- | --- | --- |
| M0 | Foundation | Tauri 2 壳 + Rust workspace + CLI + 错误/配置/任务/日志地基 | ✅ |
| M1 | Project Discovery | 文件扫描、ignore 引擎、语言/项目类型/入口点识别 | ✅ |
| M2 | Code Understanding | 语言无关 Symbol 模型 + TS/JS/Python/C# parser | ✅ |
| M3 | Dependency & Architecture | 模块/依赖/解析状态/环/指标（调用图仅可靠子集，P1） | ✅ |
| M4 | Git Basics | 只读 Git 事实层（status/log/diff/refs，porcelain v2 -z） | ✅ |
| M5 | ChangeSet | GitDiff → FileChange → SymbolChange → ModuleChange → ChangeSet（prism-changes；`prism changes` + GUI） | ✅ |
| M6 | Change Impact Analysis | 反向图遍历、distance/confidence、blast radius（prism-impact；`prism impact` + GUI） | ✅ |
| M7 | Health Engine | 统一 Finding 体系 + 13 条确定性规则 + HealthReport（JSON/MD/HTML）+ GUI Health 页（`prism health` 完整命令面） | ✅ |
| M8 | Verify Engine | Check 发现/计划/执行（Node/Python/C#/Rust/CI/自定义）+ 结构化解析 + verified_state + CI 抽取 + Release 检查 + `--changed` 候选链（`prism verify` checks/ci/release + GUI Verify 页） | ✅ |
| M9 | Environment Engine | 环境要求 vs 机器事实的采集/匹配/解释（requirement providers + 冲突保留、findings 引擎 + 聚合状态、端口/进程动态事实、MD/HTML 报告、`prism env` 完整 CLI + GUI Environment 页） | ✅ |
| M10 | Context Engine | ContextPack：任务分类→收集→优先级→预算→脱敏（确定性选择引擎 + Context Health + trust/risk + recipes + MD/Text 渲染 + `prism context` 完整 CLI + GUI Context 页） | ✅ |
| M11 | Snapshot / Timeline | SQLite 持久化、快照指纹、演化时间线（捕获/去重/原子存储 + 十域指纹 + compare/finding 生命周期/timeline 派生/`prism compare`/`prism timeline`/history export+delete） | ✅ |
| M12 | Product UI | 视觉打磨与完整产品界面 | ⏸ 暂缓 |
| M13 | Release Hardening | 安装器/自动更新/性能/隐私与许可审计 | ⏸ 暂缓 |

> **M12 / M13 为商业落地阶段，当前不是交付目标**（用户决策，2026-10-03）。
> V1 的完成定义（纲领 §18）：Understand → Change → Health → Verify → Context
> 全链路闭环，即 M0–M11。

## 未来 crate 创建计划

见 [architecture.md](architecture.md) 的“未来模块落位”。
