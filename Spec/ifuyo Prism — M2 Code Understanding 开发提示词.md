# ifuyo Prism — M2 Code Understanding

## 0. 任务定位

你现在开始实现 **ifuyo Prism — M2 Code Understanding**。

M0 已建立基础工程、Core、任务系统、CLI、GUI Shell、错误模型、日志、CommandRunner、FileSystem 等基础设施。

M1 已实现 Project Discovery：

> 打开项目 → 扫描项目 → 识别目录 / 文件 / 语言 / 配置 / 项目类型 / 候选入口点 → 得到 Discovery Result

M2 的目标是继续向下深入：

> 文件 → 源代码 → 语法结构 → Symbol → Symbol Tree → Project Code Model

M2 完成后，Prism 应该能够回答：

> “这个项目里有哪些代码文件？”  
> “这些代码文件里有哪些类、函数、方法、接口、枚举、变量等？”  
> “这些 Symbol 在哪里定义？”  
> “它们属于哪个文件 / 父级 Symbol？”  
> “代码文件能否成功解析？”  
> “哪些文件解析失败？失败原因是什么？”

M2 **不负责解释架构、不负责建立完整依赖图、不负责判断代码健康度，也不负责 Git Change Impact**。

---

# 1. M2 核心目标

M2 必须建立一套：

> **语言无关的 Code Understanding Domain Model + 可替换的 Parser / Symbol Extraction 层**

最终数据流：

```text
Project
  ↓
ProjectDiscoveryResult (M1)
  ↓
Source Files
  ↓
Language Detection (M1)
  ↓
Parser Registry
  ↓
Language Parser
  ↓
Syntax / Parse Result
  ↓
Symbol Extractor
  ↓
Normalized Symbol Model
  ↓
Project Code Model
```

核心原则：

```text
Parser implementation ≠ Prism domain model

Language-specific syntax
        ↓
Language Adapter
        ↓
Normalized Prism Symbol Model
```

不要让 TypeScript AST / Python AST / C# AST 直接污染整个 Prism Core。

---

# 2. 开发前：必须先做基线审计

开始修改代码前，先对当前仓库做完整基线审计。

要求：

### A. Git Baseline

确认：

- 当前 branch
- 当前 HEAD
- working tree
- untracked files
- M0/M1 commits
- 是否存在未提交修改
- 是否存在临时测试文件
- 是否存在 build artifacts

不得假设之前的状态仍然成立。

### B. M0 基础设施

检查：

- `prism-core`
- Task System
- Cancellation
- Error Model
- Logging
- FileSystem
- CommandRunner
- Config
- IDs
- serialization
- CLI
- Tauri IPC
- Frontend shell

### C. M1 真实实现

检查：

- Project
- ProjectDiscovery
- FileInfo
- DirectoryInfo
- LanguageRegistry
- IgnoreEngine
- ProjectTypeDetector
- EntryPoint
- Evidence
- ProjectStatistics
- ScanOptions
- scan task
- cancellation
- CLI scan
- GUI discovery
- JSON serialization

### D. 重要原则

不要因为 M2 需要某个能力就直接重写 M1。

优先：

```text
复用
→ 扩展
→ 最小重构
→ 最后才重写
```

若发现 M1 存在阻碍 M2 的结构问题：

1. 明确记录 FACT
2. 说明影响
3. 做最小必要修复
4. 不顺便“顺手优化”整个项目

---

# 3. M2 明确边界

## M2 必须做

### P0

实现：

1. Code File Identification
2. Parser abstraction
3. Parser registry
4. Language adapters
5. Parse result
6. Symbol extraction
7. Unified Symbol model
8. Symbol hierarchy
9. Source location / range
10. Parse diagnostics
11. Per-file partial failure handling
12. Project Code Model
13. CLI inspect
14. GUI Understand 基础实现
15. JSON output
16. deterministic output
17. cancellation
18. bounded parallel parsing
19. fixture projects
20. self-scan Prism

---

## M2 不做

明确禁止进入以下功能：

### 不做完整依赖分析

例如：

```text
A imports B
B imports C
A indirectly depends on C
```

这是 M3。

---

### 不做 Architecture Graph

例如：

```text
UI
 ↓
Service
 ↓
Repository
 ↓
Database
```

这是 M3。

---

### 不做 Change Impact

例如：

```text
修改 UserService
→ Controller A
→ Test B
→ Documentation C
```

这是 M6。

---

### 不做 Git Analysis

M4 / M5 负责。

---

### 不做 Health Analysis

例如：

- complexity warning
- architecture smell
- TODO risk
- duplicated code
- dependency risk

