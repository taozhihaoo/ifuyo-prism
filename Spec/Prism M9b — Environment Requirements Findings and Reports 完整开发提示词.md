# Prism M9b — Environment Requirements, Findings and Reports

## M9 完整开发提示词 · 下篇（Part 2/2）

## 【系列说明】

- 本篇是 **Prism M9（Environment Engine）规格拆分的下篇（Part 2/2）**，内容域：**Environment Requirements, Findings and Reports** —— 项目需求抽取（config > ecosystem metadata > CI > 版本文件 > 文档 > 启发式 的来源层级与 RequirementConflict）、环境变量/`.env` 事实、ports/processes/container 动态事实（observed_at 语义）、PathIssue/ExecutableShadowing/PortConflict/ToolMissing/VersionMismatch 等 EnvironmentFinding 及其与 M7 Finding 体系的集成、`prism env requirements/tools/ports/processes` 完整 CLI、报告与 GUI 数据流、真实项目验证、完整验收清单 + Final Audit + 最终 commit。
- 姊妹篇（上篇 Part 1/2）：`Prism M9a — Environment Facts and Matching 完整开发提示词.md`，内容域：M9 定位与红线、EnvironmentSnapshot 核心模型、host/OS/architecture/shell/PATH 采集与规范化、可执行文件与工具版本探测（ToolProbe allowlist、结构化版本比较）、runtime/SDK/package manager 发现、Requirement 模型基础与 RequirementMatch 匹配语义（Unknown ≠ Missing ≠ Broken）、敏感环境变量识别与 redaction、probe 超时/取消/失败隔离、EnvironmentService、`prism env` 基础输出。
- 两篇均拆分自源文档 `Prism M9 — Environment Engine 完整开发提示词.md`（该文件已拆分完毕，此后不再维护、以两篇为准）。**除【系列说明】/【承接 M9a】/红线摘要/【阶段性完成门槛】为拆分新增的框架内容外，两篇正文全部为原文逐字内容：原文章节编号与标题原样保留（正文形如 "§xx" 的交叉引用跨篇仍然有效），未删减、未改写规范语义、未新增需求。**
- 原文 §区间 → 归属篇 映射表（M9b = 本篇，M9a = 姊妹篇）：

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

- **执行顺序**：先完成并合入 M9a（其【阶段性完成门槛】已通过），再执行本篇；本篇末尾的 §421 Final Audit、§422 Completion Gate、§423 最终 Commit 是 M9 整体的最终收口。

## 【承接 M9a】

本篇假设 Part 1（M9a）**已实现、通过其【阶段性完成门槛】并合入**。以下能力已存在，本篇直接复用、不再重复实现：

```text
核心模型与序列化：EnvironmentSnapshot / EnvironmentRequirement / EnvironmentMatch / ToolProbe、schema_version（M9a §11–§13、§38–§41、§292–§294、§337–§340）
host / OS / architecture / shell / PATH 采集与规范化（M9a §14–§21、§108–§111）
ExecutableInfo 发现、ToolProbe allowlist、结构化版本比较（M9a §22–§30、§112–§117）
runtime / SDK / package manager 发现与 lockfile / Corepack 事实（M9a §31–§37、§104–§107）
RequirementMatch 匹配语义：Satisfied / Unsatisfied / Unknown / NotApplicable；Unknown ≠ Missing ≠ Broken（M9a §46–§50、§209）
敏感环境变量识别与 redaction；中央 privacy layer 复用（M9a §54–§57、§61、§346、§348–§349）
probe 超时 / 取消 / 有界并发 / 失败隔离（M9a §115–§117、§146–§150、§308–§311、§392–§394）
EnvironmentService（discover / requirements / compare / relevant_tools）与扫描选项（M9a §264–§270）
`prism env <path>` 基础输出、`--json`、exit code 与 strictness 契约（M9a §240–§242、§295–§297）
测试基线：tool / matching / redaction / version / provenance 单测与 project fixtures（M9a §196、§198–§200、§203–§205）
```

本篇在其上补齐（括号内为本篇正文章节）：

```text
项目需求抽取：source hierarchy、RequirementConflict、各生态 requirement 识别、RequirementProvider（§42–§45、§71–§77、§101–§103、§124–§125、§169–§171、§197、§201–§202）
环境变量 / `.env` 事实与 Git 集成（§58–§60）
ports / processes / container / service 动态事实、observed_at、refresh / cache 语义（§78–§100、§172–§175、§177、§194–§195、§210–§219）
EnvironmentFinding 体系、findings engine、confidence、与 M7 Finding 集成与状态聚合（§51–§53、§128–§137、§350–§359）
完整 CLI 命令集（§237–§239）
报告（JSON / Markdown / HTML）与 GUI 数据流、IPC、UI 章节（§220–§236、§249–§250、§256–§257、§259、§262–§263、§271–§281、§298–§304、§322–§333、§335–§336、§341–§342、§344–§345、§347、§382–§383）
验证与集成：真实项目、self-audit、golden paths、M8/M7 combined flow、产品集成（§206–§208、§360–§381、§390、§396–§420）
M9 整体收口：Final Audit、Completion Gate、最终 commit、语义链总结（§421–§429）
```

## 【本篇定位与执行红线】（执行摘要，以正文原文为准）

与 M9a 相同的红线在本篇继续生效（摘要，不削弱原文）：

```text
Deterministic First / No AI
  环境判断全部确定性；无 AI / LLM / RAG / MCP / agent（原文 §384，两篇共同红线）。
Observe / Compare / Explain
  不做 Install / Repair / Mutate；finding 只报告：不自动安装、不 kill 进程、不启停容器/服务、不修改环境（原文 §1、§93、§134、§319–§321、§387–§389）。
Read Only / Bounded
  probe 只读、低开销、有界；只关注 listening 端口；禁止全端口扫描与网络扫描（原文 §83、§99–§100、§176）。
No network / No telemetry
  本地优先；不联网、不上传（原文 §253–§255、§318、§386）。
Privacy / Redaction
  报告与 GUI 展示必须经过统一 redaction / privacy renderer；不展示 raw secret 值；不 raw dump 环境变量（原文 §249–§250、§344–§345、§347）。
Evidence First / Explicit Uncertainty
  Unknown ≠ Missing ≠ Broken；证据不足时输出 Unknown / Probable，不得自动升级为 PortConflict / Unsatisfied / Health Failure（原文 §49、§135、§209）。
Local commit only
  最终验收（§422）通过后本地 commit（§423）；不要 push。
```

以下为原文正文（本篇所属章节，按原文编号顺序逐字保留）：

---

# 42. Requirement source

例如：

```text
package.json
pyproject.toml
global.json
rust-toolchain.toml
.tool-versions
README
Prism config
CI configuration
```

---

# 43. Do not trust README equally

README：

```text
Node 18
```

