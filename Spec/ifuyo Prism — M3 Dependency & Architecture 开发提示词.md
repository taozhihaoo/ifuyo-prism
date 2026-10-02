# ifuyo Prism — M3 Dependency & Architecture

## 0. 任务定位

你现在进入 **ifuyo Prism M3：Dependency & Architecture**。

不要把本阶段理解成“增加一个依赖图页面”。

M3 的真正目标是：

> **在 M1 文件事实层 + M2 代码事实层之上，建立稳定、可解释、可验证的项目关系层。**

核心链路：

```text
M1 Project Discovery
        ↓
文件 / 项目 / 配置事实
        ↓
M2 Code Understanding
        ↓
Symbol / Import / Namespace / Project Reference
        ↓
M3 Dependency & Architecture
        ↓
依赖关系 / 引用关系 / 模块关系 / 调用关系 / 图结构
        ↓
M6 Change Impact
M7 Health
```

M3 必须坚持：

> **Deterministic First / Evidence First / Resolve When Possible / Explicit Uncertainty**

任何无法可靠解析的关系，都必须显式表示为：

- Resolved
- Ambiguous
- Unresolved
- External
- Builtin
- Ignored

禁止为了让图“完整”而猜测。

---

# 1. 第一原则

## 1.1 M3 回答什么问题

M3 必须能够回答：

```text
这个项目有哪些模块？
模块之间有什么关系？
文件之间依赖什么？
哪些 import / using / reference 被解析到了哪里？
哪些依赖来自外部包？
哪些关系目前无法解析？
有没有循环依赖？
哪些节点被大量依赖？
哪些节点依赖很多其他节点？
项目结构形成了什么客观图？
```

M3 输出的是：

> **关系事实**

不是：

> “这个架构设计得不好”。

例如：

```text
FACT:
A → B
B → C
C → A
cycle detected
```

而不是：

```text
ARCHITECTURE IS BAD
```

后者应该由 M7 Health 根据规则、阈值和证据判断。

---

# 2. 明确边界

## M3 必须做

- Module Model
- Dependency Model
- Graph Model
- Import / Reference Resolution
- Project / Package / Workspace Relationship
- Static Call Relationship（有限范围）
- Re-export / Export Relationship
- External / Builtin Dependency Identification
- Dependency Graph
- Architecture Graph
- Graph Metrics
- Cycle Detection
- Resolution Diagnostics
- GUI Architecture View
- CLI Graph Inspection
- JSON 输出
- 可复现测试

## M3 不做

绝对不要偷偷扩展范围：

- Git
- Git diff
- Change Impact
- Health Score
- Architecture quality judgment
- AI analysis
- LLM
- Embeddings
- RAG
- MCP
- Verify
- Build orchestration
- Environment inspection
- Snapshot
- Persistent cache/database
- IDE/editor
- Debugger
- Runtime tracing
- 动态调用分析
- 完整运行时语义推断
- 自动修改代码

尤其不要把：

> “依赖很多”

直接变成：

> “架构有问题”。

---

# 3. 开始前：完整基线审计

在任何代码修改之前，先审计：

```text
M0 Foundation
M1 Project Discovery
M2 Code Understanding
```

必须检查真实仓库，而不是假设之前阶段完全符合计划。

审计至少包括：

### A. Repository

确认：

- 当前 branch
- HEAD
- working tree
- 最近提交
- 是否存在未提交修改
- M0/M1/M2 是否已经进入当前代码

### B. Architecture

确认：

```text
frontend
   ↓
Tauri IPC
   ↓
application/use-case
   ↓
prism-* core
```

以及：

```text
prism-core
```

是否仍然保持底层独立。

不得为了 M3 绕过既有 abstraction。

### C. M1

确认：

- Project
- FileInfo
- DirectoryInfo
- FileClassification
- Language
- ProjectType
- EntryPoint
- Evidence
- IgnoreEngine
- ScanInfo

等是否可复用。

### D. M2

确认：

