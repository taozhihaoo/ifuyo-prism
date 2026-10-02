# ifuyo Prism
## 开发总纲领 / Project Charter v1.0

> **核心定位：**
> **Prism 是一个 AI-native、local-first 的项目理解与工程体检工作台。**
>
> 它不负责“替你写项目”，而负责让你和 AI **看懂项目、看懂变化、找到风险、验证结果，并生成可靠的项目上下文。**

---

# 1. 产品定位

## 1.1 Prism 解决的问题

大型或长期演化的代码项目，真正困难的通常不是“怎么写一段代码”，而是：

- 这个项目现在到底是什么结构？
- 哪些模块依赖哪些模块？
- 这次修改究竟影响了什么？
- 哪些地方正在腐化？
- 哪些文档已经过时？
- 当前环境到底能不能正确构建和测试？
- 我应该给 AI 哪些上下文？
- AI 修改之后，项目到底有没有变坏？

Prism 围绕这条链路构建：

```text
看懂项目
   ↓
看懂变化
   ↓
发现风险
   ↓
验证项目
   ↓
生成可靠上下文
```

---

# 2. 产品边界

## 2.1 Prism 做什么

Prism 的对象是：

> **一个软件项目 / Git Repository**

核心能力分为五大领域：

### A. 看懂 Understand

让用户快速建立项目心智模型。

包括：

- 项目目录结构
- 模块划分
- 文件类型统计
- Symbol / Class / Function
- 模块依赖
- 调用关系
- Entry Points
- 配置文件
- 构建系统
- 测试结构
- 外部依赖
- Architecture Graph
- Project Overview
- Project Context

目标：

> **打开一个陌生项目后，可以快速知道“它是什么、怎么组织、从哪里开始、各部分怎么连接”。**

---

### B. 变更 Change

让用户知道：

> **“这次改动到底改变了什么，以及可能影响什么。”**

包括：

- Git Status
- Commit
- Commit Range
- Diff
- File Change
- Symbol Change
- Dependency Change
- Changed Module
- Impact Analysis
- Before / After Snapshot
- Change Risk
- Related Tests
- Related Documentation

特别强调：

```text
Diff
 ↓
Changed Files
 ↓
Changed Symbols
 ↓
Dependencies
 ↓
Potential Impact
 ↓
Affected Tests / Docs
```

这是 Prism 的核心差异能力之一。

---

### C. 体检 Health

不是简单统计 lint 数量，而是建立：

> **Project Health Model**

包括：

- Code Quality
- Architecture Risk
- Dependency Risk
- Security / Secret Scan
- TODO / FIXME
- Technical Debt
- Dead / Orphaned Files
- Suspicious Dependencies
- Circular Dependencies
- Documentation Drift
- Configuration Problems
- Build Risk
- Test Coverage Signals
- Change Hotspots

输出：

```text
Finding
 ├─ Severity
 ├─ Category
 ├─ Location
 ├─ Evidence
 ├─ Why
 └─ Suggested Next Step
```

Prism 不应该凭空“评价代码”。

所有 Finding 必须尽可能建立在：

> **可定位、可复现、可解释的 Evidence**

之上。

---

### D. 验证 Verify

回答：

> **“项目现在到底能不能工作？”**

包括：

- Build
- Test
- Lint
- Type Check
- Format Check
- Static Analysis
- Package / Publish
- CI Configuration
- Release Readiness
- Documentation Consistency

Prism 不应该重新实现各种语言工具。

原则：

```text
Prism = Orchestrator

Compiler / Test Runner / Linter
        ↓
External Tool

Prism
        ↓
统一发现
统一执行
统一解析
统一展示
统一报告
```

---

### E. 环境 Environment

回答：

> **“这个项目运行所需要的环境现在是什么状态？”**

包括：

- OS
- Runtime
- SDK
- Compiler
- Package Manager
- Git
- External Tools
- Environment Variables
- Port Usage
- Process
- Tool Version
- Config Discovery
- Missing Dependency
- Environment Mismatch

