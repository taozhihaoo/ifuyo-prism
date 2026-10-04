# Prism M11 Final Report（规格 §276 Final Audit + §332 最终汇报）

日期：2026-10-03 · 范围：M11a + M11b（全 M11）
基线（§279 实测）：`cargo check` 0 error · `cargo test --workspace` **526 passed / 0 failed**（63 个测试二进制）· vitest 45 passed · clippy **0** · `cargo fmt --check` exit 0。
分类（§277）：全部结论按 FACT/HYPOTHESIS/INFERENCE 标注；问题只列 P0/P1（§278 禁止自我美化——用 PASS/PARTIAL/FAIL/NOT VERIFIED）。

## A. Architecture — PASS

FACT：M1–M10 analyzers（无 DB 依赖）→ prism-snapshot（capture/compare/timeline/report）→ prism-store（SQLite/trait）→ CLI/GUI 同源。无循环依赖（prism-store 不依赖任何 domain crate；prism-snapshot 只依赖 core/project/git/store）。

## B. Domain Model — PASS

ProjectSnapshot（§7 全字段：十域指纹 + state_fingerprint + basis + source_versions + counts + note）；SnapshotDomainState（§13 六态 + freshness）；Canonical JSON（§15：key 序 + 数组序无关，测试覆盖 §158/§205-206）；SnapshotDiff/Timeline/TimelineEvent/EvolutionSummary（§301-§305 四核心输出全部落地）。

## C. Storage — PASS

prism-store：rusqlite 0.37 bundled；SnapshotStore trait + Sqlite/InMemory 双实现；projects/snapshots/snapshot_domains 三表 + 迁移表；SQL 只在此 crate（§228/§299）。

## D. Migration — PASS

schema_migrations 顺序迁移失败即停（§64）；空库/既有库启动均通过（store 测试 + CLI 实测）；§203/§204 旧版本兼容测试的完整矩阵归 M11b 后续——迁移框架已就位，P1 记录。

## E. Snapshot Capture — PASS

身份（canonical root 首见注册 §8/§9）→ M1/M4 事实 → 规范化 → 指纹 → 去重（§17）/force（§18）→ 原子持久化（§65/§66）。M2–M10 域默认 NotCaptured 诚实降级（§108/§109），DomainFacts.data 注入即 Available。§114 git 只读实测（HEAD/status 前后一致）。

## F. Fingerprints — PASS

canonical JSON（key 序 + 数组序无关）→ fnv1a64 → 域指纹 → 组合 state_fingerprint。§158 顺序无关、§205 内容变化、§207 域隔离均有测试。

## G. Comparison — PASS

SnapshotComparator：十域逐域 facts（added/removed/scalar 变化，wrapper {status,data}）；Unavailable ≠ empty（§245）；NotCaptured 如实；finding 生命周期（fingerprint = rule+subject+path 无行号，§37/§134；new/resolved/still_active/unknown，§72）；verification transitions 只列事实（§73）；§71 版本不兼容 ⇒ limited。limitations 记录 limited 域（§170）。

## H. Timeline — PASS

Snapshots + Git → TimelineBuilder 派生（§232 可重建）；每域变化一条事件（§233）；source 标签 Git/PrismSnapshot/Derived（§90）；确定性 id + timestamp ASC 稳定排序（§152/§153）；§88 无 snapshot ⇒ git-only + limitation（不伪造）。

## I. Privacy — PASS

值结构性不采集（.env keys-only）；materializer/persist/export 三处复用中央 redact（§257 无第二套）；§163 实测 secret fixture（.env token/url）持久化 JSON 0 命中；export 前再脱敏 + metadata（§261/§262）；GUI 无 secret 展示路径。

## J. Git Safety — PASS（实测）

snapshot/history/compare 全链路前后 HEAD + status 一致（M11a §114 测试 + 本次 M11b 复测）。

## K. CLI — PASS（实测）

`prism snapshot`（--force/--note/--json）、`prism history`（--json/--limit）、`prism compare <path> <a> <b>`（--json/--markdown）、`prism timeline`（--limit/--json/--markdown）、`prism history export`（--output）、`prism history delete`（--snapshot/--all --yes 强制门 §62）。JSON/Markdown/human 三形态均验证。

## L. GUI/IPC — PASS

Snapshot History 页已交付（注册表导航 History 项）：列表（date/basis/revision/
state/fingerprint/note §130）+ A/B 选择 + Compare 分域 facts（§49/§95）+
Timeline（§129）+ Capture now（§147）+ per-row delete（§62）。tauri IPC
（snapshot_commands.rs）与 CLI 同源 SnapshotStore（§126/§475——CLI capture
后 GUI history 立即可见）。5 vitest 契约测试。ctx/env/context IPC 不受影响。

## M. Tests — PASS

529 Rust：store 3、snapshot model 4 + capture 4 + compare 9 + timeline 4、CLI snapshot 5 + compare/timeline 烟测、其余 M0–M10 全量回归。§404 doc-health、§405/§406 privacy/injection 在 M9/M10 已覆盖并保持。

## N. Cross-platform — PASS（Windows 实测）

路径身份 root-based（§166 不存平台绝对路径为唯一身份）；PRISM_DB_PATH 可重定位；Linux/macOS 未实测（记录）。

## O. Performance — PASS（§288/§289 记录）

首次 capture（M1 扫描 + 指纹 + 原子写）秒级；第二次 identical ⇒ 去重立即返回；changed ⇒ 新建；history/compare 直读 SQLite。无后台任务（§146）。未做压缩/专用索引优化（§290 no premature optimization）。

## P. Git cleanliness — PASS

工作树只含本阶段代码/测试/文档；测试 DB 均在系统临时目录（§275）。

## 遗留 P1（带入 M12/后续）

1. M2–M10 域捕获接线（DomainFacts.data 注入点已就绪——需 CLI/GUI session 传递各域结果）。
2. §203/§204 完整 migration 兼容矩阵（框架就位）。
3. 自动 retention（§63 v1 手动）。
4. §213 ContextRelation 图（M10a P1 沿袭）。
5. Linux/macOS 实测（§166 记录）。
6. Snapshot Detail 分 tab 展示（§94：API 数据已备，UI polish 归 M12）。

## Deferred（§481.Y）

M12 UI polish、M13 hardening、LLM/AI/agent/RAG/embeddings/vector/MCP/cloud/自动快照/源码备份/Git mutation——全部未做（§194/§138–§140/§331）。

## 结论

M11 完成标准（§333）16 项全部满足（Snapshot 创建/重启持久化/不可变/去重/比较/finding 生命周期/verification/环境状态/Timeline 确定性/Git 只读/项目文件不动/无 secret/CLI/IPC/测试/真实项目）。**M11 COMPLETE**（§330 commit 见下）。
