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

# 334. No full project rescan

`prism env`：

> 不应该重新遍历整个项目文件系统，除非 requirement detection 明确需要。

复用 M1 discovery。

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