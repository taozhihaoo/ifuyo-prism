# ifuyo Prism — M1 Project Discovery
## 完整、详细、专业开发提示词

你现在负责实现 **ifuyo Prism** 的 **M1 — Project Discovery / 项目发现阶段**。

M0 已建立 Prism 的 Foundation。  
M1 的任务是在 M0 基础上，让 Prism 能够：

> **打开一个本地项目 → 扫描项目 → 识别项目结构 → 识别文件与语言 → 识别常见配置 → 推断候选入口点 → 建立可持久化的 Project Snapshot 基础数据 → 在 GUI 中真实展示结果。**

M1 的核心原则：

> **发现事实，不解释事实。**

M1 只回答：

- 这个项目在哪里？
- 项目有多大？
- 有哪些目录？
- 有哪些文件？
- 文件是什么类型？
- 使用了哪些主要语言？
- 项目可能是什么技术栈？
- 存在哪些常见项目配置？
- 哪些文件可能是入口？
- 当前扫描是否完成？
- 当前扫描发现了什么？

M1 暂时不要回答：

- 为什么这样设计？
- 模块之间有什么依赖？
- 哪些架构有问题？
- 某次 Git 改动影响了什么？
- 项目健康度是多少？
- 哪些代码存在风险？
- 如何生成 AI Context？

这些属于 M2+。

---

# 1. M1 与整个 Prism 路线的关系

当前路线：

```text
M0 Foundation
    ↓
M1 Project Discovery
    ↓
M2 Code Understanding
    ↓
M3 Dependency & Architecture
    ↓
M4 Git Basics
    ↓
M5 ChangeSet
    ↓
M6 Change Impact
    ↓
M7 Health
    ↓
M8 Verify
    ↓
M9 Environment
    ↓
M10 Context
    ↓
M11 Snapshot
```

M1 是从：

> “Prism 是一个桌面程序”

进入：

> “Prism 真正开始理解项目”

的第一个阶段。

因此 M1 生成的数据模型必须认真设计，但不能提前承担 M2～M11 的职责。

---

# 2. M1 的核心目标

M1 完成以后，Prism 至少应当能：

```text
选择项目目录
      ↓
建立 Project
      ↓
执行扫描
      ↓
遍历目录
      ↓
识别文件
      ↓
过滤不应分析的内容
      ↓
识别文件类型
      ↓
识别语言
      ↓
识别常见配置
      ↓
识别候选入口点
      ↓
统计项目规模
      ↓
生成 Project Discovery Result
      ↓
GUI 展示
      ↓
JSON / CLI 输出
```

---

# 3. M1 的边界

## 必须实现

### Project Selection

- 打开本地项目目录
- 验证目录有效性
- 建立 Project

### Filesystem Discovery

- 递归目录扫描
- 文件发现
- 目录发现
- 文件 metadata
- 相对路径
- 文件大小
- 文件扩展名

### Ignore Rules

- `.git`
- `node_modules`
- `target`
- `bin`
- `obj`
- `.venv`
- `venv`
- `__pycache__`
- `dist`
- `build`
- 常见缓存 / 构建目录

并建立可扩展机制。

### Language Detection

至少支持：

- TypeScript
- JavaScript
- TSX
- JSX
- Python
- C#
- Rust
- Java
- C/C++
- Go
- HTML
- CSS
- SCSS
- JSON
- YAML
- TOML
- Markdown
- SQL
- Shell

### Project Type Detection

至少尝试识别：

- Node / npm / pnpm / yarn
- Python
- .NET / C#
- Rust / Cargo
- Java / Maven / Gradle
- C/C++
- Go

不要要求百分之百准确。

结果必须带：

```text
confidence
evidence
```

### Configuration Discovery

识别常见配置：

```text
package.json
pnpm-workspace.yaml
yarn.lock
package-lock.json
Cargo.toml
Cargo.lock
*.csproj
*.sln
*.fsproj
pyproject.toml
requirements.txt
Pipfile
pom.xml
build.gradle
build.gradle.kts
go.mod
CMakeLists.txt
Makefile
Dockerfile
docker-compose.yml
docker-compose.yaml
tsconfig.json
vite.config.*
webpack.config.*
next.config.*
vite.config.*
README.md
AGENTS.md
CLAUDE.md
.gitignore
.editorconfig
```

注意：

> 只做“存在性与基础识别”，不要解析这些文件中的完整业务内容。

---

# 4. M1 不做的事情

严格禁止提前实现：

## AST

不要做：

- function extraction
- class extraction
- method extraction
- symbol table
- AST graph

## Dependency Analysis

不要做：

- import graph
- reference graph
- call graph
- dependency graph

## Architecture

不要做：

- architecture smell
- module boundary evaluation
- coupling analysis
- architecture scoring

## Git

不要做：

- commit analysis
- diff analysis
- branch analysis
- changed symbols

Git repository 是否存在可以作为 Project Fact，但：

