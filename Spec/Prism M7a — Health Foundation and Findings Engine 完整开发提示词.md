# Prism M7a — Health Foundation and Findings Engine
## 从“发现问题”到“证据化的项目健康分析”

## 【系列说明】

- 本文件由原《Prism M7 — Health Engine 完整开发提示词》拆分而来，是 **Part 1/2（上篇）**。
- 姊妹篇：`E:\WorkSpace\ifuyo-prism\Spec\Prism M7b — Health Rules and Reporting 完整开发提示词.md`（Part 2/2，下篇）。
- 拆分原则：内容零丢失。原文每一个章节恰好归属两篇之一；本篇保留原章节编号与标题原文，因此正文中的 “§xx” 交叉引用不失效。
- 本篇先执行；M7b 假设本篇已实现并合入。
- 原文 §区间 → 归属篇 映射表：

| 原文区间 | 归属篇 |
| --- | --- |
| 文档头（M7 定位、M0–M6 链、核心链） | M7a |
| §0–§9 | M7a |
| §10 | M7b |
| §11–§20 | M7a |
| §21–§41 | M7b |
| §42–§43 | M7a |
| §44–§62 | M7b |
| §63 | M7a |
| §64 | M7b |
| §65–§74 | M7a |
| §75–§94 | M7b |
| §95–§105 | M7a |
| §106–§110 | M7b |
| §111 | M7a |
| §112–§115 | M7b |
| §116–§117 | M7a |
| §118–§120 | M7b |
| §121–§129 | M7a |
| §130–§138 | M7b |
| §139–§140 | M7a |
| §141–§146 | M7b |
| §147–§148 | M7a |
| §149–§153 | M7b |
| §154 | M7a |
| §155–§156 | M7b |
| §157 | M7a |
| §158–§159 | M7b |
| §160–§164 | M7a |
| §165–§174 | M7b |
| §175–§184 | M7a |
| §185–§187 | M7b |
| §188 | M7a |
| §189–§194 | M7b |
| §195–§199 | M7a |
| §200 | M7b |
| §201–§208 | M7a |
| §209–§218 | M7b |
| §219–§224 | M7a |
| §225–§240 | M7b |
| §241–§253 | M7b |

- 本篇范围：里程碑定位/目标/红线、Finding 数据模型（含 FindingId 稳定性要求）、HealthRule / HealthRuleRegistry / HealthEngine 骨架、HealthContext / HealthScope / HealthOptions、severity × confidence、suppression 机制、规则执行模型（collect-sort-dedup、Partial/Truncated、取消语义）、规则注册与 `health:` 配置段、`prism health rules` CLI，并以少量简单维护类规则（TODO/FIXME）作为引擎验证载体。
- 完整规则目录、HealthSummary/HealthReport 与报告集成、`prism health` 完整 CLI 面、GUI Health 数据流、误报抽样审计、真实项目验证、完整验收清单、Final Audit 与最终 commit 指令见 M7b。

---

你现在进入 **Prism M7：Health Engine**。

M0–M6 已经建立：

```text
M1 = File Facts
M2 = Code Facts
M3 = Relationship Facts
M4 = Git Facts
M5 = Change Facts
M6 = Impact Facts
```

M7 要建立：

```text
Health Facts
```

也就是让 Prism 开始回答：

> **这个项目当前有哪些值得注意的问题？**
>
> **问题在哪里？**
>
> **依据是什么？**
>
> **严重程度与置信度如何？**
>
> **哪些问题与当前改动有关？**

核心链：

```text
Project
  ↓
Discovery
  ↓
Code Model
  ↓
Relationship Graph
  ↓
Git / ChangeSet
  ↓
Impact Analysis
  ↓
Health Engine
  ↓
Findings
  ↓
Evidence
  ↓
Reports / UI / Context
```

---

# 0. 最重要的边界

严格保持：

