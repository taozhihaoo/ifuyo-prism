# Prism M8b — Verify Adapters CI and Release 完整开发提示词

> 本文件由《Prism M8 — Verify Engine 完整开发提示词》拆分而来，是 **Part 2 / 2（下篇：Verify Adapters, CI and Release）**。
> 拆分为纯物理切分：原文正文逐字保留、未删减、未改写规范语义；原 §章节编号与标题保持原文，因此正文中的 "§xx" 交叉引用不失效。

## 【系列说明】

- 系列：Prism M8 — Verify Engine 完整开发提示词（拆分为上下两篇，两个独立可执行的开发提示词）
- 本篇：**Part 2 / 2 — M8b：Verify Adapters, CI and Release**
  覆盖：各生态 ToolAdapter（Node / Python / Dotnet / Rust / Generic）与结构化输出解析（JUnit / SARIF 等；保守解析原则：解析失败不改变 execution outcome）、CI 发现与静态解析、Release 检查、`prism verify checks / ci / release` 子命令、VerificationReport / VerificationSummary 与报告集成、`--changed` / TestCandidate 集成、结果 session 存储与 staleness 标记、真实项目验证、完整验收清单 + Final Audit + 最终 commit 指令。
- 姊妹篇：**Part 1 / 2 — M8a：Verify Engine Core**
  文件：`Spec/Prism M8a — Verify Engine Core 完整开发提示词.md`

### 原文 §区间 → 归属篇 映射

| 原文区间 | 归属篇 |
|---|---|
| 原文标题与引言（§0 之前） | M8a |
| §0 – §31 | M8a |
| §32 – §51 | M8b |
| §52 – §54 | M8a |
| §55 | M8b |
| §56 – §59 | M8a |
| §60 – §69 | M8b |
| §70 – §99 | M8a |
| §100 – §125 | M8b |
| §126 – §128 | M8a |
| §129 – §140 | M8b |
| §141 – §191 | M8a |
| §192 – §197 | M8b |
| §198 – §202 | M8a |
| §203 – §224 | M8b |
| §225 – §241 | M8a |
| §242 – §253 | M8b |
| §254 – §258 | M8a |
| §259 – §267 | M8b |
| §268 – §269 | M8a |
| §270 – §318 | M8b |

## 【两篇共同基线：定位、原则与红线】

以下定位、原则与红线对 M8a 与 M8b 同等生效，任一篇都不得削弱：

- **Prism 是 Verify Orchestrator，不是 Build/Lint/Test 工具作者。** 自己实现编译器 / 测试框架 / lint engine / type checker / CI runner 都是越界（原文 §0）。
- **Deterministic First / Evidence First / Real Execution > Prediction / Explicit Uncertainty / Local First。**
- **无 AI / 无 LLM**：M8 不引入任何 AI、模型调用或猜测式结论。
- **无遥测、无源码上传、无云端上报、无远程 CI 触发**：Prism 自身不做网络行为；网络只能来自项目命令自身的正常行为（§177–§178）。
- **隐私与脱敏**：不把完整环境变量写入报告/日志；对 TOKEN / PASSWORD / SECRET / KEY / API_KEY 等明显 secret patterns 必须脱敏，覆盖 stdout/stderr、command args、environment metadata、diagnostics（§17–§18、§77、§173–§176）。
- **Git 只读**：Verify 行为禁止 `git commit / reset / checkout / clean / push / pull / merge`（§83、§171、§217）；两篇各自的本地 commit 由完成门槛驱动，**绝不 push、绝不修改 remote**。
- **不自动修改源代码**：默认写策略只允许 ReadOnly / BuildArtifacts；ArbitraryWrite 必须用户显式配置（§82–§89）。
- **不确定就不执行**：安全性优先于自动化程度（§238、§296）。
- **边界不越界**：完整 Environment Engine 属 M9，AI 上下文属 M10，Snapshot 历史属 M11，UI 产品化属 M12，Release Hardening 属 M13；M8 不提前实现（§249–§253）。

## 【承接 M8a】

本篇假设 **Part 1（M8a）已实现并合入**，即以下能力已存在并通过其阶段性完成门槛，本篇直接复用、不重写：

```text
CheckSpec / CheckRun / CheckResult / CheckState / CheckKind / VerifyPlan / Prerequisite / CheckPolicy 核心模型
VerifyEngine 内部分层：CheckDiscovery / VerifyPlanner / CheckScheduler / CheckExecutor / OutputParser / ResultNormalizer
M0 CommandRunner 复用、命令执行安全（无 shell-by-default）、working directory / environment 语义
验证入口发现、source / confidence / provenance 语义、默认执行策略（仅 Exact / HighConfidence 自动执行）
VerifyPlan / 依赖图与 Blocked 语义、bounded concurrency
Timeout / Cancellation / 进程树终止（Windows / Linux / macOS）
WritePolicy、secret 脱敏、stdout/stderr 有界采集与截断标记
FakeCommandRunner 测试基座
--dry-run 与 `prism verify <path>` 基础执行面（含 --json / --check）
```

本篇在不削弱上述边界的前提下，把 Verify Engine 延伸到完整产品面：生态 Adapter 与结构化输出解析、CI 静态解析、Release 检查、报告与 GUI / CLI 集成、`--changed`、session 存储与 staleness 标记、真实项目验证；最后以 §315 Final Audit 与 §316 Completion Gate 完成整个 M8 里程碑验收，并执行 §317 最终 commit。

以下为原文正文（逐字保留，承接 M8a 的 §0–§31 等节，自 §32 起）：

---

# 32. Current-change verification

这是 Prism 很重要的能力。

支持：

```text
prism verify . --changed
```

但第一版必须谨慎。

不要简单：

```text
git diff
→ 自动改成 pytest selected tests
```

