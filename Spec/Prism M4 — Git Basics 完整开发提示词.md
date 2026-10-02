# Prism M4 — Git Basics
## Git 仓库事实层 / Read-only Git Intelligence

你现在进入 **Prism M4：Git Basics**。

本阶段目标不是做 Change Impact，也不是做“代码变更理解”，而是把 Prism 从：

> 看懂项目

推进到：

> **准确知道项目当前 Git 状态、历史和文件级变化。**

M4 必须建立一个稳定、确定性、只读的 Git Facts Layer，为后续 **M5 ChangeSet**、**M6 Change Impact** 提供真实、可复用、结构化的 Git 基础事实。

---

# 0. 最重要的开发原则

严格遵守以下原则：

1. **Deterministic First**
   - Git 信息必须来自真实 Git 仓库。
   - 不使用 LLM 推断 Git 状态。
   - 不通过启发式猜测 branch、commit、diff。
   - Git 的原始事实优先于任何解释。

2. **Read-only First**
   - M4 只能读取 Git 状态。
   - 不执行 commit / push / pull / fetch / checkout / reset / rebase / merge / cherry-pick / stash 等会改变仓库状态的操作。
   - 不修改工作区文件。
   - 不修改 Git index。
   - 不修改 branch。
   - 不修改 HEAD。

3. **M4 只解决 Git Facts**
   - M4 负责回答：
     - 这是 Git 仓库吗？
     - 仓库根目录在哪里？
     - 当前 branch 是什么？
     - HEAD 是什么？
     - 是否 detached HEAD？
     - 当前 working tree 状态是什么？
     - 哪些文件 modified / added / deleted / renamed / untracked？
     - staged / unstaged 状态分别是什么？
     - 最近有哪些 commits？
     - 某两个 Git 状态之间的 file-level diff 是什么？
   - M4 不负责回答：
     - 某个修改影响了什么模块？
     - 某个修改影响了哪些 symbol？
     - 某个修改有没有风险？
     - 哪些测试受影响？
     - 哪种架构更好？
     - 哪些文件应该修改？
   - 这些属于 M5/M6 或后续阶段。

4. **M4 不重新实现 Git**
   - 优先使用系统 Git CLI。
   - 通过 M0 已有的 `CommandRunner` 执行 Git。
   - 不要为了 M4 引入大型 Git abstraction dependency，除非当前代码库已有明确基础设施且审计证明有必要。
   - GUI、CLI、未来其他模块都必须通过 Prism 自己的 Git domain/application layer 使用 Git，不允许前端直接 shell 调 git。

5. **不要污染 Core**
   - `prism-core` 不应该依赖 Git。
   - Git-specific 类型应位于 Git/domain/application 相关 crate。
   - 保持：
     `Frontend → IPC/Application → Git/Application → Git adapter → CommandRunner`
   - 而不是：
     `Frontend → git CLI`

---

# 1. 先做完整 Baseline Audit，再修改任何代码

第一步必须只审计，不修改。

审计范围：

- M0 Foundation
- M1 Project Discovery
- M2 Code Understanding
- M3 Dependency & Architecture

确认：

### M0
- workspace / crate 是否正常
- error model
- Task / cancellation
- CommandRunner
- FileSystem
- config
- logging
- CLI
- Tauri IPC

### M1
- Project
- ProjectDiscovery
- FileInfo
- DirectoryInfo
- ignore system
- path normalization

### M2
- Symbol
- SymbolIndex
- ProjectCodeModel
- parser abstraction
- diagnostics

### M3
- Module
- Dependency
- Reference
- CallEdge
- DependencyGraph
- ArchitectureGraph
- resolution status
- graph metrics

必须确认：

- 哪些可以直接复用
- 哪些接口需要扩展
- 哪些模块禁止修改
- 是否存在此前阶段留下的技术债
- 当前测试是否全绿

输出 baseline：

```text
FACT
HYPOTHESIS
INFERENCE
```

并给每项状态：

```text
CONFIRMED
RESOLVED
WORSENED
```

只允许列：

```text
P0
P1
```

不要为了“代码更漂亮”制造 P2/P3。

---

# 2. M4 的边界

## M4 做什么

M4 建立：

> Git Repository → Git Facts → normalized domain objects → CLI / GUI

包含：

