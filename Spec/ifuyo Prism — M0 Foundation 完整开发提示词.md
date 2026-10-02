# ifuyo Prism — M0 Foundation
## 完整、详细、专业开发提示词

你现在负责实现 **ifuyo Prism** 的 **M0 — Foundation / 基础工程阶段**。

这是一个长期维护、AI-native、local-first 的桌面开发者工具。  
当前目标不是实现 Prism 的业务能力，而是建立一个**稳定、可扩展、可测试、适合后续 AI 持续开发的工程基础**。

---

# 1. 项目定位

Prism 的核心定位：

> **理解项目 → 理解变化 → 找到风险 → 验证项目 → 给 AI 提供可靠上下文**

Prism 不是：

- IDE
- 代码编辑器
- AI Coding Agent
- ChatGPT Clone
- Issue Tracker
- 项目管理软件
- Cloud CI
- DevOps 平台
- 自动大规模修改代码的工具

Prism 更接近：

> **一个 AI-native 的本地项目理解、工程健康、变更分析与上下文工作台。**

M0 不实现上述业务能力，只负责建立底座。

---

# 2. 本阶段目标

M0 的唯一目标：

> **建立 Prism 后续所有阶段都可以稳定依赖的工程基础。**

M0 完成以后，项目应该已经具备：

1. Tauri 2 桌面应用壳
2. Rust Core
3. React + TypeScript + Vite 前端
4. Rust Workspace 基础结构
5. Core / Project / Git / Analysis / Verify 等未来模块的架构边界
6. 统一错误模型
7. 统一日志系统
8. 统一配置系统
9. 统一路径处理
10. 基础任务执行 / cancellation / progress 抽象
11. 前后端 IPC 基础通信
12. CLI 基础框架
13. Core 与 GUI / CLI 解耦
14. 基础测试体系
15. 基础 fixture 测试项目
16. 基础开发文档
17. 可以稳定运行、测试、构建的最小 Prism

**M0 不实现项目扫描、AST 分析、Git Diff 分析、健康检查、Context Engine 等真正业务功能。**

---

# 3. 最重要的开发原则

整个 M0 必须遵守以下原则。

## 3.1 Deterministic First

核心逻辑必须优先使用：

> Rust → 明确的数据结构 → 明确的规则 → 明确的结果

不要在 M0 引入 LLM。

不要设计“让 AI 决定核心逻辑”。

不要为了体现 AI-native 而强行加入 AI API。

Prism 的核心能力必须可以在：

> **完全离线、没有任何 AI API Key 的情况下运行。**

---

# 4. AI 的位置

AI 在 Prism 中属于：

> Enhancement Layer

而不是：

> Foundation Layer

因此 M0：

**禁止：**

- OpenAI API
- Anthropic API
- Gemini API
- Ollama
- embedding
- vector database
- RAG
- agent
- MCP server
- AI chat
- prompt engine

这些全部放到未来阶段。

M0 的结构必须能够支持未来加入 AI，但不能依赖 AI。

---

# 5. 技术方向

采用：

### Desktop

- Tauri 2

### Backend

- Rust

### Frontend

- React
- TypeScript
- Vite

### CLI

- Rust CLI

### Workspace

使用 Cargo Workspace。

建议根目录：

```text
prism/
├─ src/
│  ├─ app/
│  ├─ components/
│  ├─ features/
│  ├─ pages/
│  ├─ services/
│  ├─ types/
│  └─ main.tsx
│
├─ src-tauri/
│  ├─ src/
│  │  ├─ main.rs
│  │  ├─ lib.rs
│  │  ├─ commands/
│  │  └─ state/
│  └─ Cargo.toml
│
├─ crates/
│  ├─ prism-core/
│  ├─ prism-project/
│  ├─ prism-parser/
│  ├─ prism-git/
│  ├─ prism-analysis/
│  ├─ prism-health/
│  ├─ prism-verify/
│  ├─ prism-environment/
│  ├─ prism-context/
│  ├─ prism-report/
│  └─ prism-cli/
│
├─ tests/
│  ├─ fixtures/
│  └─ integration/
│
├─ docs/
│  ├─ architecture.md
│  ├─ development.md
│  └─ roadmap.md
│
├─ package.json
├─ Cargo.toml
├─ README.md
└─ .gitignore
```

