# Prism M10 — Context Engine
## 从“理解项目”到“为人和 AI 生成可靠的项目上下文”

你现在进入 **Prism M10：Context Engine**。

M0–M9 已建立 Prism 的 deterministic engineering facts：

```text
M0  Foundation
M1  Project Discovery
M2  Code Understanding
M3  Dependency & Architecture
M4  Git Basics
M5  ChangeSet
M6  Change Impact
M7  Health Engine
M8  Verify Engine
M9  Environment Engine
```

M10 的核心任务是：

> **把这些已经被 Prism 理解、验证、关联的事实，按照任务、变化、影响、风险和预算，组织成最有价值的 Context Pack。**

核心链：

```text
Project Facts
+
Code Facts
+
Architecture Facts
+
Git Facts
+
ChangeSet
+
ImpactAnalysis
+
Health Findings
+
Verification Results
+
Environment Facts
+
Project Documentation
+
User Task
        ↓
Context Engine
        ↓
Context Health
        ↓
Relevance / Risk-aware Selection
        ↓
Context Pack
        ↓
JSON / Markdown / Plain Text
        ↓
Human / AI / Agent Consumption
```

---

# 0. M10 的核心定位

M10 回答：

```text
“对于现在这个任务，
AI / 工程师真正需要知道什么？”
```

而不是：

```text
“把整个项目塞进去。”
```

Prism 的核心差异化从这里开始形成：

```text
普通工具：
Repo
  ↓
Dump files

Prism：
Repo
  ↓
Understand
  ↓
Change
  ↓
Impact
  ↓
Health
  ↓
Verify
  ↓
Environment
  ↓
Context Engine
  ↓
Relevant Context
```

---

# 1. M10 的核心原则

必须严格遵守：

```text
Deterministic First
Evidence First
Relevant Before Complete
Risk-aware
Change-aware
Task-aware
Budget-aware
Privacy-safe
Traceable Selection
No Hallucinated Context
No Full-repo Dump by Default
```

---

# 2. 最重要的一句话

M10 不是：

> “AI 自动总结整个项目。”

而是：

> **“Prism 根据已有事实，构建一个可解释、可追溯、受预算约束的项目上下文视图。”**

---

# 3. M10 不是什么

M10 不是：

```text
ChatGPT Clone
AI Coding Agent
LLM Runtime
Prompt Marketplace
RAG Platform
Vector Database
Embedding Service
MCP Server
IDE
Code Editor
Autonomous Developer
```

尤其不要第一版就加入：

```text
OpenAI SDK
Anthropic SDK
Ollama SDK
Embedding API
Vector DB
Cloud AI
Remote Retrieval
Agent Loop
Auto Tool Calling
```

---

# 4. M10 与 AI 的关系

M10 的输出：

> **为 AI 做准备。**

但：

> M10 本身第一版不需要 AI 才能工作。

例如：

```text
prism context . --task "修改 GitHub Actions 构建流程"
```

即使没有任何 LLM：

> 也应该生成高质量 Context Pack。

---

# 5. AI Principle

必须：

```text
Deterministic Context Engine
        >
Optional AI Enhancement
```

未来 AI 可以：

```text
用户任务
↓
AI 语义理解
↓
更好的 task decomposition
```

但第一版：

```text
User Task
↓
Deterministic task classification
↓
Context selection
```

---

# 6. Context Engine 的真正价值

M10 应解决：

```text
“我让 AI 修改项目，
到底应该给 AI 哪些文件？”
```

例如用户修改：

```text
BattleManager
```

不要给：

```text
整个 70k 行 Scripts/
```

应该更可能给：

```text
BattleManager.cs
相关 interface
直接依赖
关键调用者
相关 config
相关 tests
当前 diff
M6 impact
M7 findings
M8 verification
必要 Environment facts
项目约定文档
```

---

# 7. Context Pack

核心模型：

```rust
ContextPack
```

建议包含：

```text
id
project_id
request
generated_at
scope
objective
items
groups
selection
budget
health
evidence
diagnostics
redactions
coverage
limitations
```

具体根据现有 architecture 调整。

---

# 8. ContextPack 是什么

ContextPack：

> 一次针对某个任务生成的“项目上下文集合”。

它不是：

```text
ProjectSnapshot
```

也不是：

```text
EnvironmentSnapshot
```

也不是：

```text
GitDiff
```

而是：

```text
Context View
```

---

# 9. ContextPack 不应该持久化

M10 默认：

```text
in-memory
```

用户显式：

```text
Export Context Pack
```

才生成：

```text
JSON
Markdown
TXT
```

M11 才负责历史与持久化。

---

# 10. ContextRequest

建立：

```rust
ContextRequest
```

建议：

```text
project_id
task
scope
focus
budget
include
exclude
options
```

---

# 11. Task

任务可以是：

```text
“修复登录接口 500 错误”
“修改 GitHub Actions”
“重构 BattleManager”
“新增一个 card effect”
“调查为什么测试失败”
“审查最近一次提交”
```

M10 不需要：

> 先调用 LLM 理解任务。

---

# 12. Task classification

第一版可以 deterministic 分类：

```text
Change
BugInvestigation
Refactor
Feature
Build
Test
CI
Architecture
Security
Documentation
Release
Environment
Review
Unknown
```

---

# 13. Task classification

例如：

```text
“为什么 CI 在 Windows 上失败？”
```

可能得到：

```text
TaskKind:
CI
BugInvestigation
Environment
```

允许：

> 一个任务有多个 focus。

---

# 14. Task focus

建立：

```rust
ContextFocus
```

例如：

```text
Code
Architecture
Changes
Impact
Health
Verification
Environment
Documentation
Git
Configuration
Tests
CI
```

---

# 15. Task target

任务可能包含：

```text
file path
symbol name
module name
commit
branch
test name
error message
```

需要建立：

```rust
ContextTarget
```

---

# 16. Target resolution

例如：

```text
Task:
“修改 BattleManager”
```

如果有：

```text
BattleManager.cs
BattleManager::resolve()
```

则可：

```text
Resolved
```

如果存在：

```text
BattleManager.cs
LegacyBattleManager.cs
```

则：

```text
Ambiguous
```

不要猜。

---

# 17. Task parsing boundary

M10 可以解析：

```text
explicit path
explicit symbol
explicit module
known command
error text
Git reference
```

但：

> 不要假装理解所有自然语言。

---

# 18. Unknown task

如果：

```text
task = "帮我把这个做得更好"
```

则：

```text
TaskKind:
Unknown

Context:
Generic project overview
recent changes
architecture
health
verification
```

并明确：

> task specificity limited。

---

# 19. ContextSource

建立：

```rust
ContextSource
```

统一描述：

```text
source_type
source_id
path
location
kind
provenance
```

---

# 20. Source types

至少：

```text
File
Symbol
Module
Dependency
ArchitectureNode
GitCommit
GitDiff
ChangeSet
ImpactAnalysis
HealthFinding
CheckRun
EnvironmentFact
Documentation
Configuration
ProjectMetadata
```

---

# 21. ContextItem

建立：

```rust
ContextItem
```

至少：

```text
id
source
kind
title
content
location
priority
reason
evidence
truncated
redacted
```

---

# 22. ContextItem 必须能回答

每一个被选中的 item：

> “为什么我在这里？”

例如：

```text
Included because:
- directly changed
- dependency of target symbol
- impacted by current change
- related failing test
```

---

# 23. SelectionTrace

这是 M10 的关键模型。

建立：

```rust
ContextSelection
```

或：

```rust
ContextSelectionTrace
```

记录：

```text
candidate
selected
priority
reason
evidence
excluded_reason
```

这样用户可以知道：

> Prism 为什么给 AI 这些内容。

---

# 24. 不允许黑箱排序

不要：

```text
AI relevance = 0.927
```

然后：

> 谁也不知道为什么。

优先使用：

```text
Critical
High
Normal
Supporting
Excluded
```

或同等明确的 ordinal priority。

---

# 25. 可解释优先级

例如：

```text
Critical
= directly requested target

High
= directly changed / directly impacted

Normal
= strong dependency / related test

Supporting
= project convention / documentation
```

---

# 26. Risk-aware Context

M10 必须使用 M6/M7：

```text
Impact
Health Findings
```

影响 context selection。

例如：

```text
Changed symbol
+
high fan-out
+
public API
+
existing unresolved dependency
```

则相关 context：

> 优先级提高。

---

# 27. Risk is not a score

仍然不要：

```text
Risk = 82
```

使用：

```text
HighImpact
PublicApiChanged
HighFanOut
UnresolvedDependency
HealthFinding
FailedVerification
```

这种明确 signals。

---

# 28. Change-aware Context

如果当前项目有：

```text
ChangeSet
```

Context Engine 应优先理解：

```text
What changed?
```

而不是：

> 给 AI 当前项目的静态全集。

---

# 29. Default context scope

默认优先：

```text
Current Changes
+
Relevant Targets
+
Direct Dependencies
+
Direct Dependents
+
Relevant Tests
+
Relevant Configuration
+
Relevant Documentation
+
Relevant Findings
+
Relevant Verification
```

---

# 30. Full project context

允许：

```bash
prism context . --scope full
```

但：

> 仍然受 budget 限制。

不能因为用户选择 full：

> 就无限输出所有源代码。

---

# 31. Context budget

建立：

```rust
ContextBudget
```

至少：

```text
max_items
max_bytes
max_tokens_estimate
max_source_files
max_symbols
max_documentation
max_output_bytes
```

---

# 32. Token estimate

M10 不需要 tokenizer。

可以使用：

> deterministic approximate token estimation。

例如：

```text
UTF-8 bytes / heuristic factor
```

但是：

> 明确这是 estimate。

不要称为：

```text
exact token count
```

---

# 33. Token budget semantics

例如：

```text
budget = 12000 estimated tokens
```

Selection：

```text
Critical first
High next
Normal next
Supporting last
```

预算耗尽：

> stop selection。

---

# 34. Budget report

ContextPack：

```text
Budget:
estimated = 11,340
limit = 12,000

Items:
28 selected
17 excluded
```

---

# 35. No hidden truncation

如果：

```text
file content > remaining budget
```

必须：

```text
truncated = true
```

并记录：

```text
original_size
included_size
truncation_reason
```

---

# 36. Truncation strategy

不要从中间随机截断。

优先：

```text
symbol boundaries
function boundaries
class boundaries
paragraph boundaries
line boundaries
```

---

# 37. Symbol-aware truncation

如果：

```text
BattleManager.cs
```

只有：

```text
resolve()
```

相关：

> 优先截取完整 `resolve()`。

而不是：

> 从文件第 1 行切到固定字符数。

---

# 38. Structural summary

对于超预算文件：

不要完全丢掉。

可以保留：

```text
file path
language
symbols
relevant symbol signatures
relationships
```

而省略：

```text
full body
```

---

# 39. Skeleton context

建立概念：

```text
SymbolSkeleton
```

例如：

```text
class BattleManager
  - resolve(...)
  - executeCard(...)
  - applyDamage(...)
```

这是高价值、低 token 的 context。

---

# 40. File content levels

支持：

```text
Full
RelevantSymbols
Skeleton
MetadataOnly
```

---

# 41. Context detail level

可以：

```rust
ContextDetailLevel
```

例如：

```text
Full
Focused
Structural
Metadata
```

---

# 42. Default detail

对于代码：

> 默认 `Focused`。

对于：

> 直接目标文件：

可以 `Full`。

对于：

> distant dependencies：

`Skeleton` / `Metadata`。

---

# 43. Source diversity

不要 Context 全部由源码组成。

合理 Context Pack 应可能包含：

```text
Code
Diff
Impact
Tests
Docs
Config
Health
Verify
Environment
Git
```

