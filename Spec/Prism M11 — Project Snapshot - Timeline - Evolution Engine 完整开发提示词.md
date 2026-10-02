你现在负责实施 **Prism M11：Project Snapshot / Timeline / Evolution Engine**。

这是 Prism M0–M13 路线中的第 11 阶段。

---

# 0. M11 的核心使命

M0–M10 已经让 Prism 能够：

> 看懂项目 → 看懂代码 → 看懂结构 → 看懂 Git → 看懂变化 → 分析影响 → 发现风险 → 实际验证 → 理解环境 → 为当前任务生成上下文

M11 要解决的是：

> **项目过去发生了什么？**
>
> **现在和过去有什么不同？**
>
> **哪些问题出现过、解决过、反复出现？**
>
> **架构、代码、依赖、验证、环境、文档和风险是如何演变的？**

因此：

> **M11 = Prism 的项目长期记忆层。**

最终形成：

```text
M0  Foundation
 ↓
M1  Project Discovery
 ↓
M2  Code Understanding
 ↓
M3  Dependency & Architecture
 ↓
M4  Git
 ↓
M5  ChangeSet
 ↓
M6  Change Impact
 ↓
M7  Health
 ↓
M8  Verify
 ↓
M9  Environment
 ↓
M10 Context
 ↓
M11 Snapshot / Timeline / Evolution
 ↓
M12 Product UI
 ↓
M13 Release Hardening
```

M11 完成后，Prism 不再只是回答：

> “这个项目现在是什么样？”

而能够回答：

> “这个项目是怎么变成现在这样的？”

---

# 1. 最重要的产品定义

M11 不是：

- Git 替代品
- 代码备份工具
- 文件版本管理器
- 云端项目历史
- 自动提交系统
- 自动恢复系统
- 数据仓库
- 项目管理系统

M11 是：

> **对 Prism 已经分析出来的工程事实进行持久化，并让这些事实能够跨时间进行比较、追踪和回顾。**

核心对象：

```text
Project
    ↓
ProjectSnapshot
    ↓
Snapshot Comparison
    ↓
Timeline
    ↓
Evolution
```

---

# 2. M11 核心问题

M11 必须围绕以下问题设计：

```text
项目过去是什么样？
现在是什么样？
什么时候发生变化？
发生了什么变化？
哪些变化是结构性的？
哪些风险出现了？
哪些风险消失了？
哪些验证曾经失败？
哪些环境发生了变化？
架构如何演进？
当前状态相对于某个历史状态发生了什么变化？
```

不要加入：

```text
“项目健康度 82 分”
“项目比上个月好了 23%”
“架构质量提升 15%”
```

M11：

> **禁止把复杂工程状态压成单一评分。**

---

# 3. 核心设计原则

必须严格遵守以下原则。

## 3.1 Snapshot 是“工程事实快照”，不是代码备份

ProjectSnapshot 记录：

- 项目结构事实
- 代码结构事实
- 符号事实
- 模块事实
- 依赖事实
- 架构事实
- Git 状态
- ChangeSet
- ImpactAnalysis 摘要
- Health Findings
- Verification 状态
- Environment 摘要
- Context Health / Context Manifest 摘要
- 各域 fingerprint
- freshness 信息
- provenance 信息

但是：

> 默认不保存整个源码树。

更具体地说：

```text
保存：
    file path
    file hash
    symbol metadata
    module metadata
    dependency metadata
    finding metadata
    verification metadata
    environment metadata
    Git metadata
    structural fingerprints

不默认保存：
    完整源码
    完整二进制
    完整构建产物
    private key
    secret
    token
    password
    raw environment values
    完整命令行中的敏感参数
```

Snapshot 的职责是：

> **记录项目状态。**

而不是：

> **复制项目。**

---

# 4. Snapshot 与 Git 的边界

必须明确：

```text
Git
=
源代码历史

Prism Snapshot
=
工程理解历史
```

Git 已经知道：

```text
某个 commit 修改了哪些文件
```

Prism Snapshot 要知道：

```text
某个时间点：

项目有什么模块
有哪些依赖
有哪些架构关系
有哪些问题
哪些验证通过
哪些验证失败
当前环境是什么
文档是否出现漂移
当前 Change Impact 是什么
```

两者互补。

---

# 5. Snapshot 必须支持两种主要状态

## 5.1 Git Revision Snapshot

当项目：

```text
working tree clean
```

并且：

```text
HEAD = 某个 commit
```

可以记录：

```text
SnapshotBasis = GitRevision
```

例如：

```text
HEAD = abc123
clean = true
```

---

## 5.2 Working Tree Snapshot

如果存在：

```text
unstaged changes
staged changes
untracked files
```

则：

```text
SnapshotBasis = WorkingTree
```

记录：

```text
HEAD
index state fingerprint
working tree fingerprint
ChangeSet fingerprint
```

必须明确提示：

> WorkingTree Snapshot 可以记录当前工程事实，但不能保证未来能够重建当时未提交的源码内容。

---

# 6. Snapshot 必须区分不同状态维度

不要制造一个巨大的：

```text
ProjectFingerprint
```

然后所有东西都混进去。

至少拆成：

```text
ProjectStructureFingerprint
CodeStructureFingerprint
ArchitectureFingerprint
GitStateFingerprint
ChangeFingerprint
ImpactFingerprint
HealthFingerprint
VerificationFingerprint
EnvironmentFingerprint
ContextHealthFingerprint
```

然后：

```text
ProjectStateFingerprint
```

可以作为这些维度的组合 fingerprint。

但所有比较逻辑必须支持：

```text
只比较 architecture
只比较 health
只比较 verification
只比较 environment
只比较 code structure
```