**注意：**

上面的目录是目标架构建议，不意味着 M0 必须把所有 crate 都实现成有业务代码的模块。

M0 可以创建未来 crate skeleton，但：

> **不要为了“完整架构”而制造大量空模块、接口和抽象。**

只创建未来真正需要稳定边界的结构。

---

# 6. 首先进行完整基线审计

在任何修改之前：

## 6.1 检查项目

检查：

```text
当前目录
Git 状态
Git branch
Git log
已有 Cargo.toml
已有 package.json
已有 src
已有 src-tauri
已有 crates
已有 tests
已有 docs
已有配置文件
已有 CI
已有 README
```

同时确认：

- 当前是否已经存在 Prism
- 是否已经有旧实现
- 是否存在未提交修改
- 是否存在未追踪文件
- 是否存在 build artifacts
- 是否存在 node_modules
- 是否存在 target
- 是否存在 secrets
- 是否存在 `.env`
- 是否存在本地路径
- 是否存在临时文件

---

# 7. 基线原则

如果仓库已经存在代码：

> **不得直接覆盖现有实现。**

先判断：

- 哪些可以保留
- 哪些需要迁移
- 哪些明显属于旧架构
- 哪些属于当前目标
- 哪些属于临时文件

输出：

```text
BASELINE
ARCHITECTURE
RISKS
MIGRATION PLAN
```

再开始修改。

---

# 8. Git 工作原则

M0 全程：

> **不 push。**

默认允许：

- 修改代码
- 新增文件
- 删除明确属于旧模板/临时文件的内容
- 创建本地 commit

但删除旧代码之前必须先确认其用途。

最终至少形成：

```text
feat: establish prism foundation
```

这样的清晰 commit。

如果发现无法安全 commit：

> 停止 commit，但不要 push。

---

# 9. M0 Architecture

M0 建立以下依赖方向：

```text
Frontend
   ↓
Tauri Commands / IPC
   ↓
Application Layer
   ↓
Prism Core
   ↓
Domain / Infrastructure
```

原则：

```text
Frontend
    ↓
Tauri
    ↓
Application
    ↓
Core
```

不能反过来：

```text
Core → React
Core → Tauri UI
Core → Browser API
```

尤其禁止：

> Rust Core 直接依赖前端实现。

---

# 10. Core 设计

`prism-core` 是整个系统最重要的基础 crate。

它应该包含：

- 基础 ID
- 基础类型
- Result / Error
- Severity
- Confidence
- Timestamp
- Path abstractions
- Task abstractions
- 基础 domain traits
- JSON serialization contracts

不要让 Core：

- 读取 React 状态
- 调用 Tauri API
- 依赖具体 GUI
- 调用 Git CLI
- 依赖具体 parser
- 执行 shell 命令

Core 必须尽可能：

> **纯逻辑、可测试、可复用。**

---

# 11. 建立统一 ID

不要在不同模块里随意使用字符串作为实体 ID。

建立统一 ID 类型，例如：

```text
ProjectId
SnapshotId
FileId
SymbolId
ModuleId
FindingId
CheckRunId
ContextPackId
TaskId
```

M0 不需要实现所有业务实体。

但基础 ID 模型应该允许后续扩展。

要求：

- 可序列化
- 可反序列化
- 可安全比较
- 不依赖 UI
- 不依赖数据库

暂时可以采用 UUID 或稳定字符串方案。

不要为了“未来可扩展”引入数据库。

---

# 12. 统一 Error Model

建立统一错误模型。

建议至少覆盖：

```text
InvalidInput
NotFound
Io
Parse
Git
Command
Config
Serialization
Cancelled
Internal
Unsupported
```

错误必须：

- 可区分
- 可记录日志
- 可传给 CLI
- 可安全传给 GUI

禁止到处：

```rust
unwrap()
expect()
panic!()
```

业务路径中尤其禁止。

允许：

- 测试
- 明确不可达分支
- 静态常量初始化

但必须有合理依据。

---

# 13. Config System

建立最小配置系统。

配置至少考虑：

```text
PrismConfig
├─ general
├─ paths
├─ analysis
├─ logging
└─ privacy
```

M0 不需要真正实现所有字段。

但需要：

- 配置加载
- 默认值
- validation
- serialization
- config location abstraction

