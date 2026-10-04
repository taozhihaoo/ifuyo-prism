# PRISM_2_0_BASELINE.md

> 2.0-M20 产物（M20 §44）。记录 2026-10-05 的真实工程状态。
> 所有条目标注 FACT / INFERENCE / HYPOTHESIS；冲突时以代码 + 测试 + 运行结果为准。

## 1. Project Overview

- FACT：ifuyo Prism 是本地优先的软件工程智能工作台（Rust workspace + Tauri 2 GUI + React 前端）。
- FACT：无 AI、无网络、无遥测（1.0 charter 边界；workspace 依赖中无任何网络客户端 crate 用于分析路径）。
- FACT：Git 全程只读（prism-git 无写路径；§114 有只读回归测试）。

## 2. Current Version / Commit

- FACT：workspace 版本 0.1.0（Cargo.toml = package.json = tauri.conf.json 三处一致）。
- FACT：M20 基线 commit = `39c639b`（fix(2.0-m20): 项目根路径身份归一化）之后的 main。
- FACT：工作树除未跟踪的 Spec/ 提示词外干净。

## 3. Current Modules（15 crates + 2 应用壳）

| crate | 职责 | 2.0 去向（详见 MIGRATION） |
|---|---|---|
| prism-core | 错误/严重度/任务取消/privacy/配置/runtime 抽象 | Core Infrastructure（RETAIN/EXTEND） |
| prism-project | M1 发现扫描/语言识别/忽略规则 | L0（EXTEND） |
| prism-parser | 手写 Python parser + 语言注册表 | L1（EXTEND，C# parser 为 M23 新建） |
| prism-analysis | M2 代码模型（ProjectCodeModel/ParseSummary） | L1（EXTEND） |
| prism-graph | M3 模块依赖图（ModuleId/Dependency/Cycle） | 统一 Relation 层（MIGRATE） |
| prism-git | M4 只读 git 事实（status/log/diff） | Change/History 层（RETAIN） |
| prism-changes | M5 ChangeSet（WorkingTree/Revision） | Change（EXTEND） |
| prism-impact | M6 反向 BFS 影响分析 | Evidence-backed Impact（MIGRATE） |
| prism-health | M7 规则引擎 + 13+ 规则 + Finding 模型 | Finding Engine（EXTEND） |
| prism-verify | M8 验证引擎（checks/CI/release） | Validate（RETAIN） |
| prism-env | M9 环境事实/需求匹配/端口/.env | Environment Intelligence（EXTEND） |
| prism-context | M10 上下文选择引擎（TrustLevel/freshness/recipe） | Context Intelligence（EXTEND） |
| prism-snapshot | M11 快照模型/域指纹/捕获/比较/时间线/域装配 | Versioned Snapshot（EXTEND） |
| prism-store | SQLite 持久层（schema/migrations/事务） | Persistence（RETAIN） |
| prism-cli | clap CLI（14 实命令 + report 占位） | Interface（RETAIN） |
| src-tauri (prism-app) | Tauri 2 壳 + 49 个 IPC 命令 + session 状态 | Interface（RETAIN） |
| src/ (React) | 9 页面 + i18n + 主题/密度 | Interface（RETAIN） |

## 4. Current Architecture

- FACT：分层为 `Interface（CLI/GUI）→ Tauri IPC → Application Services（src-tauri commands + CLI commands）→ Domain Services（各 prism-* crate）→ Persistence（prism-store/fs/git）`。
- FACT：域 crate 之间依赖方向无环（workspace 内 15 crate 拓扑无循环；snapshot 依赖 analysis/graph/changes/impact/health/verify/env 于 M11b §14 接线时引入，方向仍是 consumer→provider）。
- INFERENCE：不存在 Core 反向依赖 UI/Tauri 的路径——prism-core 不依赖任何 interface crate。

## 5. Current Data Flow

- FACT：GUI `open_project` → `Project::from_root`（canonicalize + 去 verbatim）→ session 持有；各 `start_*` IPC 启动任务、`get_*` 轮询状态（SessionTask 状态机）。
- FACT：CLI 每命令独立进程，直接调 Domain Service，无共享缓存进程。
- FACT：快照捕获 `collect_analysis` 一次产出 M1–M9 事实 → `build_domain_data` 规范化 → `SnapshotCaptureService.capture` 去重/原子落库（2.0-M20 起项目根统一 canonical 口径，commit 39c639b）。
- 详见 PRISM_2_0_DATA_FLOW.md。

## 6. Current Database

