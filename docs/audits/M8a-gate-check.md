# M8a Gate Self-Check — Verify Engine Core

日期：2026-10-04 ｜ HEAD：见 git log ｜ 规格：`Spec/Prism M8a — Verify Engine Core 完整开发提示词.md`
（M8 最终验收以 M8b §315 Final Audit / §316 Completion Gate 为准；本文档为 M8a 前置门槛检查。）

## Baseline（§1/§2）

- **FACT（CONFIRMED）**：M0 `CommandRunner`/`CommandSpec`/`CancellationToken` 完备——M8 复用，未造第二套 runner（§2）。
- **FACT（CONFIRMED）**：`CheckRunId` 已在 M0 ids 预留；`prism.config.json` 分节 + deny_unknown_fields 模式沿用（verify 节）。
- **FACT（CONFIRMED）**：M1 项目类型/M4 Git facts/M5 ChangeSet/M7 Health findings 全部可消费；CLI verify 占位存在。

## 门槛逐项证据

| 门槛项 | 证据 |
| --- | --- |
| M0–M7 regression（§175/§176/§249） | 全工作区测试绿（见 Q）；M0 仅加 max_output_bytes/output_truncated（加法） |
| 核心模型（§5-§12/§127/§128） | CheckSpec（稳定 check id）/CheckRun（UUID run_id）/CheckState 十态/Outcome/CheckKind 七种/VerifyPlan/Prerequisite/CheckPolicy/WritePolicy |
| 引擎分层（§97-§99/§254-§258） | prism-verify: discovery/planner/executor/model/fake/render/tool 分模块；复用 M0 CommandRunner，无第二套 runner |
| 入口发现 + source/confidence/provenance（§22-§24/§56-§59/§235-§241） | package.json scripts（lockfile 选包管理器）/Cargo/pyproject/自定义 checks；configured-wins（§59）；detected_from 精确到文件与字段（§236）；Exact/HighConfidence 才自动执行（§238） |
| VerifyPlan/依赖图/Blocked（§25-§28/§161） | 拓扑层序 + Build→rest 约定依赖；依赖非 Passed ⇒ Blocked（单测 + e2e） |
| bounded concurrency + plan 顺序（§29-§30/§225） | 层内 ≤ max_concurrency；结果按 plan 顺序排序（单测） |
| Timeout/Cancel/进程树（§90-§95/§229） | 超时 ⇒ TimedOut+诊断；cancel ⇒ Err(Cancelled) ⇒ Cancelled 状态；直接子进程被终止（ping e2e）。**进程树限制如实记录**：M0 terminate 仅终止直接子进程，孙进程孤儿为已文档化平台限制（executor.rs 头注释；job object/process group 留 M8b） |
| Command errors 分类（§95） | program_not_found ⇒ Unavailable；permission/working_directory/spawn ⇒ Error；timeout ⇒ TimedOut；非零 ⇒ Failed（单测 + e2e） |
| WritePolicy（§82-§89/§85） | ArbitraryWrite 默认 Skipped 且不 spawn（e2e 断言 runner.calls 为空）；format 类发现规则默认 arbitrary_write（§87） |
| secret 脱敏（§77/§173-§176） | stdout/stderr 行级 [REDACTED]；--flag 参数值脱敏；e2e 断言值不出现 |
| 输出有界 + 截断标记（§78-§79/§201-§202） | M0 max_output_bytes（默认 1 MiB/流）+ output_truncated 标记 + 截断诊断；e2e 断言 head 保留与标记 |
| 不 dump 环境变量/无遥测（§17-§18/§176-§178） | env 元数据仅 present/absent；无网络代码路径 |
| --dry-run 零执行（§144-§146/§199） | e2e 断言 runner.calls 为空 + "No commands executed." |
| `prism verify` 基础执行面（§141-§148） | discover→plan→execute→aggregate→print→exit code；5 个 CLI 契约测试 |
| Unavailable/Blocked/Failed/Cancelled/TimedOut/Error UX（§11/§149-§154） | 渲染器状态标签 + reason + 诊断；e2e 断言 |
| FakeCommandRunner 基座（§179-§181） | fake.rs：脚本化 per-program 响应（Output/SpawnFailed/TimedOut/Cancelled）+ calls 审计 |
| 真实命令 smoke（§182-§183） | real_command_smoke（跨平台 exit 0）+ ping 超时/输出测试 |
| side-effect/dry-run/security/large-output fixtures（§198-§202） | side-effect e2e（git status diff ⇒ side_effects）；dry-run 零 spawn；SEC 脱敏（M7 已测）+ args 脱敏单测；large-output ping 截断测试 |
| 无 AI/遥测/数据库/Git mutation/auto-fix | 代码审计：零网络零 AI；Git 只读（§83）；不自动修源码 |
| 未提前实现 M8b | adapters/结构化解析/CI/Release 子命令/GUI verify 留 M8b |

## 进程树/timeout 说明（§92/§229 如实记录）

M0 `terminate` 仅终止**直接子进程**；verify 的 timeout/cancel 路径复用该机制并已测试
（TimedOut 状态 + 诊断 + ping 进程在截止期内被终止）。孙进程孤儿（npm→node→worker）
是已知平台限制：Windows Job Object / POSIX process group 清理计划在 M8b 完善。
当前无孤儿检测测试——已被本门槛如实记录为限制，而非宣称已解决。

## Risks

- **P1**：进程树孙进程孤儿（上述限制）；large-output 测试使用真实 ping（慢 ~4s）。
- **无 P0**。

## Deferred → M8b

结构化输出解析（adapters per ecosystem）、CI/Release 子命令、GUI verify 集成、
--retry、进程树 Job Object 清理、per-check side-effect 精确归因（当前为层粒度）。