```text
M1 = File Facts
M2 = Code Facts
M3 = Relationship Facts
M4 = Git Facts
M5 = Change Facts
M6 = Impact Facts
M7 = Health Facts
```

M7 做：

- 项目健康规则
- 结构问题发现
- 依赖问题发现
- 架构规则检查
- 代码复杂度 / 异常结构检查
- TODO/FIXME 等维护信号
- secrets / credential 风险检测
- 文档 / 配置 / 项目事实一致性相关检查
- Git / repository hygiene 检查
- 环境与工具事实相关问题的准备性检查，但完整环境探测留 M9
- Finding 统一模型
- severity
- confidence
- evidence
- location
- related symbols/modules/files
- suppression / baseline 机制的最小基础
- health report

M7 不做：

- AI / LLM 判断
- AI 自动修复
- 自动修改代码
- Build
- Test execution
- Lint execution
- Typecheck execution
- Runtime tracing
- Profiling
- Snapshot persistence
- RAG
- Embeddings
- MCP
- Issue tracker
- CI 平台
- 全面 DevOps
- 综合“项目质量分数”作为核心结论

尤其禁止：

```text
让规则产生一个 0~100 的“项目健康分”
```

Prism 的核心必须是：

> **Findings first, score second—or never.**

---

# 1. Baseline Audit

修改任何代码之前，先审计当前真实代码：

```text
M0 Foundation
M1 Project Discovery
M2 Code Understanding
M3 Dependency & Architecture
M4 Git Basics
M5 ChangeSet
M6 Change Impact
```

不要假设之前阶段完全按照 prompt 实现。

---

# 2. Baseline Audit 分类

先输出：

```text
FACT
HYPOTHESIS
INFERENCE
```

每条再标：

```text
CONFIRMED
RESOLVED
WORSENED
```

风险仅列：

```text
P0
P1
```

必须确认：

- 当前 `Finding` 是否已经存在
- Severity / Confidence 是否已经存在
- Evidence 是否统一
- M1 文件事实是否可靠
- M2 Symbol Model 是否可靠
- M3 Dependency Graph 是否可查询
- M4 Git facts 是否可读取
- M5 ChangeSet 是否可消费
- M6 ImpactAnalysis 是否可消费
- CLI / GUI / Report 当前入口
- 当前错误模型
- 当前 diagnostics convention
- 当前测试框架与实际测试状态

---

# 3. M7 最终回答什么

例如项目：

```text
src/
  core/
  ui/
  utils/
```

Prism 不应该只告诉用户：

```text
124 files
```

M7 应能告诉用户：

```text
Findings

Architecture
  2 architecture violations

Dependencies
  1 dependency cycle
  3 unresolved references

Code
  4 very large functions
  7 high-complexity functions

Security
  1 possible hard-coded secret

Maintenance
  13 TODO/FIXME markers

Documentation
  2 project-entry commands appear stale
```

每个 finding 都必须回答：

```text
Where?
Why?
Evidence?
How certain?
```

---

# 4. Health ≠ Score

这是整个 M7 最重要的设计原则之一。

不要默认：

```text
Health = 81
```

也不要默认：

```text
Architecture = B+
Code Quality = 72
Security = 65
```

这种数字很容易制造：

> 虚假的精确感。

优先：

```text
12 findings
3 P1
9 P2
```

以及：

```text
Finding:
DependencyCycle

Location:
src/a.ts
src/b.ts

Evidence:
a.ts imports b.ts
b.ts imports a.ts
```

---

# 5. Finding 是核心产品实体

统一建立 / 扩展：

```rust
Finding
```

建议：

```rust
Finding {
    id,
    rule_id,
    category,
    severity,
    confidence,
    title,
    message,
    explanation,
    location,
    related_files,
    related_symbols,
    related_modules,
    evidence,
    suggested_action,
    status,
    metadata,
}
```

根据当前代码实际模型调整。

---

