# Prism M8 — Verify Engine
## 从“项目健康分析”到“真实验证项目”

你现在进入 **Prism M8：Verify Engine**。

M0–M7 已建立：

```text
M1 = File Facts
M2 = Code Facts
M3 = Relationship Facts
M4 = Git Facts
M5 = Change Facts
M6 = Impact Facts
M7 = Health Facts
```

M8 建立：

```text
Verification Facts
```

核心变化：

```text
M7:
“静态上看，有哪些值得注意的问题？”

M8:
“这个项目实际上能不能通过它声明/配置的验证？”
```

核心链：

```text
Project
  ↓
Discovery
  ↓
Code Model
  ↓
Relationship Graph
  ↓
ChangeSet
  ↓
ImpactAnalysis
  ↓
Health
  ↓
Verify
  ↓
CheckRun
  ↓
CheckResult
  ↓
VerificationReport
```

M8 的核心价值不是“自己重写所有工具”，而是：

> **发现项目应该如何被验证 → 调度真实工具 → 收集真实结果 → 统一结果模型 → 保留证据 → 给出明确的验证结论。**

---

# 0. M8 最重要的边界

M8 做：

- Build
- Test
- Lint
- Formatter check
- Typecheck
- Static analysis
- Project-specific verification commands
- CI configuration inspection
- Release readiness checks
- Verify pipeline
- CheckRun
- CheckResult
- CommandResult normalization
- exit code handling
- stdout/stderr capture
- timeout
- cancellation
- environment requirements
- prerequisite handling
- check dependencies
- verification completeness
- failure evidence
- structured reports
- GUI verification UI
- CLI verification commands

M8 不做：

- AI / LLM
- 自动修复
- 自动修改代码
- 自己实现编译器
- 自己实现测试框架
- 自己实现 lint engine
- 自己实现 type checker
- 自己实现 CI runner
- Git push
- Git commit
- Git mutation
- 远程 CI 执行
- 云端构建
- Docker orchestration platform
- deployment platform
- runtime profiler
- full debugger
- project management
- issue tracker
- Snapshot persistence
- RAG / Embeddings / MCP

最重要：

> **Prism 是 Verify Orchestrator，不是 Build/Lint/Test 工具作者。**

---

# 1. Baseline Audit

修改前先审计真实代码：

```text
M0 Foundation
M1 Project Discovery
M2 Code Understanding
M3 Dependency & Architecture
M4 Git Basics
M5 ChangeSet
M6 Change Impact
M7 Health Engine
```

仍然使用：

```text
FACT
HYPOTHESIS
INFERENCE
```

以及：

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

---

# 2. 首先确认已有能力

重点检查：

```text
CommandRunner
Task
Cancellation
Error model
Project config
CLI
GUI IPC
Report system
M1 project detection
M4 Git facts
M5 ChangeSet
M7 Health findings
```

尤其是 M0 的：

```rust
CommandRunner
```

M8 必须优先复用。

不要另造：

```text
ProcessRunner
ShellRunner
VerifyCommandRunner
```

三套相似系统。

---

# 3. Verify Engine 的核心定义

M8 不是：

```text
“判断代码看起来能不能工作”
```

而是：

```text
“实际执行验证步骤，并记录实际结果”
```

例如：

```bash
npm test
```

返回：

```text
exit_code = 1
```

那么 M8 必须说：

```text
Test failed
```

而不是：

```text
Project may have testing issues
```

---

# 4. Verify 的事实优先

核心原则：

```text
Real command result
>
Project config guess
>
Static inference
```

例如：

```text
package.json:
"test": "vitest"
```

只能说明：

```text
test command candidate = npm test
```

真正执行：

```text
npm test
```

之后才能得到：

```text
Test passed
```

---

# 5. Verification ≠ Health

严格分离：

```text
M7:
Static finding

M8:
Execution result
```

例如：

```text
M7:
CODE001 High Complexity
```

M8：

```text
npm test
PASS
```

不能推导：

```text
Complexity problem is harmless
```

反过来也一样：

```text
M8:
npm test failed
```

不能让 M7 自动生成：

```text
CODE999 Test failure means bad code
```

---

# 6. Verification ≠ Prediction

禁止：

```text
“Build should probably pass.”
```

必须：

```text
“Build command completed with exit code 0.”
```

或者：

```text
“Build was not executed because required tool was unavailable.”
```

---

# 7. CheckRun

核心模型：

```rust
CheckRun
```

建议至少：