package.json：

```text
engines.node >= 22
```

M9 不应该直接：

```text
pick README
```

而应：

```text
source hierarchy
```

---

# 44. Requirement priority

建议：

```text
Explicit project config
>
ecosystem metadata
>
CI config
>
well-known version files
>
documentation
>
heuristic inference
```

但每一个项目可能不同。

最重要：

> 保留 source 和 conflict，不要 silently overwrite。

---

# 45. Requirement conflict

例如：

```text
package.json:
Node >= 22

README:
Node 20
```

输出：

```text
RequirementConflict
```

而不是：

```text
Node = 22
```

---

# 51. Environment Finding

M9 可以生成：

```rust
EnvironmentFinding
```

例如：

```text
ToolMissing
VersionMismatch
RequirementConflict
ExecutableShadowing
PathIssue
MissingEnvironmentVariable
UnexpectedEnvironment
MissingSdk
MissingPackageManager
PortConflict
ProcessConflict
```

但：

> Findings 是事实 + 明确比较结果。

---

# 52. Finding 与 M7 Finding

如果已有：

```text
Finding
```

不要重复定义两个巨大模型。

建议：

```text
Finding.category = Environment
```

或者：

```rust
HealthFinding
EnvironmentFinding
```

共用底层：

```text
FindingCore
```

以当前 architecture 为准。

核心要求：

> M7 与 M9 不要各自维护互不兼容的 Finding schema。

---

# 53. Environment category

例如：

```text
Environment
  ├── Toolchain
  ├── Runtime
  ├── SDK
  ├── PackageManager
  ├── Path
  ├── Variable
  ├── Port
  ├── Process
  ├── Container
  └── Configuration
```

---

# 58. `.env`

发现：

```text
.env
.env.local
.env.development
```

可以记录：

```text
file exists
contains likely secrets
tracked/untracked from M4
```

但：

> 不把完整 `.env` 内容加载到 environment report。

---

# 59. Git + env integration

非常重要：

如果：

```text
.env
```

且：

```text
M4 = tracked
```

M9 可以提供事实：

```text
Sensitive environment file appears tracked.
```

这可以供：

```text
M7
```

产生 Health finding。

---

# 60. Environment file reading

第一版：

> 只解析安全、必要的 key names。

例如：

```text
API_URL=...
DB_HOST=...
TOKEN=...
```

可以得到：

```text
API_URL
DB_HOST
TOKEN(sensitive)
```

不保存 raw values。

---

# 71. Node environment

识别：

```text
.nvmrc
.node-version
package.json engines
packageManager
```

建立 requirements。

---

# 72. Python environment

识别：

```text
requires-python
pyproject.toml
.python-version
runtime.txt
```

建立 requirements。

---

# 73. .NET environment

识别：

```text
global.json
TargetFramework
TargetFrameworks
Directory.Build.props
```

比较：

```text
required SDK
installed SDK
```

---

# 74. Rust environment

识别：

```text
rust-toolchain
rust-toolchain.toml
Cargo.toml
```

比较：

```text
required toolchain
active rustc
active cargo
```

---

# 75. Java environment

识别：

```text
pom.xml
build.gradle
gradle.properties
.tool-versions
.java-version
```

比较：

```text
JDK
JRE
Maven
Gradle
```

---

# 76. Go environment

识别：

```text
go.mod
go.work
```

比较：

```text
Go version
```

---

# 77. C/C++ environment

识别：

```text
CMakeLists.txt
meson.build
Makefile
```

发现：

```text
compiler
cmake
ninja
make
```

不要强行执行 build。

---

# 78. Container environment

发现：

```text
Dockerfile
docker-compose.yml
compose.yaml
```

可以记录：

```text
Docker-based environment
```

检测：

```text
docker available
docker compose available
```

但是：

> 默认不启动任何 container。

---

# 79. Docker version

可以：

```bash
docker --version
docker compose version
```

仅在：

> Docker executable exists。

否则：

```text
Unavailable
```

---

# 80. Container configuration

可以静态解析：

```text
image
services
ports
environment keys
volumes
```

但：

> 环境变量值不要默认读取/展示。

---

# 81. Port discovery

M9 可以提供：

```rust
PortInfo
```

记录：

```text
port
protocol
address
state
process
source
```

---

# 82. Why ports matter

开发项目常有：

```text
3000
5173
8000
8080
5432
6379
```

M8 执行 test/dev server 时：

> 端口冲突是典型环境问题。

M9 因此可以帮助：

```text
Port 5173 occupied
```

---

# 83. Listening ports only

第一版重点：

> listening TCP ports。

不要扫描：

```text
所有 65535 ports
```

---

# 84. Port discovery platform abstraction

建立：

```rust
PortResolver
```

Windows：

```text
netstat / Get-NetTCPConnection
```

Linux：

```text
ss
```

macOS：

```text
lsof
```

如果平台 API 可用：

> 优先 native API。

---

# 85. Port ownership

尽量解析：

```text
port
PID
process name
```

但：

> 权限不足时必须返回 Unknown。

不能猜。

---

# 86. Process discovery

M9 可以发现：

```rust
ProcessInfo
```

包括：

```text
pid
name
executable
parent_pid if available
command metadata if safe
user if permitted
ports
```

---

# 87. Process privacy

不要默认：

```text
完整 command line
```

因为 command line 可能包含：

```text
--token secret
--password ...
```

需要：

> redaction。

---

# 88. Process list scope

第一版建议：

> 当前用户可见进程 + 必要 system processes。

不要：

> 强制管理员权限扫描所有进程。

---

# 89. Process classification

可以分类：

```text
Node
Python
.NET
Java
Rust
Docker
Database
Browser
Unknown
```

但：

> 基于 executable evidence。

---

# 90. Dev server detection

如果发现：

```text
node process
port 5173
```

只能说：

```text
Node process listening on 5173
```

不要说：

> Vite server running

除非 command line 明确包含：

```text
vite
```

---

# 91. Process-to-project association

如果 command line 或 executable path：

```text
C:\project\node_modules\.bin\vite
```

可以推断：

```text
project association = project
confidence = High
```

如果只是：

```text
node.exe
```

没有项目路径：

> Unknown.

---

# 92. Working directory of process

如果 OS API 可安全提供：

记录：

```text
working_directory
```

否则：

```text
Unknown
```

不要强制管理员权限。

---

# 93. Process conflict

例如：

```text
Project expects port 5173
Actual:
PID 1234 listening 5173
```

Environment finding：

```text
PortConflict
```

但：

> 不自动 kill PID 1234。

---

# 94. Expected ports

从：

```text
package scripts
Docker compose
config
documentation
```

可以发现：

```text
expected port candidates
```

但：

> provenance + confidence 必须明确。

---

# 95. Port requirement

统一：

```rust
EnvironmentRequirement {
    kind: Port,
    subject: 5173,
    operator: Available,
}
```