# 6. Finding 必须稳定

`FindingId` 不应该每次运行都随机变化。

应尽可能基于：

```text
rule_id
+
project-relative location
+
related entity identity
+
stable finding key
```

生成稳定 identity。

例如：

```text
dependency-cycle:
module-A + module-B + module-C
```

不要：

```text
random UUID
```

否则以后：

```text
M11 Snapshot
```

无法判断：

> 这是同一个问题还是新问题？

---

# 7. Rule

建立：

```text
HealthRule
```

建议：

```rust
HealthRule {
    id,
    name,
    category,
    description,
    default_severity,
    enabled_by_default,
}
```

不要把每个规则写死进一个超大的：

```text
health.rs
```

---

# 8. HealthCheck

统一执行入口：

```rust
HealthEngine
```

类似：

```rust
analyze(project_context, health_options)
    -> HealthAnalysis
```

其中：

```text
project_context
```

可以组合：

```text
Discovery
CodeModel
Graph
Git
ChangeSet
ImpactAnalysis
```

但：

> 不要复制这些模型。

---

# 9. HealthAnalysis

建议：

```rust
HealthAnalysis {
    project_id,
    findings,
    summary,
    diagnostics,
    analysis_completeness,
    checked_rules,
    disabled_rules,
}
```

---

# 11. Severity

统一使用已有 severity 系统。

如果 M0 已定义：

```text
Critical
High
Medium
Low
Info
```

就复用。

如果尚未统一，再最小化设计。

不要 M7 单独创建第二套 severity。

---

# 12. Confidence

Confidence 与 Severity 完全分离。

例如：

```text
Severity = High
Confidence = Medium
```

表示：

> 如果成立，问题影响较大，但静态证据并不完全确定。

也可能：

```text
Severity = Low
Confidence = Exact
```

所以：

```text
severity ≠ confidence
```

必须在 UI / JSON 中保持独立。

---

# 13. Evidence First

每个 finding 至少应该拥有一种证据：

```text
DependencyEdge
Symbol
SourceLocation
FileMetadata
GitFact
ConfigFact
CommandResult
```

例如：

```text
Rule:
dependency-cycle

Evidence:
A imports B
B imports C
C imports A
```

不能只有：

```text
message = "Architecture looks problematic"
```

---

# 14. Evidence 不要复制完整源码

Evidence 只保存：

```text
location
symbol identity
relationship
small structural fact
```

不要默认把：

```text
entire file
entire source
entire diff
```

塞进 Finding。

避免：

- 内存浪费
- JSON 巨大
- 报告泄露敏感代码
- UI 加载慢

---

# 15. Category

建议至少：

```text
Architecture
Dependency
Code
Security
Maintenance
Documentation
Configuration
Git
Environment
```

但必须根据实际规则数量决定。

不要为了分类而分类。

---

# 16. Rule registry

建议：

```text
HealthRuleRegistry
```

允许：

```text
rule_id
→
rule implementation
```

例如：

```text
ARCH001
DEP001
DEP002
SEC001
CODE001
MAINT001
DOC001
```

规则 ID 必须稳定。

---

# 17. Rule ID 不要绑实现文件

例如不要：

```text
dependency_cycle_v2
```

规则 ID 应表示：

> 用户能理解的稳定规则身份。

实现可以重构。

Rule ID 不应该随实现文件变化。

---

# 18. Rule execution model

建议：

```text
Rule
 ↓
Inputs
 ↓
Evaluation
 ↓
Finding(s)
 ↓
Evidence
```

规则应该尽可能：

```text
pure / deterministic
```

只依赖传入事实。

---

# 19. Rule context

建立轻量：

```text
HealthContext
```

包含：

```text
project
discovery
code_model
relationship_graph
git
change_set
impact_analysis
```

使用借用 / 引用，而不是复制巨大 graph。

---

# 20. Rule isolation

规则 A 不应该：