```rust
CheckRun {
    id,
    project_id,
    kind,
    name,
    command,
    working_directory,
    status,
    started_at,
    finished_at,
    duration,
    exit_code,
    result,
    stdout,
    stderr,
    diagnostics,
    environment_requirements,
}
```

根据已有模型调整。

---

# 8. CheckKind

至少支持：

```text
Build
Test
Lint
Format
Typecheck
StaticAnalysis
Custom
CI
Release
```

注意：

> “CI” 和 “Release” 第一版可能更多是 configuration/check orchestration，而不是直接执行远程 CI/release。

---

# 9. CheckState

区分：

```text
Queued
Running
Passed
Failed
Skipped
Cancelled
TimedOut
Blocked
Unavailable
Error
```

不要只使用：

```text
Success
Failure
```

因为现实环境远不止两个状态。

---

# 10. CheckResult

建议：

```rust
CheckResult {
    outcome,
    exit_code,
    duration,
    diagnostics,
    evidence,
    artifacts,
}
```

例如：

```text
Passed
Failed
Unavailable
Skipped
Blocked
Cancelled
TimedOut
```

---

# 11. Outcome semantics

必须文档化：

```text
Passed:
command actually ran and returned success.

Failed:
command actually ran and returned failure.

Unavailable:
could not run because required executable/tool/runtime unavailable.

Blocked:
prerequisite failed, so this check was not attempted.

Skipped:
explicitly disabled or outside requested scope.

Cancelled:
user/system cancellation.

TimedOut:
execution exceeded configured limit.

Error:
Prism itself failed to execute the check correctly.
```

尤其：

> `Unavailable` ≠ `Failed`

---

# 12. Command specification

建立统一：

```rust
CheckSpec
```

类似：

```rust
CheckSpec {
    id,
    kind,
    name,
    command,
    args,
    working_directory,
    environment,
    timeout,
    prerequisites,
    enabled,
}
```

优先复用 M0 `CommandRunner`。

---

# 13. No shell string execution by default

不要：

```text
"npm test && npm run lint"
```

作为底层结构。

优先：

```text
program = npm
args = ["test"]
```

这样可以：

- Windows
- macOS
- Linux
- 参数转义
- 更明确的日志
- 更低的注入风险

---

# 14. Shell usage

如果项目真实要求 shell：

```text
bash -c
pwsh -Command
cmd /C
```

必须：

> 显式建模。

不要所有 command 默认：

```text
shell = true
```

---

# 15. Command execution security

禁止把未经处理的用户输入拼接到：

```text
shell command string
```

例如不要：

```rust
format!("npm test -- {}", user_input)
```

然后 shell execute。

---

# 16. Working directory

必须明确：

```text
project root
workspace root
package root
subproject root
```

每个 CheckRun 保存：

```text
working_directory
```

使用：

> project-relative representation 优先。

---

# 17. Environment inheritance

默认可以继承必要运行环境，但必须明确：

```text
inherit parent environment
```

同时：

> 不要把完整环境变量内容记录到报告。

特别是：

```text
TOKEN
PASSWORD
SECRET
KEY
API_KEY
```

---

# 18. Environment variables

M8 只负责：

```text
required env vars
available/unavailable
```

不要把 M9 的完整 environment discovery 提前搬进来。

例如：

```text
Build requires NODE_ENV
```

可以记录：

```text
present
```

但不要输出：

```text
NODE_AUTH_TOKEN=xxxxxxxx
```

---

# 19. Tool discovery

M8 经常需要：

```text
node
npm
pnpm
yarn
python
pytest
dotnet
cargo
rustc
git
```

但：

> 不要建立完整 Environment Engine。

只实现：

```text
resolve executable
check availability
version when needed
```

更完整的 toolchain model 留给 M9。

---

# 20. Tool availability

如果：

```text
npm
```

不存在：

不是：

```text
TEST FAILED
```

而是：

```text
TEST UNAVAILABLE
Required executable 'npm' was not found.
```

---

# 21. Version check

某些项目要求：

```text
Node >= 22
Python >= 3.12
.NET >= 8
```

M8 可以在执行前发现：

```text
required version
actual version
```

如果无法确认：

```text
Version requirement could not be verified.
```

完整环境矩阵留 M9。

---

# 22. Project type driven verification

M1 已识别：

```text
Node
Python
.NET
Rust
...
```

M8 可以基于这些事实发现 candidate checks。

但：

> **candidate ≠ enabled execution**

例如 Node 项目：

```text
npm test
```

只有在真实存在：

```text
package.json scripts.test
```

