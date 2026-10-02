# Prism M5 — ChangeSet
## 从 Git Diff 到结构化代码变更

你现在进入 **Prism M5：ChangeSet**。

这是 Prism 从：

> “知道 Git 发生了什么”

进入：

> **“知道代码层面发生了什么变化”**

的关键阶段。

M4 已经提供：

> Git Repository → Git Status / Commit / Diff / Hunk / Line

M2 已经提供：

> File → Symbol → Symbol Tree

M3 已经提供：

> Module / Dependency / Reference / Call / Architecture Graph

M5 现在必须把这三层事实连接起来：

```text
Git Facts
    ↓
File Changes
    ↓
Changed Symbols
    ↓
Changed Modules
    ↓
ChangeSet
```

M5 的核心产物是：

> **一个稳定、确定性、证据驱动、可供 M6 Impact Analysis 消费的 ChangeSet。**

---

# 0. 最重要的边界

严格遵守：

```text
M1 = File Facts
M2 = Code Facts
M3 = Relationship Facts
M4 = Git Facts
M5 = Change Facts
M6 = Impact Facts
```

因此：

## M5 做：

- Git diff → normalized change
- file-level change
- line/hunk-level change
- symbol-level change
- module-level change
- change classification
- before/after code facts
- change evidence
- changed symbol mapping
- changed module mapping
- ChangeSet serialization
- GUI Changes 页面升级

## M5 不做：

- Impact Analysis
- Dependency propagation
- “谁可能受影响”
- Risk Score
- Health Score
- Architecture recommendation
- AI explanation
- AI summary
- Context Pack
- test impact prediction
- documentation impact prediction
- Snapshot persistence
- LLM
- RAG
- database
- embeddings
- MCP
- code modification

尤其严格禁止：

> 把 ChangedSymbol 直接解释成 ImpactedSymbol。

这是 M5 与 M6 最重要的边界。

---

# 1. Baseline Audit

在修改任何代码之前，先完整审计：

```text
M0 Foundation
M1 Project Discovery
M2 Code Understanding
M3 Dependency & Architecture
M4 Git Basics
```

必须确认真实代码，而不是依据旧文档假设。

---

# 2. Baseline 输出格式

首先输出：

```text
FACT
HYPOTHESIS
INFERENCE
```

每项标记：

```text
CONFIRMED
RESOLVED
WORSENED
```

风险只列：

```text
P0
P1
```

必须明确回答：

### M4 是否已经提供：

- GitStatus
- GitDiff
- GitDiffFile
- GitDiffHunk
- GitDiffLine
- GitCommit
- GitRepository
- GitHead

### M2 是否已经提供：

- Symbol
- SymbolId
- SymbolIndex
- SourceLocation
- SourceRange
- ProjectCodeModel

### M3 是否已经提供：

- Module
- ModuleId
- DependencyGraph
- ArchitectureGraph
- Symbol relationships

如果实际实现不同：

> 以当前真实代码为准，不强制改造成理论结构。

---

# 3. M5 核心目标

最终必须建立：

```text
GitDiff
   ↓
ChangeSetBuilder
   ↓
FileChange
   ↓
SymbolChange
   ↓
ModuleChange
   ↓
ChangeSet
```

最终能够回答：

> 这次 Git 变化具体改了哪些文件、哪些代码结构、哪些模块？

而不能回答：

> 这些修改会影响谁？

后者留给 M6。

---

# 4. 核心设计原则

必须遵守：

## Deterministic First

同一个：

```text
same repository state
+
same revisions
+
same Prism version
```

应该产生稳定的 ChangeSet。

---

## Evidence First

每一个：

```text
ChangedSymbol
ChangedModule
ChangeClassification
```

必须有证据来源。

不要只返回：

```json
{
  "symbol": "Foo"
}
```

而要能够追溯：

```text
Git diff
→ file
→ hunk
→ source range
→ symbol
```

---

## Conservative Mapping

如果不能可靠知道某个 diff 属于哪个 symbol：

```text
不要猜。
```

应该：

```text
Unmapped
Ambiguous
Unknown
```

而不是错误地绑定到最近的函数。

---

## Explicit Uncertainty

ChangeSet 必须能够表达：

```text
Mapped
Ambiguous
Unmapped
NotApplicable
```

不要强迫所有 diff 都映射到 symbol。

---

# 5. ChangeSet Domain Model

建立统一：

```text
ChangeSet
```

建议：

```rust
ChangeSet {
    id,
    repository,
    base,
    target,
    scope,
    files,
    symbols,
    modules,
    summary,
    diagnostics,
    evidence
}
```

实际字段根据当前 architecture 调整。

---

# 6. ChangeSet Identity

不要随机 UUID 作为唯一业务身份。

推荐基于：

```text
repository identity
+
base revision
+
target revision
+
diff scope
```

构建稳定 ChangeSet identity。

例如：

```text
repo + HEAD + working-tree
repo + HEAD + index
repo + commitA + commitB
```

具体实现根据现有 ID 基础设施决定。

要求：

> 同一个 Git 状态得到相同逻辑 ChangeSet identity。

---

# 7. Change Scope

必须显式区分 ChangeSet 来源。

建议：

```text
ChangeScope
    WorkingTree
    Staged
    RevisionRange
    Commit
    Unknown
```

例如：

```text
WorkingTree:
HEAD → working tree

Staged:
HEAD → index

RevisionRange:
A → B
```

不要模糊称为：

```text
current changes
```

---

# 8. FileChange

M5 的第一层核心实体：

```text
FileChange
```

建议至少：