---

# 7. ProjectSnapshot Model

建立明确的领域模型。

建议至少包含：

```text
ProjectSnapshot

id
project_id
schema_version

captured_at
duration_ms

basis
    GitRevision
    WorkingTree
    Unknown

git_revision
git_branch

working_tree_state

project_name
project_root

state_fingerprint

structure_fingerprint
code_fingerprint
architecture_fingerprint
git_fingerprint
change_fingerprint
impact_fingerprint
health_fingerprint
verification_fingerprint
environment_fingerprint
context_health_fingerprint

source_versions
freshness

capture_metadata

notes
```

其中：

```text
project_id
```

必须是稳定的项目身份。

不能每次打开项目时随机生成一个全新的 Project ID。

---

# 8. Project Identity

M11 必须正式解决：

> “这个项目到底是不是过去那个项目？”

如果当前 M0 的 Project ID 是临时运行时 ID，则必须补上持久化的：

```text
ProjectIdentity
```

原则：

```text
first open
    ↓
创建 stable ProjectId
    ↓
持久化
    ↓
以后再次打开
    ↓
恢复同一个 ProjectId
```

默认可以以：

```text
canonical project root
```

作为本地识别依据。

允许保存：

```text
display name
canonical root
last_seen
project_id
```

但不要把：

```text
Git remote URL
```

作为项目身份的唯一依据。

原因：

- remote 可以改变
- remote 可能包含私有信息
- offline 项目可能没有 remote
- 用户可能使用多个 remote

---

# 9. ProjectRegistry

引入一个轻量级：

```text
ProjectRegistry
```

负责：

```text
project_id
project_root
display_name
created_at
last_opened_at
persistence_enabled
```

职责只有：

> 管理 Prism 已经认识过的本地项目。

不要让 ProjectRegistry 变成项目管理系统。

---

# 10. Snapshot Capture

核心入口：

```text
SnapshotCaptureService
```

负责：

```text
collect
normalize
fingerprint
persist
```

原则：

> M11 不重复实现 M1–M10。

优先：

```text
reuse existing analyzers
reuse caches
reuse normalized models
reuse Git engine
reuse Health engine
reuse Verify engine
reuse Environment engine
reuse Context engine
```

M11 只是：

> **把现有工程事实冻结成一个历史状态。**

---

# 11. Capture Pipeline

推荐流程：

```text
Capture Request
    ↓
Resolve Project Identity
    ↓
Resolve current project state
    ↓
Determine freshness
    ↓
Reuse valid cached analysis
    ↓
Refresh stale domains when requested
    ↓
Normalize all domains
    ↓
Redact sensitive data
    ↓
Calculate domain fingerprints
    ↓
Build Snapshot Manifest
    ↓
Persist Snapshot
    ↓
Compare with previous snapshot
    ↓
Build timeline/evolution information
    ↓
Return Snapshot Result
```

---

# 12. Snapshot Capture 必须考虑 freshness

不能简单地认为：

```text
“之前运行过 M8，所以 snapshot 中的 verification 永远有效。”
```

错误。

必须保留类似：

```text
captured_at
source_revision
input_fingerprint
status
freshness
```

例如：

```text
Verification:
    status = Passed
    executed_at = ...
    source_revision = abc123
    current_revision = def456
    freshness = Stale
```

这种情况必须明确显示：

> 上次验证通过，但验证对象已经发生变化。

不能写成：

> Verified

---

# 13. Snapshot Domain State

建议每个域都有：

```text
SnapshotDomainState

available
captured_at
source_revision
input_fingerprint
content_fingerprint
freshness
status
limitations
```

状态至少考虑：

```text
Available
Unavailable
Stale
Partial
NotApplicable
```

不要把：

```text
Unavailable
```

当成：

```text
Failed
```

不要把：

```text
Stale
```

当成：

```text
Failed
```

保持 M9/M10 一致的 Unknown 语义。

---

# 14. Snapshot 内容应该是“可比较的”

Snapshot 最核心的价值不是保存，而是：

> **比较。**

因此所有持久化数据必须尽量：

```text
stable
normalized
deterministic
versioned
```

例如：

```text
文件列表排序稳定
symbol 排序稳定
dependency 排序稳定
finding 排序稳定
check 排序稳定
environment tools 排序稳定
```

避免因为：

```text
HashMap iteration order
异步执行顺序
UI 顺序
```

而导致两个相同状态产生不同 fingerprint。

---

# 15. Canonical Serialization

实现统一：

```text
CanonicalSnapshotSerializer
```

要求：

```text
field ordering deterministic
collection ordering deterministic
null handling deterministic
path normalization deterministic
enum serialization deterministic
float handling deterministic
redaction deterministic
```

然后：

```text
canonical representation
    ↓
hash
    ↓
fingerprint
```

不要直接对：

```text
随机 JSON
```

做 hash。

---

# 16. Fingerprint 设计

至少实现：

```text
domain fingerprint
```

例如：

```text
architecture_fingerprint
health_fingerprint
verification_fingerprint
environment_fingerprint
```

以及：

```text
overall state fingerprint
```

用途：

```text
deduplication
change detection
snapshot comparison
timeline generation
```

---

# 17. 重复 Snapshot 处理

用户连续执行：

```text
prism snapshot .
```

如果项目状态完全没变化：

不要无意义产生大量完全相同的 snapshot。

默认行为：

```text
state unchanged
    ↓
do not create duplicate state snapshot
    ↓
return existing snapshot
```

输出类似：

```text
Snapshot unchanged.
Existing snapshot: <id>
```

但必须提供：

```text
--force
```