时才自动提出。

---

# 23. Never invent commands

不能因为检测到：

```text
Python
```

就永远假设：

```bash
python test.py
```

也不能因为：

```text
C#
```

就永远：

```bash
dotnet test
```

必须依据：

- project file
- workspace
- known conventions
- explicit config

---

# 24. Check discovery

建立：

```text
CheckDiscovery
```

回答：

```text
这个项目可能有哪些验证入口？
```

例如：

```text
Build
  npm run build

Test
  npm test

Lint
  npm run lint

Typecheck
  npm run typecheck
```

这与真正执行分开。

---

# 25. Check plan

建立：

```rust
VerifyPlan
```

包含：

```text
checks
order
dependencies
selected_scope
```

例如：

```text
Install prerequisites
      ↓
Build
      ↓
Typecheck
      ↓
Test
      ↓
Lint
```

---

# 26. Why order matters

如果：

```text
build
```

失败：

是否仍然：

```text
test
```

必须由：

```text
Check dependency
```

决定。

不要硬编码：

```text
if build fail:
everything stop
```

---

# 27. Prerequisite graph

例如：

```text
Build
  ↓
Test
```

如果：

```text
Build = Failed
```

那么：

```text
Test = Blocked
```

如果两个完全独立：

```text
Lint
Test
```

则可以并行。

---

# 28. Check dependency model

建议：

```rust
CheckDependency {
    from_check,
    to_check,
    condition,
}
```

但第一版保持简单。

---

# 29. Sequential vs parallel

默认：

> 安全优先。

如果：

```text
test
lint
```

互不依赖，可以并发。

但必须考虑：

- 共享 build artifacts
- CPU
- disk contention
- process count
- ports
- mutation
- temporary files

---

# 30. Do not over-parallelize

不要：

```text
8 checks
→ 8 process simultaneously
```

尤其：

```text
dotnet build
npm test
cargo test
```

可能：

- 高 CPU
- 高 RAM
- 冲突工作目录
- 竞争生成目录

使用：

```text
bounded concurrency
```

---

# 31. Check scope

支持：

```text
All
Build
Tests
Lint
Typecheck
StaticAnalysis
```

例如：

```bash
prism verify .
```

全部可执行。

以及：

```bash
prism verify . --check test
```

只执行 test。

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

# 52. Generic custom check

用户可以配置：

```yaml
verify:
  checks:
    - id: api-tests
      kind: test
      command: ["python", "-m", "pytest", "tests/api"]
```

这样可以覆盖：

> Prism 不认识的项目。

---

# 53. Custom command safety

配置应使用：

```yaml
command:
  program: python
  args:
    - -m
    - pytest
```

不要优先：

```yaml
command: "python -m pytest && ..."
```

---

# 54. Config validation

必须检查：

- program 非空
- args 类型正确
- working directory 合法
- timeout 合法
- duplicate check ID
- unknown kind
- dependency target exists

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

# 56. Check source

每个 CheckSpec 应知道来源：

```text
Detected
Configured
BuiltIn
Imported
```

这样用户知道：

> 为什么 Prism 选择执行这个命令？

---

# 57. Check confidence

Check discovery 也可以有：

```text
Exact
HighConfidence
Probable
Ambiguous
```

尤其 candidate discovery。

不要把：

```text
“可能是 npm test”
```

当成确定命令。

---

# 58. Explicit configuration precedence

建议：

```text
built-in detection
→ project config
→ explicit CLI selection
```

如果冲突：

> 必须明确规则。

---

# 59. No silent command override

用户配置：

```yaml
test:
  command: [...]
```

而自动 detector 又找到：

```text
npm test
```

不要偷偷执行两个。

应该：

```text
configured command wins
```

并显示：

```text
source = configured
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

# 70. Verification completeness

统一：

```text
Complete
Partial
Truncated
Failed
```

例如：

```text
Build Passed
Test Passed
Lint Unavailable
Typecheck Blocked
```

整体：

```text
Partial
```

---

# 71. Overall verification conclusion

不要：

```text
Project = Verified
```

除非：

> 明确的 required check set 全部成功。

否则：

```text
Pass
Fail
Partial
Not Run
```

---

# 72. Required checks

配置：

```yaml
verify:
  required:
    - build
    - test
    - lint