### Repository Facts
- 是否 Git repository
- repository root
- `.git` 类型
- worktree 基本信息
- 当前 HEAD
- 当前 branch
- detached HEAD
- unborn branch

### Working Tree Facts
- staged
- unstaged
- untracked
- ignored（至少能够明确区分是否 ignored）
- modified
- added
- deleted
- renamed

### Commit Facts
- commit id
- parent(s)
- author
- committer
- timestamp
- subject
- message
- refs
- branch relationship的原始事实

### Diff Facts
- old path
- new path
- change type
- old/new blob ids（可获取时）
- old/new file mode
- additions
- deletions
- hunks
- line-level diff
- binary detection
- rename metadata
- similarity metadata（如果 Git 提供）

### Refs
- branch
- remote-tracking branch
- tag
- HEAD
- symbolic refs

---

# 3. M4 明确不做什么

绝对不要在 M4 实现：

- ChangeSet
- ChangedSymbol
- ChangedModule
- Impact Analysis
- Risk Score
- Health Score
- AI interpretation
- architecture recommendation
- test impact
- documentation impact
- context generation
- Context Pack
- Snapshot persistence
- database
- embeddings
- RAG
- MCP
- AI provider
- automatic code fix
- automatic git operation
- code editor
- IDE
- debugger
- runtime tracing

尤其注意：

## 不要把 Git Diff 直接解释成 ChangeSet

例如：

```text
Git says:
src/foo.ts modified

```

M4 只能记录：

```text
file changed
```

不能直接产生：

```text
FooService.handle() changed
```

更不能直接产生：

```text
FooService may affect PaymentModule
```

这些由 M5/M6 完成。

---

# 4. Git implementation strategy

优先方案：

```text
Prism
  ↓
Git application/domain layer
  ↓
Git adapter
  ↓
CommandRunner
  ↓
git executable
```

通过结构化 Git 命令获取数据。

尽量避免解析人类可读输出。

优先使用：

- `git rev-parse`
- `git status --porcelain`
- `git status --porcelain=v2`
- `git branch`
- `git log`
- `git show`
- `git diff`
- `git diff --cached`
- `git ls-files`
- `git ls-tree`
- `git for-each-ref`

优先采用：

- NUL-delimited output
- porcelain formats
- machine-readable formats
- stable delimiters

不要依赖：

- terminal formatting
- localized output
- colorized output
- human-readable column alignment
- 当前 shell locale

---

# 5. Git abstraction

建立清晰的 Git capability abstraction。

建议：

```rust
trait GitProvider
```

或者当前架构中语义等价的接口。

至少需要支持：

```text
discover_repository()
get_repository_info()
get_status()
get_head()
get_branches()
get_refs()
get_log()
get_diff()
get_file_diff()
```

不要为了未来而建立几十个接口。

接口必须：

- 可测试
- 可 mock
- 可取消
- 错误明确
- 不与 GUI 耦合
- 不与 CLI 耦合

---

# 6. Domain Model

不要使用 Git CLI 输出字符串作为 Prism 的业务模型。

建立 normalized domain model。

建议至少包含：

---

## GitRepository

```rust
GitRepository {
    id,
    root_path,
    git_dir,
    common_git_dir,
    is_bare,
    is_worktree,
}
```

其中：

- `root_path` 使用项目规范化后的路径规则
- 尽量复用 M1 path abstraction
- Git-specific metadata 不塞进 `Project`

---

# 7. Git HEAD

建立：

```rust
GitHead
```

至少包含：

```text
kind:
    Branch
    Detached
    Unborn
    Unknown

commit_id
branch_name
symbolic_ref
```

必须正确处理：

### 正常 branch

```text
HEAD -> refs/heads/main
```

### Detached HEAD

例如：

```text
HEAD -> <commit>
```

必须显示：

```text
detached = true
```

### Unborn branch

例如新 repo：

```text
No commits yet
HEAD -> refs/heads/main
```

不能误判为 detached。

---

# 8. Git Branch

建立：

```text
GitBranch
```

至少支持：

```text
name
full_ref
is_current
is_remote
upstream
ahead
behind
```

其中 ahead/behind：

- 只有可可靠获得时才提供
- 不要自行计算并制造错误事实
- 如果当前实现没有远端信息，则使用明确 absence 状态

不要为了实现 ahead/behind 自动执行：

```text
git fetch
```

绝对禁止。

---

# 9. Git Ref

建立：

```text
GitRef
```