```text
FileChange {
    file_id,
    old_path,
    new_path,
    change_type,

    additions,
    deletions,

    hunks,

    old_file_exists,
    new_file_exists,

    old_language,
    new_language,

    old_file_kind,
    new_file_kind,

    mapping_status,
    evidence
}
```

注意：

M4 已经有 GitDiffFile。

M5 应该：

> **把 Git-specific diff facts 转化为 Prism-level change facts。**

不要简单复制 M4 类型再换名字。

---

# 9. File Change Types

统一：

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

必须能够表达：

```text
old_path != new_path
```

以及：

```text
file exists before
file exists after
```

---

# 10. Added file

例如：

```text
src/payment.ts
```

新增。

ChangeSet 应表达：

```text
before = absent
after = present
change = Added
```

后续：

```text
symbols
```

可以全部归入：

```text
Added
```

---

# 11. Deleted file

删除：

```text
before = present
after = absent
```

SymbolChange 可以：

```text
Deleted
```

但是：

> 删除后的 symbol 不应该尝试从当前 ProjectCodeModel 中寻找。

需要使用：

```text
before model
```

或者：

```text
historical model
```

---

# 12. Modified file

修改：

```text
before = present
after = present
```

需要进一步分析：

```text
diff hunks
→ source ranges
→ symbols
```

---

# 13. Renamed file

Rename：

```text
old_path
new_path
```

必须特别处理：

```text
Rename only
Rename + content modification
```

不要简单：

```text
Deleted old
Added new
```

除非 Git 本身只提供 delete/add。

如果 Git 已经明确给出 rename：

> 保留 Rename 事实。

---

# 14. Language Change

处理：

```text
foo.py
→
foo.ts
```

之类：

```text
old_language
new_language
```

如果 rename extension 发生变化：

可以：

```text
old_language = Python
new_language = TypeScript
```

M5 可以判定：

```text
LanguageChanged
```

但不要判断：

> 这是高风险变化。

风险属于 M6 / Health。

---

# 15. SymbolChange

M5 的核心第二层：

```text
SymbolChange
```

建议：

```text
SymbolChange {
    before_symbol_id,
    after_symbol_id,

    before_symbol,
    after_symbol,

    change_type,

    file_change_id,

    location_before,
    location_after,

    changed_ranges,

    mapping_status,

    evidence
}
```

---

# 16. Symbol Change Types

至少：

```text
Added
Deleted
Modified
Renamed
Moved
SignatureChanged
VisibilityChanged
BodyChanged
MetadataChanged
Unknown
```

但是这里必须非常谨慎。

---

# 17. “Modified” 与细分类型

例如函数：

```text
foo(a: number)
```

变成：

```text
foo(a: number, b: number)
```

可以可靠检测：

```text
SignatureChanged
```

如果：

```text
function body changed
```

但名称、参数、位置等稳定：

```text
BodyChanged
```

可以产生：

```text
Modified
+
BodyChanged
```

不要强制所有修改都落入单一 enum。

可以使用：

```text
primary_change_type
+
change_flags[]
```

或者当前项目更合适的模型。

---

# 18. Symbol identity

必须复用 M2 的稳定 Symbol identity 设计。

当前 symbol identity 若基于：

```text
project
+
relative path
+
qualified name
+
kind
```

则 M5 必须明确处理：

```text
before symbol identity
after symbol identity
```

尤其注意：

### rename

```text
foo()
→
bar()
```

如果仅靠 name 生成 SymbolId：

可能被错误判定为：

```text
delete foo
add bar
```

而不是：

```text
rename
```

因此需要建立：

> **symbol matching strategy**

但必须保守。

---

# 19. Symbol matching

建立：

```text
SymbolMatcher
```

或者同语义 service。

目标：

```text
before symbols
       ↕
after symbols
```

可能使用证据：

1. stable file identity
2. qualified name
3. symbol kind
4. signature
5. source location proximity
6. AST structure / body similarity
7. Git rename metadata

但注意：

> 匹配策略必须有优先级与置信度。

---

# 20. Matching Confidence

建议：

```text
Exact
HighConfidence
Probable
Ambiguous
Unmatched
```

但不要人为制造一个：

```text
97%
```

这种看似精确、实际无依据的分数。

如果未来需要 score：

必须有明确数学定义与测试。

M5 默认优先：

```text
categorical confidence
```

---

# 21. Symbol mapping status

建议：

```text
Mapped
Ambiguous
Unmapped
NotApplicable
```

例如：

```text
Added file
```

不存在 before symbol：

```text
before = null
mapping = NotApplicable
```

而：

```text
modified function
```

成功匹配：

```text
Mapped
```

---

# 22. Diff → Symbol mapping

核心问题：

> Git diff 是 line/hunk，M2 是 symbol range。

M5 必须解决：

```text
DiffHunk
    ↓
SourceRange
    ↓
SymbolRange overlap
    ↓
SymbolChange
```

---

# 23. Symbol range overlap

推荐建立确定性规则。

例如：

一个 symbol：

```text
Foo.bar()
start line = 20
end line = 45
```

diff：

```text
line 31-34 modified
```

则：

```text
Foo.bar → Modified
```

如果 diff：

```text
line 1-3
```

而 symbol：

```text
line 20-45
```

则：

```text
Foo.bar not changed
```

不要靠文件级修改猜 symbol 修改。

---

# 24. Boundary changes

特别测试：

```text
symbol declaration line changed
symbol signature line changed
attribute/decorator changed
closing brace changed
multi-line signature
nested symbol
```

需要避免：

> 一个 hunk 跨过多个 symbol 时只映射第一个 symbol。

例如：

```text
Foo()
Bar()
Baz()
```

一个大 hunk：

```text
Foo → Bar → Baz
```

则：