```

如果：

```text
build = Passed
test = Passed
lint = Failed
```

整体：

```text
Failed
```

因为 required check failed。

---

# 73. Optional checks

如果：

```text
typecheck
```

只是 optional：

```text
Unavailable
```

不应把整个 Verify 结果变成：

```text
Failed
```

---

# 74. Check policy

建议：

```rust
CheckPolicy {
    required,
    optional,
    fail_on_unavailable,
    fail_on_failure,
}
```

第一版保持简单。

---

# 75. Exit code for verify

默认：

```text
all required checks passed
→ exit 0
```

如果：

```text
required check failed
→ exit non-zero
```

如果：

```text
tool unavailable
```

默认视为：

```text
execution/infrastructure issue
```

具体退出码要与 CLI 全局 convention 统一。

---

# 76. Failure reason

CLI 必须明确：

```text
FAIL

Test
Command: npm test
Exit code: 1

Reason:
test command returned non-zero exit code.

Evidence:
42 tests passed
2 tests failed
```

不要：

```text
Verification failed.
```

然后什么都没有。

---

# 77. Failure output redaction

如果 stdout/stderr 中出现：

```text
TOKEN=...
Authorization: Bearer ...
```

报告中需要：

```text
redaction pipeline
```

至少对明显 secret patterns 脱敏。

---

# 78. Output size

测试输出可能巨大。

必须支持：

```text
stdout
stderr
```

最大字节限制。

如果截断：

```text
output_truncated = true
```

并保留：

```text
first N
last N
```

或明确策略。

---

# 79. Never silently truncate

不要：

```text
stdout = first 10000 chars
```

然后假装完整。

必须标识：

```text
truncated
```

---

# 80. Artifact handling

Build/Test 可能生成：

```text
coverage
logs
reports
binaries
test-results.xml
```

M8 第一版只记录：

```text
known output artifacts
```

不要立即建立：

> 完整 artifact management platform。

---

# 81. Temporary directories

需要隔离临时文件时：

- 创建唯一 temp dir
- lifecycle 明确
- cancellation cleanup
- failure cleanup
- no accidental project pollution

---

# 82. Project mutation

M8 要意识到：

很多命令会修改：

```text
build/
dist/
coverage/
.cache/
```

因此：

> Verify 是允许执行项目命令的。

但它不应该修改：

```text
source code
Git history
Git refs
```

除非工具自身正常产生 build artifacts。

---

# 83. Git safety

禁止 Prism 执行：

```bash
git commit
git reset
git checkout
git clean
git push
git pull
git merge
```

作为 Verify 行为。

---

# 84. Git cleanliness check

可以读取：

```text
M4 GitStatus
```

例如 Release：

```text
working tree clean?
```

但不要：

```text
git clean -fd
```

自动清理。

---

# 85. Command mutation policy

需要在 CheckSpec 中标记：

```text
mutating_project
```

例如：

```text
npm test
```

可能产生 test artifacts。

但：

```text
npm run format
```

可能修改源代码。

第一版：

> 不自动执行明确会修改源代码的 command。

---

# 86. Write policy

建议：

```rust
WritePolicy {
    ReadOnly,
    BuildArtifacts,
    ArbitraryWrite,
}
```

默认只允许：

```text
ReadOnly
BuildArtifacts
```

对于：

```text
ArbitraryWrite
```

要求：

> 用户显式配置。

---

# 87. Formatter rule

例如：

```bash
prettier --check .
```

允许。

而：

```bash
prettier --write .
```

默认拒绝执行。

---

# 88. Code generation commands

例如：

```bash
npm run codegen
```

可能修改：

```text
src/generated
```

第一版应：

```text
skip unless explicitly configured
```

或：

```text
mark as project-mutating
```

---

# 89. Build directory isolation

不能假设：

```text
build/
dist/
target/
```

一定安全。

某些 build 会写：

```text
src/generated.ts
```

所以 command policy 应以：

> 实际项目配置 + explicit command metadata

为依据。

---

# 90. Timeout

每个 CheckRun：

```text
timeout
```

默认合理。

不同类型可以：

```text
Build = longer
Test = longer
Lint = shorter
```

但具体数值必须：

> configurable + documented。

---

# 91. Timeout semantics

超时：

```text
status = TimedOut
```

必须：

- 尝试终止 process tree
- Windows 正确处理
- Linux/macOS 正确处理
- cleanup children
- 返回明确 diagnostic

---

# 92. Process tree

特别注意：

```text
npm
  ↓
node
  ↓
vitest
  ↓
