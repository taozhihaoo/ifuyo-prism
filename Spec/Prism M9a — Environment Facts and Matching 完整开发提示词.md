# Prism M9a — Environment Facts and Matching

## M9 完整开发提示词 · 上篇（Part 1/2）

## 【系列说明】

- 本篇是 **Prism M9（Environment Engine）规格拆分的上篇（Part 1/2）**，内容域：**Environment Facts and Matching** —— M9 定位与红线（Observe/Compare/Explain，不做 Install/Repair/Mutate）、EnvironmentSnapshot 核心模型、host/OS/architecture/shell/PATH 采集与规范化、可执行文件与工具版本探测（ToolProbe allowlist、结构化版本比较）、runtime/SDK/package manager 发现、Requirement 模型基础与 RequirementMatch 匹配语义（Unknown ≠ Missing ≠ Broken）、敏感环境变量识别与 redaction、probe 超时/取消/失败隔离、EnvironmentService、`prism env` 基础输出。
- 姊妹篇（下篇 Part 2/2）：`Prism M9b — Environment Requirements Findings and Reports 完整开发提示词.md`，内容域：项目需求抽取（来源层级与 RequirementConflict）、环境变量/`.env`、ports/processes/container 动态事实（observed_at 语义）、EnvironmentFinding 及其与 M7 Finding 体系的集成、`prism env requirements/tools/ports/processes` 完整 CLI、报告与 GUI 数据流、真实项目验证、完整验收清单 + Final Audit + 最终 commit。
- 两篇均拆分自源文档 `Prism M9 — Environment Engine 完整开发提示词.md`（该文件已拆分完毕，此后不再维护、以两篇为准）。**除【系列说明】/【承接 M9a】/红线摘要/【阶段性完成门槛】为拆分新增的框架内容外，两篇正文全部为原文逐字内容：原文章节编号与标题原样保留（正文形如 "§xx" 的交叉引用跨篇仍然有效），未删减、未改写规范语义、未新增需求。**
- 原文 §区间 → 归属篇 映射表（M9a = 本篇，M9b = 姊妹篇）：

| 原文 §区间 | 内容域 | 归属篇 |
|---|---|---|
| 开篇（§0 之前的标题与背景引言） | M9 总定位、M0–M8 背景与核心问题 | M9a |
| §0–§10 | 定位、不是什么、最重要边界、核心原则、与 M8/M1/M3/M7/M8/M10/M11 的关系 | M9a |
| §11–§13 | EnvironmentSnapshot 核心模型、Snapshot 含义、Environment identity | M9a |
| §14–§21 | Host / OS / Architecture / Shell / Shell path / PATH 采集与规范化、PATH diagnostics | M9a |
| §22–§30 | Executable discovery、工具范围、发现策略、No full-disk scan、Tool source、Multiple versions、ToolVersion、Version semantics、Version manager | M9a |
| §31–§37 | Runtime / SDK / PackageManager 模型与发现、Active package manager、Provenance | M9a |
| §38–§41 | EnvironmentRequirement 模型、Requirement kind、Operators | M9a |
| §42–§45 | Requirement source、Do not trust README equally、Requirement priority、Requirement conflict | M9b |
| §46–§50 | Environment matching、Match result、Example、Unknown、NotApplicable | M9a |
| §51–§53 | Environment Finding 类型、Finding 与 M7 Finding、Environment category | M9b |
| §54–§57 | Environment variables、Sensitive variable detection、Non-sensitive variables、Variable source | M9a |
| §58–§60 | `.env`、Git + env integration、Environment file reading | M9b |
| §61–§70 | Never echo secrets、Project-local tools、Local vs global、PATH shadowing 事实、Executable resolution、Project package binaries、Virtual environments、Python interpreter precedence | M9a |
| §71–§77 | Node / Python / .NET / Rust / Java / Go / C/C++ 环境 requirement 识别 | M9b |
| §78–§80 | Container environment、Docker version、Container configuration | M9b |
| §81–§100 | Port discovery、Why ports matter、Listening ports only、PortResolver、Port ownership、Process discovery、Process privacy、Process list scope、Process classification、Dev server detection、Process-to-project association、Working directory、Process conflict、Expected ports、Port requirement、Port matching、Service requirements、Service detection confidence、No network probing、localhost checks | M9b |
| §101–§103 | Environment configuration files、Environment source hierarchy、Environment conflicts | M9b |
| §104–§107 | Lockfiles、Package manager mismatch、Package manager version requirement、Corepack | M9a |
| §108–§111 | PATH normalization、Path canonicalization、Symlink awareness、Permission checks | M9a |
| §112–§119 | Tool execution safety、No arbitrary executable execution、Tool command allowlist（ToolProbe）、Tool probe timeout、Probe cancellation、Parallel probes、Discovery stages、Progress | M9a |
| §120–§123 | Lazy discovery、Environment profile、Generic mode、Project-specific scan | M9a |
| §124–§125 | Requirement parser architecture、Requirement provider | M9b |
| §126–§127 | Environment collector、Matching engine | M9a |
| §128–§137 | Findings engine、Finding confidence、Example finding、Another finding、Path finding、Duplicate executable finding、Process finding、Port finding uncertainty、No score、No magical Ready | M9b |
| §138–§145 | Environment completeness、No GPU by default、CPU/memory/disk、Resource requirement、Resource sampling、Disk context、CPU context、Memory privacy | M9a |
| §146–§153 | OS capability、Capability model、Permission errors、Partial scan、Diagnostic model、Source evidence、No source code ingestion、Reuse project facts | M9a |
| §154–§159 | Performance、Probe batching、Tool probe failure、Exists but probe fails、Version mismatch without version、Package manager install status | M9a |
| §160–§168 | Shell consistency、Shell environment probe、Environment discrepancy、Scope、User PATH、Variable precedence、Active environment、Process environment capture | M9a |
| §169–§171 | Project env variables、Required environment variables、Required env value validation | M9b |
| §172–§175 | Network service availability、Local service availability、Database health、Docker service health | M9b |
| §176 | Probe side effects（read-only 红线） | M9a |
| §177 | Docker probe | M9b |
| §178–§185 | Git environment、Git config、Git config safety、SSH environment、Certificate environment、Java/Android、CUDA/GPU、Environment plugins | M9a |
| §186–§193 | Suggested architecture、Layering、Collector boundary、Normalization、Platform adapter、Cross-platform first、Windows path case、Unicode 路径 | M9a |
| §194–§195 | Process tests、Port tests | M9b |
| §196 | Tool tests | M9a |
| §197 | Requirement tests | M9b |
| §198–§200 | Environment matching tests、Sensitive variable tests、Redaction tests | M9a |
| §201–§202 | `.env` tests、Requirement provenance tests | M9b |
| §203–§205 | Tool provenance tests、Version comparison tests、Project fixtures | M9a |
| §206–§208 | Real project validation、Prism self environment、Self-environment correctness | M9b |
| §209 | No fabricated environment（Unknown ≠ Missing） | M9a |
| §210–§219 | Scan determinism、Dynamic facts（observed_at）、Process IDs、Executable identity、Tool identity、Diff preparation、Fingerprint、No aggressive cache、Refresh semantics、Auto-refresh | M9b |
| §220–§233 | Environment UI、Overview summary、Toolchain/Requirements/Finding/Ports/Processes/Variables/PATH/Config sections、Relevant process rules、All processes、Requirement conflict UI、Evidence drill-down | M9b |
| §234–§236 | Environment report（JSON）、Markdown report、HTML report | M9b |
| §237–§239 | CLI 完整命令集、`prism env requirements`、JSON machine use | M9b |
| §240–§242 | Exit codes、`--strict`、Default strictness（`prism env` 基础输出契约） | M9a |
| §243–§248 | Environment errors、Partial environment semantics、Scan diagnostics、No root/admin by default、Elevated mode、Security boundary | M9a |
| §249–§250 | Privacy-redacted report、Private path shortening | M9b |
| §251–§255 | Secret-safe logs、Debug mode、Telemetry、Network、No external version APIs | M9a |
| §256–§257 | Environment health conclusion、Requirement coverage | M9b |
| §258 | Match evidence | M9a |
| §259 | Environment diff preparation（EnvironmentDelta） | M9b |
| §260–§261 | No M11 implementation leakage、In-memory only | M9a |
| §262–§263 | Optional report export、Auto-save prohibited | M9b |
| §264–§270 | Verify integration、M8 adapter integration、Avoid circular dependency、Dependency direction、Environment service API、Scan options、Heavy scans | M9a |
| §271–§278 | UI tabs、Search/filter、Status filters、Environment refresh、Project config change、Current state indicator、Dynamic process warning、Refresh granularity | M9b |
| §279–§281 | Host info section、Resource facts、Resource unknown（展示层） | M9b |
| §282–§291 | File system、Filesystem capability、Path length、File encoding、Locale、Timezone、Locale mismatch、Time-sensitive environment、User-specific differences、Reproducibility | M9a |
| §292–§294 | Environment export、Schema version、Backward compatibility | M9a |
| §295–§297 | CLI machine mode、CLI text mode、CLI color（基础输出） | M9a |
| §298–§304 | GUI/CLI parity、No duplicated detection、Browser limitation、IPC、IPC progress、IPC cancellation、CLI reuse | M9b |
| §305–§315 | Error model、Permission errors、Unsupported platform feature、Probe timeout、Probe budgets、Concurrency budget、Resource budget、Process/Port/PATH/Variable caps | M9a |
| §316–§321 | No secrets scanning expansion、No dependency vulnerability scan、No internet-based package checks、No automatic repair、Suggestions、Environment action model | M9a |
| §322–§331 | Environment report distinction、Complete result 示例、Important distinction、Unknown/Conflict/Process-port 示例、Already running、Association evidence、Snapshot display | M9b |
| §332–§333 | Scan performance target、Performance testing | M9b |
| §334 | No full project rescan | M9a |
| §335–§336 | Requirement parsing cache、Environment facts cache | M9b |
| §337–§340 | Snapshot lifecycle、Snapshot status、Schema、JSON normalization | M9a |
| §341–§342 | Report redaction tests、HTML security | M9b |
| §343 | Terminal output escaping（CLI 输出安全） | M9a |
| §344–§345 | Process commandline sanitization、Path sanitization（展示层） | M9b |
| §346 | Central privacy layer | M9a |
| §347 | Environment serialization privacy | M9b |
| §348–§349 | No raw environment export by default、Debug dump | M9a |
| §350–§359 | Findings severity、Health integration、No score、Environment compliance、Environment status、Aggregation 规则 | M9b |
| §360–§381 | Golden paths（final golden path 与 #2–#20）、M8/M9 combined flow、M7/M9 combined flow | M9b |
| §382–§383 | Environment context for M10、Context-safe export | M9b |
| §384–§389 | No AI、No database、No telemetry、No automatic updates、No package install、No environment mutation | M9a |
| §390 | Architecture final check | M9b |
| §391–§395 | Code quality、Error resilience、Failure isolation、Scan cancellation、Deterministic ordering | M9a |
| §396 | Stable report ordering | M9b |
| §397–§420 | Unit/Integration/Security/Performance/Large environment tests、Real self-audit、Self correctness/privacy、Full product integration、Unified project context、Multiple projects、Shared host facts、相关性边界与 UI details（§410–§420） | M9b |
| §421–§423 | Final Audit、Completion Gate、最终 Commit | M9b |
| §424–§429 | M0–M9 完整语义链、每模块一个核心问题、核心闭环、最终设计原则、最终产品定义、M9 与整个 Prism 的最终关系 | M9b |