这些属于 M7。

---

### 不做 Verify

例如：

- build
- test
- lint
- typecheck

属于 M8。

---

### 不做 Environment Discovery

属于 M9。

---

### 不做 Context Engine

属于 M10。

---

### 不做 Snapshot / Timeline

属于 M11。

---

### 不做 AI

M2 必须能够在：

> 无 API Key  
> 无网络  
> 无 LLM

的情况下完整运行。

---

# 4. 设计原则

## 4.1 Deterministic First

同样项目、同样代码、同样配置：

```text
输入相同
→ 输出必须稳定
```

不得依赖：

- hash map 随机顺序
- OS directory enumeration 顺序
- parser nondeterministic output
- timestamp 参与业务结果
- AI 判断

所有输出应按照稳定规则排序。

---

# 5. Code File Model

M1 已经知道哪些文件属于 Source。

M2 不需要重新扫描项目。

应该复用：

```text
ProjectDiscoveryResult
```

选择：

```text
FileClassification::Source
```

且满足：

```text
analyzable == true
```

的文件。

例如：

```text
src/main.ts
src/App.tsx
src/services/api.py
src/UserService.cs
```

进入 Parser Pipeline。

不要解析：

```text
node_modules/
.git/
dist/
bin/
obj/
images/
fonts/
archives/
large binary files
```

---

# 6. Parser Architecture

建立清晰的 Parser 抽象。

推荐概念：

```text
LanguageParser
ParserRegistry
ParseRequest
ParseResult
ParseDiagnostic
SymbolExtractor
```

例如：

```rust
trait LanguageParser {
    fn language(&self) -> LanguageId;

    fn parse(
        &self,
        request: &ParseRequest,
        cancellation: &CancellationToken,
    ) -> Result<ParseResult, PrismError>;
}
```

具体 parser：

```text
TypeScriptParser
JavaScriptParser
PythonParser
CSharpParser
```

后续允许加入：

```text
RustParser
JavaParser
GoParser
CppParser
CParser
```

但 M2 不要求一次全部实现。

---

# 7. Parser Registry

建立：

```text
ParserRegistry
```

作用：

```text
LanguageId
    ↓
Parser
```

例如：

```text
typescript → TypeScriptParser
javascript → JavaScriptParser
python     → PythonParser
csharp     → CSharpParser
```

Parser Registry 必须支持：

- registration
- lookup
- supported languages
- unsupported language detection

不要在业务代码里到处出现：

```rust
match language {
    TypeScript => ...
    Python => ...
    CSharp => ...
}
```

语言分派应集中在 Registry / Adapter 层。

---

# 8. Parser 技术选择

在实现前，对需要引入的 parser 技术做真实技术评估。

至少检查：

- parsing capability
- language coverage
- AST quality
- source range support
- incremental parsing capability
- ecosystem maturity
- Rust integration quality
- maintenance status
- dependency complexity
- license compatibility
- binary/package size impact
- Windows/macOS/Linux compatibility

允许采用：

```text
Tree-sitter
```

或其他成熟 parser。

但是：

**不要为了“统一”强行选择一个明显不适合所有语言的 parser。**

也不要因为某个 parser 能做完整 compiler-level analysis，就把 M2 变成 compiler implementation project。

M2 的目标只是：

> 稳定、可扩展地提取 Prism 当前真正需要的代码结构。

如果不同语言采用不同底层 parser，也完全允许：

```text
Prism Symbol Model
        ↑
 ┌──────┼────────┐
TS Parser Python C#
```

底层实现可以不同。

---

# 9. 不要构建“万能 AST”

这是 M2 非常重要的约束。

不要试图设计：

```text
UniversalAstNode
```

然后把所有语言的 AST 都硬压成：

```text
Node {
    kind,
    children,
    fields...
}
```

并让 Prism 后续所有功能依赖这个巨大抽象。

正确方向：

```text
Language Parser
       ↓
Language-local AST
       ↓
Symbol Extractor
       ↓
Prism normalized Symbol Model
```

Prism 只保存自己真正需要的结构化信息。

---

# 10. Symbol Model

建立统一的：

```text
Symbol
```

建议至少支持：

```text
SymbolId
name
kind
language
file_id
parent_symbol_id
visibility
location
range
signature
documentation
modifiers
attributes
children
```

但不要为了“以后可能用到”堆大量字段。

---

# 11. SymbolId

Symbol 必须拥有稳定的逻辑身份。

不要简单使用：

```text
random UUID
```

作为每次扫描产生的新 ID。

建议根据：

```text
Project
+ relative file path
+ symbol qualified path
+ symbol kind
```