worker
```

只终止 parent：

> 可能留下子进程。

需要尽可能：

```text
process group / job object
```

安全清理。

---

# 93. Cancellation

GUI：

```text
Cancel
```

CLI：

```text
Ctrl+C
```

必须：

```text
propagate
terminate process tree
cleanup
mark Cancelled
```

---

# 94. Cancellation ≠ failure

不要：

```text
cancel
→ exit 1
→ Test failed
```

结果应该：

```text
Cancelled
```

退出码由 CLI convention 决定。

---

# 95. Command errors

区分：

```text
program not found
permission denied
working directory missing
spawn failed
timeout
exit non-zero
```

不要都变成：

```text
Command failed
```

---

# 96. Tool version capture

如果执行：

```bash
npm test
```

可以在 CheckRun metadata 中保存：

```text
node version
npm version
```

但是：

> 不要把 M9 完整环境 snapshot 提前实现。

---

# 97. Tool adapter architecture

建议：

```text
VerifyEngine
  │
  ├── CheckDiscovery
  ├── VerifyPlanner
  ├── CheckScheduler
  ├── CheckExecutor
  ├── OutputParser
  ├── ResultNormalizer
  └── ReportBuilder
```

Tool-specific：

```text
NodeAdapter
PythonAdapter
DotnetAdapter
RustAdapter
GenericAdapter
```

---

# 98. CheckExecutor

负责：

```text
CheckSpec
→ CommandRunner
→ CheckRun
```

它不应该判断：

```text
“这个项目质量不好”
```

只记录：

> execution facts。

---

# 99. ResultNormalizer

把：

```text
npm
pytest
dotnet
cargo
eslint
mypy
```

转成统一：

```text
CheckResult
```

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

# 126. CheckRun IDs

必须稳定表示：

> 一次实际运行。

这里与 Finding 不一样：

```text
FindingId
→ stable issue identity

CheckRunId
→ unique execution instance
```

所以 CheckRun 可以使用：

```text
UUID
```

---

# 127. Do not confuse CheckRun and Check

例如：

```text
CheckSpec:
npm test
```

是定义。

每执行一次：

```text
CheckRun #1
CheckRun #2
CheckRun #3
```

是实际运行。

---

# 128. Check ID

CheckSpec：

```text
check_id = "test"
```

稳定。

CheckRun：

```text
run_id = unique execution id
```

稳定性语义不同。

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

# 141. CLI structure

建议：

```bash
prism verify <path>
prism verify <path> --json
prism verify <path> --check build
prism verify <path> --check test
prism verify <path> --changed
prism verify checks <path>
prism verify ci <path>
prism verify release <path>
```

只实现真实有价值的命令。

---

# 142. Verify help

必须真实：

```bash
prism verify --help
prism verify checks --help
```

并说明：

- commands
- status semantics
- exit codes
- scope
- safety
- limitations

---

# 143. CLI default behavior

```bash
prism verify .
```

应该：

1. Discover checks
2. Build VerifyPlan
3. Show or internally record plan
4. Execute selected checks
5. Aggregate
6. Print results
7. Return meaningful exit code

---

# 144. Dry run

非常建议支持：

```bash
prism verify . --dry-run
```

只：

> 发现 + 展示 VerifyPlan。

不得执行命令。

---

# 145. Dry run example

```text
Verify Plan

[1] Build
    npm run build
    source: package.json

[2] Test
    npm test
    source: package.json

[3] Lint
    npm run lint
    source: package.json

[4] Typecheck
    npm run typecheck
    source: package.json

No commands executed.
```

---

# 146. Dry-run safety test

必须验证：

```text
prism verify . --dry-run
```

不会：

- 写 build
- spawn npm
- spawn python
- alter files

---

# 147. Check selection

允许：

```bash
--check build
--check test
--check lint
```

多个：

```bash
--check build --check test
```

或者统一：

```text
--checks build,test
```

选择最符合现有 CLI convention 的方式。

---

# 148. Unknown check

例如：

```bash
prism verify . --check foobar
```

必须：

```text
clear validation error
```

不要：

```text
silently ignore
```

---

# 149. Tool unavailable UX

例如：

```text
Typecheck

UNAVAILABLE

Required tool:
pyright

Reason:
Executable not found
```

建议：

> Next step: install/configure pyright.

不要：

> Install pyright automatically.

---

# 150. Failed check UX

例如：

```text
TEST

FAILED
exit code 1

2 failed
42 passed
```

---

# 151. Blocked check UX

例如：

```text
TEST

BLOCKED

Prerequisite failed:
BUILD
```

---

# 152. Timeout UX

例如：

```text
TEST

TIMED OUT

Timeout:
20m

