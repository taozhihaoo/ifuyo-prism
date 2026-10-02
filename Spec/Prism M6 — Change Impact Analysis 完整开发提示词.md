# Prism M6 — Change Impact Analysis
## 从 ChangeSet 到 Impacted Components

你现在进入 **Prism M6：Change Impact Analysis**。

这是 Prism 从：

> “知道改了什么”

进入：

> **“知道这个改动可能影响哪里，以及为什么”**

的阶段。

M5 已经建立：

```text
Git
 ↓
FileChange
 ↓
SymbolChange
 ↓
ModuleChange
 ↓
ChangeSet
```

M3 已经建立：

```text
File
 ↓
Module
 ↓
Symbol
 ↓
Dependency
 ↓
Reference
 ↓
Call
 ↓
Architecture Graph
```

M6 的任务就是把两者连接：

```text
                    M5
                ChangeSet
                    │
                    ▼
            Changed Files
                    │
                    ▼
            Changed Symbols
                    │
                    ▼
            Changed Modules
                    │
                    │
         ┌──────────┴──────────┐
         ▼                     ▼
   M3 Dependency Graph    M3 Symbol Relations
         │                     │
         └──────────┬──────────┘
                    ▼
             Impact Analysis
                    │
       ┌────────────┼────────────┐
       ▼            ▼            ▼
   Components     Tests        Docs
                    │
                    ▼
             Impact Evidence
```

核心问题：

```text
What changed?
        ↓
What may be affected?
        ↓
Why?
```

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
M7 = Health Facts
```

因此：

## M6 做：

- ChangeSet → Impact Analysis
- changed symbol/module → reverse dependency traversal
- changed symbol → callers/referencers
- changed module → dependent modules
- transitive impact
- impact distance
- impact reason
- impact evidence
- potentially affected tests
- potentially affected documentation
- impact summary
- deterministic impact graph
- change blast-radius facts
- evidence-based risk signals

## M6 不做：

- AI reasoning
- LLM summary
- semantic behavior understanding
- full semantic equivalence
- bug detection
- architecture health scoring
- documentation drift detection
- build/test execution
- test pass/fail
- automatic code modification
- Snapshot persistence
- RAG
- embeddings
- MCP
- database
- runtime tracing
- dynamic execution analysis

尤其严格禁止：

> **不要把“存在依赖关系”直接等价成“必然受到影响”。**

M6 输出的是：

```text
Potential Impact
```

而不是：

```text
Guaranteed Impact
```

---

# 1. Baseline Audit

在修改代码之前，先完整审计：

```text
M0 Foundation
M1 Project Discovery
M2 Code Understanding
M3 Dependency & Architecture
M4 Git Basics
M5 ChangeSet
```

必须以：

> 当前真实代码

为准。

不要假设过去 prompt 中的模型一定已经按原计划实现。

---

# 2. Baseline Audit 输出

先输出：

```text
FACT
HYPOTHESIS
INFERENCE
```

并分别标记：

```text
CONFIRMED
RESOLVED
WORSENED
```

只列：

```text
P0
P1
```

必须确认：

### M3 当前真实能力

- File dependencies
- Module dependencies
- Symbol references
- Call edges
- Project/package relationships
- ResolutionStatus
- Evidence
- Graph traversal能力
- Cycle detection

### M5 当前真实能力

- ChangeSet
- FileChange
- SymbolChange
- ModuleChange
- change type
- before/after identity
- mapping status
- confidence
- evidence
- diagnostics

---

# 3. M6 最终回答什么

用户打开：

```text
Changes
```

看到：

```text
PaymentService.handle() changed
```

M6 应进一步回答：

```text
Potentially affected:

CheckoutService
PaymentController
OrderService
CheckoutTests
Payment API documentation

Reasons:

PaymentController calls PaymentService.handle()
CheckoutService depends on PaymentService module
CheckoutTests reference CheckoutService
```

但不能简单：

```text
Impact = High
```

然后什么都不解释。

---

# 4. 核心原则

## Deterministic First

同样的：

```text
Project
+
ChangeSet
+
CodeModel
+
DependencyGraph
```

必须得到相同的：

```text
ImpactAnalysis
```

---

## Evidence First

每一个 ImpactedItem 必须可以追溯：

```text
ChangedSymbol
      ↓
relationship
      ↓
target symbol/module/file
```

例如：

```text
PaymentService.handle()
        ↓ Calls
CheckoutService.process()
```

则：

```text
CheckoutService.process()
impact_reason = CallerOfChangedSymbol
```

---

# 5. Conservative First

如果 M3 判断：

```text
ResolutionStatus = Unresolved
```

M6 不得假设：

```text
Unresolved target = impacted
```

应该：

```text
unresolved relationship
→ diagnostic / uncertainty
```

---

# 6. No hidden inference

不要：

```text
Class A imports Class B
→ A definitely breaks
```

正确：

```text
A depends on B
→ A is a potential impact candidate
```

并记录：

```text
reason = Dependency
confidence = ...
```

---

# 7. Core Domain Model

建立：

```text
ImpactAnalysis
```

建议：

```rust
ImpactAnalysis {
    id,
    change_set_id,
    scope,
    changed_items,
    impacted_items,
    relationships,
    summary,
    diagnostics,
    evidence,
    risk_signals
}
```

实际字段根据现有 architecture 调整。

---

# 8. ImpactedItem

核心实体：

```text
ImpactedItem
```

建议：

```text
ImpactedItem {
    node_id,
    node_kind,
    impact_kind,
    distance,
    confidence,
    reasons,
    evidence,
    path
}
```

其中：

```text
node_kind
    File
    Symbol
    Module
    Project
    Package
```

不要强制所有层级都存在。

---

# 9. ImpactKind

至少考虑：

```text
DirectlyChanged
DirectDependent
TransitiveDependent
Caller
Referencer
ModuleConsumer
ProjectConsumer
TestCandidate
DocumentationCandidate
ConfigurationCandidate
Unknown
```

实际 enum 要根据 M3 graph semantics 调整。

---

# 10. Direct vs Indirect

必须严格区分：

```text
Changed
Directly impacted
Transitive impacted
```

例如：

```text
A changed
B depends on A
C depends on B
D depends on C
```

结果：

```text
A = Changed