允许用户明确创建重复的 observation。

---

# 18. Snapshot 与 Observation 的区分

可以明确区分：

```text
Snapshot
=
唯一的工程状态

Observation
=
某一次捕获行为
```

M11 第一版不必把 Observation 做成非常复杂的独立系统。

但领域模型中必须避免：

> “每一次 capture 都一定产生一个新的 ProjectSnapshot。”

可以：

```text
ProjectSnapshot
    state_fingerprint = X

CaptureResult
    created = false
    existing_snapshot_id = ...
```

未来再扩展 Observation。

---

# 19. Snapshot Storage

M11 是第一个真正需要持久化数据库的阶段。

因此：

> 从 M11 开始，允许引入本地 SQLite。

推荐：

```text
SQLite
```

不要：

```text
MongoDB
PostgreSQL
embedded server
cloud database
```

---

# 20. SQLite 设计原则

数据库应该：

```text
local-only
embedded
migration-based
transactional
testable
small
```

不要使用复杂 ORM，除非当前工程已经确定采用某个 ORM。

优先考虑：

```text
rusqlite
```

或者项目已有数据库抽象时复用已有方案。

不要为了 M11：

> 再引入一个大型 data access framework。

---

# 21. 数据库存储位置

默认：

> **不要把 Prism 数据库放到项目仓库里。**

不要默认创建：

```text
<project>/.prism/prism.db
```

因为这会：

- 污染项目目录
- 可能被 Git 提交
- 修改项目工作树
- 给用户造成“Prism 修改我的项目”的感觉

优先使用：

```text
OS user application-data directory
```

由 Tauri / Rust 平台 API 获取。

例如概念上：

```text
Windows:
    AppData / Prism

macOS:
    Application Support / Prism

Linux:
    XDG data directory / Prism
```

具体路径必须通过现有平台抽象获取。

不要硬编码。

---

# 22. CLI 与 GUI 必须使用同一个 Store

不能出现：

```text
GUI SQLite A
CLI SQLite B
```

必须：

```text
Prism Core
    ↓
SnapshotStore
    ↓
SQLite
```

然后：

```text
Tauri
    ↓
Application
    ↓
SnapshotStore

CLI
    ↓
Application
    ↓
SnapshotStore
```

这样：

```text
CLI capture
```

之后：

```text
GUI history
```

必须立即能够看到。

---

# 23. 推荐 Storage 模块

如果当前架构允许，增加：

```text
prism-store
```

职责：

```text
database connection
schema
migrations
repositories
transactions
serialization
```

不要让：

```text
prism-health
prism-git
prism-context
```

直接写 SQLite。

正确方向：

```text
Health
    ↓
Domain model

Snapshot Application Layer
    ↓
Snapshot Model
    ↓
SnapshotStore
    ↓
SQLite
```

---

# 24. 推荐数据库表

不要为了“完整”创造几十张表。

第一版至少需要：

```text
projects
snapshots
snapshot_domains
```

可以进一步加入：

```text
snapshot_findings
snapshot_checks
snapshot_environment
```

但是否需要拆表，必须根据当前已有领域模型和查询需求决定。

允许大量结构化内容以：

```text
JSON
```

保存。

但是：

> 趋势分析真正需要频繁查询的字段应该结构化。

---

# 25. 推荐最小 Schema

## projects

```text
id
display_name
canonical_root
created_at
updated_at
last_seen_at
persistence_enabled
schema_version
```

---

## snapshots

```text
id
project_id

captured_at
created_at

basis
git_revision
git_branch
working_tree_state

state_fingerprint

structure_fingerprint
code_fingerprint
architecture_fingerprint
git_fingerprint
change_fingerprint
impact_fingerprint
health_fingerprint
verification_fingerprint
environment_fingerprint
context_health_fingerprint

manifest_json
limitations_json

note
schema_version
```

---

## snapshot_domains

```text
snapshot_id
domain
status
captured_at
source_revision
input_fingerprint
content_fingerprint
freshness
data_json
```

这里的重点是：

> domain 可以独立判断 freshness 和 availability。

---

# 26. 不要复制整个 M1–M10 数据库

不要做成：

```text
snapshot_files
snapshot_symbols
snapshot_modules
snapshot_dependencies
snapshot_everything...
```

然后每个 snapshot 完整复制所有数据。

这会迅速膨胀。

优先：

```text
snapshot manifest
+
normalized domain JSON
+
critical query fields
+
fingerprints
```

必要时再做结构化索引。

---

# 27. SnapshotManifest

创建：

```text
SnapshotManifest
```

作为 Snapshot 的核心摘要。

包含：

```text
snapshot_id
project_id
schema_version
captured_at
basis

source_revisions

domain_states

fingerprints

counts

limitations
```

例如：

```text
files = 842
symbols = 3912
modules = 53
dependencies = 417
findings = 17
checks = 9
```

注意：

> counts 是事实统计，不是评分。

---

# 28. Snapshot 不能保存 secrets

在写数据库之前必须经过：

```text
SnapshotRedactor
```

至少检查：

```text
API keys
tokens
passwords
private keys
connection strings
authorization headers
secret env vars
credential-like values
```

并遵循 M9/M10 已建立的 redaction 机制。

不要复制一套完全不同的 secret detector。

---

# 29. Environment Snapshot 持久化边界

M9 当前环境可以保存：

```text
OS
architecture
runtime versions
SDK versions
package manager versions
tool availability
requirements matching
selected port state
selected process facts
```

但不要保存：

```text
raw env values
full command lines with secrets
private key content
credential stores
```

对于 process：

> 优先保存稳定身份与状态，不保存不必要的完整命令行参数。

---