> 不进入 Git 分析领域。

## Health

不要做：

- health score
- quality score
- complexity score
- architecture findings
- security findings

## Verification

不要做：

- 自动执行 test
- 自动执行 build
- lint orchestration

## Context

不要做：

- context generation
- context ranking
- prompt generation
- AI context pack

## Snapshot History

M1 可以生成**当前扫描结果快照数据结构**作为基础。

但是不要实现：

- 时间线
- 历史比较
- 趋势
- snapshot diff

这些属于 M11。

---

# 5. 首先 Baseline Audit

在修改任何代码前：

检查 M0 当前真实状态：

```text
Git branch
Git status
HEAD
recent commits
Cargo workspace
frontend
Tauri
CLI
Core
existing tests
fixture
docs
```

重点确认 M0 是否真实完成：

```text
cargo check
cargo test
frontend typecheck
frontend build
Tauri runtime
CLI
```

如果 M0 存在 P0/P1 阻塞问题：

> 先修复必要问题，再进入 M1。

不要在有严重 Foundation 问题的情况下继续堆业务代码。

---

# 6. M1 Architecture

M1 建议形成以下结构：

```text
prism-project/
├─ src/
│  ├─ project.rs
│  ├─ discovery.rs
│  ├─ scanner.rs
│  ├─ ignore.rs
│  ├─ file.rs
│  ├─ language.rs
│  ├─ project_type.rs
│  ├─ configuration.rs
│  ├─ entry_point.rs
│  └─ statistics.rs
```

具体文件结构可以根据 M0 实际结构调整。

不要机械照抄。

---

# 7. 核心 Domain Model

M1 必须正式建立以下数据模型。

---

## 7.1 Project

建议：

```rust
Project {
    id: ProjectId,
    name: String,
    root_path: PathBuf,
}
```

M1 可以加入：

```text
discovered_at
```

但不要塞入：

```text
health
git history
architecture
context
environment
```

---

# 8. ProjectDiscovery

建立扫描结果对象，例如：

```text
ProjectDiscovery
{
    project
    scan
    files
    directories
    languages
    project_types
    configurations
    entry_points
    statistics
}
```

要求：

> Discovery Result 必须是一个能够被 GUI、CLI、JSON report 同时消费的标准数据结构。

---

# 9. Scan Metadata

建立：

```text
ScanInfo
```

建议：

```text
scan_id
started_at
finished_at
duration
root_path
files_scanned
directories_scanned
files_ignored
errors_count
status
```

status：

```text
Queued
Running
Completed
Failed
Cancelled
```

直接复用 M0 Task / cancellation 基础设施，不要另造一套任务状态系统。

---

# 10. File Model

每个被发现的文件至少具备：

```text
FileInfo
{
    id
    relative_path
    file_name
    extension
    size
    language
    category
}
```

可以加入：

```text
modified_at
is_hidden
is_symlink
```

但要避免把 FileInfo 做得过大。

---

# 11. Path Rule

**永远不要把项目文件的绝对路径作为主要业务标识。**

内部应优先：

```text
project.root
+
relative_path
```

例如：

```text
project/
├─ src/
│  └─ main.ts
```

应该保存：

```text
src/main.ts
```

而不是：

```text
C:\Users\xxx\Desktop\some-project\src\main.ts
```

绝对路径只有：

> 真正执行文件系统操作时才需要。

这样可以避免：

- 报告不可移植
- 测试脆弱
- Context 泄露本地用户名
- Snapshot 无法复用

---

# 12. Directory Model

建立：

```text
DirectoryInfo
{
    relative_path
    name
    depth
}
```

不需要构建复杂的树数据库。

GUI 可以根据路径关系生成树。

---

# 13. Ignore System

这是 M1 最重要的基础之一。

不要简单写：

```text
if name == "node_modules"
```

应该建立可扩展规则：

```text
IgnoreEngine
```

支持至少：

```text
default rules
.gitignore
user exclusions
```

原则：

> 默认规则 + 项目规则 + 用户规则

最终统一判断：

```text
Ignored / Included
```

---

# 14. `.gitignore`

M1 应支持读取项目 `.gitignore`。

但不要自己重新发明 Git ignore 语法。

优先寻找：

> 可信、维护中的 ignore matcher 实现。

增加依赖之前必须检查：

```text
license
maintenance
platform support
```

并在审计中记录。

如果已经存在合适实现：

> 优先复用。

---

# 15. Ignore Precedence

建立明确规则。

推荐：

```text
System Safety Rules
    ↓
Project Ignore Rules
    ↓
User Ignore Rules
```

最终以最安全、最符合项目实际的策略为准。

但必须记录设计决策到：

```text
docs/architecture.md
```

未来 M1+ 不要再次争论。

---

# 16. Ignore Safety

以下目录默认不要深度扫描：

```text
.git/
node_modules/
target/
bin/
obj/
.venv/
venv/
__pycache__/
dist/
build/
coverage/
.cache/
```