B = Direct impact, distance 1

C = Transitive impact, distance 2

D = Transitive impact, distance 3
```

不要全部叫：

```text
Impacted
```

而丢掉距离。

---

# 11. Direction is critical

如果 M3 有：

```text
A → B
```

必须先确认真实语义：

```text
A imports B
```

还是：

```text
A is imported by B
```

再实现 traversal。

绝对不要凭字段名称猜方向。

---

# 12. Impact traversal semantics

典型：

```text
Changed dependency
        ↓
reverse dependency traversal
```

例如：

```text
A changed
B depends on A
```

则：

```text
A
 ↓
B
```

属于 impact。

而：

```text
A → B
```

如果语义是：

```text
A depends on B
```

则从 A 的 change 出发：

> 不应该错误地把 B 当成 A 的消费者。

在实现前必须建立：

```text
Relationship Direction Contract
```

并加入测试。

---

# 13. Symbol-level impact

如果 M5 得到：

```text
SymbolChange:
PaymentService.handle()
```

而 M3 有：

```text
CheckoutService.process()
    → calls
PaymentService.handle()
```

则：

```text
CheckoutService.process()
```

是：

```text
PotentiallyAffected
reason = CallsChangedSymbol
```

---

# 14. Reference impact

如果：

```text
FooService
    → references
PaymentService.handle
```

则：

```text
FooService
```

可以作为：

```text
PotentiallyAffected
reason = ReferencesChangedSymbol
```

但：

> reference 不等于 runtime execution。

不要把：

```text
References
```

自动升级成：

```text
Calls
```

---

# 15. Call impact

如果 M3 提供：

```text
CallEdge
```

则可以计算：

```text
changed function
       ↑
caller
       ↑
caller
```

例如：

```text
A()
 ↓
B()
 ↓
C()
```

若：

```text
C changed
```

则：

```text
B = caller candidate
A = transitive caller candidate
```

---

# 16. Dynamic dispatch

严格承认静态分析限制。

以下情况可能无法完全解析：

```text
reflection
dependency injection
runtime registration
dynamic import
dynamic dispatch
plugin loading
generated calls
configuration-driven dispatch
```

如果 M3 结果：

```text
Unresolved
Ambiguous
```

M6 应：

```text
preserve uncertainty
```

而不是：

```text
guess caller
```

---

# 17. Module-level impact

如果：

```text
Module A changed
```

M3：

```text
Module B depends on A
Module C depends on B
```

则：

```text
A = Changed
B = Directly impacted
C = Transitively impacted
```

---

# 18. File-level fallback

如果某个文件：

```text
Modified
```

但：

```text
Symbol mapping unavailable
```

可以根据 M3：

```text
File → Module
Module → Dependent Module
```

进行：

```text
module-level impact
```

但是必须保留：

```text
symbol-level mapping unavailable
```

不能伪造 symbol impact。

---

# 19. Rename impact

如果 M5：

```text
Symbol renamed:
foo → bar
```

并且 M3 有：

```text
caller/reference
```

则可以追踪：

```text
before symbol
after symbol
```

优先基于：

```text
after symbol
```

分析当前 impact。

但必须注明：

```text
rename-derived impact
```

因为 rename matching 本身具有 confidence。

---

# 20. Deleted symbol

如果：

```text
foo()
```

被删除。

M6 应分析：

```text
before graph
```

寻找：

```text
callers
references
dependents
```

这些是：

```text
PotentiallyAffected
```

因为它们可能仍然依赖已经删除的 symbol。

这属于：

> M6 最有价值的场景之一。

---

# 21. Added symbol

如果：

```text
newFunction()
```

新增。

默认：

```text
existing consumers = none
```

因此不应该因为“新增 symbol”就产生大量 impact。

但如果 M5 / M3 已经建立：

```text
added symbol is exported
```

可以记录：

```text
API surface changed
```

但仍不要自行推断消费者。

---

# 22. Signature change

这是重要情况：

```text
foo(a)
→
foo(a, b)
```

M5：

```text
SignatureChanged
```

M6：

寻找：

```text
callers
references
implementers
overrides
```

然后：

```text
PotentiallyAffected
```

理由：

```text
CallsChangedSymbol
ReferencesChangedSymbol
```

这是典型：

```text
Change → Impact
```

链路。

---

# 23. Visibility change

例如：

```text
public
→
private
```

M5：

```text
VisibilityChanged
```

M6：

应该特别寻找：

```text
external references
callers
cross-module consumers
```

产生：

```text
PotentiallyAffected
```

但不要自动声明：

```text
compile failure
```

除非存在另一个独立、明确的静态事实分析证明。

---

# 24. Export change

例如：

```text
export Foo
```

变成：

```text
not exported
```

可以根据 M3：

```text
resolved consumers
```

寻找：

```text
reference
import
dependency
```

结果：

```text
PotentiallyAffected
```

---

# 25. Module dependency change

如果 M5 识别：

```text
module A
imports
module B
```

发生变化。

则 M6 可寻找：

```text
dependents of changed module
```

但不要：

```text
new dependency = automatically impacted all dependency subtree
```

必须根据实际 edge direction 与 change semantics。

---

# 26. Test impact

这是 M6 的重要输出之一。

可以从：

```text
test symbol
test file
test module
```

寻找：

```text
references
calls
dependencies
module relationships
```

得到：

```text
TestCandidate
```

例如：

```text
PaymentService changed

CheckoutTest
    → calls CheckoutService
        → calls PaymentService
```

则：

```text
CheckoutTest
impact_kind = TestCandidate
distance = 2
```

---

# 27. Test wording

必须使用：

```text
Potentially affected tests
```

不要写：

```text
Tests that will fail
```

除非未来 M8 实际执行测试并得到失败结果。

---

# 28. Documentation impact

同样：

如果：

```text
public API
module
configuration
```

发生变化。

M6 可以根据 M3/M5：

```text
README
docs
API reference
configuration docs
```

中的已知 references 建立：

```text
DocumentationCandidate
```

但默认不要通过文本相似度猜测全部文档。

---

# 29. Documentation boundary

不要在 M6 实现完整：

```text
documentation drift
```

M7 / M10 才处理。

M6 只回答：

```text
known relationship suggests this document may be relevant
```

---

# 30. Impact path

每一个间接 impact 最好能够保存一条路径。

例如：

```text
PaymentService.handle
    ↓ Calls