至少包含：

```text
name
full_name
kind
target
```

kind 可以：

```text
Head
Branch
RemoteBranch
Tag
Other
```

不要把 refs 的解释扩展成架构或项目语义。

---

# 10. Git Commit

建立：

```text
GitCommit
```

至少包含：

```text
id
short_id
parents[]
author
committer
authored_at
committed_at
subject
message
refs[]
```

要求：

- commit id 使用完整 SHA-1/SHA-256 字符串
- 不要只保存 short hash
- short hash 可以作为展示字段
- merge commit 必须支持多个 parents
- initial commit 必须支持零 parents

不要对 commit message 做 LLM summary。

---

# 11. Git Status

建立：

```text
GitStatus
```

必须能够区分：

```text
index state
working tree state
untracked
ignored
```

建议 normalized enum：

```text
GitFileState:
    Unmodified
    Added
    Modified
    Deleted
    Renamed
    Copied
    TypeChanged
    Unmerged
    Untracked
    Ignored
    Unknown
```

注意：

Git status 的两个维度不能压成单一状态。

例如：

```text
MM
```

意味着：

```text
index = Modified
worktree = Modified
```

又例如：

```text
AM
```

意味着：

```text
index = Added
worktree = Modified
```

必须保留这类信息。

---

# 12. GitFileStatus

建立：

```text
GitFileStatus
```

至少：

```text
path
original_path
index_state
working_tree_state
is_untracked
is_ignored
is_renamed
is_copied
is_conflicted
```

对于 rename：

```text
original_path
path
```

两个路径都应该可用。

不要只返回最终 path。

---

# 13. Untracked / Ignored

必须明确区分：

```text
untracked
```

与：

```text
ignored
```

例如：

```text
new-file.ts
```

可以是 untracked。

而：

```text
target/
node_modules/
```

可能是 ignored。

不要把 ignored 文件混入普通 untracked。

GUI 默认可以隐藏 ignored，但底层模型不能丢失这一事实。

---

# 14. Git Diff Model

建立：

```text
GitDiff
GitDiffFile
GitDiffHunk
GitDiffLine
```

---

## GitDiff

至少：

```text
base
target
files[]
stat
```

---

## GitDiffFile

至少：

```text
old_path
new_path
change_type
old_mode
new_mode
old_blob
new_blob
is_binary
is_rename
similarity
additions
deletions
hunks[]
```

ChangeType：

```text
Added
Modified
Deleted
Renamed
Copied
TypeChanged
Unmerged
Unknown
```

---

# 15. Diff Hunk

建立：

```text
GitDiffHunk
```

至少：

```text
old_start
old_lines
new_start
new_lines
header
lines[]
```

---

# 16. Diff Line

至少：

```text
kind:
    Context
    Addition
    Deletion

old_line
new_line
content
```

不要在 M4 直接把 diff line 与：

- Symbol
- Module
- Dependency
- Health Finding

关联。

M5 才建立：

```text
Diff → Symbol
```

---

# 17. Staged / Unstaged Diff

M4 必须支持至少三种 Git diff 语义：

### Working Tree vs Index

```text
git diff
```

表示：

> 工作区相对 index 的修改

### Index vs HEAD

```text
git diff --cached
```

表示：

> 已 staged 相对 HEAD 的修改

### Commit / Revision vs Revision

例如：

```text
git diff <rev1> <rev2>
```

表示：

> 两个 revision 之间的 file-level diff

必须把这三种语义明确建模，不要全部叫简单的：

```text
diff
```

建议：

```text
GitDiffScope
```

例如：

```text
WorkingTree
Staged
RevisionRange
```

---

# 18. Git history

M4 至少提供：

```text
git log
```

对应能力：

- 最近 N 个 commits
- commit details
- parents
- author
- timestamp
- refs

支持 pagination / limit。

不要一次性读取整个仓库历史。

建议：

```text
limit
offset/cursor
```

或者当前架构中更合适的分页方式。

---

# 19. Merge Commit

必须正确处理：

```text
A
|\
B C
|/
D
```

merge commit：

```text
parents = [B, C]
```

不能假定只有一个 parent。

后续 ChangeSet / Impact 层可能需要 merge semantics，但 M4 这里只需保证原始事实完整。

---

# 20. Rename / Copy

Git rename detection 可能受 Git 配置或命令参数影响。

M4：