同时对于：

```text
大型二进制
磁盘镜像
视频
音频
压缩包
数据库文件
```

不要读取内容。

M1 只记录：

```text
path
extension
size
```

必要时将其归类为：

```text
binary
generated
asset
archive
database
```

---

# 17. Symlink

必须考虑符号链接。

默认建议：

> 不跟随目录 symlink。

原因：

- 防止循环
- 防止扫描出项目目录之外
- 防止无限增长
- 提高确定性

对于 symlink：

记录：

```text
is_symlink = true
```

必要时报告。

不要自动递归进入 symlink 指向目录。

---

# 18. Permission Errors

真实项目扫描中可能存在：

- 无权限目录
- 文件被占用
- 文件消失
- broken symlink
- transient IO error

不能因为一个文件失败而导致整个项目扫描失败。

设计：

```text
ScanError
{
    path
    kind
    message
}
```

扫描结束以后：

```text
errors_count
errors[]
```

并允许：

```text
scan completed with warnings
```

而不是简单：

```text
FAILED
```

---

# 19. Scan Error Policy

区分：

### Fatal

例如：

```text
项目根目录不存在
项目根目录无法访问
基础 scanner 初始化失败
```

→ scan failed

### Non-fatal

例如：

```text
某文件无法读取 metadata
某目录没有权限
某个 symlink broken
```

→ scan completed with warnings

必须在代码层区分。

---

# 20. Language Detection

M1 只做：

> 文件扩展名 → language

不要解析源码内容。

例如：

```text
.ts   → TypeScript
.tsx  → TypeScript React
.js   → JavaScript
.jsx  → JavaScript React
.py   → Python
.cs   → C#
.rs   → Rust
.go   → Go
.java → Java
.cpp  → C++
.c    → C
```

语言 registry 要可扩展：

```text
LanguageRegistry
```

不要把几百种语言硬编码散落在 scanner 中。

---

# 21. Unknown Files

不能简单：

```text
unknown → error
```

未知扩展名必须允许存在。

例如：

```text
LICENSE
Dockerfile
Makefile
.editorconfig
```

都可能没有传统扩展名。

因此需要：

```text
FileKind
```

例如：

```text
Source
Config
Documentation
Asset
Binary
Generated
Test
Unknown
```

语言和 FileKind 是两个不同概念。

---

# 22. Generated Files

M1 可以基于路径规则识别常见 generated 内容：

```text
dist/
build/
generated/
coverage/
```

但：

> 不要因为名字叫 `generated` 就永远认为它一定是生成代码。

采用：

```text
classification
+
reason
```

例如：

```text
kind = Generated
reason = "default generated directory"
```

未来 Health 阶段可以继续利用。

---

# 23. Test File Detection

M1 可以识别：

```text
*.test.*
*.spec.*
test/
tests/
__tests__/
```

但是只做：

```text
is_test_file
```

不要分析测试内容。

---

# 24. Configuration Detection

建立：

```text
ConfigurationInfo
```

至少：

```text
name
path
type
confidence
```

例如：

```text
package.json
type = NodeProjectManifest
confidence = High
```

或者：

```text
vite.config.ts
type = ViteConfiguration
confidence = High
```

M1 重点是：

> configuration discovery

而不是：

> configuration interpretation

---

# 25. Project Type Detection

不要直接：

```text
package.json = Node
```

因为现代项目可能同时包含：

```text
Node + Rust
Node + Python
Node + Tauri
.NET + TypeScript
Python + React
```

因此：

```text
ProjectType
```

应该允许多个结果：

```text
[
    Node,
    TypeScript,
    Tauri
]
```

并带：

```text
confidence
evidence[]
```

例如：

```text
Tauri
confidence: high
evidence:
- src-tauri/ exists
- tauri.conf.json exists
```

---

# 26. Project Type Detection 规则

建立：

```text
ProjectTypeDetector
```

采用：

```text
Evidence
   ↓
Rules
   ↓
Candidates
   ↓
Confidence
```

不要使用 AI。

例如：

### Rust

```text
Cargo.toml exists
```

→ Rust candidate

### Python

```text
pyproject.toml
```

→ Python candidate

### .NET

```text
*.csproj
*.sln
```

→ .NET candidate

### Go

```text
go.mod
```

→ Go candidate

### Node

```text
package.json
```

→ Node candidate

---

# 27. Evidence Model

这是 Prism 非常重要的设计原则。

M1 的检测结果必须尽可能回答：

> **为什么你认为它是这个项目类型？**

因此建立：

```text
Evidence
{
    source
    path
    reason
}
```

例如：

```text
ProjectType = Tauri

Evidence:
- src-tauri/ directory found
- tauri.conf.json found
```

未来 M2/M3/Health 都可以复用。

---

# 28. Entry Point Detection

M1 实现的是：

> Candidate Entry Point Detection

不是绝对正确的：

> True Entry Point

因为一个项目可能存在多个入口。

建立：