Process tree termination attempted.
```

---

# 153. Cancellation UX

例如：

```text
Verification cancelled.

Completed:
Build
Test

Cancelled:
Lint
Typecheck
```

---

# 154. Verify engine error

如果 Prism 自己异常：

```text
ERROR
Prism failed to execute check.
```

这与：

```text
tool returned exit code 1
```

必须不同。

---

# 155. Retry policy

第一版：

> 默认不自动 retry。

因为 test/build 并不是所有失败都 transient。

可以支持：

```text
--retry 1
```

以后再考虑。

不是 M8 核心。

---

# 156. Network-aware checks

如果某个 test：

```text
requires internet
```

M8 不应该为了它自动关闭网络限制或上传数据。

只执行项目声明的正常 command。

---

# 157. External service caution

如果测试依赖：

```text
database
Redis
API
Docker
cloud credentials
```

而不存在：

```text
environment prerequisite
```

应该是：

```text
Failed
Unavailable
or Blocked
```

具体取决于真实执行结果。

---

# 158. M9 preparation

M9 会提供：

```text
EnvironmentSnapshot
```

M8 可以预留接口：

```rust
EnvironmentResolver
```

但不要现在把：

```text
ports
processes
all installed software
all env vars
```

全部实现。

---

# 159. Check prerequisites

可以：

```rust
Prerequisite {
    kind,
    name,
    requirement,
}
```

例如：

```text
ExecutableExists("npm")
VersionAtLeast("node", "22")
FileExists("package.json")
```

---

# 160. Prerequisite evaluation

返回：

```text
Satisfied
Unsatisfied
Unknown
```

例如：

```text
node >= 22
actual = 20
→ Unsatisfied
```

---

# 161. Prerequisite result

如果 check 没有执行：

```text
status = Blocked
reason = prerequisite unsatisfied
```

而不是：

```text
Failed
```

---

# 162. Check graph

例如：

```text
InstallDependencies
       ↓
Build
    ↙    ↘
 Test    Typecheck