```text
调用规则 B
```

例如：

```text
DependencyCycleRule
→ HealthScoreRule
```

不要。

规则应独立产出：

```text
Finding
```

最终由 Engine 聚合。

---

# 42. TODO / FIXME

这是高性价比规则：

```text
MAINT001 TODO marker
MAINT002 FIXME marker
MAINT003 HACK marker
```

但第一版可以合并成：

```text
MAINT001 Maintenance Marker
```

需要：

- location
- marker
- line
- optional surrounding short text

不要整个文件内容。

---

# 43. Comment marker false positives

避免把：

```text
http://example.com/TODO
```

随便当 TODO。

至少要求：

```text
TODO
FIXME
HACK
XXX
```

出现在：

- comment
- known textual marker location

具体基于实际 parser / lexical data。

---

# 63. Scope model

建议：

```rust
HealthScope {
    FullProject,
    ChangedFiles,
    ChangedSymbols,
    ImpactedItems,
}
```

具体根据已有 M6 API 调整。

---

# 65. Suppression

M7 必须考虑：

> Finding 不是发现一次就永远存在。

第一版可支持极简 suppression：

```yaml
health:
  disabled_rules:
    - CODE002
```

或者：

```text
finding-level suppression
```

但不要做复杂数据库。

---

# 66. Suppression design

优先配置文件：

```text
.prism.yml
```

或现有 Prism config。

例如：

```yaml
health:
  rules:
    CODE002:
      enabled: false
```

具体路径复用现有 config system。

---

# 67. Do not hide suppressed findings by default internally

即使 UI 默认隐藏，也要区分：

```text
detected
suppressed
reported
```

否则未来 Snapshot 无法知道：

> 这个问题消失了，还是被关闭了？

---

# 68. Finding status

可以：

```text
Active
Suppressed
Ignored
```

但第一版不要模拟完整 issue 生命周期：

```text
Open
Acknowledged
InProgress
Resolved
Closed
Reopened
```

这是 issue tracker 功能。

---

# 69. Baseline / ignore

如果实现：

```text
baseline
```

要非常克制。

推荐第一版只支持：

```text
ignored rule
ignored finding identity
```

不要建立复杂持久化 finding database。

---

# 70. Config file

根据当前项目配置系统统一：

```text
.prism.yml
```

或者：

```text
prism.config.*
```

只选择一种。

不要同时制造：

```text
prism.yml
prism.json
.prism/config.yaml
```

三个系统。

---

# 71. Rule configuration

支持：

```yaml
health:
  rules:
    CODE002:
      enabled: true
      threshold: 200
```

即可。

---

# 72. Configuration schema

必须：

- parse validation
- unknown rule detection
- unknown option detection
- helpful diagnostic
- defaults
- deterministic merge

不要静默接受拼写错误：

```yaml
threshhold: 200
```

---

# 73. Rule metadata

每个 rule 应能提供：

```text
id
name
description
category
default severity
default enabled
configuration schema
```

GUI 可以未来显示。

---

# 74. Rule registry should be inspectable

CLI 可以：

```bash
prism health rules
```

输出：

```text
ARCH001  Dependency Cycle
ARCH002  High Fan-out Module
DEP001   Unresolved Dependency
CODE001  High Complexity
SEC001   Possible Hard-coded Secret
...
```

支持：

```text
--json
```

---

# 95. Determinism

同一个 project：

```text
same files
same graph
same config
```

必须得到相同：

```text
findings
IDs
ordering
summary
```

特别注意：

```text
parallel rule execution
```

不能造成随机顺序。

---

# 96. Stable finding ordering

建议排序：

```text
severity
category
rule_id
location
finding_id
```

实际顺序根据产品体验定，但必须固定。

---

# 97. Parallel rule execution

可以并行：

```text
ARCH
DEP
CODE
SEC
MAINT
DOC
```

但结果必须：

