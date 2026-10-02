# Prism M12 — Product UI / Desktop Experience

你现在进入 **ifuyo Prism M12：Product UI**。

M12 的目标不是增加新的分析引擎，也不是继续扩大 Prism 的功能边界。

M12 的唯一核心目标是：

> 把 M0–M11 已经存在的工程能力，统一成一个真正可长期使用的桌面工程工作台。

Prism 到 M11 已经能够回答：

- 项目是什么？
- 项目由什么组成？
- 代码之间如何关联？
- Git 最近发生了什么变化？
- 这些变化可能影响哪里？
- 项目有哪些风险与健康问题？
- 当前环境是什么？
- 当前任务应该给 AI 什么上下文？
- 项目过去发生了什么？
- 当前状态与历史状态相比有什么变化？

M12 要解决的是：

> **这些能力如何被工程师自然地看到、理解、操作和连接起来。**

---

# 0. 本阶段最高原则

严格遵守以下原则：

### 0.1 M12 是 Product UI，不是新能力开发阶段

不要因为 UI 缺少某个数据，就顺手重写或扩张 M1–M11 的核心引擎。

优先顺序：

1. 调用已有能力
2. 修复已有能力暴露出的 IPC / DTO / ViewModel 缺口
3. 增加必要的 UI adapter
4. 只有在确实无法工作的情况下，做最小 Core 修复

禁止：

- 借 M12 重写分析器
- 借 M12 重写 Git Engine
- 借 M12 重写 Health Engine
- 借 M12 重写 Verify Engine
- 借 M12 重写 Context Engine
- 借 M12 重写 Snapshot Engine
- 借 M12 引入 AI
- 借 M12 引入云端服务
- 借 M12 引入新的数据库体系
- 借 M12 引入复杂状态管理框架，只为“看起来专业”

---

# 1. M12 产品目标

完成后，Prism 应当从：

> “一个拥有很多命令和分析能力的工程工具”

变成：

> “一个打开项目后，可以持续查看项目状态、变化、风险、验证、环境、AI 上下文与历史演化的工程工作台。”

用户打开 Prism 后，不应该首先看到大量技术术语。

而应该快速回答：

```text
这个项目现在是什么状态？

最近发生了什么？

哪些变化值得注意？

哪里有风险？

最近验证结果如何？

当前环境有什么问题？

如果我要让 AI 帮我处理一个任务，它应该知道什么？

这个项目相比过去发生了什么？
```

---

# 2. M12 严格边界

## M12 包含

- Desktop Shell
- 全局导航
- 项目选择 / 项目打开
- Overview / Dashboard
- Project Explorer
- Code Understanding 页面
- Architecture 页面
- Changes 页面
- Impact 页面
- Health 页面
- Verify 页面
- Environment 页面
- Context 页面
- History 页面
- Snapshot Detail
- Snapshot Compare
- Timeline
- 全局搜索 / 快速定位
- 状态系统
- Loading / Empty / Error / Partial / Stale UI
- Refresh / Cancel / Retry
- 任务进度
- 通知 / Toast
- 基础 Settings
- 主题与显示密度
- 键盘快捷键
- 可访问性
- 统一 UI Design System
- 前端测试
- UI/IPC 集成测试
- 基础性能优化
- 产品级错误处理
- UI 文档

---

# 3. M12 不包含

明确禁止：

- AI Chat
- AI Agent
- LLM Provider
- OpenAI / Anthropic / Gemini API
- Embedding
- Vector DB
- RAG
- MCP Server
- 自动代码修改
- 自动修复
- Issue Tracker
- 项目管理
- 云同步
- 用户账号
- 登录
- Telemetry
- 在线项目管理
- 自动上传项目
- Remote Repository Browser
- IDE 编辑器
- debugger
- profiler
- CI 平台
- DevOps 平台

M12 不改变 Prism 的核心定位。

---

# 4. 开始之前：先审计真实项目

第一步必须是：

> **只读审计当前仓库真实状态。**

不要假设 M0–M11 已经完全按照设计实现。

首先检查：

```text
Git status
Git branch
Git HEAD
目录结构
Cargo workspace
Rust crates
Tauri commands
IPC DTO
React routes
React components
现有 state management
现有 styling system
已有 UI 页面
现有 tests
现有 build scripts
现有 CLI
```

重点寻找：

```text
M1 Discovery
M2 Code Understanding
M3 Dependency / Architecture
M4 Git
M5 ChangeSet
M6 Impact
M7 Health
M8 Verify
M9 Environment
M10 Context
M11 Snapshot
```

分别已经有哪些真实 IPC/API。

建立：

```text
M12 Existing Capability Matrix
```

字段至少包括：

```text
Capability
Backend Source
IPC Command
Request DTO
Response DTO
Frontend Consumer
Current UI
Status
Gap
```

审计结果必须区分：

```text
FACT
HYPOTHESIS
INFERENCE
```

并标记：

```text
CONFIRMED
MISSING
BROKEN
PARTIAL
```

不要根据旧文档声称“已经存在”。

以当前代码为准。

---

# 5. UI 架构原则

继续保持 Prism 原有依赖方向：

```text
React UI
   ↓
Tauri IPC
   ↓
Application / Commands
   ↓
Prism Core
```

Frontend 不允许：

```text
直接访问 SQLite
直接执行 Git
直接执行 shell
直接扫描文件系统
直接修改项目
```

UI 只能调用后端公开能力。

---

# 6. Frontend 模块边界

根据现有项目结构适配。

推荐形成类似：