# 30. Source Code 持久化边界

M11 默认：

```text
不保存完整源码
```

也不应该为了比较历史 snapshot 而自动复制源码。

对于 Git commit：

```text
snapshot
    ↓
record commit SHA
```

未来需要历史源码时：

```text
Git
    ↓
git show <revision>
```

对于 dirty working tree：

只能保存：

```text
hash
structure
symbols
metadata
analysis facts
```

因此文档中必须明确：

> Prism Snapshot 不是工作区备份。

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

# 63. Retention

M11 可以定义：

```text
RetentionPolicy
```

但：

> 默认不要自动删除用户历史。

第一版：

```text
manual cleanup
```

即可。

自动 retention 可以后续扩展。

---

# 64. 数据库 Migration

这是 M11 的 P0。

必须建立：

```text
schema version
migration mechanism
```

例如：

```text
migration 001
migration 002
...
```

启动时：

```text
read current schema
↓
apply pending migrations
↓
verify schema
```

失败时：

```text
明确错误
安全停止
不破坏已有历史
```

---

# 65. Transaction Safety

Snapshot 写入必须使用事务。

建议：

```text
BEGIN
    project update
    snapshot insert
    domain data insert
COMMIT
```

任何一步失败：

```text
ROLLBACK
```

禁止出现：

```text
snapshot header exists
domain data missing
```

这样的半成品 snapshot。

---

# 66. Atomic Snapshot

Snapshot 必须具有：

> 原子性。

数据库中不能出现：

```text
Snapshot exists = true
但 Health / Architecture / Verification 数据不存在
```

除非这些域本身状态就是：

```text
Unavailable
```

这两种情况必须严格区分：

```text
capture failed
```

与：

```text
domain unavailable
```

---

# 67. Snapshot State

建议：

```text
Preparing
Ready
Partial
Failed
```

正常数据库中：

```text
Preparing
```

不应该长期存在。

应用崩溃时，下次启动必须能检测：

```text
incomplete snapshot
```

并清理或标记 abandoned。

---

# 68. Schema Version

必须至少存在：

```text
snapshot_schema_version
```

因为未来：

```text
M12
M13
Prism 2.x
```

都可能改变模型。

禁止假设：

> 当前 JSON 永远兼容。

---

# 69. Snapshot Data Versioning

由于 M1–M10 模型仍可能演化：

Snapshot 中必须知道：

```text
Project model version
Parser model version
Architecture model version
Health ruleset version
Verify model version
Environment model version
Context model version
```

否则未来无法判断：

```text
旧 Snapshot 和新 Snapshot
到底是不是基于同一种知识模型。
```

---

# 70. Ruleset Version

特别是 M7 Health。

如果：

```text
2026-09-01:
Health Rule Set v1

2026-10-01:
Health Rule Set v2
```

同一个 Finding 的变化可能只是：

```text
规则改变
```

而不一定是：

```text
项目改变
```

因此 Snapshot 必须保存：

```text
health_ruleset_version
```

类似地可以保存：

```text
verification recipe version
parser version
```

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

# 81. Persistence Policy

增加：

```text
SnapshotPersistencePolicy
```

最少支持：

```text
Enabled
Disabled
```

后续可以扩展：

```text
StoreEnvironment = false
StoreContextMetadata = false
```

但第一版不要做复杂隐私配置矩阵。

---

# 82. 项目级 Disable

用户可以对项目关闭历史记录。

例如：

```text
Persistence disabled
```

这意味着：

```text
M11 analysis still works
```

但是：

```text
no snapshot persistence
```

这是非常重要的：

> Persistence 是能力，不应该成为 Prism 正常分析的前置条件。

---

# 83. Offline First

M11：

```text
100% local
```

不能依赖：

```text
internet
cloud
remote API
AI provider
```

---

# 84. AI Boundary

M11 禁止引入：

```text
LLM
OpenAI
Anthropic
embeddings
vector database
RAG
MCP
AI summary service
```

M11 的历史比较必须：

> **Deterministic First。**

未来 AI 可以在：

```text
Timeline
    ↓
AI summarization
```

之上做增强。

但不是 M11 基础设施。

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

# 91. Snapshot Note

用户可以创建：

```text
Snapshot note
```

例如：

```text
Before M11 refactor
Before release
Stable baseline
After architecture migration
```

Note 是：

> 用户自己的标签。

不要自动生成带主观意义的标签。

---

# 92. Baseline Snapshot

支持：

```text
baseline snapshot
```

让用户可以比较：

```text
Current
vs
Baseline
```

第一版可以只是：

```text
selected snapshot
```

不要设计复杂 baseline 管理系统。

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

# 103. 正确依赖方向

推荐：

```text
M1-M10
   ↓
Application / Snapshot orchestration
   ↓
Snapshot domain model
   ↓
SnapshotStore
   ↓
SQLite
```

而不是：

```text
M1
 ↓
Database
```

---

# 104. 建议架构

在现有架构基础上：

```text
Frontend
   ↓
Tauri IPC
   ↓
Application
   ↓
Snapshot Service
   ↓
Snapshot Comparator
   ↓
Snapshot Timeline Builder
   ↓
Snapshot Store
   ↓
SQLite
```

Domain analyzers 保持：

```text
独立
无数据库依赖
```

---

# 105. Snapshot Service API

至少设计：

```text
create_snapshot(...)
get_snapshot(...)
list_snapshots(...)
get_latest_snapshot(...)
compare_snapshots(...)
get_timeline(...)
get_finding_history(...)
get_verification_history(...)
get_environment_history(...)
delete_snapshot(...)
clear_history(...)
export_history(...)
```

---

# 106. Cancellation