注意：

> 不要硬编码 Windows 路径。

使用 Rust 跨平台路径 API。

---

# 14. Privacy-first

Prism 默认：

> local-first

M0 必须从架构上坚持：

```text
本地文件
    ↓
本地分析
    ↓
本地结果
```

不允许：

- telemetry
- analytics
- tracking
- automatic upload
- background network requests
- crash report upload
- silent external API calls

M0 完全不需要网络。

如果未来某个模块必须访问网络：

> 必须显式设计为 opt-in capability。

---

# 15. Logging

建立统一日志。

需要支持至少：

```text
DEBUG
INFO
WARN
ERROR
```

要求：

- CLI 可输出
- GUI 开发环境可看到
- release 模式不能泄露 secrets
- 不记录完整文件内容
- 不记录 API Key
- 不记录密码
- 不记录 token
- 不把整个项目源码写入日志

推荐统一：

```text
timestamp
level
target
message
context
```

而不是各模块自行 `println!()`。

---

# 16. Task System

Prism 后续会执行：

- project scan
- parser
- git diff
- health analysis
- verification
- environment detection

这些操作可能耗时。

所以 M0 建立基础 Task 抽象。

至少支持：

```text
TaskId
TaskState
Queued
Running
Completed
Failed
Cancelled
```

并预留：

```text
progress
message
started_at
finished_at
```

不要在 M0 实现复杂任务调度器。

只建立：

> 足够让 M1/M2/M3 后续安全使用的基础抽象。

---

# 17. Cancellation

任务抽象必须允许未来支持：

> 用户取消扫描。

M0 不必实现完整 GUI 取消按钮。

但底层 API 不应该把未来 cancellation 完全堵死。

推荐：

```text
CancellationToken
```

或等价抽象。

---

# 18. Progress

建立统一 progress 模型。

例如：

```text
ProgressEvent
{
    task_id
    current
    total
    percentage
    message
}
```

但不要把 progress API 写死为：

> “扫描文件”

因为未来 Git / Verify / Context 都需要使用。

应保持通用。

---

# 19. Command Execution Abstraction

Prism 后续会执行：

```text
git
cargo
npm
pnpm
python
dotnet
...
```

但 M0 不负责实现这些工具。

只建立一个通用 abstraction：

```text
CommandRunner
```

能够表达：

```text
program
args
working_directory
environment
timeout
```

结果：

```text
CommandOutput
├─ exit_code
├─ stdout
├─ stderr
└─ duration
```

注意：

> 不要在 M0 添加 Git / Cargo / Node / Python 特定逻辑。

---

# 20. Filesystem Abstraction

同样建立最小：

```text
FileSystem
```

抽象。

至少能够表达：

```text
exists
read
write
create_dir
remove
list_dir
metadata
```

但是不要为了抽象而包装 Rust std 的每一个 API。

只抽象未来需要：

> 可测试、可替换、可 mock 的边界。

---

# 21. Project Model 的基础

M0 不实现 Project Discovery。

但应该预留：

```text
Project
```

基础对象。

例如：

```text
Project
{
    id
    root_path
    name
}
```

保持非常小。

不要在 M0 添加：

```text
languages
symbols
dependencies
git
health
environment
```

这些由后续阶段产生。

---

# 22. Frontend Shell

建立最小 React 应用。

需要：

```text
App
Shell
Sidebar
MainContent
StatusBar
```

但不需要做完整 UI。

UI 暂时只需要能展示：

```text
Prism

Overview
Understand
Changes
Health
Verify
Environment
Context
Reports
```

这些页面可以是：

```text
Placeholder
```

形式。

每个页面明确标记：

```text
Coming in Mx
```

不要实现假功能。

---

# 23. GUI 设计原则

M0 不追求视觉完成度。

但建立统一：

```text
spacing
typography
layout
navigation
status
loading
error
empty state
```

基础规范。

禁止在 M0：

- 花大量时间做动画
- 做复杂图表
- 做炫酷视觉
- 做 dashboard mock
- 做假的健康评分
- 做假的架构图

现在重点是：

> **结构正确，而不是漂亮。**

---

# 24. Tauri IPC

建立最基础的：

```text
Frontend
    ↓ invoke()
Tauri command
    ↓
Application service
    ↓
Core
```