```text
EntryPoint
{
    path
    type
    confidence
    evidence[]
}
```

---

# 29. Entry Point 规则

先支持常见模式。

例如：

### Node

```text
package.json
main
bin
exports
```

### Vite

```text
index.html
src/main.*
src/main.ts
src/main.tsx
```

### React

```text
src/main.*
src/index.*
```

### Python

```text
main.py
app.py
__main__.py
```

### Rust

```text
src/main.rs
src/lib.rs
```

注意：

```text
src/lib.rs
```

更准确属于：

> library entry candidate

不要把所有入口统一叫 `main entry`.

---

# 30. Entry Point Confidence

至少：

```text
High
Medium
Low
```

例如：

```text
src/main.ts
confidence = High
reason = conventional frontend entry
```

未来允许多个候选：

```text
main.rs
lib.rs
cli.rs
```

不要强行选择唯一入口。

---

# 31. Statistics

建立：

```text
ProjectStatistics
```

至少统计：

```text
total_files
total_directories
total_size_bytes
source_files
test_files
config_files
documentation_files
asset_files
binary_files
ignored_files
error_count
```

语言统计：

```text
TypeScript: 125
Python: 37
C#: 82
```

后续可以扩展：

```text
total_lines
```

但注意：

> M1 不强制进行完整源码行数统计。

因为这会导致：

- 大文件读取
- binary detection
- encoding handling
- performance issues

可以把 LOC 推迟到 M2 或更晚。

---

# 32. Binary Detection

不要简单依赖扩展名。

可以使用：

- extension
- known binary extension
- file signature / lightweight detection

但 M1 不需要复杂 MIME engine。

原则：

> 不要为了识别 binary 而读取整个文件。

最多读取极少量 metadata / header。

---

# 33. File Size Safety

M1 不应该加载整个源码文件进内存。

扫描阶段只需要：

```text
metadata
path
extension
size
```

避免：

```text
read_to_string()
```

整个项目一次性加载。

---

# 34. Encoding

M1 不需要完整解析源码。

因此：

> 不要求识别所有文本编码。

如果某些未来逻辑需要判断文件类型：

优先：

```text
UTF-8
```

其他编码只记录：

```text
unknown / unsupported
```

不要为了 M1 加入复杂编码转换系统。

---

# 35. Incremental Scan Preparation

M0 已经有 Task 基础。

M1 建立：

```text
ScanOptions
```

例如：

```text
root_path
follow_symlinks
respect_gitignore
include_hidden
max_file_size
```

但 M1 不实现真正的：

> incremental scan cache

只要接口不要堵死未来 M2/M11 的扩展即可。

---

# 36. Scan Options

默认建议：

```text
follow_symlinks = false
respect_gitignore = true
include_hidden = true
```

注意：

> hidden ≠ ignored

例如：

```text
.gitignore
.editorconfig
.env.example
```

可能应该被发现。

而：

```text
.git/
```

默认应被忽略。

---

# 37. `.git` 的特殊处理

即使：

```text
.git/
```

不进入扫描：

M1 仍然可以记录：

```text
is_git_repository = true
```

但不要调用 Git CLI。

最小检测：

```text
root/.git exists
```

仅表示：

> 该目录看起来是 Git repository。

不要在 M1 判断：

- branch
- commit
- clean/dirty
- remote
- history

---

# 38. GUI Flow

M1 GUI 至少实现：

```text
Overview
    ↓
Open Project
    ↓
Select Directory
    ↓
Scanning...
    ↓
Discovery Result
```

---

# 39. Open Project UX

Overview 页面提供：

```text
Open Project
```

点击后选择目录。

选择以后：

```text
Project root
```

进入扫描。

扫描过程显示：

```text
Scanning project...

directories: 123
files: 1245
ignored: 481
errors: 2
```

数据必须来自真实 scanner。

禁止假进度。

---

# 40. Progress

扫描过程中进度可以：

### 如果不知道 total

使用：

```text
indeterminate progress
```

然后显示实时统计：

```text
Files discovered
Directories discovered
Ignored
Errors
```

不要伪造：

```text
73%
```

除非 total 有可靠依据。

---

# 41. Cancel

M1 应真正接入 M0 的 cancellation。

用户点击：

```text
Cancel
```

应该：

```text
request cancellation
↓
scanner periodically checks token
↓
stop
↓
state = Cancelled
```

不能只是隐藏 UI。

---

# 42. Scan Result GUI

扫描成功以后，Overview 至少展示：

```text
Project Name
Root
Project Types
Languages
Files
Directories
Size
Tests
Configs
Entry Points
Warnings
```

---

# 43. Directory Tree

增加一个基础：

```text
Project Tree
```

展示：

```text
src/
├─ components/
├─ services/
├─ main.ts
└─ app.tsx

tests/
README.md
package.json
```

只显示真实发现结果。

不要做：

- 文件内容预览
- symbol tree
- dependency graph

---

# 44. File List