生成稳定 ID。

例如：

```text
src/services/UserService.cs
UserService
UserService.create
UserService.validate
```

保证在代码未发生结构变化时：

```text
同一个 Symbol
→ 同一个 SymbolId
```

这会直接服务未来：

- ChangeSet
- Change Impact
- Snapshot
- Context
- Timeline

但本阶段不要实现这些功能。

---

# 12. SymbolKind

至少支持：

```text
File
Module
Namespace
Class
Struct
Interface
Enum
Function
Method
Constructor
Property
Field
Variable
Constant
Parameter
TypeAlias
Import
Export
Decorator / Attribute
Unknown
```

不要求所有语言都完整支持所有类型。

Language Adapter 可以只映射自身能够可靠识别的类型。

例如 Python 不一定存在：

```text
Interface
```

则不要伪造。

---

# 13. Symbol Visibility

统一抽象：

```text
Public
Private
Protected
Internal
Package
Module
Unknown
```

不同语言没有对应概念时：

```text
Unknown
```

或者由语言 adapter 根据语法做合理映射。

不要为了统一而制造虚假的语义。

---

# 14. Source Location

建立统一：

```text
SourceLocation
SourceRange
```

至少支持：

```text
byte_offset
line
column
```

以及：

```text
start
end
```

例如：

```json
{
  "start": {
    "line": 12,
    "column": 5,
    "byte_offset": 183
  },
  "end": {
    "line": 21,
    "column": 6,
    "byte_offset": 421
  }
}
```

注意：

**必须明确 offset 的语义。**

不得出现：

```text
某些 parser 用 byte offset
某些地方偷偷当 char offset
前端又当 UTF-16 offset
```

需要在 core contract 中明确：

> Prism Domain Model 使用的 offset / line / column 定义是什么。

如果 parser 原始坐标系统不同：

```text
Parser Adapter
    ↓
Normalized SourceRange
```

进行转换。

---

# 15. Unicode / 非 ASCII

必须专门测试：

```text
中文变量名
中文注释
emoji
多字节字符
CRLF
LF
混合编码异常
```

尤其检查：

```text
byte offset
character index
UTF-16 column
```

之间是否发生错位。

不要假设：

```text
1 character = 1 byte
```

---

# 16. Symbol Hierarchy

建立：

```text
File
 └── Namespace
      └── Class
           ├── Field
           ├── Property
           ├── Constructor
           └── Method
                ├── Parameter
                └── Local Symbol
```

但这里必须控制深度。

M2 的重点是：

> 稳定识别主要代码结构。

不要求第一版完整提取所有局部变量。

建议优先级：

### P0

```text
Namespace / Module
Class
Struct
Interface
Enum
Function
Method
Constructor
Property
Field
Import
Export
```

### P1

```text
Parameter
Local variable
Constant
Decorator / Attribute
TypeAlias
```

---

# 17. Qualified Name

每个 Symbol 尽可能生成：

```text
qualified_name
```

例如：

```text
MyApp.Services.UserService
MyApp.Services.UserService.Create
```

Python：

```text
app.services.user.UserService.create
```

TypeScript：

```text
services/user.UserService.create
```

不要假装不同语言的 qualified name 语义完全一致。

需要在模型中明确：

```text
display name
qualified name
symbol kind
```

是不同概念。

---

# 18. Signature

对适用语言提取：

```text
function foo(a: string, b: number): boolean
```

但应避免保存完整 source code。

Signature 是结构化理解信息。

目标是：

> 给未来 Understand / Context / Change Impact 使用。

例如：

```json
{
  "name": "createUser",
  "kind": "function",
  "signature": "(name: string, age: number): User"
}
```

---

# 19. Documentation

记录 Symbol 是否存在文档说明：

```text
documentation_present
```

可以进一步保存简短 normalized documentation。

但不要在 M2 做：

- 文档质量评分
- 文档完整度分析
- AI 文档总结

这些属于后续 Health / Context。

---

# 20. Attributes / Annotations / Decorators

允许抽取：

```text
[Serializable]
[HttpGet]
@Override
@decorator
@dataclass
```

但只做结构化事实：

```text
name
location
arguments
```

不要在 M2 推断：

> “这个 decorator 一定意味着 HTTP endpoint。”

这种语义理解属于未来分析层。

---

# 21. Import / Export

M2 可以提取：

```text
Import
Export
Using
Include
Require
```

作为 Symbol / Syntax Fact。

但是：

**M2 不建立完整 Dependency Edge。**

例如：

```text
A.ts
  imports B.ts
```

M2 可以记录：

```text
Import symbol = B
```

但不要在 M2 得出：