- **执行顺序**：先完成本篇（M9a）并通过本篇末尾【阶段性完成门槛】，本地 commit 合入后，再执行 M9b。

## 【本篇定位与执行红线】（执行摘要，以正文原文为准）

M9 第一版只做 **Observe, Compare, Explain**，不做 **Install, Repair, Mutate**（原文 §1）。本篇执行时必须严格遵守（摘要，不削弱原文）：

```text
Deterministic First / Evidence First
  环境判断全部确定性、可复现；关键事实必须带 source + evidence；无 AI / LLM / RAG / MCP / agent（§3、§151、§384）。
Read Only by Default / No automatic repair
  probe 只读、低开销、有界；禁止安装/升级/启停/修改 PATH、注册表、版本切换；只允许 suggestion，不执行（§176、§319–§321、§387–§389）。
Bounded discovery
  禁止全盘扫描；工具探测仅限 allowlist（ToolProbe），必须有 per-probe 超时、可取消、有界并发、失败隔离（§25、§112–§117、§308–§311、§392–§394）。
No network by default
  不联网探测、不查询最新版本；no telemetry / analytics / crash upload（§253–§255、§318、§386）。
Privacy / No secret exposure
  敏感变量只记录 name + present/absent + classification；日志与 `--json` 输出不得泄漏 secret；必须复用中央 privacy/redaction layer（§54–§57、§61、§248、§251–§252、§346、§348–§349）。
Explicit Uncertainty
  Unknown ≠ Missing ≠ Broken；探测不到就输出 Unknown，不虚构环境事实（§49、§156–§158、§209）。
Local commit only
  本篇通过【阶段性完成门槛】后本地 commit；不要 push（见本篇末尾）。
```

以下为原文正文（自原文开篇起逐字保留）：

# Prism M9 — Environment Engine
## 从“验证项目”到“理解项目所处的工程环境”

你现在进入 **Prism M9：Environment Engine**。

M0–M8 已建立：

```text
M1 = Project Facts
M2 = Code Facts
M3 = Relationship Facts
M4 = Git Facts
M5 = Change Facts
M6 = Impact Facts
M7 = Health Facts
M8 = Verification Facts
```

M9 建立：

```text
Environment Facts
```

核心问题从：

```text
项目是什么？
代码是什么？
发生了什么？
哪里有风险？
验证是否通过？
```

进一步扩展为：

```text
这个项目依赖怎样的本机环境？
当前机器实际提供了什么？
项目声明的环境要求与当前环境是否一致？
哪些工具、运行时、SDK、包管理器、环境变量、端口、进程、配置会影响它？
```

---

# 0. M9 的核心定位

M9 的职责：

> **发现、规范化、展示和比较“项目所处的本地工程环境事实”。**

核心链：

```text
Project
  ↓
Environment Requirements
  ↓
Local Environment Discovery
  ↓
Environment Snapshot
  ↓
Requirement Matching
  ↓
Environment Findings
  ↓
Environment Report
```

---

# 1. M9 不是什么

M9 不是：

```text
IDE
Package Manager
Installer
DevOps Platform
Container Manager
Process Manager
Terminal Emulator
System Monitor
Cloud Environment
Remote Executor
```

尤其不要做：

