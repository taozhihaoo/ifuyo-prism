# Prism M8 Final Verify Audit

日期：2026-10-03 · 范围：M8a（Verify Engine Core）+ M8b（Adapters / CI / Release）
验收基线：`Spec/Prism M8a — Verify Engine Core 完整开发提示词.md`、`Spec/Prism M8b — Verify Adapters CI and Release 完整开发提示词.md`

> Findings-first：本审计只记录事实与证据链，不打分。所有结论可由列出的命令复现。

## A. Baseline

真实 M0–M7 状态：**FACT**（全部来自已提交代码与 `cargo test --workspace`）。

- M0 CommandRunner / 配置 / 任务取消：CONFIRMED（verify 全链路复用，无第二套 runner）
- M1 Discovery（测试文件分类）：CONFIRMED（`--changed` fallback 使用）
- M2/M3 CodeModel / 依赖图：CONFIRMED（M6 impact 链消费）
- M4 Git 只读事实：CONFIRMED（副作用观察 / verified_state / release clean 检查）
- M5 ChangeSet / M6 Impact：CONFIRMED（`--changed` 候选链）
- M7 Health：CONFIRMED（与 M8 保持两个独立事实层，无相互污染）
- 先前里程碑状态：全部 RESOLVED（无 WORSENED；M8 新增 crate 不改动既有 crate 行为，
  `cargo test --workspace` 388 项 + vitest 36 项全绿）。

## B. Verify Architecture

```text
Discovery → Planning → Scheduling → Execution → Parsing → Normalization → Reporting
```

- crates/prism-verify/src/discovery.rs → planner.rs → executor.rs → parsers.rs → engine.rs → render.rs
- 计划与执行共用同一依赖解析（planner::resolved_dependencies）——计划展示的依赖
  与调度器实际执行的依赖一致（本里程碑修复：此前约定依赖只影响展示）。
- 边界正确：发现 ≠ 执行（§24/§268）；Prism 是 Orchestrator，不重造工具（§0/§6）。

## C. Command Execution

- CommandRunner reuse：唯一执行路径 prism-core StdCommandRunner（§2）
- cwd：`working_directory`（config 可配）→ CommandSpec::with_working_directory
- args：显式 argv 数组；无 shell 字符串拼接（§13）。Windows `.cmd` shim 经
  `cmd.exe /C <program> <args…>` 显式 argv 运行（std cmd 专用转义；BatBadBut 后必需）
- environment：仅透传 config 显式声明的 env；CheckRun.environment_metadata 记录元数据
- timeout：DEFAULT_TIMEOUT_MS=600s，config 可覆盖；超时 → TimedOut + 终止尝试
- cancellation：CancellationToken 传播至 M0；Cancelled 状态 + 队列清空
- process cleanup：M0 终止**直接子进程**；孙进程孤儿是文档化平台限制
  （Windows Job Object / POSIX process group → 延后，见 V）
- security：输出/命令脱敏（§77/§173），有界采集 1 MiB/流（§201/§202）

## D. Check Model

- CheckSpec：稳定 id（§128）+ kind/source/confidence/write_policy/depends_on
- CheckRun：run_id（UUID）+ 执行事实（exit_code/duration/stdout/stderr/diagnostics/
  side_effects）；CheckRun ≠ Check（§127）
- CheckResult 语义由 CheckState 10 态 + OverallConclusion 表达（§71）
- CheckDependency：显式 depends_on + 同 toolchain 约定依赖（Build → 后续），二取并集

## E. Check Discovery