```text
src/
├── app/
│   ├── router/
│   ├── shell/
│   ├── providers/
│   └── App.tsx
│
├── components/
│   ├── ui/
│   ├── layout/
│   ├── project/
│   ├── status/
│   ├── data/
│   ├── charts/
│   └── feedback/
│
├── features/
│   ├── overview/
│   ├── project/
│   ├── architecture/
│   ├── changes/
│   ├── impact/
│   ├── health/
│   ├── verify/
│   ├── environment/
│   ├── context/
│   └── history/
│
├── services/
│   ├── ipc/
│   ├── commands/
│   └── mappers/
│
├── hooks/
├── types/
├── utils/
├── styles/
└── test/
```

如果当前项目已有成熟结构，不要为了符合这个示例而机械重构。

---

# 7. 产品信息架构

Prism M12 建议最终形成以下导航：

```text
PROJECT
├── Overview
├── Project
├── Architecture
│
CHANGE
├── Changes
├── Impact
│
QUALITY
├── Health
├── Verify
├── Environment
│
AI CONTEXT
├── Context
│
HISTORY
├── History
├── Timeline
│
SYSTEM
└── Settings
```

不要做成几十个菜单。

导航必须让用户能够形成明确心智模型：

```text
Project
   ↓
Changes
   ↓
Impact
   ↓
Health / Verify
   ↓
Context
   ↓
History
```

---

# 8. Desktop Shell

设计一个固定的 Desktop Shell。

推荐：

```text
┌─────────────────────────────────────────────────────┐
│ Project Selector                         Search ... │
├──────────────┬──────────────────────────────────────┤
│              │                                      │
│ Navigation   │             Main Content              │
│              │                                      │
│ Overview     │                                      │
│ Project      │                                      │
│ Architecture│                                      │
│              │                                      │
│ Changes      │                                      │
│ Impact       │                                      │
│              │                                      │
│ Health       │                                      │
│ Verify       │                                      │
│ Environment  │                                      │
│              │                                      │
│ Context      │                                      │
│              │                                      │
│ History      │                                      │
│ Timeline     │                                      │
│              │                                      │
│ Settings     │                                      │
└──────────────┴──────────────────────────────────────┘
```

注意：

不要做网页 SaaS 风格。

Prism 是：

> Desktop Engineering Workbench

应该有：

- 高信息密度
- 清晰层级
- 稳定布局
- 少装饰
- 快速定位
- 数据优先
- 工具感

---

# 9. 视觉设计语言

建立统一 Design System。

不要让每个页面自己设计 UI。

至少统一：

```text
Typography
Spacing
Colors
Radius
Borders
Panels
Buttons
Inputs
Tabs
Tables
Tree
Badges
Status Indicators
Cards
Dialogs
Toasts
Tooltips
Code Blocks
Diff Blocks
Charts
Empty States
Error States
Loading States
```

---

# 10. 推荐视觉方向

Prism 不应该像：

```text
游戏 Dashboard
营销网站
AI Chatbot
金融 SaaS
Notion
Linear clone
```

更接近：

```text
现代 IDE
工程监控工具
代码分析工具
专业开发者桌面软件
```

整体原则：

```text
Neutral
Technical
Dense
Readable
Calm
Precise
```

不要大量使用渐变、阴影、巨大卡片、彩色插画。

---

# 11. Theme

至少支持：

```text
Dark
Light
System
```

默认：

```text
System
```

所有颜色必须语义化。

例如：

```text
--color-bg
--color-panel
--color-border
--color-text
--color-muted
--color-accent

--color-success
--color-warning
--color-error
--color-info
```

不要在 feature 页面写大量硬编码颜色。

---

# 12. 全局 Project Selector

顶部必须始终能够看到当前项目：

```text
ifuyo-prism
C:\Projects\ifuyo-prism
```

点击后：

```text
Open Project
Recent Projects
Project Path
Project Identity
Last Snapshot
```

至少支持：

```text
Open Project
Recent Projects
Remove Recent Project
```

不需要云端项目账号。

---

# 13. Overview

Overview 是整个 Prism 的入口。

目标：

> 用户打开项目后，10 秒内理解当前项目状态。

推荐结构：

```text
Project Header

Project
Path
Branch
HEAD
Last Snapshot
Working Tree State

────────────────────────

Project Structure

Files
Directories
Languages
Modules
Dependencies

────────────────────────

Recent Changes

Recent commits
Changed files
Current ChangeSet

────────────────────────

Attention

Health findings
Impact hotspots
Verification state
Environment issues
Context health

────────────────────────

Recent History

Last snapshots
Timeline preview
```

注意：

不要创建一个“总分”。

禁止：

```text
Project Health = 87
Project Quality = B+
Risk Score = 72
```

所有状态都必须保留领域语义。

---

# 14. Overview Status Cards

可以使用卡片，但卡片不能替代真实数据。

例如：

```text
Health
12 active findings
3 high severity

Verify
Last run: Passed
2m 14s ago

Environment
8 requirements
7 satisfied
1 unknown

Changes
17 changed files
4 impacted modules

Context
Health: 2 warnings
Last generated: ...
```

卡片点击后必须进入对应页面。

---

# 15. Project Explorer

Project 页面展示：

```text
Directory Tree
File Statistics
Language Distribution
Configuration Files
Entry Points
Module Boundaries
```

支持：

```text
Search
Filter
Sort
Expand
Collapse
```

点击文件后显示：

```text
Path
Language
Size
Classification
Symbols
References
Related Modules
Related Changes
```

默认不要显示完整 source code editor。

M12 不是 IDE。

---

# 16. Architecture 页面

Architecture 页面用于消费 M3。

建议：

```text
Architecture Graph
+
Module List
+
Dependency Details
+
Cycle Detection
+
Metrics
```

图必须：

- 可缩放
- 可拖拽
- 可点击
- 支持聚焦
- 支持隐藏低价值节点
- 可以从 module 跳到相关页面