CheckoutService.process
    ↓ DependsOn
CheckoutModule
```

可以：

```text
ImpactPath
```

表示：

```text
Changed
→ relationship
→ relationship
→ target
```

---

# 31. ImpactEvidence

建立：

```text
ImpactEvidence
```

例如：

```text
ImpactEvidence {
    source_change,
    relationship_path,
    source_node,
    target_node,
    relation_kind,
    resolution_status,
    confidence
}
```

这样 GUI 可以回答：

> 为什么 Prism 认为这里可能受到影响？

---

# 32. Evidence example

不要：

```json
{
  "name": "CheckoutService",
  "impact": true
}
```

应能够表达：

```text
Changed:
PaymentService.handle

Impact:
CheckoutService.process

Reason:
Calls

Evidence:
CheckoutService.process
    calls
PaymentService.handle

Distance:
1

Resolution:
Resolved
```

---

# 33. Multiple impact reasons

一个节点可能同时：

```text
Calls
References
ModuleDependsOn
```

因此：

```text
reasons[]
```

而不是只有：

```text
reason
```

例如：

```text
CheckoutService

reasons:
- CallsChangedSymbol
- DependsOnChangedModule
```

---

# 34. Duplicate impact paths

如果：

```text
A
↘
 B
↗
C
```

有多个路径到 B：

不要返回：

```text
B
B
B
```

应该：

```text
B
impact_paths = 2
reasons = [...]
```

或者统一的：

```text
evidence_paths[]
```

具体模型根据实际 architecture。

---

# 35. Cycle handling

M3 已经支持 cycle。

例如：

```text
A → B
B → C
C → A
```

如果 A changed：

不能：

```text
无限 traversal
```

必须：

```text
visited set
```

保证：

```text
termination
```

同时保留：

```text
cycle evidence
```

---

# 36. Maximum traversal depth

不能无限遍历超大型项目。

建议：

```text
max_depth
```

默认配置。

例如：

```text
1
2
3
5
10
unlimited
```

但默认值必须基于实际项目规模评估。

如果达到上限：

```text
ImpactDiagnostic:
TraversalDepthExceeded
```

而不是假装完整。

---

# 37. Maximum impacted nodes

同理：

```text
max_impacted_nodes
```

防止：

```text
Change in shared utility
→ 50,000 nodes
```

导致 UI / memory 崩溃。

超过：

```text
ImpactLimitReached
```

必须明确标记：

```text
analysis_incomplete = true
```

---

# 38. Blast radius

M6 可以建立：

```text
BlastRadiusSummary
```

例如：

```text
changed_files = 8
changed_symbols = 21

directly_impacted_modules = 4
transitively_impacted_modules = 13

potentially_affected_tests = 9
potentially_relevant_docs = 3
```

这些都是：

> 事实统计。

---

# 39. Do not make blast radius equal risk

不要：

```text
13 modules
→ High Risk
```

也不要：

```text
2 modules
→ Low Risk
```

因为：

> impact breadth 和 actual risk 不是同一个概念。

可以记录：

```text
blast_radius
```

以及独立：

```text
risk_signals
```

---

# 40. Risk Signals

M6 可以提供：

```text
RiskSignal
```

但必须是：

> 可解释的事实性信号。

例如：

```text
PublicApiChanged
HighFanOutDependencyChanged
CrossModuleImpact
TransitiveImpactDepth
LargeBlastRadius
UnresolvedRelationship
AmbiguousSymbolMapping
DeletedReferencedSymbol
ConfigurationFileChanged
CoreModuleChanged
```

这些是：

```text
signals
```

不是最终结论。

---

# 41. Risk Level

暂时不要建立复杂：

```text
0-100 risk score
```

也不要训练模型。

如果需要等级：

建议在当前阶段先只提供：

```text
RiskSignals
```

不要：

```text
overallRisk = 87
```

真正的综合风险评价可以在后续基于更多数据再设计。

---

# 42. Public API signal

如果 M2/M3 有：

```text
visibility
export
public API
```

可以产生：

```text
PublicApiChanged
```

例如：

```text
public Foo()
signature changed
```

但不要自动：

```text
BreakingChange
```

---

# 43. High fan-out dependency

例如：

```text
UtilityModule
```

被：

```text
37 modules
```

直接依赖。

如果 UtilityModule 修改：

M6 可以产生：

```text
HighFanOutDependencyChanged
```

但：

> 不要自动认为这是高风险结论。

这是事实信号。

---

# 44. Unresolved relationships

如果：

```text
20 references
```

其中：

```text
7 resolved
5 ambiguous
8 unresolved
```

M6 应显示：

```text
analysis limitations
```

例如：

```text
Resolved relationships: 7
Ambiguous: 5
Unresolved: 8
```

这样用户知道：

> impact result 不是完整证明。

---

# 45. Impact confidence

建议：

```text
Exact
HighConfidence
Probable
Ambiguous
Unknown
```

不要虚构：

```text
92%
```

confidence 必须来自：

```text
relationship resolution
symbol mapping
traversal path
change confidence
```

---

# 46. Confidence propagation

例如：

```text
ChangedSymbol mapping = HighConfidence

Caller edge = Resolved

Caller impact:
HighConfidence
```

但：

```text
ChangedSymbol mapping = Ambiguous
```

则：

```text
impact should not silently become Exact
```

可以采用：

> conservative minimum / rule-based confidence propagation。

具体算法先基于实际模型定义。

---

# 47. Change confidence × relation confidence

Impact confidence 可以考虑：

```text
Change mapping confidence
×
Relationship resolution confidence
```

但不要为了“高级”而强行做数学乘法。

优先：

```text
explicit categorical propagation
```

---

# 48. Directly changed items must remain separate

例如：

```text
A changed
B impacted
```

结果中：

```text
A:
impact_kind = DirectlyChanged

B:
impact_kind = DirectDependent
```

不要把 A 混进：

```text
affected
```

导致 UI 无法区分：

> actually changed

和：

> potentially affected。

---

# 49. Changed vs impacted sets

内部至少保持两个 distinct sets：

```text
ChangedSet
ImpactCandidateSet
```

即：

```text
M5
    Changed

