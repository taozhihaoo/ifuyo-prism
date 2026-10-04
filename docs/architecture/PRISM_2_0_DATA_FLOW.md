# PRISM_2_0_DATA_FLOW.md

> 2.0-M20 产物（M20 §43/§八）。当前真实数据流（FACT 实测）+ 2.0 目标流（DECISION）。

## 1. 现状：GUI 数据流

```text
用户操作 (React)
  → invoke IPC（49 命令白名单：open_project / start_* / get_* / cancel_* / snapshot_* / …）
  → src-tauri commands（Application 层）
  → SessionState（锁保护：project / SessionTask 状态机 Pending→Running→Done/Failed）
  → Domain Services（prism-* crate）
  → PrismStore / fs / git(只读)
  ← 结果 DTO（serde 序列化，camelCase JSON）→ 前端页面
```

- FACT：长任务走 `start_*` + 轮询 `get_*_status` + 可 `cancel_*`（8 个取消命令：scan/inspect/changeset/impact/health/verify/environment/context）。
- FACT：session 内 discovery/code-model/graph 一次产出、多命令复用（同源缓存）。

## 2. 现状：CLI 数据流

```text
prism <cmd> <path> (clap 解析, 退出码 0-6)
  → run_<cmd>（Application 层薄壳：参数校验/服务编排/输出渲染）
  → Domain Service 调用（每次进程冷跑，无跨运行缓存）
  → stdout（text/json/markdown 三态）+ stderr（诊断）
```

- FACT：CLI 与 GUI 无共享进程状态；共享的是 Domain Service 与 SQLite store（§22 同一 prism.db）。

## 3. 现状：快照捕获流（M11b 后）

```text
prism snapshot <path> / GUI snapshot_capture
  → collect_analysis(path, refresh_verification)      [M1–M9 一次跑完]
      discovery → code_model → graph
      → changeset(WorkingTree) → impact
      → health(FullProject)   [M1–M3 失败 ⇒ Err；辅助域失败 ⇒ warnings + 跳过]
      → verification(仅显式刷新；Manual First §147)
      → environment(ES scan)
  → build_domain_data(&inputs) → 七域规范化 JSON（含无行号 finding 指纹 §37）
  → SnapshotCaptureService.capture
      canonical 根（2.0-M20 §32 归一化）→ per-project 锁 §145
      → 结构域 + git 域（只读 §114）→ 十域指纹 → state_fingerprint
      → 去重 §17（force 跳过）→ 事务原子落库 §65
```

## 4. 现状：比较/时间线流

```text
prism compare/timeline / GUI compare_snapshots/snapshot_timeline
  → store.get_domains(两个快照) → wrapper {status,data} 解包
  → compare_domain（标量迁移 + 集合 added/removed §113/§158）
  → finding_lifecycles（无行号指纹 §37）/ verification_transitions
  → timeline：snapshots × git log(只读 200 条) → 派生事件（§233 每域一事件、无伪造 §88）
```

## 5. 现状：持久化流

```text
唯一 SQLite：默认 %APPDATA%/Prism/prism.db（PRISM_DB_PATH 覆盖）
表：projects / snapshots / snapshot_domains / schema_migrations
写路径：insert_snapshot_atomic（事务）、upsert_project、delete_*
读路径：按 project_id / snapshot_id / canonical_root / 指纹
并发：Mutex<Connection> + busy_timeout + per-project capture 锁 §145
```

## 6. 2.0 目标流（DECISION，落点见 MILESTONE_MATRIX）

```text
Adapter/Analysis (L0/L1/L2)
  → Facts + Evidence + Relations  → Intelligence Core（统一模型）
  → Consumers（Impact/Context/Health/Debug/…）只读 Core
  → Snapshot：Core 状态的版本化切片（含域指纹）
  → Interface（GUI/CLI/MCP/JSON）三面同源，无业务副本
```

- 变化点 1：今天各域各自的局部 Evidence/Trust 模型，M21 起向统一 Evidence 契约收敛（adapter/兼容层过渡）。
- 变化点 2：Godot/C# 事实经 adapter 进入同一 Core，而不是旁路 JSON。
- 变化点 3：Context 从"运行时选择器"升级为"消费 Core 关系图的 Intelligence"（M27）。
- 不变项：SQLite 单库 + prism-store 独占；Git 只读；Manual Snapshot First；本地优先安全边界。