可以提供：

```text
Files
```

表格：

```text
Path
Kind
Language
Size
```

支持最基础：

```text
sort
search
filter
```

如果实现这些功能很轻量，可以做。

如果导致明显范围膨胀：

> 保留最小展示即可。

---

# 45. Language Summary

例如：

```text
Languages

TypeScript   62
Python       18
CSS          12
JSON          8
Markdown      6
```

这是：

> 文件数量统计

不是：

> 代码占比分析

暂时不要做代码行占比。

---

# 46. Project Type Summary

例如：

```text
Detected Project Types

Tauri        High
React        High
TypeScript   High
Node         High
```

点击某个项目类型，可以查看：

```text
Evidence
```

例如：

```text
Tauri
Evidence:
- src-tauri/
- tauri.conf.json
```

---

# 47. Entry Points UI

显示：

```text
Candidate Entry Points
```

例如：

```text
src/main.ts        High
src/App.tsx        Medium
```

同时显示：

```text
reason
```

避免制造“Prism 已经确定真正入口”的误解。

---

# 48. Error UI

如果 scanner 遇到非致命错误：

显示：

```text
Completed with warnings
```

例如：

```text
2 files could not be inspected.
```

允许查看：

```text
path
error
```

不要让一个错误直接把整个界面变成：

```text
Scan failed
```

---

# 49. CLI

M1 正式实现：

```bash
prism scan <path>
```

至少支持：

```bash
prism scan .
```

结果默认 human-readable：

```text
Project: Prism
Root: .
TypeScript files: 126
Rust files: 48
Directories: 72
Files: 491
Ignored: 203
Entry points: 3
Warnings: 0
```

---

# 50. JSON Output

CLI 支持：

```bash
prism scan . --format json
```

或者等价设计。

必须输出：

> ProjectDiscovery JSON

不要输出 GUI 私有数据。

这要求：

```text
CLI
GUI
Report
```

共享相同 domain model。

---

# 51. Machine Readability

JSON 输出必须：

- 稳定
- 结构清晰
- deterministic
- 不包含 GUI layout
- 不包含颜色控制
- 不包含 ANSI escape
- 路径尽量使用 relative path

---

# 52. Determinism

相同项目、相同配置、相同扫描规则下：

> Discovery Result 的结构应该尽可能稳定。

目录和文件排序必须明确。

建议：

```text
directories: lexical order
files: lexical order
languages: deterministic order
entry_points: deterministic order
project_types: confidence then name
```

不要依赖 OS filesystem iteration 顺序。

---

# 53. Performance

M1 必须考虑大型项目。

禁止：

```text
一次性把所有文件加载到内存
```

推荐：

```text
directory traversal
    ↓
metadata
    ↓
classification
    ↓
append result
```

必要时逐步产生 progress。

---

# 54. Concurrency

M1 可以使用适度并发。

但是不要过度优化。

建议：

> directory traversal 保持简单，metadata/classification 可以并发。

不要因为“以后大型项目”直接建立复杂 worker pool。

先保证：

```text
correctness
cancellation
determinism
```

再考虑性能。

---

# 55. Cache

M1：

> 不实现正式磁盘 cache。

允许：

```text
in-memory state
```

但不要创建：

```text
.cache/
database
SQLite
binary cache format
```

这些应该等真正出现性能需求后再决定。

---

# 56. Persistence

M1 可以暂时：

> 内存保存当前 ProjectDiscovery。

关闭 Prism 后结果可以消失。

M11 才正式做 Snapshot persistence。

---

# 57. Security / Privacy

扫描器必须：

> 默认只访问用户明确打开的项目 root。

不要因为发现：

```text
symlink
.git
environment variable
```

而自动跳到项目外部。

不要：

```text
network request
```

不要把源码内容写日志。

不要上传扫描结果。

---

# 58. 特殊文件

以下类型原则上只做 metadata：

```text
.exe
.dll
.so
.dylib
.zip
.tar
.gz
7z
iso
png
jpg
jpeg
gif
webp
mp4
mov
mp3
wav
sqlite
db
```

不要分析内容。

---

# 59. Large Files

增加：

```text
max_file_size
```

例如默认可以设置一个合理阈值。

但注意：

> 超过限制的文件不能简单从结果中消失。

应该：

```text
discovered = true
analyzable = false
reason = too_large
```

这样 Prism 仍然知道：

> 项目存在这个文件。

---

# 60. File Classification

M1 推荐建立：

```text
FileClassification
```

至少：

```text
Source
Test
Config
Documentation
Asset
Generated
Binary
Unknown
```

分类应带：

```text
confidence
reason
```

例如：

```text
src/foo.ts
kind = Source
reason = TypeScript source extension
```

---

# 61. 不要把 Classification 与 Language 混淆

例如：

```text
package.json
```

可能：

```text
language = JSON
kind = Config
```

例如：

```text
foo.test.ts
```

可能：

```text
language = TypeScript
kind = Test
```

