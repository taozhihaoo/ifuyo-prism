# PRISM_2_0_GIT.md — Git Intelligence & Codebase Evolution 契约（2.0-M211）

> 2.0-M211 产物。§87 的十份文档（GIT_MODEL/COMMIT_MODEL/CHANGE_EVOLUTION/
> HISTORY/BLAME/HOTSPOT/CHANGE_COUPLING/GIT_FINDING_CORRELATION）按既有惯例
> 合并为本契约 + `docs/audits/2.0-M211-final-audit.md`（Final Report）。

## 1. 定位（§0/§1/§4）

`prism-evolution` crate（第 23 个成员）+ prism-git 只读扩展 + CLI
`prism git` 子命令族。Prism 从"理解现在的代码"升级为"理解它是如何一步步
演化成现在这样的"。

- **Git 是证据来源之一，不是真理本身**（§4.1）：Git fact → Change →
  File/Symbol → Relation → Impact → Finding；churn/频率是 INFERENCE 级
  工程事实。
- **不重建模型**（§0/§12/§78）：Repository/Commit/Branch/Status/Diff 模型
  = M4 RETAIN；Symbol diff = M26 change_mapping（M23/M25 口径）；Finding
  生命周期 = M28；依赖事实 = M29。无第二 Git truth。
- **Read-only**（§42）：零写命令；全部经 M4 GitProvider 安全命令面
  （CommandRunner allowlist，无任意 shell，§41）。
- **Bounded**（§47/§52/§70）：一切历史有 `recent:<n>-commits` 窗口声明；
  max_commits/top/pairs/lines 上限；禁止默认全历史。

## 2. Git 事实供给（prism-git EXTEND）

| Provider 方法 | Git 命令 | 产物 |
| --- | --- | --- |
| `commit_changes` | `show --name-status --find-renames=50% -z` | `GitFileChange{before/after/type/similarity, lines_*}`（§13：rename 用 git 原生证据，不做文件名猜测） |
| `file_history` | `log --follow --numstat --find-renames` | `GitFileHistoryEntry{commit, changes[]}`（§15 rename 穿透；numstat `old => new`/`dir/{old => new}` 双形态解析） |
| `log_numstat` | `log --numstat --find-renames -n <max>` | 全仓库近期历史（§24/§47 bounded） |
| `blame` | `blame --porcelain -L 1,<max>` | `GitBlameRange{commit, author, lines}`（§22 区段归并；"recent contributor" 非 owner §23） |

## 3. Evolution 事实（prism-evolution）

```text
FileEvolution       { commit_count, change_count, churn, contributors(去重排序),
                      first/last_committed_at, renames[] }        // §24
Hotspot             { 原始指标 commits/churn/added/deleted/authors/last }  // §25 无 Score
ChangeCoupling      { path_a, path_b, co_change_count, evidence_commits(≤5) } // §26
HistoricalCorrelation { source, target, kind, evidence }        // §65 统一结构
CorrelationType     { Introduced|Resolved|Regressed(强语义——v1 由 M28 基线链
                      承担) | Touched|CoChanged|TemporallyCorrelated|Unknown }
HistoryWindow       { recent:<n>-commits }                        // §28/§52 显式
EvolutionReport     { window, files, hotspots, couplings, correlations, truncated }
```

排序确定性（§50）：热点 (commit_count desc, churn desc, path asc)；耦合
(count desc, pair asc)；contributors 去重排序；同输入同输出（测试锁定）。

## 4. Symbol History（§15-§17）

`Commit → FileChange → blob(parent^/commit) → M23 抽取 → M26
map_file_change → SymbolChange`：

- `ContractRemoved + Added` 同 qualified 基座对 ⇒ **SignatureChanged**（§17
  `Attack(int)` → `Attack(int, bool)` 语义级表达，非 File Modified）；
  无对纯移除 ⇒ Deleted；BodyChanged ⇒ Modified；BaseOrInterfaceChanged ⇒
  SignatureChanged。
- blob 不可得（缺失/超大/非 UTF-8）⇒ `extraction_available=false` + 诊断，
  符号层 Unknown 不猜测（§19）。
- 非 .cs 路径 ⇒ "symbol layer unsupported (still valid file history)"。

## 5. M26/M28 集成（§30/§34/§63）

- **commit-impact 垂直切片**（§82/§34）：`git . commit-impact <commit>` ⇒
  FileChange ⇒ .cs blob diff ⇒ M26 change_mapping ⇒ changed symbols ⇒ M25
  relations ⇒ M26 EvidenceGraph ⇒ `analyze_impact` ⇒ Affected（原始指标：
  changed_files/changed_symbols/affected/affected_tests/truncated——
  **无 Risk Score**，§37）。
- **health2 Evolution 规则**（§63）：`EVOLUTION/HOTSPOT`（Info，
  Origin=Inference，"not a quality verdict"）+ `EVOLUTION/CHANGE_COUPLING`
  （Info，描述明示 "NOT a semantic dependency" §27）。Metrics：
  `evolution.hotspot/coupling.count`。with_all_builtins = 15 条内建规则。

## 6. CLI（§56/§57）

```text
prism git . evolution <file> [--max N] [--json]
prism git . hotspots [--max-commits N] [--top N] [--json]
prism git . coupling [--max-commits N] [--min N] [--json]
prism git . symbol-history <file.cs> [--max N] [--json]
prism git . blame <file> [--max-lines N] [--json]
prism git . commit-impact <commit> [--max-files N] [--json]
```

（变体不设 path 位置参数——仓库路径由父级 `git [PATH]` 提供；entity 用
Option 规避 clap debug-assert 的可选/必选位置序。）自宿主实测（Prism 仓库
+ Godot/C# 夹具）见 Final Audit。

## 7. 边界与遗留

- Blame 边界（§23）：输出即 "recent contributors"；Ownership/Maintainer
  不建（无明确证据）。
- Co-change ≠ 语义依赖（§26/§27）：CO_CHANGES_WITH 与 REFERENCES/CALLS
  分离；Evidence convergence（§69）留待后续证据融合，不包装成预测。
- Introduced/Resolved/Regressed 强关联（§66）：v1 由 M28 基线链轮转承担
  （finding lifecycle New/Resolved/Regressed 已可用）；跨 commit 显式
  finding↔commit 关联查询 DEFER（P1）。
- GUI（§58/§84）/MCP（§60/§85）/SQLite（§53/§54）/增量缓存（§48/§75）/
  性能基准（§76）：DEFER（P1，沿 M21-M210 先例）；Commit Evolution Graph
  结构化数据已就绪供 M212（§95）。

## 8. 实测证据

- prism-git 原有 18 项 + prism-evolution 9 项（4 lib + 5 集成：--follow
  rename 穿透/贡献者去重/热点排序确定性/耦合证据/非仓库=空/默认窗口
  bounded/符号历史签名变化回放/签名对合并/纯删除）+ health2 +3。
- 自宿主（Prism 仓库）：hotspots（Cargo.lock 61 commits/562±18 churn 行）、
  coupling（cli.rs↔main.rs co-change 28 证据 commit 留存）、evolution、
  blame、symbol-history（非 .cs 如实 unsupported）。
- 垂直切片（§82 Godot/C# 夹具）：签名扩展 commit ⇒ 4 changed symbols ⇒
  **Direct 影响 cs:App.BattleManager#class**（caller）。
- 全量：clippy 0 / fmt 0 / **739 Rust**（M210 末 727 + 12：evolution 9 + health2 3）+ 55 vitest。