```text
A → B
```

的完整依赖关系。

原因：

- alias resolution
- path resolution
- package resolution
- namespace resolution
- re-export
- conditional import

这些属于 M3 Dependency Analysis。

---

# 22. ParseResult

设计统一：

```text
ParseResult
```

至少包含：

```text
file_id
language
success
symbols
diagnostics
duration
parser_version
```

建议区分：

```text
Successful
SuccessfulWithDiagnostics
Failed
Unsupported
Skipped
```

---

# 23. Parse Diagnostics

单个文件解析失败：

**不能让整个项目失败。**

例如：

```text
1000 source files
999 parse successfully
1 file has syntax error
```

结果应该是：

```text
Project understanding = partial success
```

而不是：

```text
整个 Prism inspect 失败
```

建立：

```text
ParseDiagnostic
```

至少支持：

```text
severity
message
location
file
code
```

severity：

```text
Info
Warning
Error
```

---

# 24. 部分解析

对于语法不完整的代码：

```text
unfinished file
merge conflict
generated source
temporary code
AI 正在修改中的代码
```

parser 如果支持 recovery，应尽可能提取已有 Symbol。

例如：

```text
class UserService {
    create() {}
    
    validate(   // syntax error
```

至少应该能够保留：

```text
UserService
UserService.create
```

并报告 diagnostic。

不要追求：

> “任何破坏语法的文件都必须 100% 正确解析。”

更合理：

> 最大化结构信息 + 精确报告解析问题。

---

# 25. Project Code Model

M2 完成后建立：

```text
ProjectCodeModel
```

建议结构：

```text
ProjectCodeModel
├── project
├── source_files
├── symbols
├── symbol_index
├── parse_summary
└── diagnostics
```

至少能快速完成：

```text
file → symbols
symbol → parent
symbol → children
symbol → location
symbol → file
symbol_id → symbol
```

为 M3 / M5 / M6 提供基础。

---

# 26. Symbol Index

不要让后续模块每次都：

```text
遍历整个 Symbol Tree
```

建立基础 index：

```text
SymbolId → Symbol
FileId → Symbols
ParentSymbolId → Children
QualifiedName → Symbols
Name → Symbols
```

注意：

```text
Name
```

可能重复。

例如：

```text
foo()
foo()
```

因此：

```text
name → Vec<SymbolId>
```

而不是：

```text
name → SymbolId
```

---

# 27. File 与 Symbol 的关系

不要把：

```text
File
```

与：

```text
SourceFile
```

重复设计成两个完全独立实体。

优先检查 M1 的 `FileInfo` 是否足够。

如果足够：

```text
FileInfo
   +
CodeAnalysis
```

而不是重新制造一个平行文件模型。

避免未来：

```text
FileInfo
SourceFile
CodeFile
AnalyzedFile
ProjectFile
```

五套文件概念互相转换。

---

# 28. Parser Lifecycle

明确：

```text
Discovery
    ↓
Select analyzable source files
    ↓
Read file
    ↓
Parser
    ↓
Symbol extraction
    ↓
Normalized result
```

文件内容应当：

> 按需读取

不要把整个项目源码永久驻留在内存。

尤其禁止：

```text
scan all
→ read all files
→ keep all source strings
→ parse everything
```

---

# 29. 大文件

继续复用 M1 的：

```text
max_file_size
```

对于超过分析阈值的 source file：

```text
discovered = true
analyzable = false
skip_reason = too_large
```

不能：

> 因为 M2 开始解析，就绕过 M1 限制。

---

# 30. 并发

M2 可以并行 parser。

但必须：

```text
bounded concurrency
```

例如：

```text
N parser tasks
```

而不是：

```text
10000 files
→ spawn 10000 tasks
```

需要：

- cancellation
- progress
- bounded memory
- deterministic aggregation

结果顺序必须与并发执行顺序无关。

最终：

```text
sort by relative path
then symbol location
then symbol kind/name
```

---

# 31. Cancellation

必须复用 M0 Task System。

用户执行：

```text
prism inspect .
```

过程中取消：

```text
queued
→ running
→ cancelled
```

必须：

- 停止继续读取新文件
- 尽可能停止 parser
- 不破坏已有结果
- 正确返回 cancelled 状态
- GUI 正确显示
- CLI 使用正确 exit code

---

# 32. Incremental-friendly

M2 暂时不实现真正磁盘缓存。

但是架构要支持未来：

```text
FileHash
ParserVersion
AnalysisVersion
```

例如：

```text
AnalysisCacheKey =
    file_content_hash
    +
    parser_version
    +
    prism_analysis_version
```