```text
自动安装 Node
自动安装 Python
自动修改 PATH
自动修改系统环境变量
自动启动 Docker
自动启动数据库
自动杀进程
自动关闭端口
自动修改 SDK
自动升级依赖
自动修改项目配置
```

M9 第一版：

> **Observe, Compare, Explain**

而不是：

> **Install, Repair, Mutate**

---

# 2. 最重要的边界

M8：

```text
Can this verification check run?
What happened when it ran?
```

M9：

```text
What is the environment around this project?
What does the project require?
What does the machine provide?
Where do they differ?
```

例如：

M8：

```text
npm test
→ Unavailable
```

M9：

```text
Required executable:
node
npm

Detected:
node = absent
npm = absent

Environment finding:
Required Node.js runtime is unavailable.
```

---

# 3. M9 核心原则

必须严格遵守：

```text
Deterministic First
Evidence First
Read Only by Default
Explicit Uncertainty
Project-relative identity
Local-first
No telemetry
No automatic repair
No secret exposure
Bounded discovery
```

---

# 4. M9 与 M8 的关系

M8 可以调用 M9：

```text
M8 Verify
   ↓
EnvironmentResolver
   ↓
M9 environment facts
```

但：

> M9 不依赖 M8 才能运行。

M9 必须能够独立：

```bash
prism env .
```

---

# 5. M9 与 M1 的关系

M1：

```text
Project Discovery
```

发现：

```text
Node project
Python project
.NET project
Rust project
```

M9：

```text
Environment
```

发现：

```text
Node = 22.16.0
npm = 11.x
Python = 3.13.x
.NET SDK = 9.x
Rust = 1.x
Cargo = ...
```

M1 是：

> 项目事实。

M9 是：

> 机器 / 工程环境事实。

---

# 6. M9 与 M3 的关系

M3：

```text
Project dependencies
```

M9：

```text
Tools and runtime environment
```

例如：

```text
Project:
requires Python 3.12+

Environment:
Python 3.11
```

M9 可以判断：

```text
Requirement mismatch
```

---

# 7. M9 与 M7 的关系

M7：

```text
Health Finding
```

M9：

```text
Environment Finding
```

例如：

```text
Environment:
Node < required version
```

M9 记录环境事实。

是否：

```text
High severity health problem
```

仍然由：

> Health policy / M7 integration

决定。

不要把 M9 变成第二套 Health Engine。

---

# 8. M9 与 M8 的关系

M8：

```text
Tool unavailable
```

M9：

```text
Tool discovery:
node not found
```

M8：

> 使用这个事实决定 check 是否可执行。

M9：

> 保存完整环境事实。

---

# 9. M9 与 M10 的关系

M9 提供：

```text
EnvironmentSnapshot
EnvironmentRequirement
EnvironmentFinding
ToolchainFact
```

M10：

```text
把必要环境信息压缩到 AI Context Pack。
```

不要 M9 现在做：

```text
LLM context selection
RAG
embeddings
MCP
AI prompt
```

---

# 10. M9 与 M11 的关系

M9：

```text
Current environment snapshot
```

M11：

```text
Historical environment snapshots
```

M9 不负责长期历史数据库。

---

# 11. Environment Model

建立核心：

```rust
EnvironmentSnapshot
```

建议：

```rust
EnvironmentSnapshot {
    id,
    project_id,
    captured_at,
    host,
    operating_system,
    architecture,
    shell,
    toolchains,
    package_managers,
    runtimes,
    sdks,
    environment_variables,
    paths,
    ports,
    processes,
    project_requirements,
    findings,
    diagnostics,
}
```

根据已有架构调整。

---

# 12. Snapshot 的含义

这里的：

```text
EnvironmentSnapshot
```

不是 M11 的：

```text
ProjectSnapshot
```

严格区分：

```text
ProjectSnapshot
→ 项目在某个时间点的整体状态

EnvironmentSnapshot
→ 本地工程环境在某个时间点的环境状态
```

---

# 13. Environment identity

建议：

```text
EnvironmentSnapshotId
```

每次主动重新扫描：

> 可以创建新的 snapshot。

不要求：

```text
稳定 deterministic ID
```

因为它代表一次环境观测。

---

# 14. Host information

收集：

```text
OS
OS version
architecture
hostname if safe/useful
username if safe/useful
machine identity if necessary
```

默认：

> 尽量减少不必要的身份信息。

尤其：

```text
不要默认把 username / hostname
上传或作为核心产品数据。
```

本地报告可以根据用户需求展示。

---

# 15. Operating System

至少规范化：

```text
Windows
macOS
Linux
Other
```

附加：

```text
version
kernel/runtime version
architecture
```

不要让 GUI 依赖：

```text
"Microsoft Windows 11 Pro ..."
```

这种原始字符串。

---

# 16. Architecture

规范：

```text
x86
x86_64
arm64
arm
unknown
```

统一：

```rust
Architecture
```

---

# 17. Shell

发现：

```text
PowerShell
pwsh
cmd
bash
zsh
fish
unknown
```

M9 只是记录：

> 当前 shell。

不要把 shell 变成：

> Prism 自己的 command execution engine。

---

# 18. Shell path

记录：

```text
executable
version if available
```

但：

> 不要把完整 shell profile 内容读进报告。

---

# 19. PATH

PATH 很重要。

但：

> 不要直接将完整 PATH 无脑塞入 UI。

应该：

```text
parse PATH entries
normalize
deduplicate
detect duplicates
```

---

# 20. PATH representation

例如：

```text
PATH entries:

C:\Program Files\nodejs
C:\Users\...\cargo\bin
...
```

可以：

```text
ordered entries
existence
directory type
```

但用户身份路径可以：

```text
redact / shorten
```

根据 privacy policy。

---

# 21. PATH diagnostics

可以发现：

```text
duplicate PATH entries
missing PATH directory
nonexistent PATH directory
shadowing possibility
```

但不要轻率断言：

```text
Node command is broken because of PATH ordering.
```

除非有实际 evidence。

---

# 22. Executable discovery

核心：

```rust
ExecutableInfo
```

包括：

```rust
ExecutableInfo {
    name,
    path,
    exists,
    version,
    source,
}
```

---

# 23. Which tool?

第一版重点：

```text
node
npm
pnpm
yarn
bun

python
python3
pip
pipx
uv
poetry

dotnet
msbuild

rustc
cargo
rustup

java
javac
mvn
gradle

go

git

docker
docker compose

clang
clang++
gcc
g++
cmake
ninja

powershell
pwsh
```

实际实现范围以用户项目与已有代码为准。

不要为了“覆盖所有工具”虚构大量 adapter。

---

# 24. Tool discovery strategy

优先：

```text
PATH lookup
```

其次：

```text
known platform locations
```

最后：

```text
ecosystem-specific discovery
```

不要：

```text
全盘扫描 C:
```

---

# 25. No full-disk scan

禁止：

```text
C:\**
/
```

全盘递归搜索工具。

原因：

- 极慢
- 权限问题
- 隐私
- 巨大 IO
- 用户体验差

---

# 26. Tool source

每个工具记录：

```text
source:
PATH
KnownLocation
ProjectLocal
VersionManager
Unknown
```

例如：

```text
node
path = C:\Program Files\nodejs\node.exe
source = PATH
```

---

# 27. Multiple versions

非常重要。

例如：

```text
node
→ C:\Program Files\nodejs\node.exe
```

同时：

```text
nvm
→ another node
```

M9 可以记录：

