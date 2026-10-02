# Prism M11b — Snapshot Compare Timeline and Evolution（M11 拆分 Part 2/2）
## 完整、详细、专业开发提示词

> **【系列说明】** 本文档由《Prism M11 — Project Snapshot - Timeline - Evolution Engine 完整开发提示词》拆分而来（Part 2/2）。姊妹篇：`Prism M11a — Snapshot Capture and Storage 完整开发提示词.md`（Part 1/2）。两篇合起来覆盖原文 §0–§335 全部内容，章节编号与标题保持原文不变（正文内 "§xx" 交叉引用继续有效）。本篇是 M11 的收尾篇：完整验收清单、Final Audit 与最终 commit 指令在本篇。

> **【承接 M11a】** 本篇假设 Part 1 已实现并合入：ProjectSnapshot 与域 fingerprint、Canonical Serialization、prism-store（SQLite、migrations、事务与原子写）、捕获编排与去重、`prism snapshot` 与 `prism history` 基础面均已可用。本篇在其上完成比较、时间线、演化、导出与全部交付面。

**归属映射（原文 § → 本篇）：**

| 原文区间 | 内容域 |
| --- | --- |
| §31–§54 | Snapshot Diff、各域 Diff、Finding Identity、Health/Verification History、Environment/Context Health 历史、Timeline/TimelineEvent/事件类型、Compare UI/UX、Current Snapshot、Freshness-aware Comparison、Evolution Summary、禁单一 Score |
| §55–§62 | Historical Query、CLI 全命令面（snapshot/history/compare/timeline/export/delete） |
| §71–§80 | Compare Compatibility、历史 Finding/Verification 比较、各域 Evolution、Environment 与 Project Evolution 分离、Context History 隐私边界 |
| §85–§90 | 不允许 AI 重写历史、Provenance、Snapshot Provenance、不伪造历史、Git 历史补全、Timeline Source Label |
| §93–§102、§112–§113、§115 | 当前项目状态、Snapshot/Compare Detail、CLI JSON、Markdown 输出、Report Integration、M7/M8/M9/M10 关系、Diff 不重跑 M6、历史架构比较、Git Safety |
| §124–§136、§138–§140、§148 | Comparison/Timeline Result、历史数据不依赖 UI、GUI 页面与最低要求、Timeline UI、Snapshot List/Badge、Historical Health Detail、First Seen/Recurrence、Documentation/Context Health Evolution、No Cloud Sync/Collaboration/Upload、Recommended User Flow |
| §152–§156、§160–§163、§170、§172–§175、§178–§182 | Timeline Event IDs、Stable Ordering/Snapshot Ordering/Time Handling/Clock Uncertainty、Diff/历史 Finding/Freshness/Privacy 测试、Current 对比限制、Explainability/Evidence/Deep Link/Source Navigation、Backup/Export/Import/Data Ownership |
| §195、§208–§215、§221 | M11 完成标准、Diff Precision/History Query Performance/Privacy/CLI/GUI/Self-hosting/Real Project 测试、Git Safety Test |
| §232–§238、§240、§245–§247、§249–§251、§258–§269 | Timeline Derivation/Dedup/Historical Summary、No Hidden Inference、User Notes/System Facts、Provenance in UI、History Search、Cross-branch Compare、Unavailable/Zero Snapshot/First Diff、Current vs First、Timeline View/Historical Range、No Password Prompt/Project Privacy、Explicit/Sensitive Export/Export Metadata/History Restore、Release Semantics/M11 定位升级/用户价值/趋势图优先级/M12 输入 |
| §272–§274、§276–§295、§301–§322、§324–§335 | Required Test Matrix/Real-World/Git Commit 验收、Final Audit 全套、文档/ADR、ProjectSnapshot/SnapshotDiff/Timeline/EvolutionSummary 契约、开发顺序与 Phase 纪律、M5–M10 边界、最终结构/命名/清理/Git/Commit、最终汇报、Success Definition、最终定位、执行要求 |

**归 M11a 的部分（本篇不重复）**：§0–§30、§63–§70、§81–§84、§91–§92、§103–§111、§114、§116–§123、§137、§141–§147、§149–§151、§157–§159、§164–§169、§171、§176–§177、§183–§194、§196–§220、§222–§224、§226–§231、§239、§241–§244、§248、§252–§257、§270–§271、§275、§296–§300、§323。

---

# 31. Snapshot Diff

建立：

```text
SnapshotComparator
```

负责：

```text
Snapshot A
vs
Snapshot B
```

返回：

```text
SnapshotDiff
```

---

# 32. SnapshotDiff 至少包含

```text
ProjectStructureDiff
CodeStructureDiff
ArchitectureDiff
GitDiff
ChangeDiff
ImpactDiff
HealthDiff
VerificationDiff
EnvironmentDiff
ContextHealthDiff
```

---

# 33. Project Structure Diff

识别：

```text
files added
files removed
files renamed/moved when deterministically known
directories added
directories removed
language distribution changes
configuration changes
```

不要重新实现 Git rename detection。

尽可能复用 M4/M5。

---

# 34. Code Structure Diff

比较：

```text
symbols added
symbols removed
symbols changed
modules added
modules removed
module membership changes
public API / signature changes when already supported
```

必须区分：

```text
content changed
structure changed
```

不要简单：

```text
file hash changed
=
all symbols changed
```

优先复用 M2 symbol identity 与 location mapping。

---

# 35. Architecture Diff

识别：

```text
module added
module removed
dependency added
dependency removed
relationship changed
cycle added
cycle removed
architecture metric changed
```

结果必须是事实，例如：

```text
Dependency added:
    A -> B

Cycle introduced:
    A -> B -> C -> A
```

不要输出：

```text
架构恶化
```

除非这是用户后续自己依据具体事实判断。

---

# 36. Health Diff

M7 已经有 Finding。

M11 只需要做历史比较。

例如：

```text
New finding
Resolved finding
Persisting finding
Changed finding
```

Finding 生命周期：

```text
FirstSeen
Active
Resolved
Recurring
```

这些状态必须由历史事实计算。

不要让 M7 自己保存历史。

---

# 37. Finding Identity

如果 M7 已经存在稳定的：

```text
FindingId
RuleId
Location
Fingerprint
```

必须复用。

如果当前没有足够稳定的 identity：

M11 可以补：

```text
FindingFingerprint
```

