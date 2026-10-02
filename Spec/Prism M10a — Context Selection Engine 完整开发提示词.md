# Prism M10a — Context Selection Engine 完整开发提示词
## M10 上篇（Part 1/2）：确定性 Context 选择引擎

---

# 系列说明（必读）

本文件是《Prism M10 — Context Engine 完整开发提示词》按内容域拆分后的 **Part 1/2（上篇）**。原文拆为两篇独立可执行的开发提示词，原文章节编号与标题全部保留，正文内部的 “§xx” 交叉引用不失效。

- **姊妹篇（Part 2/2，下篇）**：`Spec/Prism M10b — Context Health Privacy and Delivery 完整开发提示词.md`。
- **本篇（M10a）范围**：定位（确定性 Context，无 LLM/Embedding/RAG，无 AI 也必须工作）、ContextRequest / TaskKind 任务分类 / ContextTarget（Resolved/Ambiguous 不猜）、ContextSource 分类与 CandidateCollector 体系、归一化与去重（身份 = Project + path + SymbolId）、优先级带（Critical/High/Normal/Supporting/Low，禁神秘浮点分）、ContextBudget（max_items / max_bytes / max_tokens、Critical reservation）、lazy materialize 与文件内容四级策略（Full/RelevantSymbols/Skeleton/MetadataOnly）、截断规则（沿 symbol/heading/line 边界）、SelectionTrace / excluded_reason、确定性排序与稳定 tie-breaker、`prism context` 基础生成与 `--json`。
- **下篇（M10b）范围**：Context Health（CTX001–CTX012）与文档健康检查、privacy/redaction（复用 M7/M9 中央脱敏、secret 永不出现、[REDACTED] 保留行号）、prompt injection 防御（仓库内容是 data 非指令）、ContextRecipe（review/debug profile）、ContextFreshness、Renderer（Json/Markdown/PlainText）与 prism-report 的关系、CLI 完整旗标（--task / --scope / --explain / --max-items / --max-bytes / --max-tokens / --include / --exclude / --profile / --dry-run / --check-health）、GUI Context 页、golden selection 测试、self-hosting（对 Prism 自身跑 review/bug/refactor/docs 任务）+ 真实项目验证、完整验收清单 + Final Audit + 最终 commit。
- 完整验收清单（§482）、Final Audit（§481）与最终 commit（§483）归下篇；本篇末尾给出【阶段性完成门槛】与本篇 commit 指令。
- 本篇正文按原文 § 编号升序排列，未重新编号；映射表中属于 M10b 的区间不出现在本篇正文。

### 原文 §区间 → 归属篇 映射表

| 原文 §区间 | 归属篇 |
| --- | --- |
| 前言（标题、M0–M9 总览、核心链） | M10a |
| §0–§60 | M10a |
| §61–§74 | M10b |
| §75–§116 | M10a |
| §117–§137 | M10b |
| §138–§158 | M10a |
| §159–§165 | M10b |
| §166–§177 | M10a |
| §178–§194 | M10b |
| §195–§207 | M10a |
| §208–§209 | M10b |
| §210–§220 | M10a |
| §221 | M10b |
| §222–§227 | M10a |
| §228–§248 | M10b |
| §249–§254 | M10a |
| §255–§257 | M10b |
| §258–§261 | M10a |
| §262 | M10b |
| §263–§264 | M10a |
| §265 | M10b |
| §266–§271 | M10a |
| §272–§282 | M10b |
| §283 | M10a |
| §284–§316 | M10b |
| §317–§319 | M10a |
| §320–§323 | M10b |
| §324–§331 | M10a |
| §332–§346 | M10b |
| §347–§348 | M10a |
| §349–§363 | M10b |
| §364–§368 | M10a |
| §369–§371 | M10b |
| §372–§373 | M10a |
| §374–§383 | M10b |
| §384–§388 | M10a |
| §389–§390 | M10b |
| §391 | M10a |
| §392–§417 | M10b |
| §418–§442 | M10a |
| §443–§450 | M10b |
| §451–§475 | M10a |
| §476–§487 | M10b |

---

（以下为原文总览，原样保留，含原文标题。）

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

# 阶段性完成门槛（M10a — Part 1/2）

M10a 只有全部满足以下条件才算阶段性完成。整体 M10 的完整验收清单（§482）、Final Audit（§481）与最终 commit（§483）见下篇 M10b。

```text
[ ] M0–M9 regression 全部通过
[ ] ContextRequest / TaskKind 任务分类 / ContextFocus / ContextTarget（Resolved/Ambiguous 不猜）存在
[ ] ContextSource 分类与 Source types 存在
[ ] ContextItem / ContextGroup / ContextRange / ContextManifest 模型存在
[ ] ContextSelection / SelectionTrace（含 excluded_reason）存在
[ ] CandidateCollector 体系存在且相互隔离（只产生候选，不做 ranking / budget / rendering）
[ ] 候选归一化与去重工作（身份 = Project + relative path + SymbolId / ModuleId / FindingId / CheckRun 实例）
[ ] 确定性优先级带工作（Critical / High / Normal / Supporting / Low；无神秘浮点分、无 magic weights）
[ ] 确定性排序与稳定 tie-breaker 工作（不依赖 HashMap iteration order）
[ ] Change-aware / Risk-aware / Impact-aware 选择生效（复用 M5/M6/M7/M8/M9 已有事实，不重新实现）
[ ] ContextBudget 工作：max_items / max_bytes / max_tokens_estimate / max_source_files 等
[ ] Critical reservation 与 must-include 工作，critical 内容不被静默挤掉
[ ] 预算边界行为正确：超大 / 极小 / 零预算不崩溃，行为明确
[ ] lazy materialization 与文件内容四级策略（Full / RelevantSymbols / Skeleton / MetadataOnly）工作
[ ] 截断沿 symbol / heading / line 边界，truncated = true 并记录 original_size / included_size / truncation_reason
[ ] Documentation / Instruction 文档的选择、path scope 与 precedence 工作，且不执行其中命令
[ ] Context provenance / selection reason / excluded_reason / source locator 工作
[ ] `prism context` 基础生成工作：`prism context <path>`、`--task`、`--json`（§283、§347、§348）
[ ] 本篇范围的单元测试存在且通过（selection / dedup / budget / truncation / reproducibility；完整测试规格见下篇 §395–§407）
[ ] 无 AI provider 依赖、无 embeddings、无 vector DB、无 RAG、无 MCP、无网络依赖
[ ] 不修改源代码与文档（read-only），不把 context header 写回原始文件
[ ] 未重新实现 parser / dependency graph / git / health / verify / environment 引擎
[ ] No P0，无未解决的阻塞 P1
```

全部通过后，本篇收尾：

```bash
git add -A
git commit -m "feat: add context selection engine (M10 part 1/2)"
```

**不要 push。** M10 的最终收尾（Final Audit + 完整验收 + 最终 commit）在下篇 M10b 完成。
