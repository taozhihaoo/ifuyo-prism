# Prism M7b — Health Rules and Reporting
## 从“发现问题”到“证据化的项目健康分析”

## 【系列说明】

- 本文件由原《Prism M7 — Health Engine 完整开发提示词》拆分而来，是 **Part 2/2（下篇）**。
- 姊妹篇：`E:\WorkSpace\ifuyo-prism\Spec\Prism M7a — Health Foundation and Findings Engine 完整开发提示词.md`（Part 1/2，上篇）。
- 拆分原则：内容零丢失。原文每一个章节恰好归属两篇之一；本篇保留原章节编号与标题原文，因此正文中的 “§xx” 交叉引用不失效。
- 本篇开头附【承接 M7a】：假设 Part 1 已实现并合入。
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

- 本篇范围：完整规则目录（架构检查、依赖检查、Secret 扫描、Documentation Drift、风险信号）、HealthSummary/HealthReport 与报告集成、`prism health` 完整 CLI 面（--changed/--rule/--category/--fail-on）、GUI Health 数据流、误报抽样审计、真实项目验证、完整验收清单 + Final Audit + 最终 commit 指令。

## 【承接 M7a】

本篇假设 Part 1（M7a）已实现并合入，即以下地基已经可用：

- Finding 统一模型与稳定 FindingId（§5、§6），含 location / multiple locations / related entities / suggested_action（§121–§124）；
- HealthRule、HealthRuleRegistry、HealthEngine 骨架与 HealthAnalysis（§7–§9、§16、§17）；
- HealthContext、HealthScope、HealthOptions（§19、§63、§222）；
- Severity × Confidence 分离（§11、§12）、Evidence First（§13、§14）、Category（§15）；
- suppression 机制与 finding status（§65–§69）；
- 规则执行模型：pure/deterministic、rule isolation、collect-sort-deduplicate、stable ordering（§18、§20、§95–§98）；
- Partial failure、Analysis completeness（Complete/Partial/Truncated/Failed）、timeout、cancellation、large project safety（§99–§105）；
- `health:` 配置段：config file、schema 校验、rule configuration、precedence（§70–§72、§219–§221）；
- `prism health rules` CLI（§74、§129）；
- MAINT（TODO/FIXME）等简单维护规则已作为引擎验证载体落地（§42、§43、§111、§154）。

本篇在此地基上完成 M7 的完整规则目录、报告集成、完整 CLI 面、GUI 数据流、审计与最终验收。执行前仍须先完成 M7a 的 §1、§2 Baseline Audit。

## 【定位与红线（承接并重申，不削弱）】

以下红线在本篇继续严格生效，全部继承自原文，不引入新语义：

- M7 = Health Facts，严格保持 M0–M6 边界（§0）；本篇只扩展规则目录与展示层，不改地基语义。
- **Findings first, score second—or never**（§0、§4）：禁止任何 0~100 健康分，禁止游戏化（§145）。
- Deterministic First：同输入同输出（§95），并行执行必须 collect-sort-deduplicate 后输出（§97），完整度语义 Complete/Partial/Truncated/Failed 与 M6 一致（§100）。
- 无 AI：核心健康分析可离线、可复现、无模型依赖；`AI → Finding` 禁止（§126、§127）。
- 无网络 / Local-first：No telemetry / No network / No source upload / No cloud dependency（§183）；禁止下载规则（§182）；禁止 npm/NuGet/PyPI registry lookup（§229）。
- 隐私与脱敏：不打印 secret（§46），日志脱敏（§184），报告不上传、用户指定输出位置（§106），报告默认 project-relative path（§231、§232）。
- No auto-fix：不自动修改代码、不自动删 secret、不自动升级依赖（§125、§227、§228）。
- 本地 commit 不 push：最终 commit 由本篇 §252 给出，不 push、不修改 remote。
- 架构所有权不变：M3 owns relationship facts / M5 owns change facts / M6 owns impact facts / M7 owns health findings，M7 只消费（§175–§177）。

---

# 10. HealthSummary

以事实统计为主：

```text
HealthSummary {
    total_findings,
    p0_count,
    p1_count,
    p2_count,
    p3_count,

    by_category,
    by_rule,

    files_with_findings,
    modules_with_findings,
    symbols_with_findings,
}
```

不要把这些统计强行计算成：

```text
health_score
```

---

# 21. Baseline built-in rules

M7 第一版优先实现**高确定性、高价值、低争议**规则。

至少覆盖下面几大类。

---

# 22. Architecture Rules

第一批建议：

```text
ARCH001 Circular Module Dependency
ARCH002 Excessive Module Fan-out
ARCH003 Excessive Module Fan-in
ARCH004 Forbidden Layer Dependency
ARCH005 Cross-boundary Dependency
```

但：

> 后三项必须基于用户实际配置规则。

不能凭空规定所谓“正确架构”。

---

# 23. Dependency Cycle

如果 M3：

```text
A → B
B → C
C → A
```

M7：

```text
ARCH001
Dependency Cycle
```

Evidence：

```text
A → B
B → C
C → A
```

必须说明 cycle 是：

```text
detected static dependency cycle
```

不要写：

```text
bad architecture
```

作为无证据结论。

---

# 24. Cycle grouping

如果：

```text
A → B
B → A

C → D
D → E
E → C
```

应该返回两个 cycle groups。

不要生成：

```text
A
B
A
B
...
```

无限重复。

---

# 25. Cycle canonicalization

同一个 cycle：

```text
A → B → C → A
```

和：

```text
B → C → A → B
```

必须识别为同一个 cycle。

需要稳定 canonical representation。

---

# 26. Fan-out

如果：

```text
A imports
B C D E F G
```

则：

```text
ARCH002
```

可以记录：

```text
outgoing_dependencies = 6
```

但是：

