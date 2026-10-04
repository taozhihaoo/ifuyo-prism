# PRISM_2_0_L0_ANALYSIS.md

> 2.0-M22 产物（§96/§97）。实现 = 新 crate `prism-l0`（commit 863cde3）+ CLI `prism intel --domain structural|all`（ee6a9fe）。
> 上游：M20 契约套件 + M21 `prism-intel`。本文回答 §97 的 25 问；不重复契约内容。

## 1. 什么是 L0 / 2. 什么不属于 L0

L0 = **Project Structural / Environmental Observation**：项目在哪、有什么文件/目录、git 状态、manifest 与依赖**声明**、配置存在性、平台常量。
不属于 L0：AST/symbol/方法调用/继承（M23/M25）、Scene/Node/Signal（M24）、变更影响判断（M26）、评价性结论（health，M28）。

## 3–4. Project 如何发现 / Root 如何确定

复用 M1 `Project::from_root`（canonicalize + 去 verbatim，M20 §32 口径）。M22 不重写 root 探测：用户显式传入根 + M1 忽略规则扫描。嵌套项目的 manifest 会被**观察到**（has_manifest 指向子路径实体），但**不强制**拆分项目身份（§八：只建立支持结构，repo=project 解耦留待真实需求）。

## 5. Repository / Workspace / Project 区分

Repository = 独立实体（EntityKind::repository，Virtual location 指向 gitdir）+ `hosts_project` 关系；Project = `ProjectIdentity`。Workspace 实体留待真实 workspace 需求（Cargo workspace 等被观察为 manifests + 依赖声明，不建独立实体）。

## 6–7. File Identity / Path identity

File EntityId = `file:{fnv1a16(stable_key|file|relative_path)}`——项目相对路径（`/` 分隔）为身份，绝对 canonical 路径只在 ProjectIdentity.root。display path 不参与身份。

## 8. Git state 表达

`git_repository_present`(Bool) / `git_branch` / `git_head_commit` / `git_working_tree_changed_files`(Int) / 逐文件 `git_file_status`（index/worktree/untracked/renamed_from，cap 100）。证据 = Git Evidence 引用 observed revision（§24）。非 git 项目诚实记录 `git_repository_present=false`。

## 9–11. Manifest / Dependency declaration / Configuration

Manifest = configuration 实体 + `has_manifest` 关系（§85：路径已有 file 实体时**复用**该实体，不建双实体）。声明抽取：package.json（serde JSON；dependencies=runtime / devDependencies=development）、Cargo.toml（行级扫描三段 scope；workspace 继承键去 `.workspace` 后缀展示）、requirements.txt（逐行）。`pyproject.toml` 仅记录存在（TOML 数组解析留待专门阶段）。每个声明 = `dependency_declared` Fact（Structured{name,version_constraint,scope,manifest}）+ `declares_dependency` 关系 → **External** 包实体——declared ≠ resolved（§27）。锁文件 = `lock_file_present` 事实（§77）。Configuration = 存在性 + has_configuration 关系，**不读内容**（§30）。

## 12. Environment 表达

`platform_os` / `platform_arch` 常量事实（零探测）。工具/版本/端口探测仍归 prism-env（M9）；env→Core 桥接归 M29（Matrix）。§32 禁采集用户档案信息——本层无任何进程探测。

## 13–15. Include/Exclude、GitIgnored vs PrismExcluded、Generated、Symlink、Binary

复用 M1 ScanOptions/IgnoreEngine：默认规则 + .gitignore 合并（follow_symlinks=false——§十一防递归/越界）。`files_ignored` 统计事实记录被排除量。逐文件的 GitIgnored/PrismExcluded 双维标注为已记录限制（当前 M1 引擎不区分来源；真实需求出现时扩展 FileInfo）。Generated/binary 分类 = M1 FileClassification（带 confidence+reason），以实体 metadata 传递。语言 = extension-based，`language_basis=extension` 显式标注（§20 不宣称语法分析）。

## 16. Evidence / Provenance

每个 producer 事件携带 Evidence（File/Manifest/Git kind；git 引用 observed revision；manifest 引用路径+行号若有）。Bundle.provenance = `prism-l0::structural_analysis` + INTEL_SCHEMA_VERSION + observed_at（§50 五问可答）。

