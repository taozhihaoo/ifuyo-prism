# ifuyo Prism

> **AI-native、local-first 的项目理解与工程体检工作台。**
>
> Prism 不替你写项目，而负责让你和 AI **看懂项目、看懂变化、找到风险、
> 验证结果、生成可靠上下文——并看见、部署它**。

## What Prism is

一个理解软件项目的本地工作台，沿这条链路构建能力：

```text
看懂项目 → 看懂变化 → 发现风险 → 验证项目 → 生成上下文 → 观察运行 → 看见/部署
```

### 1.0 工程体检（已交付）

- **Understand** — 结构/符号/依赖图（M1-M3）+ Git 事实（M4）+ ChangeSet（M5）
- **Changes → Impact** — Git 变化 → ChangeSet（M5）→ 影响分析（M6）→ 健康发现（M7：13 条确定性规则 + 证据化 Findings）
- **Health** — 基于 Evidence 的 Finding 体系（不做"项目评分"）
- **Verify** — 构建/测试/lint 统一编排（M8：Node/Python/C#/Rust 适配 + 结构化解析 + CI 静态抽取 + Release 检查）
- **Environment** — 项目要求 vs 机器事实（M9：requirement 层级、冲突保留、端口/进程动态事实）
- **Context** — 风险加权、任务感知的 AI 上下文包（M10：确定性选择 + Context Health + trust/risk，无 LLM 也能工作）
- **Snapshot** — 工程事实的持久化记忆（M11：稳定身份 + 多域指纹 + SQLite + compare/timeline/history）

### 2.0 Intelligence（M20-M215，已交付）

在 1.0 基础上建成统一的工程事实层（`prism-intel`：Entity/Fact/Evidence/
Relation/Finding），并沿这条链扩展：

- **L0/L1/L2** — 结构清单（M22）→ C# 符号引擎（M23，tree-sitter）→ 语义关系解析（M25：CALLS/INHERITS/IMPLEMENTS/OVERRIDES）
- **Godot Adapter** — Scene/Node/Script/Resource 解析（M24）+ Scene↔Script↔Symbol 跨层关联
- **Evidence Graph & Impact** — 统一证据图 + 反向影响分析（M26：Change × 关系 → 受影响集合 + 证据路径）
- **Context 2.0** — Impact/Change-aware 上下文（M27 `--impact-aware`）
- **Health 2.0** — 统一 Finding 模型 + 19 条内建规则（7 categories）+ 生命周期（New/Persistent/Resolved/Regressed/Suppressed）+ Quality Gate（M28）
- **Dependency/Security/Environment** — 依赖声明/解析分离 + lockfile + SBOM + secret 检测（全链 redaction）+ 离线 advisory 匹配（M29）
- **Debug Intelligence** — 日志→事件→错误指纹→聚类→栈帧→符号/场景关联（M210）
- **Git Evolution** — 文件/符号历史回放 + 热点（原始指标）/变更耦合（M211）
- **Visualization** — godot 跨层/architecture 层级/dependency/impact 四投影（M212，确定性有界 View Model）
- **API/Webhook Debugging** — 端点发现（ASP.NET/Express/FastAPI）+ 有界 HTTP 观测（SSRF guard）+ 本地 Webhook 接收器（HMAC 签名）+ 关联链（M213）
- **Developer Deploy** — Artifact 身份/Manifest + Local transport（原子 activate）+ Preflight/Policy/Plan/Execute 分离 + Rollback/History/Diff（M214）
- **Final Audit** — 全量回归 + 断链检查 + Release Decision = READY_WITH_KNOWN_LIMITATIONS（M215）

## What Prism is not

- IDE / 代码编辑器 / AI Coding Agent
- Git GUI / Linter Dashboard / 依赖查看器
- ChatGPT Clone / 自研 LLM / 云服务 / DevOps 平台 / CI/CD SaaS
- 自动提交或大规模修改代码的工具
- API 网关 / 抓包工具 / 渗透测试工具（M213 只做受观测面的调试事实）
- 远程 Shell（M214 是受控部署系统）

完整边界见 `Spec/ifuyo Prism — 开发总纲领 v1.0.md`（§19 明确不做）。