除非有可靠的 test-selection mapping。

---

# 33. `--changed` first version

推荐语义：

> 使用 ChangeSet / ImpactAnalysis 缩小“验证关注范围”，而不是擅自推断具体测试命令。

例如：

```text
Build
→ still full build

Test
→ candidate impacted tests may be shown

Lint
→ changed files if tool supports file arguments

Typecheck
→ project-level unless safe file-level mode exists
```

---

# 34. Never fake selective testing

如果：

```text
pytest tests/a.py
```

不能可靠判断：

> 这是修改代码应运行的全部测试。

那就显示：

```text
Candidate impacted tests
```

而不是：

```text
Verified affected tests
```

---

# 35. M6 integration

M6 提供：

```text
ChangedItem
ImpactedItem
TestCandidate
```

M8 可以消费：

```text
TestCandidate
```

但：

> TestCandidate ≠ TestSelectionTruth

必须保留这个边界。

---

# 36. Test candidate display

GUI：

```text
Potentially affected tests

tests/test_payment.py
tests/test_checkout.py
```

然后：

```text
Run selected
```

未来功能可以扩展。

M8 第一版可以允许：

```text
explicit user-selected checks
```

---

# 37. Build checks

支持常见项目：

### Node

```bash
npm run build
```

如果真实存在。

### Python

项目可能没有 build。

不要凭空生成。

### .NET

```bash
dotnet build
```

如果 project/solution 存在。

### Rust

```bash
cargo build
```

如果 Cargo workspace 存在。

---

# 38. Build semantics

Build：

```text
Command actually executed
+
exit code
```

才产生：

```text
Passed / Failed
```

---

# 39. Test checks

发现：

```text
npm test
npm run test
pytest
dotnet test
cargo test
```

但必须基于真实 project configuration。

不要：

```text
Python → pytest
```

盲猜。

---

# 40. Lint

同理：

```text
npm run lint
ruff check .
eslint .
dotnet format --verify-no-changes
cargo clippy
```

只在项目真实存在：

- config
- executable
- script
- convention

时发现。

---

# 41. Formatter verification

如果项目配置：

```text
Prettier
Ruff formatter
dotnet format
rustfmt
```

可以提供：

```text
FormatCheck
```

注意：

> `format --check` 和真正 formatting 不一样。

M8 绝不自动修改代码。

---

# 42. Typecheck

典型：

```text
TypeScript
tsc --noEmit
Python
mypy
pyright
C#
dotnet build
Rust
cargo check
```

但必须真实发现配置。

---

# 43. Static analysis

M8 可编排：

```text
clang-tidy
Roslyn analyzers
ruff
mypy
pyright
cargo clippy
```

但：

> Prism 不重新实现这些工具。

---

# 44. Third-party tool output

必须保存：

```text
tool
version if known
command
exit code
stdout/stderr
duration
```

同时尝试解析：

```text
diagnostics
file
line
column
message
severity
rule
```

---

# 45. Tool output parsing

建立：

```text
ToolOutputParser
```

支持：

```text
JSON
SARIF
JUnit XML
common compiler formats
text fallback
```

第一版不要求全部实现。

---

# 46. Parse structured output first

优先顺序：

```text
SARIF / JSON / XML
>
machine-readable CLI
>
stable textual output
>
human textual fallback
```

不要优先解析：

```text
localized human output
```

---

# 47. Localization safety

例如：

```text
Build succeeded
生成成功
Erfolgreich
```

不能依赖英文：

```text
"success" in stdout
```

来判断是否成功。

优先：

```text
exit code
structured output
```

---

# 48. Exit code truth

默认：

```text
exit_code == 0
→ success
```

但如果特定工具：

```text
0 = warnings
1 = findings
2 = execution error
```

需要 tool-specific adapter。

不要硬编码所有工具都一样。

---

# 49. Tool adapter

建议：

```rust
ToolAdapter
```

负责：

```text
detect
build command
interpret exit code
parse output
```

而：

```text
VerifyEngine
```

负责 orchestration。

---

# 50. Adapter boundary

例如：

```text
Prism
  ↓
NpmAdapter
  ↓
CommandRunner
```

而不是：

```text
Prism
  ↓
npm-specific giant logic
```

---

# 51. Keep adapters small

例如：

```text
NodeVerificationAdapter
PythonVerificationAdapter
DotnetVerificationAdapter
RustVerificationAdapter
GenericCommandAdapter
```

不要：

```text
UniversalBuildAnalyzer
```

巨大类。

---

# 55. `prism verify checks`

支持：

```bash
prism verify checks .
```

展示：

```text
Build      npm run build
Test       npm test
Lint       npm run lint
Typecheck  npm run typecheck
```

以及：

```text
source:
project config
detected
user configured
```

---

# 60. Verification profile

建议提供：

```text
default
ci
release
custom
```

但第一版尽量简单。

例如：

```yaml
verify:
  profile: default
```

---

# 61. Release profile

Release 检查未来可以：

```text
build
tests
lint
typecheck
Git clean
required docs
version consistency
```

但是：

> 不能把“release readiness”变成一个神秘分数。

输出：

```text
PASS
FAIL
BLOCKED
```

以及证据。

---

# 62. Release check

建立：

```text
ReleaseCheck
```

优先实现：

```text
version present
required files present
build passed
test passed
working tree clean
```

但只基于已有事实。

---

# 63. Release != deployment

禁止：

```text
prism release
→ upload
→ publish
→ deploy
```

M8 只判断：

> 本地项目是否满足定义好的 release verification conditions。

---

# 64. CI configuration inspection

M8 可以分析：

```text
.github/workflows/*.yml
.gitlab-ci.yml
Jenkinsfile
etc.
```

第一版重点：

> 读取 CI configuration，理解它声明了哪些 checks。

例如：