- FACT：单一 SQLite（rusqlite 0.37 bundled）；默认 `%APPDATA%/Prism/prism.db`（Linux `~/.local/share/Prism`），`PRISM_DB_PATH` 可覆盖。
- FACT：schema 全部由 prism-store 拥有：`projects` / `snapshots` / `snapshot_domains` / `schema_migrations`（顺序迁移、fail-stop、事务原子写、busy_timeout）。
- FACT：M20 无 schema 变更 → **"No schema migration required."**

## 7. Current Cache

- FACT：应用内缓存 = SessionState 中的 discovery/code-model/graph 复用（同源，无跨进程持久缓存）；快照去重依赖 store 指纹查询。
- FACT：CLI 无跨运行缓存（每命令冷跑）；GUI session 内增量复用。
- INFERENCE：不存在"同一数据多 service 各自缓存"的正确性问题；重复扫描集中在一次性命令属 Manual 语义，非缺陷。

## 8. Current CLI

- FACT：14 个实命令：scan / inspect / graph / git / changes / impact / health / verify / env / context / snapshot / history（export/delete 子命令）/ compare / timeline；`report` 仍为占位（exit 1）。
- FACT：全部命令有 `--json`（部分另有 `--markdown`）；退出码 0/1/2/3/4/5/6 有明确语义（OK/ERROR/USAGE/INVALID_INPUT/CANCELLED/WARNINGS/NOT_GIT）。

## 9. Current GUI

- FACT：9 页面：Overview / Understand / Changes / Health / Verify / Environment / Context / History（快照历史）/ Settings；分组导航 + Ctrl+K 调色板 + Ctrl+1..9 + 中英文（en/zh）+ 主题/密度。
- FACT：前端不直接访问 fs/git/sqlite——只经 49 个 IPC 命令。

## 10. Current MCP

- **FACT：MCP 不存在，待建**（2026-10-05 已核实：workspace 无任何 MCP server/tools 代码；1.0 charter 明确排除）。
- DECISION：MCP 是 2.0 新建能力；M20 的 Security Boundary（见 ARCHITECTURE §14）作为未来 MCP 实现的输入契约。

## 11. Current Analysis

- FACT：L0（结构/清单/git/配置/环境）能力完整存在（prism-project/git/env + scan）。
- FACT：L1 仅 Python 有真实 parser（`supported_languages() == [Python]`）；Language 枚举可识别 14+ 语言（TS/TSX/JS/Python/C#/Rust/Java/C/C++/Go/HTML/CSS…）但无对应 parser。
- FACT：L2 无语义关系（当前 graph 为模块级 import 解析，非符号级 reference/call）。
- FACT：Godot 无任何专用支持（.tscn/.tres/project.godot 仅作为普通文件出现在 L0）。

## 12. Current Tests

- FACT：`cargo test --workspace` 65 suites / **529 passed / 0 failed**（M20 修改前基线与修改后复测一致，2026-10-05）。
- FACT：前端 vitest 10 files / **55 passed**。
- FACT：clippy --workspace --all-targets 0 告警；`cargo fmt --check` exit 0。
- FACT：含真实进程级重启持久化测试（snapshot survives process restart）、迁移/损坏库处理测试、git 只读测试、取消测试。

## 13. Current Performance（M20 §36 复测）

- FACT：方法 = release 构建（同一 commit）、Prism 仓库自身、独立 `PRISM_DB_PATH`、每命令 warm 态 3 次取最小。M13 基线为冷态单跑（未记录 warm-up 处理），差异按 §37 归类为测量状态而非代码回退。

| 操作 | M13 基线 | M20 复测 | 判定 |
|---|---|---|---|
| scan | 2336ms | 16ms | 测量状态差异（warm）；非回退 |
| inspect | 101ms | 59ms（rc=5=WARNINGS，设计行为） | 测量状态差异 |
| changes | 354ms | 321ms | 持平 |
| impact | 415ms | 379ms | 持平 |
| health | 648ms | 169ms | 测量状态差异 |
| env | 820ms | 722ms | 持平 |
| context(review) | 501ms | 401ms | 持平 |
| **snapshot** | **230ms** | **1287ms** | **真实回退，原因明确**：M11b §14 域捕获接线（commit 5ea240c/147409e）令 capture 运行完整 M1–M9 管线——有意为之的功能成本，非意外回退 |
| history | 48ms | 7ms | 测量状态差异 |
| compare | 122ms | 8ms | 测量状态差异 |
| timeline | 127ms | 66ms | 测量状态差异 |