M6
    PotentiallyAffected
```

禁止合并。

---

# 50. Self-impact

如果：

```text
A changed
```

又通过 cycle：

```text
A → B → A
```

不能重复把：

```text
A
```

作为新的 external impact。

默认：

```text
A remains DirectlyChanged
```

cycle 作为 evidence / diagnostic。

---

# 51. Cross-project impact

如果 M3 已经支持：

```text
Project
Package
```

例如：

```text
shared-lib
    ↓
app
```

M6 可以：

```text
shared-lib changed
→ app potentially affected
```

但必须区分：

```text
same project
different project in workspace
external package
```

外部 package：

```text
External
```

不要扫描互联网寻找消费者。

---

# 52. External dependency

例如：

```text
npm package
NuGet package
PyPI package
```

如果本地项目改变的是：

```text
external package
```

M6 不应试图进行：

```text
Internet search
```

也不应下载 package。

只处理：

```text
local graph evidence
```

---

# 53. Configuration impact

配置变化：

```text
tsconfig.json
appsettings.json
package.json
Cargo.toml
```

可能影响很多东西。

M6 只有在已有 M3 relationship / known structural mapping 时才建立 impact。

例如：

```text
tsconfig path alias
```

相关模块可能被识别。

否则：

```text
ConfigurationChanged
```

仅作为：

```text
RiskSignal / Change fact
```

而不是猜测所有文件受影响。

---

# 54. Test candidates

测试候选可以按：

```text
Direct
Transitive
Reference
Call
ModuleDependency
```

分类。

例如：

```text
Test A
  direct caller

Test B
  module dependency

Test C
  transitive caller
```

这样 M8 可以直接消费。

---

# 55. M8 compatibility

M6 输出：

```text
PotentiallyAffectedTests
```

M8 将：

```text
PotentiallyAffectedTests
        ↓
Verify
        ↓
ActuallyExecuted
        ↓
Passed / Failed / Skipped
```

所以 M6 必须保持：

> candidate semantics。

---

# 56. Context compatibility

M10 将：

```text
ChangeSet
+
ImpactAnalysis
+
Health
+
Docs
```

生成：

```text
ContextPack
```

因此 M6 应输出：

```text
changed items
impact paths
affected modules
candidate tests
uncertainty
evidence
```

但 M6 不生成 ContextPack。

---

# 57. Snapshot compatibility

M11 将保存：

```text
ProjectSnapshot
ImpactAnalysis
```

用于：

```text
time-series
```

但 M6 不建立持久化。

---

# 58. ImpactAnalysis algorithm

推荐：

```text
1. Load ChangeSet
2. Separate changed files/symbols/modules
3. Load M3 relationship graph
4. Resolve changed graph nodes
5. Determine traversal seeds
6. Traverse reverse relationships
7. Record each impact path
8. Merge duplicate paths
9. Calculate distance
10. Classify impact kind
11. Find test candidates
12. Find documentation candidates
13. Generate uncertainty diagnostics
14. Generate blast-radius summary
15. Generate risk signals
16. Serialize ImpactAnalysis
```

所有步骤必须尽量独立、可测试。

---

# 59. Traversal seeds

不同 ChangeSet 类型：

### Symbol change

优先：

```text
ChangedSymbol
```

### Module change

优先：

```text
ChangedModule
```

### File-only change

使用：

```text
ChangedFile
→ Module
```

### Binary / unsupported

通常：

```text
no semantic traversal
```

除非已有明确 graph evidence。

---

# 60. Seed normalization

避免：

```text
same module
20 changed symbols
→ 20 duplicate traversals
```

可以先建立：

```text
normalized seeds
```

例如：

```text
ChangedModule A
```

下属：

```text
symbol1
symbol2
symbol3
```

如果 module-level impact 已覆盖某一层：

可以减少重复 graph traversal。

具体优化必须保证不丢 evidence。

---

# 61. Relation priority

需要定义统一关系优先级。

例如：

```text
Call
Reference
Imports
DependsOn
ProjectReference
PackageDependency
```

但：

> 不要假设哪个关系一定比哪个关系“更重要”。

应该定义的是：

```text
impact meaning
```

例如：

```text
Call → CallerOfChangedSymbol
Reference → ReferencerOfChangedSymbol
DependsOn → ConsumerOfChangedModule
```

---

# 62. Reverse graph

强烈建议内部支持：

```text
forward graph
reverse graph
```

而不是每次：

```text
遍历所有 edges 找 incoming
```

例如：

```text
Forward:
A → B

Reverse:
B → A
```

M6 主要需要：

```text
reverse adjacency
```

因为问题是：

> 谁依赖被修改对象？

---

# 63. Do not duplicate M3 graph permanently

如果 M3 已经有：

```text
DependencyGraph
```

M6 可以建立：

```text
ImpactTraversalIndex
```

或临时 reverse adjacency。

但不要：

```text
复制整个 graph model
```

形成两套 source of truth。

---

# 64. Graph ownership

必须保持：

```text
M3 = owns relationship facts

M6 = consumes relationship facts
```

M6 发现某条关系明显错误：

不能悄悄修自己的副本。

应该：

```text
diagnostic
```

并回溯 M3。

---

# 65. Architecture boundaries

M6 不分析：

```text
“这个模块设计得好不好”
```

例如：

```text
A imports UI
```

M7 Health 才判断是否违反 architecture rules。

M6 仅：

```text
A is potentially impacted by B change
```

---

# 66. No health scoring

禁止输出：

```text
Architecture Health = 62
```

或者：

```text
Change Health = 71
```

M6 是：

```text
Impact analysis
```

不是：

```text
Health analysis
```

---

# 67. No semantic bug detection

例如：

```text
if (amount > 0)
```

改成：

```text
if (amount >= 0)
```

M6 不应该说：

```text
business behavior changed
```

只能使用 M5 的：

```text
BodyChanged
```

然后依据 static relationships 计算：

```text
potentially impacted callers
```

---

# 68. No runtime claims

禁止：

```text
“这个函数运行时一定会被执行”
```

除非：

> 有运行时 tracing 数据。

M6 没有运行时 tracing。

因此统一语言：

```text
potentially affected
candidate
static relationship
```

---

# 69. Impact graph

可以建立：

```text
ImpactGraph
```

形式：

```text
Changed
   ↓