不要现在实现完整 cache database。

只留下清晰接口即可。

真正持久化缓存放到 M13 / 后续性能阶段。

---

# 33. Language Support

M2 第一阶段至少支持：

```text
TypeScript
JavaScript
Python
C#
```

重点不是“语言数量”，而是：

> 4 个语言 adapter 真正跑通。

建议至少覆盖：

### TypeScript / JavaScript

```text
function
class
method
property
variable
interface
type
enum
import
export
arrow function
```

### Python

```text
module
class
function
method
variable
import
from import
decorator
```

### C#

```text
namespace
class
struct
interface
enum
method
constructor
property
field
using
attribute
```

---

# 34. 后续语言

架构允许未来加入：

```text
Rust
Go
Java
C
C++
```

但 M2 不为了未来语言制造过度复杂的 framework。

目标：

```text
新语言 = 新 adapter
```

而不是：

```text
修改 30 个 core 模块才能支持新语言
```

---

# 35. Unsupported Language

如果 M1 识别：

```text
Ruby
PHP
Kotlin
Swift
```

但 M2 没有 parser：

不要报整个项目错误。

显示：

```text
unsupported parser
```

例如：

```json
{
  "path": "src/example.rb",
  "language": "ruby",
  "status": "unsupported",
  "reason": "no_parser"
}
```

---

# 36. GUI — Understand

M1 已有 GUI Shell：

```text
Understand
```

M2 将其变成第一个真正工作的页面。

建议结构：

```text
Understand
────────────────────────

Project
  files: 328
  source files: 172
  parsed: 168
  warnings: 4

Languages
  TypeScript 72%
  Python     18%
  C#         10%

Files
  src/
    components/
      App.tsx
        App
          render
          loadData
    services/
      api.ts
        getUsers
        createUser

Diagnostics
  ⚠ src/legacy/foo.ts
    parser warning
```

---

# 37. GUI Symbol Explorer

实现基本：

```text
File Tree
    ↓
File
    ↓
Symbol Tree
```

支持：

- 展开
- 折叠
- Symbol kind icon
- name
- language
- location
- diagnostics
- symbol selection

点击 Symbol：

显示：

```text
Name
Kind
Language
File
Location
Qualified Name
Signature
Visibility
Parent
Documentation
```

不要在 M2 做复杂 IDE 式源码编辑器。

---

# 38. Source Navigation

M2 可以提供：

```text
Open in Default Editor
```

例如：

```text
UserService.cs:42
```

点击：

> 使用系统默认编辑器打开对应位置。

如果精确跳转实现复杂：

M2 可以先实现：

```text
open file
```

并在 architecture note 中留下：

```text
line/column navigation = future enhancement
```

不要为了这一项引入重型编辑器。

---

# 39. CLI

新增：

```bash
prism inspect <path>
```

至少支持：

```bash
prism inspect .
```

以及：

```bash
prism inspect . --format json
```

建议输出：

```text
Project
Files
Languages
Symbols
Diagnostics
Summary
```

---

# 40. CLI 示例

文本模式：

```text
ifuyo-prism
Code Understanding

Project: my-project
Root: .

Files
  Source: 172
  Parsed: 168
  Unsupported: 2
  Failed: 2

Symbols
  Classes:      24
  Functions:    183
  Methods:      97
  Interfaces:   12
  Enums:        8

Diagnostics
  Errors:       2
  Warnings:     4

Top files
  src/app.ts
    App
    createApp
    initialize

  src/services/user.ts
    UserService
      create
      update
      validate
```

---

# 41. JSON Contract

JSON 必须直接来源于 Domain Model。

不要：

```text
GUI 自己组装 JSON
CLI 自己组装 JSON
```

正确：

```text
Core Domain Model
       ↓
Serializer
   ↙       ↘
GUI       CLI
```

确保：

```text
GUI
CLI
future Report
future Context Engine
```

使用相同事实源。

---

# 42. M2 数据模型建议

可以逐步加入：

```text
SymbolId
SourceLocation
SourceRange

SymbolKind
SymbolVisibility

Symbol
SymbolIndex

ParseStatus
ParseDiagnostic
ParseResult

ProjectCodeModel

LanguageParser
ParserRegistry
ParseRequest
```

必要时加入：

```text
Signature
DocumentationInfo
AttributeInfo
ImportInfo
ExportInfo
```

但保持克制。

---

# 43. Evidence 复用

M1 已存在：

```text
Evidence
```

M2 必须复用。

例如：

```text
Symbol: UserService

Evidence:
  file = src/services/UserService.cs
  range = 15:1 - 62:2
  parser = csharp
```