> 不要立即认定“6 一定过高”。

应：

```text
threshold configurable
```

或第一版仅输出：

```text
HighFanOut
```

基于经过明确文档化的默认阈值。

最好先从事实统计开始。

---

# 27. Fan-in

如果：

```text
Utility
←
37 modules
```

可以：

```text
ARCH003
```

记录：

```text
incoming_dependencies = 37
```

同样：

> fan-in 大不等于设计错误。

因此 Finding 文案应避免绝对化。

---

# 28. Forbidden Dependency

这是非常有价值的规则。

例如用户配置：

```yaml
architecture:
  forbidden_dependencies:
    - from: ui
      to: database
```

如果真实 graph：

```text
ui → database
```

则：

```text
ARCH004
```

Evidence：

```text
Observed:
ui → database

Configured restriction:
ui must not depend on database
```

这比 Prism 自己规定：

> UI 不能碰数据库

可靠得多。

---

# 29. Layer rules

允许用户定义：

```text
layers:
  - name: presentation
    paths: [...]
  - name: application
    paths: [...]
  - name: domain
    paths: [...]
  - name: infrastructure
    paths: [...]
```

以及：

```text
allowed_dependencies
```

第一版配置要简单。

不要构建复杂 DSL。

---

# 30. Configuration philosophy

M7 不应该要求用户为每个项目写大型 architecture config。

默认：

```text
zero-config rules
```

需要业务知识的规则：

```text
opt-in configuration
```

例如：

```text
forbidden dependency
layer architecture
allowed module dependency
```

---

# 31. Dependency Rules

第一批：

```text
DEP001 Unresolved Dependency
DEP002 Ambiguous Dependency
DEP003 Circular Dependency
DEP004 Missing Local Target
DEP005 Suspicious Duplicate Dependency
```

能确定多少做多少。

---

# 32. Unresolved Dependency

如果 M3：

```text
ResolutionStatus = Unresolved
```

M7 可以形成 finding。

例如：

```text
DEP001

Could not resolve import:
@foo/bar
```

Evidence：

```text
source location
requested target
resolution diagnostic
```

---

# 33. Ambiguous Dependency

例如：

```text
foo
```

存在：

```text
candidate A
candidate B
```

则：

```text
DEP002
```

不要说：

```text
wrong dependency
```

应该：

> Dependency could not be uniquely resolved.

---

# 34. Missing local target

例如 import：

```text
./foo
```

但 foo 不存在。

如果 M3 已经确认：

```text
ResolutionStatus = Unresolved
reason = MissingLocalTarget
```

M7 直接消费这个事实。

不要重新写另一套 resolver。

---

# 35. Duplicate dependency

必须先确认项目生态。

例如：

```text
package.json
```

如果：

```text
foo = 1.x
foo = 2.x
```

是否真是问题：

> 必须看 M3 已有依赖模型。

不要为了凑规则制造误报。

---

# 36. Code Rules

第一版优先：

```text
CODE001 High Complexity
CODE002 Very Large Function
CODE003 Very Large Type
CODE004 Excessive Nesting
CODE005 Duplicate Structural Signal
```

不是写一个完整静态分析器。

---

# 37. Complexity

如果 M2 已能计算：

```text
if
else if
for
while
match
switch
catch
logical branches
```

则可以计算：

```text
cyclomatic-like complexity
```

但是必须：

> 明确定义实际算法。

不要声称这是某个标准指标，除非真实实现遵循该标准。

---

# 38. Complexity wording

不要：

```text
foo is badly written
```

改为：

```text
Function complexity exceeds configured threshold.
```

Evidence：

```text
complexity = 18
threshold = 10
```

---

# 39. Large Function

基于真实指标：

```text
line count
AST node count
symbol nesting
complexity
```

优先：

```text
source lines
```

因为最容易确定。

例如：

```text
CODE002
Function contains 312 source lines.
Configured threshold: 200.
```

---

# 40. Large Type

同理：

```text
class
struct
interface
```

可以统计：

```text
method count
field count
source lines
nested symbols
```

但默认先：

> line count / direct member count。

不要构建复杂“God Class Score”。

---

# 41. Excessive Nesting

例如：

```text
if
  for
    if
      while
        switch
```

如果 parser 信息足够，可以产生：

```text
CODE004
```

Evidence：

```text
maximum nesting depth = 7
threshold = 4
```

---

# 44. Security Rules

M7 可以实现：

```text
SEC001 Possible Hard-coded Secret
SEC002 Private Key Material
SEC003 Credential-like Configuration
SEC004 Dangerous Secret Exposure in Tracked Files
```

但第一版必须极度保守。

---

# 45. Secret detection principles

禁止：

```text
看到 password 变量
→ 直接认定 secret
```

应使用：

```text
pattern
context
entropy if available
known key formats
assignment semantics
```

组合证据。

---

# 46. Never print secret

Finding：

```text
SEC001
Possible credential-like value detected.
```

不要输出：

```text
secret = sk-xxxxxxxx...
```

Evidence 只保存：

```text
location
pattern kind
redacted preview
```

例如：

```text
value = sk-***REDACTED***
```

---

# 47. Supported secret patterns

第一版只覆盖：

- PEM private key markers
- obvious credential assignment
- common token prefixes only when confidently identifiable
- known secret-shaped environment/config values

不要声称：

> 完整 secret scanner。

---

# 48. False-positive policy

所有安全 finding：

```text
confidence
```

默认不要过高。

尤其：

```text
password = "example"
```

可能只是 fixture。

可以支持：

```text
example
test
dummy
sample
placeholder
```

降低或忽略。

但规则必须可解释。

---

# 49. Generated files

M1 已经识别：

```text
Generated
```

M7 默认：

> 不对生成文件运行高噪音代码质量规则。

但是：

- secret detection
- dependency metadata
- tracked generated files

