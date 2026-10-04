# Prism M11a 阶段审计（Snapshot Capture and Storage — Part 1/2 Exit Gate）

日期：2026-10-03 · 依据：`Spec/Prism M11a …md`【阶段性完成门槛】
前置审计：`docs/audits/M11a-baseline-audit.md`（§188–§190 先审计后修改）
基线：`cargo test --workspace` **514 passed / 0 failed**（62 个测试二进制）+ vitest 45 passed，clippy **0**。

## 门槛逐项核对

| 门槛项 | 状态 | 证据 |
| --- | --- | --- |
| P0 要求全实现：ProjectSnapshot 模型 / 域 fingerprint / Canonical Serialization / prism-store（schema/migrations/事务/原子写/并发锁）/ 捕获编排与去重 / `prism snapshot` + `prism history` | ✅ | prism-snapshot/src/model.rs（§7 十域指纹 + §6 组合）；store.rs CanonicalJSON（§15，key+数组排序）；prism-store（§25 schema、§64 schema_migrations 顺序迁移、§65 事务原子写、busy_timeout §144）；capture.rs（§8 身份、§17 去重、§123 result） |
| §196 Restart Persistence 真实通过 | ✅ | snapshot_cli.rs::snapshot_survives_process_restart——进程 A capture、独立进程 B history 断言同 id 可见 |
| 本篇范围测试全绿 | ✅ | prism-store 3、prism-snapshot 8（§158 顺序无关 / §205-206 指纹变化 / §159 去重+force / §197-§199 多项目隔离 / §114 真实 git 只读）、context_cli 5（§196 restart / §17 unchanged 文案 / §197-§199 / §220 db-path / §217 reproducible） |
| cargo test + clippy 0 + fmt | ✅ | **实测（fmt 补验）**：`cargo fmt --check` 首次运行 exit 1——548 个 hunk、103 个文件跨 M1–M11 累积（fmt 在此之前从未纳入门槛）；`cargo fmt` 全仓格式化后（无语义改动，独立 style 提交）`cargo fmt --check` exit 0；复测 `cargo test --workspace` 514+45 全绿、clippy 0 |
| Git Safety（§114） | ✅ | 实测：对 Prism 仓库自身 capture 前后 HEAD + status 完全一致；capture.rs 全为 fs::read/git 只读 |
| 【M11a 阶段审计】FACT/HYPOTHESIS/INFERENCE | ✅ | docs/audits/M11a-baseline-audit.md（实施前输出） |
| 本地 commit `feat: add snapshot capture and storage (M11 part 1/2)`，不 push | ✅ | 见 git log（不 push） |

## 关键实现事实（FACT）

- **prism-store**：rusqlite 0.37 bundled（构建期拉取，运行时 100% local §83）；
  `schema_migrations` 顺序迁移 + 失败即停（§64 P0）；三表 schema（projects/
  snapshots/snapshot_domains，§25）+ 双索引（§142）；`insert_snapshot_atomic`
  事务（§65/§66 无半成品）；busy_timeout（§144）；`SnapshotStore` trait +
  InMemory 测试替身（§226）；SQL 只在本 crate（§228/§299）。
- **DB 位置**（§21）：`%APPDATA%\Prism\prism.db`（Win）/ `XDG_DATA_HOME|~/.local/
  share/Prism/prism.db`（Unix）；`PRISM_DB_PATH` 显式覆盖（§220，测试用）。
  **不入项目仓库**（§275）。
- **prism-snapshot**：ProjectSnapshot（§7 全字段：十域指纹/source_versions/
  counts/limitations/note）；SnapshotDomainState（§13：Available/NotCaptured/
  Unavailable/Stale/Partial/NotApplicable + freshness；Unavailable/Stale ≠
  Failed）；canonicalize（§15：BTreeMap key 序 + 数组字符串键排序，§158 顺序
  无关指纹测试）；fnv1a/组合指纹（§6）；SnapshotCaptureResult（§123 persisted
  标志，§120/§122 不吞错误）。
- **Capture**：resolve_project_identity（canonical root 首见注册 + last_seen，
  §8/§9——§198 同名项目 root 不同则身份不同，有测试）；basis GitRevision/
  WorkingTree（§5，clean tree 实测）；M1 structure 规范化 + 十域装配（未注入
  域 ⇒ NotCaptured 诚实降级，§108/§109 不自动跑 verification）；去重（§17
  "Snapshot unchanged. / Existing snapshot:"，--force ⇒ 新 observation §18）。
- **CLI**：`prism snapshot <path> [--force] [--note] [--json]`、
  `prism history <path> [--json] [--limit]`；store 路径 §21/§22 同源；
  Manual Snapshot First——无后台自动快照（§146）。

## P1（带入 M11b，非阻塞）

1. M2–M10 域捕获（code/architecture/change/impact/health/verification/
   environment/context_health）当前 NotCaptured——服务组装点已留
   （DomainFacts.data 注入即 Available），M11b compare 需要 which-domain 时
   逐域接线。
2. §252 自动 retention、§143 VACUUM——默认手动（§63），未实现自动清理。
3. §137 Snapshot Tags（除 note 外）、§92 baseline 标记系统——v1 仅 note。
4. §255 schema backup before migration、§256 migration logging（迁移日志
   目前只在失败时报错）。
5. §257 存储加密——明确不做（§257 原文允许不做，local-first）。
6. GUI history 页（§193 P1-1）——CLI 先行，GUI 归 M11b。

## 红线自查

- §114 不修改工作树：实测 HEAD+status 不变；代码无写路径。
- §116 delete 只动 SQLite。
- §118 不用 git remote 做身份。
- §146/§147 无自动快照。
- §83 offline / §84 无 AI / §257 无加密需求。
- §296/§297/§298/§300：M1–M10 CLI/JSON 未破坏；Snapshot 领域对象纯净；
  Data Contract 四对象中 Snapshot 已落地，Diff/Timeline/Evolution 归 M11b。

结论：**M11a 阶段性完成门槛通过**，按指令以
`feat: add snapshot capture and storage (M11 part 1/2)` 收口（不 push），
随后执行 M11b。