```text
active executable
```

以及可识别的：

```text
version manager
```

但：

> 不需要第一版列出所有可切换版本。

---

# 28. Tool version

如果可安全执行：

```bash
node --version
python --version
dotnet --version
cargo --version
```

记录规范化：

```text
ToolVersion {
    raw,
    major,
    minor,
    patch,
}
```

如果版本格式不能解析：

```text
raw_version
```

保留。

---

# 29. Version semantics

不要只比较字符串：

```text
"10" > "9"
```

应该使用：

```text
semantic/version-aware comparison
```

对不符合 SemVer 的工具：

> 使用工具专用 parser 或保守比较。

---

# 30. Version manager

识别：

```text
nvm
fnm
asdf
pyenv
rustup
conda
sdkman
```

但只是：

```text
Environment fact
```

不要切换版本。

---

# 31. Runtime model

统一：

```rust
RuntimeInfo
```

例如：

```text
Node.js
Python
.NET
JVM
Go
Rust
```

记录：

```text
name
version
path
architecture
source
```

---

# 32. SDK model

SDK 和 runtime 要分开。

例如：

```text
.NET Runtime
.NET SDK
```

或：

```text
Java Runtime
Android SDK
```

不要混成：

```text
Tool
```

一个字符串字段。

---

# 33. SDK discovery

优先读取：

```text
native SDK tools
project config
known SDK paths
```

例如：

```text
dotnet --list-sdks
rustup toolchain list
```

若执行成本合理。

---

# 34. SDK output normalization

例如：

```text
.NET SDK 8
.NET SDK 9
```

统一：

```text
SdkInfo[]
```

不要只有：

```text
sdk_string
```

---

# 35. Package manager

统一模型：

```rust
PackageManagerInfo
```

例如：

```text
npm
pnpm
yarn
bun
pip
uv
poetry
cargo
nuget
maven
gradle
```

记录：

```text
name
version
path
project association
source
```

---

# 36. Active package manager

例如：

```text
package.json
pnpm-lock.yaml
```

应该优先推断：

```text
pnpm
```

而不是发现机器上：

```text
npm
```

就说：

> project uses npm.

---

# 37. Package manager provenance

必须回答：

```text
为什么 Prism 判断这个项目使用 pnpm？
```

例如：

```text
pnpm-lock.yaml
packageManager field
```

保存 evidence。

---

# 38. Project runtime requirements

M1/M3/M8 的配置中可能出现：

```text
Node >= 22
Python >= 3.12
.NET >= 8
Rust >= 1.85
```

M9 应规范：

```rust
EnvironmentRequirement
```

---

# 39. Requirement model

建议：

```rust
EnvironmentRequirement {
    id,
    kind,
    subject,
    operator,
    expected,
    source,
    evidence,
    confidence,
}
```

---

# 40. Requirement kind

至少：

```text
Runtime
SDK
Tool
PackageManager
OS
Architecture
EnvironmentVariable
File
Directory
Port
Service
Container
```

---

# 41. Operators

至少：

```text
Exists
Equals
NotEquals
AtLeast
AtMost
InRange
```

必要时：

```text
Compatible
```

但第一版不要过度抽象。

---

# 46. Environment matching

核心：

```text
Requirement
    +
Actual Environment
    ↓
RequirementMatch
```

---

# 47. Match result

建议：

```text
Satisfied
Unsatisfied
Unknown
NotApplicable
```

---

# 48. Example

```text
Requirement:
Node >= 22

Actual:
Node 20

Result:
Unsatisfied

Evidence:
package.json: engines.node
node --version
```

---

# 49. Unknown

例如：

```text
Requirement:
CUDA >= 12
```

Prism 找不到可靠检测方法。

不要：

```text
Unsatisfied
```

而：

```text
Unknown
```

---

# 50. NotApplicable

例如：

```text
Android SDK
```

项目并不是 Android 项目。

不要：

```text
Missing
```

---

# 54. Environment variables

这是敏感区域。

M9 只能默认：

```text
name
present / absent
possibly_sensitive
source
```

例如：

```text
OPENAI_API_KEY
present
sensitive=true
```

不要：

```text
OPENAI_API_KEY=sk-xxxxx
```

---

# 55. Sensitive variable detection

模式：

```text
KEY
TOKEN
SECRET
PASSWORD
PASS
CREDENTIAL
PRIVATE
AUTH
BEARER
```

以及：

```text
*_KEY
*_TOKEN
*_SECRET
```

采用：

> conservative detection。

---

# 56. Non-sensitive variables

例如：

```text
NODE_ENV=development
CI=true
LANG=en_US.UTF-8
```

可以根据 policy 展示值。

但：

> 第一版 UI 默认只展示 key + classification。

---

# 57. Environment variable source

可能来自：

```text
ProcessEnvironment
ProjectEnvFile
ShellProfile
CIConfig
PrismConfig
Unknown
```

但是：

> 默认不要读取 shell history。

---

# 61. Never echo secrets

任何环境扫描：

禁止日志：

```text
TOKEN=secret
```

也禁止：

```text
debug!("{:?}", env)
```

---

# 62. Project-local tools

例如：

```text
node_modules/.bin/eslint
.venv/Scripts/pytest.exe
.venv/bin/pytest
```

M9 应优先发现：

> 项目 local tool。

否则全局工具可能导致：

```text
global version mismatch
```

---

# 63. Local vs global tool

统一：

```text
scope:
ProjectLocal
UserGlobal
System
Unknown
```

---

# 64. Example

```text
eslint
project-local = 9.x
global = 8.x
```

M9：

```text
Project-local eslint is available.
```

不要直接：

```text
eslint version conflict
```

除非项目实际选择 global 版本。

---

# 65. PATH shadowing

例如：

```text
Python
```

PATH 中：

```text
A\python.exe
B\python.exe
```

M9 可以报告：

```text
Multiple Python executables discovered in PATH.
Active resolution:
A\python.exe
```

这是事实。

不要猜：

> B is wrong.

---

# 66. Executable resolution

必须尽可能模拟：

> 用户运行 command 时实际找到的 executable。

例如：

```text
where.exe node
which node
command -v node
```

跨平台统一成：

```text
ExecutableResolution
```

---

# 67. Resolution result

```text
Resolved
NotFound
Multiple
Error
Unknown
```

---

# 68. Project package binaries

例如 Node：

```text
node_modules/.bin
```

应纳入：

> project-local tool resolution。

Python：

```text
.venv/bin
.venv/Scripts
```

同理。

---

# 69. Virtual environments

识别：

```text
.venv
venv
virtualenv
conda
```

记录：

```text
exists
interpreter
version
active?
```

不要：

> 自动 activate。

---

# 70. Python interpreter precedence

如果：

```text
pyproject.toml
.venv
PATH python
```

同时存在：

应分别记录：

```text
project-local interpreter
active process interpreter
PATH interpreter
```

如果无法确认：

```text
Unknown
```

---

# 104. Lockfiles

M9 不需要分析所有 dependency semantics。

但可以识别：

```text
package-lock.json
pnpm-lock.yaml
yarn.lock
uv.lock
poetry.lock
Cargo.lock
packages.lock.json
```

记录：

```text
lockfile exists
package manager association
```

依赖分析仍由：

> M3 / future dependency tooling

负责。

---

# 105. Package manager mismatch

例如：

```text
packageManager = pnpm
```

但：

```text
pnpm absent
npm present
```

M9：

```text
Required package manager unavailable.
```

