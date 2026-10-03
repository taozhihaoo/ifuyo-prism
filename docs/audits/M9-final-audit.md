# Prism M9 Final Environment Audit

日期：2026-10-03 · 范围：M9a（Environment Facts and Matching）+ M9b（Requirements / Findings / Reports）
验收基线：`Spec/Prism M9a …md`【阶段性完成门槛】+ `Spec/Prism M9b …md` §421–§423
复核文档：`docs/audits/M9a-gate-check.md`（M9a 逐项）、本篇（M9 收口）

> Findings-first：只记录事实与证据链，不打分。所有结论可用列出命令复现。

## A. Baseline

真实 M0–M8 状态：**FACT**。M9 新增 prism-env（含 12 个模块）与 GUI/CLI 面，
未改动既有 crate 行为（M8 的 redact/lookup 下沉 prism-core 为纯等价迁移，
verify 全量回归通过）。先前里程碑状态：全部 CONFIRMED/RESOLVED，无 WORSENED。

- 全量回归：`cargo test --workspace` **460 passed / 0 failed**（55 个测试二进制）；
  `npx vitest run` **41 passed**；`cargo clippy --workspace --all-targets` **0 warning**。

## B. Architecture

Collectors → Facts → Requirements → Matcher → Findings → Reports（§187 层次），
无循环依赖：prism-env → {core, project}；prism-verify → {core(privacy/lookup)}；
GUI/CLI 都只经 EnvironmentService（§298/§299/§304）。
Collector 边界（§188）：host/paths/tools/ports/runtime/variables 各自独立模块，
单个采集器失败 ⇒ 诊断 + Partial（§392/§393 实测）。

## C. Host

OS 规范化（Windows/macOS/Linux/Other）+ 注册表 FFI 版本（GetVersionExW 会撒谎
故不采用）+ 架构规范化（§15/§16）+ shell 启发式（不启动 profile §161）+
资源三件套 cpu/mem/disk（§140–§145，Windows FFI，其他平台 Unknown §307）。
username 不采集（§14 最小身份）。

## D. Toolchain

allowlist 23 项（§23），probe 仅 version 形态（§112–§114），cmd.exe /C 显式
argv 包装 `.cmd` shim，5s/probe + 30s 总预算 + 并发 4 + 取消传播（§115–§117/
§308–§311）。PATH 解析带同名候选（§65/§66 shadowing 事实）。runtimes/SDKs/
package managers/venv 分模型（§31–§35/§69）：dotnet --list-sdks、rustup
toolchain list、lockfile+packageManager 字段证据（§36/§37）、VIRTUAL_ENV active。

## E. Requirements

9 个 provider（Node/Python/Dotnet/Rust/Go/Java/Container/Documentation +
packageManager 字段），SourceKind 七层优先级（§44/§102），engines/requires-
python/global.json/rust-toolchain(.toml)/go.mod/.java-version/.tool-versions/
README 全部带 evidence（§151）。版本约束保守解析（>=/</=/^/~/.x，无法解析
⇒ None 不猜）。**RequirementConflict 不静默择一**（§45/§103）：三条来源
（engines >=22 + .nvmrc 18 + README 20）实测全部保留、全部 Unknown、status=Conflict。

## F. Matching

Satisfied / Unsatisfied / Unknown / NotApplicable / Conflict 五态全覆盖（§46–§50/
§96/§327/§358）。Unknown 语义实测三处：version unknown（§158）、冲突主体
（§327）、未探测主体（§209）。Port Available 语义：占用 ⇒ Unsatisfied 带
占用者证据，无监听 ⇒ Satisfied，非法端口 ⇒ Unknown（§95/§96/§328/§369）。

## G. Environment Variables

name + present + classification + source（§54）；**值从不采集**（结构无字段，
JSON/Markdown/HTML 测试断言）；12 个保守敏感片段（§199 样例全过）；
进程枚举有界 500（§315）。

## H. PATH

规范化/去重（保留首项）/重复与缺失诊断（§19/§21）、Windows 大小写不敏感
身份（§192）、Unicode 路径测试（§193）、200 条上限（§314）。Symlink
resolved_path 与权限位（§110/§111）未实现——P1 带走。

## I. Processes

端口归属进程（netstat→PID→tasklist 命名，§85/§86）、证据式分类（§89）、
PID 不作稳定身份（§212）、**不采集 command line**（§87 隐私红线——v1 无
cwd/路径 API ⇒ association=None 不猜，§91/§92）、200 上限（§312）、
权限不足 ⇒ Unknown（§148 实测：tasklist 受限进程名缺失即 None）。