至少实现：

```text
get_app_info()
get_prism_version()
get_runtime_info()
```

这些是真实数据。

例如：

```text
Prism version
OS
Architecture
Rust runtime/build information
```

不允许：

> 返回写死的假数据。

---

# 25. CLI Foundation

创建：

```text
prism
```

命令。

M0 至少支持：

```bash
prism --help
prism --version
```

可以预留：

```bash
prism scan
prism inspect
prism changes
prism health
prism verify
prism env
prism context
prism report
```

但是：

> M0 不实现这些命令的业务逻辑。

对于尚未实现的命令，应该明确输出：

```text
Not implemented in current milestone.
```

而不是返回伪造数据。

---

# 26. CLI 与 GUI 必须共享 Core

禁止：

```text
CLI 自己实现一套逻辑
GUI 自己实现一套逻辑
```

未来必须是：

```text
             ┌─ GUI
Core Services┤
             └─ CLI
```

也就是说：

```text
GUI
 ↓
Application/Core

CLI
 ↓
Application/Core
```

而不是：

```text
GUI → 自己扫描
CLI → 自己扫描
```

虽然 M0 没有真正业务逻辑，但架构边界现在就要固定。

---

# 27. Application Layer

建立：

```text
Application Services
```

作为：

```text
Tauri
CLI
```

和：

```text
Core
```

之间的边界。

例如未来：

```text
ProjectService
GitService
AnalysisService
HealthService
VerifyService
ContextService
```

M0 可以只建立结构和一个简单：

```text
AppInfoService
```

不要预先写大量空 service。

---

# 28. TypeScript 类型原则

Rust → Frontend 的数据模型必须尽量避免：

> Rust 定义一份，TypeScript 再手写一份。

M0 至少建立：

```text
version
app_info
task_state
error response
```

等基础契约。

如果项目条件允许，建立自动生成 TypeScript bindings 的机制。

如果当前不适合自动生成：

> 明确记录技术决策，但不要人为制造大量重复类型。

---

# 29. Serialization

Rust 对外数据使用：

```text
serde
```

JSON contract 必须：

- 稳定
- 可读
- snake_case / camelCase 统一
- 不依赖内部 Rust implementation details

确定一种命名策略以后全项目保持一致。

---

# 30. 测试体系

M0 必须建立测试基础。

至少：

### Rust Unit Tests

覆盖：

- ID
- Config
- Error
- serialization
- Task state
- Command result
- path handling

### Integration Tests

至少：

```text
Core can be initialized
Config can load
AppInfo can be generated
Command abstraction can execute
```

### Frontend Tests

至少能够验证：

```text
App renders
Navigation renders
Basic shell renders
```

不要追求高 coverage 数字。

重点：

> **测试重要边界，而不是刷覆盖率。**

---

# 31. Fixture Project

建立：

```text
tests/fixtures/
```

至少包含一个非常小的假项目。

例如：

```text
fixture-basic/
├─ README.md
├─ src/
│  ├─ main.ts
│  └─ utils.ts
├─ tests/
│  └─ basic.test.ts
└─ package.json
```

这个 fixture 是未来 M1+ 的测试基础。

但 M0 不需要扫描它。

---

# 32. Development Documentation

创建：

```text
README.md
docs/architecture.md
docs/development.md
docs/roadmap.md
```

README 至少说明：

```text
What Prism is
What Prism is not
Tech stack
Repository structure
How to run
How to test
How to build
```

Architecture 文档说明：

```text
Frontend
Tauri
Application
Core
Infrastructure
CLI
```

以及：

```text
Dependency Direction
```

Roadmap 记录：

```text
M0 Foundation
M1 Project Discovery
M2 Code Understanding
M3 Dependency & Architecture
M4 Git Basics
M5 ChangeSet
M6 Change Impact
M7 Health
M8 Verify
M9 Environment
M10 Context
M11 Snapshot
M12 Product UI
M13 Release Hardening
```

注明：

> M12/M13 暂时不是当前交付目标。

---

# 33. Gitignore

检查并完善：

```text
node_modules
dist
build
target
.vscode local settings
.env
*.log
coverage
temporary files
OS metadata
```

不要把：

```text
test fixtures
docs
source
configuration examples
```

错误地 ignore。

---

# 34. Secret Hygiene