Snapshot capture 可能比较耗时。

必须复用现有：

```text
CancellationToken
Task
Progress
```

尤其：

```text
M1 scan
M2 parse
M3 graph
M7 health
M8 verification
M9 environment
```

如果需要 refresh。

---

# 107. Progress

例如：

```text
Preparing snapshot
25%

Collecting project facts
40%

Collecting architecture
55%

Collecting health
70%

Collecting verification
80%

Persisting snapshot
95%

Completed
100%
```

如果实际使用已有 task framework，则复用，不要创建第二套。

---

# 108. Snapshot Performance

M11 默认：

> 优先复用缓存，而不是重新扫描整个项目。

目标：

```text
unchanged project
+
valid cached facts
=
fast snapshot
```

如果每次：

```text
prism snapshot
```

都完整执行：

```text
M1 → M2 → M3 → M7 → M8
```

这是不可接受的默认设计。

---

# 109. Verification 默认行为

M11：

> 默认不要因为创建 Snapshot 就自动跑完整 Verification。

否则：

```text
snapshot
```

可能意外触发：

```text
build
test
network-related CI simulation
```

正确：

```text
reuse latest verification
```

并标记：

```text
fresh/stale
```

用户明确：

```text
--refresh
```

才允许刷新相关域。

---

# 110. Environment 默认行为

M9 是低成本观察。

M11 可以默认重新获取：

```text
environment fingerprint
```

但应该：

```text
fast
local
non-invasive
```

不得做：

```text
network scanning
process killing
package installation
```

---

# 111. Snapshot Capture Options

统一定义：

```text
SnapshotCaptureOptions
```

例如：

```text
refresh_analysis
refresh_verification
refresh_environment
include_context_metadata
include_git
force
note
```

命名必须适配当前代码。

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

# 114. 不修改用户工作树

这是 M11 的 P0。

Snapshot / history / compare：

> **绝对不能改变用户项目的 Git 状态。**

禁止：

```text
checkout
reset
restore
clean
stash
commit
rebase
merge
```

即使为了比较历史。

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

# 116. Snapshot Delete 不等于项目修改

允许：

```text
delete Prism history
```

但只能：

```text
SQLite
```

不能：

```text
项目文件
Git
```

---

# 117. Project Rename / Move

第一版不需要自动智能迁移。

如果：

```text
project moved
```

发现：

```text
canonical root changed
```

应明确：

```text
new project identity
```

或者提供未来可扩展的：

```text
Re-associate History
```

但 M11 不要做复杂自动猜测。

---

# 118. 不要使用 Git Remote 做自动迁移

不要通过：

```text
origin URL
```

自动把：

```text
Project A
```

认成：

```text
Project B
```

避免：

- 私有仓库 URL 泄漏
- remote 改变
- fork
- clone
- multiple remote

---

# 119. Database Corruption Handling

必须考虑：

```text
database unavailable
database corrupted
migration failed
disk full
permission denied
locked
```

应用应该：

```text
show actionable error
```

而不是：

```text
panic
```

---

# 120. Read-only fallback

如果 Snapshot DB 写入失败：

M1–M10 的：

```text
project analysis
```

仍然可以工作。

即：

```text
Persistence failure
≠ Prism unavailable
```

可以返回：

```text
analysis succeeded
snapshot persistence failed
```

---

# 121. Persistence Failure Model

建议：

```text
SnapshotPersistResult

Success
Duplicate
Disabled
StorageUnavailable
MigrationFailed
SerializationFailed
PermissionDenied
CorruptStore
```

具体枚举根据当前错误体系设计。

---

# 122. 不要吞错误

例如：

```text
SQLite failed
```

不能：

```text
log only
```

同时 CLI 输出：

```text
Snapshot created
```

这是错误的。

必须区分：

```text
analysis result
persistence result
```

---

# 123. Snapshot Capture Result

建议：

```text
SnapshotCaptureResult

status
snapshot_id?
created
deduplicated
duration
domain_results
warnings
limitations
storage_status
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

# 137. Snapshot Tags

可选：

```text
tags
```

例如：

```text
release
baseline
milestone
before-refactor
after-refactor
```

但是：

> 第一版只支持用户手工标签/备注。

不要引入完整项目 milestone 管理。

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

# 141. Snapshot Size

实现：

```text
snapshot size estimation
```

至少能够知道：

```text
manifest size
domain data size
database growth
```

用于测试和后续优化。

但不要现在做：

```text
compression framework
```

除非真实数据证明需要。

---

# 142. SQLite Index

根据真实查询增加必要 index，例如：

```text
project_id
captured_at
state_fingerprint
git_revision
```

不要提前创建几十个索引。

---

# 143. Database Cleanup

实现必要的：

```text
VACUUM
```

或清理策略前，先验证是否真的需要。

不要因为“数据库专业”就乱加后台维护任务。

---

# 144. Concurrency

考虑：

```text
GUI 同时请求 snapshot
CLI 同时请求 snapshot
```

必须避免数据库损坏。

SQLite 可以负责：

```text
locking
transaction
```

应用层需要避免：

```text
同时 capture 同一 project
```

产生大量重复工作。

第一版可以：

```text
per-project capture lock
```

---

# 145. Snapshot Capture Lock

例如：

```text
Project A:
capture running
```

再次请求：

```text
return Busy
```

或者：

```text
coalesce
```

但不要启动：

```text
两个完全相同的扫描
```

---

# 146. No Background Auto-Snapshot

M11 第一版：

> **禁止默认后台自动创建 Snapshot。**

不要：

```text
每 5 分钟自动 snapshot
每次打开自动 snapshot
每次 Git commit 自动 snapshot
```

原因：

- 数据库快速膨胀
- 用户无法理解历史来源
- 性能不可控
- 历史噪声过大

未来再设计自动策略。

---

# 147. Manual Snapshot First

核心体验：

```text
Analyze
    ↓
