# Prism M10b — Context Health Privacy and Delivery 完整开发提示词
## M10 下篇（Part 2/2）：Context Health、Privacy、Freshness、Recipe、Renderer 与交付验收

---

# 系列说明（必读）

本文件是《Prism M10 — Context Engine 完整开发提示词》按内容域拆分后的 **Part 2/2（下篇）**。原文拆为两篇独立可执行的开发提示词，原文章节编号与标题全部保留，正文内部的 “§xx” 交叉引用不失效。

- **姊妹篇（Part 1/2，上篇）**：`Spec/Prism M10a — Context Selection Engine 完整开发提示词.md`。
- **本篇（M10b）范围**：Context Health（CTX001–CTX012）与文档健康检查、privacy/redaction（复用 M7/M9 中央脱敏、secret 永不出现、[REDACTED] 保留行号）、prompt injection 防御（仓库内容是 data 非指令）、ContextRecipe（review/debug profile）、ContextFreshness、Renderer（Json/Markdown/PlainText）与 prism-report 的关系、CLI 完整旗标（--task / --scope / --explain / --max-items / --max-bytes / --max-tokens / --include / --exclude / --profile / --dry-run / --check-health）、GUI Context 页、golden selection 测试、self-hosting（对 Prism 自身跑 review/bug/refactor/docs 任务）+ 真实项目验证、完整验收清单 + Final Audit + 最终 commit。
- **上篇（M10a）范围**：定位（确定性 Context，无 LLM/Embedding/RAG，无 AI 也必须工作）、ContextRequest / TaskKind / ContextTarget、ContextSource 与 CandidateCollector 体系、归一化与去重（身份 = Project + path + SymbolId）、优先级带（禁神秘浮点分）、ContextBudget 与 Critical reservation、lazy materialize 与文件内容四级策略、截断规则、SelectionTrace / excluded_reason、确定性排序与稳定 tie-breaker、`prism context` 基础生成与 `--json`。
- 完整验收清单（§482）、Final Audit（§481）与最终 commit（§483）在本篇。
- 本篇正文按原文 § 编号升序排列，未重新编号；映射表中属于 M10a 的区间见上篇正文。

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

# 承接 M10a（Part 1 已实现并合入）

本篇假设上篇 M10a — Context Selection Engine 已经实现并合入主干，即以下地基已存在并可用：

- `ContextRequest` / `TaskKind` 任务分类 / `ContextFocus` / `ContextTarget`（Resolved / Ambiguous，不猜）；
- `ContextSource` 分类与 CandidateCollector 体系（Target / Change / Impact / Dependency / Test / Documentation / Health / Verification / Environment / Git / Architecture 等收集器，且相互隔离）；
- 候选归一化与去重（身份 = Project + relative path + SymbolId / ModuleId / FindingId / CheckRun 实例）；
- 确定性优先级带（Critical / High / Normal / Supporting / Low，禁神秘浮点分）与稳定 tie-breaker；
- `ContextBudget`（max_items / max_bytes / max_tokens_estimate 等）与 Critical reservation、must-include；
- lazy materialization 与文件内容四级策略（Full / RelevantSymbols / Skeleton / MetadataOnly）；
- 沿 symbol / heading / line 边界的截断与 `SelectionTrace` / `excluded_reason`；
- `prism context <path>` 基础生成与 `--json`。

本篇在这套选择引擎之上，交付 M10 的健康、隐私、新鲜度、任务配方、渲染与完整交付层，并完成整个 M10 的验证、验收与收尾。

## 本篇定位与红线（继承上篇并重申，不得削弱）

- **Deterministic First**：same task + same project state + same policy 必须产生相同 selection（§138、§220）；禁神秘浮点分与黑箱排序（§24、§27、§145）；不做也不伪装单一 Context score（§126、§462）。
- **无 AI 也必须工作**：第一版无 LLM / Embedding / Vector DB / RAG / MCP（§3、§5、§253、§364、§368、§461）；AI 永远不是基础依赖（§368），也绝不自动上传任何内容（§308、§310、§460）。
- **无网络**：No fetch / No pull / No remote API / No GitHub API（§226）；不执行 remote CI（§227）；不执行文档中的命令（§56、§178、§185）。
- **隐私**：secret 永不出现；`[REDACTED]` 保留行号（§378、§379）；复用 M7/M9 中央 privacy layer，禁止第二套 redaction engine（§256、§257）；GUI 不提供显示原始 secret 的入口（§382）；用户显式要求也不能绕过（§383）。
- **仓库内容是 data 非指令**：Repository content ≠ Prism instructions（§186、§187）；不把项目文档转成 system message（§313）；User Intent > Prism System Rules > Project Data（§314、§315）。
- **只读**：context generation 永远 read-only，不改源码、不改文档、不写回 context header（§155–§158）。
- **不重复造轮子**：M10 orchestrates existing intelligence（§468）；不重新实现 parser / 依赖图 / Git / Health / Verify / Environment 引擎（§463–§467、§469）。
- **本地 commit 不 push**：最终收尾见 §483——只 commit，**不要 push**。

（注：以上为红线精炼重申；权威规范以对应 § 原文章节为准，编号保持不变。）

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

# 265. Context refresh levels

建议：

```text
Refresh Selection
Refresh Facts
```

但第一版 GUI：

> 一个 Refresh Context 就够。

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