```text
CI runs:
npm ci
npm run lint
npm test
npm run build
```

---

# 65. CI != remote execution

Prism 不应该：

```text
GitHub Actions trigger
```

也不要：

```text
GitLab pipeline trigger
```

除非未来单独设计 cloud integration。

M8：

> **CI inspection, not CI hosting.**

---

# 66. CI drift

这是一个非常有价值的验证能力：

项目本地：

```text
npm test
npm run build
```

CI：

```text
npm run lint
npm test
npm run build
```

Prism 可以显示：

```text
CI checks detected
Local verification coverage
```

但第一版不要轻率地判：

```text
CI is wrong.
```

---

# 67. Local-vs-CI comparison

输出：

```text
Local:
build
test
lint

CI:
test
lint
build

Shared:
build
test
lint
```

这是事实。

如果 CI 独有：

```text
security scan
```

标记：

```text
CI-only check
```

---

# 68. Verification coverage

可以产生：

```text
VerificationCoverage
```

但不是：

```text
87% project verified
```

而是：

```text
Build: verified
Tests: verified
Lint: verified
Typecheck: unavailable
CI security scan: configured remotely
```

---

# 69. Verification status matrix

例如：

| Check | Local | CI Config | Current Result |
|---|---|---|---|
| Build | Available | Present | Passed |
| Test | Available | Present | Passed |
| Lint | Available | Present | Failed |
| Typecheck | Missing | Present | Unavailable |

这比：

```text
Verification Score: 76
```

可靠得多。

---

# 100. Diagnostics

统一：

```text
Diagnostic {
    severity,
    message,
    location,
    code,
    source,
}
```

复用已有 diagnostic model。

例如：

```text
pytest:
FAILED tests/test_a.py::test_x

```

转换：

```text
file = tests/test_a.py
message = ...
source = pytest
```

---

# 101. Diagnostic parsing must be conservative

如果 parser 不确定：

不要猜 line/column。

输出：

```text
location = unknown
raw reference = ...
```

比假造：

```text
line 42
```

更好。

---

# 102. Structured formats

优先支持：

### SARIF

### JUnit XML

### JSON

这些格式信息最完整。

第一版可以只实现其中真实需求最高的 1–2 种。

---

# 103. Human text fallback

对于无法结构化解析的命令：

保留：

```text
stdout/stderr
exit code
```

而不是强行 regex 成大量 diagnostics。

---

# 104. Verify Report

建立：

```rust
VerificationReport {
    project_id,
    plan,
    runs,
    summary,
    diagnostics,
    completeness,
}
```

---

# 105. VerificationSummary

例如：

```text
VerificationSummary {
    total_checks,
    passed,
    failed,
    skipped,
    blocked,
    unavailable,
    cancelled,
    timed_out,
}
```

---

# 106. Overall status

例如：

```text
Passed
Failed
Partial
Cancelled
```

不能只：

```text
boolean
```

---

# 107. Evidence

每个 CheckRun 至少：

```text
command
working_directory
exit_code
duration
tool
tool_version if known
stdout/stderr or references
parsed diagnostics
```

---

# 108. Do not copy output twice

避免：

```text
CommandResult
+
CheckRun
+
CheckResult
```

把完整 stdout/stderr 存三遍。

统一引用或所有权。

---

# 109. Reports

支持：

```text
JSON
Markdown
HTML
```

复用 M7 report layer。

---

# 110. JSON example

```json
{
  "kind": "test",
  "name": "npm test",
  "status": "Passed",
  "exit_code": 0,
  "duration_ms": 18234,
  "diagnostics": [],
  "completeness": "Complete"
}
```

---

# 111. Markdown example

```markdown
# Prism Verification Report

## Summary

- Passed: 4
- Failed: 1
- Blocked: 0
- Unavailable: 1

## Build

PASS

Command:
npm run build

Exit code:
0

## Test

FAIL

Command:
npm test

Exit code:
1

Diagnostics:
...
```

---

# 112. GUI Verify page

从 placeholder 变成真实功能。

顶部：

```text
Verify

Passed   4
Failed   1
Blocked  0
Unavailable 1
```

---

# 113. Check cards

每个：

```text
BUILD
npm run build

PASSED
18.2s
```

失败：

```text
TEST
npm test

FAILED
23.4s
```

---

# 114. Verify detail

点击：

```text
TEST
```

展示：

```text
Command
Working directory
Status
Exit code
Duration
Tool version
Diagnostics
Output
```

---

# 115. Output viewer

stdout/stderr：

- 可展开
- 可搜索
- 截断标记
- monospaced
- 不默认全屏占据 UI

---

# 116. Diagnostics navigation

如果 test/lint parser 能定位：

```text
tests/test_payment.py:42
```

点击：

> 外部编辑器打开。

不做 IDE。

---

# 117. Verification plan UI

执行前允许用户看到：

```text
Verify Plan

Build
Test
Lint
Typecheck
```

并看到：

```text
reason/source
dependency
```

这样：

> Prism 不会偷偷执行一堆命令。

---

# 118. Explicit execution

建议：

```text
Run all
Run selected
Run failed
Run unavailable
```

第一版至少：

```text
Run all
Run selected
Cancel
```

---

# 119. Re-run

失败后：

```text
Run again
```

允许重新生成新的 CheckRun。

不要覆盖旧的 in-memory result。

---

# 120. Current verification context

GUI 可以显示：

```text
Current Git:
branch main
8 changed files

Current ChangeSet:
21 changed symbols

Impact:
14 potentially affected modules

Health:
3 change-related findings
```

然后：

```text
Verify
```

这是 Prism 主链的实际落地。

---

# 121. Change-aware verification result

执行：

```text
verify
```

完成后可以关联：

```text
check_run.change_set_id
check_run.impact_analysis_id
```