## 17–19. Freshness / Incremental / Partial

所有事实 fresh at observation（§25：git 状态观察即新鲜、易 stale——跨运行 stale 判定归 snapshot 比对层）。**增量 = `delta::diff(previous, current)` 正确差分**（内容寻址 id 直比；§52 规则的验证底座）；持久增量缓存按 §51 YAGNI 暂缓（M1 扫描亚秒级，4 套互不知晓的缓存比无缓存更糟——有测量证据再建）。截断（max_entities）⇒ **Partial** + 警告，不伪装完整（§55）；取消 ⇒ Cancelled + 已完成部分（§54）。失败隔离：单 manifest 读取失败/JSON 畸形 ⇒ 诊断 + 继续；项目根不可用 ⇒ Err（§56）。

## 20. Evidence 如何产生

见 16；另 scan 非致命错误（permission 等）以 scan diagnostics 进 outcome.warnings（cap 20 + 余数提示）。

## 21. Security boundary

不跟随 symlink（递归/越界防护，M1 既有 + 测试）；manifest 大小上限（1 MiB）+ 依赖抽取上限（200/manifest）+ 实体上限（20k）+ git 状态事实上限（100）（§72 资源限额）；配置不读内容 ⇒ 无 secret 泄漏面（§30）；无 shell/网络/遥测；Unicode/空格路径有测试（§73）。

## 22–23. Cache / Snapshot 接入

Cache：见 17-19（session 内无新增缓存；M1 既有扫描即增量策略的执行点）。Snapshot：**暂不桥接**——现有 snapshot structure 域已持久化文件清单（M11 域捕获）；L0 bundle → snapshot 新域留待 compare 需求出现（§62/§63 只求数据可消费，已满足：bundle 可序列化、可 delta）。已记录为 deferred。

## 24. CLI / GUI / MCP 消费

CLI：`prism intel --domain structural|all`（默认 dependency 不变）；`prism scan`/`inspect` 旧契约零改动。GUI：无新 IPC（不回归）。MCP：不存在（N/A）；bundle JSON 即未来 read-only 暴露形态（确定性序列化满足 §66）。

## 25. M23 需要从 L0 获得什么

`IntelBundle` 中：file 实体（含 language/extension metadata + classification）→ parser 选民的输入清单；`non_analyzable_reason` 标记排除者；project/scope 身份；AnalysisContext 模式。M23 不应重扫文件系统——从 bundle（或重跑 `prism intel --domain structural` 的确定性输出）取结构底座。

## L0 Coverage Matrix（§81）

| L0 Domain | Supported | Evidence | Persistence | Cache | Incremental | Tests |
|---|---|---|---|---|---|---|
| Project | Yes | discovery + M1 扫描 | 暂无（bundle transient） | M1 既有 | delta 差分 | ✓ |
| Filesystem | Yes | File/Range Evidence | 同上（snapshot structure 域既有） | M1 既有 | delta 差分 | ✓ |
| Git | Yes（状态/HEAD/branch；无 log 深挖） | Git Evidence 引用 revision | snapshot git 域既有 | M4 既有 | delta 差分 | ✓ |
| Manifest | Yes（3 生态声明抽取 + 锁文件 + 其余存在性） | Manifest Evidence | snapshot change 域部分既有 | 无（读取即得） | delta 差分 | ✓ |
| Configuration | Partial（存在性；不读内容——secret safety） | File Evidence | 无 | 无 | delta 差分 | ✓ |
| Environment | Partial（平台常量；工具探测在 prism-env/M29 桥接） | 无探测 | 无 | 无 | n/a | ✓ |
| Workspace | 观察级（manifests + 声明；无独立实体） | Manifest Evidence | 无 | 无 | n/a | ✓ |

## 已记录限制（诚实清单）

1. 逐文件 GitIgnored/PrismExcluded 双维标注未做（统计事实有 files_ignored；§38 概念分离已落实到文档与事实命名）。
2. rename 检测依赖 git status 的 original_path（有无 git 证据不伪造，§16）。
3. 持久增量缓存未建（YAGNI，§51）。
4. pyproject.toml 依赖声明未抽取（存在性已记录）。
5. env 桥接（工具探测→Core）归 M29。