点击 module：

```text
Module
Dependencies
Dependents
References
Symbols
Changes
Health Findings
```

不要把 Architecture Graph 做成纯装饰图。

---

# 17. Changes 页面

消费：

```text
M4 Git
M5 ChangeSet
```

顶部：

```text
Branch
Working Tree
HEAD
Base
```

主区域：

```text
Changed Files
```

每个文件：

```text
Added
Modified
Deleted
Renamed
Binary
```

支持：

```text
File → Symbol → Module
```

---

# 18. Changes 页面必须展示“为什么重要”

例如：

```text
src/auth/service.ts

Modified

Changed Symbols:
AuthService.login
AuthService.refreshToken

Module:
Authentication

Dependents:
API
Session
UI

Related Tests:
tests/auth/*
```

但不要在这里直接做新的 AI 风险判断。

M6 已经提供 Impact。

---

# 19. Impact 页面

Impact 页面是 Prism 非常重要的产品差异点。

页面核心结构：

```text
ChangeSet
   ↓
Changed Symbols
   ↓
Changed Modules
   ↓
Affected Components
   ↓
Related Tests / Docs
```

显示：

```text
Changed
Affected
Candidate
Unknown
```

必须展示 evidence。

例如：

```text
Why impacted?

API.UserController
   depends on
UserService

UserService.login changed
```

不要只给：

```text
HIGH IMPACT
```

而没有解释。

---

# 20. Health 页面

Health 使用 M7 Finding。

建议：

```text
Filters
Severity
Category
Status
Source
Changed Only
────────────────

Finding List
```

每个 Finding：

```text
Title
Category
Severity
Confidence
Status
Evidence
Location
First Seen
Last Seen
Related Change
Related Module
```

Finding 状态：

```text
Active
New
Resolved
Recurring
Changed
Unknown
```

M11 的历史能力必须能在这里被消费。

---

# 21. Health Finding Detail

点击 Finding 后打开详情：

```text
Finding

Description

Why detected?

Evidence

Location

Related Files

Related Symbols

Related Modules

Related Changes

History

First Seen
Last Seen
Occurrences
```

核心原则：

> Finding 必须可解释。

---

# 22. Verify 页面

消费 M8。

主结构：

```text
Verification Plan
Last Run
Check List
Result
```

每项：

```text
Build
Tests
Lint
Typecheck
Static Analysis
CI
Release
```

状态统一：

```text
Passed
Failed
Unavailable
Blocked
Skipped
Cancelled
TimedOut
Error
Stale
```

不要颜色代替文字。

---

# 23. Verify Run Detail

显示：

```text
Started
Finished
Duration
Revision
Working Tree
Command
Status
```

输出区域：

```text
stdout
stderr
structured result
```

Command 默认必须经过安全处理。

不要默认允许 UI 修改命令。

---

# 24. Environment 页面

消费 M9。

分区：

```text
Host
Runtime
SDK
Compiler
Package Manager
Environment Variables
Processes
Ports
Containers
Requirements
```

环境状态：

```text
Satisfied
Unsatisfied
Unknown
Conflict
NotApplicable
```

不能把：

```text
Unknown
```

显示成：

```text
Error
```

动态信息需要展示：

```text
Observed at
```

---

# 25. Environment Privacy

绝对禁止默认显示：

```text
SECRET
TOKEN
PASSWORD
PRIVATE KEY
ACCESS KEY
完整敏感环境变量值
```

应该显示：

```text
OPENAI_API_KEY
Present
Value hidden
```

或者：

```text
DATABASE_URL
Present
Sensitive
```

---

# 26. Context 页面

Context 是 Prism 面向 AI 的重要能力之一。

页面必须让用户理解：

> Prism 当前认为“这个任务需要知道什么”。

输入区域：

```text
Task

[ Describe the task ... ]
```

可选：

```text
Current Change
Scope
Budget
```

结果：

```text
Context Health
Context Pack
Sources
Selection
Budget
```

---

# 27. Context Pack UI

至少显示：

```text
Context Summary

Task
Scope
Generated At

Included:
Files
Symbols
Modules
Changes
Impact
Health
Verification
Environment
Documentation

Budget:
Files
Bytes
Estimated Tokens
```

支持：

```text
Preview
Explain Selection
Export Markdown
Export JSON
Copy
```

---

# 28. Context Explain

用户应该能够知道：

```text
为什么这个文件进入 Context？

为什么这个文件没有进入？

为什么这个 Symbol 优先级更高？

这个 Context 来源是什么？

它是否新鲜？
```

例如：

```text
src/auth/service.ts

Included because:
1. directly changed
2. referenced by impacted module
3. related health finding

Priority:
Critical

Source:
ChangeSet
ImpactAnalysis
HealthFinding
```

这是 Prism 与普通“代码打包器”之间的重要区别。

---

# 29. History 页面

消费 M11。

展示：

```text
Snapshot List
```

每个 Snapshot：

```text
Timestamp
Revision
Working Tree State
Note
State Summary
```

例如：

```text
2026-10-02 21:38

HEAD abc1234
Working Tree: Dirty

17 files changed
4 impacted modules
3 active findings
Verify: Passed
Environment: 1 unknown
```

支持：

```text
Open
Compare
Delete
Export
```

---

# 30. Snapshot Detail

Snapshot Detail 必须展示：

```text
Identity
Timestamp
Revision
Working Tree State
Source

Structure
Code
Architecture
Git
Changes
Impact
Health
Verification
Environment
Context Health
```

每个 Domain 要显示：

```text
Available
Freshness
Captured At
Fingerprint
Limitations
```

不要隐藏：

```text
Unavailable
Partial
Stale
```