```text
collect
sort
deduplicate
```

后再输出。

---

# 98. Deduplication

同一事实被不同规则发现：

例如：

```text
DEP001 unresolved dependency
ARCH004 forbidden dependency
```

可能是两个不同 findings。

不要过度 deduplicate。

只有：

> 完全相同 rule + location + stable identity

才考虑去重。

---

# 99. Partial failure

单条 rule 失败：

```text
CODE004
```

不应该让：

```text
SEC001
DEP001
```

全部消失。

应该：

```text
rule failed
→ diagnostic
→ remaining rules continue
```

除非共享基础数据不可用。

---

# 100. Analysis completeness

统一：

```text
Complete
Partial
Truncated
Failed
```

与 M6 保持一致。

例如：

```text
Partial:
2 health rules failed

Truncated:
large project limits reached
```

---

# 101. Rule timeout

单规则可以有：

```text
timeout
```

但优先复用 M0 Task / cancellation。

不要：

```text
每个 rule 自己实现线程终止。
```

---

# 102. Cancellation

必须支持：

```text
prism health .
cancel
```

GUI：

```text
Cancel analysis
```

取消后：

```text
Cancelled
```

而不是：

```text
0 findings
```

---

# 103. Large project safety

必须测试：

```text
100k+ files
大量 symbols
巨大 graph
```

即使没有真实超大仓库，也构造 fixture / synthetic graph。

目标：

- no crash
- no unbounded memory
- cancellable
- deterministic
- progress visible

---

# 104. Progress

GUI / CLI 可以显示：

```text
Scanning health rules...
[3/12]
```

不要假造：

```text
73%
```

如果无法获得真实进度，就提供阶段状态。

---

# 105. No fake progress

严格禁止：

```text
progress += 1%
sleep(...)
```

这种视觉模拟。

---

# 111. Maintenance noise

TODO 类 finding 数量可能巨大。

必须：

- 排序
- 限制 UI 初始显示
- 支持过滤
- 支持 rule suppression

不要让：

```text
2000 TODO
```

淹没真正的问题。

---

# 116. Parser failure

如果 M2：

```text
ParseFailure
```

M7：

- 仍运行 file-level rules
- 不运行依赖 symbol 的 rules
- 生成适当 diagnostic

例如：

```text
CODE rules unavailable for src/foo.cs
reason = parse failure
```

---

# 117. Unsupported language

同样：

```text
UnsupportedLanguage
```

不要产生：

```text
fake complexity
fake dependency
```

---

# 121. Finding location

至少：

```text
file_id
relative_path
line
column
```

如果有：

```text
symbol_id
```

也记录。

不要依赖：

```text
absolute_path
```

作为 Finding identity。

---

# 122. Multiple locations

Cycle：

```text
A
B
C
```

需要：

```text
locations[]
```

而不是只能：

```text
location
```

---

# 123. Finding relationship

允许：

```text
related_findings[]
```

但第一版只在确实需要时使用。

例如：

```text
same cycle
```

不要建立复杂 finding graph。

---

# 124. Suggested action

允许：

```text
suggested_action
```

但只能是：

> 基于规则事实给出的通用下一步。

例如：

```text
Review dependency direction.
```

不要自动：

```text
rewrite this file
```

---

# 125. No auto-fix

M7 第一版：

```text
NO AUTO FIX
```

即使非常容易：

```text
remove TODO
```

也不要。

未来可以独立设计：

```text
Fix Engine
```

---

# 126. Health + AI boundary

M10 以后：

```text
Health Findings
→ Context Pack
→ AI
```

M7 不能：

```text
AI → Finding
```

核心健康分析必须：

> 可离线、可复现、无模型依赖。

---

# 127. Rule execution without AI

在完全：

```text
no API key
no internet
```

的机器上：

```bash
prism health .
```

必须完整执行所有 built-in deterministic rules。

---

# 128. Configuration privacy