要求：

```text
same logical finding
=
same fingerprint
```

尽量不要因为：

```text
line number changed
```

就把一个问题误判成：

```text
old resolved
new added
```

可以优先：

```text
rule_id
normalized location
normalized subject
evidence fingerprint
```

构造稳定 identity。

---

# 38. Health History

M11 应支持：

```text
first seen
last seen
resolved
recurring
```

例如：

```text
CTX004
first seen: Snapshot #12
resolved: Snapshot #18
```

但不要输出：

```text
问题修复质量 = 93%
```

---

# 39. Verification History

M8 中：

```text
CheckRun
```

是历史事实的重要来源。

M11 应保存：

```text
check name
check type
status
duration
source revision
timestamp
```

然后能够回答：

```text
过去哪些验证失败？
哪些一直通过？
某个阶段为什么没有验证？
```

允许统计：

```text
passed count
failed count
unavailable count
blocked count
```

禁止：

```text
verification score
```

---

# 40. Verification Trend

可以展示：

```text
Snapshot 1:
build = Passed
test = Passed

Snapshot 2:
build = Passed
test = Failed

Snapshot 3:
build = Passed
test = Passed
```

这是合法的：

> 历史事实。

不能自动推导：

> “项目质量下降/上升。”

---

# 41. Environment Evolution

基于 M9，比较：

```text
OS version
runtime version
SDK version
compiler version
package manager version
available tools
required tools
port observations
environment requirement match
```

例如：

```text
Node:
22.14.0 → 22.16.0
```

或者：

```text
.NET SDK:
9.0.302 → unavailable
```

保持：

```text
Observed
```

而不是：

```text
Better
Worse
```

---

# 42. Context Health History

M10 的：

```text
ContextHealth
```

可以进入 Snapshot。

例如：

```text
missing instruction file
documentation drift
stale verification
ambiguous target
critical truncation
```

这样 Prism 才能回答：

> “以前 AI 上下文为什么经常缺东西？”

但注意：

> 默认不要把完整 ContextPack 内容持久化。

只保存：

```text
ContextManifest
task metadata
selection summary
budget summary
health summary
source fingerprints
```

除非用户明确导出。

---

# 43. Timeline

M11 的第二个核心能力：

```text
Project Timeline
```

时间线不是另一个日志系统。

它应该综合：

```text
Git history
+
Prism snapshots
+
Snapshot diffs
```

形成：

```text
TimelineEvent
```

---

# 44. TimelineEvent

建议模型：

```text
TimelineEvent

id
timestamp

project_id

kind

snapshot_id?
git_commit?
reference_id?

title
summary

change_summary
health_summary
verification_summary
environment_summary

source
```

---

# 45. Timeline Event 类型

至少支持：

```text
GitCommit
SnapshotCaptured
ArchitectureChanged
HealthChanged
VerificationChanged
EnvironmentChanged
DocumentationChanged
ContextHealthChanged
```

注意：

不是每个 event 都一定要永久存储。

优先：

> Snapshot + Git 作为事实来源，TimelineEvent 作为派生 view model。

这样可以避免：

```text
Snapshot
    +
Timeline DB
```

产生双份容易不一致的数据。

---

# 46. Timeline 不要制造虚构事件

例如：

```text
2026-09-30
Architecture improved
```

如果并没有足够事实证明“improved”，禁止生成。

正确：

```text
Architecture changed
2 dependencies added
1 cycle introduced
```

---

# 47. Snapshot Timeline 与 Git Timeline

最终应呈现：

```text
2026-09-20
● Git commit abc123

2026-09-21
● Prism snapshot
  42 modules
  311 dependencies
  7 findings

2026-09-23
● Git commit def456

2026-09-24
● Prism snapshot
  45 modules
  326 dependencies
  9 findings

2026-09-25
● Verification changed
  integration tests: Failed
```

但最后一条是否单独显示，应由：

```text
SnapshotDiff
```

派生。

---

# 48. Timeline Filtering

至少支持：

```text
All
Git
Snapshots
Architecture
Health
Verification
Environment
Context
```

以及：

```text
date range
```

不要一开始加入：

```text
复杂查询语言
全文搜索引擎
AI timeline summarization
```

---

# 49. Snapshot Compare UI

虽然 M12 才做正式产品 UI，但 M11 必须提供基础 UI/IPC 数据能力。

最低可以提供：

```text
Snapshot list
Snapshot detail
Compare A vs B
Timeline
```

UI 不需要漂亮。

重点：

> 数据结构和交互正确。

---

# 50. Compare 的 UX

用户应能够选择：

```text
Current
Previous Snapshot
Selected Snapshot
```

并查看：

```text
What changed?
```

分类：

```text
Structure
Code
Architecture
Git
Health
Verification
Environment
Context
```

---

# 51. “Current” Snapshot

允许：

```text
Current = live analysis
```

然后：

```text
Current
vs
Snapshot #18
```

但必须清楚标注：

```text
Current state is live
Snapshot #18 was captured at ...
```

不要偷偷比较：

```text
不同时间 / 不同 freshness
```

然后当成完全可比。

---

# 52. Freshness-aware Comparison

比较时必须检查：

```text
A freshness
B freshness
current project revision
```

如果某域不可比较：

例如：

```text
Snapshot A:
verification = Passed @ abc123

Snapshot B:
verification = Passed @ abc123
```

可以比较。

但：

```text
Snapshot A:
verification = Passed @ abc123

Current:
code changed to def456
verification = stale
```

则：

```text
Verification diff:
Unavailable / Stale comparison
```

而不是：

```text
Passed → Unknown = regression
```

---

# 53. Snapshot Evolution Summary

可以提供：

```text
EvolutionSummary
```

例如：

```text
Between Snapshot A and B:

Files:
+24 / -8

Modules:
+3 / -1

Dependencies:
+17 / -6

Cycles:
+1

Findings:
+4 / resolved 2

Verification:
3 passed → 2 passed + 1 failed

Environment:
Node 22.14 → 22.16
```

全部为事实。

---

# 54. 不允许单一 Evolution Score

禁止：

```text
EvolutionScore
HealthScore
QualityScore
ArchitectureScore
TrendScore
```

M11 的价值不是：

> 给项目打分。

而是：

> 给用户足够可靠的历史事实。

---

# 55. Historical Query

SnapshotStore 至少支持：