---

# 44. Current diff priority

如果用户任务与当前变更相关：

```text
GitDiff
ChangeSet
ChangedSymbol
```

通常：

> Critical / High。

---

# 45. Impact priority

当前变更影响的：

```text
callers
referencers
dependents
tests
```

根据距离和 evidence：

```text
Direct
High
Normal
Supporting
```

---

# 46. Reverse dependency context

例如：

```text
Function A
→ Function B
→ Function C
```

用户修改 B：

Context 可以包含：

```text
B
A
C
```

但必须标注：

```text
why included
caller / dependency
```

---

# 47. Do not over-expand graph

不能：

```text
B
→ all graph
→ 30,000 nodes
```

默认：

> bounded traversal。

复用 M6 的：

```text
max depth
max nodes
```

---

# 48. Impact graph reuse

M10：

> 消费 M6。

不要：

```text
Context Engine
→ recreate reverse dependency engine
```

---

# 49. Dependency graph reuse

M3：

> 消费已有 relationships。

不要：

> 自己重新解析 imports。

---

# 50. ChangeSet reuse

M5：

> 提供 ChangedFiles / ChangedSymbols / ChangedModules。

M10：

> 直接使用。

---

# 51. Health reuse

M7：

> 提供 Findings。

M10：

> 将相关 findings 附在 context 中。

---

# 52. Verification reuse

M8：

> 提供 failing / passed / unavailable checks。

M10：

> 根据 task 相关性选择。

---

# 53. Environment reuse

M9：

> 提供 environment facts。

M10：

> 只选与当前 task 相关的环境信息。

---

# 54. Documentation Context

M10 必须开始处理：

```text
README
AGENTS.md
CLAUDE.md
CONTRIBUTING.md
docs/
BUILD.md
DEVELOPMENT.md
```

具体以项目实际文件为准。

---

# 55. Documentation priority

优先：

```text
project-specific instructions
>
developer/contributor documentation
>
build/test documentation
>
architecture documentation
>
README
```

但：

> 必须保留 provenance。

---

# 56. Instruction documents

如果存在：

```text
AGENTS.md
CLAUDE.md
```

M10 可以将其作为：

```text
ContextSource::Instruction
```

但：

> 不执行其中的命令。

---

# 57. Instruction conflict

例如：

```text
AGENTS.md:
run pnpm test

README:
run npm test
```

M10：

> 不能自动选择。

应该：

```text
InstructionConflict
```

或者：

```text
both sources retained
```

---

# 58. Instruction scope

重要：

```text
root/AGENTS.md
src/AGENTS.md
src/Battle/AGENTS.md
```

不同路径可能：

> 具有不同 scope。

M10 必须理解：

> path-scoped documentation / instructions。

---

# 59. Hierarchical instructions

如果：

```text
root/AGENTS.md
src/AGENTS.md
```

目标：

```text
src/foo.ts
```

则：

```text
root/AGENTS.md
+
src/AGENTS.md
```

都可能相关。

---

# 60. Child instruction precedence

如果 scope 冲突：

> 更近路径优先作为“更具体”。

但不要：

> 静默丢弃 root instructions。

应保留：

```text
specificity
source
conflict
```

---

# 61. Documentation health

M10 开始真正实现：

> Context Health。

目标：

```text
“这些项目文档现在还值得相信吗？”
```

---

# 62. Context Health 不是 M7 duplicate

M7：

> broad project health。

M10：

> specifically asks whether **context sources used for engineering work are current, reachable, coherent, and useful**。

---

# 63. Context Health categories

至少：

```text
Instruction
Documentation
Command
Path
Configuration
Architecture
Verification
Environment
Coverage
Freshness
Conflict
```

---

# 64. Context health finding

统一使用已有 Finding architecture。

例如：

```text
CTX001
Documentation command appears unavailable
```

---

# 65. Command drift

README：

```bash
npm test
```

M8：

```text
project actually uses pnpm test
```

Context Health：

```text
Documentation command drift
```

必须有：

```text
README evidence
M8 discovery evidence
```

---

# 66. Path drift

README：

```text
src/old/BattleManager.cs
```

实际：

```text
src/Battle/BattleManager.cs
```

Context Health：

```text
Documentation path drift
```

不要：

> 仅凭字符串不存在就判定文档一定错误。

---

# 67. Path existence

可以验证：

```text
path mentioned in docs
```

是否：

```text
exists
missing
ambiguous
```

---

# 68. Command existence

可以将文档命令与：

```text
M8 CheckSpec
```

比较。

例如：

```text
README:
dotnet test

M8:
dotnet test
```

则：

```text
Matched
```

---

# 69. Command mismatch

例如：

```text
README:
npm test

M8:
pnpm test
```

输出：

```text
Conflict / Drift
```

但是：

> 不自动证明 README 错了。

---

# 70. Tool availability

README：

```text
requires pnpm
```

M9：

```text
pnpm missing
```

Context Health 可以：

```text
Instruction/environment mismatch
```

但仍然：

> 保持事实和解释的边界。

---

# 71. Version drift

README：

```text
Node 18
```

package.json：

```text
Node >=22
```

M10：

```text
Documentation / Requirement conflict
```

复用 M9 evidence。

---

# 72. Architecture doc drift

如果文档明确：

```text
src/A depends on src/B
```

而 M3：

```text
relationship not observed
```

不要直接：

> “Architecture documentation is wrong.”

应该：

```text
Observed relationship differs from documented relationship.
```

---

# 73. Avoid semantic guessing

如果文档写：

```text
“BattleManager handles battles”
```

不能拿代码：

> 自动判断语义是否一致。

M10 只比较：

> 可验证的 structural facts。

---

# 74. Freshness

可以记录：

```text
last_modified
git commit relation
```

但：

> 文件比较新 ≠ 内容正确。

所以 freshness 只是：

> signal。

---

# 75. Documentation relevance

不是所有 docs 都应该进入 context。

选择：

```text
directly related docs
path-scoped docs
build/test docs
architecture docs
instruction docs
```

---

# 76. Documentation exclusion

例如：

```text
historical changelog
old release notes
unrelated design drafts
```

默认：

> Supporting / Excluded。

除非 task 明确要求。

---

# 77. Context Policy

建立：

```rust
ContextPolicy
```

控制：

```text
default_sources
max_depth
max_items
max_tokens
documentation_policy
include_environment
include_health
include_verification
include_git
include_changes
include_tests
```

---

# 78. No multiple configuration systems

沿用：

> M7/M8 已有统一配置体系。

不要新增：

```text
.prism-context.yml
.prism-ai.yml
.prism-rag.yml
```

如果已有：

```text
.prism.yml
```

则：

> 在同一配置体系扩展。

---

# 79. User policy

例如允许：

```yaml
context:
  max_tokens: 12000
  max_files: 40
  include_environment: true
  include_docs: true
```

具体 schema 根据已有 config system。

---

# 80. Policy precedence

建议：

```text
CLI explicit options
>
project config
>
default policy
```

---

# 81. Context Request examples

CLI：

```bash
prism context . --task "修改 GitHub Actions 构建流程"
```

或：

```bash
prism context . --task "调查最近一次提交为什么导致测试失败"
```

或：

```bash
prism context . --task "重构 BattleManager"
```

---

# 82. Context scope

至少：

```text
Auto
CurrentChange
Target
Full
```

---

# 83. Auto scope

默认：

> 根据 task + changes + project state 自动选择。

---

# 84. CurrentChange scope

只围绕：

```text
current ChangeSet
Impact
related health
related verify
```

---

# 85. Target scope

围绕：

```text
explicit file
symbol
module
```

---

# 86. Full scope

允许：

> 更广泛项目上下文。

但：

> budget 仍然生效。

---

# 87. Context candidate generation

第一阶段：

```text
CandidateCollector
```

从：

```text
M1-M9
+ docs
+ task
```

产生：

```text
ContextCandidate[]
```

---

# 88. Candidate sources

候选至少来自：

```text
TargetCollector
ChangedCollector
ImpactCollector
DependencyCollector
TestCollector
DocumentationCollector
HealthCollector
VerificationCollector
EnvironmentCollector
GitCollector
ArchitectureCollector
```

---

# 89. Candidate collector isolation

每个 collector：

> 只负责产生候选。

不要负责：

```text
final ranking
budget enforcement
rendering
```

---

# 90. Selection stage

统一：

```text
Candidate
    ↓
Normalize
    ↓
Deduplicate
    ↓
Prioritize
    ↓
Budget
    ↓
Select
```

---

# 91. Candidate normalization

同一个文件可能由：

```text
target
change
dependency
impact
test
```

同时发现。

必须：

> 合并为一个 ContextItem。

---

# 92. Multiple reasons

例如：

```text
BattleManager.cs
```

可能：

```text
Target
Changed
HighImpact
HealthRelated
```

必须：

> 保留多个 reasons。

---

# 93. Deduplication identity

优先：

```text
Project + relative path + symbol identity
```

不要：

> random IDs。

---

# 94. Symbol candidate identity

沿用 M2：

```text
SymbolId
```

---

# 95. Module candidate identity

沿用 M3：

```text
ModuleId
```

---

# 96. Finding candidate identity

沿用 M7 stable FindingId。

---

# 97. Check candidate identity

CheckRun：

> unique execution instance。

如果 context 引用 verification：

> 保留 check id + execution time + status。

---

# 98. Diff candidate

可以包含：

```text
GitDiff
ChangedFile
ChangedSymbol
```

---

# 99. Diff selection

如果 task：

```text
“review my current changes”
```

Diff：

> Critical。

如果 task：

```text
“写一个新功能”
```

当前 Diff：

> High / Supporting。

---

# 100. Impact selection

如果 task：

```text
“重构这个方法”
```

Impact：

> High。

---

# 101. Test selection

测试候选来自：

```text
M6 TestCandidate
```

以及：

```text
M8 relevant checks
```

---

# 102. Do not claim complete affected tests

例如 M6：

```text
2 candidate tests
```

M10：

```text
2 candidate tests
```

不能写：

> “These are all affected tests.”

---

# 103. Documentation candidate selection

优先：

```text
path-scoped instructions
build/test docs
architecture docs
task-related docs
README
```

---

# 104. Environment candidate selection

默认只选择：

```text
task-relevant runtime
required SDK
tool version
relevant environment requirement
relevant port/process issue
```

不要：

> 把所有环境变量都放进去。

---

# 105. Health candidate selection

只包括：

```text
findings related to selected targets
changed files
impacted modules
verification
environment
```

不要：

> dump all 100 findings。

---

# 106. Verification candidate selection

例如 task：

```text
“为什么测试失败”
```

优先：

```text
failed CheckRun
diagnostics
relevant output excerpts
changed files
impacted tests
environment
```

---

# 107. Verification output truncation

不要把：

```text
100MB stdout
```

全部塞进 Context。

只选：

```text
error region
diagnostic region
summary
relevant output
```

---

# 108. Output excerpt selection

M8 可以以后扩展：

```rust
DiagnosticRegion
```

M10 可以基于：

```text
stderr
error line
file
symbol
```

选择局部输出。

---

# 109. No false causality

例如：

```text
test failed
```

同时：

```text
recent diff
```

不能自动说：

> “recent diff caused test failure.”

应该：

```text
Current change and failing verification are both included.
Causal relationship not established.
```

---

# 110. Context provenance

所有 item：

必须知道：

```text
source module
source id
generated_at
reason
evidence
```

---

# 111. Context item content

一个 ContextItem：

```text
Header
Metadata
Content
Evidence
SelectionReason
```

例如：

```text
FILE: src/Battle/BattleManager.cs
Reason:
Target symbol + changed symbol + high fan-out

Content:
...
```