- Symbol
- SymbolId
- SymbolIndex
- ProjectCodeModel
- ParserRegistry
- LanguageParser
- ParseResult
- ParseDiagnostic
- Import / Export syntax facts

等是否存在且真实可用。

### E. Test

确认：

- M0/M1/M2 测试
- fixture projects
- CLI
- GUI
- JSON serialization

当前状态。

---

# 4. 先解决架构问题，再开始实现

如果基线审计发现 M0–M2 存在影响 M3 的 P0/P1：

```text
先修复
↓
重新运行相关测试
↓
确认基线恢复
↓
再开始 M3
```

不要建立在明显错误的数据模型上继续堆功能。

---

# 5. M3 核心数据模型

建立稳定的关系模型。

建议至少包含：

```text
Module
Dependency
DependencyEdge
Reference
CallEdge
GraphNode
GraphEdge
DependencyGraph
ArchitectureGraph
ResolutionResult
ResolutionStatus
GraphMetrics
```

---

# 6. Module Model

M3 引入：

```text
Module
```

注意：

> Module ≠ File ≠ Symbol

三者必须保持清晰区别。

建议：

```text
Module {
    id: ModuleId
    name: String
    qualified_name: Option<String>

    kind: ModuleKind

    project_id: ProjectId

    file_ids: Vec<FileId>

    parent_module_id: Option<ModuleId>

    language: Option<Language>

    path: Option<RelativePath>

    evidence: Vec<Evidence>
}
```

`ModuleKind` 根据实际需求设计，例如：

```text
FileModule
Namespace
Package
Project
Workspace
VirtualModule
ExternalPackage
Builtin
Unknown
```

不要为了“统一”而把所有东西强行建模成同一种 Module。

---

# 7. Module Identity

Module ID 必须稳定。

禁止：

```text
Uuid::new_v4()
```

作为唯一关系身份。

应该从稳定事实派生，例如：

```text
ProjectId
+
module kind
+
normalized relative identity
```

形成稳定：

```text
ModuleId
```

例如：

```text
project + src/components/Button.tsx
```

或者：

```text
project + namespace:MyApp.Services
```

同一个项目重复扫描时：

> 相同实体必须得到相同 identity。

这对于后续：

- M6 Change Impact
- M11 Snapshot

非常重要。

---

# 8. Dependency Model

建议：

```text
Dependency {
    id: DependencyId

    source: GraphNodeId
    target: GraphNodeId

    kind: DependencyKind

    resolution: ResolutionStatus
    confidence: Confidence

    evidence: Vec<Evidence>

    location: Option<SourceRange>

    imported_names: Vec<String>

    is_external: bool
}
```

`DependencyKind` 至少考虑：

```text
Import
Export
ReExport
NamespaceReference
TypeReference
ProjectReference
PackageReference
ModuleReference
Call
Inheritance
Implementation
Include
Unknown
```

但不要在当前支持语言无法可靠识别时制造假关系。

---

# 9. Dependency Edge 与 Call Edge 分离

不要让：

```text
A imports B
```

和：

```text
A calls B.foo()
```

完全等价。

建议：

```text
DependencyEdge
```

处理结构依赖：

```text
import
using
reference
project reference
package reference
re-export
```

而：

```text
CallEdge
```

表示：

```text
caller → callee
```

调用关系必须单独建模。

---

# 10. ResolutionStatus

这是 M3 非常重要的模型。

建议：

```text
Resolved
Ambiguous
Unresolved
External
Builtin
Ignored
NotApplicable
```

含义必须明确。

### Resolved

明确解析到了项目内实体。

例如：

```text
./utils
→ src/utils.ts
```

### Ambiguous

存在多个合理候选，不能确定。

例如：

```text
foo
→ candidate A
→ candidate B
```

### Unresolved

已知道存在关系，但当前 resolver 无法解析。

例如：

```text
some-runtime-generated-module
```

### External

确认依赖的是项目外部 package。

例如：

```text
react
requests
System.Text.Json
```