如果当前架构支持。

---

# 122. Causality caution

如果：

```text
Build failed
```

同时：

```text
M7 finding exists
```

不要自动写：

> Build failed because of this finding.

除非真实 compiler/test diagnostics 明确指向该问题。

---

# 123. M7 + M8 integration

可以在 UI：

```text
Health
  ↓
Finding
  ↓
Verify
```

例如：

```text
SEC001
```

旁边：

```text
No verification relation
```

不要伪造。

---

# 124. Verify-to-health relation

如果一个 lint tool 自己报告：

```text
complexity exceeded
```

可以把：

```text
CheckRun diagnostic
```

映射为：

```text
verification diagnostic
```

但：

> 不要自动变成 M7 Finding，除非后续明确设计统一 ingestion。

第一版保持两个事实层。

---

# 125. Verify-to-change relation

如果某个 check 是：

```text
--changed
```

报告：

```text
scope = changed
change_set_id = ...
```

---

# 129. No persistence yet

M8 第一版：

> 不需要数据库。

运行结果保存在：

```text
current session
report file
```

未来：

```text
M11 Snapshot
```

再处理历史。

---

# 130. Session vs Snapshot

M8 当前：

```text
Run
```

产生：

```text
CheckRun
```

退出程序后可以消失。

M11：

```text
Snapshot
```

保存长期历史。

不要现在做数据库。

---

# 131. CI configuration model

可以建立轻量：

```rust
CIConfig
CIJob
CICheck
```

记录：

```text
provider/config_file
job
commands
dependencies
```

不需要完整 YAML execution engine。

---

# 132. CI parser

优先支持：

```text
GitHub Actions
```

如果项目实际主要使用它。

后续：

```text
GitLab
Jenkins
Azure Pipelines
```

不要第一版全部实现。

---

# 133. CI YAML safety

YAML 中：

```yaml
run: |
  npm ci
  npm test
  npm build
```

可以静态解析。

不要执行 YAML 中所有命令：

> CI inspection ≠ CI execution。

---

# 134. CI command extraction

提取：

```text
npm test
npm run lint
npm run build
```

并保留：

```text
job
step
file
line
```

作为 evidence。

---

# 135. CI drift finding

M8 可以输出：

```text
CIRule:
Local verification does not include a CI-only check
```

但是：

> 第一版最好作为 informational comparison，而不是 failure。

---

# 136. Release checks

建议第一版支持：

```text
required documentation exists
working tree clean
version metadata present
build passed
required tests passed
```

具体项目规则允许配置。

---

# 137. Version consistency

只有在项目明确存在：

```text
package.json version
Cargo.toml version
.csproj version
pyproject.toml version
```

等多个来源时，才能检查：

```text
version consistency
```

不能假定所有项目都必须在多个地方同步版本。

---

# 138. Required files

Release profile 可以配置：

```yaml
release:
  required_files:
    - README.md
    - LICENSE
```

如果缺失：

```text
ReleaseCheck = Failed
```

---

# 139. Release cleanliness

读取 M4：

```text
working_tree
```

如果要求 clean：

```text
clean = pass
dirty = failed
```

但：

> 不能自动 clean。

---

# 140. Verify profile configuration

推荐：

```yaml
verify:
  checks: ...
  required: ...
  release: ...
```

保持与 M7 config architecture 一致。

---

# 192. Node fixture

至少准备：

```text
package.json

scripts:
build
test
lint
typecheck
```

并真实提供：

```text
pass
fail
```

场景。

---

# 193. Python fixture

覆盖：

```text
pyproject.toml
pytest
ruff
mypy/pyright
```

只配置真正安装的工具。

---

# 194. C# fixture

覆盖：

```text
csproj
solution
dotnet build
dotnet test
```

---

# 195. Rust fixture

覆盖：

```text
Cargo.toml
cargo check
cargo test
cargo clippy
```

如果 CI/environment 可用。

---

# 196. CI fixture

GitHub Actions：

```yaml
jobs:
  test:
    steps:
      - run: npm test
      - run: npm run lint
```

确认：

```text
job
step
command
source location
```

全部正确。

---

# 197. Release fixture

测试：

```text
clean repo
dirty repo
build success
test success
required file missing
version mismatch
```

---

# 203. Real project validation

至少：

### Prism 自身

### 一个 TypeScript 项目

### 一个 Python 项目

### 一个 C# 项目

### 一个 Rust 项目（环境允许时）

真实运行：

```text
build
test
lint/typecheck where available
```

---

# 204. Self verification

Prism 最终能够：

```bash
prism verify .
```

验证自己：

```text
Rust build
Rust tests
Frontend build
frontend tests/lint if configured
```

具体根据真实 project structure。

---

# 205. Self-verification importance

这一步必须回答：

> Prism 自己能不能通过 Prism 的 Verify Engine 验证？

如果不能：

> 必须记录限制，不得为了“全绿”修改 detection semantics。

---

# 206. Real command evidence audit

最终随机抽取 CheckRun：

确认：

```text
command matches actual process
exit code matches process
duration plausible
status matches semantics
stdout/stderr not swapped
working directory correct
```

---

# 207. No fake adapters

如果工具没有安装：

不要写：

```text
fake adapter result
```

必须：

```text
Unavailable
```

---

# 208. No fake CI result

如果 CI 文件写：

```text
npm test
```

只说明：

```text
CI config contains npm test
```

不能：

```text
CI test passed
```

---

# 209. No fake release result

如果：

```text
README
LICENSE
build command
```

都存在：

不能直接：

```text
Release passed
```

必须根据真正执行的 required checks。

---

# 210. No “verified by static analysis”

静态分析：

```text
M7
```

执行验证：

```text
M8
```

必须在 UI 中分开。

---

# 211. Verify status colors