```text
get_project(project_id)

list_snapshots(project_id)

get_snapshot(snapshot_id)

find_snapshot_by_fingerprint(...)

get_previous_snapshot(snapshot_id)

get_latest_snapshot(project_id)

compare_snapshots(a, b)

get_snapshot_history(project_id)

get_timeline(project_id)

get_finding_history(project_id)

get_verification_history(project_id)

get_environment_history(project_id)
```

---

# 56. CLI

新增：

```bash
prism snapshot <path>
```

基本行为：

```text
capture current project state
persist
show summary
```

---

# 57. Snapshot CLI 参数

至少：

```bash
prism snapshot <path>

--json
--markdown

--note "<text>"

--force

--refresh

--no-environment

--no-verification
```

具体参数必须根据当前 CLI 架构调整。

不要为了参数数量而复杂化。

---

# 58. History CLI

增加：

```bash
prism history <path>
```

默认：

```text
latest snapshots
```

支持：

```bash
--limit 20
--json
--markdown
```

---

# 59. Compare CLI

增加：

```bash
prism compare <path> <snapshot-a> <snapshot-b>
```

输出：

```text
Snapshot A
Snapshot B

Structure
Code
Architecture
Health
Verification
Environment
Context
```

支持：

```bash
--json
--markdown
```

---

# 60. Timeline CLI

可以：

```bash
prism timeline <path>
```

支持：

```bash
--limit
--since
--until
--kind
--json
--markdown
```

如果当前 CLI 命令数量已经较多，也可以统一成：

```bash
prism history timeline <path>
prism history compare ...
```

优先遵循现有 CLI 风格，不要为了“命名漂亮”破坏已有接口一致性。

---

# 61. History Export

M11 建议支持：

```bash
prism history export <path> --output history.json
```

导出：

```text
project metadata
snapshot manifests
snapshot diffs
timeline
```

默认不导出：

```text
secrets
raw env
private keys
full source
```

这是：

> 项目历史备份。

不是：

> 项目代码备份。

---

# 62. History Delete

为了避免数据库无限膨胀，至少提供：

```text
delete selected snapshot
clear project history
```

CLI 删除必须要求明确确认：

```bash
--yes
```

不得因为：

```text
snapshot
```

命令本身带有隐式删除行为。

---

# 71. Compare Compatibility

SnapshotComparator 必须考虑：

```text
schema compatibility
domain version compatibility
```

如果两个 snapshot：

```text
architecture model versions incompatible
```

输出：

```text
Comparison limited
```

而不是假装正常比较。

---

# 72. Historical Finding Comparison

至少实现：

```text
new
resolved
still_active
changed
unknown
```

其中：

```text
unknown
```

非常重要。

如果因为：

```text
ruleset changed
model changed
data unavailable
```

无法确认，不要硬推断。

---

# 73. Historical Verification Comparison

同理：

```text
Passed → Failed
Failed → Passed
Passed → Unavailable
Unavailable → Passed
Stale → Passed
```

这些都是状态变化。

不能自动解释原因。

---

# 74. Architecture Evolution

提供：

```text
ArchitectureEvolution
```

可以回答：

```text
modules over time
dependency count over time
cycle count over time
```

例如：

```text
Snapshot 1:
32 modules
188 edges
0 cycles

Snapshot 2:
37 modules
224 edges
1 cycle
```

这是事实。

不要生成：

```text
architecture quality -18%
```

---

# 75. Health Evolution

允许显示：

```text
finding count over snapshots
severity distribution
category distribution
new/resolved/recurring findings
```

但始终保持：

> “数量和状态” ≠ “质量分数”。

---

# 76. Verification Evolution

允许：

```text
build
test
lint
typecheck
static analysis
CI
release checks
```

按时间查看状态。

支持：

```text
latest result
historical result
failure recurrence
```

但不要预测：

```text
下一次 CI 会不会失败
```

---

# 77. Environment Evolution

允许回答：

```text
什么时候 Node 版本变了？
什么时候 .NET SDK 消失？
什么时候某个工具变为 unavailable？
什么时候 requirement 开始 conflict？
```

这会成为 Prism 很有价值的能力。

---

# 78. Environment 与 Project Evolution 必须分开

非常重要：

```text
Project Evolution
```

与：

```text
Environment Evolution
```

不能混为一谈。

例如：

```text
项目没改
但 Node 升级了
```

正确：

```text
Project state unchanged
Environment changed
```

而不是：

```text
Project changed
```

---

# 79. Context Evolution

M10 是：

```text
ephemeral context
```

M11 可以记录：

```text
context generation metadata
```

例如：

```text
Task:
Refactor TriggerRegistry

Included:
12 files
34 symbols
3 findings
2 failed checks

Health:
1 context warning
```

但：

> 默认不永久存储完整 ContextPack 内容。

---

# 80. Context History 的隐私边界

因为 ContextPack 可能包含：

```text
源码
配置
命令
路径
文档内容
```

所以：

```text
Context History
```

默认只保存：

```text
manifest
selection summary
fingerprints
task description
health metadata
```

用户明确导出时才允许输出完整 pack。

---

# 85. 不允许 AI 重写历史

历史事实必须来源于：

```text
snapshot
Git
analysis result
verification result
environment result
```

而不是：

```text
LLM interpretation
```

AI 未来可以说：

> “过去三次变化都集中在 Battle/Core。”

但底层证据必须能够回溯到具体 Snapshot/ChangeSet。

---

# 86. Provenance

所有历史事实都应该能回答：

```text
这是什么？
什么时候得到？
从哪个分析域得到？
基于哪个 revision？
使用哪个规则/模型版本？
```

因此继续复用 M10 的：

```text
Provenance
SourceLocator
Freshness
TrustLevel
```

思想。

---

# 87. Snapshot Provenance

例如：

```text
Health:
source = prism-health
captured_at = ...
ruleset = v1
git_revision = abc123
```

Verification：

```text
source = prism-verify
executed_at = ...
check_spec_version = ...
source_revision = abc123
```

Environment：

```text
source = prism-environment
observed_at = ...
```

---

# 88. 不要伪造历史

特别重要。

如果 M11 第一次运行，而此前从未保存过 Snapshot：

不能生成：

```text
过去 30 天架构趋势
```

只能：

```text
Current snapshot available since 2026-10-XX
```

Git 历史可以提供 Git timeline。

Prism-specific historical analysis：