---

# 112. Context metadata

建议：

```text
language
file_kind
symbol_kind
line_range
module
change_state
impact_distance
health_status
verification_status
```

---

# 113. No giant metadata

不要在每个代码片段重复：

```text
整个项目 metadata
```

保持 compact。

---

# 114. Context Groups

建立：

```rust
ContextGroup
```

例如：

```text
Task
Project
Current Change
Target Code
Dependencies
Impact
Tests
Documentation
Health
Verification
Environment
```

---

# 115. Recommended order

默认：

```text
1. Task
2. Project Overview
3. Current Change
4. Target Code
5. Relevant Architecture
6. Dependencies / Impact
7. Tests
8. Health
9. Verification
10. Environment
11. Documentation
```

具体可按 task 动态调整。

---

# 116. Do not rigidly order all tasks

例如：

```text
CI failure task
```

应：

```text
Task
Failure
Verification
Environment
CI config
Relevant code
Changes
Docs
```

因此：

> group order 是 policy，不是硬编码固定顺序。

---

# 117. Task-adaptive context recipe

建立：

```rust
ContextRecipe
```

根据：

```text
TaskKind
```

调整 selection priority。

---

# 118. Bug Investigation recipe

优先：

```text
Failed verification
Error diagnostics
Target code
Recent changes
Impact
Relevant tests
Environment
Config
Docs
```

---

# 119. Refactor recipe

优先：

```text
Target code
Callers
Dependencies
Dependents
Tests
Architecture
Current changes
Health
```

---

# 120. Feature recipe

优先：

```text
Architecture
Target modules
Related patterns
Interfaces
Tests
Configuration
Docs
Current changes
```

---

# 121. CI recipe

优先：

```text
CI workflow
M8 local verification
Relevant environment
Build scripts
Changed files
Failure diagnostics
Dependency/config
Docs
```

---

# 122. Security recipe

优先：

```text
Relevant code
M7 security findings
Configuration
Dependency relationships
Verification
Environment metadata
Docs
```

不要：

> expose secrets。

---

# 123. Documentation task recipe

优先：

```text
target docs
referenced code
project structure
M7/M10 drift findings
verification commands
environment requirements
```

---

# 124. Review recipe

优先：

```text
ChangeSet
Impact
Health
Verification
Relevant architecture
Tests
```

---

# 125. Release recipe

优先：

```text
Git status
version metadata
M8 release verification
build/test results
CI inspection
environment
release docs
```

---

# 126. Context Quality

M10 应建立：

```rust
ContextHealth
```

但：

> 不做单一数字 score。

---

# 127. ContextHealth status

例如：

```text
Healthy
Attention
Incomplete
Conflict
Unavailable
```

---

# 128. Context Health signals

至少：

```text
MissingInstruction
BrokenDocumentationPath
CommandDrift
RequirementConflict
StaleProjectMetadata
AmbiguousTarget
InsufficientImpactData
UnverifiedEnvironment
MissingRelevantTest
TruncatedCriticalSource
SecretRedaction
SelectionIncomplete
```

---

# 129. Context completeness

例如：

```text
Target:
Resolved

Impact:
Complete

Verification:
Available

Environment:
Partial

Docs:
2 conflicts
```

整体：

```text
Context:
Incomplete
```

不是：

```text
Context Score = 84
```

---

# 130. Context limitations

ContextPack 必须明确：

```text
limitations[]
```

例如：

```text
Impact analysis truncated at depth 3.
Process inspection partially unavailable.
Target symbol ambiguous.
Verification result older than current working tree.
```

---

# 131. Staleness

Context 必须知道：

```text
current project state
```

例如：

```text
M8 CheckRun
captured at commit X
```

而项目现在：

```text
dirty workspace
```

则：

```text
verification may be stale
```

不要：

> 把旧的 pass 当成当前 pass。

---

# 132. Verification freshness

M10 应消费 M8 的 freshness information。

例如：

```text
CheckRun:
commit = abc123

Current:
working tree dirty

Status:
StaleForCurrentWorkspace
```

如果 M8 尚未实现该字段：

> 在 M10 使用最小 wrapper / comparison。

不要修改 M8 职责边界。

---

# 133. Health freshness

同理：

M7 finding：

> 基于旧 project state。

需要：

> 不把它无条件解释为当前状态。

---

# 134. Environment freshness

M9：

```text
captured_at
```

M10：

> 带 observation timestamp。

---

# 135. Git state freshness

M4：

```text
branch
status
HEAD
```

Context Pack：

> 应显示当前 scope。

---

# 136. Context snapshot identity

ContextPack 可以记录：

```text
project_state_reference
```

例如：

```text
HEAD
working_tree_dirty
```

但：

> 不要提前实现 M11 snapshot database。

---

# 137. Current-state descriptor

建立：

```rust
ContextStateReference
```

包括：

```text
head_commit
branch
dirty
generated_at
```

---

# 138. Context pack reproducibility

同样：

```text
same task
same project state
same policy
```

原则上：

> 应产生相同 candidate selection。

动态 environment/process data 除外。

---

# 139. Deterministic selection

排序必须：

```text
stable priority
stable source identity
stable path
stable location
```

不要：

> HashMap iteration order。

---

# 140. Selection tie-breaker

如果两个 candidates priority 相同：

```text
source kind
relative path
line
symbol
```

使用稳定排序。

---

# 141. Explain exclusions

对于重要 candidate 被排除：

必须记录：

```text
excluded_reason
```

例如：

```text
File X
Excluded:
budget exhausted
```

或：

```text
Excluded:
outside task scope
```

---

# 142. Don't report every exclusion

SelectionTrace 内部可以完整记录。

用户-facing：

> 默认只显示重要 exclusions。

避免：

> UI 被几百条 exclusion 淹没。

---

# 143. Context candidate scoring

内部可以有：

```text
priority dimensions
```

但不要依赖神秘浮点分数。

建议维度：

```text
DirectTarget
DirectChange
DirectImpact
VerificationFailure
HealthFinding
DependencyDistance
DocumentationScope
TaskMatch
ProjectConvention
```

---

# 144. Priority calculation

使用：

> deterministic rule-based priority bands。

例如：

```text
Critical:
DirectTarget + Critical verification / explicit task target

High:
Changed + DirectImpact

Normal:
Strong dependency + related test

Supporting:
Documentation / project overview

Low:
Distant dependency
```

---

# 145. No universal magic weights

禁止：

```text
target = 42
changed = 17
docs = 3
```

然后：

> 得出不可解释的排序。

如果需要内部 scoring：

> 必须封装、文档化、可测试，并且用户看到的是原因而不是神秘分数。

---

# 146. Context selection explainability

UI 应能显示：

```text
Why included?
```

例如：

```text
Changed
Direct dependency
Impacted caller
Related failing test
Project instruction
```

---

# 147. Context exclusion explainability

例如：

```text
README/old-architecture.md
Excluded:
unrelated to task
```

---

# 148. Path relevance

例如 task：

```text
src/Battle/BattleManager.cs
```

以下可能：

```text
src/Battle/*
src/Tests/Battle/*
docs/battle.md
```

提高 priority。

但：

> 不能只靠目录名推断架构。

---

# 149. Symbol relevance

M2：

```text
Symbol
```

提供最精细 context unit。

优先：

> symbol over whole file when practical。

---

# 150. Multiple symbols same file

可以输出：

```text
File: BattleManager.cs

Relevant symbols:
resolve()
executeCard()
applyDamage()
```

而不是：

> 反复复制整文件三次。

---

# 151. Shared file grouping

一个文件：

> 只出现一次。

内部：

```text
selected_ranges[]
```

---

# 152. Range merging

如果：

```text
resolve()
executeCard()
```

相邻：

> 合并 range。

不要输出：

> 大量重复 header。

---

# 153. Context range model

建立：

```rust
ContextRange
```

包含：

```text
start
end
reason
symbol_id
```

---

# 154. Code context headers

推荐：

```text
// Prism Context
// Source: src/Battle/BattleManager.cs
// Relevant symbols: resolve, executeCard
// Reasons: changed, impacted
```

但：

> 具体格式必须可配置。

---

# 155. Preserve original code

不要修改源代码内容。

Context rendering：

> 只包装，不改源码。

---

# 156. No auto-fix

M10：

> 不修改 README。

即使发现：

```text
command drift
```

也不能：

> 自动修改 docs。

---

# 157. No code modification

同理：

> context generation 永远 read-only。

---

# 158. No generated comments in source

Context header：

> 不能写回原始文件。

---

# 159. Context export

支持：

```text
JSON
Markdown
PlainText
```

---

# 160. JSON output

结构建议：

```json
{
  "schema_version": 1,
  "project": {...},
  "request": {...},
  "state": {...},
  "health": {...},
  "budget": {...},
  "groups": [...],
  "items": [...],
  "selection": {...},
  "limitations": [...]
}
```

---

# 161. Markdown output

目标：

> 人和 AI 都易读。

结构建议：

```markdown
# Prism Context Pack

## Task

...

## Project

...

## Current Changes

...

## Relevant Code

...

## Impact

...

## Tests

...

## Health

...

## Verification

...

## Environment

...

## Documentation

...

## Limitations

...
```

---

# 162. Plain text output

用于：

```text
copy/paste
terminal
chat
```

保持：

> 简洁 + source traceability。

---

# 163. Context Pack machine consumption

JSON 必须：

> 容易被 AI agent / future MCP / IDE integration 消费。

但：

> M10 不实现 MCP。

---

# 164. Context source locator

每个 item 应提供：

```text
path
line/range
symbol
```

或：

```text
finding id
check id
commit id
```

让下游：

> 可以回到原始事实。

---

# 165. No duplicated source-of-truth

ContextPack 不是：

> 第二份 ProjectModel。

它应该：

> 引用 + render M1-M9 facts。

---

# 166. Avoid giant object copying

不要：

```text
ContextPack
  contains full ProjectCodeModel
  contains full DependencyGraph
  contains full EnvironmentSnapshot
```

应：

> 只 materialize selected content。

---

# 167. Source materialization

Context Engine：

```text
candidate metadata
      ↓
selected
      ↓
read necessary source
      ↓
render excerpt
```

而不是：

> 先复制整个仓库到内存。

---

# 168. File read policy

只读取：

```text
selected files
selected ranges
required docs
```

---

# 169. Binary files

Context：

> 默认排除。

可保留：

```text
metadata
path
kind
```

---

# 170. Generated files

默认：

> low priority / excluded。

除非：

> task 明确涉及 generated artifact。

---

# 171. Vendor files

默认：

> excluded。

除非：

> target / dependency requires direct context。

---

# 172. Lockfiles

默认：

> metadata / relevant excerpt。

不要：

> 把整个 lockfile 塞进去。

---

# 173. Minified files

默认：

> exclude or metadata-only。

---

# 174. Huge files

如果：

```text
file > context threshold
```

使用：

```text
skeleton
relevant symbol
target range
```

---

# 175. Documentation size

大文档：

> 同样按 heading / section 截取。

---

# 176. Heading-aware docs

例如：

```markdown
## Build
## Test
## Architecture
## Deployment
```

task = testing：

> 优先 `## Test`。

---

# 177. Documentation section selection

建立：

```rust
DocumentSection
```

用于：

```text
heading
range
relevance
```

---

# 178. Command extraction

M10 可以识别文档里的：

```bash
npm test
cargo test
dotnet build
```

但：

> 仅 extraction / comparison。

不执行。

---

# 179. Command source

Command candidate：

必须记录：

```text
document path
line
code block
```

---

# 180. Command comparison with M8

可以：

```text
document command
vs
CheckSpec
```