### Builtin

语言 / runtime 内置模块。

### Ignored

依据 M1 IgnoreEngine 或特定项目规则主动排除。

---

# 11. Evidence 必须贯穿关系解析

每条重要关系尽量附：

```text
Evidence {
    source
    kind
    location
    description
}
```

例如：

```text
source_file:
src/App.tsx

syntax:
import { Button } from "./components/Button"

resolved_target:
src/components/Button.tsx
```

或者：

```text
source:
MyApp.csproj

reference:
ProjectReference

target:
Core/Core.csproj
```

目标是：

> 用户不仅看到“这两个节点有关”，还能够知道“Prism 为什么认为它们有关”。

---

# 12. Language Resolver Architecture

不要把解析逻辑全部塞进一个：

```text
DependencyAnalyzer
```

必须建立语言适配边界。

建议：

```text
DependencyResolver
        ↓
ResolverRegistry
        ↓
TypeScriptResolver
JavaScriptResolver
PythonResolver
CSharpResolver
...
```

每个 resolver 负责：

```text
syntax facts
+
language semantics
+
project conventions
+
resolution strategy
```

但最终输出必须转换成统一 Prism Domain Model。

---

# 13. 首批语言

第一优先：

```text
TypeScript
JavaScript
Python
C#
```

因为这些语言已经在 M2 成为第一批支持对象。

第二批可以预留：

```text
Rust
Go
Java
C
C++
```

但不要在 M3 为未来语言写大量空壳代码。

---

# 14. TypeScript / JavaScript Resolution

至少支持：

### 相对 import

```ts
import x from "./utils";
```

解析：

```text
./utils
./utils.ts
./utils.tsx
./utils.js
./utils.jsx
./utils/index.ts
...
```

实际候选集合必须依据项目真实配置和 resolver 规则生成。

### TypeScript paths

识别：

```json
{
  "compilerOptions": {
    "baseUrl": ".",
    "paths": {
      "@/*": ["src/*"]
    }
  }
}
```

并尝试解析：

```text
@/components/Button
```

### package import

区分：

```text
local
external package
builtin/runtime
unresolved
```

### re-export

例如：

```ts
export { foo } from "./foo";
```

建立：

```text
ReExport
```

关系。

不要把所有 export 都错误地当成普通 import。

---

# 15. Python Resolution

至少覆盖：

```python
import foo
from foo import bar
from . import x
from ..utils import y
```

处理：

- absolute import
- relative import
- package/module path
- project package roots
- `__init__.py` 等真实存在的项目结构

同时注意：

> Python import 的最终运行时解析受环境、`sys.path`、安装方式等影响。

因此不要宣称：

```text
完全确定的 runtime resolution
```

在无法确认时：

```text
ResolutionStatus = Unresolved
```

并提供 evidence。

---

# 16. C# Resolution

至少支持：

```csharp
using Foo.Bar;
using X = Foo.Bar.Baz;
```

以及：

```text
namespace
class/type reference
project reference
```

重点识别：

```text
.csproj
ProjectReference
```

形成：

```text
Project A → Project B
```

关系。

对于 namespace/type resolution：

- 能可靠解析则 Resolved
- 候选多个则 Ambiguous
- 无法解析则 Unresolved

不要模拟完整 C# 编译器。

---

# 17. Project / Package / Workspace Graph

M3 不只分析源文件。

需要识别项目结构，例如：

```text
workspace
 ├── package
 ├── package
 └── package
```

或：

```text
solution
 ├── project
 ├── project
 └── project
```

或者 Python package / Rust workspace 等。

建立层级：

```text
Workspace
    ↓
Project / Package
    ↓
Module
    ↓
File
    ↓
Symbol
```

注意：

> 不是每个项目都需要完整填充所有层级。

没有证据时不要创造虚拟结构。

---

# 18. Symbol Reference

M2 已经拥有 Symbol。

M3 可以开始建立：

```text
Reference
```

例如：

```text
source Symbol
    ↓
references
    ↓
target Symbol
```