Impact Path
   ↓
Potentially Affected
```

例如：

```text
PaymentService.handle()
        │
        ├──calls──> CheckoutService.process()
        │                  │
        │                  └──calls──> CheckoutController.submit()
        │
        └──referenced by──> PaymentTest
```

但：

> ImpactGraph 必须建立在 M3 graph 事实上，不重新定义 relationship semantics。

---

# 70. UI Impact visualization

Changes 页面可以增加：

```text
Impact
```

例如：

```text
Changed: 8 files
Changed: 21 symbols

Potentially Affected
--------------------------------
12 modules
9 tests
3 documents

Top paths
--------------------------------
PaymentService
  ↓
CheckoutService
  ↓
CheckoutController
```

---

# 71. Impact filtering

GUI 至少支持：

```text
All
Modules
Symbols
Tests
Docs
Direct
Transitive
```

以及：

```text
Confidence
```

过滤。

---

# 72. Impact detail

点击：

```text
CheckoutService.process()
```

应该看到：

```text
Potential impact
Reason:
Calls changed symbol

Changed source:
PaymentService.handle()

Relationship:
CheckoutService.process()
    calls
PaymentService.handle()

Distance:
1

Confidence:
HighConfidence
```

---

# 73. Impact path UI

允许：

```text
Why impacted?
```

展示：

```text
Changed
PaymentService.handle

↓ Calls

CheckoutService.process

↓ DependsOn

CheckoutModule
```

这应该成为 Prism 非常核心的交互。

---

# 74. Uncertainty UI

例如：

```text
8 relationships unresolved
3 symbol mappings ambiguous
```

不能藏起来。

应该显示：

```text
Analysis limitations
```

用户需要知道：

> 结果受静态分析能力限制。

---

# 75. No giant graph by default

如果有：

```text
20,000 nodes
```

不要默认全部渲染。

默认：

```text
changed node
+
top direct impacts
```

点击后展开。

---

# 76. Large project strategy

必须考虑：

```text
1 changed utility
→ thousands of dependents
```

不能一次构建一个巨大 UI DOM。

需要：

- lazy expansion
- pagination
- virtualization
- bounded traversal
- cancellation
- background analysis

---

# 77. Performance

性能重点：

```text
ChangeSet
+
reverse graph traversal
+
path aggregation
```

不要：

```text
每个 changed symbol
×
扫描整个 repository
```

应该：

```text
build reverse adjacency once
→ traverse from seeds
```

---

# 78. Bounded traversal

至少支持：

```text
max_depth
max_nodes
max_paths
timeout
```

并产生：

```text
analysis_incomplete
```

明确说明结果是否被截断。

---

# 79. Deterministic ordering

所有：

```text
impacted_items
impact_paths
reasons
diagnostics
```

必须稳定排序。

不能：

```text
HashMap iteration order
```

导致每次 JSON 不同。

---

# 80. Cycle deterministic handling

如果：

```text
A → B → C → A
```

从 A 开始：

结果必须稳定。

例如：

```text
A
B distance 1
C distance 2
```

不要：

```text
A
B distance 1
C distance 2
A distance 3
B distance 4
...
```

visited-state 必须明确。

---

# 81. Distance semantics

建议：

```text
DirectlyChanged = 0

DirectImpact = 1

TransitiveImpact = N
```

但是必须在 schema 中正式定义。

例如：

```text
distance = number of relationship hops
```

不要：

```text
distance = “感觉上的距离”
```

---

# 82. Multiple paths and distance

如果：

```text
A → B
A → C → B
```

则 B：

```text
distance = 1
```

因为最短 impact path 是 1。

同时可以保留：

```text
additional_paths = 1
```

或完整 evidence path。

---

# 83. Impact aggregation

如果一个 target：

```text
B
```

同时：

```text
changed symbol X
changed module A
```

影响到它。

不要生成：

```text
B
B
```

应该合并：

```text
B
reasons = [...]
sources = [...]
min_distance = ...
```

---

# 84. Same target multiple kinds

例如：

```text
TestFoo
```

既是：

```text
Caller
```

又是：

```text
TestCandidate
```

允许多个：

```text
impact classifications
```

不要强制：

```text
single enum
```

---

# 85. Risk signals generation

风险信号必须 deterministic。

例如：

```text
if deleted symbol has resolved callers:
    DeletedReferencedSymbol

if public API signature changed:
    PublicApiChanged

if direct impacted module count > threshold:
    LargeBlastRadius

if unresolved relations exist:
    UnresolvedRelationship

if mapping ambiguous:
    AmbiguousSymbolMapping
```

阈值必须：

- configurable
- documented
- tested

或者先只输出原始计数，不定义阈值。

优先后者。

---

# 86. Prefer facts over thresholds

例如：

```text
direct_impacted_modules = 4
transitive_impacted_modules = 19
max_depth = 4
unresolved_relationships = 7
```

这些已经非常有价值。

不要急于：

```text
risk = high
```

---

# 87. CLI

新增：

```bash
prism impact .
```

默认：

```text
current ChangeSet
→ ImpactAnalysis
```

也可以：

```bash
prism impact . --from <rev1> --to <rev2>
```

---

# 88. CLI options

至少考虑：

```text
--json
--format markdown
--from
--to
--staged
--working-tree
--max-depth
--max-nodes
--include-tests
--include-docs
```

参数命名必须与 M4/M5 保持一致。

---

# 89. CLI output

例如：

```text
Changed
  8 files
  21 symbols
  4 modules

Potentially affected
  12 modules
  16 symbols
  9 tests
  3 docs

Analysis
  direct: 7
  transitive: 21
  unresolved: 4
  ambiguous: 2