是否继续检测，必须单独定义。

不要简单：

```text
Generated = ignore everything
```

---

# 50. Binary files

默认：

```text
code rules = NotApplicable
```

但：

```text
Git tracking
secret policy
file metadata
```

可继续使用。

---

# 51. Documentation Rules

M7 第一个重点不是“AI 判断文档写得好不好”。

而是：

> **事实一致性。**

例如：

README 写：

```bash
npm run build
```

但项目当前：

```text
package.json
```

不存在：

```text
build
```

这可以形成：

```text
DOC001 Documentation Command Drift
```

---

# 52. Documentation command drift

需要基于：

```text
package.json
Cargo.toml
pyproject.toml
.csproj
solution
Makefile
```

等真实 config。

如果 README 提到：

```bash
npm test
```

项目没有对应 script：

```text
DOC001
```

Evidence：

```text
README line
package.json scripts
```

---

# 53. Documentation entry-point drift

如果文档说：

```text
src/index.ts
```

但该路径已经不存在：

```text
DOC002
```

但是必须避免：

> 普通文字中的路径误报。

优先识别：

- code blocks
- markdown links
- clearly formatted paths

---

# 54. No semantic documentation grading

禁止：

```text
README quality = 63
```

也不要判断：

```text
documentation is poor
```

除非有明确事实规则。

---

# 55. Configuration Rules

例如：

```text
CONF001 Invalid Project Configuration Reference
CONF002 Missing Referenced File
CONF003 Conflicting Configuration
```

但只有在已有 parser / discovery facts 支持时实现。

---

# 56. Git Rules

第一版可实现：

```text
GIT001 Large Tracked Generated Directory
GIT002 Possible Secret Tracked by Git
GIT003 Dirty Working Tree
```

但注意：

> “dirty” 不一定是问题。

因此 `GIT003` 最好不是默认健康错误，而是：

```text
informational state
```

除非用户明确配置：

```text
release cleanliness required
```

---

# 57. Environment Rules boundary

M9 才建立完整：

```text
EnvironmentSnapshot
```

M7 不要自己扫描整个环境。

当前只允许使用：

```text
M0 known runtime
M1 detected project config
M4 Git facts
```

进行低成本事实检查。

例如：

```text
project declares Node >= 22
current known runtime = Node 20
```

如果 M0 当前已有 runtime information：

可以产生：

```text
ENV001
```

否则：

> 留到 M9。

---

# 58. Change-aware Health

M6 已经有：

```text
ChangeSet
ImpactAnalysis
```

M7 可以增加：

```text
finding.related_changes
```

例如：

```text
DependencyCycle
```

如果 cycle 中：

```text
A changed
B changed
```

则：

```text
related_to_change = true
```

但不要改变 Finding 的基础事实。

---

# 59. Change-linked finding

例如：

```text
M5:
payment.ts changed

M7:
Payment module has unresolved dependency
```

Finding 可以：

```text
related_files = payment.ts
change_set_id = ...
```

这样：

> M10 可以生成“与当前改动相关的健康问题”。

---

# 60. M6 integration

非常重要：

如果 M6：

```text
changed Utility
→
35 impacted modules
```

M7 可以识别：

```text
LargeBlastRadius
```

但不要把 M6 的事实重新实现。

M7 消费：

```text
ImpactAnalysis
```

即可。

---

# 61. Health rule: changed code

可以支持：

```text
only_changed
```

模式。

例如：

```text
prism health . --changed
```

意味着：

> 只报告当前 ChangeSet 涉及的代码及相关影响范围内的 finding。

这对开发工作流非常有价值。

---

# 62. Full vs Changed scope

必须区分：

```text
FullProjectHealth
ChangedScopeHealth
```

例如：

```text
Full:
200 findings

Current change:
3 findings
```

而不是：

```text
prism health --changed
```

仍输出整个项目 200 条。

---

# 64. Finding relatedness

如果 `ChangedSymbols` 中：

```text
A
```

M6：

```text
B
C
```

是 impacted。

那么：

```text
health --scope impacted
```

可以检查：

```text
A B C
```

但必须保留：

```text
finding.location
```

不能把 B 的 finding 伪装成 A 的。

---

# 75. Health CLI

建立：

```bash
prism health <path>
```

至少：

```text
prism health .
prism health . --json
prism health . --changed
prism health . --rule CODE001
prism health . --category Security
```

实际参数保持简单。

---

# 76. Health CLI summary

默认：

```text
Health Findings

Critical  0
High      2
Medium    7
Low       12
Info      4

Categories
Architecture  3
Dependency    4
Code          8
Security      1
Maintenance   7
```

不要：

```text
Health Score: 81
```

---

# 77. CLI detail

例如：

```text
[HIGH] DEP001 Unresolved Dependency

src/payment/service.ts:17

Could not resolve:
@shared/payment

Evidence:
resolution status = Unresolved
reason = missing local target
```

---

# 78. CLI machine output

JSON 必须包括：

```text
rule_id
category
severity
confidence
title
message
location
related_files
related_symbols
related_modules
evidence
status
```

不要只序列化：

```text
message
```

---

# 79. CLI exit behavior

需要区分：

```text
analysis failed
```

与：

```text
findings exist
```

例如：

```text
health completed successfully
but findings exist
```

不应该默认当成执行失败。

建议：

```text
exit 0 = analysis successful
exit non-zero = engine / input failure
```

如果需要：

```text
--fail-on <severity>
```

可以实现为可选能力。

例如：

```bash
prism health . --fail-on high
```

有 High/Critical finding 时退出非零。

---

# 80. `--fail-on`

第一版如果实现，必须：

- 文档化
- 测试
- 不改变默认行为

例如：

```text
default:
findings do not cause failure

--fail-on high:
High/Critical findings → non-zero
```

---

# 81. GUI Health Page