Snapshot
    ↓
Compare
```

这是 M11 第一阶段最可靠的模型。

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

# 149. M11 API 不应该依赖 React

核心模型必须在：

```text
Rust core
```

中定义。

禁止：

```text
React state
```

成为历史数据模型。

---

# 150. Serialization Contract

确保：

```text
Rust Snapshot
    ↔
Tauri IPC
    ↔
TypeScript Snapshot
```

类型一致。

尽量生成/集中定义 schema，而不是：

```text
Rust 一份
TS 手写一份
```

最后漂移。

---

# 151. Snapshot IDs

使用稳定、不可变：

```text
SnapshotId
```

不要：

```text
array index
timestamp
```

作为 ID。

推荐：

```text
UUID/ULID
```

具体复用项目现有 ID 体系。

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

# 157. Schema Migration Tests

必须测试：

```text
empty DB
old DB
latest DB
migration
downgrade detection
corrupt DB
```

至少确保：

> 新版本可以可靠读取旧版本历史。

---

# 158. Snapshot Serialization Tests

测试：

```text
same logical state
→ same canonical JSON
→ same fingerprint
```

并测试：

```text
reordered collections
→ same fingerprint
```

---

# 159. Dedup Tests

测试：

```text
snapshot A
snapshot A again
```

结果：

```text
created once
second capture deduplicated
```

然后：

```text
--force
```

可以创建新 observation/state record，具体实现遵循前述 Snapshot/Observation 边界。

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

# 164. Working Tree Tests

Fixture：

```text
clean repo
dirty repo
staged repo
unstaged repo
untracked file
```

Snapshot 必须正确记录。

---

# 165. Git Rename Tests

利用 M4 已有：

```text
rename
```

验证 Snapshot Diff 不把：

```text
rename
```

简单判断成：

```text
delete + add
```

如果 Git 无法确定：

```text
Unknown
```

优于错误推断。

---

# 166. Cross-platform Tests

至少验证：

```text
Windows
Linux
macOS
```

路径：

```text
case
separator
drive
UNC
symlink
```

不要在 Snapshot 中保存无法跨平台解释的内部绝对路径作为唯一 identity。

---

# 167. Symlink

遵循 M1 的既有策略。

Snapshot 记录：

```text
resolved/ignored/link state
```

不要重新定义 symlink semantics。

---

# 168. Permission

如果某目录：

```text
Permission denied
```

Snapshot 应记录：

```text
partial / limitation
```

而不是：

```text
project has no files
```

---

# 169. Partial Snapshot

如果：

```text
M1 = partial
M3 = available
M7 = available
M8 = unavailable
```

Snapshot 仍然可以存在。

但：

```text
domain states
```

必须清楚。

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

# 171. Snapshot Limitations

Snapshot 必须携带：

```text
limitations[]
```

例如：

```text
Verification reused from older revision
Environment unavailable
Some files unreadable
Context pack not persisted
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

# 176. Dirty Snapshot Source

对于 Working Tree Snapshot：

如果后来源码已经变化：

不能假装还能打开当时完整源码。

应该显示：

```text
Historical source content not persisted.
Structural facts are available.
```

这是正确的诚实行为。

---

# 177. Snapshot Purpose

M11 要让用户理解：

> Snapshot 是“记忆”，不是“备份”。

UI 中可以明确显示：

```text
Engineering State Snapshot
```

不要叫：

```text
Backup
```

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

# 183. Logging

记录：

```text
snapshot capture started
snapshot capture completed
snapshot deduplicated
storage error
migration
comparison
```

不要记录：

```text
secret values
raw source
private env values
```

---

# 184. Observability

本地日志可以包含：

```text
project id
snapshot id
duration
domain states
counts
```

但不要：

```text
full paths everywhere
```

如果路径属于敏感信息，遵循现有日志 redaction。

---

# 185. Error Taxonomy

复用现有：

```text
PrismError
```

新增：

```text
SnapshotError
StorageError
MigrationError
ComparisonError
```

具体分类遵循当前架构。

不要创造第二套完全独立错误系统。

---

# 186. Domain Layer

建议：

```text
prism-core
    Snapshot primitives

prism-project
    Project identity

prism-store
    Persistence

Application
    Capture orchestration

prism-cli
    CLI commands
```

实际 crate 划分以当前 repo 为准。

---

# 187. 不要大规模重构

M11 不是：

```text
architecture rewrite
```

必须：

```text
reuse M0-M10
minimum invasive changes
```

如果当前架构已经存在合理的：

```text
storage abstraction
repository pattern
task system
```

优先复用。

---

# 188. 实施前必须先审计

收到本提示词后：

> **先审计，后修改。**

第一步检查：

```text
M0-M10 当前真实代码
```

重点确认：

```text
Project model
Project ID
cache
M1 discovery result
M2 code result
M3 architecture result
M4 Git result
M5 ChangeSet
M6 ImpactAnalysis
M7 Health
M8 Verify
M9 Environment
M10 Context
Application layer
IPC
CLI
serialization
error system
task/cancellation
```

---

# 189. 不要相信旧文档

必须：

```text
以当前仓库真实代码为准。
```

旧设计文档仅作为：

```text
intent
```

不能当作：

```text
FACT
```

---

# 190. 审计输出

修改前先输出：