---

# 96. Port matching

结果：

```text
Satisfied
Unsatisfied
Unknown
```

---

# 97. Service requirements

例如项目要求：

```text
PostgreSQL
Redis
Docker daemon
```

M9 可检测：

```text
docker executable
listening port
process
```

但：

> 不保证 service application-level health。

例如：

```text
5432 listening
```

只能说明：

> something is listening on 5432.

不应该说：

> PostgreSQL is healthy.

---

# 98. Service detection confidence

```text
Port 5432
→ likely PostgreSQL
```

只有在 process / banner / executable evidence 支持时：

```text
HighConfidence
```

否则：

```text
Probable
```

---

# 99. No network probing by default

不要：

```text
scan LAN
scan internet
port scan other machines
```

M9 只关注：

> 本机工程环境。

---

# 100. localhost checks

允许：

```text
localhost
127.0.0.1
::1
```

必要时进行最小范围 connection check。

但是：

> 必须由明确项目要求触发。

不要全端口扫描。

---

# 101. Environment configuration files

发现：

```text
.env
.nvmrc
.node-version
.python-version
global.json
rust-toolchain.toml
.tool-versions
Dockerfile
compose.yaml
```

统一：

```text
ConfigurationSource
```

---

# 102. Environment source hierarchy

推荐：

```text
ProjectConfig
Lock/VersionFile
ToolchainFile
CIConfig
ContainerConfig
Documentation
Heuristic
```

---

# 103. Environment conflicts

例如：

```text
.node-version = 22
package.json engines >= 20
.nvmrc = 18
```

不要自动选择。

生成：

```text
RequirementConflict
```

并显示：

```text
source A
source B
source C
```

---

# 124. Requirement parser architecture

建议：

```text
RequirementProvider
```

实现：

```text
NodeRequirementProvider
PythonRequirementProvider
DotnetRequirementProvider
RustRequirementProvider
ContainerRequirementProvider
GenericConfigProvider
```

---

# 125. Requirement provider

负责：

```text
project facts
→ environment requirements
```

不是：

```text
machine inspection
```

---

# 128. Findings engine

M9 findings 应来自：

```text
matcher
tool discovery
path analysis
port/process analysis
config conflict
```

而不是随意业务规则。

---

# 129. Finding confidence

统一：

```text
Exact
HighConfidence
Probable
Ambiguous
Unknown
```

---

# 130. Example finding

```text
Environment mismatch

Requirement:
Node >= 22

Detected:
Node 20.19.0

Source:
package.json: engines.node
Probe:
node --version

Confidence:
Exact
```

---

# 131. Another finding

```text
Project requires pnpm@10.x

pnpm:
not found

npm:
found

Result:
Unsatisfied

Suggestion:
Use the project's configured package manager.
```

注意：

> 不自动安装。

---

# 132. Path finding

```text
PATH contains a nonexistent directory:

C:\old-node\bin
```

这是：

```text
Informational / Warning
```

具体 severity 根据规则。

不要：

> 直接修改 PATH。

---

# 133. Duplicate executable finding

```text
Multiple node executables found.
Active:
...
Other:
...
```

这是：

> context information。

除非真实导致 requirement mismatch。

---

# 134. Process finding

```text
Port 5173 is already occupied.

PID:
1234

Process:
node

Evidence:
...
```

不要：

> kill process。

---

# 135. Port finding uncertainty

如果：

```text
5173 occupied
```

但：

```text
project expected port = 5173
```

来自 README：

则：

```text
ExpectedPort confidence = Probable
```

不能自动叫：

```text
PortConflict
```

除非来源更可靠或用户配置明确。

---

# 136. Environment health score

禁止：

```text
Environment Score = 73
```

也不要：

```text
Environment Grade = B+
```

Prism 应提供：

```text
Satisfied
Unsatisfied
Unknown
Conflict
```

和 evidence。

---

# 137. No magical “Ready”

不要：

```text
Environment Ready
```

除非：

> 明确 requirements 全部 satisfied。

即便：

```text
Node
Python
Git
```

全部存在：

也不能宣称：

> “项目环境完全正确”。

项目可能还有：

```text
database
API credentials
Docker
CUDA
```

未检测。

---

# 169. Project env variables

项目可以声明：

```text
DATABASE_URL
API_URL
PORT
```

M9：

```text
present / absent
sensitive
source
```

---

# 170. Required environment variables

例如：

```yaml
required_env:
  - DATABASE_URL
  - API_URL
```

匹配：

```text
DATABASE_URL present
API_URL absent
```

---

# 171. Required env variable value validation

默认：

> 只检查 presence。

不要验证 secret 内容是否“正确”。

例如：

```text
API_KEY present
```

不能：

> API_KEY valid

除非未来通过明确、安全的 provider integration。

---

# 172. Network service availability

不要默认：

```text
GET https://api.example.com
```

验证所有 external services。

这已经进入：

> 网络验证 / external integration。

M9 第一版：

> 本地环境优先。

---

# 173. Local service availability

如果项目明确要求：

```text
localhost:5432
```

可以：

> check whether TCP listener exists。

但不能：

> 通过 credentials 登录数据库。

除非未来单独设计。

---

# 174. Database health

不属于 M9 默认职责。

M9：

```text
5432 reachable
```

不等于：

```text
PostgreSQL healthy
```

---

# 175. Docker service health

同样：

```text
docker executable exists
```

不等于：

```text
Docker daemon healthy
```

如果明确执行：

```bash
docker info
```

成功：

才可以报告：

```text
Docker daemon reachable
```

---

# 177. Docker probe

允许：

```bash
docker version
docker info
```

如果：

> explicit environment inspection policy permits.

不要：

```bash
docker compose up
```

---

# 194. Process tests

测试：

```text
current process
child process
unknown process
permission denied
terminated process
```

---

# 195. Port tests

测试：

```text
free
occupied
unknown ownership
IPv4
IPv6
```

---

# 197. Requirement tests

测试：

```text
>=
<=
=
range
unknown
conflict
invalid requirement
```

---

# 201. `.env` tests

测试：

```text
.env
.env.local
.env.example
```

区分：

```text
secret candidate
template/example
```

不能全部误判。

---

# 202. Requirement provenance tests

例如：

```text
package.json
README
Prism config
```

发生冲突：

必须：

```text
preserve all sources
```

---

# 206. Real project validation

至少：

```text
Prism
真实 TypeScript 项目
真实 Python 项目
真实 C# 项目
```

Rust / Docker 根据环境可用性。

---

# 207. Prism self environment

执行：

```bash
prism env .
```

必须能回答至少：

```text
OS
Architecture
Rust
Cargo
Node
Package manager
Git
Project requirements
```

具体以 Prism 实际技术栈为准。

---

# 208. Self-environment correctness

抽查：