M7 让：

```text
Health
```

从 placeholder 变为真实功能。

首页：

```text
Health

Findings: 25

Critical 0
High 2
Medium 7
Low 12
Info 4
```

下面：

```text
Architecture
Dependency
Code
Security
Maintenance
Documentation
```

---

# 82. Finding list

每条：

```text
[HIGH]
DEP001
Unresolved Dependency

src/payment/service.ts:17
```

并支持：

```text
severity
category
rule
file
status
```

过滤。

---

# 83. Finding detail

点击后：

```text
Title
Severity
Confidence
Location

What:
Could not resolve dependency.

Why:
Resolver found no valid local target.

Evidence:
...

Related:
module
symbol
change
impact
```

---

# 84. Evidence interaction

用户应该能点击：

```text
Evidence
```

然后看到：

```text
Source file
Symbol
Dependency edge
Config rule
Git state
```

不要要求打开完整 IDE。

Prism 仍不是代码编辑器。

---

# 85. Source navigation

如果 GUI 能定位文件：

点击：

```text
src/payment/service.ts:17
```

应该：

> 在系统默认编辑器 / 文件浏览器打开。

不需要内置 editor。

---

# 86. Health dashboard

首页可以展示：

```text
Findings
Changed-related findings
Top categories
Recent findings
Analysis limitations
```

但不要：

```text
漂亮圆环 + 健康分数
```

作为主价值。

---

# 87. “Recent findings” boundary

M11 才做真正时间线。

M7 当前可以：

```text
current analysis
```

不保存长期历史。

---

# 88. Report integration

复用：

```text
prism-report
```

支持：

```text
JSON
Markdown
HTML
```

报告应该包含：

```text
summary
findings
evidence
rules
analysis completeness
suppression
```

---

# 89. Markdown report

例如：

```markdown
# Prism Health Report

## Summary

- Findings: 25
- High: 2
- Medium: 7

## Architecture

### ARCH001 — Dependency Cycle

Location:
...

Evidence:
...

## Security

### SEC001 — Possible Hard-coded Secret

...
```

不要生成宣传式：

> “Your project is in excellent shape.”

---

# 90. HTML report

HTML 允许：

- category filters
- severity filters
- expandable evidence
- source location links

但：

> 不要把 HTML 报告变成第二套 SPA。

简单、静态、可离线打开。

---

# 91. Rule execution performance

不要：

```text
每个 rule 自己完整扫描项目
```

例如：

```text
20 rules
×
full source scan
```

会造成：

> 20 次重复工作。

应该尽量复用：

```text
M1 facts
M2 symbols
M3 graph
```

必要时增加共享 derived facts。

---

# 92. Derived facts

例如：

```text
ComplexityMetrics
NestingMetrics
DependencyStatistics
TrackedFileClassification
DocumentationReferences
```

可以构造：

```text
HealthDerivedFacts
```

但不要：

```text
复制所有 ProjectModel
```

---

# 93. Complexity precomputation

如果多个 code rules 都需要：

```text
line count
nesting
complexity
member count
```

最好一次计算：

```text
CodeMetrics
```

然后多个 rule 消费。

---

# 94. Dependency precomputation

类似：

```text
fan_in
fan_out
cycles
unresolved
ambiguous
```

由 M3 graph / shared metrics 提供。

不要每个 rule 自己重新遍历 graph。

---

# 106. Security of reports

报告可能包含：

```text
file names
config paths
security findings
environment hints
```

因此：

- 不打印 secret
- 不上传
- 不自动写到公共目录
- 用户指定输出位置
- 日志脱敏

---

# 107. Secret detection and Git

如果：

```text
secret-like file
```

处于 Git tracked：

应该可以产生：

```text
SEC004
```

例如：

```text
Possible secret is present in a tracked file.
```

但不要上传 GitHub 扫描。

---

# 108. `.env`

默认：

```text
.env
```

本身不等于 secret。

应该：

```text
tracked .env
+
secret-like value
```

才提高 confidence。

---

# 109. Private key

如果发现：

```text
-----BEGIN PRIVATE KEY-----
```

这是强证据。

Finding：

```text
SEC002
Private key material detected
```

并：

```text
high severity
high confidence
```

具体 severity 必须根据实际规则定义文档。

---

# 110. Example/test secrets

Fixture 中经常有：

```text
sk-test...
password123
```

因此测试：

- placeholders
- test values
- sample values
- obviously fake credentials

确保不会产生大量误报。

---

# 112. Documentation noise

同理：

README 普通文本中的：

```text
foo.ts
```

不要全部变成 finding。

必须使用：

```text
evidence confidence
```

只有可信引用才产生 finding。

---

# 113. Architecture configuration UX

如果用户没有配置：

```text
forbidden dependencies
layers
```

不要运行：

```text
ARCH004
ARCH005
```

或者显示：

```text
not configured
```

不要制造所谓：

> 默认架构真理。

---

# 114. Project-type awareness

规则必须尊重 M1：

```text
project types
languages
generated files
```

例如：

```text
Cargo project
```

不应该报告：

```text
npm script missing
```

除非真检测到 Node 项目。

---

# 115. Language awareness

`CODE` 规则必须考虑：

```text
TypeScript
JavaScript
Python
C#
```

不要实现一个：

```text
regex-based universal parser
```

如果某语言无法计算某 metric：

```text
NotApplicable
```

而不是猜。

---

# 118. Generated code

默认关闭：

```text
CODE complexity
CODE size
MAINT markers
```

对 Generated 文件的检查可配置。

---

# 119. Vendor code

如果 M1 能识别：

```text
third_party
vendor
```

默认减少代码健康规则。

但不要猜目录：

```text
vendor/
third_party/
```

就是永远 vendor。

应该：

> 作为 heuristics / classification evidence。

---

# 120. Health scope by file kind