```

第一版不一定执行 install。

尤其：

> 不要自动 `npm install` / `pip install` / `cargo fetch`。

除非用户显式配置。

---

# 163. Installation policy

默认：

```text
No dependency installation
No package update
No network setup
```

Prism 验证已有环境。

---

# 164. Dependency cache

不要：

```text
prism verify
→ create separate package cache
```

使用项目已有工具缓存。

---

# 165. Test artifact policy

可以保留：

```text
coverage
test-results
```

但要明确：

```text
artifact paths
```

避免污染 Prism 自己的数据目录。

---

# 166. Generated artifact handling

M8 需要在完成后重新读取：

```text
M4 Git status
```

如果用户执行 build 后：

```text
working tree
```

变化：

应该：

> 显示 command 造成了新的 file changes。

但不能自动 revert。

---

# 167. Verify side-effect report

非常建议 CheckRun 记录：

```text
Git status before
Git status after
```

只在已有 M4 能力可复用时。

例如：

```text
Build:
Files changed during verification:
dist/
generated/
```

这对开发工作流非常有价值。

---

# 168. Do not classify every artifact as problem

例如：

```text
dist/
```

build 后新增：

不是：

```text
GIT001 problem
```

只记录：

```text
verification side effect
```

---

# 169. Source modification detection

如果：

```text
format --write
```

被显式允许执行后：

```text
src/foo.ts changed
```

M8 应明确：

```text
warning:
verification command modified tracked source.
```

但默认：

> 禁止这类 command。

---

# 170. Verification transaction illusion

不要声称：

> Prism 会自动恢复项目状态。

除非真实实现 snapshot/worktree isolation。

M8 第一版：

> **观察副作用，不负责神奇回滚。**

---

# 171. Git mutation prohibition

即使 release check：

```text
working tree dirty
```

也只能：

```text
report
```

不能：

```text
git clean
```

---

# 172. Command audit trail

每次真实执行至少：

```text
command
args
cwd
start
end
exit
status
```

这样用户知道：

> Prism 到底执行了什么。

---

# 173. Command display

为了安全：

如果 args 中存在敏感内容：

```text
--token xxx
```

报告：

```text
--token [REDACTED]
```

---

# 174. Secret-aware command rendering

不要只在 stdout 中做脱敏。

还要考虑：

```text
command args
environment metadata
diagnostics
```

---

# 175. Environment secret filtering

如果环境变量用于 execution：

> 可以传递。

但 report：

> 默认只展示 key 名和 availability。

例如：

```text
NPM_TOKEN = present
```

不要：

```text
NPM_TOKEN = eyJ...
```

---

# 176. Process environment safety

不要把：

```text
std::env::vars()
```

完整 dump 到日志。

---

# 177. No network telemetry

M8 运行外部命令本身可能发生网络，例如：

```text
npm test
```

但这是：

> 项目命令自己的行为。

Prism 自己：

```text
No telemetry
No source upload
No cloud reporting
```

---

# 178. No remote CI trigger

再次强调：

```text
No GitHub workflow dispatch
No GitLab pipeline trigger
No cloud build
```

---

# 179. Testing architecture

测试：

```text
VerifyEngine
CheckPlanner
CheckExecutor
ToolAdapter
OutputParser
ResultNormalizer
CLI
GUI
```

---

# 180. Fake command runner

必须建立：

```text
FakeCommandRunner
```

用于：

```text
unit tests
```

不要每条 unit test 都真的启动：

```text
npm
python
dotnet
```

---

# 181. Fake execution scenarios

至少：

```text
success
non-zero
not found
permission denied
timeout
cancelled
large output
stderr
stdout
child process
```

---

# 182. Real command integration tests

另外建立少量：

```text
real_command
```

测试。

例如：

```bash
prism --version
```

确保 M0 CLI integration。

---

# 183. Fixture commands

测试项目可以提供：

```text
scripts/pass
scripts/fail
scripts/sleep
scripts/large-output
```

用于跨平台需要时，优先用：

> 已安装、稳定、跨平台 runtime。

例如现有 Rust test helper。

不要依赖：

```text
bash only
```

而测试 Windows。

---

# 184. Cross-platform command tests

必须至少考虑：

```text
Windows
Linux
macOS
```

特别是：

```text
process termination
path quoting
working directory
executable lookup
```

---

# 185. Shell quoting tests

测试：

```text
project path with spaces
Unicode path
non-ASCII path
parentheses
quotes
```

---

# 186. Output tests

测试：

```text
UTF-8
CRLF
large output
non-UTF8 fallback
stdout/stderr separation
```

---

# 187. Exit code tests

测试：

```text
0
1
2
non-zero
signal-like termination
```

---

# 188. Timeout tests

测试：

```text
command finishes before timeout
command exceeds timeout
command spawns child
child cleanup
```

---

# 189. Cancellation tests

测试：

```text
cancel before execution
cancel while running
cancel between checks
cancel after check completion
```

---

# 190. Plan tests

验证：

```text
dependency ordering
blocked checks
parallel-safe checks
explicit selection
unknown check
dry-run
```

---

# 191. Prerequisite tests

测试：

```text
satisfied
unsatisfied
unknown
```

并确认：

```text
unsatisfied
→ Blocked/Unavailable
```

语义一致。

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

# 198. Side-effect fixture

建立：

```text
build script
→ writes generated file
```

确认 M8：

```text
before status
after status
changed artifacts
```

能正确观察。

---

# 199. Dry-run fixture

确认：

```text
--dry-run
```

不会：

```text
spawn process
modify files
```

---

# 200. Security fixture

包含：

```text
fake env token
fake command argument
fake secret in stderr
```

确认报告：

```text
REDACTED
```

---

# 201. Large output fixture

输出：

```text
10MB+
```

确认：

```text
bounded memory
truncation marker
process completes
```

---

# 202. Large output policy

不要为了保存全部日志：

```text
unbounded Vec/String
```

可能造成：

> 内存爆炸。

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

# 225. Parallel result ordering

执行顺序可能：

```text
lint finishes first
test second
build third
```

UI / JSON 仍按：

```text
VerifyPlan order
```

展示。

---

# 226. Run timing

所有结果：

```text
started
finished
duration
```

使用单调时钟计算 duration，时间戳仅作为 metadata。

---

# 227. Clock changes

不要依赖 system clock difference：

```text
end_time - start_time
```

计算 duration。

应该：

> monotonic timer。

---

# 228. Process output encoding

优先：

```text
UTF-8
```

无法 decode：

```text
lossy decode
```

但标记：

```text
encoding_recovery = true
```

如已有基础设施支持。

---

# 229. Process tree cleanup audit

必须实际测试：

```text
parent
→ child
→ grandchild
```

确认 timeout/cancel 后：

> 不残留后台进程。

---

# 230. Port conflicts

某些 tests/build：

```text
localhost:3000
```

可能冲突。

M8 不负责：

> 自动寻找新端口。

真实失败时报告：

```text
command failed
diagnostic
```

M9 才会更深入环境/process/port introspection。

---

# 231. Docker

如果项目 test 要：

```bash
docker compose up
```

第一版：

可以作为：

```text
Custom Check
```

但：

> Prism 不建立 Docker 管理器。

---

# 232. Database prerequisites

同样：

```text
postgres
redis
```

可以：

```text
Custom prerequisite/check
```

但不构建：

> service orchestration platform。

---

# 233. Verify config example

推荐：

```yaml
verify:
  checks:
    build:
      kind: build
      command:
        program: npm
        args: ["run", "build"]
      required: true

    test:
      kind: test
      command:
        program: npm
        args: ["test"]
      required: true

    lint:
      kind: lint
      command:
        program: npm
        args: ["run", "lint"]
      required: false