```text
node --version
```

与：

```text
Prism Environment UI
```

是否一致。

同样：

```text
rustc --version
cargo --version
git --version
```

---

# 210. Environment scan determinism

同一环境短时间内运行两次：

应该：

```text
same normalized facts
```

除了：

```text
timestamp
process IDs
dynamic ports
```

等 inherently dynamic facts。

---

# 211. Dynamic facts

Process / port：

> 不是静态项目事实。

必须明确：

```text
observed_at
```

---

# 212. Process IDs

PID 会变化。

因此：

> 不要用 PID 作为长期 stable identity。

---

# 213. Executable identity

可以：

```text
normalized executable path
```

作为 identity 的一部分。

---

# 214. Tool identity

例如：

```text
Node.js
```

stable logical identity：

```text
node
```

不要用：

```text
node22.exe
```

作为核心 ID。

---

# 215. Environment diff preparation

未来 M11 需要：

```text
EnvironmentSnapshot A
EnvironmentSnapshot B
```

比较：

```text
tool version changed
runtime added
runtime removed
PATH changed
requirement changed
port changed
```

M9 模型必须：

> 足够结构化，方便后续 diff。

---

# 216. Environment fingerprint

可以预留：

```text
environment_fingerprint
```

不要一开始依赖它做 caching correctness。

因为动态环境很多。

---

# 217. No aggressive cache

工具版本：

> 可以 session cache。

Process/port：

> 默认每次 refresh。

否则 UI 会：

> 显示过期信息。

---

# 218. Refresh semantics

GUI：

```text
Refresh Environment
```

明确：

> 重新扫描。

不能：

```text
cache-only
```

冒充实时状态。

---

# 219. Auto-refresh

第一版不要：

```text
每秒刷新 process/ports
```

这会让 Prism 变成系统监控器。

手动 refresh 足够。

---

# 220. Environment UI

新增真实：

```text
Environment
```

页面。

---

# 221. Overview Environment summary

显示：

```text
OS
Architecture
Runtime status
Toolchain status
Project environment status
Port issues
Process issues
```

不要做：

> CPU/RAM/GPU 华丽仪表盘。

---

# 222. Toolchain section

例如：

```text
Node.js      22.14.0      OK
npm          11.3.0       OK
pnpm         —             Missing
Python       3.13.7        OK
Rust         1.90.0        OK
Cargo        1.90.0        OK
```

---

# 223. Requirements section

例如：

```text
Project requirements

Node       >=22       Satisfied
pnpm       10.x       Satisfied
Python     >=3.12     Unsatisfied
```

---

# 224. Finding section

例如：

```text
Environment findings

1. Python requirement mismatch
2. pnpm not found
3. Port 5173 occupied
```

点击：

> 查看 evidence。

---

# 225. Ports section

例如：

```text
Ports

5173   LISTENING   node   PID 1234
8000   FREE
5432   LISTENING   unknown
```

---

# 226. Processes section

不要默认：

> 显示整个任务管理器。

默认只：

```text
Relevant processes
```

例如：

```text
node
python
dotnet
cargo
docker
```

与项目相关的优先。

---

# 227. Relevant process rules

优先：

```text
process working directory == project
executable path under project
known ecosystem executable
listening on expected project port
```

---

# 228. All processes

可以提供：

```text
Show all visible processes
```

但第一版可以延后。

---

# 229. Environment variable section

默认：

```text
Variable
Status
Sensitivity
Scope
```

例如：

```text
DATABASE_URL   Present   Sensitive   Process
NODE_ENV       Present   Normal      Process
API_KEY        Missing   Sensitive   Process
```

不要默认显示 raw values。

---

# 230. PATH section

显示：

```text
PATH entries
```

以及：

```text
duplicates
missing dirs
shadowing
```

默认折叠。

---

# 231. Config sources

Environment page 可以显示：

```text
Detected configuration

package.json
.nvmrc
pyproject.toml
global.json
rust-toolchain.toml
.env
compose.yaml
```

并显示：

```text
purpose
```

---

# 232. Requirement conflict UI

例如：

```text
Node requirement conflict

package.json:
>=22

README:
20

.nvmrc:
18
```

这是非常有价值的：

> “环境为什么看起来不一致？”

---

# 233. Evidence drill-down

每个事实点击：

```text
Source
Evidence
Probe
Timestamp
```

---

# 234. Environment report

JSON：

```json
{
  "os": {...},
  "architecture": "x86_64",
  "tools": [...],
  "requirements": [...],
  "matches": [...],
  "ports": [...],
  "processes": [...],
  "findings": [...]
}
```

---

# 235. Markdown report

例如：

```markdown
# Environment Report

## Host

- OS: Windows
- Architecture: x86_64

## Toolchain

- Node.js: 22.14.0
- npm: 11.3.0
- Rust: 1.90.0

## Requirements

| Requirement | Actual | Result |
|---|---|---|
| Node >= 22 | 22.14.0 | Satisfied |
| pnpm 10.x | Not found | Unsatisfied |

## Findings

...
```

---

# 236. HTML report

复用 M7/M8 report system。

不要：

> 又造一套独立 HTML framework。

---

# 237. CLI

至少：

```bash
prism env <path>
prism env <path> --json
prism env <path> --refresh
prism env <path> --requirements
prism env <path> --tools
prism env <path> --ports
prism env <path> --processes
```

实际命令数量按现有 CLI 设计控制。

---

# 238. `prism env checks`

可考虑：

```bash
prism env requirements <path>
```

专门：

> 只输出 requirement matching。

---

# 239. JSON machine use

必须方便：

```text
AI agent
CI script
shell
future MCP
```

消费。

但 M9 本身：

> 不实现 MCP。

---

# 249. Privacy-redacted report

报告中应处理：

```text
username
home path
hostname
tokens
secrets
command args
private paths
```

具体哪些字段显示：

> 遵循统一 privacy policy。

---

# 250. Private path shortening

例如：

```text
C:\Users\Gray\Projects\Foo
```

可以显示：

```text
<HOME>\Projects\Foo
```

具体是否启用按现有 privacy conventions。

---

# 256. Environment health conclusion

不要：

```text
Environment score 83
```

而：

```text
Requirements:
8 satisfied
2 unsatisfied
1 unknown
1 conflict
```

---

# 257. Requirement coverage

可以：

```text
Requirements discovered: 12
Evaluated: 10
Unknown: 2
```

不要：

```text
Environment correctness = 83%
```

---

# 259. Environment diff preparation

设计：

```rust
EnvironmentDelta
```

未来支持：

```text
ToolAdded
ToolRemoved
VersionChanged
RequirementChanged
PortOccupied
PortReleased
VariableAdded
VariableRemoved
PathChanged
```

M9 可以先预留 model。

---

# 262. Optional report export

用户可以：

```text
Save report
```

这是：