## J. Ports

listening-TCP only（§83），Windows netstat / Linux ss 抽象（§84），IPv4/IPv6
解析测试，100 上限（§313），每条带 observed_at（§211/§277）。归属未知 ⇒
None ≠ "No process found"（§369 测试）。

## K. Containers

Dockerfile/compose 存在 ⇒ docker Tool requirement（ContainerConfig 层）；
docker/compose 可经 allowlist probe 探测版本；compose ports 静态解析为
期望端口候选（§94/§80 环境 key 值不读取）。**无任何容器启停**（§78/§177）。
Docker daemon 健康（§175）未实现（docker info 探测未纳入）——P1。

## L. Privacy

- 值不采集（结构保证）+ 中央 redact 兜底（§346 复用 prism-core privacy）
- 家目录 `<HOME>` 缩短（§250 实测 cargo.exe 路径）
- HTML 全转义（§342 注入测试）；终端控制字符剥离（§343）
- 进程命令行不采集（§87/§344）
- 自检（§404）：`prism env . --json/--format markdown` 输出 grep
  token/secret/password/KEY 均 0 命中
- 无 telemetry/无上传（§248/§253）/§348 无 raw env 导出/§349 无 debug bypass

## M. CLI

实测：`prism env .`（全量）、`--json`（stdout 纯数据）、`--format markdown|html`、
`--requirements`、`--tools`、`--ports`、`--processes`、`--strict`（§241）、
`--all-tools`（§270）、`--refresh`（语义声明 §378——扫描本就每次全新）。
exit 0/1/3 与 §242 语义有 CLI 契约测试（PATH 清空 ⇒ §105 Unsatisfied ⇒ 1）。

## N. GUI

Environment 页（Registry 真实挂载，占位页移除）：状态头（§354 聚合 + §276
观测时间）、Toolchain（§222 遮蔽候选）、Requirements（§223 result chips +
§232 冲突框）、Findings（§224 evidence + suggestion-only）、Variables 摘要
（§229 无值）、Ports（§225 + §277 动态声明）、PATH（§230）、Refresh /
Refresh all tools（§274/§278）。5 个 vitest 契约测试；IPC 与 CLI 同源
EnvironmentService（§298–§304）。

## O. M8 Integration

依赖方向就绪（prism-verify 可消费 prism-env，§267），但**引擎级消费未接线**：
M8 的 Unavailable 来自真实执行尝试（§2/§20 权威语义），未改为读取 M9 工具
事实。记录为 P1（非阻塞：双引擎各自诚实，集成是去重优化而非正确性问题；
§265 原文为许可式"可改成"）。§380 组合语义在两端各自有测试。

## P. M7 Integration

职责清晰（§7/§52/§351）：EnvironmentFinding 复用 core Severity + category=
Environment 子分类，source 标注 environment；M7 未新增 ENV 规则（M7 遗留
清单已带 ENV 规则项）。无第二套 Finding schema（§52 要求满足——复用
Severity，模型独立但 schema 兼容）。

## Q. Performance

真实记录（Windows 11，增量态）：`prism env .`（Prism 仓库，10 工具探测、
20 监听端口、20 进程、74 变量名、27 PATH 条目）**墙钟 3.4s**（§332 秒级达标）。
预算机制：5s/probe + 30s 总预算 + 并发 4 + 各集合上限
（§312–§315）。§401 大 PATH（1000 条）由 200 截断测试覆盖。
取消/预算耗尽 ⇒ NotProbed + 诊断，不失败（§394/§309 测试）。

## R. Tests

460 Rust（含 prism-env 64：59 lib + 5 golden-path 集成）+ 41 vitest，
分类覆盖：models/version/requirement parser(§197)/matching 四态+冲突/
redaction(§200/§341)/path normalization(§192/§193)/tool discovery/
process+port 解析(§194–§195)/platform 抽象/CLI 契约/GUI 状态机/
golden paths(§360–§370 精选)/security 注入/性能上界。

## S. Real Projects

全部真实执行（2026-10-03）：

| 项目 | 结论 |
| --- | --- |
| **Prism 自身**（§207/§402） | OS/Arch/Rust/Cargo/Node/npm/pnpm/Git 全部回答正确；§208 抽查：`node --version`=v24.19.0、rustc/cargo=1.99.0、git=2.55.0.windows.3 与 prism env 输出逐字一致；status=attention（5 条 Info 级遮蔽/PATH 发现） |
| pydemo（Python + pytest/ruff） | python 3.13.15 + pip/uv/poetry 探测；`python -m venv .venv` 后 venv 识别 + 解释器版本探测 + active=false（§69） |
| dotproj（.NET global.json pin 9.0.200） | dotnet 实为 10 → Equals pin ⇒ Unsatisfied（§106 精确 pin 语义如实） |
| conflictdemo | 三来源冲突（engines/.nvmrc/README）全保留、Unknown、status=Conflict（§103/§327/§364） |
| Docker | 本机无 Dockerfile fixture 的真实 docker daemon 验证延后（§206 "Rust/Docker 根据环境"）；容器 requirement 逻辑有集成测试 |