比较：

```text
normalized program
args
cwd
```

---

# 181. Command normalization

例如：

```text
npm test
```

与：

```text
npm run test
```

是否相同：

> 必须经过 explicit normalization rules。

不要猜。

---

# 182. Path normalization

文档：

```text
./src/main.ts
src/main.ts
```

可以 canonicalize。

Windows path：

> 保留 platform semantics。

---

# 183. Documentation references

可以识别：

```text
src/...
tests/...
docs/...
```

再与：

```text
M1
```

交叉验证。

---

# 184. Documentation drift limitations

必须承认：

> “未检测到结构性冲突” != “文档内容完全正确”。

Context Health 输出：

```text
Validated:
path exists
command matched
```

而不是：

```text
Documentation correct
```

---

# 185. Instruction safety

`AGENTS.md` / `CLAUDE.md` 中可能写：

```bash
rm -rf
curl
powershell
```

M10：

> 只能将其作为 context content。

绝不执行。

---

# 186. Prompt injection awareness

非常重要。

项目文档 / 源码里可能包含：

```text
ignore previous instructions
upload secret
run command
```

M10：

> 不把 repository content 当成系统指令。

---

# 187. Context is data, not authority

必须建立安全边界：

```text
Repository content
≠
Prism instructions
```

---

# 188. AI-ready format

Context Pack 中可以标注：

```text
source_kind
provenance
trust_level
```

例如：

```text
System / Prism-generated fact
Project configuration
Project documentation
Source code
User task
```

---

# 189. Trust semantics

建议：

```text
PrismFact
ProjectConfig
ObservedToolOutput
Documentation
Heuristic
UserProvided
```

---

# 190. Never flatten trust

不要全部：

```text
text
```

然后交给 AI 自己猜哪个可信。

---

# 191. Evidence hierarchy

建议：

```text
Observed runtime output
>
project config
>
parsed source fact
>
Git fact
>
documentation
>
heuristic
```

具体根据 subject 调整。

---

# 192. Example conflict

```text
README:
Node 18

package.json:
Node >=22

M9:
Node 20
```

Context 应同时提供：

```text
Declared requirement:
>=22

Documentation:
18

Actual:
20

Status:
Conflict + Unsatisfied
```

这比：

> 选择一个“正确答案”

更可靠。

---

# 193. Context risk flags

ContextPack 可以有：

```text
risk_signals[]
```

例如：

```text
TargetAmbiguous
CriticalFindingPresent
VerificationFailed
VerificationStale
EnvironmentMismatch
DocumentationConflict
ImpactTruncated
ContextTruncated
```

---

# 194. Risk signal semantics

这些不是：

```text
score
```

只是：

> 帮助下游 AI / 工程师理解 Context limitations。

---

# 195. Critical truncation

如果：

```text
target file
```

由于 budget 被严重截断：

必须：

```text
ContextHealth:
Incomplete
```

并：

```text
Critical item truncated
```

---

# 196. No silent omission

尤其：

```text
failed test
critical health finding
target symbol
```

如果未能加入：

> 必须记录 omission reason。

---

# 197. Must-include items

建立：

```rust
ContextMustInclude
```

例如：

```text
explicit user target
current failing check
critical security finding
current diff
```

如果超预算：

> 不应该静默被低优先级内容挤掉。

---

# 198. Budget reservation

可以将 budget 分为：

```text
Critical reservation
General selection budget
Supporting budget
```

例如：

```text
Target + failure
```

拥有：

> 最小保留空间。

---

# 199. No pathological budget

如果用户：

```text
max_tokens = 10
```

Context Engine：

> 仍然不能崩溃。

输出：

```text
Minimal context
+ limitations
```

---

# 200. Zero budget

如果：

```text
max_tokens = 0
```

应：

```text
validation error
```

或者：

> 明确规定为 metadata-only。

不要随机行为。

---

# 201. Candidate budget estimation

估算：

```text
bytes
lines
symbols
token_estimate
```

不要：

> 读取所有内容后再发现超预算。

---

# 202. Lazy file loading

流程：

```text
candidate metadata
↓
estimate
↓
priority
↓
select
↓
materialize
```

---

# 203. Content extraction

支持：

```text
symbol range
line range
heading range
diagnostic excerpt
diff hunk
```

---

# 204. Related tests

M6：

```text
TestCandidate
```

M10：

> 根据 task/context priority 选择。

---

# 205. Test content selection

不要默认：

> 整个测试文件。

如果：

```text
testFoo()
```

明显相关：

> 优先 target test symbol。

---

# 206. Existing test failure

如果 M8：

```text
test_x FAILED
```

则 context：

```text
failure diagnostic
test location
related source
```

---

# 207. Verification output excerpt

保留：

```text
command
status
exit code
diagnostic
small relevant output
```

而不是：

> 整个 stdout。

---

# 208. Environment content

例如：

```text
Node 22.14.0
pnpm 10.17.1
Windows x64
```

这是有价值的。

但：

```text
API_KEY=...
```

永远不应该出现。

---

# 209. Environment redaction

ContextPack 在 materialization 时：

> 再经过 privacy/redaction layer。

不能认为：

> M9 已经安全，所以 M10 就直接复制所有内容。

---

# 210. Health finding content

不要：

> 把完整 source code embedded in finding 重复输出。

只带：

```text
title
severity
message
location
evidence
suggestion
```

相关 source：

> 单独作为 ContextItem。

---

# 211. Context item references

例如：

```text
Finding:
CODE001

Related source:
BattleManager.cs
resolve()
```

Context 可以：

> 将二者通过 relationship metadata 关联。

---

# 212. Verification references

例如：

```text
Check:
dotnet test
Failed

Related:
src/Battle/BattleManager.cs
tests/BattleManagerTests.cs
```

---

# 213. Context relationship graph

可建立轻量：

```rust
ContextRelation
```

例如：

```text
Item A
→ explains
Item B

Item A
→ affected-by
Item C

Item A
→ verifies
Item D
```

不需要重新实现 M3 graph。

---

# 214. Why relationships matter

AI 最容易缺：

> “为什么这些东西一起出现？”

ContextRelation 能表达：

```text
changed
depends_on
called_by
tested_by
documented_by
verified_by
flagged_by
required_by
```

---

# 215. Context graph size

同样：

> bounded。

不要：

> 把整个 M3 graph 再搬进 ContextPack。

---

# 216. Task-specific graph

只输出：

> selected items 之间的关系。

---

# 217. Context manifest

生成：

```rust
ContextManifest
```

包含：

```text
schema_version
project
task
state
item_count
estimated_tokens
selected_sources
excluded_count
limitations
```

---

# 218. ContextPack integrity

可以计算：

```text
content_hash
```

用于：

> 确认 context 内容变化。

但：

> 不把它当作 M11 persistence identity。

---

# 219. Secret-safe hashing

不要：

> 使用 secret raw values 构建 fingerprint。

对 redacted content：

> 可以 hash sanitized form。

---

# 220. ContextPack regeneration

相同：

```text
project state
task
policy
```

再次生成：

> 应尽量一致。

---

# 221. Current dirty state

如果 workspace：

```text
dirty
```

ContextPack 显示：

```text
Project state:
working tree dirty
```

这样 AI 不会误以为：

> 所有代码都来自 HEAD。

---

# 222. Staged vs unstaged

M4：

> 已有 distinction。

M10：

> 必须保留。

例如：

```text
staged diff
unstaged diff
```

如果 task：

```text
review staged changes
```

则：

> 只使用 staged scope。

---

# 223. Revision task

如果用户：

```text
“比较 HEAD~1 和 HEAD”
```

M10 应使用：

```text
M5 Revision ChangeSet
```

---

# 224. Commit review

如果：

```text
task = review commit abc123
```

优先：

```text
commit metadata
diff
changed symbols
impact
health
verification
```

---

# 225. Branch context

如果 task：

```text
“这个分支目前有什么风险？”
```

M10 可以：

```text
branch
ahead/behind if available
current changes
health
verification
```

但：

> 不做 Git remote network fetch。

---

# 226. No remote data

M10 默认：

```text
No fetch
No pull
No remote API
No GitHub API
```

---

# 227. CI context

M8：

> 可检查 GitHub Actions YAML。

M10：

> 可以选相关 CI workflow、job、run command。

不执行 remote CI。

---

# 228. CI task

例如：

```text
task:
“修复 CI 失败”
```

Context：

```text
workflow yaml
job step
local equivalent CheckSpec
environment
recent changes
failure diagnostics
```

---

# 229. CI/local mismatch

如果：

```text
CI:
pnpm test
```

M8：

```text
local configured:
npm test
```

ContextHealth：

> surface discrepancy。

不自动判定谁错。

---

# 230. Release context

如果 task：

```text
“准备 release”
```

优先：

```text
Git status
version metadata
M8 release result
build/test
CI
environment
release docs
```

---

# 231. Architecture task

如果：

```text
“评估这个模块怎么重构”
```

优先：

```text
M3 ArchitectureGraph
Module
Dependencies
Dependents
Cycles
Fan-in/fan-out
Related health findings
```

---

# 232. Security task

优先：

```text
M7 security findings
target code
config
Git change
environment metadata
verification
```

但：

> secret values remain redacted。

---

# 233. Documentation task

Context：

```text
target documentation
relevant source
project structure
commands
requirements
verification
Context Health
```

---

# 234. Generic task

如果：

```text
TaskKind = Unknown
```

默认输出一个小型：

```text
Project Overview
Current Changes
Architecture summary
Health summary
Verification summary
Environment summary
Relevant docs
```

而不是：

> 全项目代码。

---

# 235. Project Overview Context

统一：

```rust
ProjectOverviewContext
```

来自：

```text
M1
M2
M3
M4
```

简洁：

```text
project type
languages
main directories
entry points
build/test system
Git state
```

---

# 236. Overview should be factual

不要：

```text
“项目架构良好。”
```

而：

```text
“Project contains 6 modules with 2 dependency cycles.”
```

来自 M3/M7。

---

# 237. Context summaries

可以有：

```text
summary
```

但第一版：

> deterministic templates。

不要 LLM summary。

---

# 238. Summary generation

例如：

```text
Current changes:
3 files modified
2 symbols changed
1 module affected
2 candidate tests
```

这类 summary：

> 直接从 M5/M6 聚合。

---

# 239. No semantic hallucination

绝不能生成：

```text
“This refactor improves maintainability.”
```

除非：

> 明确来源于 documented external analysis。

第一版禁止这种 subjective claim。

---

# 240. Context language

CLI 默认：

> 当前 Prism language convention。

如果现有产品支持中英：

> 遵循 existing localization architecture。

不要为 Context 单独建立语言系统。

---

# 241. Localization

Context content：

> source code / project docs 原样保留。

Prism generated labels：

> 使用既有 translation system。

---

# 242. Markdown code fences

源代码：

> 必须正确 escape fenced blocks。

避免：

```text
```
/// source
```
```

破坏 Markdown。

---

# 243. HTML rendering

HTML report：

> 必须 escape source code。

防止：

```text
script injection
```

---

# 244. Plain text escaping

Terminal:

> 防止 ANSI/control character 注入。

---

# 245. JSON safety

所有：

```text
paths
commands
diagnostics
docs
source
```

确保：

> 正确 JSON escaping。

---

# 246. Context schema versioning

必须：

```text
schema_version
```

例如：

```json
{
  "schema_version": 1
}
```

---

# 247. Context schema stability

未来：

```text
M11 history
future MCP
IDE integration
AI adapters
```

都会依赖它。

因此：

> 不要把 UI-only 字段当核心 schema。

---

# 248. Internal vs external models

如果需要：

```text
ContextPackModel
ContextRenderModel
```