如果 `.prism.yml` 包含：

```text
paths
tokens
private config
```

不要直接日志打印全部内容。

---

# 129. CLI rules command

支持：

```bash
prism health rules
```

例如：

```text
ARCH001  Dependency Cycle       enabled
DEP001   Unresolved Dependency  enabled
CODE001  High Complexity        enabled
SEC001   Possible Secret        enabled
```

---

# 139. Finding explanation structure

推荐：

```text
What:
...

Evidence:
...

Why it matters:
...
```

最后这一项必须克制。

例如：

> Unresolved dependencies can prevent reliable static analysis.

不要：

> This will definitely break production.

---

# 140. Finding language

默认避免：

```text
bad
terrible
poor architecture
messy
dangerous
broken
```

除非有非常具体、可证实的语义。

推荐：

```text
Detected
Unresolved
Ambiguous
Exceeds threshold
Violates configured rule
Missing
Potential
```

---

# 147. Testing strategy

测试必须分：

```text
unit
rule
engine
integration
CLI
GUI
real project
```

---

# 148. Rule unit tests

每条 rule：

至少：

```text
positive
negative
edge
configuration
determinism
```

---

# 154. TODO tests

测试：

```text
TODO
FIXME
HACK
XXX
```

以及：

```text
URL
string
identifier
generated code
```

避免误报。

---

# 157. Partial parser tests

如果 M2：

```text
parse failure
```

确认：

```text
dependency rules degrade gracefully
code rules skip unavailable symbols
file rules remain available
```

---

# 160. Cancellation tests

运行大 fixture：

```text
health
→ cancel
```

确认：

```text
Cancelled
```

并释放资源。

---

# 161. Determinism tests

执行：

```text
health(project)
health(project)
health(project)
```

比较：

```text
finding IDs
findings
summary
ordering
```

应一致。

---

# 162. Rule isolation test

禁用：

```text
CODE001
```

不应该让：

```text
DEP001
SEC001
```

消失。

---

# 163. Rule failure test

人为让一个 rule 输入失败。

确认：

```text
other rules still run
```

同时：

```text
analysis_completeness = Partial
```

---

# 164. Suppression tests

测试：

```text
rule enabled
rule disabled
finding suppressed
config invalid
```

确认：

```text
suppressed ≠ not detected
```

---

# 175. P0/P1 protection

新增任何规则之前：

> 不得破坏 M0–M6 已有能力。

尤其：

```text
M3 graph
M5 ChangeSet
M6 ImpactAnalysis
```

不能为了 M7：

```text
修改其 semantics
```

除非发现真实 P0/P1 问题。

---

# 176. API ownership

架构必须保持：

```text
M3 owns relationship facts
M5 owns change facts
M6 owns impact facts
M7 owns health findings
```

而：

```text
M7 consumes
M3
M5
M6
```

不是反过来。

---

# 177. No circular dependency

禁止：

```text
M6 → M7
```

如果确有共享逻辑：

提取到：

```text
prism-analysis
```

或：

```text
prism-core
```

而不是：

```text
M6 imports HealthEngine
```

---

# 178. Suggested package/crate structure

根据真实 workspace 调整。

可以类似：

```text
prism-health/
├── engine
├── model
├── rules
│   ├── architecture
│   ├── dependency
│   ├── code
│   ├── security
│   ├── maintenance
│   ├── documentation
│   └── git
├── registry
├── config
├── diagnostics
└── tests
```

不要机械复制目录。

---

# 179. Rule modules

例如：

```text
architecture/dependency_cycle
architecture/forbidden_dependency
dependency/unresolved
code/complexity
code/large_function
security/secret
maintenance/markers
documentation/command_drift
```

每个 rule：

```text
small
isolated
testable
deterministic
```

---

# 180. Avoid macro-heavy rule system

不要为了“可扩展”一开始建立复杂：