---

# 31. Snapshot Compare

这是 M11 的核心 UI。

页面结构：

```text
Snapshot A

        VS

Snapshot B
```

下面：

```text
Project Structure
Code Structure
Architecture
Git
Changes
Impact
Health
Verification
Environment
Context Health
```

例如：

```text
Files
+12
-4

Modules
+2
-1

Dependencies
+7
-3

Health
New: 2
Resolved: 5

Verification
Passed → Failed

Node.js
20.18 → 22.1
```

所有变化必须基于实际 SnapshotDiff。

不要重新猜测。

---

# 32. Timeline

Timeline 是 M11 的第二个重要 UI。

时间线展示：

```text
Git Commit
Snapshot
Architecture Changed
Health Changed
Verification Changed
Environment Changed
Documentation Changed
Context Health Changed
```

每条事件：

```text
Timestamp
Kind
Source
Summary
Related Snapshot / Commit
```

禁止主观事件：

```text
Architecture improved
Project got healthier
Developer made progress
```

只能使用可验证事实：

```text
2 modules removed
1 dependency cycle introduced
3 findings resolved
Verification changed Passed → Failed
```

---

# 33. Global Search

M12 推荐加入一个轻量级全局搜索。

快捷键：

```text
Ctrl/Cmd + K
```

搜索：

```text
Pages
Files
Modules
Symbols
Findings
Commits
Snapshots
```

不要在 M12 引入复杂全文检索数据库。

优先使用现有索引和内存数据。

---

# 34. Command Palette

提供：

```text
Open Project
Refresh
Run Verification
Capture Snapshot
Open History
Open Context
Open Health
Open Changes
Open Architecture
Export Report
Settings
```

所有 command 必须调用现有能力。

不要建立第二套业务命令体系。

---

# 35. Refresh 模型

统一 Refresh 机制。

页面必须知道：

```text
Not Loaded
Loading
Refreshing
Loaded
Partial
Stale
Error
```

刷新时：

```text
保留旧数据
显示 refreshing 状态
完成后替换
失败时保留旧数据并显示错误
```

不要：

```text
刷新 → 整页白屏
```

---

# 36. Task / Progress UI

Prism 中很多动作可能耗时：

```text
Scan
Parse
Architecture Analysis
Health Analysis
Verification
Environment Refresh
Context Generation
Snapshot Capture
```

统一使用 Task 状态：

```text
Queued
Running
Completed
Failed
Cancelled
```

显示：

```text
Progress
Current Step
Elapsed Time
Cancel
```

必须支持取消。

不要让用户认为应用已经卡死。

---

# 37. Error UX

错误分三类：

### User Error

例如：

```text
Project path invalid
No Git repository
```

### Capability Unavailable

例如：

```text
dotnet not installed
npm unavailable
```

### Internal Error

例如：

```text
IPC failure
Database error
Unexpected parser error
```

不要全部显示：

```text
Something went wrong
```

错误必须包含：

```text
What happened
Why
What can user do
Technical details
```

Technical details 可折叠。

---

# 38. Empty State

每个页面必须有真正的 Empty State。

例如 Changes：

```text
No changes detected.

Working tree is clean.
```

Impact：

```text
No active ChangeSet.

Run or refresh project changes first.
```

History：

```text
No snapshots yet.

Capture the first project snapshot to begin history.
```

Context：

```text
Describe a task to generate a context pack.
```

不要使用空白页面。

---

# 39. Stale State

这是 Prism 必须认真处理的状态。

例如：

```text
Health
Last analyzed:
14:25

Working tree changed:
14:32
```

不能让 UI 看起来像：

```text
Health = current
```

应该显示：

```text
Stale
Captured 7 minutes ago
Refresh
```

同理：

```text
Environment
Verification
Context Health
Snapshot domain
```

---

# 40. Cross-page Context

页面之间必须能够连续导航。

例如：

```text
Health Finding
   ↓
Related Module
   ↓
Architecture
   ↓
Dependents
   ↓
Changes
   ↓
Impact
```

不要让用户必须重新寻找同一对象。

统一对象引用：

```text
file
symbol
module
finding
change
snapshot
commit
```

点击后进入对应 Detail 或页面 focus。

---

# 41. Deep Links / Route State

尽可能让 URL / route 能表达：

```text
/project
/architecture?module=auth
/health?finding=...
/changes?file=...
/history/snapshot/...
/history/compare/a/b
/context?task=...
```

Tauri desktop 环境下根据实际 router 体系实现。

目标：

> 页面状态可以被重现，而不是所有状态都藏在临时 React state 中。

---

# 42. Table / List 设计

工程数据优先使用：

```text
table
tree
list
split pane
detail panel
```

而不是满屏卡片。

适合 Table 的数据：

```text
Findings
Changed Files
Dependencies
Checks
Environment Requirements
Snapshots
Timeline Events
```

统一支持：

```text
sort
filter
selection
keyboard navigation
```

---

# 43. Detail Panel

推荐大量使用：

```text
Master / Detail
```

例如：

```text
┌──────────────────┬─────────────────────────┐
│ Finding List     │ Finding Detail          │
│                  │                         │
│ CTX001           │ Description             │
│ ARCH003          │ Evidence                │
│ DEP002            │ Location                │
│                  │ History                 │
└──────────────────┴─────────────────────────┘
```

这样减少页面跳转。

---

# 44. Responsive / Window Size

Prism 不是手机应用。

优先优化：

```text
1280×800
1440×900
1920×1080
```

必须避免：

```text
固定过宽
固定过窄
内容被裁切
横向滚动满屏
```

对于非常小窗口：

```text
sidebar collapse
detail stack
```

但不要为了“响应式”建立复杂 CSS Framework。

