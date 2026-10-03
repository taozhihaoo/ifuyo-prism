# ifuyo Prism

> **AI-native、local-first 的项目理解与工程体检工作台。**
>
> Prism 不替你写项目，而负责让你和 AI **看懂项目、看懂变化、找到风险、验证结果，并生成可靠的项目上下文。**

## What Prism is

一个理解软件项目的本地工作台，沿这条链路构建能力：

```text
看懂项目 → 看懂变化 → 发现风险 → 验证项目 → 生成可靠上下文
```

- **Understand** — 结构/符号/依赖图（M1-M3）+ Git 事实（M4 已交付）
- **Changes** — Git 变化 → ChangeSet → 影响分析
- **Health** — 基于 Evidence 的 Finding 体系（不做“项目评分”）
- **Verify** — 构建/测试/lint 的统一编排（Orchestrator，不重造工具）
- **Environment** — 项目要求 vs 机器事实
- **Context** — 风险加权、任务感知的 AI 上下文包（Deterministic First）

## What Prism is not

- IDE / 代码编辑器 / AI Coding Agent
- Git GUI / Linter Dashboard / 依赖查看器
- ChatGPT Clone / 自研 LLM / 云服务 / DevOps 平台
- 自动提交或大规模修改代码的工具

完整边界见 `Spec/ifuyo Prism — 开发总纲领 v1.0.md`（§19 明确不做）。

## Tech stack

| 层 | 技术 |
| --- | --- |
| Desktop 壳 | Tauri 2 |
| Core | Rust（Cargo workspace，`crates/prism-*`） |
| Frontend | React 18 + TypeScript + Vite |
| CLI | Rust（`prism` 命令，与 GUI 共享 Core） |

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
│  ├─ prism-git/             # Git facts 层：只读 status/log/diff/refs（M4）
│  └─ prism-cli/             # prism 命令行（M0-M4）
├─ src-tauri/                # Tauri 2 应用壳（IPC 边界，无业务逻辑）
├─ src/                      # React 前端（app/components/pages/services/types）
├─ tests/
│  ├─ fixtures/              # 合成测试项目（fixture-basic 等）
│  └─ integration/           # 跨 crate 集成测试
├─ docs/                     # architecture / development / roadmap
├─ scripts/                  # 构建辅助脚本（图标生成等）
└─ package.json              # 前端工作区根
```

## How to run

```bash
# 前置：Rust stable (windows-msvc) 与 Node >= 20
npm install

# 开发模式（会同时启动 Vite 与 Tauri 窗口）
npm run tauri dev
```

CLI（不启动窗口）：

```bash
cargo run -p prism-cli -- --version
cargo run -p prism-cli -- info          # 真实应用/运行时信息
cargo run -p prism-cli -- scan <path>      # 项目发现（M1）
cargo run -p prism-cli -- scan <path> --format json
cargo run -p prism-cli -- inspect <path>   # 代码符号（M2）
cargo run -p prism-cli -- graph <path>     # 依赖与架构图（M3）
cargo run -p prism-cli -- git status <path>  # Git 状态（M4，只读）
cargo run -p prism-cli -- git log <path>     # 最近提交（M4）
cargo run -p prism-cli -- git diff <path>    # 文件级 diff（M4）
cargo run -p prism-cli -- inspect <path> --format json
cargo run -p prism-cli -- --help
```

## How to test

```bash
cargo test --workspace          # Rust 单元 + 集成测试
npm run typecheck               # 前端类型检查
npm test                        # 前端组件测试（vitest）
```

## How to build

```bash
npm run tauri build             # 桌面应用（Windows 下产出 exe/安装器）
npm run build                   # 仅前端产物（dist/）
cargo build --workspace         # 仅 Rust（含 prism CLI）
```

## Principles（摘要）

- **Deterministic First**：核心分析是确定性代码，AI 只是增强层；无网络、无 AI Key 也必须完整可用
- **Local-first**：无遥测、无自动上传；上下文生成默认脱敏
- **Evidence first**：所有结论可定位、可复现、可解释
- **GUI/CLI 共享 Core**：业务逻辑只有一份，Tauri 与 CLI 都是薄壳

详细工程规范见 [docs/architecture.md](docs/architecture.md) 与 [docs/development.md](docs/development.md)。