允许分离。

核心：

> domain model 不应依赖 React。

---

# 249. Architecture layering

建议：

```text
prism-core
    ↑
prism-project
    ↑
prism-context
    ↑
CLI / Tauri
```

Context 可消费：

```text
M1-M9 service interfaces
```

但：

> 不形成循环依赖。

---

# 250. Context Engine modules

建议：

```text
prism-context/
├── model
├── task
├── candidate
├── collectors
├── ranking
├── budget
├── materializer
├── documentation
├── health
├── recipes
├── renderer
├── privacy
├── integrity
└── tests
```

具体以实际项目规模调整。

---

# 251. Candidate collectors

推荐：

```text
TargetCollector
ChangeCollector
ImpactCollector
ArchitectureCollector
TestCollector
HealthCollector
VerificationCollector
EnvironmentCollector
DocumentationCollector
GitCollector
```

---

# 252. Ranking engine

建立：

```rust
ContextSelector
```

输入：

```text
candidates
task
policy
project_state
```

输出：

```text
selected
excluded
selection_trace
```

---

# 253. No LLM dependency

`ContextSelector`：

> 不应该 import AI SDK。

---

# 254. Materializer

建立：

```rust
ContextMaterializer
```

负责：

```text
selected candidate
↓
load content
↓
slice range
↓
redact
↓
truncate
↓
ContextItem
```

---

# 255. Renderer

建立：

```rust
ContextRenderer
```

至少：

```text
JsonRenderer
MarkdownRenderer
PlainTextRenderer
```

---

# 256. Privacy

复用 M7/M9：

```text
SecretRedactor
PathPrivacyPolicy
CommandLineRedactor
```

---

# 257. No second redaction engine

禁止：

```text
ContextSecretRedactor
```

与：

```text
EnvironmentRedactor
```

互相不兼容。

尽量：

> 共享统一 privacy layer。

---

# 258. Context diagnostics

统一：

```text
Diagnostic
```

例如：

```text
Target ambiguous.
Relevant test candidate unavailable.
Documentation section could not be parsed.
Context budget caused truncation.
```

---

# 259. Partial Context

ContextPack：

```text
status:
Complete
Partial
Incomplete
Failed
```

不要：

> 一个 source 读取失败就整个失败。

---

# 260. Candidate failure isolation

如果：

```text
DocumentationCollector failed
```

仍然可以：

```text
Code
Change
Impact
Verification
Environment
```

继续生成。

---

# 261. Context generation cancellation

复用 M0 Task：

```text
Context generation
→ cancel
```

必须：

> 干净退出。

---

# 262. Progress

GUI：

```text
Resolving task...
Collecting changes...
Collecting impact...
Collecting documentation...
Selecting context...
Materializing content...
Rendering...
```

不要：

> 假进度。

---

# 263. Context generation performance

必须避免：

```text
read entire repository
parse entire repository again
rebuild entire dependency graph
rerun all verification
rerun health
```

应该：

> 复用 M1-M9 已有 results。

---

# 264. Context invalidation

如果：

```text
user changes task
```

可以：

> 重新 selection。

不一定重新：

> scan entire project。

---

# 265. Context refresh levels

建议：

```text
Refresh Selection
Refresh Facts
```

但第一版 GUI：

> 一个 Refresh Context 就够。

---

# 266. Cache

可以 session cache：

```text
candidate metadata
document parse
token estimates
```

但：

> 不建立持久 database。

---

# 267. Source content cache

谨慎。

项目修改后：

> old content 不能继续被当作 current。

建议：

```text
content hash
mtime
Git state
```

用于 invalidation。

---

# 268. Context stale detection

如果：

```text
selected file hash != current hash
```

重新 materialize。

不要：

> 输出旧内容。

---

# 269. Current file identity

沿用 M1/M2：

```text
project-relative path
```

---

# 270. Symbol range validity

如果 source changed：

> 重新从 current ProjectCodeModel 获取 ranges。

不要：

> 永久使用旧 line number。

---

# 271. Historical context

如果 task 明确要求：

```text
compare commit A/B
```

可以使用：

```text
M5 historical ChangeSet
```

但：

> 不修改 worktree。

---

# 272. Current context vs historical context

必须显示：

```text
Context state:
Current workspace
```

或：

```text
Context state:
Historical revision range
```

---

# 273. Context source date

Docs / checks：

> 记录 observed / generated timestamp。

---

# 274. Context health vs project health

必须明确：

```text
Project Health
≠
Context Health
```

例如：

项目可能：

```text
健康
```

但：

```text
README command stale
```

导致：

```text
Context Health = Attention
```

---

# 275. Context health vs correctness

同理：

> Context Health 不代表代码正确。

它只代表：

> 当前生成给 AI/工程师的 context 是否完整、相关、可追溯、少冲突。

---

# 276. Context Quality signals

可以总结：

```text
Coverage
Freshness
Consistency
Relevance
Traceability
Privacy
Budget
```

但：

> 不生成一个总分。

---

# 277. Coverage

例如：

```text
Target:
covered

Impact:
complete

Verification:
available

Docs:
partial

Environment:
partial
```

---

# 278. Relevance

通过：

```text
selection reasons
```

而不是：

```text
Relevance 91%
```

---

# 279. Traceability

每个 context item：

> 能回到 source。

---

# 280. Privacy

ContextPack：

> 不得泄露 secrets。

---

# 281. Budget

ContextPack：

> 明确 estimated usage。

---

# 282. Context report

最终报告可：

```text
Task
Context status
Selected items
Selection reasons
Important findings
Important limitations
Budget
Sources
```

---

# 283. CLI commands

至少：

```bash
prism context <path>
prism context <path> --task "..."
prism context <path> --json
prism context <path> --markdown
prism context <path> --scope current-change
prism context <path> --scope target
prism context <path> --scope full
```

具体根据现有 CLI convention 精简。

---

# 284. Context health CLI

支持：

```bash
prism context health <path>
```

或者：

```bash
prism context <path> --check-health
```

优先选择：

> 与现有 CLI 风格一致的方案。

---

# 285. Context preview

非常有价值：

```bash
prism context . --dry-run
```

含义：

> 只展示将选择什么、为什么选择、预计预算，不完整 materialize 内容。

---

# 286. Dry-run example

```text
Target:
src/Battle/BattleManager.cs

Selected:
1. BattleManager.cs
   Reason: explicit target

2. BattleEngine.cs
   Reason: direct dependency

3. BattleManagerTests.cs
   Reason: impacted test candidate

4. AGENTS.md
   Reason: applicable project instruction

Excluded:
7 unrelated files
```

---

# 287. Explain mode

可以：

```bash
prism context . --explain
```

显示：

> selection trace。

---

# 288. Context item limit

CLI：

```bash
--max-items
--max-bytes
--max-tokens
```

根据现有 conventions。

---

# 289. Include / exclude

例如：

```bash
--include docs
--exclude environment
```

但：

> 显式 exclude 不能违反安全要求。

例如：

> 不允许通过 `--include-env-values` 绕过 secret redaction。

---

# 290. Context profiles

可以：

```bash
prism context . --profile review
```

未来支持：

```text
review
debug
refactor
feature
release
ci
```

如果第一版实现 profile：

> 使用 `ContextRecipe`。

---

# 291. Avoid profile explosion

不要：

```text
20 profiles
```

第一版：

```text
auto
review
debug
```

足够。

---

# 292. GUI Context page

新增真实：

```text
Context
```

---

# 293. Context page top

显示：

```text
Task input
Scope
Budget
Generate
Refresh
Export
```

---

# 294. Task input

例如：

```text
What are you working on?

> Investigate why the latest test run failed after changing BattleManager.
```

---

# 295. Generated context summary

显示：

```text
Context generated
28 items
11.3k estimated tokens
3 high-priority findings
2 limitations
```

---

# 296. Context groups UI

例如：

```text
Task
Changes
Code
Impact
Tests
Health
Verification
Environment
Documentation
```

---

# 297. Context item card

显示：

```text
BattleManager.cs
Changed + Target + High Impact

42 lines selected
Reason:
Direct task target
```

点击：

> 查看完整 excerpt。

---

# 298. Why included

必须：

> UI 可解释。

例如：

```text
Included because:
✓ Explicit task target
✓ Changed in working tree
✓ Directly referenced by 4 modules
```

---

# 299. Why excluded

可以点击：

```text
7 items excluded
```

查看：

```text
Outside task scope
Budget exhausted
Low relevance
Generated/vendor
Duplicate
```

---

# 300. Context Health UI

显示：

```text
Context Health

Complete
1 documentation conflict
1 verification result may be stale
```

---

# 301. Budget UI

例如：

```text
Context Budget

Estimated:
11.3k / 12k tokens

Files:
14 / 20
```

不要：

> “AI Context Score”。

---

# 302. Export

支持：

```text
Copy Markdown
Save JSON
Save Markdown
Save TXT
```

---

# 303. Copy behavior

用户点击：

```text
Copy Context
```

得到：

> 最终 Markdown/Text。

而不是内部 JSON。

---

# 304. AI adapter boundary

预留：

```rust
ContextConsumer
```

但不要实现：

```text
OpenAIConsumer
AnthropicConsumer
```

第一版。

---

# 305. Future AI integration

未来可以：

```text
ContextPack
→ OpenAI
→ Claude
→ local model
→ agent
```

因此：

> ContextPack 必须 provider-neutral。

---

# 306. Provider neutrality

ContextPack 不应出现：

```text
openai_prompt
claude_prompt
system_prompt
```

核心模型。

---

# 307. Prompt separation

未来 adapter：

```text
ContextPack
+
UserTask
→ PromptAdapter
```

M10：

> 不做 prompt engineering framework。

---

# 308. No automatic LLM summary

即使用户安装了 AI：

> M10 不应该自动向云端发送 repository context。

---

# 309. Explicit AI handoff

未来：

```text
Send to AI
```

也必须：

> 用户显式操作。

---

# 310. Privacy before handoff

AI handoff future path：

```text
ContextPack
↓
Privacy Filter
↓
User Preview
↓
Explicit Send
```

不是：

```text
ContextPack
↓
automatic upload
```

---

# 311. M10 current boundary

第一版：

```text
ContextPack
→ Copy / Export
```

不：

```text
ContextPack
→ Cloud AI
```

---

# 312. Prompt injection defense in export

导出的 Context：

可以保留：

```text
<project documentation>
```

但未来 AI adapter 必须：

> 将它与 instructions 分离。

M10 应在 schema 中保留：

```text
trust/provenance
```

帮助未来 adapter。

---

# 313. Repository instruction content

不要自动转换成：

```text
system message
```

---

# 314. User task priority

用户输入的 task：

> 是 context objective。

Repository documents：

> 是 data/context，不是 superior instruction。

---

# 315. Security model

核心：

```text
User Intent
>
Prism System Rules
>
Project Data
```

具体 AI adapter 后续再映射。

---

# 316. Context pack integrity

可提供：

```text
generated_at
project_state
content_hash
```

便于：

> 用户确认“AI 实际拿到的 context”。

---

# 317. Reproducibility report

可以：

```text
Context manifest
```

包括：

```text
task
policy
project state
source ids
selected ranges
```

以后可以：

> 复现 context selection。

---

# 318. Context Manifest

建议：

```rust
ContextManifest {
    schema_version,
    project_id,
    task,
    state,
    policy,
    selected_items,
    excluded_items_summary,
    generated_at,
}
```

---

# 319. No source duplication

Manifest：

> 记录 references，不复制完整 source。

---

# 320. Large pack handling

ContextPack 本身可能大。

GUI：

> 分页 / lazy rendering。

不要：

> 一次渲染全部 DOM。