---

# 45. Accessibility

至少支持：

```text
Keyboard navigation
Visible focus
Semantic buttons
Tooltip
ARIA where needed
Readable contrast
No color-only status
```

例如：

不能只显示绿色圆点：

```text
●
```

必须至少存在：

```text
Passed
```

---

# 46. Keyboard Shortcuts

至少：

```text
Ctrl/Cmd + K
Global Search / Command Palette

Ctrl/Cmd + R
Refresh

Ctrl/Cmd + Shift + R
Hard Refresh current capability

Esc
Close dialog / palette

?
Keyboard Shortcut Help
```

如果现有项目已有快捷键体系，以现有体系为准。

---

# 47. Notifications / Toast

只用于：

```text
operation completed
operation failed
snapshot captured
export completed
copy succeeded
```

不要滥用：

```text
Health changed!
Environment changed!
AI context ready!
```

Prism 不应该不断打扰用户。

---

# 48. Settings

M12 Settings 只处理产品 UI / 本地行为。

例如：

```text
Appearance
Theme
Density

Behavior
Auto Refresh
Default Project
Confirm Destructive Actions

Privacy
Sensitive value display
Redaction

Performance
Background task concurrency
```

禁止把 M12 Settings 变成：

```text
AI Provider Management
Cloud Account
Telemetry Dashboard
```

这些不是当前产品范围。

---

# 49. No Fake Data

强制要求：

> M12 不允许使用 fake/mock/demo data 冒充真实功能。

开发阶段可以使用：

```text
fixture
test data
```

但生产 UI 必须：

```text
真实 IPC
真实 backend result
真实 project
真实 Git
真实 Snapshot store
```

---

# 50. Loading Skeleton

可以使用 skeleton，但必须适量。

数据列表：

```text
Rows skeleton
```

复杂图：

```text
Graph loading
```

不要让所有东西同时闪烁。

---

# 51. Backend DTO 与 ViewModel

不要让 React 组件直接理解 Rust 内部复杂类型。

建议：

```text
IPC DTO
   ↓
Frontend Mapper
   ↓
ViewModel
   ↓
UI
```

例如：

```text
CheckRunDto
    ↓
VerifyCheckViewModel
```

这样以后后端变化不会污染 UI。

---

# 52. IPC 错误模型

统一 IPC error handling。

不要每个页面：

```ts
try {}
catch {}
```

写一套完全不同的错误解析。

统一：

```text
PrismIpcError
ErrorCode
Message
Details
Recoverability
```

UI 再决定：

```text
toast
inline error
dialog
retry
```

---

# 53. State Management

先检查当前项目是否已经有：

```text
Zustand
Redux
Context
Signals
TanStack Query
```

已有方案优先复用。

不要为了 M12 引入重量级状态管理。

推荐的状态边界：

```text
UI state
Local component state

Server/application state
IPC + cache/query layer

Navigation state
Router

Task state
Global task store
```

---

# 54. Cache

M12 不建立新的业务缓存系统。

可以做：

```text
query cache
memoization
frontend derived state
```

但：

> 真正的数据一致性仍然由 Prism Core / Snapshot / existing engines 决定。

---

# 55. Performance

重点：

不要让 Overview 同时触发：

```text
完整扫描
完整解析
完整依赖分析
完整 Health
完整 Verify
完整 Environment
完整 Context
完整 Snapshot
```

打开项目时必须按需加载。

推荐：

```text
Project Identity
    ↓
Basic Discovery
    ↓
Overview
    ↓
Lazy sections
```

Architecture 页面再加载 Architecture。

History 页面再加载 History。

Context 页面只有进入时才加载 Context 数据。

---

# 56. Cancellation

用户离开页面时：

不能留下大量无主任务。

例如：

```text
打开 Architecture
开始加载
马上切到 Health
```

应该：

```text
取消或让任务脱离 UI
```

并且不能产生：

```text
state update on unmounted component
```

---

# 57. Visual Hierarchy

优先级：

```text
Current State
↓
Important Changes
↓
Risk / Problems
↓
Evidence
↓
Raw Details
```

不要：

```text
Raw Details
↓
Raw Details
↓
Raw Details
↓
Raw Details
```

---

# 58. Product Language

UI 文案必须准确。

不要使用：

```text
AI magically understands your code
Perfect project health
Everything looks great
Your architecture is excellent
```

推荐：

```text
Detected
Analyzed
Observed
Affected
Unavailable
Stale
Unknown
No evidence
```

Prism 是工程工具，不是营销页面。

---

# 59. No Overall Score

再次强调：

整个 M12 任何地方都不能出现：

```text
Overall Score
Health Score
Quality Score
Architecture Score
Project Grade
AI Readiness Score
```

可以显示领域状态：

```text
12 active findings
3 high severity

14/16 checks passed

4 impacted modules

2 environment requirements unknown
```

事实比分数更重要。

---

# 60. Product Home

Overview 首页必须让用户看到：

```text
PROJECT
──────────────────

Name
Path
Branch
HEAD
Dirty / Clean

──────────────────

WHAT CHANGED

Changed files
Changed symbols
Impacted modules

──────────────────

WHAT NEEDS ATTENTION

Health findings
Verification
Environment
Context health

──────────────────

WHAT HAPPENED RECENTLY

Commits
Snapshots
Timeline

──────────────────

WHAT AI NEEDS TO KNOW

Context health
Current task context
```

这是 Prism 的核心首页。

---

# 61. Navigation Logic

推荐：

```text
Overview
  ├── Project
  ├── Architecture
  ├── Changes
  ├── Impact
  ├── Health
  ├── Verify
  ├── Environment
  ├── Context
  └── History
```

不要孤立页面。

所有页面必须互相连接。