> 从第一次 Snapshot 开始。

---

# 89. Git 历史补全

即使没有 Prism Snapshot：

M11 仍可以利用 M4：

```text
Git commits
```

显示：

```text
Git timeline
```

但不要假装 Git commit 等于：

```text
完整 ProjectSnapshot
```

因为 commit 不包含：

```text
Health
Environment
Verification
Context health
```

---

# 90. Timeline Source Label

每条 TimelineEvent 都应该标识：

```text
Git
Prism Snapshot
Derived
```

这样用户知道：

> 这是 Git 原生事实，还是 Prism 在两个 Snapshot 间推导出来的变化。

---

# 93. 当前项目状态

Snapshot Detail 页面至少应该显示：

```text
Captured at
Basis
Git revision
Branch
Working tree
Fingerprint

Files
Symbols
Modules
Dependencies
Findings
Checks

Domain freshness
```

---

# 94. Snapshot Detail

点击一个 Snapshot：

```text
Overview
Structure
Architecture
Health
Verification
Environment
Context
Git
```

第一版不需要每个 tab 都有完整高级 UI。

但 API 数据结构必须准备好。

---

# 95. Compare Detail

Compare 页面至少：

```text
Overview
Files
Symbols
Architecture
Health
Verification
Environment
Context
```

然后：

```text
Added
Removed
Changed
Unchanged
Unavailable
```

---

# 96. CLI JSON 要可机器消费

JSON 必须：

```text
schema_version
project
snapshot
domains
fingerprints
diff
limitations
```

保持稳定。

不要把：

```text
漂亮的 UI 文本
```

直接当 JSON API。

---

# 97. Markdown 输出

Markdown 用于：

```text
README
issue
release note
AI context
engineering report
```

因此应该：

```text
human-readable
evidence-oriented
deterministic
```

例如：

```markdown
# Snapshot Comparison

## Architecture

- Modules: 42 → 45
- Dependencies: 311 → 329
- New cycles: 1

## Health

- New findings: 3
- Resolved findings: 1
```

---

# 98. Report Integration

复用：

```text
prism-report
```

不要 M11 再做一套：

```text
MarkdownRenderer
JsonRenderer
HtmlRenderer
```

---

# 99. M11 与 M7 的关系

必须：

```text
M7 = 当前 Health
M11 = Health 历史
```

M7 不应该开始依赖：

```text
SnapshotStore
```

来完成实时分析。

依赖方向：

```text
Health Engine
    ↓
Health Result

Snapshot Engine
    ↓
persist Health Result
```

而不是：

```text
Health
    ↓
SnapshotStore
```

---

# 100. M11 与 M8 的关系

同理：

```text
M8 = 当前 Verification
M11 = Verification History
```

M8 不依赖 M11。

---

# 101. M11 与 M9 的关系

```text
M9 = 当前 Environment
M11 = Environment History
```

M9 不依赖 M11。

---

# 102. M11 与 M10 的关系

```text
M10 = 当前 Context
M11 = Context history metadata
```

M10 不需要数据库。

M11 可以保存：

```text
Context Manifest
```

但不要反过来让 M10 强制依赖数据库。

---

# 112. Snapshot Diff 不应该重跑 M6

M6：

```text
current ChangeSet
    ↓
current ImpactAnalysis
```

M11 比较两个历史 Snapshot 时：

不能简单：

```text
checkout old commit
run M6
```

尤其：

> M11 不应该修改用户工作树。

历史比较必须优先使用：

```text
persisted normalized snapshot facts
```

而不是改变项目状态。

---

# 113. 历史架构比较

Snapshot A：

```text
module graph A
```

Snapshot B：

```text
module graph B
```

直接：

```text
Graph Diff
```

不要：

```text
git checkout A
分析
git checkout B
分析
```

---

# 115. Git Safety

M11 的 Git 使用仍保持：

```text
read-only
```

只能：

```text
read repository state
read commit
read diff
read history
```

---

# 124. Snapshot Comparison Result

建议：

```text
SnapshotComparisonResult

snapshot_a
snapshot_b

compatibility
summary

structure_diff
code_diff
architecture_diff
git_diff
change_diff
impact_diff
health_diff
verification_diff
environment_diff
context_health_diff

limitations
```

---

# 125. Timeline Result

建议：

```text
Timeline

project
events[]
range
sources
limitations
```

---

# 126. 历史数据不要依赖 UI

CLI 可以完全使用：

```text
SnapshotStore
```

GUI 同样使用：

```text
SnapshotStore
```

这样 M12 才能轻松做出真正的 UI。

---

# 127. GUI 页面

M11 可提供基础：

```text
History
Snapshot Detail
Compare
Timeline
```

但不要现在花大量时间：

```text
animations
charts
complex visual design
```

这些留给：

> M12 Product UI。

---

# 128. M11 GUI 最低要求

用户至少能：

```text
看到历史 snapshot
看到 capture time
看到 Git revision
看到状态 summary
选择两个 snapshot
查看比较
看到 timeline
```

---

# 129. Timeline UI

第一版可以简单：

```text
● 2026-10-01
  Snapshot
  42 modules / 311 dependencies

● 2026-10-02
  Commit abc123
  Changed 8 files

● 2026-10-02
  Snapshot
  +2 modules
  +11 dependencies
  1 new finding
```

重点：

> 时间轴逻辑正确。

---

# 130. Snapshot List

至少显示：

```text
date/time
basis
revision
branch
working tree state
summary counts
```

例如：

```text
2026-10-02 21:14
Working Tree
abc123
Dirty
43 modules
326 deps
8 findings
```

---

# 131. Snapshot Badge

可以标记：

```text
Latest
Baseline
Git Revision
Working Tree
Stale Verification
Partial
```

但不要制造：

```text
Best Snapshot
Worst Snapshot
Healthy Snapshot
Bad Snapshot
```

---

# 132. Historical Health Detail

用户点一个 Finding：

应该可以看到：

```text
First Seen
Last Seen
Resolved
Recurrences
Snapshots
```

这会让 M7 的 finding 真正具有时间维度。

---

# 133. First Seen / Last Seen

例如：

```text
Finding:
ARCH003

First seen:
Snapshot #7

Still active:
Snapshot #8
Snapshot #9

Resolved:
Snapshot #10
```

如果之后又出现：

```text
Snapshot #14
```

则：