但是必须区分：

```text
Syntax reference
Semantic reference
Resolved reference
```

例如一个名字：

```text
Button
```

如果 parser 只能告诉你：

```text
identifier = Button
```

不能直接宣称：

```text
Button → components/Button.tsx::Button
```

只有 resolver 成功后才建立 resolved edge。

---

# 19. Static Call Graph：只做“可靠子集”

M3 可以实现静态调用关系：

```text
Function A
    ↓ calls
Function B
```

但范围必须严格控制。

需要明确支持：

```text
direct function call
direct method call
resolved imported function call
resolved class method call
```

可以识别：

```text
foo()
service.fetch()
obj.method()
```

但以下情况必须保守：

```text
reflection
dynamic import
eval
dependency injection not statically resolvable
function pointers
higher-order dynamic dispatch
runtime generated methods
metaprogramming
duck typing ambiguity
```

不能“猜”。

例如：

```python
handler = get_handler()
handler()
```

如果无法确定 `handler`：

```text
CallEdge:
resolution = Unresolved
```

而不是随便指向某个函数。

---

# 20. Import / Reference / Call 三层关系

最终应该可以表达：

```text
File A
  └─ imports → File B

Module A
  └─ depends_on → Module B

Symbol A
  └─ references → Symbol B

Function A
  └─ calls → Function B
```

这些关系可以同时存在，但含义不同。

---

# 21. Graph Model

建立统一图模型：

```text
GraphNode {
    id: GraphNodeId
    kind: GraphNodeKind
    label: String
    entity_id: String
    metadata: ...
}
```

以及：

```text
GraphEdge {
    id: GraphEdgeId
    source: GraphNodeId
    target: GraphNodeId

    kind: GraphEdgeKind

    resolution: ResolutionStatus
    confidence: Confidence

    evidence: Vec<Evidence>
}
```

GraphNodeKind 可以包括：

```text
Workspace
Project
Package
Module
File
Namespace
Symbol
ExternalPackage
BuiltinModule
```

GraphEdgeKind 至少考虑：

```text
Contains
Imports
DependsOn
Exports
ReExports
References
Calls
ProjectReferences
PackageDependsOn
```

不要为了“统一”而失去语义。

---

# 22. 多级 Graph

M3 至少提供：

## File Dependency Graph

```text
file → file
```

## Module Dependency Graph

```text
module → module
```

## Project / Package Graph

```text
project → project
package → package
```

## Symbol Reference Graph

```text
symbol → symbol
```

## Call Graph

```text
function → function
```

这些图应该从同一底层关系模型派生，而不是复制五份不同实现。

---

# 23. Architecture Graph

Architecture Graph 不等于：

```text
LLM 认为的架构
```

M3 的 Architecture Graph 必须是：

> **从实际项目结构、依赖、引用和项目边界中生成的客观结构图。**

例如：

```text
Workspace
    ↓
Applications
    ↓
Services
    ↓
Libraries
    ↓
Modules
```

如果项目没有明确 layer：

> 不要伪造 layer。

可以提供：

```text
Detected structure
```

而不是：

```text
Recommended architecture
```

后者不属于 M3。

---

# 24. Graph Metrics

M3 可以计算客观指标：

```text
node_count
edge_count

in_degree
out_degree

fan_in
fan_out

connected_components

cycle_count

unresolved_edge_count
ambiguous_edge_count

external_dependency_count
```

可以进一步提供：

```text
most_referenced_nodes
most_dependent_nodes
```

但是：

> 指标是 FACT，不是评价。

例如：

```text
fan_in = 37
```

是事实。

不能在 M3 写：

```text
high coupling = bad
```

这类判断留给 M7。

---

# 25. Cycle Detection

实现：

```text
cycle detection
```

至少支持：

```text
A → B → C → A
```

输出稳定结果：

```text
Cycle {
    nodes: [...]
    edges: [...]
}
```

保证：

- deterministic ordering
- stable IDs
- 可复现测试