```text
# M11 Baseline Audit

## A. FACT
...

## B. HYPOTHESIS
...

## C. INFERENCE
...

## D. Current Architecture
...

## E. Existing Reusable Infrastructure
...

## F. Gaps
...

## G. P0
...

## H. P1
...
```

---

# 191. 不要擅自修 unrelated issues

审计发现：

```text
unrelated bug
```

不要顺手修。

只修：

```text
M11 necessary
```

的问题。

---

# 192. P0 范围

优先确保：

```text
P0-1 Snapshot domain model
P0-2 Stable Project Identity
P0-3 SQLite persistence
P0-4 Migration
P0-5 Atomic capture
P0-6 Fingerprints
P0-7 Deduplication
P0-8 Snapshot comparison
P0-9 History query
P0-10 Timeline
P0-11 Privacy/redaction
P0-12 Git read-only safety
```

---

# 193. P1 范围

之后：

```text
P1-1 GUI History
P1-2 GUI Compare
P1-3 Timeline filtering
P1-4 History export
P1-5 Finding history UX
P1-6 Verification history UX
P1-7 Environment evolution UX
P1-8 Context health evolution
```

如果时间有限：

> 后端正确性优先于 UI polish。

---

# 194. P2 不要现在做

禁止扩张到：

```text
automatic snapshots
cloud sync
AI historical summaries
semantic embeddings
LLM timeline analysis
collaboration
remote history
source backup
full project restore
Git mutation
```

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

# 196. Restart Persistence Test

必须做：

```text
process A
create snapshot
exit

process B
open same project
list history
```

验证：

> Snapshot 仍存在。

不能只测试：

```text
same process memory
```

---

# 197. Multi-project Test

至少：

```text
Project A
Project B
```

分别创建 Snapshot。

验证：

```text
A history != B history
```

不会串项目。

---

# 198. Same-name Project Test

测试：

```text
C:\foo\app
D:\bar\app
```

虽然：

```text
display_name = app
```

但：

```text
project_id
```

不能冲突。

---

# 199. History Isolation

测试：

```text
delete A history
```

不能影响：

```text
B
```

---

# 200. Database Isolation

测试：

```text
GUI
CLI
```

访问同一个 project history。

验证：

```text
same SQLite state
```

---

# 201. Concurrent Access

至少：

```text
CLI history
GUI history
```

同时读取。

以及：

```text
one capture
one read
```

不能损坏数据库。

---

# 202. Crash Safety

尽可能模拟：

```text
process interrupted
```

验证：

```text
no corrupted completed snapshot
```

以及：

```text
database remains readable
```

---

# 203. Migration Test

建立 fixture：

```text
schema v1
```

升级：

```text
current
```

验证：

```text
history readable
```

---

# 204. Old Snapshot Compatibility

创建：

```text
old snapshot
```

然后升级 Prism。

要求：

```text
read old snapshot
compare when compatible
```

不兼容时：

```text
clear explanation
```

---

# 205. Fingerprint Regression Test

建立多个 fixture：

内容逻辑相同但：

```text
collection order different
```

结果必须：

```text
same fingerprint
```

---

# 206. Fingerprint Change Test

微小变化：

```text
one dependency added
```

必须：

```text
architecture fingerprint changed
```

而无关域不应该无故改变。

---

# 207. Domain Fingerprint Isolation

例如只改变：

```text
environment tool version
```

期望：

```text
environment fingerprint changed
```

而：

```text
architecture fingerprint
```

不应该随之变化。

---

# 208. Snapshot Diff Precision

测试：

```text
one file changed
```

确保不会错误报告：

```text
整个项目全部 changed
```

---

# 209. History Query Performance

测试：

```text
100+
500+
1000+
```

Snapshot 后：

```text
list
latest
compare
timeline
```

不得明显退化。

不需要为性能测试而生成百万 Snapshot。

---

# 210. Database Size Test

测量：

```text
100 snapshots
```

数据库大小。

确认没有：

```text
完整源码复制
```

导致异常膨胀。

---

# 211. Privacy Regression Test

导出：

```text
history.json
```

扫描：

```text
secret
token
password
private key
```

确保没有泄漏。

---

# 212. CLI Integration Test

真实执行：

```bash
prism snapshot fixture
prism history fixture
prism compare fixture A B
prism timeline fixture
```

不要只测试 service unit test。

---

# 213. GUI Integration Test

验证：

```text
IPC
    ↓
snapshot
    ↓
history
    ↓
compare
```

不能只 mock 后端。

---

# 214. Self-hosting Test

M11 完成后：

> 使用 Prism 自己的仓库作为真实测试对象。

至少执行：

```text
prism snapshot Prism
prism history Prism
prism compare Prism
prism timeline Prism
```

---

# 215. Real Project Test

另外选择至少一个真实项目：

```text
TypeScript
Python
C#
```

中的项目进行验证。

最好覆盖：

```text
Git
dirty working tree
health findings
verification
environment
```

---

# 216. Test Fixtures

建立：

```text
fixtures/snapshot/
```

建议：

```text
clean-project
changed-project
health-regression
health-resolution
environment-change
verification-change
dirty-working-tree
partial-analysis
```

fixture 尽量小而明确。

---

# 217. Tests 必须可重复

禁止依赖：

```text
真实用户电脑随机环境
网络
GitHub
当前天气
当前外部服务
```

对于 environment fixture：

使用：

```text
mock/adapters
```

而真实端到端项目测试则单独标记。

---

# 218. Clock Testing

需要测试：

```text
same timestamp
different timezone
```

避免 timeline 排序 flaky。

---

# 219. Path Normalization Tests

测试：

```text
Windows separator
Unix separator
relative
absolute
case
symlink
```

---

# 220. Database Path Tests

确保：