---

# 321. Markdown generation

可以：

> 流式写入。

---

# 322. JSON generation

确保：

> 大 pack 不产生不必要多份内存 copy。

---

# 323. Source content memory

注意：

> source + rendered content + JSON string 不应同时保留太多大对象。

---

# 324. Cancellation during materialization

例如：

```text
read file
→ redact
→ render
```

任何阶段：

> 都应响应 cancel。

---

# 325. Error recovery

单文件：

```text
permission denied
```

不要：

> 整个 context failed。

生成：

```text
item unavailable
reason
```

---

# 326. Context item unavailable

例如：

```text
BattleManager.cs
```

突然删除。

输出：

```text
Context item unavailable
Current file no longer exists
```

---

# 327. Concurrent editing

用户在 context generation 时修改文件：

M10：

> 需要检测并避免输出明显混合版本。

至少：

```text
read timestamp/hash
```

必要时重新读取。

---

# 328. File consistency

可以：

```text
before_hash
after_hash
```

如果变化：

> retry once / mark unstable。

---

# 329. Unstable file

Context item：

```text
content_stability = unstable
```

并：

> 不默默声称这是稳定 snapshot。

---

# 330. Git state race

Context generation 同时 Git 改动：

> 必须尽可能在同一逻辑 scope 内读取。

如果无法保证：

> diagnostics。

---

# 331. Context consistency

目标：

> 一个 ContextPack 尽可能代表一个统一 project state。

不能：

```text
Diff from T1
Code from T2
Health from T0
```

还假装：

> 都是同一状态。

---

# 332. Fact timestamps

M5-M9 如果有 timestamps：

> 记录来源 freshness。

---

# 333. Staleness policy

建立：

```rust
ContextFreshness
```

例如：

```text
Current
PossiblyStale
Stale
Unknown
```

---

# 334. Freshness rules

例如：

```text
M8 check generated before current workspace change
→ Stale
```

```text
M9 environment 5 seconds old
→ Current enough
```

具体时间阈值：

> 应配置/文档化，不要拍脑袋。

---

# 335. Dynamic environment freshness

Processes/ports：

> freshness threshold 比 code facts 更短。

---

# 336. Static docs freshness

Git metadata：

> 只作为 weak signal。

---

# 337. No fake freshness

文件刚改过：

> 不能因此说“内容 fresh”。

---

# 338. Context Health rules

建议初版：

```text
CTX001 Applicable instruction missing
CTX002 Documentation path drift
CTX003 Documentation command drift
CTX004 Requirement conflict
CTX005 Verification result stale
CTX006 Environment observation incomplete
CTX007 Target ambiguous
CTX008 Context candidate unavailable
CTX009 Critical item truncated
CTX010 Selection incomplete
CTX011 Instruction scope conflict
CTX012 Documentation reference ambiguous
```

具体 rule ID 按现有 registry convention 调整。

---

# 339. Conservative rules

所有 CTX rules：

> evidence first。

---

# 340. No business semantics

不能判断：

```text
“这份架构文档写得不好。”
```

只判断：

```text
Observed architecture differs from documented relationship.
```

---

# 341. Context Health severity

复用 M7 severity。

不要另造：

```text
ContextSeverity
```

---

# 342. Context Health confidence

复用现有 confidence enum。

---

# 343. Suppression

沿用 M7 suppression system。

例如：

```text
CTX003 suppressed
```

不要：

> 另造 suppression format。

---

# 344. Change-aware Context Health

例如：

```text
README command
```

与：

```text
current M8 check
```

不一致。

若命令和文档都处于：

> Current state

可以生成：

```text
Context health finding
```

---

# 345. Context selection and findings

Finding 本身：

> 可以是 ContextItem。

但：

> 不要让 finding 自动包含大量 source content。

---

# 346. Context-aware Health

未来 M11 可以记录：

> context health history。

M10：

> 只返回 current/session data。

---

# 347. CLI result semantics

`prism context`：

默认：

> 输出 context artifact。

不要混入：

```text
debug logs
```

---

# 348. `--json`

stdout：

> machine-readable ContextPack JSON。

---

# 349. `--markdown`

stdout：

> markdown only。

---

# 350. `--dry-run`

stdout：

> selection plan，不 materialize full source。

---

# 351. Exit codes

建议：

```text
0
→ context generated successfully

non-zero
→ invalid request / failed critical generation
```

Context incomplete：

> 不一定 exit nonzero。

具体与 CLI conventions 对齐。

---

# 352. Context generation failure

只有：

> 无法生成最基本 pack

才：

```text
Failed
```

---

# 353. Partial generation

例如：

```text
Docs unavailable
```

但：

```text
Code
Changes
Impact
Health
Verify
Environment
```

都可用。

则：

```text
Partial
```

---

# 354. GUI error

不要：

> 弹 generic “Something went wrong”。

应该：

```text
Context generation partially completed.

Documentation:
Unavailable

Code:
Available

Verification:
Available
```

---

# 355. Context preview before generation

GUI 可以提供：

```text
Preview selection
```

对应：

```bash
prism context . --dry-run
```

---

# 356. Context item expansion

点击：

```text
BattleManager.cs
```

才加载 full excerpt。

---

# 357. Copy individual item

允许：

> Copy item。

---

# 358. Copy pack

允许：

> Copy whole context。

---

# 359. Search within context

支持：

```text
find
filter
group
```

而不是重新搜索整个 repo。

---

# 360. Context page state

维护：

```text
task
scope
policy
generated_pack
selected_group
```

不要：

> 将 ContextPack 存入 frontend state 后复制多个巨大版本。

---

# 361. IPC size

ContextPack 可能比较大。

不要：

> 一次 IPC 传递无限大小 giant JSON。

如果现有 Tauri architecture 允许：

> 分块或按需读取。

第一版至少：

> 有明确 max output size。

---

# 362. Output size safety

设置：

```text
max_output_bytes
```

防止：

> GUI 卡死。

---

# 363. Context pagination

CLI：

> 大结果可按 item。

GUI：

> lazy rendering。

---

# 364. No external embeddings

再次明确：

```text
No embeddings
No vector DB
No semantic search model
```

---

# 365. Search strategy

M10 选择候选时：

使用：

```text
M1 paths
M2 symbols
M3 dependencies
M5 changes
M6 impact
M7 findings
M8 checks
M9 environment
```

而不是：

> embedding similarity。

---

# 366. Semantic similarity future hook

可以预留：

```rust
ContextRelevanceProvider
```

未来：

```text
DeterministicProvider
AIProvider
```

但第一版：

> 只有 deterministic provider。

---

# 367. AI extension boundary

以后：

```rust
trait RelevanceProvider
```

可能扩展。

但：

> 核心 selector 不依赖 AI。

---

# 368. No AI fallback dependency

不能：

```text
AI unavailable
→ context unavailable
```

M10：

> AI 永远不是基础依赖。

---

# 369. Agent-ready output

ContextPack JSON 应包含：

```text
facts
sources
relationships
limitations
```

让未来 agent：

> 既能读内容，也能看到证据。

---

# 370. Agent safety

未来 agent：

> 不应该只看到“结论”。

它应该看到：

```text
what
source
why
confidence
freshness
```

---

# 371. Context source confidence

例如：

```text
Prism Fact:
High

Documentation:
Medium

Heuristic:
Low
```

具体 enum 与 core alignment。

---

# 372. Context reason

例如：

```text
reason:
ExplicitTaskTarget
DirectlyChanged
DirectlyImpacted
FailedVerification
RelevantDocumentation
RequiredEnvironment
HealthFinding
```

---

# 373. Context exclusion reason

例如：

```text
OutsideScope
Duplicate
LowPriority
Generated
Vendor
BudgetExceeded
Unavailable
PrivacyRestricted
```

---

# 374. Privacy restricted

如果一个 source：

> 因为敏感信息不能输出。

应：

```text
selected = false
reason = PrivacyRestricted
```

或：

> 输出 redacted metadata-only item。

---

# 375. Secret-bearing file

例如：

```text
.env
```

不要：

> 直接加入 full content。

可以：

```text
File exists
Contains sensitive candidates
Tracked status
Relevant to task
```

---

# 376. Credential config

例如：

```text
config/settings.json
```

如果可能含 secret：

> metadata + redacted excerpt。

---

# 377. Source code secret context

如果 source 内：

```text
API_KEY = "secret"
```

Context：

> 必须 redacted。

---

# 378. Redaction marker

统一：

```text
[REDACTED]
```

或已有 policy marker。

---

# 379. Preserve line structure after redaction

尽量：

> 不改变 line numbers。

例如：

```text
API_KEY = "[REDACTED]"
```

而不是：

> 删除整行导致位置映射变化。

---

# 380. Context integrity after redaction

如果 redaction：

> 改变了 content hash。

应该：

```text
hash = sanitized content
```

不要声称：

> equals original raw content hash。

---

# 381. Context health with redaction

若 critical code 过度 redacted：

> 标记：

```text
Context coverage limited by privacy filtering
```

---

# 382. No sensitive preview bypass

GUI：

> 不提供“显示原始 secret”。

---

# 383. User explicit request doesn't bypass

即使 task：

```text
“把 API KEY 给我”
```

M10：

> 仍然遵守 privacy/redaction policy。

---

# 384. Context pack max source count

例如：

```text
max_files
max_symbols
```

必须：

> deterministic。

---

# 385. Context file count vs item count

区分：

```text
files
items
ranges
symbols
```

因为：

> 一个文件可以包含多个 context items。

---

# 386. Summary counts

ContextPack summary：

```text
Files:
12

Symbols:
23

Docs:
4

Findings:
3

Checks:
2
```

---

# 387. Context budget estimate

```text
Estimated tokens:
11,340

Actual bytes:
45,211
```

---

# 388. No exact token claim

如果使用 heuristic：

必须写：

```text
estimated
```

---

# 389. Context pack version

例如：

```text
Prism Context Pack
schema_version = 1
generator_version = ...
```

---

# 390. Generator metadata

可以记录：

```text
prism_version
context_schema
policy_version
```

不要：

> 暴露系统内部路径。

---

# 391. Context manifest reproducibility

如果用户导出：

```text
context-manifest.json
```

可以知道：

```text
what was selected
why
from which state
under which policy
```

---

# 392. No raw internal diagnostics in default pack

内部 debug stack trace：

> 不加入默认 context。

---

# 393. Error context

如果 generation failed：

> 用户可看到 concise diagnostic。

不是：

> 10,000 字 Rust backtrace。

---

# 394. Debug log

仍通过：

> unified logging。

---

# 395. Test architecture

必须大量使用 fixtures。

---

# 396. Context fixture

建立：

```text
fixture/
  context/
    node/
    python/
    csharp/
    rust/
    docs/
    ci/
    conflicts/
    failures/
    secrets/
    large/
```

---

# 397. Golden Context tests

对于确定性 fixture：

输入：

```text
project
task
policy
```

期望：

```text
selected item ids
ordering
reasons
budget
limitations
```

必须稳定。

---

# 398. Snapshot test caution

可以：

> snapshot test rendered Markdown。

但：

> 动态 timestamp / path / PID 要 normalize。

---

# 399. Selection tests

至少覆盖：

```text
explicit target
changed file
direct dependency
direct dependent
transitive dependency
related test
health finding
failed verification
environment mismatch
documentation
```

---

# 400. Budget tests

测试：

```text
large budget
small budget
critical reservation
zero/invalid budget
single huge file
many tiny files
```

---

# 401. Dedup tests

一个文件同时：

```text
target
changed
impact
test relation
```

必须：

> 只输出一次 source content。

---

# 402. Truncation tests

测试：