不是：

```text
npm is good enough
```

---

# 106. Package manager version requirement

例如：

```text
packageManager:
pnpm@10.17.0
```

比较：

```text
actual pnpm
```

输出：

```text
Satisfied
Unsatisfied
Unknown
```

---

# 107. Corepack

Node 项目可能：

```text
packageManager field
```

由 Corepack 管理。

M9：

> 可以记录 Corepack availability。

但：

> 不自动 enable/install。

---

# 108. PATH normalization

跨平台：

```text
case sensitivity
separator
symlink
relative segments
```

必须统一。

---

# 109. Environment path canonicalization

安全地：

```text
normalize
canonicalize where possible
```

但：

> symlink/case-sensitive filesystem 差异必须保留。

---

# 110. Symlink awareness

如果工具路径是：

```text
symlink → target
```

可以记录：

```text
resolved_path
original_path
```

不要：

> 修改 symlink。

---

# 111. Permission checks

必要时：

```text
readable
executable
writable
```

但不要：

> 尝试修改权限。

---

# 112. Tool execution safety

发现工具版本时：

优先使用：

```text
--version
-version
version
```

来自：

> trusted adapter。

不要执行：

```text
tool --help
```

因为某些工具：

> help command 本身可能产生复杂 side effect。

---

# 113. No arbitrary executable execution

M9 绝不能因为：

```text
README
```

里出现：

```bash
curl ...
powershell ...
```

就执行。

工具 discovery 只允许：

> allowlisted version/discovery commands。

---

# 114. Tool command allowlist

建议：

```rust
ToolProbe
```

包含：

```text
program
args
timeout
expected_output_kind
```

例如：

```text
node --version
npm --version
```

---

# 115. Tool probe timeout

所有 probe：

> 必须极短 + bounded。

不能某工具卡住：

```text
prism env
```

整个界面冻结。

---

# 116. Probe cancellation

GUI 关闭 / cancel：

```text
EnvironmentScan
→ cancel
→ terminate probes
```

---

# 117. Parallel probes

可以：

```text
node
python
dotnet
cargo
git
```

并发 probe。

但：

> bounded concurrency。

---

# 118. Discovery stages

环境扫描建议：

```text
Host
  ↓
Toolchain
  ↓
Project Requirements
  ↓
Environment Variables
  ↓
Processes
  ↓
Ports
  ↓
Matching
  ↓
Findings
```

---

# 119. Progress

GUI：

```text
Detecting host...
Detecting runtimes...
Detecting SDKs...
Checking package managers...
Inspecting project requirements...
Checking ports...
Comparing requirements...
```

不要伪造 0–100%。

---

# 120. Lazy discovery

非常重要：

> 不要默认一次扫描所有可能工具。

基础：

```text
OS
Architecture
Git
Project-specific ecosystem
```

然后根据项目类型扩展：

Node → node/npm/pnpm/yarn

Python → python/pip/uv/poetry

C# → dotnet

Rust → rustc/cargo/rustup

---

# 121. Environment profile

建议：

```text
EnvironmentProfile
```

例如：

```text
Web
Python
.NET
Rust
C/C++
Container
Generic
```

由 M1 project detection 提供启发。

---

# 122. Generic mode

未知项目：

至少：

```text
OS
Architecture
Shell
Git
PATH
common tool availability
```

不要：

> 因为项目未知就失败。

---

# 123. Project-specific scan

例如 TS project：

```text
Node
Package manager
TypeScript compiler
ESLint
Prettier
Vite/Webpack/etc. if configured
```

只做：

> project-relevant discovery。

---

# 126. Environment collector

负责：

```text
machine
→ actual environment facts
```

不要混成：

```text
NodeAdapter
→ requirements + environment + findings + UI
```

---

# 127. Matching engine

独立：

```text
EnvironmentMatcher
```

输入：

```text
requirements
actual_environment
```

输出：

```text
matches
conflicts
unknowns
```

---

# 138. Environment completeness

使用：

```text
Complete
Partial
Unknown
```

例如：

```text
Host facts: complete
Node: complete
Python: complete
GPU: not inspected
Database: unknown
```

整体：

```text
Partial
```

---

# 139. No GPU implementation by default

CUDA/GPU 可以留接口：

```text
GpuInfo
```

但第一版不是核心。

除非：

> 用户项目真实需要。

---

# 140. CPU / memory / disk

第一版可以提供轻量：

```text
CPU logical count
memory total
disk available
```

但：

> 不要做系统监控工具。

这些数据主要作为：

```text
environment context
```

---

# 141. Resource requirement

未来可以：

```text
requires >= 8GB RAM
requires >= 20GB disk
```

但第一版：

> 只在项目明确声明时比较。

不允许 Prism 自己规定：

```text
8GB is enough
```

---

# 142. Resource sampling

读取：

```text
memory total
disk free
```

属于：

> snapshot-time facts。

不要连续监控。

---

# 143. Disk context

对于项目 root 所在 filesystem：

记录：

```text
available_bytes
```

可帮助解释：

```text
build unavailable due to disk
```

但：

> 只有真实 command / OS error 支持时才做因果判断。

---

# 144. CPU context

记录：

```text
logical CPUs
architecture
```

而不是：

```text
CPU load chart
```

---

# 145. Memory privacy

系统内存：

> 不敏感。

但不要顺带读取：

```text
process memory dump
```

---

# 146. OS capability

不要为每个 OS 做：

```text
1000 feature flags
```

第一版仅：

```text
filesystem
process
port
executable
shell
```

相关能力。

---

# 147. Capability model

建立：

```rust
EnvironmentCapability
```

例如：

```text
ProcessInspection
PortInspection
ExecutableDiscovery
ToolVersionProbe
EnvironmentVariableInspection
```

这样权限不足时：

```text
Capability unavailable
```

而不是：

> engine crash。

---

# 148. Permission errors

例如：

```text
cannot inspect another user's process
```

结果：

```text
Unknown
PermissionDenied
```

不要：

> 把整个 Environment Scan 标记为 Failed。

---

# 149. Partial scan

环境扫描应支持：

```text
partial success
```

例如：

```text
OS        OK
Toolchain OK
PATH      OK
Processes PARTIAL
Ports     UNAVAILABLE
```

---

# 150. Diagnostic model

复用 Core：

```text
Diagnostic
```

例如：

```text
Permission denied while inspecting process 1234.
```

---

# 151. Environment source evidence

所有关键事实必须带：

```text
source
evidence
```

例如：

```text
Node 22.14.0

Source:
PATH executable

Evidence:
C:\Program Files\nodejs\node.exe
node --version
```

---

# 152. No source code ingestion

M9 不需要：

> 再次解析全部项目源码。

它可以直接使用：

```text
M1 ProjectDiscovery
M2 ProjectCodeModel
M3 Graph
```

已有事实。

---

# 153. Reuse project facts

例如：

M1：

```text
package.json exists
```

M9：

```text
read only specific relevant config
```

不要重复：

> full project scan。

---

# 154. Performance

`prism env .` 应避免：

```text
hundreds of processes
```

默认只 probe：

> project-relevant tools + lightweight host facts。

---

# 155. Probe batching

可以：

```text
single command
→ multiple version facts
```

如果工具支持。

例如：

```text
dotnet --info
```

但 parser 必须可靠。

---

# 156. Tool probe failure

如果：

```text
node --version
```

返回非零：

记录：

```text
probe failed
```

不要：

> node missing。