例如：

```text
README.md
```

可能：

```text
language = Markdown
kind = Documentation
```

---

# 62. Hidden Files

不要默认排除所有 hidden files。

例如：

```text
.gitignore
.env.example
.eslintrc
.prettierrc
```

很多都是项目重要上下文。

原则：

> hidden 不等于 irrelevant。

---

# 63. README / AGENTS / CLAUDE

M1 只做：

> 文件发现与分类。

例如：

```text
README.md
AGENTS.md
CLAUDE.md
```

归类：

```text
Documentation
```

并可以在 Discovery Result 中列为：

```text
Project Documents
```

但：

> 不解析其内容。

解析放 M10 Context Health。

---

# 64. Tests

建立专门 fixture 覆盖：

### Basic

```text
普通 TS 项目
```

### Multi-language

```text
TS + Rust
```

### Ignore

```text
node_modules
target
.gitignore
```

### Broken filesystem

如果测试框架可行：

```text
permission / missing path
```

### Symlink

测试：

```text
no recursive follow
```

### Unknown file

### Large file

### Test file

### Config discovery

### Project type detection

### Entry point detection

---

# 65. Minimum Test Matrix

至少覆盖：

```text
1. scan empty project
2. scan basic project
3. scan nested directories
4. ignore default directories
5. respect .gitignore
6. detect languages
7. detect file kinds
8. detect project types
9. detect configuration files
10. detect candidate entry points
11. handle symlink safely
12. handle scan errors
13. cancellation works
14. deterministic ordering
15. JSON serialization works
```

---

# 66. Testing Principle

不要只测试：

```text
happy path
```

必须覆盖：

```text
real-world filesystem problems
```

因为：

> Prism 的扫描器本质上是 filesystem infrastructure。

---

# 67. Frontend State

M1 的前端状态至少需要表达：

```text
NoProject
Selecting
Scanning
Completed
CompletedWithWarnings
Failed
Cancelled
```

不要把 scanner state 散落在多个 React component。

推荐使用：

> M0 已确定的 state architecture。

如果 M0 没有正式 state layer：

建立一个小而清晰的：

```text
ProjectStore
```

即可。

不要为了 M1 引入大型状态管理框架。

---

# 68. Backend / Frontend API

推荐：

```text
open_project(path)
start_scan(project_id, options)
cancel_scan(task_id)
get_scan_status(task_id)
get_discovery(project_id)
```

但：

> 不一定要求全部实时 streaming。

可以根据 M0 实际 IPC 能力选择。

如果实现 streaming：

优先：

```text
ScanProgressEvent
```

而不是 frontend 自己轮询 filesystem。

---

# 69. Separation of Responsibilities

明确：

### Scanner

负责：

```text
filesystem traversal
```

### IgnoreEngine

负责：

```text
should ignore?
```

### Classifier

负责：

```text
what kind of file?
```

### LanguageRegistry

负责：

```text
what language?
```

### ProjectTypeDetector

负责：

```text
what project technologies?
```

### EntryPointDetector

负责：

```text
what entry candidates?
```

### Statistics

负责：

```text
what statistics?
```

不要让 `scanner.rs` 变成：

> 1500 行 God Object。

---

# 70. Recommended Pipeline

建议：

```text
Project Root
    ↓
Filesystem Walker
    ↓
Ignore Engine
    ↓
File Metadata
    ↓
File Classification
    ↓
Language Detection
    ↓
Configuration Detection
    ↓
Project Type Detection
    ↓
Entry Point Detection
    ↓
Statistics
    ↓
ProjectDiscovery
```

Project Type / Entry Point 可以在扫描完成后基于 discovered files 进行。

这样职责更加清晰。

---

# 71. Result Model

推荐：

```text
ProjectDiscovery
├─ project
├─ scan_info
├─ files[]
├─ directories[]
├─ configurations[]
├─ project_types[]
├─ entry_points[]
├─ statistics
└─ warnings[]
```

未来 M2 可以在其上继续添加：

```text
symbols[]
modules[]
```

但不要现在添加。

---

# 72. Versioning

M1 输出 JSON 时考虑 schema version。

例如：

```json
{
  "schema_version": 1
}
```

因为未来：

```text
M2
M3
M10
```

可能修改模型。

不要让 JSON contract 无版本。

---

# 73. Schema Stability

内部 Rust struct 可以变化。

但对：

```text
CLI JSON
Report
IPC
```

暴露的数据模型需要明确。

建议区分：

```text
Internal Domain Model
External DTO
```

但：

> 只有在确实有必要时才增加 DTO 层。

不要为了架构漂亮而重复定义几十个 struct。

---

# 74. GUI 不要追求“假 Dashboard”

M1 Overview 可以有：

```text
Project
Technology
Structure
Files
Entry Points
Documents
```

不要做：

```text
Health: 98
Architecture: A+
Code Quality: Excellent
Risk: Low
```

因为这些在 M1 根本没有事实依据。