```text
Recurring
```

---

# 134. Recurrence

Recurrence 必须基于稳定 Finding identity。

不能因为：

```text
line 52 → line 73
```

就误判为不同问题。

---

# 135. Documentation Evolution

M10 已经有：

```text
documentation drift
```

M11 可以记录：

```text
when drift first appeared
when resolved
whether recurring
```

这会形成：

```text
Documentation Evolution
```

---

# 136. Context Health Evolution

类似：

```text
CTX001 appeared
CTX001 persisted
CTX001 resolved
```

以后可以回答：

> “为什么 AI 经常缺少项目上下文？”

---

# 138. No Cloud Sync

明确：

M11 不做：

```text
cloud history
account sync
cross-device sync
remote backup
```

未来有需要另行设计。

---

# 139. No Collaboration

M11 不做：

```text
multi-user history
team history
shared timeline
permission system
```

---

# 140. No Automatic Upload

永远保持：

```text
local-first
privacy-first
```

---

# 148. Recommended User Flow

用户打开 Prism：

```text
Project
    ↓
Analyze
    ↓
Review Health / Verify / Context
    ↓
Create Snapshot
```

以后：

```text
Project
    ↓
History
    ↓
Compare
    ↓
Understand evolution
```

---

# 152. Timeline Event IDs

Timeline 是派生对象时，可以不持久化独立 ID。

如果当前架构需要 ID：

使用：

```text
deterministic event identity
```

避免每次查询都生成随机 ID，导致 UI diff 抖动。

---

# 153. Stable Ordering

Timeline：

```text
timestamp ASC
```

相同 timestamp：

```text
source priority
stable id
```

必须 deterministic。

---

# 154. Snapshot Ordering

默认：

```text
captured_at DESC
```

如果相同：

```text
created_at DESC
id ASC
```

保持稳定。

---

# 155. Time Handling

数据库保存：

> UTC 时间。

UI 根据本地时区显示。

不要把：

```text
北京时间
新加坡时间
Windows locale
```

直接写入历史事实。

---

# 156. Clock Uncertainty

如果本机时间发生跳变，Timeline 应依赖：

```text
captured_at
```

但不要做复杂时间同步。

M11 不需要 NTP。

---

# 160. Diff Tests

fixture：

```text
snapshot A
snapshot B
```

测试：

```text
file added
file removed
symbol added
symbol removed
module added
dependency added
cycle added
finding added
finding resolved
verification changed
environment changed
context health changed
```

---

# 161. Historical Finding Tests

至少：

```text
first seen
persisted
resolved
recurring
unknown
```

全部覆盖。

---

# 162. Freshness Tests

测试：

```text
same revision → fresh
different revision → stale
environment time differs → environment observation differs
ruleset differs → compatibility warning
```

---

# 163. Privacy Tests

必须测试：

```text
API key
token
password
private key
secret env var
connection string
command line secret
```

不会进入 Snapshot 持久化 JSON。

---

# 170. “Current” 与 Snapshot 对比限制

当前分析若：

```text
部分失败
```

Compare 不能假装：

```text
完整 compare
```

应显示：

```text
Comparison incomplete
```

并列出：

```text
missing domains
```

---

# 172. Explainability

CLI：

```bash
prism compare ... --explain
```

应该可以回答：

```text
Why is this marked as changed?
```

例如：

```text
Architecture:
dependency edge changed
source: ArchitectureGraph
```

Health：

```text
Finding resolved because the stable finding fingerprint
was absent from Snapshot B.
```

不要解释不存在的证据。

---

# 173. Evidence

每一个历史变化尽量有：

```text
source
snapshot A
snapshot B
entity id
location if available
```

让：

```text
Timeline
```

能够跳回：

```text
Snapshot
```

进一步跳回：

```text
Project / source / Git
```

---

# 174. Deep Link

M11 应设计可扩展的 locator：

```text
snapshot://...
project://...
file://...
symbol://...
commit://...
finding://...
```

不一定真的实现 URI scheme。

目标是：

> 历史事实最终能够指向原始工程对象。

---

# 175. Historical Source Navigation

对于 Git-backed Snapshot：

可以：

```text
Snapshot
 ↓
Git revision
 ↓
File
 ↓
Symbol
```

如果当前项目对应 commit 仍存在：

读取 Git 内容。

但：

> M11 不得 checkout。

优先：

```text
git show
git cat-file
```

类只读操作。

---

# 178. Database Backup

如果用户需要真正备份：

建议通过：

```text
history export
```

导出历史元数据。

不是：

```text
copy project source
```

---

# 179. Export Format

至少：

```text
JSON
```

可选：

```text
Markdown
```

不要 M11 做：

```text
Protobuf archive
Parquet
custom binary database format
```

---

# 180. Export Schema

例如：

```json
{
  "schema_version": "...",
  "project": {},
  "snapshots": [],
  "timeline": [],
  "limitations": []
}
```

保持 forward-compatible。

---

# 181. Import

M11 第一版：

> 不要求 history import。

原因：

- schema reconciliation
- project identity mapping
- security
- conflict
- duplicate snapshots

可以在后续版本独立规划。

---

# 182. Data Ownership

明确：

```text
Prism DB
=
Prism-owned local state
```

而：

```text
project source
=
user-owned project state
```

M11 不得修改后者。

---

# 195. M11 完成标准

M11 至少必须能够：

```text
Open project
    ↓
Analyze
    ↓
Create Snapshot
    ↓
Persist locally
    ↓
Close Prism
    ↓
Reopen Prism
    ↓
Find previous Snapshot
    ↓
Create another Snapshot
    ↓
Compare
    ↓
See exact domain-level changes
    ↓
See timeline
```

这个闭环必须真实工作。

---

# 221. Git Safety Test

运行：

```text
snapshot
compare
history
```

前后检查：

```text
git status --porcelain
HEAD
index
```

必须完全一致。

---

# 225. Context Test

确认：

```text
M10 Context
```

不因 M11 引入数据库而产生强制依赖。

---

# 232. Timeline Derivation

理想：

```text
Snapshots + Git
    ↓
TimelineBuilder
    ↓
TimelineEvent[]
```

不要把 Timeline 变成：

```text
不可重建的另一个数据库
```

---

# 233. Timeline Event Dedup

相同 Snapshot Diff 不应该生成：

```text
duplicate events
```

例如：

```text
snapshot A → B
```