```text
Foo changed
Bar changed
Baz changed
```

只要 source ranges 有实际 overlap。

---

# 25. Nested Symbols

例如：

```text
class Foo {
    methodA() {
        ...
    }
}
```

如果：

```text
methodA body
```

发生变化：

M5 可以得到：

```text
methodA = BodyChanged
Foo = Modified
```

但是必须明确：

> 父 symbol 的 Modified 是结构事实，不等于业务影响。

---

# 26. Symbol propagation

允许：

```text
nested child changed
→ parent symbol structurally changed
```

但不要无限向上：

```text
method
→ class
→ namespace
→ module
→ project
```

然后全部标记 Changed。

只标记实际具有 source-range / structural evidence 的层级。

例如：

```text
method body changed
```

合理：

```text
method = BodyChanged
class = StructurallyContainsChangedChild
```

但不要：

```text
namespace = BusinessLogicChanged
project = HighRisk
```

---

# 27. Imports / exports changes

M5 可以识别：

```text
import X from "..."
export X
```

这些属于 syntax/code change。

例如：

```text
import A from "./a"
```

变为：

```text
import B from "./b"
```

则对应 symbol / file change 可以被记录。

但是：

> 不在 M5 计算依赖影响。

M3 已经知道 dependency。

M6 才将二者关联。

---

# 28. ModuleChange

第三层：

```text
ModuleChange
```

建议：

```text
ModuleChange {
    module_id_before,
    module_id_after,
    change_type,
    changed_files[],
    changed_symbols[],
    mapping_status,
    evidence
}
```

---

# 29. Module identity

必须复用 M3 的：

```text
ModuleId
```

不要 M5 重新发明 module identity。

Module 不是 file。

例如：

```text
project
    ↓
package
    ↓
module
    ↓
file
    ↓
symbol
```

具体 hierarchy 以 M3 当前真实实现为准。

---

# 30. File → Module mapping

对于修改文件：

```text
FileChange
    ↓
Module
```

使用 M3 已有 module mapping。

如果一个文件：

```text
belongs to exactly one module
```

则：

```text
Mapped
```

如果：

```text
ambiguous
```

则：

```text
Ambiguous
```

不要猜。

---

# 31. Deleted / Renamed module

必须考虑：

```text
module exists before
module exists after
```

例如文件被删除：

```text
module before exists
module after maybe still exists
```

因此：

> 不要把 FileChange = Deleted 自动解释为 Module Deleted。

Module lifecycle 必须由模块事实决定。

---

# 32. ChangeSet Summary

建立：

```text
ChangeSummary
```

只统计事实。

例如：

```text
files changed: 12
files added: 2
files modified: 8
files deleted: 1
files renamed: 1

symbols changed: 37
symbols added: 6
symbols modified: 28
symbols deleted: 3

modules changed: 4
```

不要输出：

```text
risk = 83
```

也不要：

```text
impact = high
```

---

# 33. ChangeSet diagnostics

必须建立：

```text
ChangeDiagnostic
```

例如：

```text
SymbolMappingAmbiguous
SymbolMappingUnavailable
HistoricalModelUnavailable
UnsupportedLanguage
ParseFailed
BinaryFile
GeneratedFile
DeletedFileWithoutBaseline
RenameMappingUncertain
```

重要：

> 映射失败不是整个 ChangeSet 失败。

例如：

```text
100 files changed
95 mapped
5 unmapped
```

应该仍然返回：

```text
ChangeSet
```

同时：

```text
diagnostics = 5
```

---

# 34. Partial success

M5 必须采用：

> Partial Success

例如：

```text
Git diff = success
90 files = success
82 files = symbol mapped
8 files = unsupported language
```

最终：

```text
ChangeSet = valid
```

而不是整个操作失败。

---

# 35. Before / After models

这是 M5 的核心难点之一。

为了判断 SymbolChange：

```text
before
vs
after
```

必须分别获得代码模型。

---

# 36. Current working tree

例如：

```text
HEAD
    ↓
baseline ProjectCodeModel
```

以及：

```text
working tree
    ↓
current ProjectCodeModel
```

但是：

> 不要直接重扫整个 repository 两次而没有必要。

复用 M1/M2 现有 scanner/parser infrastructure。

---

# 37. Revision models

当用户：

```text
prism changes --from A --to B
```

需要比较：

```text
A code model
vs
B code model
```

设计必须允许未来：

```text
historical revision model
```

但 M5 不需要实现数据库历史缓存。

可以采用：

```text
temporary checkout-less revision materialization
```

或：

```text
git show / git archive / temporary workspace
```

具体方案必须基于真实代码审计后确定。

---

# 38. Critical restriction: never mutate user's worktree

为了分析：

```text
A vs B
```

禁止：

```text
git checkout A
git checkout B
```

更不能通过改变用户工作目录来生成 historical model。

如果需要 materialize revision：

必须：

```text
temporary isolated location
```

并在结束后清理。

---

# 39. Historical source handling

需要分析历史 revision 时：

可考虑：

```text
git archive
git show
temporary worktree
```

具体采用哪一种：

> 先 benchmark / audit 再决定。

不要为了简单直接：

```text
checkout
```

---

# 40. Deleted files

对于：

```text
A → B
```

如果：

```text
foo.ts
```

在 A 存在，B 删除。

M5 必须能够获取：

```text
before source
before symbols
```

否则不能产生：

```text
DeletedSymbol
```

此时应：

```text
Delete file fact
+
SymbolMappingUnavailable
```

而不是伪造 symbol。

---

# 41. Added files

同样：

```text
after source
after symbols
```

可得到：

```text
AddedSymbol
```

不需要 before source。

---

# 42. Parser failure

如果：