不要因为发现 cycle 就产生 Health finding。

M3 只记录：

```text
FACT:
cycle exists
```

M7 再决定这是否构成问题。

---

# 26. Resolution Diagnostics

对于不能解析的关系，用户应该看到：

```text
Unresolved dependency
```

而不是静默丢弃。

诊断至少包含：

```text
source
requested target
resolver
status
reason
candidates
location
evidence
```

例如：

```text
Requested:
@/utils

Status:
Unresolved

Reason:
tsconfig path mapping could not be resolved

Location:
src/App.tsx:4
```

---

# 27. Determinism

这是整个 Prism 的硬要求。

相同项目：

```text
同样输入
+
同样配置
+
同样工具环境
```

必须得到：

```text
相同 ModuleId
相同 GraphNodeId
相同 DependencyEdge
相同排序
相同 JSON 结构顺序
```

禁止：

- random ID
- unordered output
- nondeterministic traversal
- 根据线程完成顺序决定结果

并发执行可以有，但结果必须 canonicalize。

---

# 28. Incremental-Friendly Design

M3 暂时不要实现真正持久化缓存。

但设计时必须考虑未来：

```text
M11 Snapshot
M13 performance/incremental scan
```

关系计算输入应该可以明确拆分：

```text
discovery facts
+
code facts
+
project config
+
resolver context
```

未来能够：

```text
只重算 changed files
```

而不是整个系统强耦合。

可以预留：

```text
resolution fingerprint
```

但不要实现完整数据库/cache。

---

# 29. Performance

M3 可能是第一次真正出现“图规模”问题。

必须：

- bounded parallelism
- cancellation
- progress reporting
- avoid quadratic scans where possible
- avoid loading every source file repeatedly
- reuse M2 parse results
- avoid duplicate symbol traversal
- avoid duplicate resolution

对于大型项目：

```text
解析
→ 建立索引
→ resolve
→ build graph
```

应该尽量是：

```text
single logical pipeline
```

而不是每个 UI 页面重复分析。

---

# 30. GUI

M3 必须让：

```text
Understand
```

和：

```text
Architecture
```

产生真实可用的关系查看体验。

可以新增：

```text
Architecture
```

导航项，或者纳入 Understand。

建议至少显示：

### Project Overview

```text
Projects
Packages
Modules
Files
Dependencies
External Packages
Cycles
Unresolved
```

### Dependency Explorer

用户可以选择：

```text
Project
Module
File
Symbol
```

查看：

```text
Dependencies
Dependents
References
Calls
```

### Graph View

提供：

```text
nodes
edges
zoom
pan
selection
filter
```

至少支持：

```text
File
Module
Project
External Package
```

默认不要一次渲染整个超大型 symbol graph。

应支持：

```text
depth
node kind
edge kind
incoming/outgoing
```

等过滤。

---

# 31. 图可视化技术

先检查现有前端架构和依赖。

原则：

> **不要为了一个图强行引入庞大图形生态。**

先评估现有 React/TypeScript 环境下：

- SVG
- Canvas
- 当前已有依赖
- 小型稳定 graph library

哪种最适合。

如果使用第三方库：

- 必须说明选择理由
- 必须检查许可证
- 不得引入无必要的大型依赖
- 不得把 domain model 与图 UI library API 绑定

核心模型必须保持：

```text
Prism Graph Model
        ↓
Graph View Adapter
        ↓
UI renderer
```

以后即使更换图形库，核心关系模型也不能被污染。

---

# 32. CLI

新增 CLI：

```bash
prism graph <path>
```

建议支持：

```bash
prism graph <path>
prism graph <path> --format json
prism graph <path> --kind module
prism graph <path> --kind file
prism graph <path> --kind project
```

具体参数根据现有 CLI framework 调整。

CLI 必须复用 GUI 使用的同一 Rust application/core pipeline。

禁止：

```text
GUI 一套分析逻辑
CLI 另一套分析逻辑
```

---

# 33. JSON Contract

