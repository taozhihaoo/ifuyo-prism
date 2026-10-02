# Prism M8a — Verify Engine Core 完整开发提示词

> 本文件由《Prism M8 — Verify Engine 完整开发提示词》拆分而来，是 **Part 1 / 2（上篇：Verify Engine Core）**。
> 拆分为纯物理切分：原文正文逐字保留、未删减、未改写规范语义；原 §章节编号与标题保持原文，因此正文中的 "§xx" 交叉引用不失效。

## 【系列说明】

- 系列：Prism M8 — Verify Engine 完整开发提示词（拆分为上下两篇，两个独立可执行的开发提示词）
- 本篇：**Part 1 / 2 — M8a：Verify Engine Core**
  覆盖：定位与红线（Verify Orchestrator 不是工具作者）、核心模型（CheckSpec / CheckRun / CheckResult / CheckState / CheckKind / VerifyPlan / Prerequisite / CheckPolicy）、引擎内部分层（VerifyEngine / CheckDiscovery / VerifyPlanner / CheckScheduler / CheckExecutor / OutputParser / ResultNormalizer）、验证入口发现、计划与依赖图（Blocked / 有界并发）、执行（超时 / 取消 / 进程终止 / WritePolicy / secret 脱敏 / stdout 截断）、`--dry-run`、FakeCommandRunner 测试基座、`prism verify` 基础执行面。
- 姊妹篇：**Part 2 / 2 — M8b：Verify Adapters CI and Release**
  文件：`Spec/Prism M8b — Verify Adapters CI and Release 完整开发提示词.md`

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

## 【M8a 本篇范围与实现顺序】

本篇目标：交付 Verify Engine 的核心与基础执行面，使 `prism verify <path>` 真实可用。建议按本篇正文（原文）顺序阅读与实现：

1. 边界与原则（§0–§6）
2. 核心模型（§7–§12）
3. 命令执行基础与工具发现（§13–§21）
4. 验证入口发现与计划 / 依赖图（§22–§31）
5. 自定义 check 配置与 discovery 语义（§52–§54、§56–§59）
6. 聚合语义与 CheckPolicy（§70–§76）
7. 执行红线：脱敏 / 截断 / 副作用 / WritePolicy / Git 安全（§77–§89）
8. 执行机制：超时 / 取消 / 进程树 / 错误分类（§90–§96）
9. 引擎内部分层（§97–§99）
10. 身份语义（§126–§128）
11. CLI 基础执行面与 `--dry-run`（§141–§148）
12. 结果 UX 与边界情形（§149–§158）
13. Prerequisites 与安装策略（§159–§164）
14. 副作用观察与审计 / 遥测红线（§165–§178）
15. 测试基座（§179–§191、§198–§202）
16. 执行细节与平台边界（§225–§232）
17. 配置 / 自动检测 / 置信度 / 默认执行策略（§233–§241）
18. 架构归属与分层约束（§254–§258、§268–§269）

范围说明：原文 §141（CLI structure）同时列出 `prism verify checks / ci / release` 子命令；这些子命令的完整规格（§55、§60–§69、§129–§140、§192–§197 等）归 **M8b（Part 2/2）** 实现。本篇交付 `prism verify <path>`（含 `--json` / `--check` / `--dry-run`）基础执行面即可，但须按 §97 / §259 / §295 的方向为 adapter / parser / 子命令预留扩展点，不得把 M8b 的实现提前塞进本篇。

以下为原文标题、引言与正文（逐字保留，自 §0 起）：

---
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
---

## 【阶段性完成门槛（M8a / Part 1 of 2）】

M8a 只有全部满足以下条件，才算阶段性完成并允许 commit。
完整里程碑验收清单（§316 Completion Gate）、Final Audit（§315）与最终 commit（§317）归 **M8b（Part 2/2）**。

```text
[ ] M0–M7 regression 全部通过

[ ] CheckSpec / CheckRun / CheckResult / CheckState / CheckKind / VerifyPlan / Prerequisite / CheckPolicy 核心模型落地
[ ] 引擎内部分层清晰：CheckDiscovery / VerifyPlanner / CheckScheduler / CheckExecutor / OutputParser / ResultNormalizer
[ ] 复用 M0 CommandRunner，无 ProcessRunner / ShellRunner / VerifyCommandRunner 三套重复系统（§2）
[ ] 无 giant VerifyEngine，crate/module 分层符合 §254–§258

[ ] 验证入口发现可用，source / confidence / provenance 语义正确（§22–§24、§56–§59、§233–§241）
[ ] VerifyPlan / 依赖图正确，Blocked 语义正确（§25–§28、§159–§162）
[ ] bounded concurrency 生效，结果按 VerifyPlan 顺序呈现（§29–§30、§225）

[ ] Timeout / Cancellation / 进程树终止在 Windows / Linux / macOS 行为正确，无 orphan process（§90–§95、§229）
[ ] Command errors 分类正确，不都变成 "Command failed"（§95）
[ ] WritePolicy 生效，不自动执行会修改源代码的 command（§82–§89、§85）
[ ] secret 脱敏覆盖 stdout/stderr / command args / env metadata（§77、§173–§176）
[ ] stdout/stderr 有界采集 + 截断标记，无静默截断（§78–§79、§201–§202）
[ ] 不 dump 完整环境变量，无遥测 / 源码上传 / 远程触发（§17–§18、§176–§178）

[ ] --dry-run 只发现 + 展示 VerifyPlan，零命令执行（§144–§146、§199）
[ ] `prism verify <path>` 基础执行面可用：discover → plan → execute → aggregate → print → exit code（§141–§148）
[ ] Unavailable / Blocked / Failed / Cancelled / TimedOut / Error 语义正确且 UX 明确（§11、§20、§149–§154）

[ ] FakeCommandRunner 测试基座建立，本篇全部测试场景通过（§179–§191、§198–§202）
[ ] 少量真实命令 integration smoke 可用（§182–§183）
[ ] side-effect / dry-run / security / large-output fixture 通过（§198–§202）

[ ] 无 AI / 无遥测 / 无源码上传 / 无数据库 / 无 Git mutation / 无自动修改源代码
[ ] 未提前实现 M8b 范围（各生态 Adapter 细节、结构化输出解析、CI / Release、报告与 GUI 集成）
[ ] 无 P0，无未解决的阻塞性 P1
```

## 【M8a Commit】

全部通过后：

```bash
git add -A
git commit -m "feat: add verify engine core (M8 part 1/2)"
```

不要 push。

不要修改 remote。

完成后进入姊妹篇 **M8b（Part 2/2）**：`Spec/Prism M8b — Verify Adapters CI and Release 完整开发提示词.md`。