```text
before parse = success
after parse = failure
```

不能说：

```text
all symbols deleted
```

只能：

```text
FileChange = Modified
after symbol mapping = unavailable
diagnostic = ParseFailed
```

这是非常重要的正确性要求。

---

# 43. Generated files

复用 M1/M2：

如果：

```text
FileKind = Generated
```

仍可以记录：

```text
FileChange
```

但是不要擅自认为：

```text
generated = irrelevant
```

M5 只记录事实。

---

# 44. Binary files

binary：

```text
FileChange exists
```

但：

```text
SymbolChange = NotApplicable
```

不要尝试：

```text
UTF-8 decode
parse AST
```

---

# 45. Unsupported language

例如：

```text
foo.xyz
```

可以：

```text
FileChange = Modified
SymbolMapping = NotApplicable
```

而不是：

```text
ChangeSet failed
```

---

# 46. Empty / whitespace changes

例如代码只是：

```text
whitespace
formatting
```

M5 可以识别：

```text
Line changes exist
Symbol range overlap exists
```

但默认不要判断：

```text
SemanticChange = false
```

因为：

> M5 不做语义等价证明。

可以预留：

```text
change_characteristics
```

但不要伪装成语义分析。

---

# 47. Comment-only changes

同理：

```text
// comment changed
```

必须记录：

```text
diff exists
```

可以进一步记录：

```text
CommentOnly
```

但只有实际 parser / lexical analysis 能证明时才标记。

否则：

```text
Unknown
```

不要猜。

---

# 48. Formatting-only changes

同理。

不要把：

```text
2 spaces → 4 spaces
```

判断为：

```text
Semantic change
```

但也不要毫无证据地判定：

```text
No semantic change
```

---

# 49. Rename symbol detection

例如：

```text
getUser()
```

变成：

```text
fetchUser()
```

如果能通过：

- same parent
- same kind
- similar range
- same body structure
- Git evidence

可靠匹配：

```text
SymbolChange = Renamed
```

如果无法可靠判断：

```text
Deleted + Added
```

并保持：

```text
mapping uncertainty
```

优先正确性，而不是漂亮结果。

---

# 50. Move detection

例如：

```text
Foo.bar()
```

从：

```text
a.ts
```

移动到：

```text
b.ts
```

如果可靠匹配：

```text
Moved
```

但是：

> 不要实现一个复杂的全仓库 fuzzy matching engine。

M5 只需要可靠覆盖常见情况。

复杂历史代码重构匹配可留给未来。

---

# 51. Matching algorithm

可以采用分层策略：

```text
Stage 1
stable identity / exact key

Stage 2
same file + qualified identity

Stage 3
rename-aware matching

Stage 4
kind + signature + structural evidence

Stage 5
conservative fallback
```

每层必须：

- deterministic
- bounded
- testable

禁止：

```text
LLM matching
```

---

# 52. No AI matching

尤其禁止：

```text
“让 AI 判断 foo.ts 的哪个函数对应 bar.ts 的哪个函数”
```

M5 必须：

> 不依赖 AI 即可运行。

未来可以增加：

```text
AI-assisted explanation
```

但不能进入核心 ChangeSet。

---

# 53. Symbol change evidence

例如：

```text
SymbolChange {
    symbol = Foo.bar
    type = BodyChanged
}
```

至少 evidence 要能追溯：

```text
file
hunk
line range
old range
new range
```

或者等价的结构。

推荐：

```text
ChangeEvidence {
    file_change_id
    hunks[]
    source_ranges[]
}
```

---

# 54. Module change evidence

例如：

```text
ModuleChange:
  src/services/payment
```

必须能解释：

```text
because:
  file A changed
  file B changed
  symbol C changed
```

不是：

```text
module changed = true
```

没有来源。

---

# 55. Change graph

M5 可以建立一个内部：

```text
ChangeGraph
```

例如：

```text
ChangeSet
 ├── FileChange
 │    ├── SymbolChange
 │    └── Evidence
 └── ModuleChange
```

但是不要把：

```text
DependencyGraph
```

拷贝进 ChangeSet。

M3 的 graph 仍然独立。

---

# 56. M5 vs M6

必须保证：

## M5

```text
What changed?
```

## M6

```text
What may be affected?
```

例：

```text
FooService.handle() changed
```

属于 M5。

而：

```text
PaymentController
PaymentModule
CheckoutTest
```

可能受影响：

属于 M6。

---

# 57. Dependency graph usage

M5：

```text
可以读取 M3 DependencyGraph
```

用于：

```text
module lookup
symbol lookup
file → module mapping
```

但是：

> 不做 graph traversal。

特别禁止：

```text
ChangedSymbol
→ caller
→ caller
→ caller
```

这是 M6。

---

# 58. Change classification

可以建立：

```text
ChangeCharacteristics
```

例如：

```text
DeclarationChanged
SignatureChanged
BodyChanged
ImportChanged
ExportChanged
DocumentationChanged
MetadataChanged
Unknown
```

但每项必须有证据。

不要输出：

```text
BreakingChange
```

因为 Breaking Change 已经隐含 impact / semantic judgment。

---

# 59. Public API change

即使：

```text
public function signature changed
```

M5 只记录：

```text
VisibilityChanged
SignatureChanged
```

不能记录：

```text
BreakingApiChange
```

是否 breaking：

> M6 或未来 dedicated API analysis。

---

# 60. Test file changes

M1 已经有：

```text
FileKind = Test
```

M5 可以标记：

```text
FileChange.file_kind = Test
```

可以 summary：

```text
test files changed = N
```

但是不能：

```text
these tests are impacted by production code change
```

那是 M6。

---

# 61. Documentation changes

