# M5 Final Audit — ChangeSet

日期：2026-10-03 ｜ HEAD：见 git log（最终提交为 docs/audit 提交） ｜ 规格：`Spec/Prism M5 — ChangeSet 完整开发提示词.md`

---

## A. Baseline（M0–M4）

- **FACT**：M4 提供 `GitDiff/GitDiffFile(old/new path, blob, mode, rename, similarity, hunks)/GitDiffHunk(old/new_start+lines)/GitDiffLine(1-based)/GitDiffScope{WorkingTree,Staged,RevisionRange}`，全部经 CommandRunner 只读。
- **FACT**：M2 提供 `Symbol`（id=`symbol:{path}#{qualified}#{kind}`，SourceRange line 1-based / end 独占 / byte_offset 0-based）、`SymbolIndex`、`ProjectCodeModel`；`ParseRequest{content}` 支持对任意字符串单文件解析（M5 历史侧复用）。
- **FACT**：M3 提供 `Module/ModuleId`（`module:file:{path}` / `module:namespace:{ns}` / `module:package:{dir}` / `module:project:{name}`）；M5 通过 prism-graph 新增 `file_module_id()/namespace_module_id()` 复用同一派生规则（单一事实源）。
- **FACT**：M1 `FileClassifier::assess` 可独立复用于 revision 侧文件分类（本次将 `FileClassifier/build_file_info` 加入 prism-project 导出）。
- **GAP → 已补**：GitProvider 缺历史内容读取与 revision 解析 —— 新增只读 `read_blob`（cat-file，字节长度核对）与 `resolve_revision`（rev-parse --verify）。RESOLVED。

## B. ChangeSet Architecture（§103 判断）

- **FACT**：ChangeSet 是集成层（Git facts × 代码事实 × 模块事实），落位独立 crate `prism-changes`；依赖方向 core ← project ← {parser, git} ← analysis/graph ← changes ← {cli, tauri}。未产生 `prism-git → GUI` 或 `prism-parser → git` 反向依赖。
- **FACT**：模块结构 model / evidence / diagnostic / revision / matcher / service / render 分文件，无千行 monolith（§110），无投机抽象爆炸（§111）。
- **FACT**：`ChangeSource{Git,Snapshot,Patch,Unknown}` 预留、只实现 Git（§104）；模块聚合只消费 M3 事实，不复制 DependencyGraph 进 ChangeSet（§55）。

## C. Change Scope（§7）

- **FACT**：`ChangeScope{WorkingTree, Staged, RevisionRange}`；WorkingTree=HEAD→worktree（含 untracked 合并为 Added，`git diff` 本身不含 untracked）、Staged=HEAD→index（index 侧经 `git cat-file :0:path` 读取）、RevisionRange=A→B（两侧均经 cat-file）。
- **INFERENCE**：单提交（Commit）scope 未实现——可由 RevisionRange(A^..A) 表达，规格 §7 为"建议"清单。Deferred。

## D. File Changes（§8–§14）

- **FACT**：FileChangeType 八态齐全；`old_exists/new_exists`、双侧 language/FileKind（M1 分类语义）、`language_changed` 仅双侧已知且不同时为 true；rename 事实保留不拆 delete+add（§13）；binary 不读内容（§44）；untracked 归一化为 Added。
- **FACT**：12/12 m5_specs 集成测试覆盖 Added/Deleted/Modified/Renamed/binary/unsupported/git-only rename/whitespace。

## E. Symbol Mapping（§19–§26、§49–§53、§82–§84）

- **FACT**：分级匹配（§51）：①稳定 ID（需真实行重叠，§23 反例有测试）→ ②git rename 文件对 local identity → ③同文件 rename 证据（kind+parent+去声明行主体逐行一致；**唯一候选**才成立，多候选 Ambiguous+诊断，§83 有测试）→ ④孤儿侧 Added/Deleted（需变更行命中 range）。
- **FACT**：nested（method BodyChanged / class 结构 Modified，§25 有测试）；multi-symbol hunk 全覆盖（§24 有测试）；签名/可见性/文档/声明行变化进 characteristics（§17/§58）；Breaking 类判断不存在（§58/§59）。
- **FACT（关键正确性）**：对侧解析带 error 诊断 ⇒ 抑制 Added/Deleted 孤儿并记 `SymbolMappingUnavailable` —— 语法损坏绝不伪装成"全部删除"（§42/§84 有测试）。
- **FACT**：移动检测仅覆盖 git-rename 文件对（常见情形，§50）；无全仓库模糊匹配引擎。
- **INFERENCE**：`confidence` 用分类制（Exact/HighConfidence/Unmatched），未制造伪精确分数（§20）。