GUI 可以：

```text
Passed
Failed
Blocked
Unavailable
```

使用状态颜色。

但不要：

```text
大型仪表盘
神秘分数
游戏化
```

---

# 212. Overview integration

Overview 可以显示：

```text
Last Verification

Build     Passed
Test      Passed
Lint      Failed
```

如果当前 session 有结果。

退出后不要假装仍然是最新结果，除非有持久化来源。

---

# 213. Staleness

非常重要：

如果代码发生新的 Git changes：

之前：

```text
Build passed
```

可能已经不适用于当前代码。

Overview 应显示：

```text
Verification is stale
```

如果可以通过：

```text
ChangeSet
```

准确判断。

---

# 214. Verification fingerprint

可以使用：

```text
project state
+
check spec
+
relevant configuration
```

生成：

```text
verification input fingerprint
```

这样可以判断：

> 上一次验证是不是针对当前代码状态。

但第一版可以只做轻量：

```text
verified_commit
```

对于 dirty working tree：

> 标记为 dirty verification.

---

# 215. Dirty verification

如果：

```text
working tree dirty
```

运行：

```text
npm test
```

结果仍然有效。

应该表示：

```text
Passed
scope = dirty working tree
```

不能：

```text
verification invalid
```

只是：

> verification state tied to current dirty workspace.

---

# 216. Revision verification

第一版不要支持：

```text
verify arbitrary old Git commit
```

除非已经有成熟：

```text
RevisionSourceProvider
```

M5 有这个设计基础，但执行 verify 可能需要：

> 隔离 worktree。

不要为了支持历史 commit：

```text
git checkout
```

修改当前 worktree。

优先留后续。

---

# 217. No Git mutation for historical verify

严格禁止：

```bash
git checkout <old>
```

然后回来。

---

# 218. Worktree possibility

未来：

```text
isolated verification worktree
```

可以支持。

不是 M8 第一版必需。

---

# 219. Report stale semantics

如果当前代码发生变化：

原报告：

```text
Build passed
```

仍然保存为：

```text
historical run
```

但不要显示：

```text
Current build passed
```

---

# 220. Verify report metadata

至少：

```text
project
commit if clean
dirty status
command
tool versions
start/end
```

---

# 221. Tool version

如果能低成本获取：

```text
node --version
npm --version
python --version
dotnet --version
cargo --version
```

可以附加。

如果获取失败：

```text
unknown
```

不要阻止验证。

---

# 222. No version probing explosion

不要：

```text
每个 check 前 probe 20 个工具
```

只检查：

> 当前 CheckSpec 所需工具。

---

# 223. Check discovery cache

第一版不要持久化。

当前 session：

```text
Discovery
→ plan
```

足够。

---

# 224. Check result cache

M8 第一版不要偷偷缓存：

```text
npm test passed
```

然后代码发生变化仍复用。

验证结果必须：

> 来自明确的一次执行。

未来 M11 可以保存历史，但不是缓存冒充新结果。

---

# 242. CI commands never auto-execute blindly

即使 CI 中：

```yaml
run: npm test
```

也不能因为：

```text
CIConfig
```

就自动执行所有命令。

只有：

> 本地 Verify adapter 明确认可的检查。

---

# 243. Release profile safety

Release profile 不得默认：

```text
publish
pack
upload
```

只：

```text
validate
```

---

# 244. Package artifact check

例如：

```bash
npm pack --dry-run
```

可以作为：

```text
release check
```

如果项目适用。

但是：

> `npm pack` 本身也可能生成 tarball。

优先：

```text
dry-run
```

---

# 245. No deployment

再次：

```text
verify release
≠
deploy
```

---

# 246. Health → Verify workflow

建议最终 GUI workflow：

```text
Health
  ↓
3 findings
  ↓
inspect
  ↓
Run verification
  ↓
Build
Test
Lint
  ↓
actual results
```

---

# 247. Changes → Verify workflow

```text
Changes
  ↓
ChangeSet
  ↓
Impact
  ↓
Potentially affected tests
  ↓
Verify
```

但必须标：

```text
Potential
```

不要声称：

> 这是完整测试集合。

---

# 248. Overview workflow

最终 Overview：

```text
Project
────────────────────

Current Changes
8 files

Potential Impact
14 modules

Health
3 findings

Verification
Build   PASS
Test    PASS
Lint    FAIL
```

这就是 Prism 的工程闭环。

---

# 249. Final product semantic chain

M7：

```text
What should I pay attention to?
```

M8：

```text
Did the project actually pass its verification?
```

因此最终：

```text
M1  What is here?
M2  What is the code?
M3  How is it connected?
M4  What is Git saying?
M5  What changed?
M6  What may be affected?
M7  What should I pay attention to?
M8  Did it actually verify?
```

---

# 250. M8 / M9 Boundary

M8：

```text
What tools does this check require?
Can I execute this check?
What was the result?
```

M9：

```text
What is the complete environment?
What SDKs/runtimes/tools are installed?
What processes/ports exist?
What environment variables/configuration exist?
```

---

# 251. M8 / M10 Boundary

M8：

```text
CheckRun
CheckResult
Diagnostics
```

M10：

```text
Which verification facts should AI receive?
What context explains the failure?
```

---

# 252. M8 / M11 Boundary

M8：

```text
Current verification execution
```

M11：

```text
Verification history
Pass/fail trend
Finding evolution
```

---

# 253. M8 / M13 Boundary

M8：

```text
can execute locally
```

M13：

```text
packaging
installer
cross-platform release hardening
distribution
```

---

# 259. Generic command adapter

必须有一个最小：

```text
GenericCommandAdapter
```

支持：

```text
program
args
cwd
env
timeout
```

这能让 Prism：

> 不认识所有生态也能验证项目。

---

# 260. Native ecosystem adapters