可以默认：

```text
Source
Config
Documentation
Generated
Asset
Binary
```

分别适用不同 rules。

例如：

```text
Security:
Source + Config + Documentation

Code:
Source only

Documentation:
Documentation only
```

---

# 130. Rule documentation

每条 rule 必须有文档：

```text
Rule ID
Title
Category
Description
Default severity
Confidence semantics
Inputs
Detection logic
False positive notes
Configuration
Example
```

---

# 131. Rule examples

例如：

```text
ARCH001 Dependency Cycle

Input:
M3 module dependency graph

Detection:
Find strongly connected components with >1 node.

Output:
One finding per canonical cycle/SCC.

Evidence:
cycle path / component members.
```

这种程度必须达到。

---

# 132. SCC implementation

对于 cycle：

推荐：

```text
Tarjan
```

或：

```text
Kosaraju
```

具体根据项目规模与现有 graph utilities 决定。

不要重复实现，如果 M3 已有 SCC。

---

# 133. Architectural layer rule

如果用户配置：

```yaml
architecture:
  layers:
    ui:
      paths:
        - src/ui/**
    domain:
      paths:
        - src/domain/**
    infrastructure:
      paths:
        - src/infra/**
```

并配置：

```yaml
allowed:
  ui:
    - domain
  domain: []
  infrastructure:
    - domain
```

发现：

```text
ui → infrastructure
```

生成：

```text
ARCH004
```

Evidence 必须包含：

```text
source module
target module
configured allowed relationships
observed relationship
```

---

# 134. Do not infer architecture from folder names alone

不要：

```text
ui folder
→ presentation layer
```

然后强行要求 architecture。

除非：

> 用户显式配置。

默认只能报告实际 graph facts。

---

# 135. Dependency direction correctness

再次确认：

```text
A → B
```

到底是：

```text
A depends on B
```

必须使用 M3 的真实语义。

Health rule 的方向错误是：

> P0 级架构错误。

所以必须用 fixture 专门验证。

---

# 136. Change-aware health example

真实场景：

```text
git diff:
PaymentService changed

M6:
CheckoutService potentially affected

M7:
CheckoutService has unresolved dependency
```

UI：

```text
Current change related
```

这是非常有价值的交互。

---

# 137. Changed finding relation

Finding 可带：

```text
change_relation:
    DirectlyChanged
    Impacted
    Unrelated
```

但如果当前架构已有更好的 relation model，复用。

---

# 138. Avoid false “causality”

不要写：

```text
Because you changed PaymentService,
CheckoutService dependency is broken.
```

除非真实证据。

应该写：

```text
CheckoutService has an unresolved dependency.
It is related to the current change through the dependency graph.
```

---

# 141. Health UI filters

至少：

```text
All
Critical
High
Medium
Low
Info
```

以及：

```text
Architecture
Dependency
Code
Security
Maintenance
Documentation
```

---

# 142. Changed-only UI

提供：

```text
Current Changes
```

视图：

```text
Findings related to current ChangeSet
```

非常适合开发过程。

---

# 143. Finding density

如果：

```text
1000 TODOs
```

首页不能直接展示全部。

使用：

```text
group by rule
group by category
```

例如：

```text
Maintenance Marker
132 findings
```

点击展开。

---

# 144. Health overview

建议：

```text
Health

25 findings
2 high
7 medium
12 low
4 info

Current change
3 findings

Categories
...
```

不要做成“游戏化”。

---

# 145. No gamification

禁止：

```text
XP
level
streak
green plant
health bar
```

Prism 是工程工具。

---

# 146. Report reproducibility

同样项目：

```text
same commit
same configuration
```

报告必须尽量一致。

时间戳只能作为 metadata，不影响：

```text
finding IDs
ordering
content
```

---

# 149. Architecture tests

测试：

```text
A → B → C → A
```

输出：

```text
one canonical cycle
```

以及：

```text
A → B
```

不应报告 cycle。

---

# 150. Fan-in/fan-out tests

测试：

```text
fan_in = 0
fan_in = 1
fan_in = threshold
fan_in = threshold + 1
```

以及：

```text
fan_out
```

---

# 151. Configuration rule tests

测试：

```text
allowed
forbidden
unconfigured
invalid config
unknown layer
unknown rule
```

---

# 152. Complexity tests

测试：

```text
simple function
complex function
nested function
multiline condition
switch/match
```

并验证 metric 实际定义。

---

# 153. Large function tests

测试：

```text
below threshold
equal threshold
above threshold
```

边界必须精确。

---

# 155. Secret tests

必须包含：

```text
known private-key marker
obvious credential pattern
fake test credential
placeholder
normal password word
```

验证：

> 不泄露 secret。

---

# 156. Documentation tests

至少：

```text
README references existing script
README references missing script
README references existing path
README references deleted path
normal prose false-positive case
```

---

# 158. Generated code tests

确认：

```text
generated source
```

默认不会产生预期之外的大量 code findings。

---

# 159. Binary tests

确认：

```text
binary file
```

不会：

```text
panic
UTF-8 decode crash
fake source finding
```

---

# 165. CLI integration tests

至少：

```bash
prism health .
prism health . --json
prism health . --changed
prism health rules
```

测试：

```text
exit code
stdout
stderr
JSON schema
```

---

# 166. GUI integration tests

至少确认：

```text
Health page loads
findings appear
filters work
finding detail works
location navigation works
current-change findings work
cancel works
error state works
```

---

# 167. Real project validation

至少：

### Prism 自身

### 一个 TypeScript 项目

### 一个 Python 项目

### 一个 C# 项目

检查：

```text
rule execution
finding quality
false positives
performance
GUI
CLI
```

---

# 168. Real-world security validation

不要只使用人为 fixture。

可以扫描：

> 本地真实项目中已存在的测试代码 / 示例配置。