同理：

```text
README.md changed
```

记录：

```text
FileChange
FileKind = Documentation
```

不要自动产生：

```text
documentation drift
```

M7 Context Health / Health Engine 再做。

---

# 62. Config changes

例如：

```text
package.json
tsconfig.json
Cargo.toml
```

照样：

```text
FileChange
```

可以根据 M1 FileClassification 记录：

```text
Config
```

不要在 M5 解释配置影响。

---

# 63. Output model

ChangeSet 必须能 JSON 序列化。

建议：

```json
{
  "id": "...",
  "scope": "RevisionRange",
  "base": "...",
  "target": "...",

  "files": [],
  "symbols": [],
  "modules": [],

  "summary": {},
  "diagnostics": []
}
```

要求：

- stable field names
- explicit nullability
- deterministic ordering
- machine-readable
- no debug internals
- no absolute paths by default

---

# 64. CLI

M4 已经有：

```text
prism git diff
```

M5 现在增加：

```text
prism changes
```

语义：

> 输出 Prism normalized ChangeSet，而不是原始 Git diff。

例如：

```bash
prism changes .
```

默认：

```text
HEAD → working tree
```

或者根据 M4 已有 scope semantics 保持一致。

---

# 65. Revision changes

至少支持：

```bash
prism changes . --from <rev1> --to <rev2>
```

例如：

```bash
prism changes . --from HEAD~1 --to HEAD
```

输出：

```text
File Changes
Symbol Changes
Module Changes
Summary
Diagnostics
```

---

# 66. JSON / Markdown

至少支持：

```bash
prism changes . --json
```

以及：

```bash
prism changes . --format markdown
```

HTML 可复用 M4/M0 report infrastructure；如果当前 report layer 已成熟，可以接入。

不要因为 M5 单独重新实现完整报告系统。

---

# 67. GUI Changes page

M5 后，Changes 页面升级为：

```text
Changes
```

结构建议：

```text
Change Summary

Files
  Added
  Modified
  Deleted
  Renamed

Symbols
  Added
  Modified
  Deleted
  Renamed
  Moved

Modules
  Changed

Diagnostics
```

---

# 68. File → Symbol UI

点击：

```text
src/services/payment.ts
```

显示：

```text
File Change
    ↓
Changed Symbols
    ↓
Diff Hunks
```

例如：

```text
PaymentService
  Modified
  SignatureChanged

PaymentService.handle
  Modified
  BodyChanged
```

这是真正体现 Prism 与普通 Git GUI 差异的第一个地方。

---

# 69. Symbol details

选择 symbol：

显示：

```text
Symbol
Change Type
Before
After
Location
Changed Ranges
Evidence
```

不要显示：

```text
Risk: High
Impact: 17 modules
```

因为还没有 M6。

---

# 70. Module details

选择 module：

显示：

```text
Module
Changed Files
Changed Symbols
Evidence
```

不要展示：

```text
affected modules
```

---

# 71. Large ChangeSets

大型 diff：

例如：

```text
10,000 files
```

不能一次解析整个 symbol mapping 并阻塞 UI。

必须：

- incremental
- bounded concurrency
- cancellation
- progress
- lazy detail loading

---

# 72. Performance strategy

M5 性能热点是：

```text
Git diff
+
historical source
+
parsing
+
symbol matching
```

必须避免：

```text
每个 file
→ 单独启动 git
→ 单独 parse
→ 单独查 symbol index
```

尽量：

```text
1 Git query
→ normalized file changes
→ batch source retrieval
→ batch parser
→ index lookup
→ change mapping
```

---

# 73. Incremental-friendly

虽然 M13 才做正式 persistent cache，但 M5 的架构必须允许未来：

```text
unchanged file
→ reuse M2 model
```

新增或修改：

```text
only reparse affected source
```

不要把：

```text
ChangeSet
```

实现成：

```text
full repository re-scan mandatory
```

否则未来性能会非常差。

---

# 74. M2 reuse

不要重新实现 parser。

应该复用：

```text
ProjectDiscovery
ParserRegistry
LanguageParser
ProjectCodeModel
SymbolIndex
SourceRange
```

ChangeSet 层只负责：

```text
comparison / mapping
```

---

# 75. M3 reuse

不要重新实现：

```text
ModuleResolver
DependencyGraph
Symbol references
```

M5 使用 M3 已建立的：

```text
File → Module
Symbol → Module
```

等关系。

如果 M3 当前能力不足：

只增加最小必要接口。

不要重新造一套 architecture model。

---

# 76. Historical revision analysis

这是 M5 最容易失控的部分。

先实现一个清晰的 abstraction：

```text
RevisionSourceProvider
```

例如：

```text
CurrentWorkingTree
GitRevision
```

目标：

```text
RevisionSourceProvider
    ↓
ProjectDiscovery
    ↓
ProjectCodeModel
```

这样：

```text
current
vs
historical
```

都能进入同一套 M2 analysis pipeline。

---

# 77. Avoid git worktree pollution

历史版本分析：

不要污染用户：

```text
working tree
index
branch
HEAD
```

任何临时 materialization：

必须：

```text
temporary directory
```

并：

```text
cleanup guaranteed
```

异常、取消也必须 cleanup。

---

# 78. Resource limits

为 historical revision analysis 添加：

```text
max files
max file size
timeout
max revisions
max parallel parsers
```

复用 M1/M2 的配置。

不要因为：

```text
git changes huge-project
```

导致几十 GB 临时目录失控。

---

# 79. Cancellation

用户关闭 Changes 页面：

应：

```text
cancel ChangeSet task
```

并取消：

```text
git
source retrieval
parsing
matching
```

不得出现：