## Tech stack

| 层 | 技术 |
| --- | --- |
| Desktop 壳 | Tauri 2 |
| Core | Rust（Cargo workspace，26 个 `crates/prism-*`） |
| Frontend | React 18 + TypeScript + Vite |
| CLI | Rust（`prism` 命令，与 GUI 共享 Core） |
| 解析 | tree-sitter（C# 等增量语法解析） |
| 哈希 | SHA-256（Artifact/Manifest 身份） |

## Repository structure

```text
ifuyo-prism/
├─ Spec/                     # 总纲领与各里程碑开发提示词（规格文档）
├─ crates/
│  ├─ prism-core/            # 地基：ID/错误/配置/任务/日志/运行时抽象（M0）
│  ├─ prism-project/         # 项目发现：扫描/ignore/语言/分类/入口点（M1）
│  ├─ prism-parser/          # 代码理解：Symbol 模型 + tree-sitter adapters（M2）
│  ├─ prism-analysis/        # 分析编排：CodeModel/SymbolIndex（M2）
│  ├─ prism-graph/           # 关系层：模块/解析/依赖图/环/指标（M3）
│  ├─ prism-git/             # Git facts：status/log/diff/refs/blame/file_history（M4+M211）
│  ├─ prism-changes/         # ChangeSet：Git diff × 代码模型 → 变更事实（M5）
│  ├─ prism-impact/          # 1.0 影响层（M6）
│  ├─ prism-health/          # 1.0 健康层：13 条规则（M7，RETAIN）
│  ├─ prism-verify/          # 验证编排（M8）
│  ├─ prism-env/             # 环境事实（M9）
│  ├─ prism-context/         # Context Pack（M10/M27）
│  ├─ prism-snapshot/        # 快照 + compare/timeline（M11）
│  ├─ prism-store/           # SQLite 持久化（M11）
│  ├─ prism-intel/           # 2.0 统一核心：Entity/Fact/Evidence/Relation/Finding + Impact（M21/M26）
│  ├─ prism-l0/              # 2.0 结构分析（M22）
│  ├─ prism-csharp/          # 2.0 C# 符号/语义/变更映射（M23/M25）
│  ├─ prism-godot/           # 2.0 Godot Adapter（M24）
│  ├─ prism-health2/         # 2.0 统一 Finding：19 条规则/7 categories/生命周期/Gate（M28）
│  ├─ prism-supply/          # 2.0 依赖/安全/SBOM/环境对照（M29）
│  ├─ prism-debug/           # 2.0 日志/错误指纹/聚类/栈帧关联（M210）
│  ├─ prism-evolution/       # 2.0 演化事实/热点/耦合（M211）
│  ├─ prism-viz/             # 2.0 可视化投影（M212）
│  ├─ prism-api/             # 2.0 API/Webhook 调试（M213）
│  ├─ prism-deploy/          # 2.0 受控部署（M214）
│  └─ prism-cli/             # prism 命令行（全命令族）
├─ src-tauri/                # Tauri 2 应用壳（IPC 边界，无业务逻辑）
├─ src/                      # React 前端（app/components/pages/services/types）
├─ tests/                    # 跨 crate 集成测试
├─ docs/                     # architecture（各里程碑契约）/ audits（Final Report）/ development / roadmap
├─ scripts/                  # 构建辅助脚本
└─ package.json              # 前端工作区根
```

## CLI（全命令族）