但：

> 不允许把真实 secret 打进报告。

---

# 169. Self-scan

Prism 必须能够：

```bash
prism health .
```

扫描自己的：

```text
Rust
TypeScript
config
docs
tests
```

这是一项重要验收。

---

# 170. Self-scan expectations

Self-scan 中发现的问题：

必须全部分类：

```text
real finding
expected finding
false positive
unsupported analysis
```

不要为了“全绿”简单关闭规则。

---

# 171. False-positive audit

完成后人工检查前：

先统计：

```text
total findings
by rule
by category
```

然后抽样：

```text
10–20 findings per high-volume rule
```

确认：

```text
evidence exists
location correct
rule logic matches finding
```

---

# 172. Finding quality audit

每条 finding 必须问：

```text
Can user locate it?
Can user understand why?
Can user verify evidence?
Can user tell certainty?
```

任何一项为 No：

> 修复 finding model / UI，而不是增加更多规则。

---

# 173. Do not optimize for finding count

目标不是：

```text
find 10,000 problems
```

而是：

> 让用户看到真正值得注意、且能解释的事实。

---

# 174. Rule count

第一版不追求几十、上百规则。

推荐：

```text
10–20 high-value deterministic rules
```

具体数量根据真实实现质量决定。

宁可：

```text
12 reliable rules
```

不要：

```text
73 noisy rules
```

---

# 185. Performance measurements

必须实际记录：

```text
project files
source files
symbols
graph nodes
graph edges
rules enabled
findings
execution time
peak memory if available
```

至少测试：

```text
small
medium
real
```

---

# 186. Performance target

不要凭空指定：

```text
100k files < 1 second
```

这种不现实指标。

应该：

1. 测量当前实现。
2. 找出明显重复扫描。
3. 修复实际瓶颈。
4. 记录真实结果。

---

# 187. Rule execution timing

可以记录：

```text
ARCH001 = 30ms
DEP001 = 14ms
CODE001 = 120ms
SEC001 = 90ms
```

用于内部优化。

GUI 不需要默认展示所有 timing。

---

# 189. Incremental health

即使不做持久缓存，也应支持：

```text
--changed
```

依赖：

```text
M5
M6
```

缩小分析范围。

---

# 190. Full scan vs changed scan

必须验证：

```text
full
```

和：

```text
changed
```

结果语义不同，但规则本身一致。

例如：

```text
full:
findings anywhere

changed:
findings in changed / impacted scope
```

---

# 191. No accidental scope widening

如果：

```text
--changed
```

不要因为一个：

```text
TODO
```

就扫描整个 repository。

Scope 必须由：

```text
ChangeSet / ImpactAnalysis
```

决定。

---

# 192. Health + Git

可以显示：

```text
Current branch
Working tree dirty
Last commit
Changed files
```

但这些是：

> M4 facts。

M7 不重新实现 Git。

---

# 193. Health + ChangeSet

可以：

```text
Current change:
8 files changed
21 symbols changed

Health:
3 findings related to current change
```

这会成为 Prism 很重要的开发循环。

---

# 194. Health + Impact

例如：

```text
changed Utility
→ impacted 24 modules

Health findings among impacted:
2 unresolved deps
1 excessive fan-out
```

这比：

```text
24 impacted
```

更有价值。

---

# 200. Health report contract

JSON：

```json
{
  "project_id": "...",
  "analysis_completeness": "Complete",
  "summary": {},
  "findings": [],
  "diagnostics": [],
  "checked_rules": [],
  "disabled_rules": []
}
```

---

# 209. Example rule metadata

例如：

```text
DEP001

Name:
Unresolved Dependency

Category:
Dependency

Default Severity:
High

Input:
M3 DependencyGraph

Detection:
ResolvedStatus == Unresolved
for local dependency candidate

Evidence:
source + requested target + resolution diagnostic
```

---

# 210. Example finding

```text
DEP001

Unresolved Dependency

Location:
src/payment/service.ts:17

Message:
Could not resolve local dependency "@shared/payment".

Evidence:
ResolutionStatus = Unresolved
Reason = MissingLocalTarget

Confidence:
Exact
```

---

# 211. Example architecture finding

```text
ARCH001

Circular Module Dependency

Modules:
payment
checkout
order

Evidence:
payment → checkout
checkout → order
order → payment
```

---

# 212. Example code finding

```text
CODE002

Large Function

Location:
src/foo.ts:120

Evidence:
Source lines = 312
Threshold = 200
```

---

# 213. Example security finding

```text
SEC002

Private Key Material Detected

Location:
config/dev.pem:1

Evidence:
PEM private-key marker detected

Value:
[REDACTED]
```

---

# 214. Example documentation finding

```text
DOC001

Documented Command Not Found

Location:
README.md:42

Documentation:
npm run build

Observed project scripts:
dev
test
lint

Evidence:
package.json does not define "build"
```

---

# 215. Important caution on documentation

如果 README 是历史说明：

可能故意提到：

```text
legacy command
```

所以 confidence / message 必须谨慎：

```text
appears inconsistent
```

而不是：

```text
documentation is wrong
```

---

# 216. Health rule thresholds

所有 threshold 必须：

```text
configured
documented
testable
```

默认阈值要记录来源：

```text
product default
```

而不是装成：

```text
universal software-engineering law
```

---

# 217. Avoid standards laundering

禁止文案：

> “根据最佳实践，超过 10 个依赖就是错误。”

除非真有明确标准来源。

Prism 第一版只需要：

> 当前配置阈值。

---

# 218. User-configured thresholds

例如：

```yaml
health:
  rules:
    CODE001:
      threshold: 15
```

Finding evidence：

```text
value = 18
threshold = 15
```

非常清晰。

---

# 225. UI truncated state

例如：