```text
GUI cancelled
but background parser keeps running for minutes
```

---

# 80. Error recovery

某个历史 revision：

```text
git show failed
```

不能整个应用 crash。

返回：

```text
ChangeSet
+
diagnostic
```

只有在：

```text
base revision unavailable
target revision unavailable
```

等真正无法构建基础输入时才失败整个 task。

---

# 81. Determinism tests

同一个 fixture：

连续执行：

```text
prism changes
```

至少两次。

比较 normalized JSON：

必须一致。

允许：

```text
execution duration
task ids
```

等 runtime metadata 不同。

业务结果必须一致。

---

# 82. Matching tests

必须加入：

### Exact modification

```text
same symbol
body changed
```

### Signature change

```text
foo(a)
→
foo(a,b)
```

### Rename

```text
foo
→
bar
```

### Move

```text
a.ts/foo
→
b.ts/foo
```

### Delete

```text
foo removed
```

### Add

```text
bar added
```

### Nested symbol

```text
Class
└── method
```

method changed。

### Multiple symbols in one hunk

确保：

> 所有实际覆盖的 symbol 都被识别。

---

# 83. Ambiguity tests

必须主动测试：

两个非常相似的 symbols：

```text
foo()
foo()
```

或者：

```text
duplicate structures
```

如果无法可靠匹配：

```text
Ambiguous
```

而不是硬选一个。

---

# 84. Parser failure tests

例如：

before：

```text
valid TypeScript
```

after：

```text
syntactically broken TypeScript
```

预期：

```text
FileChange = Modified

before symbols = available
after symbols = unavailable

diagnostic = ParseFailed
```

绝不能：

```text
after symbols = all deleted
```

---

# 85. Rename tests

至少：

```text
pure rename
rename + body modification
rename + multiple symbols
```

验证：

```text
FileChange = Renamed
```

以及 symbol mapping 的保守行为。

---

# 86. Cross-language tests

至少：

```text
TypeScript
JavaScript
Python
C#
```

验证：

```text
File → Symbol
Diff → Symbol
Symbol → Module
```

每种语言各有 fixtures。

---

# 87. Unsupported-language tests

例如：

```text
unknown.xyz
```

预期：

```text
FileChange works
Symbol mapping = NotApplicable
ChangeSet remains valid
```

---

# 88. Binary tests

例如：

```text
image.png
```

修改。

预期：

```text
FileChange = Modified
is_binary = true
SymbolChange = NotApplicable
```

---

# 89. Git-only changes

测试：

```text
rename
mode change
```

即使：

```text
source content unchanged
```

也必须产生正确：

```text
FileChange
```

不能因为：

```text
no textual hunk
```

就认为：

```text
nothing changed
```

---

# 90. Whitespace/comment tests

测试：

```text
whitespace-only
comment-only
formatting
```

确保：

- 不崩溃
- diff 保留
- symbol mapping 不错误扩大
- 不虚构 semantic result

---

# 91. Path edge tests

至少：

```text
中文
空格
括号
emoji
特殊符号
长路径
```

确保：

```text
Git path
→ FileChange
→ Symbol mapping
```

不损坏。

---

# 92. CLI tests

测试：

```bash
prism changes .
prism changes . --json
prism changes . --from HEAD~1 --to HEAD
```

以及：

```text
non-git
invalid revision
empty repo
detached HEAD
clean repository
```

---

# 93. GUI tests

至少验证：

```text
Changes page loads
clean repo
dirty repo
untracked files
renamed files
symbol changes
mapping diagnostics
large change loading
cancel
error state
```

---

# 94. No accidental Git operations

完成后必须扫描整个新增代码，确保没有：

```text
commit
push
pull
fetch
checkout
switch
reset
restore
clean
rebase
merge
stash
```

等 mutate repository 的执行路径。

尤其检查：

```text
temporary revision materialization
```

不能偷偷通过 checkout 实现。

---

# 95. Security / Privacy

M5 继续保持：

```text
local-first
no telemetry
no network
no AI
no source upload
```

日志禁止完整打印：

- diff body
- source content
- commit message
- secrets
- env values
- authentication tokens
- private absolute paths

报告默认：

```text
project-relative paths
```

---

# 96. No database

M5 不引入：

```text
SQLite
PostgreSQL
MongoDB
```

也不保存：

```text
historical ChangeSet
```

持久化 Snapshot 属于 M11。

---

# 97. No AI

即使用户未来希望：

```text
“总结这次改了什么”
```

M5 也不调用 LLM。

M5 可以产生：

```text
deterministic summary
```

例如：

```text
12 files changed
37 symbols changed
4 modules changed
```

未来 AI 可以把这些 facts 转成自然语言。

---

# 98. Reports

复用现有：

```text
prism-report
```

如果 M4 已建立 report abstractions：

接入：

```text
ChangeSetReport
```

输出：

```text
JSON
Markdown
HTML
```

但：

> 不重新造 report framework。

---

# 99. Context compatibility

M5 输出应成为未来 M10 的重要输入：

```text
ChangeSet
   ↓
Context Engine
```

但 M5 不生成：

```text
ContextPack
```

尤其不要：

```text
automatic source selection
```

那属于 M10。

---

# 100. Snapshot compatibility

M5 可以产生：

```text
ChangeSet
```

但不保存 Snapshot。

M11 才负责：

```text
ProjectSnapshot
```

以及：

```text
snapshot-to-snapshot change
```

---

# 101. Recommended module structure

根据当前真实 workspace 调整，不要机械照搬。

推荐方向：

```text
prism-git/
```

继续负责：

```text
Git facts
```

而 M5 最好放在：

```text
prism-analysis/
```