至少：

```text
Node
Python
Dotnet
Rust
```

根据现有环境决定是否全部实现。

不能为了“覆盖率”写假的 adapter。

---

# 261. Node adapter

真实支持：

- package scripts
- npm / pnpm / yarn detection
- build/test/lint/typecheck based on scripts

但：

> 不应该偏爱 npm。

如果项目使用 pnpm：

> 正确使用 pnpm。

---

# 262. Python adapter

识别：

```text
pyproject.toml
pytest.ini
tox.ini
ruff
mypy
pyright
```

命令以：

> 项目真实配置为准。

---

# 263. C# adapter

识别：

```text
.sln
.csproj
Directory.Build.props
```

支持：

```text
dotnet build
dotnet test
```

其它 analyzers 以后扩展。

---

# 264. Rust adapter

识别：

```text
Cargo.toml
workspace
```

支持：

```text
cargo check
cargo build
cargo test
cargo clippy
```

如果工具实际存在。

---

# 265. CI adapter

第一版：

```text
GitHubActionsAdapter
```

抽取：

```text
workflow
job
step
run
uses
```

重点：

> run commands。

---

# 266. CI line evidence

每个 command：

```text
.github/workflows/ci.yml:42
```

这样 M8 GUI：

> 可以定位 CI 配置。

---

# 267. Report source provenance

所有 CheckSpec：

```text
source
confidence
```

所有 CheckRun：

```text
actual_execution
```

严格区分。

---

# 270. Progress stages

UI 可以显示：

```text
Discovering checks
Planning verification
Running Build
Running Tests
Running Lint
Collecting results
```

---

# 271. Real progress only

不要：

```text
fake percentage
```

只显示真实阶段。

---

# 272. Check-level progress

如果工具无法提供：

```text
0–100%
```

只显示：

```text
Running...
```

不要伪造。

---

# 273. Result persistence within session

GUI 中：

```text
Run history
```

可以保存本次应用 session 内：

```text
CheckRun list
```

但不需要数据库。

---

# 274. Session run list

例如：

```text
Verification runs

10:42 Build PASS
10:42 Test PASS
10:43 Lint FAIL
```

关闭应用后可以消失。

---

# 275. No misleading “last verified forever”

Overview 若无持久化历史：

```text
No verification run in this session
```

而不是：

```text
Last verified yesterday
```

---

# 276. Current-code validity

可以把每次 Verify 标记：

```text
verified_state:
CleanAtCommit
DirtyWorkspace
```

如果 M4 已提供 status。

---

# 277. Verification against changed state

如果 dirty：

```text
verified_state = DirtyWorkspace
```

并显示：

```text
8 working-tree changes present at verification time.
```

---

# 278. Failure diagnostics integration

如果：

```text
compiler
```

输出：

```text
src/foo.cs(42,13): error CS0103
```

应转换：

```text
severity = error
code = CS0103
file = src/foo.cs
line = 42
column = 13
source = dotnet
```

---

# 279. Diagnostic normalization

统一：

```text
Diagnostic {
    source,
    code,
    severity,
    message,
    location,
    related_locations,
}
```

复用已有 core diagnostic。

---

# 280. Error vs diagnostic

区分：

```text
Diagnostic:
tool says code has an issue.

Engine Error:
Prism failed to run/parse the tool.
```

---

# 281. Verification findings vs health findings

Do not automatically merge:

```text
M8 Diagnostic
```

into：

```text
M7 Finding
```

他们属于：

```text
different lifecycle
```

可在 UI 联动，但模型保持分离。

---

# 282. Verification report export

CLI：

```bash
prism verify . --json > verify.json
```

或者复用现有 report output conventions。

---

# 283. Output file safety

如果：

```text
--output verify.json
```

不要默认：

```text
overwrite without warning
```

具体遵循现有 report conventions。

---

# 284. JSON schema stability

字段：

```text
status
kind
name
command
exit_code
diagnostics
```

应保持：

> machine-readable。

不要用纯 human strings 承担核心语义。

---

# 285. Enum serialization

例如：

```json
"status": "timed_out"
```

不要：

```json
"status": "Oops, this command took too long"
```

---

# 286. Human message separate

可以：

```json
{
  "status": "timed_out",
  "message": "Verification exceeded configured timeout."
}
```

---

# 287. Test parser stability

对于 parser：

> 一个工具版本升级不应轻易让 Prism 崩溃。

解析失败：

```text
diagnostic
+
raw output fallback
```

---

# 288. Output parser no panic

必须：

```text
malformed JSON
malformed XML
unexpected text
```

都不会：

> panic。

---

# 289. CI YAML malformed

如果 workflow YAML invalid：

```text
CI inspection failed
```

但：

> 不影响 local Build/Test。

---

# 290. One check parser failure

如果：

```text
JUnit parser failed
```

不要让：

```text
CheckRun itself vanish
```

仍然保留：

```text
exit code
stdout/stderr
parser diagnostic
```

---

# 291. Build succeeded but parser failed

这种情况：

```text
exit_code = 0
parser_error = true
```

总体：

```text
execution outcome = Passed
analysis completeness = Partial
```

而不是：

```text
Failed
```

因为：

> command 实际成功。

---

# 292. Test returned zero but output says failures

除非工具定义明确：

> 以 exit code 为主要执行结果。

不要自行把文字：

```text
2 tests failed
```

判成 Failed if exit=0。

如果结构化 output 明确表示 test failures：

> tool adapter 可使用它。

---

# 293. Tool-specific semantics

这是 adapter 存在的核心价值：

```text
Generic:
exit code

Tool-specific:
exit + structured result
```

---

# 294. No universal output parser

不要：

```text
one giant regex
```

解析所有工具。

---

# 295. Verify engine extensibility

以后增加：