不要重新定义：

```text
SymbolEvidence
```

除非真实需求证明 M1 的模型不足。

---

# 44. Error Model

继续复用 M0：

```text
PrismError
```

区分：

### 项目级 fatal error

例如：

```text
project path does not exist
cannot access project
```

### 文件级 non-fatal error

例如：

```text
file cannot be read
parser failed
unsupported language
invalid encoding
```

项目仍应尽可能继续分析。

---

# 45. Encoding

需要谨慎处理：

```text
UTF-8
UTF-8 BOM
UTF-16
```

M2 至少不要因为单个文件编码问题导致整个 inspect 失败。

如果不支持某种 encoding：

```text
status = failed
diagnostic = unsupported_encoding
```

不要 silently skip。

---

# 46. Generated Code

M1 已有：

```text
FileClassification
```

M2 要尊重：

```text
Generated
```

对于明显 generated code：

可以默认：

```text
skip_analysis = true
```

或者根据 ScanOptions 控制。

不要在 M2 重写生成文件识别逻辑。

---

# 47. 测试要求

必须建立真实 fixture projects。

至少：

```text
tests/fixtures/
├── basic-typescript/
├── basic-javascript/
├── basic-python/
├── basic-csharp/
├── nested-symbols/
├── syntax-errors/
├── unicode/
├── empty/
├── unsupported-language/
├── generated/
└── large-file/
```

---

# 48. Symbol Tests

测试：

### Basic

```text
function
class
method
property
field
interface
enum
```

### Hierarchy

```text
namespace
 → class
   → method
     → parameter
```

### Location

确认：

```text
line
column
byte offset
start/end
```

一致。

### Unicode

例如：

```python
def 计算价格():
    ...
```

确认 location 正确。

---

# 49. Determinism Tests

同一个 fixture：

执行：

```text
inspect
inspect
inspect
```

比较：

```text
JSON
```

必须一致。

如果存在：

```text
timestamp
duration
runtime-specific fields
```

这些字段不能污染 deterministic business result。

---

# 50. Parse Failure Tests

构造：

```text
valid.ts
broken.ts
valid.py
```

确认：

```text
valid.ts  → parsed
valid.py  → parsed
broken.ts → failed/diagnostic
whole project → partial success
```

---

# 51. Cancellation Tests

构造：

```text
large fixture project
```

启动分析：

```text
running
```

中途 cancel。

确认：

```text
cancelled
```

并且：

- 无线程泄漏
- 无 panic
- GUI 不冻结
- CLI 正确结束
- 临时状态正确清理

---

# 52. Parser Contract Tests

每个 parser 至少通过同一组 contract tests：

```text
supported_language
parse_valid_file
parse_invalid_file
location_mapping
symbol_extraction
determinism
diagnostics
cancellation
```

这样未来添加语言时：

```text
NewLanguageParser
      ↓
same contract
```

而不是重新设计测试体系。

---

# 53. Self Scan

必须运行 Prism 分析 Prism 自己。

例如：

```bash
prism inspect .
```

确认能够识别自身：

```text
Rust symbols
TypeScript symbols
React components
Tauri commands
```

若当前仓库结构不同，以实际检测结果为准。

这是重要的 dogfooding acceptance test。

---

# 54. Performance

M2 不要求极限性能。

但禁止明显错误实现：

```text
每个 Symbol 重读一次文件
每个 Symbol 再 parse 一次
O(files × symbols)
```

应该是：

```text
read once
→ parse once
→ extract once
→ index
```

并尽可能：

```text
bounded parallel
```

记录：

```text
files discovered
files parsed
files failed
symbols extracted
duration
```

---

# 55. 内存控制

禁止：

```text
所有源码全文
+
所有 AST
+
所有 normalized model
```

全部长期保存。

推荐：

```text
Source Text
   ↓
Parser
   ↓
Extract Symbols
   ↓
Discard temporary source/AST
   ↓
Store normalized information
```

以后如果 Context Engine 需要源码片段，再按需读取。

---

# 56. Architecture Boundary

保持：

```text
Frontend
   ↓
Tauri IPC
   ↓
Application / Service
   ↓
Prism Core
   ↓
Parser / Analysis
```

Parser 不允许依赖：

```text
React
Tauri frontend
GUI state
```

Core 不允许依赖：

```text
React
browser APIs
DOM
```

Frontend 不允许直接操作：

```text
filesystem
parser internals
```

---

# 57. 推荐目录结构

根据现有 M0/M1 实际结构调整，不要机械重排。

目标结构类似：