重点不是成为系统监控软件，而是：

> **解释“当前环境是否适合这个项目”。**

---

# 3. Prism 最重要的差异化能力

Prism 不应该成为另一个：

- IDE
- Git GUI
- Linter Dashboard
- Dependency Viewer
- AI Chat Window

真正应该形成 Prism 自己的四个核心能力。

## 3.1 Context Health

判断：

> **“项目提供给 AI / 开发者的上下文是否可信？”**

例如：

```text
README
AGENTS.md
CLAUDE.md
Architecture.md
Build Instructions
Project Structure
```

Prism 检查：

- 文件是否存在
- 路径是否仍然存在
- 命令是否有效
- 文档描述是否与代码冲突
- 模块是否已经发生变化
- 项目入口是否变化
- 配置是否过时

形成：

```text
Context Health
├─ Valid
├─ Stale
├─ Conflicting
└─ Missing
```

这是 Prism 的关键身份之一。

---

## 3.2 Change Impact

不是只告诉用户：

> `src/foo.ts changed`

而是继续分析：

```text
foo.ts
 ↓
Changed Symbol
 ↓
Dependent Modules
 ↓
Potentially Affected Components
 ↓
Tests
 ↓
Documentation
```

最终回答：

> **“这次改动可能影响哪里？”**

---

## 3.3 Risk-weighted Context

Prism 生成 AI Context 时，不应该简单把项目全部代码塞进去。

应该根据：

- 当前任务
- 当前 Diff
- Dependency Graph
- Project Structure
- Recent Changes
- Findings
- Relevant Docs

选择：

> **最相关、最可信、风险最高的上下文。**

输出：

```text
Context Pack
├─ Project Overview
├─ Relevant Files
├─ Relevant Symbols
├─ Dependencies
├─ Recent Changes
├─ Known Risks
├─ Relevant Tests
└─ Relevant Documentation
```

---

## 3.4 Project Timeline / Snapshot

Prism 不只看“现在”。

应该能够逐渐形成：

```text
Snapshot A
   ↓
Commit
   ↓
Snapshot B
   ↓
Commit
   ↓
Snapshot C
```

观察：

- 项目规模变化
- 模块变化
- 依赖变化
- 风险变化
- Findings 变化
- Test / Build 状态变化

最终形成：

> **Project Evolution**

而不是单次扫描工具。

---

# 4. AI 原则

## 4.1 Prism 不自己造大模型

Prism **不内置自研模型层**。

核心分析必须：

> **Deterministic First**

即：

```text
代码
 ↓
Parser / Analyzer
 ↓
Facts
 ↓
Findings
 ↓
Context
 ↓
AI（可选）
```

AI 的角色主要是：

- 总结
- 解释
- 关联
- 生成上下文
- 辅助排障

而不是替代基础事实分析。

---

## 4.2 AI 不能成为核心依赖

即使没有 AI Provider：

- Project Scan 仍然工作
- Git Analysis 仍然工作
- Health Check 仍然工作
- Build/Test 仍然工作
- Environment Check 仍然工作
- Report 仍然工作

AI 必须是：

> **增强层，而不是地基。**

---

## 4.3 面向 AI 的输出必须标准化

所有重要分析能力都应能够输出：

```text
JSON
Markdown
CLI Output
Machine-readable Report
Context Pack
```

后期再支持：

- MCP
- 外部 AI Agent
- IDE Agent
- Claude Code
- Copilot
- 其他 Agent

Prism 本身不绑定任何单一 AI 平台。

---

# 5. 技术总架构

## 5.1 总体原则

采用：

```text
Tauri 2
   │
   ├── Frontend
   │     ├── TypeScript
   │     ├── React
   │     ├── Vite
   │     └── State Management
   │
   └── Rust
         ├── Prism Core
         ├── Project
         ├── Parser
         ├── Git
         ├── Analysis
         ├── Health
         ├── Verify
         ├── Environment
         ├── Context
         └── Report
```