- 判定：>15% 持续回退警戒线（§37）仅 snapshot 触发，原因已定位并记录——无"未解释的显著回退"。

## 14. Current Security

- FACT：Tauri capability 最小化；IPC 命令白名单（generate_handler 49 个）；前端无 fs/git/sqlite 直连。
- FACT：中央 privacy 层（prism_core::privacy）+ 各输出面二次脱敏（env markdown/html、snapshot export redact、CLI 文本）；secret 值结构上不入库。
- FACT：无网络、无遥测；verify 只执行 prism.config.json 声明的 check 命令（write_policy 声明制）。
- FACT：SQL 全参数化；markdown/HTML 输出有转义路径。

## 15. Known Issues / 1.0 Residuals（Legacy Issue Ledger，M20 §23）

决策值域：FIX NOW / DEFER / DOWNGRADE / REMOVE / NOT REPRODUCED / ALREADY FIXED。

| # | Issue | 来源 | 证据 | Severity | Decision | Target | 理由 |
|---|---|---|---|---|---|---|---|
| L1 | M2–M10 域数据未进 Snapshot | M11 audit P1 | 2026-10-04 E2E 实测八域 Available + compare 深度（docs/audits/M11-final-audit.md） | — | **ALREADY FIXED** | — | collect_analysis/build_domain_data 已接线 CLI+GUI |
| L2 | include_git 默认值漂移（git 域恒 NotApplicable） | M20 explore | commit 8c65d80 修复 + 冒烟实测 git 域 Available | P1(已闭合) | **ALREADY FIXED** | — | M11b 收尾修复 |
| L3 | snapshot 项目身份不按 canonical 根注册（`.` 与绝对路径分裂同一项目） | M20 explore | commit 39c639b + 跨路径形态 E2E 实测 | P1(已闭合) | **ALREADY FIXED** | — | 本里程碑 §32 修复 |
| L4 | Linux/macOS 打包与安装实测 | M13 audit P1 | 仅 Windows 实测记录 | P2 | **DEFER** | M215 前 | 需要对应平台环境；不影响 2.0 数据/架构工作 |
| L5 | 自动 retention（快照清理） | M11/M13 audit | §63 v1 手动 delete 命令可用 | P3 | **DOWNGRADE** | 2.0 后段 | Manual First 语义下非核心 |
| L6 | Toast 统一通知组件 | M12 audit P1 | 各页面就地错误文本，无统一 toast | P3 | **DOWNGRADE** | M212 | UX polish；2.0 方向下非阻塞 |
| L7 | 独立 Architecture View | M12 audit P1 | 图数据在 Understand 页内嵌展示 | P2 | **DEFER** | M212 | M20 §25：Architecture View 最终属 M212 Visualization，M20 只定义 contract |
| L8 | 完整 WCAG audit | M12/M13 audit | 基础 aria/键盘已做，未做全面审计 | P3 | **DEFER** | M215 | 系统 audit 阶段统一做 |
| L9 | §203/§204 migration 兼容矩阵 | M11a audit P1 | §157 迁移/损坏库测试已补（M11a 收尾） | P3 | **DOWNGRADE** | M215 | 框架已就位，扩展矩阵归最终审计 |
| L10 | report 命令占位（exit 1） | 1.0 交付记录 | `prism report` exits 1 | P3 | **DOWNGRADE** | 2.0 后段评估 | 2.0 由各域命令 + history export 覆盖其主要意图；若 M211/M212 需要再实现 |
| L11 | MCP 缺失 | 总纲领 §三十七/§三十九 | 本文件 §10 | — | **DEFER（新建）** | 2.0 中后段（候选 M27 后） | 2.0 新建能力，非 1.0 遗留；需按 Security Boundary 单独设计 |
| L12 | C# / Godot 分析能力缺失 | 2.0 目标差距 | 本文件 §11 | — | **DEFER（新建）** | M23/M24/M25 | 2.0 主线路线本身 |

P0 = 0；P1 = 0（现存）；P2 = 2（L4、L7）；P3 = 其余。

## 16. Explore 覆盖声明（M20 §四）

FACT：M20 执行中完整检查了 workspace 15 crate 结构、prism-cli 11 命令文件、src-tauri 7 个 command 模块与 lib.rs 白名单、9 个前端页面、prism-store 全部 migration/schema、docs/audits 全部审计文档、git log/状态、Spec/ 16 份 2.0 提示词。MCP、SQLite schema、parser 注册表、性能数据均为本会话实测/实读，非沿用文档。