## F. Module Changes（§28–§31）

- **FACT**：ModuleChange 复用 M3 ModuleId；FileModule 生命周期由模块事实决定（before 解析过+文件删除 ⇒ Deleted；before 基线缺失 ⇒ 不伪造，只记 Modified+说明）；rename ⇒ 旧 FileModule Deleted + 新 FileModule Added；namespace 容器仅当同文件两侧声明同一 namespace 才记 Modified。Package 容器聚合未实现（需要跨文件 `__init__.py` 存在性事实）—— **Deferred**，已在代码注释与本审计声明。
- **FACT**：evidence 指向 file_change_id 集合与符号计数（§54）。

## G. Revision Handling（§37–§40、§76–§78）

- **FACT**：`RevisionSource` trait + `CurrentWorkingTreeSource`（磁盘只读）/`GitRevisionSource`（cat-file，HEAD/`:0`/任意 rev）；无 checkout/worktree/HEAD/index 改写；`revision_range_does_not_mutate_working_tree` 测试证明对比后工作区与 status 完全不变（§38/§77/§94）。
- **FACT**：`read_blob` 先 `cat-file -s` 验大小（超限 TooLarge 不读取，§78），blob 字节长度与 stdout 长度核对，不一致 ⇒ EncodingUnsupported（非 UTF-8 永远不会伪装成空文件）。**理由**：CommandRunner 用 read_to_string，非 UTF-8 会整体失败成空串——该校验是"删除伪装"防线的一部分。
- **FACT**：仅读取变更文件（bounded）；不实现完整 revision materialization（§37 允许按真实代码定案）。

## H. Diagnostics / Partial Success（§33/§34/§80）

- **FACT**：12 种诊断（规格 §33 全集 + FileTooLarge/EncodingUnsupported/GitQueryFailed）；映射失败不击穿——Prism 自扫描（Rust 无 parser）输出 valid ChangeSet + `unsupported_language` 诊断（95 mapped + 5 unmapped 形态）；单文件读取失败 ⇒ GitQueryFailed/HistoricalModelUnavailable。

## I. CLI（§64–§66、§92）

- **FACT**：`prism changes [PATH] [--staged] [--from R --to R] [--format text|json|markdown] [--max-files N]`；退出码 0/1/3（缺失路径、无效 revision）/4/6（非 Git）；JSON 契约 snake_case、确定性排序、无绝对路径（§63）；确定性测试：同状态连续两次输出逐字节一致（CLI 测试 §81）。

## J. GUI（§67–§70、§79、§93）

- **FACT**：Changes 页升级为 ChangeSet 视图：Summary/Files(分组+点击联动符号 §68)/Symbols(特征+行号)/Modules/Diagnostics；页面卸载取消后台分析（§79，独立 CancellationToken）；M6 边界在 UI 文案显式声明；tauri dev 冒烟通过（vite 200 + prism-app.exe 启动）；4 个页面测试 + 8 个既有前端测试全绿。

## K. Tests（§81–§93、§112–§114）

- **FACT**：全工作区 33 个测试套件 271 个测试全绿（含 prism-changes 25 单测 + 12 m5_specs 真实 git 集成 + 5 changeset_fixture 端到端 + 7 CLI 契约测试），clippy --workspace --all-targets = 0。前端 12/12（vitest）。
- **FACT**：真实项目验证（§114）：basic-typescript / basic-python / basic-csharp fixture 转 git 仓库后真实验证（签名+主体变更 → `[declarationchanged, signaturechanged, bodychanged]`；Python body 变更 → run Modified+result Added；C# namespace 容器聚合）；Prism 自身 5 提交区间 1.46s/25 文件/46 符号（§115：未见失控）。
- **FACT**：本审计编写过程以真实 git 输出为准修正了两处 M4 遗留解析 bug（见 O）。