除非 executable lookup 本身失败。

---

# 157. Executable exists but probe fails

语义：

```text
exists = true
version = unknown
probe = failed
```

这比：

```text
node unavailable
```

准确。

---

# 158. Version mismatch without version

如果：

```text
required Node >= 22
node exists
version unknown
```

结果：

```text
Unknown
```

不是：

```text
Unsatisfied
```

---

# 159. Package manager install status

不要：

```text
pnpm not in PATH
→ automatically use corepack
```

先：

```text
detect corepack
```

然后：

> 展示可能路径。

---

# 160. Environment shell consistency

如果：

GUI application：

```text
PATH
```

与：

Terminal：

```text
PATH
```

不一致，

可以检测：

> M9 当前 process environment 与 shell environment 可能不同。

但不要假定 shell command output 一定等于 GUI environment。

---

# 161. Shell environment probe

第一版不建议：

```text
启动用户 shell profile
```

因为：

- side effects
- startup scripts
- interactive prompts
- unpredictable environment

优先：

> 当前 process env + known tool lookup。

---

# 162. Environment discrepancy

如果 GUI process：

```text
node = unavailable
```

但系统 shell：

```text
node = found
```

未来可支持：

```text
shell probe
```

但第一版最好：

> 明确标记 scope。

---

# 163. Scope

Environment facts 必须标注：

```text
scope:
Process
User
System
Project
Unknown
```

---

# 164. Example

```text
PATH:
scope = Process
```

而：

```text
System PATH:
scope = System
```

不要混成一个 PATH。

---

# 165. User PATH

Windows / Unix：

可以根据 platform APIs 获取。

但：

> 用户级环境配置获取失败时，返回 Unknown。

---

# 166. Environment variable precedence

例如：

```text
System
User
Process
Project env
```

必须明确：

> 哪个值是真正 active。

M9 不应简单覆盖。

---

# 167. Active environment

最重要：

> 对项目命令真正可见的 environment。

因此：

```text
Process scope
```

是最可靠的 baseline。

---

# 168. Process environment capture

只记录：

```text
known safe variables
```

或者：

```text
names + classified availability
```

不要：

> 全量 dump。

---

# 176. Probe side effects

所有 environment probes：

必须尽量：

```text
read-only
low-cost
bounded
```

禁止：

```text
install
update
start
stop
delete
modify
```

---

# 178. Git environment

M4 已经提供 Git facts：

```text
git executable
repo
branch
status
```

M9 只补充：

```text
git version
git executable path
Git config scope if needed
```

不要重复实现 Git.

---

# 179. Git config

如果需要：

可以读取：

```text
user.name
user.email
core.autocrlf
core.filemode
```

但：

> 邮箱属于可能的个人信息。

GUI 默认可以：

```text
configured
```

而非自动展示完整个人信息。

---

# 180. Git config safety

不要读取：

```text
credential.helper contents
```

或任何：

```text
credential
token
password
```

值。

---

# 181. SSH environment

只记录：

```text
ssh executable available
```

不要读取：

```text
~/.ssh/private keys
```

不要扫描私钥内容。

---

# 182. Certificate environment

不要：

> 搜索整个系统证书库。

除非项目明确需要。

---

# 183. Java / Android

如果项目是 Android：

M9 可以发现：

```text
Java
Android SDK
adb
gradle
```

但：

> 第一版支持范围依项目实际需求。

---

# 184. CUDA / GPU

如果用户项目明确声明：

可以扩展：

```text
nvidia-smi
nvcc
CUDA version
GPU
```

否则：

> 非核心。

---

# 185. Environment plugins

不要第一版：

```text
dynamic plugin marketplace
```

采用：

```text
static adapters
```

和：

```text
trait
```

即可。

---

# 186. Suggested architecture

如果现有 Rust workspace 允许：

```text
prism-environment/
├── model
├── host
├── executable
├── runtime
├── sdk
├── package_manager
├── requirements
├── matcher
├── variables
├── process
├── ports
├── container
├── probes
├── findings
└── tests
```

不要机械拆出过多 crate。

---

# 187. Layering

推荐：

```text
Collectors
    ↓
Normalized Environment Facts
    ↓
Requirement Providers
    ↓
Matcher
    ↓
Findings
    ↓
Report
```

---

# 188. Collector boundary

例如：

```text
ProcessCollector
PortCollector
ToolCollector
HostCollector
EnvCollector
```

只负责：

> raw facts → normalized facts。

---

# 189. Normalization

例如不同平台：

```text
where node
which node
```

最终：

```text
ExecutableInfo
```

统一。

---

# 190. Platform adapter

推荐：

```rust
PlatformEnvironmentProvider
```

实现：

```text
WindowsEnvironmentProvider
UnixEnvironmentProvider
```

共享核心：

```text
EnvironmentMatcher
Requirements
Models
```

---

# 191. Cross-platform first

Windows：

重点测试：

```text
PowerShell
cmd
where.exe
process tree
Windows paths
PATH casing
job objects
```

Linux/macOS：

重点：

```text
which
ps
ss/lsof
process group
Unix paths
```

---

# 192. Windows path case

Windows：

```text
C:\Node
c:\node
```

一般视为：

> same path identity where appropriate。

Linux：

```text
/node
/Node
```

可能不同。

---

# 193. Environment paths with Unicode

测试：

```text
C:\项目\测试
/用户/项目
```

以及：

```text
emoji
spaces
non-ASCII
```

---

# 196. Tool tests

测试：

```text
found
missing
multiple
version parse success
version parse fail
probe timeout
probe failure
```

---

# 198. Environment matching tests

必须：

```text
Satisfied
Unsatisfied
Unknown
NotApplicable
```

全部覆盖。

---

# 199. Sensitive variable tests

测试：

```text
API_KEY
PASSWORD
TOKEN
SECRET
DATABASE_URL
PORT
NODE_ENV
```

确认：

> classification 正确。

---

# 200. Redaction tests

真实测试：

```text
command args
stdout
stderr
environment
diagnostics
reports
logs
```

都不能泄漏测试 secret。

---

# 203. Tool provenance tests

例如：

```text
node from PATH
node from project-local
```

必须：

> 正确区分 scope/source。

---

# 204. Version comparison tests

测试：

```text
20
20.1
20.10
22
22.0.0
```

避免字符串比较 bug。

---

# 205. Project fixtures

至少：

```text
Node project
Python project
.NET project
Rust project
Generic project
Docker project
Conflicting requirements project
Missing tool project
Port conflict project
Sensitive env project
```

---

# 209. No fabricated environment

如果：

```text
Prism cannot discover Python
```

不要：

```text
python = not installed
```

除非：

> executable lookup 已确认 not found。

否则：

```text
Unknown
```

---

# 240. Exit codes

建议：

```text
0
→ environment requirements satisfied / no required mismatch

non-zero
→ required requirement unsatisfied / explicit environment error
```

但：

> Unknown 不应自动等于 failed。

具体 exit policy 必须与 M8 保持一致。

---

# 241. `--strict`

很有价值：

```bash
prism env . --strict
```

定义：

```text
Unknown
```

是否视为失败。

第一版如果实现，必须明确文档化。

---

# 242. Default strictness

默认建议：

> Unknown ≠ Unsatisfied。

原因：

```text
absence of evidence
≠
evidence of absence
```

---

# 243. Environment errors

如果 OS API 完全失败：

```text
EnvironmentScan Error
```

但：

> 尽可能返回 partial facts。

---