或者一个明确的：

```text
prism-changes/
```

核心结构可以类似：

```text
change/
  mod.rs
  model/
    changeset.rs
    file_change.rs
    symbol_change.rs
    module_change.rs
    summary.rs
    diagnostic.rs
    evidence.rs

  matcher/
    symbol_matcher.rs
    file_matcher.rs
    module_matcher.rs

  mapper/
    diff_to_symbol.rs
    symbol_to_module.rs

  revision/
    source_provider.rs
    current.rs
    git_revision.rs

  service/
    changeset_service.rs
    change_builder.rs
```

实际以当前 architecture 为准。

---

# 102. Dependency direction

推荐：

```text
prism-core
   ↑
prism-project
   ↑
prism-parser
prism-git
prism-analysis / prism-changes
   ↑
application
   ↑
CLI / Tauri
   ↑
React
```

不要产生：

```text
prism-git → prism-gui
```

不要：

```text
prism-parser → prism-git
```

除非真实 architecture 有充分理由。

---

# 103. Important architectural decision

认真检查：

> ChangeSet 应该属于 Git 层，还是 Analysis 层？

不要自动接受任何预设。

你的目标是：

```text
Git = source of version-control facts

Code Model = source of code facts

ChangeSet = integration layer
```

因此原则上：

```text
GitDiff
+
ProjectCodeModel
+
Architecture facts
```

共同构成：

```text
ChangeSet
```

不要把 ChangeSet 永久绑定到 Git。

未来 Prism 可能产生：

```text
ChangeSet
from:
  Git
  Snapshot comparison
  external patch
```

因此 ChangeSet 应该是：

> Prism 的通用 change domain。

---

# 104. Future-compatible ChangeSource

可以预留：

```text
ChangeSource
    Git
    Snapshot
    Patch
    Unknown
```

但只实现：

```text
Git
```

其他类型先：

```text
not implemented
```

不要为了“架构完整”实现未来系统。

---

# 105. File identity vs path identity

这是重点。

Rename 后：

```text
old_path
new_path
```

不能简单把：

```text
FileId = path
```

作为永恒身份。

复用 M1/M2 当前 FileId 策略，但：

> M5 必须允许同一逻辑文件在 before / after 中路径发生改变。

不要假设：

```text
FileId_before == FileId_after
```

---

# 106. Module identity vs changed files

同样：

```text
FileChange
```

不能直接创建：

```text
new ModuleId
```

必须复用 M3。

M5 只做：

```text
File → existing module mapping
```

---

# 107. Source Range semantics

复用 M2：

```text
SourceRange
```

严格确认：

- line is 1-based or 0-based
- column semantics
- byte offset semantics
- inclusive/exclusive
- CRLF handling
- UTF-8 handling

不要在 M5 私自发明第二套 range semantics。

---

# 108. Diff line → source range conversion

必须建立明确规则。

尤其注意：

Git diff：

```text
line number
```

和：

parser：

```text
SourceRange
```

可能有：

```text
old line
new line
```

所以：

### before

使用：

```text
old_start / old_lines
```

### after

使用：

```text
new_start / new_lines
```

不能把 new line range 错当 old range。

---

# 109. Changed symbol algorithm

推荐：

```text
1. 获取 FileChange
2. 获取 before/after source
3. 获取 before/after ProjectCodeModel
4. 建立 symbol indexes
5. 先进行 exact matching
6. 再处理 rename/move candidates
7. 将 diff hunks 映射到 before/after ranges
8. 确定 symbol change type
9. 生成 evidence
10. 记录 ambiguity
11. 汇总 ModuleChange
12. 创建 ChangeSet
```

每一步应尽量可独立测试。

---

# 110. No monolithic ChangeSetBuilder

禁止把整个 M5 写成一个：

```text
build_changeset()
```

几千行函数。

至少拆成：

```text
revision collection
file normalization
symbol matching
diff mapping
module aggregation
diagnostic aggregation
summary generation
```

保持边界清晰。

---

# 111. No speculative abstraction explosion

不要建立几十个：

```text
ChangeProvider
ChangeResolver
ChangeOrchestrator
ChangeInterpreter
ChangeSemanticEngine
ChangeHeuristicEngine
ChangeProjectionEngine
```

如果没有真实需求：

> 不要抽象。

目标是：

```text
small interfaces
clear ownership
testable services
```

---

# 112. Test data design

建立：

```text
fixtures/changes/
```

或者根据现有 fixture 架构组织：

```text
basic-modification
symbol-add-delete
rename
move
nested
multi-symbol-hunk
parse-failure
unsupported
binary
config
tests
docs
```

每个 fixture 应尽可能小。

---

# 113. Golden JSON

对于核心 ChangeSet：

建议使用少量 golden fixtures：

```text
input repository
→ expected changeset.json
```

用来保证：

```text
determinism
schema stability
mapping behavior
```

不要把所有测试都做成巨大 JSON snapshots。

---

# 114. Regression against real projects

至少使用：

### Prism 自身

以及：

### 一个小型真实 TypeScript 项目
### 一个 Python 项目
### 一个 C# 项目

如果环境允许，再选一个具有：

- rename
- nested symbols
- module hierarchy
- multiple commits

的真实开源项目。

不要因为测试方便而只用人工 fixture。

---

# 115. Large repository validation

至少选择一个真实大型项目做：

```text
prism changes
```

观察：

- execution time
- memory
- cancellation
- parse failures
- number of unmapped symbols

不要求此阶段极限优化。

但是必须确保：

> 不会明显失控。

---

# 116. Final audit

完成后必须输出：

# M5 Final Audit

## A. Baseline

M0-M4：

```text
FACT
HYPOTHESIS
INFERENCE
```