- 尽可能保留 Git 识别出的 rename/copy
- 保存 old path / new path
- 保存 similarity（若有）
- 不自行基于内容猜测 rename
- 不把 rename 强行解释成 delete + add

如果 Git 没有识别 rename：

```text
就按 Git 返回的事实记录
```

不要自作主张修改。

---

# 21. Binary files

必须支持 binary diff。

对于 binary 文件：

```text
is_binary = true
```

可以只有：

```text
additions/deletions = None
```

或者其他明确的无行级 diff 表达。

不要读取大量 binary 内容。

不要把 binary 文件强行解析为 UTF-8。

---

# 22. File Mode / Type Change

至少考虑：

```text
100644 -> 100755
```

以及：

```text
file -> symlink
```

之类 Git mode change。

如果 Git 提供：

```text
old_mode
new_mode
```

则保存。

M4 不必解释其安全意义。

---

# 23. Submodule

对于 submodule：

至少不要把它当普通文本文件处理。

能够识别：

```text
Submodule
```

是加分项。

至少保证：

- status 不崩溃
- diff 不崩溃
- path 正确
- Git object change 信息不被误解析

M4 不要求递归分析 submodule。

默认：

```text
parent repo only
```

递归 submodule analysis 留给未来阶段。

---

# 24. Git repository discovery

Prism 打开一个项目后：

必须能够判断：

```text
project root
→ nearest Git repository
```

同时处理：

### 普通 repo

```text
project/
  .git/
```

### Git worktree

`.git` 可能是文件而不是目录。

不要简单：

```text
Path.join(".git").is_dir()
```

就认为不是 Git repo。

必须通过 Git 本身验证。

---

# 25. Non-Git project

对于非 Git 项目：

不能当成错误。

例如：

```text
GitStatus {
    repository: None
}
```

或者等价设计。

CLI：

```text
prism git status ./project
```

应输出明确状态：

```text
not a git repository
```

但不能 panic。

GUI：

```text
Git repository not detected
```

而不是显示系统错误堆栈。

---

# 26. Git errors

区分：

```text
NotGitRepository
GitUnavailable
InvalidRepository
PermissionDenied
CommandFailed
MalformedGitOutput
Timeout
Cancelled
Unsupported
Unknown
```

如果系统没有 Git：

```text
GitUnavailable
```

不要把它包装成：

```text
Internal
```

如果 Git 命令存在但仓库不存在：

```text
NotGitRepository
```

如果 Git 输出无法解析：

```text
MalformedGitOutput
```

并保留必要诊断。

---

# 27. Git version

Environment 是 M9。

因此 M4 不需要建立完整：

```text
EnvironmentSnapshot
```

但 Git adapter 可以探测：

```text
git --version
```

作为 Git capability metadata。

不要重复实现 M9。

M9 最终会把 Git 版本纳入完整 Environment。

---

# 28. Git command runner

复用 M0：

```text
CommandRunner
```

必须：

- timeout
- cancellation
- cwd
- captured stdout
- captured stderr
- exit code
- execution duration

Git adapter 不得直接：

```rust
std::process::Command
```

到处散落。

只允许通过统一 command execution abstraction。

---

# 29. Parsing Git output

重要：

不要：

```rust
split('\n')
```

然后用复杂字符串规则猜 status。

优先：

```text
porcelain v2
-z
```

尽可能使用机器稳定格式。

需要注意：

- filenames may contain spaces
- filenames may contain tabs
- filenames may contain unicode
- filenames may contain newline
- rename paths can contain unusual characters
- shell escaping is not safe parsing

因此必须：

> 优先使用 NUL-delimited Git output。

测试必须覆盖特殊文件名。

---

# 30. Encoding / Unicode

Git path 不应假定只能是 ASCII。

至少测试：

```text
中文文件名
emoji
spaces
parentheses
brackets
特殊字符
```

尽量保证：

```text
Git output → normalized UTF-8
```

如果平台编码问题无法完整解决：

- 必须有诊断
- 不允许静默损坏路径

---

# 31. Paths

遵循 M1 的 path strategy：

### Domain identity

使用：

```text
project-relative path
```

### OS operation

使用：

```text
absolute filesystem path
```

Git domain 不应把机器绝对路径当成文件身份。

例如：

```text
src/main.ts
```

作为逻辑 path。

而：

```text
C:\Users\...\project\src\main.ts
```