M0 必须做最小 secrets 检查。

确认仓库：

```text
不存在 API key
不存在 token
不存在密码
不存在私钥
不存在 .env
不存在个人绝对路径
```

如果发现：

> 不要擅自把真实秘密上传到 Git。

立即隔离并报告。

---

# 35. Code Quality

Rust：

- `cargo fmt`
- `cargo check`
- `cargo test`

Frontend：

- package manager install
- type check
- build
- existing lint command（如果已经存在）

不要为了 M0：

> 强行添加大量 lint 工具。

优先使用项目已经采用的工具。

---

# 36. 禁止过度工程

M0 特别容易犯的错误：

### 不要做：

```text
Plugin system
Event bus
Message broker
Database
Dependency injection framework
Generic repository pattern
Enterprise service locator
Complex state machine
Microservice-like architecture
AI abstraction framework
```

除非现有项目已经有明确需求。

原则：

> **为未来留边界，不为未来制造负担。**

---

# 37. 不要提前实现 M1+

以下能力本阶段明确禁止：

### Project Discovery

不要实现：

- 文件扫描
- language detection
- project type detection
- entry point discovery

### Parser

不要实现：

- AST
- symbol extraction
- class extraction
- function extraction

### Git

不要实现：

- git status
- git log
- git diff
- branch graph

### Analysis

不要实现：

- dependency graph
- architecture analysis
- complexity detection

### Health

不要实现：

- health score
- findings
- architecture smell
- security scan

### Verify

不要实现：

- test discovery
- build orchestration
- lint orchestration

### Context

不要实现：

- context generation
- context ranking
- AI prompt generation
- README health

### Snapshot

不要实现：

- historical snapshots
- trend graph

这些属于后续 milestone。

---

# 38. UI 的最小完成标准

完成后启动 GUI，用户应该可以：

1. 打开 Prism
2. 看到主 Shell
3. 看到导航
4. 打开 Overview
5. 看到真实 app/runtime 信息
6. 在页面之间切换
7. 看到尚未实现模块的明确状态

例如：

```text
Project Discovery
Coming in M1
```

而不是：

```text
0 projects
0 findings
Health 100
```

禁止制造虚假的产品反馈。

---

# 39. CLI 的最小完成标准

必须可以：

```bash
prism --version
```

输出真实版本。

以及：

```bash
prism --help
```

输出清晰帮助。

未来命令可以显示，但不能伪装成已经实现。

---

# 40. Acceptance Criteria

M0 最终必须满足：

## Build

```text
cargo check PASS
cargo test PASS
frontend typecheck PASS
frontend build PASS
tauri dev PASS
```

具体命令根据项目实际 package manager 调整。

---

## Runtime

GUI：

```text
启动成功
无明显 panic
无明显 console error
```

CLI：

```text
--help PASS
--version PASS
```

---

## Architecture

必须能够明确回答：

```text
Core 在哪里？
Application Layer 在哪里？
Tauri 在哪里？
Frontend 在哪里？
CLI 在哪里？
未来模块在哪里？
依赖方向是什么？
```

不得出现：

```text
Core → Frontend
Core → Tauri
Domain → React
```

之类反向依赖。

---

## Error Handling

核心代码中不存在明显：

```text
unwrap()
expect()
panic!()
```

业务路径滥用。

---

## Privacy

不存在：

```text
Telemetry
Analytics
Automatic network call
Automatic upload
```

---

## Git

提交前：

```text
git diff
git status
```

确认：

- 没有临时文件
- 没有 secrets
- 没有 debug artifacts
- 没有无意义的大文件

然后建立清晰 commit。

---

# 41. 最终审计

实现完成以后：

不要直接告诉我“完成”。

先进行一次：

# M0 Final Audit

输出：

## A. Baseline

```text
Before:
After:
```

## B. Architecture

```text
PASS / FAIL
```

说明：

- Core
- Application
- Tauri
- Frontend
- CLI
- dependency direction

## C. Build

列出实际执行的：

```text
cargo check
cargo test
frontend typecheck
frontend build
tauri build/dev
```

以及真实结果。

## D. Tests

列出：

```text
unit
integration
frontend
```

真实结果。

## E. Security / Privacy

检查：

```text
secrets
.env
absolute path
network
telemetry
```