```

不要默认输出大量源码。

---

# 90. Exit codes

复用现有 CLI exit code contract。

特别注意：

```text
analysis incomplete
```

和：

```text
analysis failed
```

必须区分。

例如：

```text
valid but truncated
```

不能冒充：

```text
failed
```

具体 code 根据 M0/M4 当前真实 contract 调整。

---

# 91. Report integration

复用：

```text
prism-report
```

增加：

```text
ImpactAnalysisReport
```

支持：

```text
JSON
Markdown
HTML
```

不要重新做 report engine。

---

# 92. Tests — basic topology

至少：

```text
A → B
B → C
```

测试：

```text
A changed
```

确认：

```text
B
C
```

的 distance 与 classification 正确。

---

# 93. Tests — reverse dependency

构建：

```text
Utility
  ↑
Service
  ↑
Controller
```

修改：

```text
Utility
```

预期：

```text
Service = direct
Controller = transitive
```

---

# 94. Tests — call graph

```text
A()
 ↓
B()
 ↓
C()
```

修改：

```text
C()
```

预期：

```text
B = caller
A = transitive caller
```

---

# 95. Tests — reference graph

测试：

```text
symbol X
referenced by:
A
B
C
```

修改：

```text
X
```

预期：

```text
A B C
```

得到：

```text
Referencer
```

---

# 96. Tests — deleted symbol

测试：

```text
foo()
```

被删除。

before graph：

```text
A → foo
B → foo
```

after graph：

```text
foo absent
```

预期：

```text
A = potentially affected
B = potentially affected
```

并且 evidence 指向：

```text
deleted symbol
```

---

# 97. Tests — signature change

```text
foo(a)
```

改为：

```text
foo(a,b)
```

测试：

```text
caller1
caller2
caller3
```

预期：

```text
all resolved callers
```

进入：

```text
PotentiallyAffected
```

---

# 98. Tests — visibility

```text
public foo
→
private foo
```

如果跨 module 有 references：

测试：

```text
cross-module consumer
```

是否被识别。

---

# 99. Tests — rename

```text
foo
→
bar
```

验证：

```text
M5 rename confidence
+
M3 relationships
+
M6 impact
```

整个链条不能因为 SymbolId 变化而断裂。

---

# 100. Tests — file rename

测试：

```text
a.ts
→
b.ts
```

内容不变。

预期：

```text
FileChange = Renamed
```

并根据 M3 graph：

> 不产生虚假的全量 dependency impact。

---

# 101. Tests — cycle

```text
A → B
B → C
C → A
```

修改 A。

验证：

```text
terminates
no duplicate targets
correct distance
cycle does not create infinite path
```

---

# 102. Tests — ambiguous relationship

M3：

```text
ResolutionStatus = Ambiguous
```

预期：

```text
do not invent impact target
```

产生：

```text
AmbiguousRelationship
```

---

# 103. Tests — unresolved relationship

同理：

```text
ResolutionStatus = Unresolved
```

确认：

```text
no false positive impacted item
```

但保留：

```text
analysis limitation
```

---

# 104. Tests — partial ChangeSet

例如：

```text
10 changed files
2 unsupported
1 parse failure
7 fully analyzed
```

M6 必须：

```text
return partial valid analysis
```

同时：

```text
diagnostics
analysis completeness
```

明确。

---

# 105. Tests — unsupported language

```text
foo.xyz
```

发生修改。

预期：

```text
File changed
Impact analysis may stop at file/module boundary
No fake symbol graph
```

---

# 106. Tests — binary

```text
image.png
```

变化。

预期：

```text
FileChange exists
symbol impact = NotApplicable
```

---

# 107. Tests — tests

建立：

```text
Production
  ↓
Service
  ↓
Test
```

修改 production。

确认：

```text
TestCandidate
```

出现。

但不是：

```text
TestWillFail
```

---

# 108. Tests — documentation

建立：

```text
API
 ↓
known doc reference
```

修改 API。

验证：

```text
DocumentationCandidate
```

如果没有可靠 relationship：

```text
不要猜。
```

---

# 109. Tests — multiple paths

```text
A → B
A → C → B
```

测试：

```text
B appears once
min distance = 1
multiple evidence paths retained
```

---

# 110. Tests — large fan-out

构造：

```text
A
↓
B1...B1000
```

修改 A。

验证：

- no crash
- stable output
- bounded performance
- deterministic ordering
- max nodes respected

---

# 111. Tests — max depth

例如：

```text
A → B → C → D → E
```

设置：

```text
max_depth = 2
```

预期：

```text
B = included
C = included
D/E = excluded
```

并：

```text
analysis_incomplete = true
```

---

# 112. Tests — cancellation

一个大型 graph：

```text
impact task starts
↓
cancel
```

必须：

```text
stop traversal
release resources
return Cancelled
```

不能：

```text
UI cancelled
but traversal continues indefinitely
```

---

# 113. Tests — determinism

同一个：

```text
ChangeSet
```

连续执行：

```text
impact analysis
impact analysis
```

JSON business output 必须一致。

排序必须稳定。

---

# 114. Real project validation

至少测试：

### Prism 自己

以及：

### 一个 TypeScript 项目
### 一个 Python 项目
### 一个 C# 项目

重点观察：

```text
changed symbols
reverse dependencies
callers
modules
tests
unresolved relationships
```

不要只用人工 fixture。

---

# 115. Real-world refactor validation

选择一个真实项目，至少制造：

```text
rename
signature change
method body change
module move
delete
```

通过 Git commit / temporary fixture 验证：

```text
M5 ChangeSet
→
M6 ImpactAnalysis
```

完整链路。

---

# 116. Performance validation

至少记录：

```text
project size
nodes
edges
changed symbols
impacted symbols
execution time
peak memory if available
```

不要只说：

```text
fast
```

应该输出实际 measurement。

---

# 117. No persistent cache

M6 不引入：

```text
SQLite
RocksDB
database
embedding store
```

如果现有系统已经存在缓存：

> 复用，但不要为 M6 新建持久化系统。

---

# 118. No network

禁止：

```text
web search
package registry lookup
GitHub API
NPM API
NuGet API
PyPI API
```

ImpactAnalysis 完全建立于：

```text
local project facts
```

---

# 119. No AI

禁止：

```text
LLM
embeddings
semantic retrieval
AI relationship inference
AI impact prediction
```

尤其不能：

```text
“让 AI 猜哪些文件受影响”
```

核心能力必须：

> 无 AI 正常工作。

---

# 120. No runtime analysis

不要添加：

```text
runtime instrumentation
debugger
profiling
execution trace
coverage collection
```

这些留给未来独立能力。

---

# 121. Architecture quality

M6 新代码必须保持：

```text
M3 owns graph facts
M5 owns change facts
M6 owns impact interpretation
```

依赖方向：

```text
prism-core
   ↑