```text
symbol boundary
function boundary
line boundary
documentation heading
diagnostic excerpt
```

---

# 403. Freshness tests

测试：

```text
current verification
stale verification
dirty workspace
changed file after candidate generation
```

---

# 404. Documentation health tests

至少：

```text
path exists
path missing
command matches
command drifts
requirement conflicts
instruction scope
```

---

# 405. Privacy tests

测试：

```text
.env
API_KEY
PASSWORD
TOKEN
PRIVATE_KEY
credential-like CLI args
secret in source
secret in docs
```

---

# 406. Injection tests

测试：

```text
README containing fake system instructions
source containing <script>
Markdown fence injection
ANSI escape
HTML injection
```

确认：

> Context Engine treats all project content as data.

---

# 407. Cross-platform tests

至少：

```text
Windows path
Linux path
macOS path where possible
CRLF
LF
Unicode
spaces
case-sensitive paths
```

---

# 408. Real project validation

至少：

```text
Prism
一个真实 TypeScript 项目
一个真实 Python 项目
一个真实 C# 项目
```

必要时：

```text
Rust
Docker
```

---

# 409. Self context generation

对 Prism 自己：

```bash
prism context . --task "review the latest changes"
```

必须得到：

```text
Current changes
Relevant code
Impact
Health
Verification
Environment
Docs
```

---

# 410. Self CI task

执行：

```bash
prism context . --task "inspect the CI build workflow"
```

检查：

> Context 是否包含 workflow + local check + relevant environment。

---

# 411. Self bug investigation

人为制造一个 fixture failure：

```text
test fails
```

然后：

```bash
prism context . --task "why does this test fail?"
```

应优先：

```text
failure
test
source
change
environment
```

---

# 412. Self refactor task

例如：

```text
prism context . --task "refactor the project service layer"
```

如果 target ambiguous：

> 必须显示 ambiguity。

不能：

> 随便选择一个 service layer。

---

# 413. Self documentation task

例如：

```text
prism context . --task "update the build instructions"
```

应：

```text
README
AGENTS
M8 checks
M9 requirements
relevant package files
```

---

# 414. Real AI handoff simulation

不调用 AI。

仅：

```text
generate markdown
copy
```

然后人工检查：

> 是否能够让另一个没有打开仓库的人理解任务所需上下文。

---

# 415. Context usability test

至少让：

> 一个独立 AI / reviewer

只看 Context Pack：

确认它能知道：

```text
task
project scope
changed area
relevant code
impact
known risks
verification status
environment constraints
documentation constraints
limitations
```

---

# 416. No full-repo leakage

对于一个大型项目：

> 默认 Context Pack 不应该接近整个 repository size。

需要记录：

```text
project files
context files
selection ratio
```

但：

> 不把 ratio 当 score。

---

# 417. Context compression

M10 的价值之一：

```text
Project:
1000 files

Relevant:
18 files
```

这不是：

> 单纯删文件。

还要：

```text
18 files
↓
selected symbols
↓
relevant ranges
↓
facts
↓
evidence
```

---

# 418. Context hierarchy

推荐：

```text
Project
  ↓
Module
  ↓
File
  ↓
Symbol
  ↓
Range
```

选择可以：

> 在任意层级停止。

---

# 419. Context escalation

如果：

```text
Symbol-only context
```

不足：

可以：

```text
expand to file
```

如果还不足：

```text
expand related files
```

必须：

> budget-aware。

---

# 420. Context expansion policy

建议：

```text
Explicit target
→ symbol

If dependency reason requires:
→ related symbol

If structural context missing:
→ file skeleton

If task requires:
→ full file
```

---

# 421. Avoid source flooding

例如：

```text
class
```

有 30 methods。

task 只涉及：

```text
resolve
```

不要：

> 默认全 class body。

---

# 422. Class skeleton

可以显示：

```text
class BattleManager {
    resolve(...)
    executeCard(...)
    applyDamage(...)
}
```

并：

> 只展开 resolve body。

---

# 423. Interface context

如果 target implements:

```text
IBattleResolver
```

接口通常：

> High relevance。

因为：

> understanding contract。

---

# 424. Public API context

如果 M6 signals：

```text
PublicApiChanged
```

则：

```text
interface
export
public callers
```

优先。

---

# 425. High fan-out

如果 M6：

```text
HighFanOutDependencyChanged
```

context：

> include representative dependents / callers。

注意：

> 不一定全部塞进去。

---

# 426. Representative dependents

如果 100 callers：

> 可以先提供结构 summary：

```text
100 dependents
top direct callers:
A
B
C
```

而不是：

> 复制 100 files。

---

# 427. Impact summary

Context 可以：

```text
Direct dependents: 8
Transitive dependents: 42
Candidate tests: 5
```

这些是：

> M6 facts。

---

# 428. Large impact context

如果 blast radius 很大：

> Context Health 可以提醒：

```text
Impact is large; context is intentionally summarized.
```

---

# 429. Impact truncation

如果 M6 自己是：

```text
Partial / Truncated
```

Context：

> 必须保留这个 limitation。

---

# 430. Health finding grouping

例如：

```text
3 findings
2 related to target
1 unrelated
```

只选择：

> 相关 finding。

---

# 431. Verification grouping

例如：

```text
8 checks
1 failed
2 relevant
5 unrelated
```

Context：

> 优先 relevant + failed。

---

# 432. Environment grouping

例如：

```text
20 installed tools
3 required
```

Context：

> 只带 required 3。

---

# 433. Documentation grouping

例如：

```text
30 markdown files
```

Context：

> 只带 applicable。

---

# 434. Git grouping

例如：

```text
500 commits
```

Context：

> 默认不加载历史。

只有 task：

```text
review history
```

才扩展。

---

# 435. Commit history context

如果 task：

```text
“为什么这段代码这样设计？”
```

可以从：

```text
git blame/history
```

未来扩展。

但第一版：

> 只在用户明确要求 historical context 时使用 M4 data。

---

# 436. Blame

不作为 default context source。

原因：

> 噪声大。

---

# 437. Commit history budget

如果需要：

> 只提供最近相关 commits。

---

# 438. Git message content

作为：

```text
Supporting context
```

不能：

> 把 commit message 当 source-of-truth。

---

# 439. Architecture overview context

M3：

```text
graph metrics
cycles
modules
dependencies
```

M10 可以输出：

```text
Relevant architecture
```

---

# 440. Architecture summary

例如：

```text
BattleManager
→ BattleEngine
→ EffectResolver

Dependents:
CardSystem
CombatUI
ReplaySystem
```

必须：

> based on M3 facts。

---

# 441. No architecture recommendation

M10 不应说：

```text
“你应该把 BattleManager 拆成 3 个类。”
```

除非用户 task explicitly asks for design and future AI does interpretation.

M10：

> 只提供 factual architecture context。

---

# 442. Context recommendation boundary

Context Engine：

> 选择资料。

它不是：

> architecture consultant。

---

# 443. Future AI design

未来 AI：

```text
ContextPack
→ reasoning
→ recommendation
```

这是：

> AI layer 的工作。

---

# 444. Context Pack quality criteria

必须达到：

```text
Relevant
Traceable
Bounded
Fresh
Safe
Deterministic
Explainable
Composable
```

---

# 445. Final architecture diagram

完成 M10 后：

```text
                    ┌───────────────┐
                    │    Project    │
                    └───────┬───────┘
                            ↓
                    ┌───────────────┐
                    │   M1–M4 Facts │
                    └───────┬───────┘
                            ↓
                    ┌───────────────┐
                    │ M5–M6 Changes │
                    └───────┬───────┘
                            ↓
               ┌────────────────────────┐
               │      M7–M9 Facts       │
               │ Health / Verify / Env  │
               └────────────┬───────────┘
                            ↓
                     ┌──────────────┐
                     │ M10 Context  │
                     └──────┬───────┘
                            ↓
                ┌──────────────────────┐
                │    Context Pack      │
                └───────┬──────────────┘
                        ↓
              ┌───────────────────────┐
              │ Human / AI / Agent    │
              └───────────────────────┘
```

---

# 446. M0–M10 完整语义链

完成 M10 后：

```text
M0  Prism 怎么稳定运行？
M1  项目里有什么？
M2  代码是什么？
M3  它们怎么连接？
M4  Git 记录了什么？
M5  到底改了什么？
M6  可能影响什么？
M7  哪些地方值得注意？
M8  实际验证结果是什么？
M9  当前工程环境是什么？
M10 对于当前任务，真正需要知道什么？
```

---

# 447. Prism 最核心的数据流

```text
Repository
    ↓
Facts
    ↓
Relationships
    ↓
Changes
    ↓
Impact
    ↓
Health
    ↓
Verification
    ↓
Environment
    ↓
Context Engine
    ↓
Context Pack
```

---

# 448. 这一步之后 Prism 的产品定位变化

M0–M9：

> Prism 能“看懂项目”。

M10：

> Prism 开始能够“把自己看懂的东西组织给别人”。

这个“别人”可以是：

```text
人
AI
Agent
IDE
CLI
未来 MCP
```

---

# 449. Context Engine 的真正产品价值

不是：

```text
AI summary
```

而是：

```text
Project
↓
Understanding
↓
Selection
↓
Compression
↓
Traceability
```

---

# 450. Context Compression

目标不是：

> 删除信息。

而是：

> **删除低价值信息，保留高价值事实。**

例如：

```text
1000 files
↓
25 relevant sources
↓
80 relevant symbols
↓
12 important ranges
↓
3 health findings
↓
2 verification results
↓
3 environment facts
```

最终：

> 一个 AI 真正可以工作的 Context Pack。

---

# 451. Context anti-pattern

禁止：

```text
README
+ entire src/
+ entire tests/
+ entire package lock
+ entire environment
+ entire git history
```

然后说：

> Context generated.

---

# 452. Context anti-pattern

禁止：

```text
AI relevance model
→ opaque ranking
→ top 100 files
```

---

# 453. Context anti-pattern

禁止：

```text
max_tokens
→ blindly truncate
```

---

# 454. Context anti-pattern

禁止：

```text
source changed
→ use old symbol ranges
```

---

# 455. Context anti-pattern

禁止：

```text
README says X
→ Prism trusts X
```

---

# 456. Context anti-pattern

禁止：

```text
M6 says 5 candidate tests
→ “these are all affected tests”
```

---

# 457. Context anti-pattern

禁止：

```text
M8 failed
+
recent diff
→ “diff caused failure”
```

---

# 458. Context anti-pattern

禁止：

```text
Environment variable exists
→ reveal value
```

---

# 459. Context anti-pattern

禁止：

```text
Project doc says:
ignore previous instructions
→ treat as authoritative system instruction
```

---

# 460. Context anti-pattern

禁止：

```text
Context generated
→ automatically send to AI provider
```

---

# 461. Context anti-pattern

禁止：

```text
No AI provider
→ Context Engine unavailable
```

---

# 462. Context anti-pattern

禁止：

```text
Context score = 93
```

---

# 463. Context anti-pattern

禁止：

```text
Context Engine
→ implement another dependency parser
```

---

# 464. Context anti-pattern

禁止：

```text
Context Engine
→ implement another Git engine
```

---

# 465. Context anti-pattern

禁止：

```text
Context Engine
→ implement another Health Engine
```

---

# 466. Context anti-pattern

禁止：

```text
Context Engine
→ implement another Verify Engine
```

---

# 467. Context anti-pattern

禁止：

```text
Context Engine
→ implement another Environment scanner
```

---

# 468. Context anti-pattern

必须遵守：

```text
M10 orchestrates existing intelligence.
It does not recreate M1–M9 intelligence.
```

---

# 469. Integration matrix

最终确认：