```text
Maintenance Marker
100 shown
More findings were suppressed by analysis limit.
```

不要静默截断。

---

# 226. Security-sensitive UI

对于 SEC findings：

> 默认禁止“一键复制 evidence”。

避免用户无意复制 secret。

可以只复制：

```text
location
rule
redacted evidence
```

---

# 227. No automatic secret deletion

禁止：

```text
Prism found secret
→ automatically remove secret
→ git clean
```

M7 只报告。

---

# 228. No automatic dependency upgrades

禁止：

```text
unresolved dependency
→ npm install
```

或者：

```text
upgrade package
```

---

# 229. No network package resolution

M3/M7：

```text
must remain local
```

不要：

```text
npm registry lookup
NuGet lookup
PyPI lookup
```

---

# 230. Health report should be shareable locally

用户可以：

```text
prism report health .
```

生成：

```text
health-report.json
health-report.md
health-report.html
```

这些文件：

> 可以离线阅读。

---

# 231. Report redaction

报告中：

```text
absolute path
username
home directory
```

默认尽量使用：

```text
project-relative path
```

---

# 232. Environment path

如果某个 rule 必须显示 absolute path：

> 只在明确需要时。

否则：

```text
src/foo.ts
```

优先于：

```text
C:\Users\Gray\Documents\...
```

---

# 233. Testing project pollution

不要把：

```text
test fixtures
```

里的：

```text
secrets
```

写入 Git 真 secret。

所有 fixture 使用：

```text
fake values
```

并在 test 中验证：

> reports are redacted.

---

# 234. Documentation update

必须更新：

```text
README
CLI help
config documentation
rule documentation
architecture docs
```

至少覆盖：

```text
M7
Health Engine
finding model
rule list
configuration
CLI
scope
severity
confidence
limitations
```

---

# 235. Self-documenting CLI

完成后：

```bash
prism health --help
prism health rules --help
```

应该真实可用。

---

# 236. Final UX language

推荐语言：

```text
Detected
Potential
Unresolved
Ambiguous
Exceeds threshold
Violates configured rule
Not enough evidence
Analysis incomplete
```

避免：

```text
Bad
Terrible
Broken
Guaranteed failure
Will definitely fail
```

---

# 237. M7 / M8 boundary再次确认

M7：

```text
This code appears to have a complexity finding.
```

M8：

```text
Build failed.
Test X failed.
```

M7 不冒充 Verify。

---

# 238. M7 / M9 boundary再次确认

M7：

```text
Known project requirement:
Node >= 22

Known current runtime:
Node 20
```

M9：

```text
Complete environment snapshot
SDKs
compilers
package managers
tool versions
ports
processes
environment variables
```

---

# 239. M7 / M10 boundary再次确认

M7：

```text
Finding
Evidence
```

M10：

```text
Which findings should AI see?
Which source context should AI see?
```

不要混淆。

---

# 240. M7 / M11 boundary再次确认

M7：

```text
Current health
```

M11：

```text
Health over time
```

---

# 241. Product-level success scenario

最终必须真实跑通：

```text
Project
  ↓
prism health .
```

得到：

```text
2 dependency findings
3 code findings
1 maintenance finding
```

每条都有：

```text
location
severity
confidence
evidence
```

---

# 242. Second success scenario

当前改动：

```text
PaymentService
```

M6：

```text
11 potentially impacted modules
```

M7：

```text
3 findings in current change/impact scope
```

GUI：

```text
Current change
    ↓
Potential impact
    ↓
Health findings
```

整个链路完全基于：

```text
M5
M6
M7
```

而不是 AI。

---

# 243. Third success scenario

项目存在：

```text
A → B → C → A
```

M7：

```text
ARCH001
Circular Module Dependency
```

点击：

```text
Why?
```

显示：

```text
A → B
B → C
C → A
```

---

# 244. Fourth success scenario

代码：

```text
private key marker
```

M7：

```text
SEC002
```

报告中：

```text
value = REDACTED
```

确认：

> 不会泄露秘密。

---

# 245. Fifth success scenario

README：

```bash
npm run build
```

实际：

```json
{
  "scripts": {
    "dev": "...",
    "test": "...",
    "lint": "..."
  }
}
```

M7：

```text
DOC001
```

并显示：

```text
documented command
observed commands
```

---

# 246. Sixth success scenario

当前配置：

```yaml
forbidden:
  ui -> database
```

实际：

```text
ui → database
```

M7：

```text
ARCH004
```

如果用户删除配置：

```text
ARCH004 disappears
```

说明：

> 该 finding 来自项目规则，而不是 Prism 自己臆测。

---

# 247. Seventh success scenario

执行：

```bash
prism health . --changed
```

结果只涉及：

```text
changed
+
impacted
```

范围。

而：

```bash
prism health .
```

仍扫描：

> 全项目。

---

# 248. Eighth success scenario

大项目：

```text
100k files
```

执行：

```bash
prism health .
```

测试：

```text
cancel
```

结果：

```text
Cancelled
```

而不是：

```text
UI stopped
but Rust process continues
```

---

# 249. Regression protection

M7 不得破坏：

```text
M0 CLI
M1 scan
M2 inspect
M3 graph
M4 git
M5 changes
M6 impact
```

所有已有测试必须继续通过。

---

# 250. Final Audit

完成之后，输出：

# Prism M7 Final Health Audit

## A. Baseline

M0–M6 当前真实状态。

分类：

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

---

## B. Health Architecture

确认：

```text
M3 owns relationship facts
M5 owns change facts
M6 owns impact facts
M7 owns health findings
```

---

## C. Finding Model

确认：

- stable finding identity
- category
- severity
- confidence
- location
- related entities
- evidence
- status

---

## D. Rule Engine

确认：

- registry
- execution model
- rule isolation
- deterministic ordering
- configuration
- enable/disable
- partial failure