对应：

```text
ArchitectureChanged
HealthChanged
VerificationChanged
```

应稳定。

---

# 234. Historical Summary

允许：

```text
summary counts
```

例如：

```text
Architecture:
+2 modules
+14 edges
```

但 summary 必须能够追溯。

---

# 235. No Hidden Inference

不要把：

```text
dependency count increased
```

解释成：

```text
complexity worsened
```

不要把：

```text
finding count decreased
```

解释成：

```text
quality improved
```

M11 只保存与展示事实。

---

# 236. User Notes vs System Facts

明确区分：

```text
system fact
```

与：

```text
user note
```

例如：

```text
System:
3 modules added.

User:
“完成 Battle.Core 重构第一阶段。”
```

不能混淆来源。

---

# 237. Provenance in UI

UI 可以显示：

```text
Source:
Prism Health Engine

Observed:
2026-10-02 21:43

Revision:
abc123

Ruleset:
health-v3
```

这样历史不是黑箱。

---

# 238. History Search

M11 第一版：

允许：

```text
date
snapshot id
Git revision
branch
```

搜索。

不要马上做：

```text
full-text source search
semantic search
AI search
```

---

# 240. Cross-branch Compare

允许：

```text
Snapshot A on branch A
vs
Snapshot B on branch B
```

但 UI 必须明确：

```text
different branches
```

---

# 245. Unavailable Semantics

例如：

```text
Git unavailable
```

必须：

```text
Git = Unavailable
```

不要：

```text
Git = empty
```

---

# 246. Zero Snapshot Project

新项目第一次运行：

```text
No history
```

然后：

```text
Create first snapshot
```

UI 应该清楚显示：

> This is the first recorded Prism snapshot for this project.

不要显示：

```text
No changes
```

因为没有 baseline。

---

# 247. First Snapshot Diff

第一份 Snapshot：

```text
compare = none
```

不要把整个项目误判成：

```text
Added everything
```

---

# 249. Current vs First Snapshot

这是一个很重要的体验：

```text
Current
vs
First Snapshot
```

能够看到项目自 Prism 开始记录后的完整演化。

---

# 250. Timeline View

可以提供：

```text
Since first snapshot
```

这个入口。

---

# 251. Historical Range

支持：

```text
latest
first
previous
selected
```

不要第一版做复杂时间序列语言。

---

# 258. No Password Prompt

不要为了“安全”加入：

```text
Prism database password
```

这会严重损害自用桌面工具体验。

---

# 259. Project Privacy

如果用户关闭：

```text
persistence_enabled = false
```

则：

```text
M1-M10
```

继续工作。

只有：

```text
M11 persistence
```

关闭。

---

# 260. Explicit Export

用户明确执行：

```text
history export
```

才产生：

```text
history.json
```

不要后台生成。

---

# 261. Sensitive Export

Export 前再次执行：

```text
redaction
```

不能因为数据库已经安全，就假设 export 自动安全。

---

# 262. Export Metadata

导出必须记录：

```text
exported_at
Prism version
schema version
project identity
```

但不要导出：

```text
OS user name
unnecessary absolute paths
tokens
```

除非已有安全抽象明确允许。

---

# 263. History Restore

M11 不做：

```text
restore source
restore environment
restore Git
```

历史只能：

```text
inspect
compare
export
delete
```

---

# 264. Release Semantics

M11 完成以后，Prism 应该拥有：

```text
Project Memory
```

但仍然没有：

```text
AI memory
```

这个区别必须保留。

---

# 265. M11 产品定位升级

M0–M3：

> Prism 能看懂项目。

M4–M9：

> Prism 能理解项目发生了什么。

M10：

> Prism 能为当前任务组织可靠上下文。

M11：

> **Prism 能记住项目如何演化。**

---

# 266. M11 最终用户价值

用户以后打开一个项目：

可以看到：

```text
第一次被 Prism 分析：
2026-09-15

现在：
2026-10-02

期间：
+19 modules
+142 dependencies
3 architecture changes
8 findings resolved
5 findings recurring
2 verification regressions
Node upgraded
documentation drift appeared then resolved
```

这就是 M11 的价值。

---

# 267. 不要做“漂亮趋势图”优先

图表可以：

> M12 再做好。

M11 首先确保：

```text
data
model
storage
comparison
history
timeline
```

正确。

---

# 268. M12 的输入

M11 完成之后，M12 可以直接消费：

```text
ProjectSnapshot
SnapshotDiff
Timeline
EvolutionSummary
FindingHistory
VerificationHistory
EnvironmentHistory
```

因此 M11 API 必须：

> 为 M12 提供干净的数据模型。

---

# 269. M12 不应该重新查询 SQL

M12 不应该：

```text
SELECT ...
```

而应该：

```text
invoke Application service
```

---

# 272. Required Test Matrix

至少覆盖：

```text
project type:
Git / non-Git

working tree:
clean / dirty / staged / untracked

snapshot:
first / duplicate / changed / forced

health:
none / new / resolved / recurring

verification:
passed / failed / stale / unavailable

environment:
same / changed / unavailable

database:
new / migrated / locked / corrupt

platform:
Windows / Linux / macOS

privacy:
secret present / secret absent
```

---

# 273. Real-World Acceptance Test

最终至少完成以下流程：

```text
1. 打开 Prism repo
2. 完成 M1-M10 分析
3. 创建 Snapshot A
4. 修改少量真实代码
5. 不提交
6. 再分析
7. 创建 Snapshot B
8. Compare A/B
9. 查看 Health changes
10. 查看 Architecture changes
11. 查看 Verification freshness
12. 查看 Timeline
13. 关闭 Prism
14. 重启 Prism
15. History 仍存在
```

---

# 274. Git Commit Acceptance

完整 M11 结束后：

```text
git status
```

确认：

```text
仅 M11 相关源码
测试
必要文档
```

没有：

```text
database
build artifacts
temporary exports
local reports
```

进入 repo。

---

# 276. Final Audit

开发完成后必须执行：

```text
# M11 Final Audit

A. Architecture
B. Domain Model
C. Storage
D. Migration
E. Snapshot Capture
F. Fingerprints
G. Comparison
H. Timeline
I. Privacy
J. Git Safety
K. CLI
L. GUI/IPC
M. Tests
N. Cross-platform
O. Performance
P. Git cleanliness
```

---

