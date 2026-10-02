# Prism M11a — Snapshot Capture and Storage（M11 拆分 Part 1/2）
## 完整、详细、专业开发提示词

> **【系列说明】** 本文档由《Prism M11 — Project Snapshot - Timeline - Evolution Engine 完整开发提示词》拆分而来（Part 1/2）。姊妹篇：`Prism M11b — Snapshot Compare Timeline and Evolution 完整开发提示词.md`（Part 2/2）。两篇合起来覆盖原文 §0–§335 全部内容，章节编号与标题保持原文不变（正文内 "§xx" 交叉引用继续有效）。本篇为独立可执行的子里程碑：完成【阶段性完成门槛】后即可按本篇指令本地 commit（不 push）。

**归属映射（原文 § → 本篇）：**

| 原文区间 | 内容域 |
| --- | --- |
| 引言、§0–§30 | 使命/原则/Git 边界/两种状态/ProjectSnapshot/Identity/Registry/Capture Pipeline/freshness/Domain State/Canonical Serialization/Fingerprint/去重/Storage/SQLite/存储位置/Storage 模块/表与 Schema/SnapshotManifest/持久化边界 |
| §63–§70 | Retention、Migration、Transaction Safety、Atomic Snapshot、Snapshot State、Schema Version、数据版本、Ruleset Version |
| §81–§84 | Persistence Policy、项目级 Disable、Offline First、AI Boundary |
| §91–§92 | Snapshot Note、Baseline Snapshot |
| §103–§111 | 依赖方向、建议架构、Snapshot Service API、Cancellation、Progress、Performance、Verification/Environment 默认行为、Capture Options |
| §114、§116–§123 | 不修改工作树、Delete 不等于修改、Rename/Move、Corruption、Read-only fallback、Persistence Failure、不吞错误、Capture Result |
| §137、§141–§147、§149–§151 | Snapshot Tags、Size/SQLite Index/Cleanup/Concurrency/Capture Lock/No Auto-Snapshot/Manual First、API 不依赖 React、Serialization Contract、Snapshot IDs |
| §157–§159、§164–§169、§171、§176–§177、§183–§194 | Migration/Dedup/Serialization 测试、Working Tree/Rename/跨平台/Symlink/Permission/Partial 测试、Limitations、Dirty Source/Purpose、Logging/Observability/Error Taxonomy/Domain Layer、执行纪律与 P0/P1/P2 |
| §196–§220、§222–§224 | Restart Persistence、多项目、同名、Isolation、并发、Crash、Migration、Old Snapshot、Fingerprint、Database Size、fixtures、可重复、Clock/Path/Database Path、File Mutation、No Network、No AI |
| §226–§231、§239、§241–§244、§248 | Store/Repository/SQL 边界/JSON Boundary/Immutability/Deletion、Branch Dimension、Branch Rename/Detached/No Git Repo/Gitless、Baseline Semantics |
| §252–§257、§270–§271、§275、§296–§300、§323 | Retention Warning/Database Health/Recovery/Schema Backup/Migration Logging/Encryption、Testing Philosophy/Test Count、.gitignore、向后兼容/稳定 API/Core Purity/Infra Isolation/Data Contract、Store 不主动运行分析器 |

**归 M11b 的部分（本篇不做）**：§31–§62（Snapshot Diff/各域 Diff/Finding Identity/History/Timeline/Compare/Evolution/Historical Query/全部 CLI 命令面）、§71–§80、§85–§90、§93–§102、§112–§113、§115、§124–§136、§138–§140、§148、§152–§156、§160–§163、§170、§172–§175、§178–§182、§195、§208–§215、§221、§232–§238、§240、§245–§247、§249–§251、§258–§269、§272–§274、§276–§295、§301–§322、§324–§335（含完整验收、Final Audit、最终 commit）。

---

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



---

# 【阶段性完成门槛 — M11a】

在进入 M11b 之前，本篇必须满足：

1. 本篇归属章节中全部 P0 要求已实现：ProjectSnapshot 模型、域 fingerprint、
   Canonical Serialization、prism-store（schema/migrations/事务/原子写/并发锁）、
   捕获编排与去重、`prism snapshot` 与 `prism history` 基础命令。
2. Restart Persistence Test（§196）真实通过：建快照 → 重启进程 → 快照仍在。
3. 本篇范围测试全绿（§157–§159、§164–§171、§183–§194、§196–§224 中归本篇者）。
4. `cargo test --workspace`、clippy 0 warning、`cargo fmt --check` 通过；
   前端如涉及改动则 typecheck/test/build 通过。
5. Git Safety：捕获前后 `git status` / `HEAD` 完全一致；项目工作树零修改。
6. 输出【M11a 阶段审计】（FACT/HYPOTHESIS/INFERENCE，只列 P0/P1）。
7. 本地 commit：`feat: add snapshot capture and storage (M11 part 1/2)`。不 push。
   随后执行姊妹篇 `Prism M11b — Snapshot Compare Timeline and Evolution 完整开发提示词.md`。