## B. ChangeSet Architecture

- ownership
- dependency direction
- services
- interfaces

## C. Change Scope

- WorkingTree
- Staged
- RevisionRange
- Commit

## D. File Changes

- Added
- Modified
- Deleted
- Renamed
- Copied
- TypeChanged

## E. Symbol Mapping

- matching algorithm
- confidence
- ambiguous
- unmapped
- rename
- move
- signature/body

## F. Module Changes

- mapping
- aggregation
- evidence

## G. Revision Handling

- historical source
- no worktree mutation
- cleanup
- cancellation

## H. Diagnostics

- parse failure
- unsupported language
- mapping failure
- ambiguity
- partial success

## I. CLI

真实命令、参数、JSON。

## J. GUI

Changes 页面。

## K. Tests

- unit
- integration
- fixtures
- CLI
- GUI
- regression
- real projects

## L. Performance

- large changes
- batching
- cancellation
- resource limits

## M. Security / Privacy

- no network
- no AI
- no telemetry
- no source upload
- no Git mutation

## N. Git

确认：

```text
working tree clean or intentionally changed
no unintended files
no accidental generated files
```

## O. Risks

只列：

```text
P0
P1
```

## P. Deferred

至少明确：

```text
M6 Change Impact
M7 Health
M8 Verify
M9 Environment
M10 Context
M11 Snapshot
```

---

# 117. Audit classification

所有结论必须明确：

```text
FACT
```

或：

```text
HYPOTHESIS
```

或：

```text
INFERENCE
```

例如：

```text
FACT:
82/87 changed files received symbol mappings.

FACT:
5 files are unsupported language or binary.

FACT:
No working-tree mutation occurs during historical comparison.

HYPOTHESIS:
Cross-language historical analysis may become expensive for very large revisions.

INFERENCE:
Current implementation is sufficient for the M5 scope.
```

---

# 118. M5 Completion Gate

只有全部满足才算完成：

```text
[ ] M0 regression passes
[ ] M1 regression passes
[ ] M2 regression passes
[ ] M3 regression passes
[ ] M4 regression passes
[ ] Git → FileChange works
[ ] WorkingTree ChangeSet works
[ ] Staged ChangeSet works
[ ] RevisionRange ChangeSet works
[ ] Added files handled
[ ] Deleted files handled
[ ] Modified files handled
[ ] Renamed files handled
[ ] Binary files handled
[ ] Unsupported languages handled
[ ] Symbol matching works
[ ] Symbol ambiguity is explicit
[ ] Symbol rename is conservative
[ ] Symbol move is conservative
[ ] Signature changes detected
[ ] Body changes detected
[ ] Nested symbols handled
[ ] Multi-symbol hunk handled
[ ] Module changes aggregated
[ ] Evidence is preserved
[ ] Diagnostics are preserved
[ ] Partial success works
[ ] Historical revision does not mutate user's worktree
[ ] Cancellation works
[ ] CLI works
[ ] JSON contract works
[ ] GUI Changes page works
[ ] Real Git integration tests pass
[ ] Prism self-change analysis works
[ ] At least several real projects validated
[ ] No AI
[ ] No network
[ ] No telemetry
[ ] No database
[ ] No Git mutation commands
[ ] No P0
[ ] No unresolved P1 blocking M6
```

---

# 119. Final commit

全部完成并通过审计后：

```bash
git add -A
git commit -m "feat: add changeset analysis"
```

不要 push。

不要修改 remote。

不要进行额外 Git 操作。

---

# 120. 最终状态

M5 完成后，Prism 应形成完整的：

```text
Project Understanding Stack
```

即：

```text
                    ┌───────────────┐
                    │    Project    │
                    └───────┬───────┘
                            │
                   ┌────────▼────────┐
                   │      M1         │
                   │   File Facts    │
                   └────────┬────────┘
                            │
                   ┌────────▼────────┐
                   │      M2         │
                   │   Code Facts    │
                   │    Symbols      │
                   └────────┬────────┘
                            │
                   ┌────────▼────────┐
                   │      M3         │
                   │ Relationship    │
                   │     Facts       │
                   └────────┬────────┘
                            │
                   ┌────────▼────────┐
                   │      M4         │
                   │    Git Facts    │
                   └────────┬────────┘
                            │
                   ┌────────▼────────┐
                   │      M5         │
                   │   ChangeSet     │
                   │  Change Facts   │
                   └────────┬────────┘
                            │
                   ┌────────▼────────┐
                   │      M6         │
                   │ Impact Analysis │
                   └─────────────────┘
```

最重要的是：

```text
M5 = What changed?
M6 = What may be affected?
```

不要越界。

---

# 121. 开始执行

现在直接开始：

```text
1. Baseline audit M0-M4
2. 输出真实审计结果
3. 找到当前代码中的最小 integration points
4. 设计 ChangeSet domain
5. 实现 FileChange
6. 实现 SymbolChange
7. 实现 ModuleChange
8. 实现 Evidence / Diagnostics
9. 实现 RevisionSourceProvider
10. 实现 deterministic matching
11. 实现 CLI
12. 实现 GUI Changes
13. 添加真实 Git integration fixtures
14. 添加 regression tests
15. 自扫描 Prism
16. 运行真实项目验证
17. Final Audit
18. 只修 P0/P1
19. 创建本地 commit
20. 不 push
```

始终遵守：

> **先事实，再映射；先确定，再推断；无法确认就明确标记未知；绝不为了让结果“好看”而猜测。**

最终交付一个真正可以被 **M6 Impact Analysis、M10 Context Engine、M11 Snapshot** 直接消费的 **稳定 ChangeSet 层**。