project
parser
git
analysis/change
   ↑
impact
   ↑
application
   ↑
CLI / Tauri
   ↑
React
```

不要：

```text
M3 imports M6
```

不要：

```text
M5 imports GUI
```

---

# 122. Avoid monolithic implementation

禁止：

```text
analyze_impact()
```

几千行。

至少拆分：

```text
seed_builder
reverse_graph
impact_traversal
impact_classifier
path_builder
test_candidate_analyzer
documentation_candidate_analyzer
risk_signal_generator
summary_builder
```

实际需要几个模块，以真实代码复杂度决定。

---

# 123. Avoid abstraction explosion

不要因为 M6 而创建大量：

```text
ImpactResolver
ImpactProvider
ImpactInterpreter
ImpactOrchestrator
ImpactStrategyFactory
ImpactTraversalStrategyFactory
```

没有真实多态需求不要抽象。

保持：

```text
small interfaces
clear ownership
deterministic behavior
easy tests
```

---

# 124. Important API boundary

未来 M10 需要：

```text
ChangeSet
ImpactAnalysis
```

所以 M6 应提供稳定的：

```text
ImpactAnalysisService
```

语义类似：

```text
analyze(change_set, project_model, relationship_graph, options)
    -> ImpactAnalysis
```

CLI / GUI 都调用这一层。

不要：

```text
React
→ direct graph traversal
```

---

# 125. Configuration

可以建立：

```text
ImpactOptions
```

例如：

```text
max_depth
max_nodes
max_paths
include_tests
include_docs
include_symbols
include_modules
include_external
```

但默认配置必须保守。

不要提供几十个参数。

---

# 126. Machine-readable schema

JSON 必须明确：

```text
changed_items
impacted_items
impact_paths
summary
diagnostics
risk_signals
analysis_completeness
```

例如：

```json
{
  "change_set_id": "...",
  "analysis_completeness": "Complete",
  "changed_items": [],
  "impacted_items": [],
  "impact_paths": [],
  "summary": {},
  "diagnostics": [],
  "risk_signals": []
}
```

---

# 127. Analysis completeness

至少：

```text
Complete
Partial
Truncated
Failed
```

含义必须写入 schema 文档。

例如：

```text
Complete:
all requested traversal completed

Partial:
some source/relationship mappings unavailable

Truncated:
limits reached

Failed:
analysis could not produce valid result
```

---

# 128. Impact diagnostics

至少支持：

```text
UnresolvedRelationship
AmbiguousRelationship
UnmappedChange
UnsupportedLanguage
ParseFailure
TraversalDepthExceeded
ImpactNodeLimitReached
ImpactPathLimitReached
Timeout
Cancelled
HistoricalModelUnavailable
IncompleteGraph
```

具体名称复用已有 diagnostic conventions。

---

# 129. Impact summary

建议：

```text
ImpactSummary {
    changed_files,
    changed_symbols,
    changed_modules,

    direct_impacted_files,
    direct_impacted_symbols,
    direct_impacted_modules,

    transitive_impacted_files,
    transitive_impacted_symbols,
    transitive_impacted_modules,

    test_candidates,
    documentation_candidates,

    unresolved_relationships,
    ambiguous_relationships,

    max_distance,
    analysis_completeness
}
```

全部是统计事实。

---

# 130. RiskSignals summary

例如：

```text
RiskSignals:
- PublicApiChanged
- DeletedReferencedSymbol
- CrossModuleImpact
- LargeBlastRadius
- AmbiguousSymbolMapping
```

每个 signal 必须带：

```text
evidence
```

例如：

```text
DeletedReferencedSymbol
count = 3
evidence = [...]
```

---

# 131. Never hide uncertainty

GUI / CLI / Report 都必须告诉用户：

```text
Impact analysis is based on static relationships.
Some relationships could not be resolved.
```

中文 UI 可表达为：

> 影响分析基于当前可解析的静态关系；部分关系无法解析，因此结果可能不完整。

不要把这个限制藏在 debug log。

---

# 132. M6 / M7 boundary

必须最终验证：

## M6

```text
What may be affected?
Why?
How far?
How certain?
```

## M7

```text
Is the project healthy?
What is wrong?
What architectural / dependency / documentation / security issues exist?
```

例如：

```text
A changed
B may be affected
```

是 M6。

而：

```text
A has excessive fan-out
```

是 M7。

---

# 133. M6 / M8 boundary

M6：

```text
PotentiallyAffectedTests
```

M8：

```text
ActuallyRunTests
Passed
Failed
Skipped
```

不要交叉。

---

# 134. M6 / M10 boundary

M6：

```text
impact graph
```

M10：

```text
Context Pack
```

例如 M10 可以：

```text
changed symbol
+
impacted callers
+
related docs
+
health findings
```

生成 AI context。

M6 不做 context selection。

---

# 135. M6 / M11 boundary

M6：

```text
on-demand ImpactAnalysis
```

M11：

```text
snapshot
history
trend
```

不在 M6 保存历史。

---

# 136. Final UI structure

Changes 页面升级为：

```text
Changes

Summary
├─ Changed
├─ Direct Impact
├─ Transitive Impact
├─ Tests
└─ Docs

Changed
├─ Files
├─ Symbols
└─ Modules

Impact
├─ Modules
├─ Symbols
├─ Tests
└─ Docs

Impact Paths
├─ Why impacted?
└─ Evidence

Analysis Limitations
└─ Unresolved / Ambiguous / Truncated
```

---

# 137. Default UX

默认不要展示：

```text
1000 nodes
```

而是：

```text
Changed
8 files
21 symbols
4 modules

Potentially affected
12 modules
16 symbols
9 tests

View all impact
```

然后用户逐步展开。

---

# 138. Main success scenario

最终必须真实跑通：

```text
1. Modify PaymentService.handle()
2. prism changes .
3. Prism detects:
   PaymentService.handle = BodyChanged