---

## E. Architecture Rules

确认真实支持：

- dependency cycles
- fan-in/fan-out
- configured forbidden dependencies
- layer rules if implemented

---

## F. Dependency Rules

确认：

- unresolved
- ambiguous
- missing local targets
- other implemented rules

---

## G. Code Rules

确认：

- complexity
- large functions
- nesting
- type size
- other implemented rules

---

## H. Security Rules

确认：

- credential detection
- private key
- redaction
- false positives
- no secret leakage

---

## I. Maintenance Rules

确认：

- TODO
- FIXME
- HACK
- noise handling

---

## J. Documentation Rules

确认：

- command drift
- path drift
- false-positive handling
- confidence

---

## K. Configuration

确认：

- config file
- schema
- defaults
- override precedence
- invalid config behavior

---

## L. Scope

确认：

```text
full
changed
impacted
```

实际语义。

---

## M. Change Integration

确认：

```text
M5 ChangeSet
+
M6 ImpactAnalysis
```

可以正确筛选：

```text
current change related findings
```

---

## N. CLI

真实执行：

```bash
prism health .
prism health . --json
prism health . --changed
prism health rules
```

并验证：

```text
stdout
stderr
exit codes
JSON
```

---

## O. GUI

确认：

```text
Health overview
Finding list
Filters
Finding detail
Evidence
Current change
Cancellation
Error state
```

---

## P. Reports

确认：

```text
JSON
Markdown
HTML
```

以及：

```text
redaction
determinism
relative paths
```

---

## Q. Tests

统计：

```text
unit
rule
integration
CLI
GUI
fixture
real projects
security
determinism
cancellation
```

必须给出真实通过数量。

---

## R. Performance

至少提供：

```text
project size
symbols
graph size
rules
findings
analysis time
memory if available
```

---

## S. Privacy / Security

确认：

```text
No network
No telemetry
No AI
No source upload
No secret leakage
No Git mutation
No auto-fix
```

---

## T. Git

确认：

```text
working tree
generated files
temporary reports
fixtures
logs
```

没有误提交。

---

## U. Risks

只列：

```text
P0
P1
```

不要为了完整而大量制造 P2/P3。

---

## V. Deferred

明确不要偷偷实现：

```text
M8 Verify Engine
M9 Environment
M10 Context Engine
M11 Snapshot
M12 Product UI polish
M13 Release hardening
```

---

# 251. Completion Gate

M7 只有满足以下条件才算完成：

```text
[ ] M0 regression passes
[ ] M1 regression passes
[ ] M2 regression passes
[ ] M3 regression passes
[ ] M4 regression passes
[ ] M5 regression passes
[ ] M6 regression passes

[ ] Finding model stable
[ ] Finding evidence works
[ ] Severity/Confidence separated
[ ] Rule registry works
[ ] Rule configuration works
[ ] Deterministic ordering works
[ ] Cancellation works
[ ] Partial rule failure works

[ ] Dependency cycle detection works
[ ] Unresolved dependency detection works
[ ] Ambiguous dependency detection works
[ ] Configured architecture rules work
[ ] Code complexity rule works
[ ] Large function rule works
[ ] Nesting rule works if implemented
[ ] Maintenance markers work
[ ] Secret detection works conservatively
[ ] Secret redaction verified
[ ] Documentation drift rule works where evidence exists

[ ] Full project health works
[ ] Changed-scope health works
[ ] Impact-scope health works
[ ] Change-linked findings work

[ ] CLI works
[ ] JSON works
[ ] Markdown works
[ ] HTML works where report infrastructure supports it
[ ] GUI Health page works

[ ] Prism self-scan works
[ ] TypeScript validated
[ ] Python validated
[ ] C# validated
[ ] Real project validation completed

[ ] No AI
[ ] No network
[ ] No telemetry
[ ] No database introduced for M7
[ ] No runtime tracing
[ ] No automatic source modification
[ ] No Git mutation
[ ] No secret leakage
[ ] No P0
[ ] No unresolved blocking P1
```

---

# 252. Final Commit

全部通过后：

```bash
git add -A
git commit -m "feat: add health engine"
```

不要 push。

不要修改 remote。

---

# 253. 最终 Prism 主链

M7 完成后，Prism 的核心链变成：

```text
M1
File Facts
    ↓
M2
Code Facts
    ↓
M3
Relationship Facts
    ↓
M4
Git Facts
    ↓
M5
Change Facts
    ↓
M6
Impact Facts
    ↓
M7
Health Facts
```

产品语义也变成：

```text
M1
它是什么？

M2
代码是什么？

M3
它们怎么关联？

M4
Git 现在是什么状态？

M5
发生了什么变化？

M6
可能影响哪里？

M7
有哪些值得注意的问题？
```

其中 M7 的核心闭环是：

```text
                Project
                   │
        ┌──────────┼──────────┐
        ▼          ▼          ▼
      Facts      Graph      Changes
        │          │          │
        └──────────┼──────────┘
                   ▼
             Health Rules
                   │
                   ▼
               Findings
                   │
          ┌────────┼────────┐
          ▼        ▼        ▼
       Location  Evidence  Confidence
                   │
                   ▼
             Developer Action
```

最终目标不是：

> “Prism 告诉你这个项目健康得分是多少。”

而是：

> **“Prism 告诉你当前项目有哪些具体事实值得注意，并且每一条都能回到证据。”**

严格保持：

```text
M5 = What changed?
M6 = What may be affected?
M7 = What should I pay attention to?
M8 = Can it be verified?
M9 = What environment am I actually running in?
M10 = What context should an AI receive?
M11 = How has the project changed over time?
```

不要把 M7 变成一个“大而全的静态分析器”，也不要为了增加功能数量引入几十个低价值规则。

**优先实现少量、高确定性、可解释、可复现的 Finding。**