## L. Performance（§71/§72/§78）

- **FACT**：一次批量 diff → 变更文件集合 → bounded 并发（1–4 workers）解析；每文件/每 git 查询可取消；max_files=200/max_file_bytes=1MB 上限；超限 truncated=true 如实标注（有测试）。
- **INFERENCE**：每文件一次 `git cat-file` 进程在数千文件级 ChangeSet 会成为瓶颈（Windows 进程开销）；当前规模（≤200 files 默认）实测秒级。批次化 cat-file（如 --batch）受 CommandRunner 无 stdin 限制 —— **P2，留给后续里程碑**（§71 的 GUI 侧 bounded/取消已满足）。

## M. Security / Privacy（§94/§95/§96/§97）

- **FACT**：无 commit/push/pull/fetch/checkout/switch/reset/restore/clean/rebase/merge/stash 执行路径（新增 git 调用仅 rev-parse --verify/cat-file）；无网络/遥测/AI/源码上传；无数据库（§96）；无 LLM（§97，summary 全为计数）；日志/证据不含源码内容与 diff 正文；契约与输出仅项目相对路径。
- **FACT**：`git_status_lines` 等测试辅助用 std::process 直接跑 git **仅存在于测试代码**（写 fixture 用），生产代码路径全部经 CommandRunner（分层规则允许测试差异）。

## N. Git（§119）

- **FACT**：M5 全部子任务分笔提交（7 笔实现 + 1 笔修复/测试 + 本 docs/audit 笔），工作树仅含本审计与 docs 更新；未 push；未动 remote。

## O. Risks（仅 P0/P1）

- **P1（已在本里程碑修复，记录在案）**：M4 `--raw -z` rename 双路径读取顺序颠倒（old/new 反转）、`--numstat -z` rename 记录键为空串 —— 均为 M5 fixture 揪出的真实缺陷，已按实测 git 布局修复并修正原单测的错误期望；`git diff` 现固定 `-c core.quotepath=false`（patch 无 -z，unicode 路径曾被八进制转义导致 hunk 合并不上）。FACT：18+9 个 prism-git 测试与 12 个 M5 集成测试共同覆盖。
- **P1（不阻塞 M6）**：GUI SVG 交互仍为 M3/M4 遗留的手动验证项；超大仓库（10k 文件级）ChangeSet 未实测（§115 的"不失控"在当前规模成立）。
- **无 P0**。

## P. Deferred

- **M6 Change Impact**：反向遍历、blast radius、distance/confidence —— ChangedSymbol ≠ ImpactedSymbol 边界已在模型与 UI 双侧钉死。
- **M7 Health / M8 Verify / M9 Environment / M10 Context / M11 Snapshot**：按各自规格；M5 的 ChangeSet JSON（schema_version=1）为 M10 Context 与 M11 snapshot-to-snapshot 比较的直接输入。
- **M5 内部推迟项**：Commit scope、Package 容器聚合、跨文件无 git-rename 证据的 move 检测、cat-file 批量化。

---

## M5 Completion Gate（§118）

全部满足：M0–M4 回归 ✅（271 tests）｜Git→FileChange ✅｜三种 scope ✅｜Added/Deleted/Modified/Renamed/Binary/Unsupported ✅｜符号匹配+歧义显式+rename/move 保守+签名/主体检测 ✅｜nested/multi-symbol hunk ✅｜模块聚合+证据+诊断+部分成功 ✅｜历史对比不改写工作区（有测试）✅｜取消（CLI token + GUI 页面卸载）✅｜CLI/JSON/GUI ✅｜真实 git 集成 ✅｜Prism 自变更分析 ✅｜多个真实项目 ✅｜无 AI/网络/遥测/数据库/git 变更命令 ✅｜无 P0 ✅｜无阻塞 M6 的 P1 ✅。