# 244. Partial environment semantics

例如：

```text
Host             Passed
Tool discovery   Passed
Process discovery Partial
Port discovery   Unavailable
Requirements     Passed
```

整体：

```text
Partial
```

---

# 245. Environment scan diagnostics

例如：

```text
Port enumeration requires elevated privileges.
Some process ownership information may be unavailable.
```

不是：

```text
Environment broken.
```

---

# 246. No root/admin by default

Prism 不应要求：

```text
Administrator
root
sudo
```

才能正常运行。

如果某功能需要：

> gracefully degrade。

---

# 247. Elevated mode

第一版不要：

```text
sudo prism
```

自动 elevation。

如果未来支持：

> 用户显式操作。

---

# 248. Security boundary

M9 读取：

```text
environment
process metadata
ports
paths
```

因此：

> 一定存在敏感信息风险。

默认：

```text
local only
no telemetry
no remote upload
```

---

# 251. Secret-safe logs

日志：

> 只记录 discovery event。

例如：

```text
Detected node executable.
```

不要：

```text
node path = ...
full environment = ...
```

除非 debug mode 明确且仍经过 redaction。

---

# 252. Debug mode

如果已有：

```text
PRISM_LOG=debug
```

也不能绕过：

```text
secret redaction
```

---

# 253. Telemetry

严格：

```text
No telemetry
No analytics
No crash upload
```

除非用户未来主动配置。

---

# 254. Network

M9 默认：

> 不需要互联网。

工具探测：

```text
node --version
```

本地。

不要：

> 在线查询最新版本。

---

# 255. No external version APIs

不要：

```text
query latest node version
```

M9 关注：

> installed version 与 project requirement。

---

# 258. Match evidence

例如：

```text
Node >=22

Required:
package.json: engines.node

Actual:
node --version → 22.14.0

Result:
Satisfied
```

这就是 Prism 的核心：

> evidence-first environment reasoning。

---

# 260. No M11 implementation leakage

不要现在做：

```text
SQLite
ProjectHistory database
Timeline
snapshot persistence
```

M9 snapshot：

> current session observation object。

---

# 261. In-memory only

默认：

```text
scan
→ EnvironmentSnapshot
→ GUI/CLI/report
```

关闭应用后：

> 可消失。

---

# 264. Verify integration

M8 应能够：

```text
CheckSpec
→ required tool
→ M9 tool facts
```

例如：

```text
Node 22 required
```

M8 不再自己：

> 重复探测 node version。

---

# 265. M8 adapter integration

M8：

```text
EnvironmentResolver
```

可改成：

```text
EnvironmentService
```

读取：

```text
EnvironmentSnapshot
```

---

# 266. Avoid circular dependency

严格：

```text
prism-environment
→ may depend on core/project
```

M8：

```text
prism-verify
→ may consume environment
```

不要：

```text
environment
→ verify
```

反向依赖。

---

# 267. M9 dependency direction

推荐：

```text
prism-core
       ↑
prism-project
       ↑
prism-environment
       ↑
prism-verify
```

具体以 workspace dependency graph 为准。

核心要求：

> 不形成循环依赖。

---

# 268. Environment service API

建议：

```rust
EnvironmentService {
    discover(project, options)
    requirements(project)
    compare(snapshot, requirements)
    relevant_tools(project)
}
```

具体 interface 根据现有 architecture 调整。

---

# 269. Scan options

建议：

```rust
EnvironmentScanOptions {
    include_processes,
    include_ports,
    include_variables,
    include_path,
    include_resources,
    include_containers,
    include_all_tools,
}
```

默认：

```text
project relevant
```

---

# 270. Heavy scans

例如：

```text
all processes
all ports
all SDK versions
```

应该是：

> explicit optional scan。

---

# 282. File system

可检测：

```text
project filesystem
case sensitivity where reliably detectable
free space
```

不要：

> benchmark disk。

---

# 283. Filesystem capability

预留：

```text
CaseSensitivity
SymlinkSupport
PathLengthConstraints
```

但第一版只实现真实需要的能力。

---

# 284. Path length

Windows 项目可能受：

```text
MAX_PATH
```

影响。

不要默认：

> 所有长路径都报错。

只有在：

> 实际 capability/config evidence

支持时才报告。

---

# 285. File encoding

环境层不负责：

> 扫描所有源文件 encoding。

M2 负责代码解析。

---

# 286. Locale

可以记录：

```text
LANG
LC_ALL
system locale
```

但：

> 默认不把完整环境变量内容展示出来。

---

# 287. Timezone

可以记录：

```text
timezone
```

如果有业务意义。

不是 M9 核心。

---

# 288. Locale mismatch

如果测试明确依赖：

```text
en-US
UTF-8
```

可以提供 requirement。

否则：

> 不主动判定 locale 是问题。

---

# 289. Time-sensitive environment

不保存：

```text
“current time” 
```

作为稳定 environment fact。

只记录：

```text
captured_at
```

---

# 290. User-specific differences

Environment：

> 与机器/用户高度相关。

因此报告要明确：

```text
This report describes the current local environment.
```

不能：

> 推断其他机器环境。

---

# 291. Reproducibility

M9 可以提供：

```text
Environment facts
```

帮助回答：

> 为什么我这里能运行、另一台不一定能。

但不要声称：

> “environment snapshot guarantees reproducibility”。

因为：

```text
network
external services
hardware
OS policies
credentials
```

仍可能不同。

---

# 292. Environment export

JSON should include：

```text
schema_version
```

方便以后 M11。

---

# 293. Schema version

所有：

```text
EnvironmentSnapshot
EnvironmentRequirement
EnvironmentMatch
```

应有稳定序列化 schema。

---

# 294. Backward compatibility

M9 第一版不要求历史 migration。

但：

> schema version should be explicit。

---

# 295. CLI machine mode

例如：

```bash
prism env . --json
```

不应输出：

```text
Detecting...
```

到 stdout。

机器模式：

> stdout = machine-readable data。

日志：

> stderr。

---

# 296. CLI text mode

人类模式：

```text
Environment
────────────

Node       22.14.0   OK
npm        11.3.0    OK
Python     3.11.9    MISMATCH
```

---

# 297. CLI color

可使用 status colors。

但：

```text
--no-color
```

必须支持。

---

# 305. Error model

复用：

```text
InvalidInput
NotFound
Io
Command
Config
Permission
Cancelled
Unsupported
Internal
```

如果新增：

```text
Environment
```

确保不会和现有 error architecture 重复。

---

# 306. Permission errors

Windows/Linux/macOS：

> 都可能出现。

统一：

```text
PermissionDenied
```

同时保留：

```text
platform_detail
```

---

# 307. Unsupported platform feature

例如：

```text
Port ownership unavailable
```

输出：

```text
Unsupported
```

不要：

```text
Failed
```

---

# 308. Probe timeout

例如：

```text
docker info
```

卡住。

应该：

```text
ProbeTimedOut
```

不会让：

```text
Environment Scan
```

无限等待。

---

# 309. Probe budgets

建议：

```text
per-tool timeout
total environment scan budget
```

防止：

> 环境扫描无限扩张。

---

# 310. Concurrency budget

例如：

```text
max_parallel_probes
```

使用：

> configuration。

---

# 311. Resource budget

可以控制：

```text
max_processes
max_ports
max_output
max_probe_time
```

避免大机器上数据爆炸。

---

# 312. Process list cap

如果有：

```text
5000 processes
```

不要全部：