只作为执行时路径。

---

# 32. GUI

M4 开始：

## Changes 页面从 placeholder 变成真实数据页面。

至少显示：

### Repository Header

```text
Repository
Branch
HEAD
Status
```

例如：

```text
main
a93f2d1
5 modified
2 untracked
1 staged
```

不要制造“健康百分比”。

---

# 33. Changes 页面

建议结构：

```text
Changes

Repository
  branch
  HEAD
  repository root

Working Tree
  Staged
  Modified
  Added
  Deleted
  Renamed
  Untracked

Diff
  file list
  additions
  deletions
  hunks

History
  recent commits
```

---

# 34. File-level diff UI

选择一个文件：

左/右或 unified diff 均可。

至少能看：

```text
old path
new path
change type
additions
deletions
diff hunks
```

M4 不需要做：

- symbol highlighting
- architecture impact overlay
- risk score
- semantic explanation
- AI summary

---

# 35. Large repositories

不能因为一个 repo 很大就：

```text
git log entire repository
git diff entire repository
```

一次全部载入内存。

必须设计：

- bounded result size
- pagination
- diff file limit
- log limit
- cancellation
- background execution
- progress

尤其是 GUI 打开 Changes 页面时。

默认只加载合理范围。

---

# 36. Determinism

同一个：

```text
repo state
+
same command arguments
```

应产生稳定的 normalized output。

排序必须明确。

例如：

```text
Git refs
Git status
Git diff files
```

必须采用稳定排序。

但 commit history 本身应保持 Git 的历史顺序。

不要为了“稳定”打乱时间/拓扑语义。

---

# 37. JSON contract

CLI 输出的 JSON 必须是明确 contract。

不要直接：

```text
println!("{:?}", value)
```

必须：

```json
{
  "repository": {},
  "head": {},
  "status": {},
  "branches": [],
  "refs": [],
  "commits": [],
  "diff": {}
}
```

具体结构依据当前 Rust domain model 决定。

要求：

- serde
- versionable
- stable field names
- 明确 nullability
- 不暴露内部 debug fields
- 不暴露绝对机器路径，除非命令明确要求

---

# 38. CLI

M4 不要抢占 M5 的：

```text
prism changes
```

推荐建立 Git 专属命令：

```text
prism git status <path>
prism git log <path>
prism git diff <path>
prism git show <path> <revision>
prism git refs <path>
```

如果当前 CLI 架构更适合：

```text
prism git status
prism git log
prism git diff
```

则保持统一。

至少支持：

```text
--json
--limit
--staged
--working-tree
--from
--to
```

具体参数根据现有 CLI 风格统一。

---

# 39. Exit codes

CLI 必须区分：

```text
0 = success
1 = requested operation failed
2 = invalid user input
```

非 Git project：

是否算 error 必须根据当前命令语义定义。

建议：

```text
scan
```

非 Git 不应失败。

而：

```text
prism git status
```

明确请求 Git：

```text
not a git repository
```

可以使用专门的非零 exit code。

重点是：

> exit code 必须稳定且 documented。

---

# 40. Git safety

M4 必须做安全审计。

确保没有任何代码路径执行：

```text
git commit
git push
git pull
git fetch
git checkout
git switch
git reset
git clean
git restore
git rebase
git merge
git stash
```

即使未来 GUI 想加按钮：

```text
“创建提交”
```

M4 也不要实现。

M4 是：

> Git observer

而不是：

> Git operator

---

# 41. Security / Privacy

Prism local-first。

禁止：

- telemetry
- analytics
- 上传 repository
- 上传 diff
- 上传 commit message
- 自动联网
- 自动调用 AI

尤其注意：

Git commit、diff、路径本身可能包含敏感信息。

日志中：

不要输出：

```text
full source
full diff
tokens
credentials
private env values
```

调试日志只记录必要 metadata。

---

# 42. Testing strategy

测试必须是真实 Git 仓库行为，而不是只 mock parser。

建议建立：

```text
GitFixtureRepository
```

测试运行时临时创建 repo。

不要依赖用户机器现有 Git repo。

---

# 43. 必须覆盖的 Git fixtures

至少覆盖：

### Repo basics
- empty repo
- initial commit
- normal branch
- detached HEAD
- unborn branch

### Status
- clean
- modified
- staged
- staged + modified
- added
- deleted
- untracked
- ignored
- renamed
- copied
- conflicted/unmerged（尽可能）