> explicit user action。

---

# 263. Auto-save prohibited

不要每次扫描：

```text
write .prism/environment.json
```

除非未来明确设计 persistence。

---

# 271. UI tabs

Environment 页面建议：

```text
Overview
Tools
Requirements
Variables
Ports
Processes
Config
```

不一定全部独立 tab。

第一版可折叠 sections。

---

# 272. Search/filter

至少：

```text
tool search
requirement filter
finding filter
port filter
process filter
```

---

# 273. Status filters

统一：

```text
Satisfied
Unsatisfied
Unknown
Conflict
Unavailable
```

---

# 274. Environment refresh

按钮：

```text
Refresh
```

执行：

> 当前 environment discovery。

不：

> 重扫项目源码。

除非 requirement source 可能变化。

---

# 275. Project config change

如果：

```text
package.json
```

发生变化：

之前 environment requirement：

```text
Node >=22
```

可能过期。

M9 应在下一次：

```text
refresh
```

重新解析。

---

# 276. Current state indicator

GUI：

```text
Environment observed:
23:41:12
```

明确：

> observation timestamp。

---

# 277. Dynamic process warning

Processes/ports：

> 动态状态。

UI 必须显示：

```text
Observed 3s ago
```

而不是：

```text
Current forever
```

---

# 278. Toolchain section refresh granularity

工具版本：

> 相对静态。

Process/ports：

> 动态。

所以可以：

```text
Refresh environment
```

或：

```text
Refresh runtime/toolchain
Refresh processes/ports
```

第一版可统一按钮。

---

# 279. Host info section

显示：

```text
OS
Architecture
CPU
Memory
Disk
Shell
```

但：

> 简洁。

---

# 280. Resource facts

建议模型：

```rust
HostResources {
    cpu_logical_count,
    memory_total,
    disk_available,
}
```

全部：

> optional。

---

# 281. Resource unknown

例如：

```text
disk unavailable
```

输出：

```text
Unknown
```

不要：

```text
0 bytes
```

---

# 298. GUI / CLI parity

核心数据：

> GUI 和 CLI 必须来自同一个 Rust environment service。

不能：

```text
GUI uses JS implementation
CLI uses Rust implementation
```

造成结果不一致。

---

# 299. No duplicated detection

必须：

```text
Frontend
→ Tauri IPC
→ EnvironmentService
```

而不是：

```text
React
→ browser APIs
→ fake environment
```

---

# 300. Browser limitation

React frontend 不应该直接相信：

```text
navigator
```

来代表真实开发环境。

Rust side：

> authoritative source。

---

# 301. IPC

增加：

```text
get_environment
refresh_environment
get_environment_requirements
get_environment_findings
```

根据现有 IPC convention。

---

# 302. IPC progress

长时间 scan：

```text
task_id
progress
status
```

复用 M0 Task。

---

# 303. IPC cancellation

复用：

```text
CancellationToken
TaskId
```

不要另造 cancellation model。

---

# 304. CLI reuse

CLI：

```text
prism env
```

直接调用：

```text
EnvironmentService
```

---

# 322. Environment report distinction

必须把：

```text
Detected
Required
Matched
Finding
```

分开。

---

# 323. Example complete result

```text
Project:
ifuyo-prism

Environment:
Windows x64

Required:
Node >=22
pnpm 10.x
Rust >=1.85

Detected:
Node 22.14.0
pnpm 10.17.1
Rust 1.90.0

Matches:
3/3 satisfied

Processes:
Vite on 5173
Tauri dev process running

Ports:
5173 occupied by project process
8000 free

Findings:
No required environment mismatch
```

---

# 324. Important distinction

```text
3/3 satisfied
```

不是：

```text
Environment score = 100
```

---

# 325. Another complete result

```text
Required:
Python >=3.12
Docker available

Detected:
Python 3.11
Docker unavailable

Matches:
Python = Unsatisfied
Docker = Unsatisfied

Environment status:
Requirements not satisfied
```

---

# 326. Unknown example

```text
Required:
CUDA >=12

Detected:
Unknown

Environment status:
Partial

Reason:
CUDA detection is not available on this platform.
```

---

# 327. Conflict example

```text
package.json:
Node >=22

.nvmrc:
20

Environment:
Node 20
```

输出：

```text
Requirement conflict detected.

Node environment cannot be interpreted as a single unambiguous requirement.
```

不要偷偷选择。

---

# 328. Process/port example

```text
Required:
Port 5173 available

Observed:
LISTENING
PID 1234
node

Result:
Unsatisfied
```

只有当：

> 5173 是“must be available”的明确 requirement。

---

# 329. If project is already running

如果：

```text
project dev server already listening
```

不要把：

```text
port occupied
```

默认判成问题。

如果：

```text
process associated with same project
```

可以：

```text
Informational
```

---

# 330. Project association evidence

例如：

```text
PID 1234
cwd = project root
command = vite --host 0.0.0.0 --port 5173
```

于是：

```text
project association = HighConfidence
```

---

# 331. Environment snapshot display

可以：

```text
Captured:
2026-10-02 23:50
```

并显示：

```text
Scan duration:
1.7s
```

---

# 332. Scan performance target

普通项目：

> Environment initial scan 应保持在用户可接受的秒级范围。

具体 benchmark：

> 用真实数据记录，不拍脑袋设绝对数值。

---

# 333. Performance testing

至少记录：

```text
small project
medium project
large project
many processes
many PATH entries
many tools
```

---

# 335. Requirement parsing cache

当前 session 可以：

> 复用 M1/M2 facts。

不要建立另一个完整 filesystem cache。

---

# 336. Environment facts cache

session cache：

```text
tool version
```

可以。

dynamic facts：

```text
process
ports
```

默认 refresh。

---

# 341. Report redaction tests

任何：

```text
JSON
Markdown
HTML
logs
```

都必须：

> 通过 secret fixture 测试。

---

# 342. HTML security

不要把：

```text
environment variable values
```

直接注入 HTML。

避免：

> HTML injection。

---

# 344. Process commandline sanitization

例如：

```text
node --password=secret
```

UI：

```text
node --password=[REDACTED]
```

---

# 345. Path sanitization

对：

```text
private home path
```

统一：

> privacy renderer。

不要到处自己 replace。

---

# 347. Environment serialization privacy

重要：

> 原始 EnvironmentSnapshot 内存模型可以比报告模型更完整。

但：

```text
to_json()
to_markdown()
to_html()
```

必须经过：

```text
privacy/redaction layer
```

---

# 350. Environment findings severity

建议使用现有：

```text
Info
Warning
Error
```

或者复用 M7 severity。

不要：

```text
EnvironmentSeverity
HealthSeverity
VerificationSeverity
```

三套 enum。

---

# 351. Health integration

M7 可以读取：

```text
Environment Findings
```

但：

> 必须保留 source = environment。