---

# 62. Product-level Cross-links

至少实现以下关系：

```text
File
 ↔ Symbol
 ↔ Module

Changed File
 ↔ Changed Symbol
 ↔ ChangeSet
 ↔ Impact

Finding
 ↔ File
 ↔ Module
 ↔ Change

Check
 ↔ Revision
 ↔ Snapshot

Snapshot
 ↔ Timeline
 ↔ Compare

Context Item
 ↔ Source
 ↔ Finding
 ↔ Change
```

---

# 63. Snapshot UI 特别要求

M11 是 M12 必须重点消费的能力。

Snapshot Detail 不要只是：

```text
JSON viewer
```

必须是：

```text
工程状态快照
```

用户应该能够快速看到：

```text
What changed
What became different
What stayed the same
```

---

# 64. History Compare Visuals

变化应该使用明确的：

```text
Added
Removed
Changed
Unchanged
Unknown
Unavailable
```

对于数字：

```text
17 → 21
+4
```

对于状态：

```text
Passed → Failed
```

对于集合：

```text
Added:
A
B

Removed:
C
```

---

# 65. Timeline Filter

支持：

```text
All
Git
Snapshot
Architecture
Health
Verification
Environment
Documentation
Context
```

但这些过滤器必须基于真实 event kind。

---

# 66. Global Status Bar

可以在底部放置：

```text
Project
Branch
Working Tree
Last Analysis
Task
```

例如：

```text
ifuyo-prism · main · dirty
Last analyzed 2m ago
```

不要塞太多信息。

---

# 67. First Launch Experience

首次启动：

```text
Welcome to Prism

Open a local project

[ Open Project ]
```

不要要求：

```text
Sign in
Create account
Connect AI
Enable telemetry
```

---

# 68. Recent Projects

显示：

```text
Project Name
Path
Last Opened
Last Snapshot
```

仅本地。

如果项目不存在：

```text
Missing
Locate
Remove
```

---

# 69. Project Switching

切换项目必须：

```text
cancel current UI tasks where safe
clear project-scoped state
load new ProjectIdentity
load project data
```

防止：

```text
Project A findings
显示在
Project B
```

这是 P0 级数据隔离问题。

---

# 70. Multi-project Isolation

测试至少覆盖：

```text
Project A
Project B
```

确保：

```text
history
snapshots
findings
changes
environment
context
cached state
```

不会串项目。

---

# 71. Persistence

不要把 UI state 随意塞 SQLite。

需要持久化的内容：

```text
Project Registry
Snapshot History
M11 domain data
```

适合 local UI preference 的：

```text
theme
sidebar width
density
last opened page
```

可以使用本地 configuration。

---

# 72. Destructive Actions

例如：

```text
Delete Snapshot
Clear History
Remove Recent Project
```

必须有：

```text
confirmation
```

例如：

```text
Delete snapshot?

This only removes Prism's local history.
It does not delete source files or Git history.
```

---

# 73. Never Mutate User Project From UI

M12 不允许：

```text
git reset
git checkout
git clean
git commit
git push
git pull
```

除非未来明确阶段定义。

当前所有页面默认：

```text
read-only engineering intelligence
```

---

# 74. Reports

复用已有 M7/M8/M10/M11 export 能力。

UI 提供：

```text
Export JSON
Export Markdown
Export HTML
```

不要 M12 重新实现一套 reporting engine。

---

# 75. Report Preview

如果现有能力允许，可以提供：

```text
Preview
Export
Copy
```

但 preview 必须复用 backend report data。

---

# 76. Product Consistency

建立 UI 组件规范：

```text
PageHeader
SectionHeader
Stat
StatusBadge
SeverityBadge
DataTable
TreeView
DetailPanel
EmptyState
ErrorState
LoadingState
StaleState
TaskIndicator
FilterBar
SearchInput
ConfirmDialog
CodeBlock
DiffViewer
Timeline
```

这些组件应该真正复用。

---

# 77. Avoid Component Explosion

不要出现：

```text
HealthCard.tsx
HealthCardV2.tsx
HealthCardCompact.tsx
HealthCardNew.tsx
HealthCardReusable.tsx
```

组件粒度必须服务真实复用。

---

# 78. Testing Strategy

必须增加前端测试。

至少覆盖：

## Unit

```text
DTO mapper
formatters
status mapping
route parsing
filtering
sorting
view model generation
```

## Component

```text
Health List
Verify List
Changes List
Snapshot List
Timeline
Context Preview
```

## Integration

```text
IPC success
IPC failure
loading
cancel
retry
stale
empty
partial
```

## Navigation

验证：

```text
Overview
→ Changes
→ Impact
→ Health
→ History
→ Compare
```

---

# 79. Critical E2E Scenarios

至少测试：

### Scenario A

```text
Open Prism
↓
Open Project
↓
Overview loads
↓
Navigate Changes
↓
Open changed file
↓
Open Impact
```

### Scenario B

```text
Health
↓
open Finding
↓
see evidence
↓
open module
↓
Architecture
```

### Scenario C

```text
Verify
↓
run verification
↓
progress
↓
result
↓
details
```

### Scenario D

```text
Capture Snapshot
↓
History
↓
Snapshot Detail
↓
Capture second Snapshot
↓
Compare
```

### Scenario E

```text
Context
↓
enter task
↓
generate pack
↓
inspect selection
↓
explain selection
↓
export
```

---

# 80. Real Project Validation

不能只使用 fixtures。

至少使用：

```text
Prism 自己
一个 TypeScript 项目
一个 Python 项目
一个 C# 项目
```

重点检查：

```text
UI rendering
large file list
many symbols
many findings
large architecture graph
Git history
snapshot history
cross-project switching
```