目标：

> **GUI 与核心逻辑严格分离。**

---

# 6. Core Architecture

推荐 Rust 内部拆成明确模块：

```text
crates/
├── prism-core
├── prism-project
├── prism-parser
├── prism-git
├── prism-analysis
├── prism-health
├── prism-verify
├── prism-environment
├── prism-context
├── prism-report
└── prism-cli
```

Tauri 仅作为桌面壳和交互层。

---

# 7. 核心数据模型

Prism 的所有能力最终应该围绕统一数据模型运行。

核心对象：

```text
Project
ProjectSnapshot
File
Directory
Symbol
Module
Dependency
GitCommit
ChangeSet
Finding
CheckRun
EnvironmentSnapshot
ContextPack
Report
```

核心关系：

```text
Project
 ↓
Snapshot
 ├─ Files
 ├─ Symbols
 ├─ Modules
 ├─ Dependencies
 ├─ Findings
 └─ Environment

Git
 ↓
Commit
 ↓
ChangeSet
 ↓
Impact

Project + Change + Findings
 ↓
ContextPack
```

避免每个功能自己建立一套数据结构。

---

# 8. Parser / Language Strategy

Prism 必须设计语言无关的分析接口：

```text
Language Adapter
       ↓
Parse
       ↓
Normalized AST / Symbol Model
       ↓
Generic Analysis
```

第一阶段优先覆盖：

```text
TypeScript / JavaScript
Python
C#
```

并保留后续扩展：

```text
Rust
Java
C / C++
Go
```

原则：

> **Prism 的架构不允许把任何一种语言写死在 Core。**

---

# 9. Git Strategy

Git 是 Prism 的核心数据源之一。

至少支持：

```text
Repository Detection
Status
Branch
Commit
Commit Range
Diff
Rename
Delete
Add
Modify
History
```

然后进一步转换成 Prism 自己的：

```text
ChangeSet
```

所有上层 Change Analysis 均基于 `ChangeSet`，而不是直接依赖 Git 输出。

---

# 10. Findings 体系

所有风险、问题、异常统一进入：

```text
Finding
```

标准结构：

```text
Finding
├─ id
├─ category
├─ severity
├─ confidence
├─ location
├─ evidence
├─ message
├─ explanation
├─ relatedFiles
├─ relatedSymbols
└─ suggestedAction
```

Category 示例：

```text
Architecture
Dependency
Security
Build
Test
Documentation
Environment
Maintenance
Complexity
Git
Context
```

---

# 11. Severity 原则

只允许：

```text
Info
Warning
Critical
```

内部实现可以有优先级：

```text
P0
P1
P2
P3
```

但 UI 不应该制造无意义的“项目评分”。

Prism 更关注：

> **问题是什么、证据是什么、影响在哪里。**

而不是：

> “你的项目只有 72 分。”

---

# 12. Privacy / Security

Prism 默认：

```text
Local-first
No Telemetry
No Automatic Upload
```

这是与 ifuyo 产品整体方向保持一致的原则；Aura 的既有基线同样强调本地优先、无遥测以及严格的软件许可边界。

涉及：

- Secrets
- API Keys
- Tokens
- Private Paths
- Credentials
- Environment Variables

必须：

```text
Detect
Redact
Warn
```

生成 Context 时默认脱敏。

任何向外部 AI 服务发送项目内容，都必须：

> **显式用户行为触发。**

---

# 13. UI 总体结构

建议主导航固定为：

```text
Overview
Understand
Changes
Health
Verify
Environment
Context
Reports
```

而不是根据功能不断增加几十个页面。

核心首页：

```text
Project
├─ Project Overview
├─ Current Changes
├─ Health Findings
├─ Verification Status
├─ Environment Status
└─ Context Health
```

---

# 14. CLI

CLI 与 GUI 使用同一个 Rust Core。

例如：

```text
prism scan
prism inspect
prism changes
prism health
prism verify
prism env
prism context
prism report
```