### Diff
- added file
- modified file
- deleted file
- rename
- binary
- mode change
- staged diff
- working-tree diff
- revision diff

### History
- one commit
- multiple commits
- merge commit
- branch refs
- tags
- remote-tracking refs if fixture supports it

### Path edge cases
- unicode
- Chinese
- spaces
- special characters
- filenames with unusual characters

### Failure
- Git executable missing
- invalid path
- non-repository
- inaccessible path
- command timeout
- malformed output fixture where parser tests are applicable
- cancellation

---

# 44. Regression tests

M0-M3 全部回归。

至少保证：

```text
M0 tests pass
M1 tests pass
M2 tests pass
M3 tests pass
M4 tests pass
```

尤其不能因为添加 Git dependency：

- 破坏 Core
- 破坏 CLI
- 破坏 parser
- 破坏 graph
- 改变已有 JSON contract

除非经过明确且有证据的迁移。

---

# 45. Self-scan

必须再次运行 Prism 自身：

```text
Prism repository
```

确认：

- Git repository detected
- branch detected
- HEAD detected
- status detected
- history detected
- diff works
- no panic
- no unexpected untracked state
- JSON valid
- GUI Changes 页面可用

这是本阶段最重要的真实项目验证之一。

---

# 46. Performance

M4 不追求极限性能，但必须避免明显错误。

禁止：

```text
每个文件执行一次 git
每个 commit 执行一次 git
每个 status 项目再执行一次 git
```

应该尽可能：

```text
batch query
```

例如 status 一次取得整个 repository 状态。

不要：

```text
git status file1
git status file2
git status file3
...
```

---

# 47. Caching

M4 暂时：

> 不做 persistent cache。

允许：

- request-local caching
- command result reuse
- in-memory reuse

但不要引入：

- database
- SQLite
- persistent analysis cache

正式持久化缓存留给 M13 性能阶段，或根据真实需要再决定。

---

# 48. 与 M1/M2/M3 的关系

严格保持：

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
ChangeSet
    ↓
M6
Impact
```

不要越级。

---

# 49. 为 M5 留接口

M4 完成后，M5 应该能够很自然地实现：

```text
GitDiffFile
    ↓
ChangedFile
    ↓
ChangedSymbol
    ↓
ChangedModule
    ↓
ChangeSet
```

因此 Git diff domain 必须保留足够原始信息：

```text
old_path
new_path
change_type
hunks
old/new line
blob
mode
rename
```

但是：

> M4 不负责把它们绑定到 Symbol / Module。

---

# 50. 为 M6 留接口

M6 将使用：

```text
ChangeSet
+
DependencyGraph
+
ProjectCodeModel
```

生成：

```text
ImpactAnalysis
```

所以 M4 不应该提前把：

```text
GitDiff
DependencyGraph
```

耦合。

保持：

```text
Git facts
```

与：

```text
Architecture facts
```

独立。

---

# 51. Recommended crate/module structure

根据当前真实 workspace 调整，不要机械照抄。

推荐方向：

```text
crates/
  prism-core/
  prism-project/
  prism-parser/
  prism-git/
  prism-analysis/
  prism-health/
  prism-verify/
  prism-environment/
  prism-context/
  prism-report/
  prism-cli/
```

M4 主要实现：

```text
prism-git
```

必要时：

```text
prism-core
prism-cli
Tauri application layer
React Changes page
```

但：

> 只有确有必要才修改其他 crate。

不要为了“重构漂亮”大规模移动代码。

---

# 52. Recommended Git module structure

可参考：

```text
prism-git/
  src/
    lib.rs
    model/
      repository.rs
      head.rs
      branch.rs
      reference.rs
      commit.rs
      status.rs
      diff.rs
    provider/
      mod.rs
      cli.rs
    parser/
      status.rs
      log.rs
      refs.rs
      diff.rs
      show.rs
    service/
      repository_service.rs
      status_service.rs
      history_service.rs
      diff_service.rs
    error.rs