例如：

```text
ENV001
Category = Environment
Source = M9
Evidence = ...
```

---

# 352. M9 should not score

再次强调：

```text
No score
No grade
No overall health ranking
```

---

# 353. Environment compliance

可以称：

```text
Requirement matching
```

不要：

```text
Environment compliance score
```

---

# 354. Environment status

最终建议：

```text
Satisfied
Attention
Unknown
Conflict
Partial
```

如果需要 overall status。

但它必须：

> 从具体 matches 聚合而来。

---

# 355. Aggregation rule

例如：

```text
Required:
5

Satisfied:
5
Unknown:
0
Conflict:
0
Unsatisfied:
0

Overall:
Satisfied
```

---

# 356. Aggregation #2

```text
Satisfied:
4
Unsatisfied:
1
```

总体：

```text
Unsatisfied
```

---

# 357. Aggregation #3

```text
Satisfied:
4
Unknown:
1
```

总体：

```text
Partial / Unknown
```

具体 enum 与 M8 convention 对齐。

---

# 358. Aggregation #4

```text
Satisfied:
4
Conflict:
1
```

总体：

```text
Conflict
```

---

# 359. No hidden weighting

不要：

```text
OS mismatch = 20 points
Node mismatch = 10 points
```

然后：

> 算一个神秘分数。

---

# 360. Testing final golden path

最终：

```text
Open Project
    ↓
Environment
    ↓
Detect OS
    ↓
Detect Toolchains
    ↓
Detect Requirements
    ↓
Detect Variables
    ↓
Detect Relevant Processes
    ↓
Detect Relevant Ports
    ↓
Match Requirements
    ↓
Findings
    ↓
Report
```

---

# 361. Golden path #2

```text
Project:
Node

Requirements:
Node >=22
pnpm 10

Environment:
Node 22
pnpm 10

Result:
All required requirements satisfied
```

---

# 362. Golden path #3

```text
Node >=22

Actual:
Node 20

Result:
Unsatisfied
```

---

# 363. Golden path #4

```text
Node >=22

Actual:
Node executable found
version probe failed

Result:
Unknown
```

---

# 364. Golden path #5

```text
README:
Node 20

package.json:
Node >=22

Result:
Requirement conflict
```

---

# 365. Golden path #6

```text
Required env:
DATABASE_URL

Actual:
absent

Result:
Unsatisfied
```

---

# 366. Golden path #7

```text
Required env:
API_KEY

Actual:
present

Report:
present
sensitive=true

Raw value:
never shown
```

---

# 367. Golden path #8

```text
Project:
expects port 5173

Observed:
5173 occupied

Process:
same project

Result:
Informational
```

---

# 368. Golden path #9

```text
Project:
expects port 5173

Observed:
5173 occupied

Process:
unknown foreign process

Result:
Potential conflict
```

---

# 369. Golden path #10

```text
Port ownership unavailable

Result:
Unknown
```

而不是：

```text
No process found
```

---

# 370. Golden path #11

```text
Docker project
Docker executable missing

Result:
Docker unavailable
```

不执行：

```text
docker install
```

---

# 371. Golden path #12

```text
Python project
.venv exists
```

输出：

```text
Project-local Python environment detected
```

不：

```text
activate environment
```

---

# 372. Golden path #13

```text
Two Node versions in PATH
```

输出：

```text
Multiple candidates
Active:
...
```

---

# 373. Golden path #14

```text
PATH contains invalid directory
```

输出：

```text
PATH finding
```

不修改 PATH。

---

# 374. Golden path #15

```text
Process inspection permission denied
```

输出：

```text
Process discovery:
Partial
```

环境 scan：

> 仍然完成其他部分。

---

# 375. Golden path #16

```text
Tool probe timeout
```

输出：

```text
Node:
Found
Version:
Unknown
Probe:
TimedOut
```

不阻塞整个 environment scan。

---

# 376. Golden path #17

```text
prism env .
```

CLI 与 GUI：

> 结果来自同一个 EnvironmentService。

---

# 377. Golden path #18

```text
prism env . --json
```

stdout：

> 只有 JSON。

stderr：

> logs。

---

# 378. Golden path #19

```text
prism env . --refresh
```

必须：

> 重新 probe。

---

# 379. Golden path #20

```text
M8 verify
```

读取 M9：

```text
node availability
node version
pnpm availability
```

不要重复执行全部 discovery。

---

# 380. M8/M9 combined flow

最终：

```text
M9:
Node 20

M8:
Node >=22 required

M8 result:
Prerequisite Unsatisfied
Check:
Blocked / Unavailable
```

这是非常关键的集成测试。

---

# 381. M7/M9 combined flow

最终：

```text
M9:
Node version mismatch

M7:
Environment finding
severity = Error
```

前提：

> M7 rules 明确配置为接收此类 finding。

不是 M9 自己决定 Health severity。

---

# 382. Environment context for M10

以后：

```text
M10 Context
```

可以选择：

```text
Node 22
Python 3.13
pnpm 10
Windows x64
```

而不用上传：

```text
full PATH
full env
full process list
```

---

# 383. Context-safe export

提供：

```text
EnvironmentContextView
```

只输出：

> 对 AI agent 有必要且经过 privacy filtering 的环境事实。

但 M10 才真正使用。

---

# 390. Architecture final check

确认：

```text
M9 owns:
Environment Facts
Environment Requirements
Environment Matching
Environment Discovery
```

不拥有：

```text
Git
Source analysis
Health policy
Verification execution
AI context
Snapshot history
```

---

# 396. Stable report ordering

同一环境两次 report：

> 除了动态字段，不应随机排序。

---

# 397. Unit tests

至少覆盖：

```text
models
version comparison
requirement parser
matching
redaction
path normalization
tool discovery
process normalization
port normalization
platform abstraction
```

---

# 398. Integration tests

至少：

```text
Windows environment integration where available
Linux/macOS integration where available
real node
real python
real rust
real dotnet
real git
```

根据当前运行环境执行。

---

# 399. Security tests

测试：

```text
secret env
commandline secret
.env
private path
Git credential config
ANSI terminal injection
HTML injection
```

---

# 400. Performance tests

测试：

```text
1000 PATH entries
1000 processes
hundreds of listening sockets
many project requirements
many tool probes
```

确认：

> bounded execution / memory。

---

# 401. Large environment tests

机器可能：

```text
PATH very large
env very large
many processes
many ports
```

必须：

> graceful degradation。

---

# 402. Real self-audit

最后在 Prism 自己执行：

```bash
prism env .
```

检查：

```text
OS
Rust
Cargo
Node
npm/pnpm
Git
Tauri environment
frontend toolchain
```

---

# 403. Self environment correctness

人工抽查：

```bash
rustc --version
cargo --version
node --version
git --version
```

与 Prism 输出一致。

---