```text
crates/
├── prism-core/
├── prism-project/
├── prism-parser/
│   ├── registry/
│   ├── common/
│   └── languages/
│       ├── typescript/
│       ├── javascript/
│       ├── python/
│       └── csharp/
├── prism-analysis/
└── prism-cli/
```

如果当前项目实际采用单 crate：

不要为了“看起来专业”强制拆 crate。

优先：

> 正确边界 > crate 数量。

---

# 58. Source Generator / Macro

除非真实需要：

不要在 M2 引入：

```text
复杂 procedural macro
自动生成几十个模型
代码生成 framework
```

M2 核心模型应该让 AI 和人都容易阅读。

---

# 59. Logging

日志需要记录：

```text
scan started
parse started
parse completed
parse failed
symbols extracted
inspect completed
inspect cancelled
```

但不得默认记录：

```text
source code
API key
token
.env values
secret contents
```

---

# 60. Security / Privacy

Prism 是：

> local-first project intelligence tool

因此：

默认：

```text
no network
no telemetry
no source upload
no automatic AI calls
```

M2 不新增任何网络能力。

日志和错误信息必须避免泄露：

```text
secret values
credentials
tokens
passwords
private source contents
```

绝对路径只在确有必要时出现。

业务模型优先：

```text
project-relative path
```

---

# 61. AI 不参与 Symbol Truth

必须坚持：

```text
Code
 ↓
Parser
 ↓
Symbol
```

而不是：

```text
Code
 ↓
LLM
 ↓
"我觉得这里是一个 class"
```

AI 未来可以：

```text
解释 Symbol
总结 module
生成 context
解释风险
```

但是：

> Symbol existence / location / kind 是 deterministic fact。

---

# 62. M2 GUI 不做“漂亮化”

允许基础视觉优化，但不要把时间耗在：

```text
动画
粒子
复杂图表
炫酷 graph
```

重点是：

```text
Information density
Navigation
Hierarchy
Readability
Status
Diagnostics
```

M12 再做统一产品级 UI polish。

---

# 63. M2 输出应该回答什么

最终用户打开 Understand：

应该很快知道：

```text
这个项目里有什么代码？
有哪些语言？
有多少代码文件？
哪些文件被解析？
哪些文件失败？
每个文件有哪些 Symbol？
Symbol 在哪里？
Symbol 属于谁？
```

但暂时不能直接回答：

```text
为什么这样设计？
模块之间如何依赖？
修改这个函数影响谁？
项目哪里有架构风险？
README 是否过时？
这个项目现在能不能发布？
```

这些属于：

```text
M3
M6
M7
M10
M8
```

---

# 64. M2 Acceptance Criteria

M2 只有在以下条件全部满足时才算完成。

## P0

### Project

```text
✅ 可以对 M1 discovered source files 进行代码分析
✅ 不重复实现 discovery
```

### Parser

```text
✅ Parser abstraction 完成
✅ Parser Registry 完成
✅ TypeScript 可用
✅ JavaScript 可用
✅ Python 可用
✅ C# 可用
```

### Symbol

```text
✅ Symbol model 完成
✅ SymbolId 稳定
✅ Symbol hierarchy 完成
✅ SourceRange 完成
✅ Visibility 完成
✅ Qualified name 完成
✅ Signature 基础支持
```

### Diagnostics

```text
✅ 文件级 parse error 不会击穿整个项目
✅ diagnostics 可定位
✅ partial success 可表达
```

### Code Model

```text
✅ ProjectCodeModel
✅ SymbolIndex
✅ File → Symbols
✅ Symbol → Parent
✅ Symbol → Children
✅ Symbol → File
```

### CLI

```text
✅ prism inspect <path>
✅ text output
✅ JSON output
✅ correct exit status
```

### GUI

```text
✅ Understand 页面使用真实数据
✅ 文件树
✅ Symbol tree
✅ Symbol details
✅ parser status
✅ diagnostics
```

### Engineering

```text
✅ cancellation
✅ bounded concurrency
✅ deterministic output
✅ tests
✅ self-scan
✅ no AI dependency
✅ no network
```

---

# 65. P1 Acceptance Criteria

完成后再处理：

```text
P1:
- 更完整的局部变量提取
- decorators / attributes 完整抽取
- 更好的 source navigation
- parser-specific diagnostics enrichment
- 更多语言
- 更复杂的 signature normalization
```

如果时间或复杂度不允许：

> 不要为了 P1 推迟 P0 完成。

---

# 66. 禁止事项

本阶段禁止：

