# M11 Final Report（规格 §332 汇报格式）

## 1. Baseline

实施前审计 `docs/audits/M11a-baseline-audit.md`：ProjectId 运行期 UUID（不稳定）→ 稳定身份由 ProjectRegistry 承担；无 storage 抽象 → 新建 prism-store；rusqlite 需引入（bundled）。全部 FACT 有代码行号。

## 2. Implemented

M11a：ProjectSnapshot 模型（十域指纹）、Canonical JSON（序无关）、prism-store（SQLite 三表 + schema_migrations + 事务原子写 + SnapshotStore trait + InMemory 替身）、capture 编排（身份/去重 §17/force §18/§145 Busy 锁）、`prism snapshot`/`prism history`。
M11b：SnapshotComparator（十域 diff + finding 生命周期（无行号指纹）+ verification 迁移）、Timeline 派生（snapshots+git，确定性 id/排序）、EvolutionSummary、markdown 渲染、history export/delete、GUI History 页（list/compare/timeline/delete）。

## 3. Snapshot

`prism snapshot <path> [--force/--note/--json]`：M1+M4 事实（M2–M10 注入点就绪，默认 NotCaptured 诚实降级 §108/§109）；basis GitRevision/WorkingTree；§145 Busy 锁；§17 去重文案。

## 4. Storage

SQLite at `%APPDATA%/Prism/prism.db`（PRISM_DB_PATH 覆盖 §220）；schema_migrations + 0001（三表+双索引）；事务原子写（§65/§66 无半成品）；busy_timeout（§144）；SQL 只在 prism-store（§228/§299）。

## 5. Comparison

`prism compare <path> <a> <b>`：十域 wrapper {status,data} 直比 → facts（added/removed/scalar）；finding 生命周期（new/resolved/still_active/unknown；fingerprint 无行号 §37/§134）；verification transitions（§73）；compatibility limited（§71）；Unavailable ≠ empty（§245）。

## 6. Timeline

`prism timeline <path>`：Snapshots + Git commits → 派生事件（每域变化一条 §233）；source 标签 Git/PrismSnapshot/Derived（§90）；确定性 id + ASC 排序（§152/§153）；§88 无 snapshot ⇒ git-only + limitation。

## 7. Privacy

值结构性不采集；persist/export 前中央 redact（§257 单一 redaction engine）；`.env` keys-only（§58-§60）；§163 实测 secret fixture 0 泄漏。

## 8. CLI

`prism snapshot`（--force/--note/--json）、`prism history`（--json/--limit）、`prism compare`（--json/--markdown）、`prism timeline`（--limit/--json/--markdown）、`prism history export`（--output）、`prism history delete`（--snapshot/--all --yes）。exit 0/1/3。

## 9. GUI

History 页（导航 History 项）：列表（§130）→ A/B 选择 → Compare 分域 facts（§49/§95）→ Timeline（§129）→ Capture now / per-row delete（§62）。5 vitest。GUI/CLI 同源 SnapshotStore（§475/§126）。

## 10. Tests

529 Rust（store 5、snapshot model 4/capture 5/compare 9/timeline 4、CLI snapshot 5）+ 45 vitest（含 ContextPage 5）。clippy 0，fmt exit 0。

## 11. Real Project Validation

TS（ctxdemo）/Py（pydemo）/C#（csdemo）snapshot+history exit 0；Prism 自身 review 任务 18 items；compare/timeline/markdown/health 人工检查；§163 secret grep 0。

## 12. Git

工作树干净（§275）；测试 DB 在系统临时目录；全程无 push。

## 13. Remaining P0/P1

P0 无。P1：M2–M10 域捕获接线、§203/§204 迁移兼容矩阵、自动 retention、GUI snapshot detail 分 tab、Linux/macOS 实测。

## 14. Commit

`feat: add snapshot and timeline engine`（6ba643f + 后续 GUI/审计提交；不 push）。