JSON 输出必须是稳定 machine-readable contract。

至少包含：

```json
{
  "project": {},
  "modules": [],
  "nodes": [],
  "edges": [],
  "cycles": [],
  "metrics": {},
  "diagnostics": []
}
```

保证：

- stable field names
- stable enum values
- deterministic ordering
- no absolute path unless explicitly requested
- no secrets
- no full source dump

默认使用：

```text
project-relative paths
```

保持与 M1/M2 一致。

---

# 34. Error Handling

单个文件无法解析：

```text
不要导致整个 M3 失败
```

例如：

```text
100 files
2 unresolved
3 parse failure
95 resolved
```

仍然应该产生：

```text
partial success
```

但是整个项目无法建立基本 discovery/code model 时，可以：

```text
fatal
```

错误等级必须与 M0 Error Model 一致。

---

# 35. Privacy / Security

Prism 是：

> local-first

M3：

- 不联网
- 不上传项目
- 不调用 LLM
- 不发送 source code
- 不读取项目外无关目录
- 不自动访问 cloud services

外部依赖识别只基于：

```text
project files
configuration
local metadata
```

不要实现：

```text
online package lookup
```

---

# 36. 测试 Fixture

必须建立真实 fixture 项目。

至少覆盖：

### TypeScript

```text
relative import
path alias
re-export
external package
unresolved import
```

### JavaScript

```text
relative import
CommonJS
ESM
external package
```

### Python

```text
absolute import
relative import
package
unresolved module
```

### C#

```text
namespace
using
project reference
type reference
unresolved symbol
```

### Graph

```text
simple chain
diamond dependency
cycle
multiple components
external node
builtin node
ambiguous resolution
unresolved resolution
```

### Project hierarchy

```text
workspace
 ├── app
 ├── core
 └── shared
```

### Call Graph

```text
direct call
method call
resolved imported function
unresolved dynamic call
```

### Edge cases

```text
Unicode filename
Unicode symbol
Windows path
POSIX path
case sensitivity differences
empty project
unsupported language
generated files
large project
broken import
```

---

# 37. Contract Tests

必须确保：

```text
M2 Symbol Model
        ↓
M3 Resolution
        ↓
Graph
```

之间的 contract 稳定。

重点测试：

```text
same input → same IDs
same input → same graph
same input → same diagnostics
```

---

# 38. Regression Tests

M3 完成后必须重新跑：

```text
M0 tests
M1 tests
M2 tests
M3 tests
```

不能因为 M3 改造 domain model 导致早期测试悄悄失效。

---

# 39. Self Scan

使用：

> Prism 自己扫描自己

作为正式 acceptance test。

要求能够得到：

```text
Projects / Packages / Modules
Files
Dependencies
External dependencies
Unresolved relationships
Cycles
Graph metrics
```

并验证：

```text
GUI
CLI
JSON
```

三者来源一致。

---

# 40. 不要提前进入 M6/M7

特别禁止在 M3 中出现：

```text
change risk
architecture risk
health score
bad dependency
critical module
architecture smell
recommended refactor
```

除非只是 UI 中明确标识为：

```text
raw metric
```

或：

```text
objective structural fact
```

M3 的输出：

```text
关系
证据
解析状态
结构指标
```

M6 才负责：

```text
Change Impact
```

M7 才负责：

```text
Health / Findings / Risk
```

---

# 41. M3 完成定义

只有以下全部成立，M3 才算完成：

## Core

- [ ] Module Model
- [ ] Dependency Model
- [ ] Graph Model
- [ ] ResolutionStatus
- [ ] Evidence integration
- [ ] stable IDs
- [ ] deterministic output

## Resolver

- [ ] TypeScript
- [ ] JavaScript
- [ ] Python
- [ ] C#

至少达到：

```text
常见静态关系可解析
```

而不是声称：

```text
100% language semantics
```

## Graph