```text
procedural macros
dynamic plugin loading
scripting runtime
```

Rust trait + registry 足够。

---

# 181. Plugin boundary

M7 第一版不要引入第三方 rule plugin marketplace。

内部：

```text
built-in rules
```

未来再考虑：

```text
external rule SDK
```

---

# 182. No remote rule updates

禁止：

```text
download rules from server
```

所有 built-in rules 随版本发布。

---

# 183. Local-first

保持：

```text
No telemetry
No network
No source upload
No cloud dependency
```

---

# 184. Logging

日志只能提供：

```text
rule start
rule finish
counts
duration
error
```

不要默认记录：

```text
source content
secret values
full diff
```

---

# 188. Caching

M7 不建立新的持久化数据库。

如果已有：

```text
incremental cache
```

可以复用。

第一版优先：

```text
reuse M1/M2/M3/M5/M6 computed facts
```

而不是：

```text
health DB
```

---

# 195. Future M8 compatibility

M8 将执行：

```text
build
test
lint
typecheck
static analysis
CI checks
release checks
```

M7 不执行这些。

但是 Finding schema 应允许未来关联：

```text
CheckRunId
```

无需现在实现完整 Verify。

---

# 196. Future M9 compatibility

M9 将提供：

```text
EnvironmentSnapshot
```

M7 未来可以：

```text
EnvironmentFinding
```

因此不要把当前 environment checks 写死成 M7-specific system。

---

# 197. Future M10 compatibility

M10 需要：

```text
Health Findings
+
ChangeSet
+
ImpactAnalysis
```

生成：

```text
ContextPack
```

因此 Finding 必须：

```text
stable
structured
evidence-backed
```

---

# 198. Future M11 compatibility

M11 需要：

```text
FindingId
```

稳定。

能够比较：

```text
snapshot N
snapshot N+1
```

判断：

```text
new
existing
disappeared
```

---

# 199. Finding lifecycle preparation

当前不需要历史数据库。

但是 schema 至少保证未来可以区分：

```text
first_seen
last_seen
```

如果这些字段会导致持久化设计侵入：

> 暂缓，不为了未来而过度设计。

---

# 201. Rule result contract

每个规则内部可以返回：

```rust
RuleResult {
    findings,
    diagnostics,
}
```

而不是直接修改全局：

```text
HealthAnalysis
```

---

# 202. Rule context immutability

尽可能：

```text
Rule receives read-only context
Rule returns result
```

避免：

```text
rule A mutates shared context
```

造成执行顺序依赖。

---

# 203. Thread safety

如果规则并行执行：

确保：

```text
read-only shared models
```

或：

```text
immutable references
```

不要使用：

```text
global mutable state
```

---

# 204. Global state prohibited

尤其禁止：

```text
static mutable findings
static rule state
global project model
```

CLI 多次执行 / GUI 多项目同时分析很容易出问题。

---

# 205. Multiple projects

如果架构未来支持多个 project：

M7 不应把：

```text
Project A
Project B
```

的 findings 混在一起。

`project_id` 必须稳定。

---

# 206. Multi-root workspace

如果 M1/M3 当前已经支持 workspace：

M7 应明确：

```text
workspace-level
project-level
```

不要悄悄把一个 project 的 finding 当另一个 project。

---

# 207. Rule applicability

每个 rule 可以声明：

```text
applies_to:
Project
File
Module
Symbol
Git
Documentation
```

在不适用时：

```text
NotApplicable
```

而不是：

```text
Finding
```

---

# 208. No unnecessary diagnostics

正常：

```text
rule not applicable
```

不要生成一条用户看得见的警告。

Diagnostics 仅在：

```text
unexpected limitation
configuration error
analysis failure
```

等情况下产生。

---

# 219. Severity configuration

可以允许：

```yaml
health:
  rules:
    CODE001:
      severity: medium
```

但第一版不一定需要。

如果实现：

> 必须统一。