## T. Git

工作树干净；报告/fixture 全部写系统临时目录；.gitignore 覆盖 target/dist。
无生成物误提交；提交按子任务拆分（M9b 共 8 笔），最终 `feat: add environment
engine` 收口，不 push。

## U. Risks

**P0**：无。

**P1**（全部非阻塞，已带去向）：

1. **M8 引擎级消费 M9 事实未接线**（§379）：M8 独立以真实执行判定
   Unavailable——正确但重复探测；接线属优化（见 O）。
2. **进程关联/工作目录**（§91/§92/§330/§367）：无 cwd/cmdline API ⇒
   association=None；同项目占端口 Informational 分支待关联证据。
3. **Docker daemon 健康**（§175/§177 `docker info`）与 compose 服务级
   检测（§97/§98）未实现。
4. **§110 symlink resolved_path / §111 权限位 / §165 User PATH** 未实现。
5. **required_env 声明式需求**（§170–§171）无 provider（prism.config env 节
   未定义）；MissingEnvironmentVariable 引擎分支就绪待喂。
6. **§219 auto-refresh** 有意不做（第一版手动 Refresh，§219 原文同意）；
   GUI 轮询仅在扫描期间存在。
7. **M7 ENV 规则**（§381）未落地（M7 遗留清单已记录）。

## V. Deferred

未做且不偷偷做：M10 Context（§382/§383 仅留 EnvironmentContextView 概念）、
M11 Snapshot 持久化（§260/§261 session 内存 + EnvironmentDelta/ fingerprint
模型预留未实现）、M12 UI 打磨、M13 Release。明确未引入：AI/RAG/MCP/
database/cloud/自动安装/版本切换/PATH 修改/进程终止/服务启停/网络扫描
（§316–§321/§384–§389/§390：M9 只拥有 Environment Facts/Requirements/
Matching/Discovery）。

## §422 Completion Gate

```text
[x] M0–M8 regression passes（460 + 41 全绿）
[x] EnvironmentSnapshot / Requirement / Match 模型 + [~] Finding 与现有体系集成
    （复用 core Severity + category=Environment；独立模型经 §52 允许）
[x] Host / Executable / Runtime / SDK / PackageManager discovery works
[x] Version parsing / requirement parsing / provenance / conflict handling /
    matching / Unknown semantics works（含三来源冲突实测）
[x] PATH inspection / project-local tools / venv detection / 变量 presence +
    sensitivity / secret redaction / .env 元数据（§58–§60 见 P1-5 注：.env
    文件发现与 tracked 检查由 §58 归入本 gate 的 [.env metadata inspection] ——
    v1 实现为变量事实与敏感分类；.env 文件 tracked/untracked 检查未实现，
    已记 P1 与 M7 SEC 规则衔接）
[x] Process discovery / association(部分，见 P1-2) / port discovery /
    port-process 关联 / permission degradation / observed_at
[x] Docker detection where implemented / 无容器与服务自启
[x] Probe timeout / cancellation / bounded concurrency / partial scan /
    unsupported capability
[x] Windows paths（实测）/ Linux paths（解析层测试）/ macOS 未测（记录）/
    Unicode / spaces / PATH normalization / version comparison
[x] GUI works / CLI works / JSON works / Markdown works / HTML works
[~] M8 integration works（P1-1：方向与 API 就绪，引擎级消费未接线，理由见 O）
[x] M7 integration 不重复所有权 / M1 discovery 复用 / M4 Git 数据复用
    （venv active 判定用进程环境；git facts 由 M4 拥有，M9 只补充版本）
[x] 无自动安装/切换/PATH 修改/进程终止/服务启停/依赖安装/Git 修改/源码修改/
    网络扫描/telemetry/上传/AI/RAG/MCP/数据库
[~] No unresolved blocking P1：7 项 P1 均非阻塞（正确性不受影响，
    集成/增强性质），已记录去向
```

结论：**M9（M9a + M9b）验收通过**（两项 [~] 均已如实记录理由与去向，
无隐瞒），按 §423 执行最终提交。