```

实际结构根据已有项目风格决定。

不要为了达到这个目录结构而重构整个 workspace。

---

# 53. Error philosophy

Git adapter 的错误必须：

- contextual
- typed
- actionable
- non-panicking

例如：

```text
Failed to execute git status:
repository=/project
exit_code=128
stderr=<sanitized>
```

不要：

```text
unwrap()
expect()
panic!()
```

出现在普通 Git failure path。

---

# 54. Cancellation

GUI 中如果用户：

```text
打开巨大仓库
```

然后立刻切换项目：

旧 Git task 应能够：

```text
cancel
```

不能留下大量后台 git process。

CommandRunner 的 cancellation 要真正传递到底层执行。

如果平台能力有限：

- 明确记录 limitation
- 不伪造“已取消”

---

# 55. Concurrency

允许多个只读 Git 查询：

```text
status
head
refs
```

但不要无意义并行几十个 Git process。

应该根据现有 Task infrastructure 控制：

```text
bounded concurrency
```

尤其 GUI 进入 Changes 页面时。

---

# 56. GUI 数据流

建议：

```text
ChangesPage
   ↓
Tauri IPC
   ↓
Git application service
   ↓
Git service
   ↓
GitProvider
   ↓
CommandRunner
   ↓
git
```

React 不直接：

```text
spawn("git")
```

---

# 57. GUI loading / errors

必须有：

```text
Idle
Loading
Loaded
Empty
NotGitRepository
Error
Cancelled
```

不能：

```text
loading forever
```

---

# 58. Important UI semantics

状态：

```text
Modified
```

不能直接解释成：

```text
Risk
```

Rename：

```text
Rename
```

不能直接解释成：

```text
Breaking Change
```

Commit 数量：

```text
20 commits
```

不能解释成：

```text
Healthy / unhealthy
```

M4 只展示事实。

---

# 59. Architecture Graph integration

M4 可以把：

```text
Git status
```

与：

```text
Architecture Graph
```

放在不同页面展示。

不要在 M4 自动修改 M3 graph。

因为：

```text
Architecture graph = current project structure
Git diff = changes between states
```

两个概念不同。

---

# 60. Documentation

更新：

```text
README
architecture docs
CLI docs
development docs
```

至少说明：

- M4 做什么
- Git requirements
- supported Git versions / assumptions（只有真实测试确认后才能写）
- CLI examples
- JSON output
- read-only guarantee
- known limitations

不要写未经测试的：

```text
supports all git repositories
```

---

# 61. Example CLI behavior

例如：

```bash
prism git status .
```

JSON 大致：

```json
{
  "repository": {
    "root": "/project"
  },
  "head": {
    "kind": "Branch",
    "branch": "main",
    "commit_id": "..."
  },
  "files": [
    {
      "path": "src/app.ts",
      "index_state": "Unmodified",
      "working_tree_state": "Modified"
    }
  ]
}
```

注意：

以上只是 contract 方向。

必须根据真实 domain model 最终定义。

---

# 62. Example diff semantics

例如：

```bash
prism git diff .
```

应能够表达：

```text
src/app.ts
  Modified
  +5
  -2

src/new.ts
  Added
  +38

src/old.ts → src/new-name.ts
  Renamed
```

但不要输出：

```text
App architecture weakened
```

也不要：

```text
this probably breaks X
```

---

# 63. Quality gate

完成实现后，必须执行真实质量检查：

### Rust

```text
cargo fmt --check
cargo check
cargo test
```

如果项目已有：

```text
clippy
```

执行现有约定。

### Frontend

执行当前项目实际存在的：

```text
typecheck
build
test
lint
```

不要假定命令名。

---

# 64. Git fixture validation

除了单元测试，还必须：

1. 创建临时真实 Git repo
2. 执行实际 Git commands
3. 创建 commits
4. 修改文件
5. staged / unstaged
6. rename
7. delete
8. untracked
9. diff
10. detached HEAD
11. merge commit（能实现则测试）
12. 使用 Prism 读取

这是 M4 的真实验收核心。

---

# 65. No fake tests

禁止大量这种测试：

```rust
assert_eq!(
    parse_git_output("...")
)
```

却没有验证：

```text
真实 git → output
```

parser unit tests 可以存在。

但必须搭配：

```text
real Git integration tests
```

否则无法证明系统真正可用。

---

# 66. Test categories

明确区分：

```text
Unit
Integration
Fixture
CLI
IPC
GUI
Regression
```

不要把所有测试塞进一个模块。

---

# 67. Release-quality audit

完成后进行一次完整审计。

必须输出：

# M4 Final Audit

## A. Baseline
M0-M3 当前状态。

## B. Git Architecture
- GitProvider
- CommandRunner integration
- crate dependency direction

## C. Repository Model
- repo detection
- worktree
- bare repo handling

## D. HEAD / Branch / Ref
- branch
- detached HEAD
- unborn HEAD
- refs

## E. Status
- staged
- unstaged
- untracked
- ignored
- rename
- conflict

## F. History
- commit
- parent
- merge commit
- pagination

## G. Diff
- file change
- hunk
- line
- binary
- rename
- mode
- staged/unstaged/revision

## H. CLI
实际命令与 exit code。

## I. GUI
Changes 页面。

## J. Tests
Unit / Integration / CLI / GUI / Regression。

## K. Performance
large repo / pagination / cancellation。

## L. Security & Privacy
- no network
- no telemetry
- no secrets
- read-only guarantee

## M. Git
- branch
- status
- diff
- untracked
- unintended files

## N. Risks
只输出：

```text
P0
P1
```

## O. Deferred
明确列出留给 M5+：

```text
ChangeSet
ChangedSymbol
ChangedModule
Impact Analysis
Risk Analysis
Context generation
Snapshot
```

---

# 68. Audit writing format

任何审计结论必须区分：

```text
FACT
HYPOTHESIS
INFERENCE
```

例如：

```text
FACT:
Git status is obtained through git porcelain output.