---

# 75. Empty State

没有项目时：

```text
No project opened.

Open a local project to begin.
```

不要默认打开 Prism 自己。

用户选择项目后才开始扫描。

---

# 76. Scan Progress UI

显示真实信息：

```text
Scanning...

Directories    128
Files          1,482
Ignored        623
Warnings       2
```

完成：

```text
Scan completed
```

或者：

```text
Scan completed with warnings
```

---

# 77. Error Detail

错误必须允许追踪到：

```text
path
operation
error
```

例如：

```text
tests/data/large.db
operation: metadata
error: permission denied
```

但 GUI 可以简化呈现。

JSON 应保留机器可读信息。

---

# 78. CLI Error Exit Code

CLI 必须区分：

```text
successful scan
successful scan with warnings
invalid input
fatal scan failure
cancelled
```

不要所有情况都 exit 0。

具体 exit code 按项目统一规范设计，并写入：

```text
docs/development.md
```

---

# 79. Cross-platform

M1 要避免：

```text
Windows-specific separators
Linux-only commands
macOS-only assumptions
```

路径一律由 Rust Path API 处理。

例如：

```text
PathBuf
Path
```

而不是手工拼：

```text
"C:\\..."
```

---

# 80. Performance Acceptance

至少用一个中等规模项目进行真实测试。

不要只测试：

```text
fixture-basic
```

建议至少：

```text
Prism 自己
```

或者一个真实的中等规模开源项目。

记录：

```text
files
directories
scan duration
warnings
memory issues
```

不要设一个没有依据的“必须 < 1 秒”。

重点是：

> 扫描过程中 UI 不冻结。

---

# 81. Self-scan

M1 完成后：

> Prism 必须能够扫描 Prism 自己。

这是重要验收。

例如：

```bash
prism scan .
```

应该得到：

```text
Rust
TypeScript
HTML
CSS
Markdown
JSON
...
```

并识别：

```text
Cargo.toml
package.json
src-tauri/
src/
README.md
```

---

# 82. M1 的最终用户体验

用户第一次打开 Prism：

```text
Prism
    ↓
Open Project
    ↓
选择项目
    ↓
Scanning
    ↓
Overview
```

此时用户应该已经能够直观看到：

> **Prism 已经真的理解这个项目的“外壳与组成结构”。**

但不会误以为 Prism 已经理解：

> 代码逻辑、架构关系、Git 变化和风险。

---

# 83. Documentation Updates

M1 完成后更新：

```text
README.md
docs/architecture.md
docs/development.md
docs/roadmap.md
```

尤其记录：

## Discovery Pipeline

```text
Walker
→ Ignore
→ Metadata
→ Classification
→ Language
→ Config
→ Project Type
→ Entry Point
→ Statistics
```

以及：

## Explicitly Deferred

```text
AST
Symbols
Dependencies
Git
Health
Verify
Context
Snapshot history
```

---

# 84. Dependency Policy

如需新增第三方依赖：

先检查：

```text
Name
Version
License
Repository
Maintenance
Transitive dependencies
```

输出：

```text
Dependency Decision
```

不要看到一个需求就直接：

```text
cargo add xxx
npm install xxx
```

---

# 85. 不允许的实现方式

禁止：

### 全项目一次性读取

```text
read all files
```

### 一个 Scanner 做所有事情

```text
Scanner {
    scan
    classify
    detect_language
    detect_project
    detect_entry
    statistics
}
```

这种实现会导致 M2/M3 快速膨胀。

### AI 检测项目类型

M1 完全不需要。

### Shell 拼接路径

不要：

```text
format!("{}\\{}", ...)
```

### 忽略错误

不要：

```rust
let _ = ...
```

然后把真实错误吞掉。

---

# 86. M1 完成后的数据边界

最终数据结构应该让后续阶段自然接入：

```text
M1

Project
 ├─ Files
 ├─ Directories
 ├─ Languages
 ├─ Project Types
 ├─ Configurations
 ├─ Entry Points
 └─ Statistics

M2

+ Symbols
+ Modules

M3

+ Dependencies
+ Architecture

M4

+ Git

M5

+ ChangeSet

...
```

不要让 M1 的 File model 以后无法扩展。

---

# 87. M1 Acceptance Criteria

M1 必须满足：

```text
[ ] 用户可以打开本地项目目录
[ ] 项目根路径正确
[ ] 项目名称正确
[ ] 文件递归扫描正常
[ ] 目录递归扫描正常
[ ] 默认 ignore 正常
[ ] .gitignore 正常
[ ] symlink 不会造成递归
[ ] metadata 正常
[ ] language detection 正常
[ ] file classification 正常
[ ] configuration detection 正常
[ ] project type detection 正常
[ ] project type 有 evidence
[ ] entry point detection 正常
[ ] entry point 有 confidence
[ ] statistics 正确
[ ] warnings 可追踪
[ ] cancellation 正常
[ ] UI 不冻结
[ ] CLI scan 正常
[ ] JSON output 正常
[ ] 输出 deterministic
[ ] Prism 可以扫描自己
[ ] fixture tests 通过
[ ] cargo test 通过
[ ] frontend tests/typecheck/build 通过
[ ] 无 telemetry
[ ] 无 network dependency
[ ] 无明显 architectural violation
```