# 404. Self environment privacy

确认：

> Prism 自己生成的 JSON/HTML/Markdown 报告里没有：

```text
API token
private key
password
credential
```

---

# 405. Full product integration test

完成 M9 后：

```text
Overview
   ↓
Understand
   ↓
Changes
   ↓
Impact
   ↓
Health
   ↓
Verify
   ↓
Environment
```

所有页面：

> 使用统一 ProjectId。

---

# 406. Unified project context

一个打开的 project：

```text
ProjectId
```

可以关联：

```text
Discovery
CodeModel
Graph
GitRepo
ChangeSet
Impact
Health
Verification
Environment
```

这一步非常重要：

> 不允许每个模块自己重新定义“当前项目”。

---

# 407. Environment and project identity

EnvironmentSnapshot：

```text
project_id
```

但：

> environment 本身仍可独立观察。

---

# 408. Multiple projects

用户同时打开：

```text
Project A
Project B
```

环境数据：

> 必须根据 active project requirements 分离。

Host facts：

> 可以共享。

Project requirements：

> 不可以混淆。

---

# 409. Shared host facts

例如：

```text
Windows
x64
Node 22
Rust 1.90
```

两个项目：

> 可以引用同一环境事实语义。

但第一版：

> 不需要做跨项目 environment database。

---

# 410. Environment context boundary

如果 Project A：

```text
Python
```

而 Project B：

```text
Node
```

A 的 environment page：

> 不应该显示几十个无关 Go/Java/CUDA 工具作为主内容。

相关性排序必须存在。

---

# 411. Relevant tool heuristic

排序优先：

```text
Project explicitly requires
Project ecosystem
M8 required
CI configured
Known development tool
Generic host tool
```

---

# 412. Avoid tool noise

不要：

```text
Installed tools:
127 entries
```

作为首页。

Environment 页面首先回答：

> “这个项目真正需要什么？”

---

# 413. Tool details

用户点击：

```text
Node.js
```

显示：

```text
version
path
scope
source
requirement
match
probe evidence
```

---

# 414. Requirement details

用户点击：

```text
Node >=22
```

显示：

```text
source
location
actual
comparison
result
```

---

# 415. Process details

点击：

```text
PID 1234
```

显示：

```text
name
pid
path if safe
project relation
ports
observed_at
```

命令行：

> redacted。

---

# 416. Port details

点击：

```text
5173
```

显示：

```text
address
protocol
state
pid
process
project relation
observed_at
```

---

# 417. Requirement-to-process relation

例如：

```text
Port 5173 requirement
    ↓
Process PID 1234
```

这个关系可以帮助：

> “谁占用了这个端口？”

---

# 418. Requirement-to-tool relation

例如：

```text
Node >=22
    ↓
node.exe
    ↓
22.14.0
```

这是 M9 最核心的 UI explanatory path。

---

# 419. Requirement-to-config relation

例如：

```text
Node >=22
    ↓
package.json: engines.node
```

---

# 420. Complete explanation path

最终用户点击：

```text
Python requirement mismatch
```

看到：

```text
Requirement:
Python >=3.12

Declared by:
pyproject.toml: requires-python

Detected:
C:\project\.venv\Scripts\python.exe

Actual:
Python 3.11.9

Result:
Unsatisfied
```

这就是：

> Prism 的 evidence-first UX。

---

# 421. Final Audit

完成后输出：

# Prism M9 Final Environment Audit

## A. Baseline

真实：

```text
M0–M8
```

检查：

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

风险：

```text
P0
P1
```

---

## B. Architecture

确认：

```text
Collectors
→ Facts
→ Requirements
→ Matcher
→ Findings
→ Reports
```

无循环依赖。

---

## C. Host

确认：

```text
OS
Architecture
Shell
Resources
```

---

## D. Toolchain

确认：

```text
Runtime
SDK
Compiler
PackageManager
Executable
Version
Path
Source
Scope
```

---

## E. Requirements

确认：

```text
Requirement
Source
Evidence
Confidence
Operator
Expected
```

---

## F. Matching

确认：

```text
Satisfied
Unsatisfied
Unknown
NotApplicable
Conflict
```

---

## G. Environment Variables

确认：

```text
presence
scope
source
sensitivity
redaction
```

---

## H. PATH

确认：

```text
normalization
duplicate detection
missing directories
executable resolution
```

---

## I. Processes

确认：

```text
PID
process
path
project relation
observed_at
permission handling
```

---

## J. Ports

确认：

```text
port
protocol
address
state
PID
process
project relation
```

---

## K. Containers

如果实现：

```text
Docker
Compose
availability
configuration inspection
```

不得启动/停止容器。

---

## L. Privacy

确认：

```text
No secrets
No credential values
No private keys
No raw env dump
No sensitive command line leak
No telemetry
No upload
```

---

## M. CLI

真实验证：

```bash
prism env .
prism env . --json
prism env . --requirements
```

---

## N. GUI

确认：

```text
Environment
Tools
Requirements
Variables
Ports
Processes
Config
Findings
Refresh
```

---

## O. M8 Integration

确认：

```text
M8 Verify
→ M9 Environment facts
```

避免重复 probe。

---

## P. M7 Integration

确认：

```text
M9 Environment finding
→ M7 Health
```

如果已实现。

职责清晰。

---

## Q. Performance

真实记录：

```text
scan duration
tool probe count
process count
port count
memory
cancellation
```

---

## R. Tests

至少：

```text
unit
integration
platform
tool discovery
version
matching
process
port
privacy
redaction
performance
real projects
```

---

## S. Real Projects

至少：

```text
Prism
TypeScript
Python
C#
```

Rust / Docker 根据环境。

---

## T. Git

确认：

```text
generated reports
temporary files
fixtures
build artifacts
```

不会污染 repository。

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
M10 Context Engine
M11 Snapshot / Timeline
M12 UI Productization
M13 Release Hardening
```

也不要提前做：

```text
AI
RAG
MCP
database
automatic installation
environment repair
cloud environment
remote execution
system monitoring
```

---

# 422. Completion Gate

M9 只有满足以下条件才算完成：

```text
[ ] M0 regression passes
[ ] M1 regression passes
[ ] M2 regression passes
[ ] M3 regression passes
[ ] M4 regression passes
[ ] M5 regression passes
[ ] M6 regression passes
[ ] M7 regression passes
[ ] M8 regression passes

[ ] EnvironmentSnapshot exists
[ ] EnvironmentRequirement exists
[ ] EnvironmentMatch exists
[ ] EnvironmentFinding integrated with existing Finding architecture
[ ] Host discovery works
[ ] Executable discovery works
[ ] Runtime discovery works
[ ] SDK discovery works where implemented
[ ] Package manager discovery works
[ ] Version parsing works
[ ] Project requirement parsing works
[ ] Requirement provenance works
[ ] Requirement conflict handling works
[ ] Matching works
[ ] Unknown semantics work