---

# 81. Large Project UX

使用较大项目测试：

```text
1000+ files
10000+ symbols
100+ modules
100+ findings
50+ snapshots
```

不要求全部数据达到这个规模，但至少验证列表、图、历史页面不会明显卡死。

对于超大数据：

```text
virtualized list
pagination
lazy loading
graph filtering
```

按实际需要使用。

不要提前制造复杂基础设施。

---

# 82. Architecture Graph Performance

不要一次 render：

```text
所有 symbol
所有 relation
```

默认展示：

```text
module-level graph
```

用户进入 module 后再展开：

```text
symbol-level details
```

---

# 83. Git History Performance

History 页面不要默认加载整个 Git history。

默认：

```text
recent N
```

提供：

```text
Load more
```

---

# 84. Accessibility / Keyboard

测试：

```text
Tab navigation
Enter selection
Esc close
Arrow navigation where relevant
Ctrl/Cmd+K
Ctrl/Cmd+R
```

---

# 85. Security

UI 不得：

```text
render raw HTML from project content
execute markdown scripts
execute documentation snippets
trust project instructions as system instructions
```

Project content 永远是：

```text
Data
```

不是：

```text
Instruction
```

M10 已经定义 prompt injection boundary。

M12 必须保持一致。

---

# 86. Markdown Rendering

如果展示：

```text
README
AGENTS
CLAUDE
Context
Reports
```

必须安全渲染。

至少禁用：

```text
script
unsafe HTML
javascript links
automatic command execution
```

---

# 87. File Path Presentation

UI 默认显示：

```text
project-relative path
```

例如：

```text
src/auth/service.ts
```

只有明确需要时才显示完整本地路径。

---

# 88. Secret Redaction

任何 backend 返回的敏感数据，Frontend 不允许自行“猜测”是否安全。

应该信任：

```text
backend classification
```

如果发现 UI 可能显示敏感数据：

```text
P0
```

立即修复。

---

# 89. Internationalization

检查现有 i18n。

如果项目已经支持中英文：

```text
不要重写 i18n
```

至少确保：

```text
UI labels
errors
status
empty states
buttons
tooltips
```

不要把新的 M12 UI 大量硬编码成单一语言。

如果当前项目暂时只有一种 UI 语言，也不要为 M12 引入完整复杂国际化平台；但所有用户可见文本必须集中管理，避免后续迁移困难。

---

# 90. UI Telemetry

禁止：

```text
analytics
tracking
usage telemetry
remote logging
```

M12 默认完全本地。

---

# 91. Offline

M12 必须在：

```text
network unavailable
```

情况下正常工作。

不能因为：

```text
font CDN
icon CDN
remote assets
analytics endpoint
```

而出现 UI 功能依赖网络。

所有 UI 依赖必须本地打包。

---

# 92. Icons

优先复用当前已有 icon system。

如果没有：

选择一个轻量、稳定、MIT/compatible 的本地 icon library。

不要：

```text
下载大量 SVG 到仓库
```

不要：

```text
在线 CDN icon
```

---

# 93. Animations

动画只用于：

```text
navigation
expand/collapse
loading
feedback
```

速度克制。

不要：

```text
大量粒子
渐变动画
无限旋转
页面进入大幅移动
```

Prism 是工程工具。

---

# 94. P0 产品问题

M12 出现以下任意问题必须优先修复：

```text
Project A / B 数据串线
Snapshot 数据错误
Compare 结果错误
Health Finding 显示错误
Verify 状态错误
Context selection 与 backend 不一致
Environment sensitive value 泄露
UI 执行用户 project mutation
IPC 与真实 backend 不一致
Stale 数据伪装为 current
History 数据丢失
应用启动失败
关键页面无法打开
```

---

# 95. P1 产品问题

包括：

```text
明显 UI 布局问题
列表大量卡顿
导航不一致
错误状态不清晰
键盘导航问题
暗色/亮色主题错误
国际化缺失
某页面数据展示不完整
图表交互异常
Export UI 异常
Tooltip 缺失
```

---

# 96. 不要花时间处理

M12 暂不优先：

```text
极端视觉炫技
复杂动画
3D
WebGL
高级图表动画
AI avatar
聊天气泡
营销 landing page
用户账号
云同步
插件市场
Marketplace
```

这些都不提高 Prism 核心产品价值。

---

# 97. Product UX 核心原则

整个界面必须遵守：

### Principle 1

> Evidence over score

### Principle 2

> State over decoration

### Principle 3

> Explain over label

### Principle 4

> Freshness over illusion

### Principle 5

> Navigation over duplication

### Principle 6

> Reuse Core over frontend reimplementation

### Principle 7

> Deterministic data over AI-generated interpretation

---

# 98. 页面最终最低要求

M12 完成后必须至少存在以下真实页面：

```text
Overview
Project
Architecture
Changes
Impact
Health
Verify
Environment
Context
History
Timeline
Settings
```

并且每个页面：

```text
真实数据
真实 IPC
真实 loading
真实 empty
真实 error
真实 stale / unavailable（适用时）
```

不能只是 UI mock。

---

# 99. M12 不要求视觉最终定稿

M12 的 UI 是：

> Functional Product UI

不是最终商业视觉。

因此：

```text
M12
= 信息架构 + 交互 + 产品化 + 可用性

M13
= Performance + Hardening + Release
```

不要为了追求所谓“Pixel Perfect”而牺牲架构和功能正确性。

---

# 100. M12 实施顺序

严格按照以下顺序：

## M12.0

Baseline Audit

## M12.1

Design System

## M12.2

Desktop Shell + Navigation

## M12.3

Project Selector + Global Task State

## M12.4

Overview

## M12.5

Project + Architecture