CLI 不应该重新实现业务逻辑。

```text
GUI ─┐
     ├── Prism Core
CLI ─┘
```

统一输出 JSON / Markdown。

不把一个额外的“总控 ify CLI”作为第一阶段基础设施；两个独立产品各自拥有清晰 CLI，未来再考虑统一入口。

---

# 15. 报告系统

所有重大操作都应该能够产生：

```text
Report
```

例如：

```text
Project Report
Health Report
Change Report
Verification Report
Environment Report
Context Report
```

支持：

```text
JSON
Markdown
HTML
```

报告应该可保存、比较和重新打开。

---

# 16. Snapshot Engine

Prism 的长期能力建立在 Snapshot 之上。

流程：

```text
Scan
 ↓
Normalize
 ↓
Snapshot
 ↓
Analyze
 ↓
Report
```

之后：

```text
Snapshot A
       ↓
Change
       ↓
Snapshot B
```

可以比较：

```text
Files
Symbols
Modules
Dependencies
Findings
Tests
Build
Environment
```

---

# 17. 开发阶段

## M0 — Foundation

目标：

> 建立真正稳定的工程地基。

完成：

- Tauri 2
- Rust Core
- React / TS
- Project Model
- Logging
- Configuration
- Error Model
- Event Model
- CLI
- JSON Serialization
- Test Infrastructure

---

## M1 — Understand

目标：

> **第一次真正“看懂项目”。**

完成：

- Project Discovery
- Directory Tree
- File Statistics
- Language Detection
- Entry Points
- Config Detection
- Dependency Detection
- Symbol Extraction
- Basic Architecture View
- Project Overview

---

## M2 — Changes

目标：

> **第一次真正“看懂变化”。**

完成：

- Git
- Status
- Commit
- Diff
- Changed Symbols
- ChangeSet
- Dependency Impact
- Basic Change Impact

---

## M3 — Health

目标：

> **第一次真正“体检项目”。**

完成：

- Findings
- Severity
- Architecture Checks
- Dependency Checks
- Secret Scan
- TODO / FIXME
- Documentation Drift
- Risk Detection
- Health Report

---

## M4 — Verify

目标：

> **第一次真正“验证项目”。**

完成：

- Build
- Test
- Lint
- Type Check
- Static Checks
- CI Discovery
- Release Checks
- Unified Results

---

## M5 — Environment

目标：

> **理解项目与开发环境之间的关系。**

完成：

- Runtime Detection
- SDK Detection
- Toolchain
- Environment Variables
- Port Usage
- Process
- External Dependencies
- Environment Findings

---

## M6 — Context

目标：

> **让 Prism 成为 AI 的可靠项目上下文入口。**

完成：

- Context Health
- Context Discovery
- Context Validation
- Change-aware Context
- Risk-weighted Context
- Context Pack
- Markdown Export
- JSON Export
- MCP-ready Interface

---

## M7 — Snapshot / Evolution

目标：

> **让 Prism 从一次性扫描工具变成长期项目观察工具。**

完成：

- Snapshots
- Snapshot Diff
- Health History
- Change History
- Risk Trend
- Project Evolution

---

## M8 — Release

目标：

> **形成真正可以长期使用的软件。**

完成：

- Windows
- macOS
- Linux
- Installer
- Auto Update
- CLI Distribution
- Documentation
- Fixture Projects
- Release Audit
- Reproducible Builds
- Privacy Audit
- License Audit

---

# 18. V1 的定义

Prism V1 不追求“什么都分析”。

V1 必须能够完整跑通：

```text
打开项目
 ↓
理解项目
 ↓
查看 Git 变化
 ↓
分析影响
 ↓
发现风险
 ↓
执行验证
 ↓
检查环境
 ↓
生成 Context Pack
 ↓
导出报告
```

也就是：

> **Understand → Change → Health → Verify → Context**

这是 Prism 的完整闭环。

---

# 19. 明确不做

Prism 不做：