# 277. Audit Classification

所有问题按：

```text
FACT
HYPOTHESIS
INFERENCE
```

分类。

状态：

```text
RESOLVED
CONFIRMED
WORSENED
```

风险只输出：

```text
P0
P1
```

---

# 278. Final Audit 禁止自我美化

不要写：

```text
Excellent
Production ready
Perfect
Best architecture
```

除非有明确客观依据。

应该写：

```text
PASS
PARTIAL
FAIL
NOT VERIFIED
```

---

# 279. Final Verification Commands

根据实际项目执行：

```bash
cargo check
cargo test
```

以及现有：

```bash
npm test
npm run build
```

或项目实际命令。

不要假设不存在的脚本。

---

# 280. CLI Verification

实际执行：

```bash
prism snapshot <real-project>
prism history <real-project>
prism timeline <real-project>
prism compare <real-project> <a> <b>
```

至少验证：

```text
JSON
Markdown
human readable
```

---

# 281. Migration Verification

运行：

```text
fresh database
existing database
```

都通过。

---

# 282. Persistence Verification

验证：

```text
GUI creates snapshot
CLI reads it

CLI creates snapshot
GUI reads it
```

---

# 283. Security Verification

检查：

```text
database
logs
exports
```

确认：

```text
no secret
no private key
no raw env
```

---

# 284. Source Safety Verification

检查：

```text
snapshot
history
compare
export
```

确认：

```text
no source files duplicated by default
```

---

# 285. Git Safety Verification

执行 M11 前后比较：

```bash
git status --short
git rev-parse HEAD
```

要求：

```text
HEAD unchanged
index unchanged
working tree unchanged
```

除了开发过程中本来就存在的修改。

---

# 286. No Network Verification

必要时：

```text
disable network
```

执行 M11。

必须：

```text
snapshot/history/compare
```

仍然可以工作。

---

# 287. No AI Verification

没有任何：

```text
AI API
```

情况下：

```text
M11
```

必须完整工作。

---

# 288. Performance Verification

至少测量：

```text
first snapshot
second identical snapshot
changed snapshot
history listing
comparison
timeline
```

记录：

```text
duration
snapshot size
database size
```

---

# 289. Expected Performance Philosophy

重点不是：

```text
benchmark champion
```

而是：

```text
incremental
cache-aware
non-blocking
predictable
```

---

# 290. No Premature Optimization

不要在没有数据之前加入：

```text
complex indexing
compression
background workers
specialized binary storage
distributed cache
```

---

# 291. Documentation

必须新增/更新：

```text
docs/m11-snapshot.md
```

或遵循当前项目文档组织方式。

至少说明：

```text
What Snapshot is
What it stores
What it does not store
Where data is stored
How history works
How comparison works
Privacy boundaries
CLI
Limitations
```

---

# 292. 用户必须知道的数据边界

文档明确：

> Prism Snapshot 默认保存工程事实和结构化分析结果，不保存完整源码备份。

并明确：

> Working Tree Snapshot 无法保证未来恢复当时未提交源码。

---

# 293. Developer Documentation

记录：

```text
Snapshot Model
Storage Model
Migration Strategy
Fingerprint Strategy
Comparison Semantics
Timeline Derivation
Privacy Model
```

便于 M12/M13 继续开发。

---

# 294. Architecture Diagram

文档增加简图：

```text
M1-M10 Analysis
       ↓
Snapshot Capture
       ↓
Normalize
       ↓
Fingerprint
       ↓
Snapshot Store
       ↓
SQLite
       ↓
History / Compare / Timeline
```

---

# 295. ADR

如果当前项目有 ADR：

新增：

```text
ADR: Local SQLite Project History
```

核心记录：

```text
Why persistence starts in M11
Why SQLite
Why app-data directory
Why snapshots don't store source
Why timeline is derived
```

---

# 301. ProjectSnapshot

回答：

> 某个时间点项目是什么样？

---

# 302. SnapshotDiff

回答：

> 两个时间点之间发生了什么？

---

# 303. Timeline

回答：

> 项目什么时候发生了什么？

---

# 304. EvolutionSummary

回答：

> 一段时间内哪些工程事实发生了持续变化？

---

# 305. 四者关系

```text
Snapshot
    │
    ├── SnapshotDiff
    │
    └── Timeline
             │
             └── EvolutionSummary
```

---

# 306. M11 最终能力

完成后，Prism 应至少能回答：

```text
这个项目第一次什么时候被 Prism 记录？

上一版本是什么样？

现在发生了哪些变化？

哪些模块增长最快？

哪些依赖发生变化？

什么时候出现架构 cycle？

什么时候出现某个 health finding？

什么时候解决？

是否再次出现？

哪次 verification 失败？

当时验证的是哪个 revision？

环境什么时候改变？

Context Health 什么时候开始出现问题？

Current 和一个月前相比有什么客观差异？
```

---

# 307. 但 Prism 不负责回答

禁止输出未经证据支持的：

```text
项目现在更健康
项目现在更差
架构越来越糟
代码越来越好
开发效率提升了
团队变优秀了
```

这些需要用户/分析工具基于事实做判断。

---

# 308. M11 的边界一句话

实现过程中始终牢记：

> **M11 保存“工程状态随时间的事实”，不替用户评价项目，也不保存项目源码副本。**

---

# 309. 开发顺序

严格按照以下顺序推进：

```text
Phase 1
Baseline Audit

Phase 2
Project Identity

Phase 3
Snapshot Domain Models

Phase 4
Storage / SQLite

Phase 5
Migration

Phase 6
Snapshot Capture

Phase 7
Normalization / Fingerprints

Phase 8
Deduplication

Phase 9
Snapshot Comparison

Phase 10
Finding / Verification / Environment History

Phase 11
Timeline

Phase 12
CLI

Phase 13
IPC / Minimal GUI

Phase 14
Export / Delete / Privacy

Phase 15
Tests

Phase 16
Real Project Validation

Phase 17
Final Audit

Phase 18
Commit
```

---

# 310. 每个 Phase 的纪律

每个 phase：

```text
inspect
implement
test
verify
```

不要：

```text
一次性生成全部代码
```

然后最后才发现模型不一致。

---

# 311. 代码质量要求

继续遵循 Prism 全局工程原则：

```text
readability > cleverness
explicitness > magic
deterministic > heuristic
small interfaces > giant abstractions
real tests > fake confidence
evidence > assumption
```