```text
Java
Go
C++
```

应该只需要：

```text
adapter
```

而不是重写：

```text
VerifyEngine
```

---

# 296. Default safety

如果 Prism 不确定：

```text
Do not execute.
```

展示：

```text
Ambiguous check detected.
```

安全性优先于自动化程度。

---

# 297. User agency

任何会：

- 执行项目命令
- 写文件
- 使用网络
- 消耗大量资源

的行为：

> GUI 必须有明确可见的执行意图。

不要后台偷偷跑。

---

# 298. First-run UX

第一次打开项目：

```text
No verification run yet.

Detected checks:
Build
Test
Lint
Typecheck

[Review plan]
[Run verification]
```

不要第一次：

> 直接启动所有脚本。

---

# 299. User-selected execution

即使默认可以自动执行 Exact checks：

GUI 仍应：

> 给用户可见的执行范围。

CLI 才适合：

```text
prism verify .
```

直接执行。

---

# 300. Verify Engine golden path

最终必须真实跑通：

```text
Open Project
    ↓
Discover Checks
    ↓
Review Plan
    ↓
Run
    ↓
Build PASS
    ↓
Test PASS
    ↓
Lint FAIL
    ↓
Diagnostic
    ↓
Summary
```

---

# 301. Golden path #2

项目没有 lint：

```text
Build PASS
Test PASS
Lint UNAVAILABLE
```

整体：

```text
Partial
```

不是：

```text
Failed
```

---

# 302. Golden path #3

Build fail：

```text
Build FAIL
Test BLOCKED
Lint PASS
```

整体：

```text
Failed
```

---

# 303. Golden path #4

Tool missing：

```text
Typecheck UNAVAILABLE
```

如果 optional：

```text
overall can still PASS
```

如果 required：

```text
overall = Failed/Incomplete
```

按明确 policy。

---

# 304. Golden path #5

Cancel：

```text
Build PASS
Test CANCELLED
Lint CANCELLED
```

整体：

```text
Cancelled
```

---

# 305. Golden path #6

Timeout：

```text
Test TIMED_OUT
child process terminated
```

确认：

> 没有 orphan process。

---

# 306. Golden path #7

Dry-run：

```text
prism verify . --dry-run
```

输出完整 plan：

> 零命令执行。

---

# 307. Golden path #8

Current changes：

```text
M5
+
M6
```

然后：

```text
prism verify . --changed
```

显示：

```text
scope = current changes
impacted test candidates
```

但不冒充完整 selective verification。

---

# 308. Golden path #9

Release：

```text
Build PASS
Test PASS
Required docs PASS
Git clean PASS
Version metadata PASS
```

最终：

```text
Release verification = PASS
```

---

# 309. Golden path #10

Release dirty：

```text
Build PASS
Test PASS
Git clean FAIL
```

最终：

```text
Release verification = FAIL
```

但：

> 不执行 `git clean`。

---

# 310. Golden path #11

Secret in command args：

```text
command = tool --token abc123
```

报告必须：

```text
tool --token [REDACTED]
```

---

# 311. Golden path #12

Huge stdout：

```text
10MB
```

报告：

```text
stdout truncated
```

且 Prism 内存受控。

---

# 312. Golden path #13

Malformed structured output：

```text
exit 0
invalid JSON
```

结果：

```text
execution = Passed
parser = Partial/Error
raw output retained according to policy
```

---

# 313. Golden path #14

CI config：

```text
.github/workflows/ci.yml
```

解析：

```text
build
test
lint
```

不执行任何 CI command。

---

# 314. Golden path #15

Prism self verify：

```bash
prism verify .
```

验证 Prism 自己。

---

# 315. Final Audit

完成后输出：

# Prism M8 Final Verify Audit

## A. Baseline

真实 M0–M7 状态：

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

## B. Verify Architecture

确认：

```text
Discovery
→ Planning
→ Scheduling
→ Execution
→ Parsing
→ Normalization
→ Reporting
```

边界正确。

---

## C. Command Execution

确认：

- CommandRunner reuse
- cwd
- args
- environment
- timeout
- cancellation
- process cleanup
- security

---

## D. Check Model

确认：

```text
CheckSpec
CheckRun
CheckResult
CheckDependency
```

职责清晰。

---

## E. Check Discovery

确认：

- Node
- Python
- C#
- Rust
- Generic
- CI
- custom config

实际实现情况。

---

## F. Prerequisites

确认：

```text
Satisfied
Unsatisfied
Unknown
```

语义正确。

---

## G. Execution States

确认：

```text
Passed
Failed
Skipped
Blocked
Unavailable
Cancelled
TimedOut
Error
```

---

## H. Output Parsing

确认：

- stdout
- stderr
- exit code
- structured output
- diagnostics
- parser failures
- truncation

---

## I. Security

确认：

```text
No secret leakage
No environment dump
No command injection
No shell-by-default
No Git mutation
No automatic source modification
```

---

## J. CI

确认：

```text
CI config inspection
command extraction
provenance
location
No remote execution
```

---

## K. Release

确认：

```text
required checks
Git cleanliness
required files
version checks where implemented
No deployment
```

---

## L. Change Integration

确认：

```text
M5 ChangeSet
M6 ImpactAnalysis
```

能够被：

```text
--changed
```

以及 test candidate / verification scope 正确消费。

---

## M. Health Integration

确认：

```text
M7 Findings
```

与：

```text
M8 CheckRuns
```

保持两个独立事实层。

---

## N. GUI

确认：

```text
Verify plan
Run
Cancel
Check list
Detail
Diagnostics
Output
Current change scope
Release
CI inspection
```

---

## O. CLI

真实执行：

```bash
prism verify .
prism verify . --dry-run
prism verify . --json
prism verify . --changed
prism verify checks .
```

并验证：