> 默认加载 UI。

先：

```text
project relevance filter
```

必要时再展开。

---

# 313. Port list cap

同理。

---

# 314. PATH cap

如果 PATH 异常巨大：

> bounded entries。

并：

```text
truncated
```

---

# 315. Environment variable cap

不要读取：

```text
unbounded env data
```

默认：

> key metadata only。

---

# 316. No secrets scanning expansion

M9 不要变成：

> 全盘 secret scanner。

Secrets detection 的工程健康职责由：

```text
M7
```

负责。

M9 只关注：

```text
environment variables
environment configuration
```

---

# 317. No dependency vulnerability scan

M9 不负责：

```text
CVE
advisory
latest package
```

除非 future external security integration。

---

# 318. No internet-based package checks

不要：

```text
npm registry
PyPI
crates.io
NuGet
```

查询：

> 最新版本。

M9 是 local-first。

---

# 319. No automatic repair

例如：

```text
Node missing
```

不要：

```text
download Node
```

例如：

```text
PATH broken
```

不要：

```text
modify registry
```

例如：

```text
Python mismatch
```

不要：

```text
switch pyenv
```

---

# 320. Suggestions

可以提供：

```text
Suggested action:
Use Node >=22.
```

但：

> suggestion only。

不要执行。

---

# 321. Environment action model

如果未来：

```text
Fix environment
```

也应该是独立模块。

M9 第一版：

> 没有 Fix Environment。

---

# 334. No full project rescan

`prism env`：

> 不应该重新遍历整个项目文件系统，除非 requirement detection 明确需要。

复用 M1 discovery。

---

# 337. Environment snapshot lifecycle

```text
Created
Running
Completed
Partial
Cancelled
Failed
```

如果已有 Task semantics：

> 直接复用。

---

# 338. Snapshot status

不要把：

```text
scan status
```

和：

```text
environment requirement status
```

混在一起。

---

# 339. Schema

建议：

```rust
EnvironmentSnapshot
├── Host
├── Toolchains
├── Requirements
├── Matches
├── Variables
├── Paths
├── Processes
├── Ports
├── Containers
└── Diagnostics
```

---

# 340. JSON normalization

路径：

> 使用规范 platform-independent representation。

数值：

> 使用 typed values。

版本：

> structured + raw。

状态：

> enums。

---

# 343. Terminal output escaping

CLI：

> 必须正确 escape control sequences。

环境变量/进程 command line 可能包含：

```text
ANSI
```

不要让：

> 恶意 terminal escape 污染终端。

---

# 346. Central privacy layer

如果 M0/M7 已有：

```text
Redactor
SecretFilter
PathPrivacyPolicy
```

必须复用。

不要：

```text
EnvironmentRedactor
GitRedactor
VerifyRedactor
```

各造一套互不兼容逻辑。

---

# 348. No raw environment export by default

即使：

```bash
prism env --json
```

也不是：

```text
dump all env vars
```

仍然：

> redacted structured output。

---

# 349. Debug dump

如果未来要：

```text
--unsafe-debug
```

也不能：

> 默认开放。

第一版不要提供 bypass。

---

# 384. No AI

M9：

```text
No LLM
No embeddings
No prompt
No agent
```

环境判断：

> deterministic。

---

# 385. No database

M9：

```text
No DB
No SQLite
No persistent history
```

除非现有 foundation 已有明确 persistence infrastructure，但不要因为 M9 新增。

---

# 386. No telemetry

必须：

```text
No analytics
No remote logs
No upload
```

---

# 387. No automatic updates

不要：

```text
check latest runtime
download runtime
self-update
```

---

# 388. No package install

不要：

```text
npm install
pip install
cargo install
dotnet tool install
```

---

# 389. No environment mutation

不要：

```text
setx
export persistence
registry edit
symlink modification
PATH modification
version switch
service start
service stop
process kill
```

---

# 391. Code quality

禁止：

```text
environment.rs
3000 lines
```

把所有平台代码塞一起。

按：

```text
collector
platform
probe
matcher
model
```

拆分。

---

# 392. Error resilience

单个 tool：

```text
node --version
```

失败：

> 不影响：

```text
python
rust
git
OS
```

继续扫描。

---

# 393. Failure isolation

单个：

```text
PortCollector
```

失败：

> 不影响 ToolCollector。

---

# 394. Scan cancellation

取消：

```text
EnvironmentScan
```

必须：

```text
stop pending probes
stop process enumeration if possible
return partial snapshot
```

不要：

> panic。

---

# 395. Deterministic ordering

工具列表：

> 按稳定 logical name。

Requirements：

> source location + subject。

Processes：

> dynamic ordering may be PID or stable sort。

Ports：

> numeric ascending。

---

# 【阶段性完成门槛】（M9 Part 1/2 Exit Gate）

> 本章为拆分新增的阶段性收口，不属于原文。原文的 §421 Final Audit、§422 Completion Gate、§423 最终 Commit 由 M9b 承接；本篇以下列门槛收口，全部通过后才可进入 M9b。

```text
[ ] 本篇正文所列要求全部落实（§0–§41、§46–§50、§54–§57、§61–§70、§104–§123、§126–§127、§138–§168、§176、§178–§193、§196、§198–§200、§203–§205、§209、§240–§248、§251–§255、§258、§260–§261、§264–§270、§282–§297、§305–§321、§334、§337–§340、§343、§346、§348–§349、§384–§389、§391–§395）

[ ] EnvironmentSnapshot / EnvironmentRequirement / EnvironmentMatch / ToolProbe 模型存在，且有带 schema_version 的稳定序列化（§11–§13、§38–§41、§292–§294、§337–§340）
[ ] Host / OS / Architecture / Shell / PATH 采集与规范化可用（§14–§21、§108–§111、§191–§193）
[ ] 可执行文件发现、ToolProbe allowlist、结构化版本比较可用（§22–§30、§112–§117、§155–§158、§204）
[ ] Runtime / SDK / Package manager 发现可用（§31–§37、§104–§107、§159）
[ ] RequirementMatch 匹配语义正确：Satisfied / Unsatisfied / Unknown / NotApplicable；Unknown ≠ Missing ≠ Broken（§46–§50、§209）
[ ] 敏感环境变量识别与 redaction 生效：日志、diagnostics、`--json` 输出均无 secret（§54–§57、§61、§199–§200、§248、§251–§252、§346、§348）
[ ] Probe 超时 / 取消 / 有界并发 / 失败隔离生效，单个工具或 collector 失败不影响整体扫描（§115–§117、§146–§150、§308–§311、§392–§394）
[ ] EnvironmentService 可独立运行（不依赖 M8），workspace 依赖方向无循环；M8 可消费 M9 facts（§4、§264–§268）
[ ] `prism env <path>` 与 `--json` 基础输出可用：stdout 只输出数据、日志走 stderr；exit code 与 strictness 语义明确且已文档化（§240–§242、§295–§297）
[ ] 本篇范围单测通过：tool discovery / version comparison / requirement model 与 matching / redaction / path normalization / tool provenance / project fixtures（§196、§198–§200、§203–§205）
[ ] 无 P0；遗留问题全部记录为 P1 并带入 M9b
```

全部通过后：

```bash
git add -A
git commit -m "feat: add environment facts and matching (M9 part 1/2)"
```

**不要 push。**

完成后进入姊妹篇：`Prism M9b — Environment Requirements Findings and Reports 完整开发提示词.md`（Part 2/2）。