```text
project directory
```

不会出现：

```text
prism.db
```

等非用户预期文件。

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

# 222. File Mutation Test

Snapshot 操作前后比较：

```text
project file hashes
```

必须没有意外变化。

---

# 223. No Network Test

运行 M11：

> 不应发起网络请求。

可以通过测试环境：

```text
network disabled
```

验证。

---

# 224. No AI Test

M11 测试环境不提供任何：

```text
AI API key
```

仍然必须完整运行。

---

# 225. Context Test

确认：

```text
M10 Context
```

不因 M11 引入数据库而产生强制依赖。

---

# 226. Store Abstraction

测试代码尽量能够：

```text
SnapshotService
    ↓
InMemorySnapshotStore
```

运行 unit tests。

不要所有单测都必须启动 SQLite。

然后单独：

```text
SQLite integration tests
```

---

# 227. Repository Abstraction

例如：

```text
SnapshotStore
    create()
    get()
    list()
    compare()
```

上层不直接写：

```text
SQL strings everywhere
```

---

# 228. SQL 边界

SQL 只存在于：

```text
storage layer
```

不要：

```text
UI component
Application service
Health rule
```

直接访问 SQLite。

---

# 229. JSON Boundary

JSON 只是：

```text
storage / IPC / export
```

中的一种 representation。

不要让：

```text
核心领域模型 = JSON string
```

---

# 230. Snapshot Object Immutability

一个已经 Ready 的：

```text
ProjectSnapshot
```

原则上：

> 不修改。

若新状态：

```text
create new snapshot
```

不要：

```text
update snapshot in-place
```

这样历史才可信。

---

# 231. Snapshot Deletion

删除历史：

```text
delete snapshot
```

是显式管理操作。

删除以后：

```text
timeline
```

必须重新正确派生。

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

# 239. Branch Dimension

Snapshot 必须记录：

```text
branch
```

因为：

```text
feature/a
feature/b
main
```

上的历史可能完全不同。

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

# 241. Branch Rename

Git branch 可能改名。

不要把 branch name 作为 Snapshot identity。

它只是：

```text
captured metadata
```

---

# 242. Detached HEAD

必须支持：

```text
detached HEAD
```

Snapshot 可以：

```text
branch = null
git_revision = abc123
```

---

# 243. No Git Repo

项目不是 Git 仓库也必须支持：

```text
SnapshotBasis = WorkingTree / Unknown
```

M11 不应依赖 Git。

---

# 244. Gitless Project

Snapshot 仍然可以比较：

```text
file hash
symbol
module
architecture
health
environment
```

但是：

```text
Git domain = Unavailable
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

# 248. Baseline Semantics

第一 Snapshot 可以自动成为：

```text
first historical baseline
```

但不要自动标记成：

```text
best
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

# 252. Data Retention Warning

如果 Snapshot 很多：

UI 可以显示：

```text
History:
248 snapshots
Database:
18.4 MB
```

这是事实。

---

# 253. Database Health

可以在 Environment / Settings 显示：

```text
Storage status
database size
snapshot count
last write
```

但不要创建：

```text
Storage Health Score
```

---

# 254. Recovery

如果数据库损坏：

必须：

```text
do not delete automatically
```

可以：

```text
rename corrupted db
show error
```

具体策略遵循当前应用数据管理方案。

---

# 255. Schema Backup Before Migration

对于重要 migration：

可以在：

```text
user app-data
```

中生成安全备份。

但不要：

```text
复制项目源码
```

---

# 256. Migration Logging

记录：

```text
old schema
new schema
migration
result
```

便于排障。

---

# 257. Snapshot Storage Encryption

M11 第一版：

> 不要求数据库加密。

因为：

```text
OS user data directory permissions
+
redaction
+
no source persistence
```

已经是合理基础。

未来若需求明确，再设计 encryption-at-rest。

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

# 270. Testing Philosophy

必须维持：

```text
domain unit tests
storage integration tests
CLI integration tests
IPC integration tests
real project tests
```

不要只有：

```text
UI click tests
```

---

# 271. Test Count

不要为了“测试数量很好看”而大量生成重复测试。

重点：

```text
behavior coverage
failure mode coverage
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

# 275. .gitignore

确认：

```text
Prism SQLite database
local app data
temporary snapshot exports
test databases
```

不会污染仓库。

尤其：

> 默认数据库根本不应位于项目目录。

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

# 296. Backward Compatibility

M11 不得破坏：

```text
M1-M10 CLI
IPC
JSON schema
core model
```

除非：

> 有明确必要。

若必须修改：

> 提供兼容层或 migration。

---

# 297. API Stability

M11 新 API 必须尽量：

```text
clear
small
composable
```

不要把：

```text
UI-specific fields
```

塞进核心 Snapshot model。

---

# 298. Core Domain Purity

确保：

```text
Snapshot
SnapshotDiff
TimelineEvent
EvolutionSummary
```

都是工程领域对象。

不能出现：

```text
React
DOM
Tauri window
SQL connection
```

等 UI/infrastructure 污染。

---

# 299. Infrastructure Isolation

SQLite 类型：

```text
SqliteConnection
SqlTransaction
Row
```

不能泄漏到：

```text
prism-core
```

---

# 300. Data Contract

最终形成：

```text
ProjectSnapshot
SnapshotDiff
Timeline
EvolutionSummary
```

四个核心输出。

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

# 323. Snapshot Store 不应主动运行分析器

错误：

```text
SnapshotStore.create()
    ↓
run parser
run health
run verify
...
```

正确：

```text
Application Snapshot Service
    ↓
collect facts
    ↓
SnapshotStore.persist()
```

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