## M12.6

Changes + Impact

## M12.7

Health + Verify

## M12.8

Environment

## M12.9

Context

## M12.10

History + Snapshot Detail

## M12.11

Snapshot Compare + Timeline

## M12.12

Global Search / Command Palette

## M12.13

Settings + Theme + Preferences

## M12.14

Accessibility + Keyboard

## M12.15

Loading / Error / Empty / Stale Polish

## M12.16

Performance Audit

## M12.17

Integration / E2E

## M12.18

Product UI Audit

---

# 101. 每个子阶段的执行纪律

每一步都必须：

```text
1. Inspect current implementation
2. Identify gap
3. Make minimum necessary change
4. Run targeted tests
5. Run build/typecheck/lint
6. Inspect affected UI
7. Verify IPC contract
```

不要：

```text
一次性重写整个 frontend
```

---

# 102. Visual Review

完成主要页面后必须实际启动 Tauri Desktop app。

不能只：

```text
cargo test
npm test
npm build
```

必须实际看：

```text
1280×800
1440×900
1920×1080
```

至少人工检查：

```text
layout
overflow
spacing
font
alignment
navigation
empty states
dark mode
light mode
dialogs
tables
graph
timeline
```

---

# 103. M12 Audit

最终必须执行完整 Product UI Audit。

输出：

```text
# M12 Product UI Audit

## A. Baseline

## B. Architecture

## C. Navigation

## D. Overview

## E. Project / Architecture

## F. Changes / Impact

## G. Health / Verify

## H. Environment

## I. Context

## J. History / Timeline

## K. State Handling

## L. Error Handling

## M. Accessibility

## N. Security / Privacy

## O. Performance

## P. Testing

## Q. Real Project Validation

## R. Git

## S. Final Risk
```

每条结论必须使用：

```text
FACT
HYPOTHESIS
INFERENCE
```

问题只标：

```text
P0
P1
```

并标：

```text
RESOLVED
CONFIRMED
WORSENED
```

禁止使用：

```text
perfect
100%
production ready
无问题
```

除非有真实证据支持。

---

# 104. Git 要求

在开始前：

```bash
git status
git branch
git log -1
```

M12 所有修改完成并通过验证后：

```text
git status
git diff
git diff --stat
```

确认：

```text
没有无关文件
没有临时文件
没有 node_modules
没有 build artifacts
没有 local DB
没有 secrets
没有 local paths
```

然后提交。

推荐：

```text
feat: productize prism desktop ui
```

只 commit。

不要 push。

---

# 105. M12 最终验收标准

只有同时满足下面条件，才算 M12 完成：

## Product

```text
Desktop Shell 可用
导航完整
项目切换可用
Overview 可用
Project 可用
Architecture 可用
Changes 可用
Impact 可用
Health 可用
Verify 可用
Environment 可用
Context 可用
History 可用
Timeline 可用
Settings 可用
```

## Data

```text
真实 IPC
真实 backend data
没有 fake production data
没有 project 数据串线
Snapshot 正确
Compare 正确
Finding 正确
Verify 状态正确
Context 正确
Environment privacy 正确
```

## UX

```text
Loading
Empty
Error
Partial
Unavailable
Stale
Refresh
Retry
Cancel
```

均得到统一处理。

## Performance

```text
启动正常
项目打开正常
页面切换正常
大列表可用
Architecture Graph 可用
History 可用
```

## Security

```text
No telemetry
No automatic upload
No secret exposure
No project mutation
No documentation execution
No unsafe HTML
```

## Compatibility

```text
CLI 继续可用
Tauri IPC 正常
Rust tests 通过
Frontend tests 通过
Build 通过
```

## Validation

```text
Prism 自身项目
TypeScript 项目
Python 项目
C# 项目
```

至少真实验证核心页面。

---

# 106. 最重要的产品判断

M12 不应该让 Prism 看起来：

> “有很多功能。”

而应该让它看起来：

> “这些能力本来就应该在一个地方。”

最终产品心智模型应该非常简单：

```text
                PRISM
                  │
       ┌──────────┼──────────┐
       │          │          │
     PROJECT    CHANGE     QUALITY
       │          │          │
       │       ┌──┴──┐    ┌──┴──┐
       │       │     │    │     │
   Structure  Diff  Impact Health Verify
       │                       │
Architecture              Environment
       │
       └──────────┬────────────┘
                  │
               CONTEXT
                  │
               HISTORY
                  │
              TIMELINE
```

Prism 的 UX 不需要向用户解释所有内部架构。

用户只需要始终能够回答：

```text
我现在在哪里？

我正在看的是什么？

这个数据从哪里来的？

它什么时候采集的？

它为什么重要？

我下一步可以去哪里？
```

---

# 107. 最终定位

M12 完成之后，Prism 应该正式具备这样的产品形态：

> **一个本地运行、以工程事实为核心、能够持续理解项目当前状态、变化、风险、验证、环境、AI 上下文与历史演化的 Desktop Engineering Intelligence Workbench。**

M0–M9 负责：

> Understand / Change / Impact / Health / Verify / Environment

M10 负责：

> Context

M11 负责：

> History / Snapshot / Evolution

M12 负责：

> **把它们变成真正的产品。**

因此：

> **M12 不再追求“功能更多”，而追求“已有能力终于成为一个完整的工具”。**

执行时始终遵循：

```text
真实代码优先
已有能力优先
最小修改
数据正确优先于视觉
可解释优先于炫技
稳定优先于复杂
产品体验优先于 Demo 感
```

M12 完成后不要进入新的功能扩张。

下一阶段 M13 才负责：

```text
Performance
Caching
Cancellation
Cross-platform
Release
Installer
Packaging
Release Audit
```