FACT:
Rename paths are preserved.

HYPOTHESIS:
Git behavior may differ across very old Git versions.

INFERENCE:
Current implementation is sufficient for supported environments.
```

不要把 inference 写成 fact。

---

# 69. Regression gate

只有以下全部成立，才算 M4 完成：

```text
[ ] M0 passes
[ ] M1 passes
[ ] M2 passes
[ ] M3 passes
[ ] M4 tests pass
[ ] Real Git repo validation passes
[ ] Non-Git project handled
[ ] Detached HEAD handled
[ ] Unborn repo handled
[ ] Staged/unstaged separated
[ ] Untracked/ignored separated
[ ] Rename handled
[ ] Delete handled
[ ] Binary handled
[ ] Revision diff works
[ ] Commit history works
[ ] Merge commit handled
[ ] Unicode paths handled
[ ] Cancellation works
[ ] GUI Changes page works
[ ] CLI JSON works
[ ] No Git mutating command exists
[ ] No network / telemetry
[ ] No AI dependency
[ ] No P0
[ ] No unresolved P1 that blocks M5
```

---

# 70. Git commit

完成并通过审计后：

只创建一个本阶段功能 commit：

```text
feat: add git basics
```

不要 push。

不要修改：

```text
origin
remote config
branch policy
```

---

# 71. 最终目标

M4 完成后的 Prism 应该能够做到：

```text
打开一个本地项目
        ↓
发现 Git repository
        ↓
知道当前 branch
        ↓
知道 HEAD
        ↓
知道工作区状态
        ↓
知道 staged / unstaged / untracked
        ↓
知道最近 commits
        ↓
知道两个 revision 之间发生了哪些文件级变化
        ↓
以统一 JSON / GUI / CLI 展示
```

此时 Prism 已经拥有：

```text
M1 = File Facts
M2 = Code Facts
M3 = Relationship Facts
M4 = Git Facts
```

下一阶段 M5 才开始真正把：

```text
Git Diff
    ↓
Changed Files
    ↓
Changed Symbols
    ↓
Changed Modules
    ↓
ChangeSet
```

建立起来。

**不要在 M4 偷跑 M5。**

---

# 72. 最终开发要求

现在开始工作。

严格遵循：

```text
1. Baseline audit
2. Identify exact integration points
3. Implement M4 incrementally
4. Add unit tests
5. Add real Git integration fixtures
6. Wire CLI
7. Wire GUI
8. Run full regression
9. Self-scan Prism
10. Final audit
11. Fix only P0/P1
12. Commit locally
13. Do not push
```

开发过程中：

- 不进行无关重构
- 不增加无关依赖
- 不为了未来需求提前实现复杂抽象
- 不添加 AI
- 不联网
- 不实现 Git mutation
- 不实现 ChangeSet
- 不实现 Impact
- 不实现 Health
- 不实现 Context
- 不实现 Snapshot

最终交付的是：

> **一个可靠、只读、确定性、可测试、可复用的 Git Facts Layer。**

而不是一个 Git GUI 或 Git 客户端。