- [ ] file graph
- [ ] module graph
- [ ] project/package graph
- [ ] symbol reference graph
- [ ] limited call graph
- [ ] cycle detection
- [ ] graph metrics

## CLI

- [ ] prism graph
- [ ] JSON output
- [ ] correct exit codes
- [ ] deterministic output

## GUI

- [ ] real dependency explorer
- [ ] real graph
- [ ] node selection
- [ ] dependency/dependent inspection
- [ ] resolution diagnostics
- [ ] basic filters
- [ ] progress/cancellation

## Tests

- [ ] fixture projects
- [ ] resolver tests
- [ ] graph tests
- [ ] cycle tests
- [ ] ambiguity tests
- [ ] unresolved tests
- [ ] deterministic tests
- [ ] regression tests
- [ ] self-scan

## Safety

- [ ] no network
- [ ] no telemetry
- [ ] no AI
- [ ] no secret leakage
- [ ] no source upload

---

# 42. 最终审计

实现完成后，不要直接宣布完成。

执行完整：

# M3 Final Audit

## A. Baseline

记录：

```text
branch
HEAD
working tree
M0/M1/M2 status
```

## B. Architecture

确认：

```text
Frontend
↓
Tauri
↓
Application
↓
Core
```

没有反向依赖。

## C. Data Model

检查：

```text
Module
Dependency
Reference
CallEdge
GraphNode
GraphEdge
```

是否稳定、无重复职责。

## D. Resolution

按语言分别报告：

```text
resolved
ambiguous
unresolved
external
builtin
```

## E. Graph

报告：

```text
nodes
edges
cycles
components
fan-in/out
```

## F. CLI

真实执行：

```bash
prism graph
```

以及：

```bash
prism graph --format json
```

## G. GUI

真实打开：

```text
Overview
Understand
Architecture
```

确认数据是真实分析所得，而非 mock。

## H. Tests

报告：

```text
M0
M1
M2
M3
```

各自测试结果。

## I. Performance

至少记录：

```text
small fixture
medium fixture
Prism self-scan
```

发现明显问题时记录证据。

## J. Privacy / Security

检查：

```text
network
telemetry
secret exposure
absolute paths
source leakage
```

## K. Git

检查：

```text
untracked
ignored
generated artifacts
debug files
temporary files
```

不要把：

```text
target/
node_modules/
coverage/
logs/
IDE files
local DB
```

等垃圾提交进去。

---

# 43. 审计输出格式

严格使用：

```text
# M3 Final Audit

## 1. Baseline

## 2. Architecture

## 3. Module Model

## 4. Dependency Model

## 5. Resolution

## 6. Graph

## 7. Call Graph

## 8. CLI

## 9. GUI

## 10. Tests

## 11. Performance

## 12. Privacy / Security

## 13. Git

## 14. Risks

## 15. Deferred
```

所有重要结论尽量标：

```text
FACT
HYPOTHESIS
INFERENCE
```

状态使用：

```text
CONFIRMED
RESOLVED
WORSENED
```

风险只允许：

```text
P0
P1
```

不要制造大量低价值风险。

---

# 44. Git 提交

M3 完成并审计通过后：

```bash
git status
git diff
git add ...
git commit -m "feat: add dependency and architecture analysis"
```

默认：

> **只 commit，不 push。**

---

# 45. 最终目标

完成 M3 后，Prism 应该从：

```text
M1:
“这个项目有什么？”
```

升级为：

```text
M2:
“这个项目里的代码是什么？”
```

再升级为：

```text
M3:
“这些代码彼此是什么关系？”
```

形成：

```text
Project
 ↓
Files
 ↓
Symbols
 ↓
Modules
 ↓
Dependencies
 ↓
References
 ↓
Calls
 ↓
Architecture Graph
```

最终为 M4–M11 提供可靠基础：

```text
M4 Git
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

**核心验收标准不是“图画出来了”，而是：**

> **Prism 能够用可追溯的证据，把项目中的“实体之间有什么关系”稳定地变成机器可用、CLI 可用、GUI 可用的结构事实。**