| 生态 | 触发 | 产物 | 实现 |
| --- | --- | --- | --- |
| Node (§261) | package.json scripts | build/test/lint/typecheck/format（npm/pnpm/yarn 探测） | ✓ |
| Rust (§264) | Cargo.toml | cargo-build / cargo-test / cargo-check / cargo-clippy | ✓ |
| Python (§262) | pyproject [tool.pytest]/[tool.ruff]/[tool.mypy] | pytest / ruff / mypy | ✓ |
| C# (§263) | .sln / .csproj / global.json | dotnet-build / dotnet-test | ✓ |
| Generic | verify.checks 显式配置 | 任意 program+args（WritePolicy 默认 ReadOnly） | ✓ |
| CI | .github/workflows/*.yml | 静态抽取（job/step/run + 行号），绝不执行 | ✓ |
| custom config | prism.config.json verify.checks | configured wins（§59，不静默覆盖） | ✓ |

polyglot id 冲突修复：生态前缀稳定 id（§128）；本仓库自验证发现并修复了
“planner 静默丢弃同 id cargo check”缺陷。

## F. Prerequisites

- ExecutableExists（§20）：不存在 ⇒ Unavailable（不是 Failed）——真实复现
  （首次自验证 npm 未解析时如实 Unavailable，随后修复解析而非改语义）
- 依赖未过 ⇒ Blocked（§27，同 toolchain 传播；cargo-build 失败 ⇒ cargo-test Blocked、
  dotnet-build 失败 ⇒ dotnet-test Blocked，真实项目复现）
- Unknown 语义：Prism 不猜（§23）；无法判定时留空/None（如非 git 仓库的 verified_state）

## G. Execution States

10 态全部有真实触发路径与测试：Passed / Failed（真实 exit≠0）/ Skipped（WritePolicy
ArbitraryWrite 默认拒绝）/ Blocked（约定依赖）/ Unavailable（程序缺失）/ Cancelled
（IPC cancel + e2e）/ TimedOut（ping -n 30 超时 e2e）/ Error（spawn 失败、权限、环）/
Queued / Running（中间态）。真实项目复现：fail、blocked、unavailable 三类均有录得。

## H. Output Parsing

- stdout/stderr 分流捕获、不互换（§206 抽查确认：pytest 摘要在 stdout、
  dotnet 诊断在 stdout、cargo 错误在 stderr）
- exit code 逐 run 记录并与进程一致（§206）
- 结构化解析（保守，§288-§291）：pytest 摘要行 / dotnet MSBuild 行 / SARIF JSON
  （stdout 嗅探：以 `{` 开头且含 `"runs"` 才尝试，解析失败回退逐行）/ JUnit XML
  （内容解析器已实现，供后续 config 声明 junitxml 产物路径时接入）；解析仅增补
  diagnostics，绝不改变 state/exit code
- 解析失败 ⇒ 空诊断（无 parser_error 污染执行结果）
- 截断：1 MiB/流 head 保留 + output_truncated 标记 + e2e 测试
- 真实复现：CS0103 编译错误 → `[dotnet] path:5:26: CS0103: …`；pytest 失败 →
  `[pytest] tests/test_app.py::test_add - assert 5 == 6`

## I. Security

- 无 secret 泄漏：[REDACTED] 管道覆盖 stdout/stderr/args/env 元数据（§77/§173-§176，有测试）
- 无环境变量倾倒：仅元数据
- 无命令注入：argv 数组 + cmd.exe 专用转义；无 shell 字符串执行（§13）
- 无默认 shell；无 Git 写操作（§83，status/diff 只读）；无自动源码修改
  （format 类默认 ArbitraryWrite ⇒ Skipped，§85-§87）

## J. CI

- 静态抽取 `.github/workflows/*.yml`（缩进式保守解析器，无 YAML 引擎）：
  job / step / run 命令 + 文件行号（§196 fixture 断言齐全）
- `prism verify ci .` 输出 "inspection only — never executed by Prism"（§208/§242）
- 本地 vs CI 对照：VerificationCoverage{shared, ci_only}，ci_only 明示 "not judged"
- 无远程执行：无任何网络调用路径

## K. Release

- `prism verify release .`：required files + build/test 结论（取自真实 CheckRun 结论，
  §209 不因文件存在而放行）+ require_clean_tree（M4 GitStatus）
- 真实运行：Prism 仓库 Release PASS（README 存在 + build/test 实际通过 + 树干净）
- 无部署：不存在任何 deploy/uploader 代码路径（§245）

## L. Change Integration

- `--changed`：M5 ChangeSet → M6 ImpactAnalysis → TestCandidate
  （prism-verify/changed.rs，CLI 与 GUI 共用）
- 输出/展示恒标注 **Potential**，绝不冒充 "Verified affected tests"（§33/§34/§247）
- 图不可建时退回 M1 测试文件分类（仍是候选清单，如实更粗）
- 真实运行：干净树上候选为空（不捏造）；有变更时列出受影响测试路径

## M. Health Integration

- M7 Findings 与 M8 CheckRuns 是两个独立事实层：health 不声称 “verified”，
  verify 不产出 findings；GUI 两个页面分开（§210）
- 唯一交点：M7 的 `--changed` 与 M8 的 `--changed` 各自消费 M5/M6，互不引用

## N. GUI

- Verify 页：plan 先行（§298 First-run 不自动执行）→ 显式 Run / Run (current changes
  scope) / Run selected → 轮询 → 结果汇总（counts + verified_state 脏/洁标注 §277）
  → check 卡片（状态色、时长、截断标注）→ 展开详情（cwd/exit code/stdout/stderr/
  diagnostics）→ Notes（如实声明 session 有效性）→ Run again 生成新 run 不覆盖旧结果
  （§119）
- Overview：session Verification 卡（§212）+ "No verification run in this session"（§275）
  + stale 判定（HEAD 移动 / 树变脏，§213）
- vitest 36 项覆盖上述状态机
- Deferred（如实记录）：CI inspection / Release 专属 GUI 面板（CLI 已全量覆盖，
  产品化面板属 M12 范畴）

## O. CLI

真实执行记录（2026-10-03，Windows 11 / rustc 1.99 / node 22 / python 3.13 / dotnet 10）：

| 命令 | exit | 结果 |
| --- | --- | --- |
| `prism verify .` | 0 | 7 checks 全 Passed（npm build/test/typecheck + cargo build/check/clippy/test） |
| `prism verify . --dry-run` | 0 | 完整 plan，零执行（"No commands executed."） |
| `prism verify . --json` | 0 | schema 化 JSON（含 verified_state/side_effects/candidates） |
| `prism verify . --changed` | 0/1 | 候选链 + Potential 标注；树干净时候选为空 |
| `prism verify checks .` | 0 | 7 个入口 + 来源标注 |
| `prism verify ci <fixture>` | 0 | job/step/命令/行号抽取，本地对照 |
| `prism verify release .` | 0 | 基于真实 build/test 结论 + required files |

stdout/stderr/exit code/JSON 均经人工核对（§206 抽查：命令与实际进程一致——
含 Windows 下如实显示 `cmd.exe /C npm run build`）。

## P. Reports

- JSON：完整 VerificationReport（serde，schema_version）
- Text/Markdown：render_text（计划态 + 结果态 + verified_state + 候选）
- HTML：render_html（静态表格报告）
- 每条 run 含 command provenance（command_display 与实际进程一致）/ exit code /
  duration / diagnostics / redaction / truncation 标记

## Q. Tests

真实统计（2026-10-03）：

- Rust：`cargo test --workspace` **388 passed / 0 failed**（51 个测试二进制）
- 前端：`npx vitest run` **36 passed / 0 failed**（6 个文件）
- 分类覆盖：unit（parsers/planner/model）、executor（并发/WritePolicy/cancel/timeout）、
  scheduler（拓扑/环/Blocked 传播）、adapter（polyglot 发现 + 真实项目）、parser
  （junit/dotnet/pytest + 保守失败）、CLI（verify_cli 契约 5 项）、GUI（Verify/Overview
  状态机）、跨平台路径（cfg(windows) 分支 + .cmd wrap 测试）、timeout/cancellation
  e2e、security（脱敏/截断）、fixtures（§192-§197 临时工程）、real projects（见 R）
- clippy --workspace --all-targets：0 warning

## R. Real Projects

全部真实执行（同日）：

| 项目 | 栈 | 结果 |
| --- | --- | --- |
| **Prism 自身**（§204/§205） | npm + Cargo workspace | 7/7 Passed，exit 0，clean_at_commit |
| pydemo | Python 3.13 + pytest + ruff | 通过路径：2/2 Passed；失败路径：pytest fail + 解析诊断 `[pytest] tests/test_app.py::test_add - assert 5 == 6`；ruff 真实失败（import 排序）如实 Failed |
| csdemo | C# / .NET 10 classlib | dotnet build + dotnet test 通过；注入 CS0103 编译错误后：build Failed + 结构化诊断 + dotnet-test Blocked |
| （Rust 即 Prism 自身 workspace） | cargo build/check/clippy/test | 全 Passed |

自验证回答（§205）：**Prism 能通过自己的 Verify Engine 验证**。过程中发现并修复了两个
真实缺陷（polyglot id 冲突静默丢弃、Windows `.cmd` 无法 spawn），未为“全绿”修改任何
detection semantics。已记录的平台限制：验证 Prism 自身时若从 `target/debug/prism.exe`
运行，cargo 无法替换正在执行的二进制（os error 5）⇒ 从副本运行即可；属于
Windows 文件锁语义，非引擎缺陷。

## S. Performance

真实测量（7 checks，Prism 仓库，增量构建态）：

- check 数：7（4 cargo + 3 npm）
- 并行度：2（拓扑层内并发；cargo 层与 npm 层并行）
- 总墙钟：**28.6 s**；各 check 时长和：32.7 s（并行重叠）
  - cargo test 17.9 s / npm build 5.9 s / cargo build 2.1 s / clippy 1.6 s / check 1.5 s /
    typecheck 1.4 s / npm test 2.3 s
- stdout 最大：cargo test 28,362 B（无截断，远低于 1 MiB 上限）
- 内存观察：无专门仪器；进程常驻为命令采集缓冲（≤2 MiB/run 上界），无异常

## T. Git

- `git status` 干净；target/、node_modules/、dist/ 均在 .gitignore
- 报告/fixture 均写入系统临时目录，无误提交
- 提交按子任务拆分（见 §317 前的提交历史），无 push、无 remote 修改

## U. Risks

**P0**：无。

**P1**：

1. 进程树清理仅覆盖直接子进程（M0 terminate）；npm→node→vitest 孙进程在
   timeout/cancel 时可能残留。平台修复（Job Object / process group）已文档化延后。
2. CI 抽取器为缩进式保守解析，仅覆盖 `- run:` / `run:` 常见形态；复杂 YAML
   （anchors、多行块标量）不解析——表现为少列出命令（保守方向），不误判。
3. SARIF 已实现并在执行路径上保守嗅探 stdout（`parse_sarif` + `parse_failure_output`
   集成，单元测试覆盖）；JUnit 解析器已实现但默认执行路径不读取文件产物（不擅自追加
   `--junitxml` 参数）；读取工具产物文件需 config 显式声明输出路径，留后续。

## V. Deferred

未做且不偷偷做：

- M9 Environment Engine（toolchain model 仅保留 §19 接口位）
- M10 Context Engine / M11 Snapshot（无 SQLite、无持久化数据库——M8 仅 session 内存）
- M12 UI 产品化（CI/Release GUI 面板）
- M13 Release Hardening
- 明确未引入：database / cloud CI / remote execution / AI / RAG / MCP / deployment

## §316 Completion Gate

```text
[x] M0–M7 regression passes（cargo test --workspace 388 + vitest 36 全绿）
[x] CheckSpec / CheckRun / CheckResult / VerifyPlan 存在且职责清晰
[x] Check discovery works（Node/Python/C#/Rust/Generic/CI/custom）
[x] Check provenance works（source + detected_from 逐 run 记录）
[x] Confidence semantics work（Exact/HighConfidence 才入自动执行；未知 script 不猜）
[x] Prerequisites work（ExecutableExists ⇒ Unavailable）
[x] Check dependency ordering works（显式 + 同 toolchain 约定，计划/执行一致）
[x] Bounded concurrency works（层内 take(max_concurrency)）
[x] Cancellation works（IPC/CLI/引擎三层 e2e）
[x] Timeout works（TimedOut + 终止尝试）
[x] Process tree cleanup works（直接子进程；孙进程限制已文档化）
[x] Node/Python/C#/Rust verification works where configured（真实项目验证）
[x] Generic custom check works（verify.checks + WritePolicy）
[x] Build/Test/Lint/Format/Typecheck/Static analysis works where configured
[x] Unavailable/Blocked/Failed/Cancelled/TimedOut/Error semantics work（各有测试）
[x] Structured output parsing works where implemented（pytest/dotnet 默认路径；
    SARIF 已实现并接入 stdout 嗅探；junit 内容解析器已实现，
    产物文件路径待 config 声明）
[x] Diagnostic normalization works（§278 真实 CS0103 / pytest 复现）
[x] stdout/stderr handling works（分流不互换）
[x] output truncation works（1 MiB head + 标记 + 测试）
[x] secret redaction works（[REDACTED] 管道 + 测试）
[x] CI inspection works（静态抽取 + 行号 + 本地对照）
[x] No remote CI execution / No deployment / No Git mutation / No automatic source modification
[x] --dry-run works（零执行）
[x] --changed works（M5/M6 候选链，Potential 标注）
[x] M5/M6 integration works / M7/M8 separation works
[x] CLI works / GUI works / JSON works / Markdown-Text works / HTML works
[x] Prism self-verification works（7/7）
[x] Real project validation completed（Prism/TS/Python/C#/Rust）
[x] No AI / No telemetry / No source upload / No database / No cloud CI / No deployment system
[x] No P0 / No unresolved blocking P1（P1 三项均有记录且非阻塞）
```

结论：**M8（M8a + M8b）验收通过。**