```bash
# ---- 1.0 工程体检 ----
prism scan <path>                  # 项目发现
prism inspect <path>               # 代码符号
prism graph <path>                 # 依赖与架构图
prism git status|log|diff <path>   # Git facts（只读）
prism changes <path>               # ChangeSet
prism impact <path>                # 影响分析
prism health <path>                # 健康体检（1.0 规则）
prism health <path> rules          # 规则检视
prism verify <path>                # 验证编排
prism env <path>                   # 环境事实
prism context <path> --task "..."  # AI 上下文包
prism snapshot|history|compare|timeline <path>  # 快照记忆

# ---- 2.0 Intelligence（M21-M215）----
prism intel <path> --domain dependency|structural|symbols|semantic|godot|impact
                                   # 统一智能视图（M21-M26）
prism health2 <path> [--baseline f.json]   # 统一 Finding + 生命周期 + Gate（M28）
prism supply <path> [--advisories db.json] [--policy p.json] [--sbom out.json]
                                   # 依赖/安全/license/SBOM（M29）
prism debug <path> [--source f.log]        # 日志→错误指纹→聚类→栈帧（M210）
prism git evolution|hotspots|coupling|symbol-history|blame|commit-impact <file>
                                   # Git 演化智能（M211）
prism viz --view architecture|godot|dependency      # 可视化投影（M212）
prism api endpoints|observe|correlate|webhook-listen  # API/Webhook 调试（M213）
prism deploy artifact|plan|run|history|rollback      # 受控部署（M214）
```

全部命令支持 `--json`（稳定 schema）与 `--help`。

带子命令的命令族（git/viz/api/deploy）路径可用三种等价形态：省略（缺省
`.`）、位置参数（`prism git . evolution <file>`，兼容保留）、`--path`
标志（`prism git --path <repo> evolution <file>`）。deploy 以各子命令的
`source`/`--target-root` 为准，父级路径仅为兼容占位。

## MCP server（AI 客户端接入）

```bash
prism mcp   # 只读 JSON-RPC stdio 服务器（newline-delimited）
```

让本地 AI 客户端（Claude Desktop、Claude Code 等）直接消费 Prism 的
**持久化分析结果**（与 GUI 共用 SQLite：`analysis_runs`，`PRISM_DB_PATH`
同口径）。工具：`list_projects` / `list_runs` / `get_report` /
`get_finding`（含证据展开，§37 可追溯）。不重跑分析、不读项目文件、
不写任何数据；stdout 只承载协议，日志走 stderr。

客户端配置示例（Claude Desktop `claude_desktop_config.json`）：

```json
{
  "mcpServers": {
    "prism": {
      "command": "E:/WorkSpace/ifuyo-prism/target/release/prism.exe",
      "args": ["mcp"]
    }
  }
}
```

GUI 每次完成 健康分析（1.0）/ health2 后自动落库（每项目每域保留最近
50 次运行）；health2 的生命周期基线 = 库中上次运行（跨会话
Persistent/Resolved/Regressed），首轮或 schema 变更则回退空基线（全部
New，§26）。

## Desktop app（打包）

```bash
npm run tauri build
```

产物位于 `target/release/bundle/`（Windows：NSIS 安装器 + 独立 exe）。
数据边界：快照数据库在系统 AppData（不在项目里）；Prism 不修改用户项目、
不联网（网络动作仅 M213 显式观测/M214 显式部署且经 SSRF/allowlist 强制）、
无遥测、secret 全链 redaction。

## How to test

```bash
cargo test --workspace          # 804 项 Rust 单元 + 集成测试
npm run typecheck               # 前端类型检查
npm test                        # 61 项前端组件测试（vitest）
cargo clippy --workspace --all-targets   # 0 warnings
```

## How to build

```bash
npm run tauri build             # 桌面应用（Windows 下产出 exe/安装器）
npm run build                   # 仅前端产物（dist/）
cargo build --workspace         # 仅 Rust（含 prism CLI）
cargo build --release           # release CLI（~21 MB）
```

## Principles（摘要）

- **Deterministic First**：核心分析是确定性代码，AI 只是增强层；无网络、无 AI Key 也必须完整可用
- **Local-first**：无遥测、无自动上传；上下文生成默认脱敏
- **Evidence first**：所有结论可定位、可复现、可解释
- **GUI/CLI 共享 Core**：业务逻辑只有一份，Tauri 与 CLI 都是薄壳
- **投影非事实**：可视化/部署/观测都是 Intelligence Core 的投影，不建第二事实源
- **Unknown ≠ False**：数据缺失/解析失败/凭证不可用全部如实标注，不伪装成零或正常

详细工程规范见 [docs/architecture.md](docs/architecture.md)、
[docs/development.md](docs/development.md)、各里程碑契约
（`docs/architecture/PRISM_2_0_*.md`）与最终审计报告
（`docs/audits/2.0-M215-final-audit.md`）。