---

# 220. Rule disabling

支持：

```yaml
health:
  rules:
    SEC001:
      enabled: false
```

CLI：

```bash
prism health rules
```

显示最终状态。

---

# 221. Config precedence

必须定义：

```text
built-in defaults
→ project config
→ CLI override
```

不要出现：

```text
CLI 和 config 谁优先？
```

这种未定义状态。

---

# 222. Health analysis options

建议：

```rust
HealthOptions {
    scope,
    enabled_rules,
    disabled_rules,
    max_findings,
    fail_on,
}
```

保持小。

---

# 223. Max findings

可以支持：

```text
max_findings
```

但如果截断：

```text
analysis_completeness = Truncated
```

不要输出：

```text
100 findings
```

让用户以为：

> 全部就是 100 条。

---

# 224. Finding cap per rule

可以支持：

```text
max_findings_per_rule
```

尤其：

```text
TODO
```

但同样必须：

```text
Truncated
```

并显示：

```text
reported 100 of at least 237
```

如果可计算。

---

# 【阶段性完成门槛】（M7a / Part 1/2）

M7a 只有满足以下条件才算本篇完成。M7 的最终验收以 M7b 的 §250 Final Audit 与 §251 Completion Gate 为准；本篇门槛是其地基部分的前置检查。

```text
[ ] M0–M6 regression passes（所有已有测试继续通过，§175、§176）
[ ] Baseline Audit 完成，输出 FACT/HYPOTHESIS/INFERENCE 分类，风险仅列 P0/P1（§1、§2）
[ ] Finding model 落地，FindingId 稳定、可复现（§5、§6、§121–§124、§198）
[ ] Severity 与 Confidence 分离且在 JSON 中独立（§11、§12）
[ ] Evidence First，且不复制完整源码（§13、§14）
[ ] HealthRule / HealthRuleRegistry / HealthEngine 骨架可用（§7、§8、§9、§16、§17）
[ ] HealthContext / HealthScope / HealthOptions 就位（§19、§63、§222）
[ ] Rule result / context immutability / thread safety / 无全局状态（§201–§204）
[ ] Determinism + stable ordering + collect-sort-deduplicate 测试通过（§95–§98、§161）
[ ] Partial failure / Analysis completeness（Complete/Partial/Truncated/Failed）行为正确（§99、§100、§163）
[ ] Cancellation / timeout / large project safety / 真实 progress 行为正确（§101–§105、§160）
[ ] Parser failure / Unsupported language 优雅降级（§116、§117、§157）
[ ] suppression 与 finding status 按配置生效，suppressed ≠ not detected（§65–§69、§164）
[ ] health: 配置段解析、schema 校验、unknown 检测、precedence 定义（§70–§72、§219–§221）
[ ] max_findings / max_findings_per_rule 截断必须标记 Truncated（§223、§224）
[ ] `prism health rules` CLI 可用（§74、§129）
[ ] MAINT（TODO/FIXME/HACK）验证规则满足 §42–§43、§111、§154，并通过 §148 的 positive/negative/edge/configuration/determinism 测试
[ ] 无 AI、无网络、无遥测、无 Git mutation、无 auto-fix、无远程规则更新（§125–§128、§182、§183）
[ ] 无 M7 专用持久化数据库，复用 M1–M6 事实（§188）
[ ] 本篇范围内 finding 文案符合 §139、§140 的解释结构与措辞要求
```

全部通过后：

```bash
git add -A
git commit -m "feat: add health foundation (M7 part 1/2)"
```

不要 push。

不要修改 remote。

完整规则目录（架构 / 依赖 / Secret / Documentation Drift / 风险信号）、HealthSummary/HealthReport 与报告集成、`prism health` 完整 CLI 面、GUI Health 数据流、误报抽样审计、真实项目验证、完整验收清单、Final Audit 与最终 commit 指令，见姊妹篇 M7b。
