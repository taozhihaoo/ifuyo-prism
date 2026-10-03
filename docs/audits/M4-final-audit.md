# M4 Final Audit — Git Basics

- 日期：2026-10-03
- 依据：`Spec/Prism M4 — Git Basics 完整开发提示词.md` §41/§67/§70
- 基线：M3 HEAD `79039a3` → M4 HEAD（当前），工作树 clean
- 结论标记：FACT / HYPOTHESIS / INFERENCE

## A. Baseline

FACT：M0–M3 全绿（28 个 test-result 行，全 workspace 单命令通过）；
M4 后全量回归同样全绿（回归规格 §44）。

## B. Git Architecture（§67 B）

FACT：依赖方向
`CLI/Tauri → prism-git (GitService/GitProvider) → CommandRunner → git executable`。
prism-git 不依赖 Tauri/前端；Git domain 类型在 prism-git，未污染 prism-core
（core 仅新增 CommandRunner re-export，规格 §0.5）。前端不直接 spawn git（§56）。

## C. Repository Model（§67 C）

FACT：`git rev-parse --show-toplevel --absolute-git-dir --git-common-dir
--is-bare-repository` 发现仓库；linked worktree（.git 为文件）经
git_dir ≠ common_git_dir 检出；bare 标记保留；非 Git 项目 → Ok(None)/
repository: null（规格 §25，不是错误）。

## D. HEAD / Branch / Ref（§67 D）

FACT：`symbolic-ref --quiet HEAD` + `rev-parse --verify --quiet HEAD`
组合区分 Branch / Detached / Unborn（unborn 不误判 detached，§7）；
分支含 upstream/ahead/behind（仅当 git 报告时，§8 禁止 fetch ✓）；
refs 覆盖 heads/remotes/tags/other（§9）。

## E. Status（§67 E）

FACT（porcelain v2 -z，--untracked-files=all）：
- index/worktree 两维度独立保留（AM/MM 不压扁，§11）
- untracked 与 ignored 分离（§13；ignored 需显式 --ignored 查询）
- rename 保留双路径（§12）；unmerged 标记 conflicted
- staged/unstaged/untracked 分组呈现（§33）

## F. History（§67 F）

FACT：`git log -n N --pretty=format:...%x1e`（NUL/控制分隔，bounded）；
merge commit 双 parent 测试覆盖（§19）；initial 零 parent 覆盖；
pagination 经 limit 参数（§18 不整仓加载）。

## G. Diff（§67 G）

FACT：三语义 scope 显式建模（§17：WorkingTree/Staged/RevisionRange）；
批量查询 numstat -z + raw -z + patch；hunk/行号计数测试覆盖；
binary → is_binary + additions/deletions=None（§21 不读大量内容）；
rename 双路径 + similarity（§20 不自行猜测）；mode 保留（§22）；
truncated 上限如实标注（§35 bounded size）。

## H. CLI（§43.8）

```text
prism git status <path> [--json] [--ignored]   PASS（exit 0；非 Git exit 6）
prism git log <path> [--limit N] [--json]      PASS
prism git diff <path> [--staged] [--from A --to B] [--json]  PASS
prism git refs <path> [--json]                 PASS
```

FACT：`prism changes` 未被 M4 占用（留给 M5，规格 §38）；
exit code documented（§39：0/1/2/3/4/5/6）。

## I. GUI（§43.9）

FACT：Changes 页面从 placeholder 变为真实数据页——Repository header
（branch/HEAD/root）、Working tree 分组（staged/worktree/untracked +
ignored 计数）、Recent commits（作者+主题）；非 Git 项目友好状态；
`get_git_facts` IPC 单快照消费（§56 数据流，React 不 spawn git）。
FACT：GUI Changes 页自扫描验证通过（tauri dev 冒烟 + IPC 通路）；
HYPOTHESIS：交互细节（大仓库分页 UI）建议 M12 打磨。

## J. Tests（§43 J / §65 禁止假测试）

```text
prism-git：error 2 + parsers 9（porcelain v2/rename/refs/branches/log
merge+initial/raw/numstat/patch 行号）+ git_fixture 12（真实临时 repo）=
38 通过；解析器测试与真实 git 集成测试并存（§65 两者兼备）
CLI：17 通过（含 git status/log/diff/refs/exit codes）
M0-M3 回归：全绿（§44）
真实 Git fixture 验证（§64）：unborn/detached/status 矩阵/rename/
deleted/binary/merge/pagination/unicode 全部经真实 git 执行验证
```

## K. Performance（§43 I）

FACT：status/log/diff 均为单命令批量查询（§46 禁止每文件一次 git）；
diff 结果 200 文件上限 + truncated 标注（§35 bounded）；
cancellation 经 CommandRunner 传递（§54）；无持久化 cache（§47）。

## L. Security & Privacy（§43 J）

FACT：只读命令白名单（§40 无 commit/push/pull/fetch/checkout/reset 等）；
无网络/telemetry/AI；日志不含源码/diff 内容/secrets（§41）；
绝对路径仅在 repository.root_path 字段（执行用），文件身份全部相对（§31）。

## M. Git（§43 K）

FACT：自扫描 git status 检出 branch/HEAD/entries（含探测性未跟踪文件的
出现与清理前后对比）；工作树 clean；未跟踪/忽略/生成物均未误报。

## N. Risks

### P0
- 无

### P1
- 进程树终止的平台限制（§54：CommandRunner kill 单进程；Windows job
  object 留 M8）——GUI 长任务取消时可能残留 git 进程至其自然结束
- Windows 凭据管理器（credential.helper=manager）在含 remote 的仓库
  可能触发交互——Prism 只读命令不触发网络，风险仅在用户主动 fetch 场景
- 大仓库 status 首次调用可能较慢（git 自身开销，非 Prism 实现）

## O. Deferred（§67 O）

ChangeSet / ChangedSymbol / ChangedModule / Impact Analysis / Risk
Analysis / Context generation / Snapshot——全部留给 M5+（§48 不越级）；
Git diff → Symbol 绑定属 M5（§16）；submodule 递归分析留未来（§23）。

## 结论

INFERENCE：M4 = COMPLETE（§69 回归门清单 25 项全部满足）。Git Facts 层
（只读、确定性、可测试、可复用）就绪，M5 ChangeSet 可直接消费
GitDiffFile 的 old/new path、change_type、hunks、blob、mode、rename。