---

# 312. AI Coding Agent 行为要求

如果你作为 AI coding agent 执行本提示词：

不要：

```text
一次生成几十个互相耦合的文件
```

应该：

```text
先检查真实代码
建立最小模型
编译
测试
再扩展
```

---

# 313. 不要重新发明已有抽象

在当前代码里如果已有：

```text
ProjectId
FileId
SymbolId
Task
Cancellation
Progress
Finding
CheckRun
EnvironmentSnapshot
ContextPack
SourceLocator
```

必须：

> 先复用。

只有真正缺失才创建新类型。

---

# 314. 不要出现重复 Model

例如不能同时存在：

```text
ProjectSnapshot
ProjectStateSnapshot
HistoricalProjectSnapshot
ProjectHistorySnapshot
```

分别做同一件事。

应该收敛到：

```text
ProjectSnapshot
```

---

# 315. 不要出现双份 Diff

如果已有：

```text
ChangeSet
```

不要重新定义：

```text
HistoricalChangeSet
```

除非语义真的不同。

---

# 316. Snapshot Diff 是历史语义

M5：

```text
ChangeSet
=
一次 Git/working-tree change
```

M11：

```text
SnapshotDiff
=
两个历史工程状态之间的差异
```

两者不同，但可以互相引用。

---

# 317. M5/M11 边界

例如：

```text
Snapshot A:
HEAD abc123

Snapshot B:
HEAD def456
```

SnapshotDiff 可以引用：

```text
Git commits abc123 → def456
ChangeSet summary
```

但不要复制完整 Git engine。

---

# 318. M6/M11 边界

SnapshotDiff 可以比较：

```text
ImpactAnalysis summary
```

但不要重算整个 impact graph。

---

# 319. M7/M11 边界

SnapshotDiff：

```text
finding lifecycle
```

而：

```text
finding generation
```

仍由 M7。

---

# 320. M8/M11 边界

Snapshot：

```text
stores check result
```

而：

```text
executes build/test
```

仍由 M8。

---

# 321. M9/M11 边界

Snapshot：

```text
stores environment observation
```

而：

```text
discovers environment
```

仍由 M9。

---

# 322. M10/M11 边界

Snapshot：

```text
stores Context metadata
```

而：

```text
selects context
```

仍由 M10。

---

# 324. Storage 完成之后

必须真实验证：

```text
create
read
update project metadata
list
compare
delete
export
```

其中：

> Snapshot 本身原则上 immutable。

---

# 325. 最终项目结构

不要机械照抄，但目标应接近：

```text
src/
  prism-core/
  prism-project/
  prism-parser/
  prism-git/
  prism-analysis/
  prism-health/
  prism-verify/
  prism-environment/
  prism-context/
  prism-snapshot/
  prism-store/
  prism-report/
  prism-cli/
```

如果当前架构是：

```text
crates/
```

或其他布局：

> 保持现有组织方式。

---

# 326. M11 新增的主要模块

逻辑上至少有：

```text
snapshot/
    model
    capture
    fingerprint
    compare
    timeline
    history

store/
    database
    migrations
    repositories
    serialization
```

---

# 327. 最终命名建议

优先：

```text
Snapshot
SnapshotCapture
SnapshotStore
SnapshotComparator
SnapshotDiff
Timeline
TimelineEvent
EvolutionSummary
ProjectIdentity
ProjectRegistry
```

不要：

```text
HistoryManagerManager
SnapshotDataManager
MegaProjectMemoryService
```

---

# 328. 完成后进行代码清理

删除：

```text
temporary database
temporary export
debug snapshot
manual fixture output
```

确认：

```text
.gitignore
```

正确。

---

# 329. 最终 Git 状态

确认：

```text
working tree
```

只包含本阶段必要代码、测试、文档。

---

# 330. 最终 Commit

完成全部：

```text
build
test
CLI
GUI
privacy
migration
real project
git safety
```

之后再提交。

Commit message：

```text
feat: add snapshot and timeline engine
```

---

# 331. 不要 Push

M11：

> **只 commit，不 push。**

除非后续明确要求。

---

# 332. 最终汇报格式

完成后只输出真实结果：

```text
# M11 Final Report

## 1. Baseline
...

## 2. Implemented
...

## 3. Snapshot
...

## 4. Storage
...

## 5. Comparison
...

## 6. Timeline
...

## 7. Privacy
...

## 8. CLI
...

## 9. GUI
...

## 10. Tests
...

## 11. Real Project Validation
...

## 12. Git
...

## 13. Remaining P0/P1
...

## 14. Commit
...
```

---

# 333. Final Success Definition

只有同时满足：

```text
Snapshot can be created
Snapshot persists across restart
Snapshots are immutable
Duplicate state is deduplicated
Snapshots can be compared
Findings have historical lifecycle
Verification has historical state
Environment has historical state
Timeline is deterministic
Git remains read-only
Project files remain untouched
Secrets are not persisted
CLI works
IPC works
Tests pass
Real project works
```

才算：

> **M11 COMPLETE**

---

# 334. M11 的最终定位

M0–M10：

> Prism 知道项目现在是什么。

M11：

> **Prism 开始记得项目曾经是什么。**

最终形成完整闭环：

```text
现在
 ↓
Snapshot
 ↓
过去
 ↓
Compare
 ↓
变化
 ↓
Timeline
 ↓
演化
```

并且保持：

```text
Local
Deterministic
Evidence-based
Privacy-first
No AI dependency
No source backup
No Git mutation
No cloud
No scoring
No overengineering
```

---

# 335. 执行要求

现在不要直接进入实现。

第一步：

> **先对当前仓库执行 M11 Baseline Audit。**

必须以当前真实代码为事实来源。

先找出：

```text
哪些 M11 基础设施已经存在
哪些模型可以复用
ProjectId 当前是否稳定
M0-M10 的结果如何获取
缓存机制如何工作
当前 CLI/IPC 如何组织
是否已经存在 storage abstraction
SQLite 应该插在哪里
```

然后再按：

```text
P0 → P1
```

实施。

不要因为本提示词中的建议结构与当前代码不同，就机械重构现有工程。

**目标不是把代码变成这份提示词的样子。**

目标是：

> **在当前 Prism 架构上，以最小改动实现一个真正可靠的 Project Snapshot / Timeline / Evolution Engine。**