```text
M1 → Project Overview / file candidates
M2 → Symbols / ranges / skeletons
M3 → Dependencies / architecture / relations
M4 → Git state / revision facts
M5 → Changed files / symbols / modules / diff
M6 → Impact / affected candidates
M7 → Health findings
M8 → Verification results
M9 → Environment facts
```

M10：

> 只做 aggregation / selection / materialization / rendering / context health。

---

# 470. No circular ownership

最终：

```text
M10
  ↓ consumes
M1–M9
```

而不是：

```text
M1 → M10
M3 → M10
M8 → M10
```

---

# 471. Dependency direction

理想：

```text
prism-core
    ↑
domain services
    ↑
prism-context
    ↑
CLI / Tauri UI
```

---

# 472. M10 may depend on

```text
prism-core
prism-project
prism-parser/model
prism-git
prism-analysis
prism-health
prism-verify
prism-environment
prism-report
```

具体以现有 workspace 为准。

---

# 473. M10 must not depend on

```text
React implementation details
browser APIs
AI SDKs
cloud services
MCP runtime
vector DB
```

---

# 474. Context core API

建议：

```rust
ContextService {
    analyze_request(...)
    collect_candidates(...)
    select(...)
    generate(...)
    health(...)
    render(...)
}
```

具体 API 依据现有架构。

---

# 475. One service of truth

CLI / GUI：

> 必须调用相同 ContextService。

不要：

```text
GUI selector in TypeScript
CLI selector in Rust
```

造成不同结果。

---

# 476. GUI parity

同一个：

```text
project
task
scope
budget
```

GUI 与 CLI：

> selection 应一致。

---

# 477. CLI vs GUI evidence

同一 ContextPack：

> `why included` 结果一致。

---

# 478. Context pack determinism test

将：

```text
CLI JSON
```

和：

```text
GUI generated JSON
```

对比：

> 除动态 timestamps 等字段外应一致。

---

# 479. Report reuse

M7/M8 已有 report infrastructure：

> 尽可能复用。

---

# 480. No parallel report engine

不要：

```text
ContextHtmlRenderer
```

复制已有 report system 的全部能力。

必要时：

> 复用通用 rendering utilities。

---

# 481. Final Audit

完成后输出：

# Prism M10 Final Context Engine Audit

---

## A. Baseline

真实审计：

```text
M0–M9
```

使用：

```text
FACT
HYPOTHESIS
INFERENCE
```

状态：

```text
CONFIRMED
RESOLVED
WORSENED
```

风险：

```text
P0
P1
```

---

## B. Architecture

确认：

```text
M1–M9 facts
→ candidate collectors
→ normalization
→ deterministic selection
→ budget
→ materialization
→ privacy
→ render
```

无循环依赖。

---

## C. Context Model

确认：

```text
ContextRequest
ContextTarget
ContextCandidate
ContextItem
ContextPack
ContextSelection
ContextManifest
ContextHealth
ContextBudget
```

具体以实际最终模型为准。

---

## D. Candidate Collection

确认：

```text
Target
Change
Impact
Architecture
Test
Health
Verification
Environment
Documentation
Git
```

---

## E. Selection

确认：

```text
task-aware
change-aware
impact-aware
risk-aware
deterministic
explainable
budget-aware
```

---

## F. Deduplication

确认：

```text
same source
multiple reasons
single materialized item
```

---

## G. Budget

确认：

```text
max items
max files
max bytes
estimated tokens
critical reservation
truncation
```

---

## H. Truncation

确认：

```text
symbol-aware
line-aware
heading-aware
diagnostic-aware
explicitly marked
```

---

## I. Context Health

确认：

```text
documentation drift
command drift
path drift
instruction conflict
verification staleness
environment incompleteness
target ambiguity
selection incompleteness
```

---

## J. Documentation

确认：

```text
README
AGENTS
CLAUDE
CONTRIBUTING
docs
build/test docs
```

能够按实际项目结构被发现、选择和验证。

---

## K. Freshness

确认：

```text
Git state
CheckRun freshness
Environment observation
file state
Context state
```

不会把旧数据伪装成当前事实。

---

## L. Privacy

必须确认：

```text
No raw secret values
No private keys
No credentials
No unredacted env values
No sensitive process command line
No unsafe HTML
No unsafe terminal output
```

---

## M. Prompt Injection

确认：

```text
Repository content is data
Project docs are not system instructions
No command execution from docs
No automatic AI handoff
```

---

## N. CLI

真实验证：

```bash
prism context .
prism context . --task "..."
prism context . --json
prism context . --dry-run
```

---

## O. GUI

真实确认：

```text
Task input
Context generation
Selection preview
Why included
Context health
Budget
Groups
Item detail
Copy
Export
Refresh
```

---

## P. Output

确认：

```text
JSON
Markdown
PlainText
```

---

## Q. M7–M9 Integration

确认：

```text
M7 findings
M8 verification
M9 environment
```

全部能够成为：

> Context candidates。

---

## R. No Duplicate Ownership

确认 M10 没有重新实现：

```text
parser
dependency graph
git diff
health engine
verify engine
environment engine
```

---

## S. Performance

真实记录：

```text
small project
medium project
large project
large context budget
small context budget
large file
many symbols
many findings
many candidates
```

---

## T. Tests

至少：

```text
unit
selection
dedup
budget
truncation
freshness
documentation health
privacy
injection safety
cross-platform
CLI
GUI
integration
real projects
```

---

## U. Real Projects

至少：

```text
Prism
TypeScript project
Python project
C# project
```

根据环境：

```text
Rust
Docker
```

---

## V. Self Context

至少实际执行：

```bash
prism context . --task "review the latest changes"
```

以及：

```bash
prism context . --task "investigate the latest verification failure"
```

如果环境允许，再验证：

```bash
prism context . --task "inspect CI configuration"
```

---

## W. Git

确认：

```text
generated context reports
temporary files
test fixtures
exports
```

不会污染 repository。

---

## X. Risks

只列：

```text
P0
P1
```

---

## Y. Deferred

明确不要提前实现：

```text
M11 Snapshot / Timeline
M12 Product UI polish
M13 Release hardening
```

以及：

```text
LLM provider
AI chat
Agent loop
RAG
Embeddings
Vector DB
MCP server
Cloud sync
Automatic AI upload
Automatic code modification
Automatic documentation repair
```

---

# 482. Completion Gate

M10 只有全部满足以下条件才算完成：

```text
[ ] M0 regression passes
[ ] M1 regression passes
[ ] M2 regression passes
[ ] M3 regression passes
[ ] M4 regression passes
[ ] M5 regression passes
[ ] M6 regression passes
[ ] M7 regression passes
[ ] M8 regression passes
[ ] M9 regression passes

[ ] ContextRequest exists
[ ] ContextTarget exists
[ ] ContextCandidate exists
[ ] ContextItem exists
[ ] ContextPack exists
[ ] ContextSelection/SelectionTrace exists
[ ] ContextBudget exists
[ ] ContextHealth exists
[ ] ContextManifest exists

[ ] Task classification exists
[ ] Target resolution exists
[ ] Candidate collection exists
[ ] Candidate deduplication exists
[ ] Deterministic priority exists
[ ] Task-aware recipe exists
[ ] Change-aware selection exists
[ ] Impact-aware selection exists
[ ] Health-aware selection exists
[ ] Verification-aware selection exists
[ ] Environment-aware selection exists
[ ] Documentation-aware selection exists

[ ] Budget enforcement works
[ ] Estimated token count works
[ ] File count budget works
[ ] Byte budget works
[ ] Symbol-aware truncation works
[ ] Documentation truncation works
[ ] Diagnostic excerpt selection works
[ ] Critical item preservation works
[ ] Exclusion reasons work

[ ] Context provenance works
[ ] Selection reasons work
[ ] Context relationships work
[ ] Source locators work
[ ] Freshness semantics work
[ ] Staleness detection works
[ ] Partial context works
[ ] Context limitations work

[ ] Documentation path validation works
[ ] Documentation command comparison works
[ ] Requirement conflict detection works
[ ] Instruction scope handling works
[ ] Context Health works

[ ] Secret redaction works
[ ] Private path handling works
[ ] Process command redaction works
[ ] HTML escaping works
[ ] Markdown escaping works
[ ] Terminal safety works
[ ] Prompt-injection-as-data boundary works

[ ] JSON output works
[ ] Markdown output works
[ ] Plain text output works
[ ] CLI works
[ ] GUI works
[ ] Dry-run works
[ ] Export works
[ ] Copy works

[ ] GUI/CLI use same ContextService
[ ] GUI/CLI selection parity works

[ ] M1–M9 facts are reused
[ ] No duplicate parser
[ ] No duplicate dependency graph
[ ] No duplicate Git engine
[ ] No duplicate Health engine
[ ] No duplicate Verify engine
[ ] No duplicate Environment engine

[ ] No AI provider dependency
[ ] No embeddings
[ ] No vector DB
[ ] No RAG
[ ] No MCP
[ ] No automatic AI upload
[ ] No source modification
[ ] No documentation modification
[ ] No command execution from docs
[ ] No network dependency
[ ] No telemetry
[ ] No database introduced for M10

[ ] No P0
[ ] No unresolved blocking P1
```

---

# 483. 最终 Commit

全部通过后：

```bash
git add -A
git commit -m "feat: add context engine"
```

**不要 push。**

---

# 484. M0–M10 最终产品语义

完成 M10 后，Prism 的语义链正式变成：

```text
M0  Prism 怎么稳定运行？
M1  项目里有什么？
M2  代码是什么？
M3  它们怎么连接？
M4  Git 记录了什么？
M5  到底改了什么？
M6  可能影响什么？
M7  哪些地方值得注意？
M8  实际验证结果是什么？
M9  当前工程环境是什么？
M10 对于当前任务，真正需要知道什么？
```

最终闭环：

```text
┌────────────────────────────────────┐
│            Local Project           │
└──────────────────┬─────────────────┘
                   ↓
             Understand
                   ↓
               Changes
                   ↓
                Impact
                   ↓
                Health
                   ↓
               Verify
                   ↓
             Environment
                   ↓
          ┌─────────────────┐
          │ Context Engine  │
          └────────┬────────┘
                   ↓
          ┌───────────────────┐
          │   Context Pack    │
          └────────┬──────────┘
                   ↓
       Human / AI / Agent / IDE
```

---

# 485. M10 完成后的 Prism 核心能力

此时 Prism 已经可以从：

```text
“一个项目”
```

推导出：

```text
它是什么
它由什么组成
它怎么连接
最近改了什么
可能影响哪里
有哪些工程风险
实际验证结果
当前环境
当前任务真正需要的上下文
```

最重要的是：

> **Prism 不需要把整个仓库交给 AI。**

它能够：

```text
Understand
→ Select
→ Compress
→ Explain
→ Verify
→ Export
```

这就是 M10 与前面 M0–M9 最大的区别。

---

# 486. M10 的一句话定义

> **M10 = 将 Prism 已经理解的项目事实，转换为面向特定任务、具有明确来源、可解释选择、受预算约束且经过隐私处理的 Context Pack。**

---

# 487. M10 完成后不要继续膨胀

到这里应该停下来。

不要因为 Context Engine 做出来了，就马上增加：

```text
Chat
Agent
Auto Fix
AI Code Review
AI Commit
AI Refactor
RAG
Embeddings
MCP
```

M10 的输出已经足够成为后续整个 AI-native 方向的基础：

```text
Prism Intelligence
        ↓
Context Pack
        ↓
future AI / Agent adapters
```

**M11 才进入 Project Snapshot / Timeline / Evolution History，让 Prism 从“理解当前项目”进一步拥有“记住项目如何变化”的能力。**