```

具体 schema 根据现有 config system 调整。

---

# 234. Do not duplicate project config

如果：

```text
package.json
```

已经定义：

```json
"scripts": {}
```

Prism 不应该要求用户再写：

```yaml
build: npm run build
test: npm test
```

除非：

> 项目特殊需求。

---

# 235. Auto-detection strategy

第一优先：

```text
explicit Prism config
```

第二：

```text
project native config
```

第三：

```text
known ecosystem conventions
```

第四：

```text
none
```

---

# 236. Auto-detection explanation

每个 discovered check 都应显示：

```text
Detected from:
package.json scripts.test
```

或者：

```text
Configured in:
.prism.yml
```

而不是：

```text
Detected automatically
```

这种无信息描述。

---

# 237. Check confidence

例如：

```text
package.json:
scripts.test exists
```

可以：

```text
Exact
```

而：

```text
pytest installed but no project test configuration
```

可能：

```text
Probable
```

第一版可以不自动执行 Probable check。

---

# 238. Default execution policy

非常重要：

> 第一版自动执行只允许 `Exact` / `HighConfidence` checks。

`Probable` / `Ambiguous`：

```text
discovered
but not auto-run
```

这样可以明显降低误执行风险。

---

# 239. Ambiguous check

例如：

```text
two possible test commands
```

UI：

```text
Ambiguous verification entry

A: npm test
B: vitest

Select one.
```

---

# 240. No arbitrary execution from README

README 里：

```bash
rm -rf /
```

之类字符串：

> 绝对不能因为文档中出现就执行。

Only trusted sources:

```text
explicit Prism config
project-native config
safe detector rules
```

---

# 241. Command provenance

CheckSpec 必须有：

```text
provenance
```

例如：

```text
Configured
PackageScript
Cargo
Dotnet
Pyproject
CI
BuiltIn
```

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

# 254. Architecture ownership

严格保持：

```text
M0:
CommandRunner / Task / cancellation

M1:
project discovery

M2:
code facts

M3:
relationship facts

M4:
Git facts

M5:
ChangeSet

M6:
ImpactAnalysis

M7:
Health findings

M8:
Verification orchestration
```

不要反过来。

---

# 255. Suggested crate

如果当前 architecture 允许：

```text
prism-verify/
├── model
├── discovery
├── planner
├── scheduler
├── executor
├── adapters
├── parsers
├── diagnostics
├── release
├── ci
└── tests
```

不要机械照抄。

---

# 256. Module responsibility

推荐：

```text
discovery
→ 找到 candidate checks

planner
→ 决定执行计划

scheduler
→ 决定顺序/并行

executor
→ 真正执行

adapter
→ 理解具体工具

parser
→ 解析 output

normalizer
→ 转为统一模型

report
→ 输出
```

每层只承担自己的责任。

---

# 257. No giant VerifyEngine

禁止：

```text
verify.rs
2000+ lines
```

把：

- detection
- execution
- parsing
- reporting
- CLI

全塞一起。

---

# 258. No framework overengineering

第一版不要：

```text
plugin marketplace
dynamic loading
WASM rules
remote workers
distributed execution
workflow DSL
```

Rust trait + static registry + adapters 足够。

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

# 268. Discovery vs execution lifecycle

生命周期：

```text
discover
→ plan
→ validate prerequisites
→ schedule
→ execute
→ parse
→ normalize
→ summarize
→ report
```

不要：

```text
discover
→ immediately execute
```

因为 GUI 需要先展示 plan。

---

# 269. Verify plan immutability

Plan 创建后：

> execution 期间不要偷偷修改。

如果用户改变 selection：

> 新建 plan。

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

做到这里，Prism 就从一个**项目分析器**开始真正成为一个**工程验证工作台**。