- IDE
- Code Editor
- AI Coding Agent
- 自研 LLM
- ChatGPT Clone
- Issue Tracker
- Project Management
- Cloud CI Platform
- Cloud Code Review SaaS
- 自动提交代码
- 自动修改大量项目代码
- 自动上传项目
- 复杂 DevOps 平台
- 完整 Debugger
- 完整 Profiler
- 完整 Dependency Manager

这些能力即使有价值，也不属于 Prism 的核心身份。

---

# 20. AI Coding 开发规则

Prism 本身也是一个 AI 辅助开发项目，因此必须特别限制 AI 生成代码的方式。

## 20.1 先事实，后实现

任何 AI 改码前：

```text
Read
 ↓
Understand
 ↓
Inspect
 ↓
Plan
 ↓
Implement
 ↓
Test
```

禁止：

```text
猜架构
 ↓
直接重写
```

---

## 20.2 禁止无边界重构

每个 Phase：

```text
Baseline
 ↓
Audit
 ↓
Change
 ↓
Test
 ↓
Audit
 ↓
Commit
```

禁止 AI 因“顺手优化”而扩大修改范围。

---

## 20.3 每个模块必须可以独立理解

要求：

- 单向依赖
- 小型接口
- 明确的数据模型
- 独立测试
- Core 与 UI 分离
- Adapter 与业务逻辑分离

核心目标：

> **让 AI 每次只需要理解有限范围的代码。**

---

# 21. 测试策略

Prism 必须优先测试 Core，而不是优先测试 UI。

```text
Unit Tests
    ↓
Integration Tests
    ↓
Fixture Projects
    ↓
Headless Tests
    ↓
UI Tests
```

尤其建立：

```text
fixtures/
├── typescript-project
├── python-project
├── csharp-project
├── git-project
├── broken-project
├── dependency-project
├── secret-project
└── documentation-drift-project
```

这些 Fixture 是 Prism 最重要的长期测试资产之一。

---

# 22. “真项目验证”原则

不能只在自己构造的 Demo 项目里验证。

每个重要 Analyzer 必须经过：

```text
Synthetic Fixture
+
Real Open Source Project
+
Real Personal Project
```

验证。

最终关注：

> **False Positive / False Negative**

而不仅仅是：

> “程序没有崩溃。”

---

# 23. 性能原则

Prism 需要适合长期打开。

原则：

```text
Incremental Scan
Caching
Lazy Analysis
Background Execution
Cancellation
Progress
Snapshot Reuse
```

避免：

```text
每次打开项目
→ 全项目重新解析
→ 全部重新计算
```

---

# 24. 数据与缓存原则

默认：

```text
Project-local or User-local Cache
```

缓存应当：

- 可失效
- 可重建
- 可删除
- 不影响项目正确性

任何缓存都不能成为：

> **项目运行的必要条件。**

---

# 25. 产品哲学

Prism 不追求“信息越多越好”。

它追求：

> **正确的信息在正确的位置出现。**

界面应该始终帮助用户回答五个问题：

```text
它是什么？
发生了什么？
有什么影响？
哪里有风险？
现在能不能交付？
```

---

# 26. ifuyo 产品体系中的位置

```text
ifuyo Aura
    ↓
听音乐 / 发现音乐 / 看见音乐

ifuyo Prism
    ↓
理解项目 / 理解变化 / 找到风险 / 验证项目

ifuyo Weave
    ↓
处理文件 / 数据 / 文档 / 批量操作
```

三者分别处理：

```text
Aura  = Experience
Prism = Understanding
Weave = Operation
```

Prism 不应该侵入 Weave 的文件处理领域，也不应该侵入 Aura 的音乐体验领域。

---

# 27. 最终一句话

> **ifuyo Prism：看懂项目 → 看懂变化 → 找到风险 → 验证项目 → 给 AI 上下文。**

它不是“又一个开发工具集合”。

它真正要成为的是：

> **一个能够持续理解软件项目状态的本地工程知识层。**