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
| M7 | Health Engine | 统一 Finding 体系 + 首批高确定性规则 | |
| M8 | Verify Engine | build/test/lint 编排与结果归一 | |
| M9 | Environment Engine | 环境要求 vs 机器事实的采集/匹配/解释 | |
| M10 | Context Engine | ContextPack：任务分类→收集→优先级→预算→脱敏 | |
| M11 | Snapshot / Timeline | SQLite 持久化、快照指纹、演化时间线 | |
| M12 | Product UI | 视觉打磨与完整产品界面 | ⏸ 暂缓 |
| M13 | Release Hardening | 安装器/自动更新/性能/隐私与许可审计 | ⏸ 暂缓 |

> **M12 / M13 为商业落地阶段，当前不是交付目标**（用户决策，2026-10-03）。
> V1 的完成定义（纲领 §18）：Understand → Change → Health → Verify → Context
> 全链路闭环，即 M0–M11。

## 未来 crate 创建计划

见 [architecture.md](architecture.md) 的“未来模块落位”。