[ ] PATH inspection works
[ ] Project-local tool detection works
[ ] Virtual environment detection works
[ ] Environment variable presence detection works
[ ] Sensitive variable classification works
[ ] Secret redaction works
[ ] .env metadata inspection works

[ ] Process discovery works where supported
[ ] Project-process association works
[ ] Port discovery works where supported
[ ] Port-process association works
[ ] Permission degradation works
[ ] Dynamic observation timestamps work

[ ] Docker detection works where implemented
[ ] No container auto-start
[ ] No service auto-start

[ ] Probe timeout works
[ ] Probe cancellation works
[ ] Bounded concurrency works
[ ] Partial scan works
[ ] Unsupported capability works

[ ] Windows paths work
[ ] Linux paths work
[ ] macOS paths work where testable
[ ] Unicode paths work
[ ] Spaces in paths work
[ ] PATH normalization works
[ ] Version comparison works

[ ] GUI works
[ ] CLI works
[ ] JSON works
[ ] Markdown works
[ ] HTML works where supported

[ ] M8 integration works
[ ] M7 integration does not duplicate ownership
[ ] M1 discovery is reused
[ ] M4 Git data is reused

[ ] No automatic installation
[ ] No automatic version switching
[ ] No PATH modification
[ ] No process termination
[ ] No service start/stop
[ ] No dependency installation
[ ] No Git mutation
[ ] No source modification
[ ] No network scanning
[ ] No telemetry
[ ] No source upload
[ ] No AI
[ ] No RAG
[ ] No MCP
[ ] No database introduced for M9

[ ] No P0
[ ] No unresolved blocking P1
```

---

# 423. 最终 Commit

全部通过后：

```bash
git add -A
git commit -m "feat: add environment engine"
```

**不要 push。**

---

# 424. M0–M9 完整语义链

M9 完成以后，Prism 的核心工程认知链变为：

```text
M1  它是什么？
 ↓
M2  它由什么代码构成？
 ↓
M3  它们怎么关联？
 ↓
M4  Git 现在是什么状态？
 ↓
M5  发生了什么变化？
 ↓
M6  可能影响哪里？
 ↓
M7  有哪些值得注意的问题？
 ↓
M8  实际验证结果是什么？
 ↓
M9  它所处的工程环境到底是什么？
```

换成用户真正会问的问题：

```text
项目是什么？
        ↓
代码是什么？
        ↓
关系是什么？
        ↓
改了什么？
        ↓
会影响什么？
        ↓
有什么风险？
        ↓
实际跑起来怎么样？
        ↓
为什么我这里能/不能跑？
```

M9 正好补上最后一个非常重要的缺口：

```text
Code
+
Change
+
Impact
+
Health
+
Verification
+
Environment
```

因此到 M9 为止，Prism 已经不是单纯的：

> “代码分析器”。

而开始形成一个完整的：

> **Local Engineering Intelligence Workbench**

---

# 425. M0–M9 每个模块最终只回答一个核心问题

```text
M0 Foundation
→ Prism 怎么稳定运行？

M1 Discovery
→ 项目里有什么？

M2 Understanding
→ 代码是什么？

M3 Architecture
→ 它们怎么连接？

M4 Git
→ Git 记录了什么？

M5 ChangeSet
→ 到底改了什么？

M6 Impact
→ 可能影响什么？

M7 Health
→ 哪些地方值得注意？

M8 Verify
→ 实际验证结果是什么？

M9 Environment
→ 当前工程环境是什么？
```

不要让模块之间重新互相吞并职责。

---

# 426. M9 完成后的核心闭环

```text
                 ┌──────────────┐
                 │   Project    │
                 └──────┬───────┘
                        ↓
              ┌──────────────────┐
              │   Understand     │
              └────────┬─────────┘
                       ↓
              ┌──────────────────┐
              │     Changes      │
              └────────┬─────────┘
                       ↓
              ┌──────────────────┐
              │      Impact      │
              └────────┬─────────┘
                       ↓
              ┌──────────────────┐
              │      Health      │
              └────────┬─────────┘
                       ↓
              ┌──────────────────┐
              │      Verify      │
              └────────┬─────────┘
                       ↓
              ┌──────────────────┐
              │   Environment    │
              └──────────────────┘
```

其中：

```text
M1–M4
= Facts

M5–M6
= Change / Impact Facts

M7
= Static Engineering Findings

M8
= Real Execution Facts

M9
= Environment Facts
```

这样整个 Prism 的数据层会非常干净。

---

# 427. Prism M9 的最终设计原则

请始终坚持以下判断顺序：

```text
先发现事实
    ↓
再发现项目要求
    ↓
再做要求/环境匹配
    ↓
再报告冲突
    ↓
最后才允许上层模块进行解释
```

而不要反过来：

```text
先猜项目需要什么
    ↓
再扫描机器
    ↓
强行找证据
```

尤其遵守：

```text
Unknown
≠
Missing

Missing
≠
Broken

Broken
≠
Health Failure

Health Failure
≠
Verification Failure
```

---

# 428. M9 最终产品定义

M9 完成以后，用户打开 Prism 的 Environment 页面，应该能够清楚回答：

```text
我现在是什么 OS / 架构？

这个项目需要什么 Runtime / SDK / Tool / Package Manager？

这些东西现在有没有？

版本对不对？

项目实际会使用哪个 executable？

有没有多个版本互相遮蔽？

PATH 有没有明显问题？

项目需要哪些 environment variables？

哪些存在，哪些缺失？

哪些变量可能敏感？

当前有哪些与这个项目相关的进程？

哪些端口正在使用？

谁在占用它？

项目声明的环境要求与当前机器有没有冲突？

哪些信息是确定的？

哪些只是 Unknown？

这些结论的证据在哪里？
```

而不是：

```text
Environment Score: 82
```

或：

```text
Environment Looks Healthy
```

---

# 429. M9 与整个 Prism 的最终关系

M9 完成后：

```text
M5 ChangeSet
        ↓
M6 Impact
        ↓
M7 Health
        ↓
M8 Verify
        ↓
M9 Environment
```

已经把：

```text
What changed?
What may be affected?
What looks wrong?
What actually happened?
What environment caused/limits execution?
```

这几个工程师最常面对的问题全部建立了**可验证的数据基础**。

下一阶段 M10 才开始真正进入 Prism 最关键的产品差异化层：

```text
Facts
  +
Changes
  +
Impact
  +
Health
  +
Verify
  +
Environment
      ↓
Context Engine
      ↓
AI / Agent
```

也就是说：

> **M10 之前，Prism 的核心数据地基应该尽量保持 Deterministic；M10 才是把这些已经被 Prism 理解过的事实，压缩、筛选、组织成真正适合 AI/Agent 使用的项目上下文。**