```text
stdout
stderr
exit code
JSON
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
command provenance
exit code
duration
diagnostics
redaction
truncation
```

---

## Q. Tests

必须真实统计：

```text
unit
executor
planner
scheduler
adapter
parser
CLI
GUI
cross-platform
timeout
cancellation
security
fixtures
real projects
```

---

## R. Real Projects

至少：

```text
Prism
TypeScript
Python
C#
```

以及 Rust（环境允许时）。

---

## S. Performance

记录真实：

```text
check count
execution time
parallelism
stdout size
memory observations
```

---

## T. Git

确认：

```text
working tree
generated artifacts
temporary files
reports
tests
```

没有误提交。

---

## U. Risks

只列：

```text
P0
P1
```

---

## V. Deferred

不要偷偷做：

```text
M9 Environment Engine
M10 Context Engine
M11 Snapshot
M12 UI Productization
M13 Release Hardening
```

尤其不要现在引入：

```text
database
cloud CI
remote execution
AI
RAG
MCP
deployment
```

---

# 316. Completion Gate

M8 只有全部满足以下条件才算完成：

```text
[ ] M0 regression passes
[ ] M1 regression passes
[ ] M2 regression passes
[ ] M3 regression passes
[ ] M4 regression passes
[ ] M5 regression passes
[ ] M6 regression passes
[ ] M7 regression passes

[ ] CheckSpec exists
[ ] CheckRun exists
[ ] CheckResult exists
[ ] VerifyPlan exists
[ ] Check discovery works
[ ] Check provenance works
[ ] Confidence semantics work
[ ] Prerequisites work
[ ] Check dependency ordering works
[ ] Bounded concurrency works
[ ] Cancellation works
[ ] Timeout works
[ ] Process tree cleanup works

[ ] Node verification works where configured
[ ] Python verification works where configured
[ ] C# verification works where configured
[ ] Rust verification works where configured
[ ] Generic custom check works

[ ] Build works
[ ] Test works
[ ] Lint works where configured
[ ] Formatter check works where configured
[ ] Typecheck works where configured
[ ] Static analysis works where configured

[ ] Unavailable semantics work
[ ] Blocked semantics work
[ ] Failed semantics work
[ ] Cancelled semantics work
[ ] TimedOut semantics work
[ ] Error semantics work

[ ] Structured output parsing works where implemented
[ ] Diagnostic normalization works
[ ] stdout/stderr handling works
[ ] output truncation works
[ ] secret redaction works

[ ] CI inspection works
[ ] No remote CI execution
[ ] Release verification works
[ ] No deployment
[ ] No Git mutation
[ ] No automatic source modification

[ ] --dry-run works
[ ] --changed works
[ ] M5/M6 integration works
[ ] M7/M8 separation works

[ ] CLI works
[ ] GUI works
[ ] JSON works
[ ] Markdown works
[ ] HTML works where supported

[ ] Prism self-verification works
[ ] Real project validation completed

[ ] No AI
[ ] No telemetry
[ ] No source upload
[ ] No database introduced for M8
[ ] No cloud CI
[ ] No deployment system
[ ] No P0
[ ] No unresolved blocking P1
```

---

# 317. Final Commit

全部通过后：

```bash
git add -A
git commit -m "feat: add verify engine"
```

不要 push。

不要修改 remote。

---

# 318. Prism M0–M8 主链最终形态

M8 完成后：

```text
M0
Foundation
        ↓
M1
Discovery
        ↓
M2
Code Understanding
        ↓
M3
Dependency / Architecture
        ↓
M4
Git
        ↓
M5
ChangeSet
        ↓
M6
Impact
        ↓
M7
Health
        ↓
M8
Verify
```

产品语义变成：

```text
M1  它是什么？
M2  它由什么代码构成？
M3  它们怎么关联？
M4  Git 现在是什么状态？
M5  发生了什么变化？
M6  可能影响哪里？
M7  有哪些值得注意的问题？
M8  实际验证结果是什么？
```

而 Prism 的核心闭环开始真正成立：

```text
┌───────────────┐
│   Understand  │
└───────┬───────┘
        ↓
┌───────────────┐
│    Changes    │
└───────┬───────┘
        ↓
┌───────────────┐
│    Impact     │
└───────┬───────┘
        ↓
┌───────────────┐
│    Health     │
└───────┬───────┘
        ↓
┌───────────────┐
│    Verify     │
└───────┬───────┘
        ↓
┌───────────────┐
│   Evidence    │
└───────────────┘
```

最重要的产品原则保持不变：

```text
Deterministic First
Evidence First
Real Execution > Prediction
Explicit Uncertainty
Local First
No AI Dependency
No Telemetry
No Automatic Modification
```

因此 M8 的最终目标不是：

> “Prism 给项目打一个通过分数。”

而是：

> **“Prism 清楚地告诉你：它发现了哪些验证入口、实际执行了什么、执行在哪里、用了什么工具、结果是什么、哪里失败、哪里没执行，以及这些结论各自有什么证据。”**

做到这里，Prism 就从一个**项目分析器**开始真正成为一个**工程验证工作台**。---

## 【拆分尾注】

本篇（M8b，Part 2/2）与姊妹篇《Prism M8a — Verify Engine Core 完整开发提示词.md》（Part 1/2）按篇首映射表合并，即完整原文《Prism M8 — Verify Engine 完整开发提示词》，无增删、无改写。

- 上篇负责 Verify Engine Core（核心模型、分层、发现、计划、执行、`--dry-run`、测试基座、`prism verify` 基础执行面）。
- 本篇负责生态 Adapter 与结构化输出解析、CI / Release、报告与 GUI / CLI 完整集成、真实项目验证，以及 §315 Final Audit、§316 Completion Gate、§317 最终 commit（`feat: add verify engine`）。