---

# 88. M1 Final Audit

完成以后不要直接报告：

> M1 complete.

先执行完整审计。

输出：

## A. Baseline

```text
M0 HEAD:
M1 HEAD:
working tree:
```

## B. Architecture

检查：

```text
scanner
ignore
classifier
language
project type
configuration
entry point
statistics
```

职责是否清晰。

## C. Discovery Accuracy

至少使用：

```text
Prism itself
fixture-basic
multi-language fixture
```

验证：

```text
files
directories
languages
configs
project types
entry points
statistics
```

## D. Ignore Accuracy

明确：

```text
included
ignored
symlink
generated
binary
large file
```

## E. Error Handling

检查：

```text
fatal
non-fatal
cancelled
```

## F. Determinism

同一个项目至少扫描两次：

```text
Result A
Result B
```

确认结构性结果一致。

## G. Performance

记录：

```text
project
files
directories
duration
```

## H. CLI

验证：

```bash
prism scan .
prism scan . --format json
```

## I. GUI

验证：

```text
open project
scan
cancel
completion
warnings
tree
file list
languages
project types
entry points
```

## J. Privacy

确认：

```text
no telemetry
no upload
no network
no source logging
no secret leakage
```

## K. Git Hygiene

检查：

```text
git diff
git status
untracked files
generated files
secret scan
```

## L. Risks

只输出：

```text
P0
P1
```

不要制造虚假风险。

## M. Deferred

明确没有做：

```text
M2 AST / Symbols
M3 Dependency / Architecture
M4 Git analysis
M7 Health
M8 Verify
M10 Context
M11 History
```

---

# 89. FACT / HYPOTHESIS / INFERENCE

审计必须继续遵循：

```text
FACT
HYPOTHESIS
INFERENCE
```

例如：

```text
FACT:
Prism self-scan discovered 1,248 files.

FACT:
node_modules was excluded.

FACT:
same project scanned twice produced deterministic ordering.

INFERENCE:
M1 discovery layer is ready for M2.

HYPOTHESIS:
very large monorepos may require persistent incremental caching later.
```

不要把：

> “未来可能需要”

写成：

> “当前已经存在问题”。

---

# 90. P0 / P1

判断标准：

### P0

阻碍 M2：

- discovery model 错误
- filesystem traversal 不可靠
- ignore system 错误
- deterministic output 无法保证
- Core boundary 被破坏
- scan 经常崩溃

### P1

当前可以继续，但会明显影响后续：

- 某类 project type detection 不完整
- 某类 entry point detection 缺失
- GUI 信息组织不足
- 某些边缘文件类型处理不足

不要把：

```text
“以后可以支持更多语言”
```

自动定义成 P1。

---

# 91. Commit

完成并通过 Final Audit 后：

创建本地 commit。

建议：

```text
feat: add project discovery
```

如果本项目已有 Conventional Commits 规范：

> 遵循现有规范。

---

# 92. 最终交付要求

最终回复必须包含：

```text
# M1 Final Audit

A. Baseline
B. Architecture
C. Discovery
D. Ignore
E. Detection
F. Entry Points
G. Statistics
H. Tests
I. CLI
J. GUI
K. Performance
L. Privacy
M. Git
N. Risks
O. Deferred
```

每项只写真实情况。

不要：

- 夸大
- 猜测
- 用“应该”
- 用“理论上”
- 用“基本完成”代替测试结果

所有测试必须给出真实命令和真实结果。

---

# 93. 最终判断

只有满足：

```text
M1 = COMPLETE
```

才能进入：

# M2 — Code Understanding

M2 的输入将是 M1 建立的：

```text
Project
Files
Directories
Languages
Configurations
ProjectTypes
EntryPoints
Statistics
```

M2 再基于这些事实进一步建立：

```text
Symbols
Functions
Classes
Methods
Modules
```

因此：

> **M1 的核心不是“把文件列出来”，而是建立 Prism 后续代码理解系统可信赖的 Project Discovery Layer。**

严格遵守：

> **Facts First**
>
> **Deterministic First**
>
> **Local First**
>
> **Evidence First**
>
> **Minimal Abstraction**
>
> **No Premature AI**
>
> **No Premature Architecture Analysis**

现在开始：

```text
1. Baseline Audit
2. 修复必要的 M0 阻塞问题
3. 实现 M1
4. 测试
5. 自扫描
6. Final Audit
7. Commit
```

默认：

```text
DO NOT PUSH
```

不要向我重复索要已经明确给出的信息；除非出现真正阻塞，否则直接做合理的工程决策并在最终审计中记录。