```text
❌ 引入 LLM
❌ 引入 embeddings
❌ RAG
❌ MCP
❌ Git dependency graph
❌ Architecture graph
❌ Call graph
❌ Change impact
❌ Health score
❌ AI code modification
❌ 自动修改用户项目
❌ 上传项目
❌ telemetry
❌ cloud backend
❌ database
❌ 全功能 IDE
```

尤其禁止：

> 因为已经拿到了 AST，就顺手实现完整静态分析平台。

M2 只解决：

> **Code Structure Understanding**

---

# 67. 实施策略

按以下顺序执行：

```text
1. Baseline audit
2. Model design
3. Parser abstraction
4. Parser registry
5. First language adapter
6. Second language adapter
7. Third language adapter
8. Fourth language adapter
9. Symbol normalization
10. Symbol index
11. ProjectCodeModel
12. Diagnostics / partial failure
13. CLI inspect
14. GUI Understand
15. cancellation / concurrency
16. fixture tests
17. self-scan
18. final audit
19. local commit
```

每一步：

```text
implement
→ test
→ inspect
→ continue
```

不要一次修改整个仓库然后最后才发现架构问题。

---

# 68. 修改纪律

如果发现已有代码存在问题：

优先：

```text
P0 correctness
→ P0 architecture blocker
→ P1 cleanup
→ deferred
```

不要顺手做：

```text
rename everything
rewrite frontend
rewrite M1
change build system
change UI framework
replace all error types
```

除非它是 M2 的实际 blocker。

---

# 69. 最终 Audit

M2 完成后必须执行：

# M2 Final Audit

## A. Baseline

报告：

```text
FACT
branch:
HEAD:
working tree:
```

---

## B. Architecture

检查：

```text
FACT
Parser dependency direction
Core dependency direction
GUI isolation
Domain model ownership
```

同时：

```text
HYPOTHESIS
```

任何尚未完全确认的架构风险。

---

## C. Parser

报告：

```text
supported languages
unsupported languages
parser implementation
parse success rate
parse failure cases
```

---

## D. Symbols

报告：

```text
symbol kinds
symbol hierarchy
location correctness
stable SymbolId
index integrity
```

---

## E. Diagnostics

报告：

```text
fatal
non-fatal
unsupported
partial success
```

---

## F. Determinism

证明：

```text
same fixture
→ same JSON structure
```

---

## G. Performance

至少报告：

```text
files discovered
files parsed
symbols extracted
duration
peak/problematic behavior if observed
```

不要伪造 benchmark。

---

## H. Tests

至少报告：

```text
unit tests
parser contract tests
fixture tests
integration tests
cancellation tests
determinism tests
self-scan
```

---

## I. CLI

确认：

```text
prism inspect .
prism inspect . --format json
```

真实运行。

---

## J. GUI

确认：

```text
project open
→ inspect
→ progress
→ results
→ symbol tree
→ diagnostics
```

真实运行。

---

## K. Privacy

确认：

```text
no network
no source upload
no telemetry
no secret logging
```

---

## L. Git

确认：

```text
working tree clean
temporary files removed
ignored files correct
no secrets
no generated garbage
```

---

# 70. Audit 输出格式

最终审计严格使用：

```text
# M2 Final Audit

## 1. Baseline

## 2. Architecture

## 3. Parser

## 4. Symbol Model

## 5. Diagnostics

## 6. CLI

## 7. GUI

## 8. Tests

## 9. Performance

## 10. Security / Privacy

## 11. Git

## 12. Risks

### P0
- ...

### P1
- ...

## 13. Deferred

- ...
```

其中：

```text
FACT
HYPOTHESIS
INFERENCE
```

明确区分。

不要把推测写成事实。

---

# 71. Commit

只有在：

```text
tests pass
build pass
CLI pass
GUI pass
self-scan pass
audit complete
working tree clean except intended changes
```

之后进行本地 commit。

推荐：

```text
feat: add code understanding
```

**默认不要 push。**

---

# 72. M2 完成定义

M2 的真正完成状态不是：

> “用了一个 AST parser。”

而是：

```text
Project
   ↓
Files
   ↓
Language
   ↓
Parser
   ↓
Symbols
   ↓
Hierarchy
   ↓
Index
   ↓
ProjectCodeModel
```

整个链路：

```text
稳定
可测试
可取消
可序列化
可复用
可扩展
无 AI 依赖
```

并且为后续：

```text
M3 Dependency & Architecture
M4 Git
M5 ChangeSet
M6 Change Impact
M7 Health
M8 Verify
M10 Context
M11 Snapshot
```

提供稳定的代码事实层。

**最终原则：**

> M1 让 Prism 看见“文件”。  
> M2 让 Prism 看见“代码结构”。  
> M3 才开始让 Prism 看见“代码之间的关系”。