4. prism impact .
5. Prism detects:
   CheckoutService.process = Caller
   CheckoutController.submit = TransitiveCaller
   CheckoutTest = TestCandidate
6. GUI displays paths/evidence
```

整个过程中：

```text
no AI
no network
no manual tagging
```

---

# 139. Second success scenario

```text
1. Delete public API symbol
2. prism changes .
3. M5 detects DeletedSymbol
4. M6 loads before relationships
5. Finds resolved callers
6. Reports potentially affected callers
7. Shows deleted-symbol evidence
```

这应该是 M6 的核心验收场景。

---

# 140. Third success scenario

```text
1. Change common utility module
2. utility has many dependents
3. M6 computes reverse graph
4. direct + transitive impact calculated
5. blast radius shown
6. large fan-out signal generated
7. no arbitrary overall risk score
```

---

# 141. Fourth success scenario

```text
1. One dependency is unresolved
2. Change occurs
3. M6 cannot safely traverse it
4. Target is not fabricated
5. Diagnostic is generated
6. analysis completeness reflects limitation
```

---

# 142. Git behavior

M6 默认只读取：

```text
Git / ChangeSet / Project Model / Graph
```

禁止：

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

---

# 143. Privacy

保持：

```text
local-first
no telemetry
no source upload
no network
```

日志：

不要输出：

```text
complete source
complete diff
secrets
tokens
private environment values
```

默认使用：

```text
project-relative path
```

---

# 144. Final Audit

完成后必须输出：

# M6 Final Audit

## A. Baseline

M0-M5。

使用：

```text
FACT
HYPOTHESIS
INFERENCE
```

## B. Impact Architecture

- ownership
- dependency direction
- M3 integration
- M5 integration

## C. Graph Semantics

- edge direction
- reverse traversal
- resolution states
- cycle handling

## D. Impact Model

- changed items
- impacted items
- distance
- confidence
- reasons
- paths
- evidence

## E. Symbol Impact

- callers
- references
- signature
- visibility
- rename
- delete

## F. Module Impact

- direct
- transitive
- cross-project

## G. Test Candidates

- direct
- transitive
- evidence

## H. Documentation Candidates

- resolved references
- uncertainty

## I. Risk Signals

- exact rules
- evidence
- counts
- no hidden score

## J. Limits

- depth
- nodes
- paths
- timeout
- unresolved graph

## K. CLI

真实命令、参数、输出。

## L. GUI

Impact 页面 / Changes 页面。

## M. Tests

- unit
- graph
- integration
- fixtures
- CLI
- GUI
- cancellation
- determinism
- real projects

## N. Performance

真实数据：

```text
nodes
edges
changed
impacted
time
memory if available
```

## O. Security / Privacy

确认：

```text
no network
no telemetry
no AI
no source upload
no Git mutation
```

## P. Git

确认：

```text
no accidental files
no generated junk
no unintended source changes
```

## Q. Risks

只列：

```text
P0
P1
```

## R. Deferred

明确保留：

```text
M7 Health Engine
M8 Verify Engine
M9 Environment
M10 Context Engine
M11 Snapshot
```

---

# 145. Completion Gate

只有以下全部满足，M6 才算完成：

```text
[ ] M0 regression passes
[ ] M1 regression passes
[ ] M2 regression passes
[ ] M3 regression passes
[ ] M4 regression passes
[ ] M5 regression passes

[ ] ChangeSet → ImpactAnalysis works
[ ] direct impact works
[ ] transitive impact works
[ ] reverse dependency traversal works
[ ] symbol caller impact works
[ ] symbol reference impact works
[ ] module impact works
[ ] deleted symbol impact works
[ ] signature change impact works
[ ] visibility change impact works
[ ] rename impact works conservatively

[ ] test candidates work
[ ] documentation candidates work where evidence exists

[ ] impact distance works
[ ] impact paths work
[ ] duplicate paths merge correctly
[ ] cycles terminate
[ ] unresolved relationships remain uncertain
[ ] ambiguous relationships remain uncertain
[ ] partial analysis works
[ ] truncation is explicit
[ ] cancellation works
[ ] deterministic output works

[ ] blast radius summary works
[ ] risk signals are evidence-based
[ ] no arbitrary overall risk score

[ ] CLI works
[ ] JSON works
[ ] Markdown/HTML report integration works if existing infrastructure supports it
[ ] GUI Changes/Impact works

[ ] Prism self-analysis works
[ ] TypeScript project validated
[ ] Python project validated
[ ] C# project validated
[ ] real refactor scenario validated

[ ] no AI
[ ] no network
[ ] no telemetry
[ ] no database
[ ] no runtime tracing
[ ] no Git mutation
[ ] no P0
[ ] no unresolved P1 blocking M7
```

---

# 146. Final commit

全部完成、测试和最终审计通过后：

```bash
git add -A
git commit -m "feat: add change impact analysis"
```

不要 push。

不要修改 remote。

---

# 147. 最终状态

M6 完成后，Prism 的核心理解链条变成：

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
```

其中最关键的产品闭环：

```text
                    WHAT
                     │
                  M5 ChangeSet
                     │
                     ▼
                    WHY
                     │
              M3 Relationships
                     │
                     ▼
                   WHERE
                     │
              M6 Impact Analysis
                     │
             ┌───────┼────────┐
             ▼       ▼        ▼
          Modules   Tests    Docs
                     │
                     ▼
                  EVIDENCE
```

最终用户看到的不是一个简单的：

```text
“12 files changed”
```

而是：

```text
PaymentService.handle changed

Potentially affected:
  CheckoutService.process
  CheckoutController.submit
  CheckoutTest

Why:
  CheckoutService.process
      calls
  PaymentService.handle

  CheckoutController.submit
      calls
  CheckoutService.process

Confidence:
  High

Analysis limitations:
  2 unresolved relationships
```

这就是 M6 的核心价值：

> **M5 告诉 Prism “改了什么”，M6 让 Prism 开始告诉用户“可能波及哪里，以及证据是什么”。**

严格记住：

```text
M5 = What changed?
M6 = What may be affected?
M7 = What is unhealthy?
M8 = Can it actually be verified?
```

不要跨阶段偷功能。