## F. Git

输出：

```text
branch
HEAD
changed files
working tree status
commit
```

## G. Risks

仅列：

```text
P0
P1
```

不要为了凑数量制造风险。

## H. Deferred

明确列出：

```text
M1
M2
...
```

哪些工作没有实现。

---

# 42. FACT / HYPOTHESIS / INFERENCE

审计报告必须严格区分：

```text
FACT
HYPOTHESIS
INFERENCE
```

不要把推测写成事实。

例如：

```text
FACT:
cargo test 通过。

FACT:
当前没有网络模块。

INFERENCE:
当前基础架构适合进入 M1。

HYPOTHESIS:
未来大型项目扫描可能需要独立 worker pool。
```

---

# 43. P0 / P1 优先级

只有：

### P0

会阻塞后续开发或导致架构方向错误的问题。

### P1

会明显影响下一阶段开发，但目前可以继续推进的问题。

不要输出：

```text
P2
P3
Nice to have
Future idea
```

这些留到后续阶段。

---

# 44. 完成标准

当且仅当以下全部成立，才视为：

# M0 COMPLETE

```text
[ ] Tauri 2 desktop shell
[ ] React + TypeScript + Vite shell
[ ] Rust workspace
[ ] Core foundation
[ ] Application boundary
[ ] CLI foundation
[ ] IPC foundation
[ ] unified error model
[ ] logging
[ ] configuration
[ ] filesystem abstraction
[ ] command execution abstraction
[ ] task abstraction
[ ] cancellation-ready architecture
[ ] progress abstraction
[ ] basic IDs
[ ] serialization contracts
[ ] tests
[ ] fixture project
[ ] documentation
[ ] gitignore
[ ] secret hygiene
[ ] build pass
[ ] test pass
[ ] no obvious architectural violation
```

---

# 45. 执行方式

严格按照以下顺序执行：

```text
1. Baseline Audit
2. Architecture Decision
3. Minimal Implementation Plan
4. Implement
5. Unit Tests
6. Integration Tests
7. Frontend Validation
8. Full Build
9. Final Audit
10. Commit
```

不要跳过 Baseline Audit。

不要先写代码再解释架构。

---

# 46. 修改原则

每次修改：

> 优先最小变更。

不要：

- 重写整个项目
- 大规模 rename
- 无必要重构
- 无必要换依赖
- 无必要升级工具链
- 无必要引入第三方库

如果现有依赖可以完成任务：

> 保留。

如果必须增加依赖：

必须说明：

```text
Dependency
Why needed
Alternative considered
License
Maintenance impact
```

---

# 47. 依赖策略

M0 的依赖数量必须尽可能少。

优先：

```text
Tauri
Rust standard library
Serde
基础日志组件
现有 frontend stack
```

不要为了“专业”加入十几个 framework。

特别禁止：

> 一个简单问题对应一个第三方 crate。

---

# 48. 给后续 AI Agent 的设计原则

M0 完成以后，任何后续 AI Agent 都应该能够快速理解：

```text
这个文件属于哪一层？
这个模块能依赖谁？
业务逻辑应该放在哪里？
GUI 应该通过什么调用 Core？
CLI 应该通过什么调用 Core？
错误如何传播？
任务如何运行？
日志怎么写？
配置在哪里？
测试放在哪里？
```

如果这些问题无法通过：

```text
目录
架构文档
类型
接口
```

快速回答，则说明 M0 没有真正完成。

---

# 49. 重要的最终约束

这是一个长期 AI-assisted development 项目。

所以：

> **可理解性 > 炫技**
>
> **边界清晰 > 抽象复杂**
>
> **可测试 > 代码数量**
>
> **真实状态 > 假数据**
>
> **确定性 > AI 魔法**
>
> **本地优先 > 网络依赖**
>
> **可演进 > 一次性写死**

M0 的成功标准不是：

> “写了多少代码”。

而是：

> **后面的 M1～M11 能否在这个基础上持续增加，而不需要反复推翻整个架构。**

请现在开始执行 M0。

先做 Baseline Audit，再实施。

除非遇到真正阻塞的问题，不要向我反复索要确认；按照以上章程做最小、合理、可验证的工程决策。

完成后严格按照 **M0 Final Audit** 输出真实结果。

**默认不 push。**