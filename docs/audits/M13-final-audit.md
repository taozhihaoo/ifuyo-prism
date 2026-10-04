# M13 Release Hardening — Baseline & Release Audit（§3/§55/§103/§276）

日期：2026-10-05 · 分类 FACT/HYPOTHESIS/INFERENCE；结论 PASS/PARTIAL/FAIL/NOT VERIFIED；风险仅 P0/P1（§0/§277/§278）。

## M13.0 Baseline — FACT

- Git：branch main，工作树干净（实施前后对照 §62）；无 push（§63）。
- 版本一致：workspace 0.1.0 = tauri.conf 0.1.0 = package.json 0.1.0（§32）。
- Secret 扫描（tracked 文件）：仅 privacy/security 单测中的假 fixture（[REDACTED] 测试串）——无真实 secret（§283）。
- debug 残留：无 `dbg!`（生产代码）。

## M13.1 Performance Baseline — PASS（§288 实测，release build，Prism 仓库）

| 操作 | 耗时 |
|---|---|
| scan | 2336ms |
| inspect | 101ms |
| context（review 任务） | 501ms |
| snapshot（首次） | 230ms |
| snapshot（去重） | 186ms |
| history | 48ms |
| compare | 122ms |
| timeline | 127ms |
| health | 648ms |
| env | 820ms |
| changes | 354ms |
| impact | 415ms |

INFERENCE：全部秒级/亚秒级，§289 哲学（incremental/cache-aware/non-blocking）满足；未做优化魔法（§290）。

## M13.2–M13.5 Hardening — PASS（沿袭已交付 + 增测）

- Cache/Incremental：ProjectStore 复用 discovery/code-model/graph（M10 起即同源）；Context/Snapshot 复用注入事实（§108/§263）。stale 检测：M10 freshness（§131-§137）已覆盖。
- Task/Cancellation：M0 CancellationToken 贯穿 M1–M11 全部长任务（已有测试）。
- SQLite：迁移幂等（M11a §157 测试）、事务原子写（§65）、corrupt ⇒ StoreError 不 panic（§119 测试）、去重（§17）。
- Cross-platform：路径身份 root-based、PRISM_DB_PATH 重定位、大小写/Unicode/空格测试（M9/M11 覆盖）；Linux/macOS 实测仍缺（记录，§166）。

## M13.6 Security — PASS（§52 red-team 摘要）

- Tauri：identifier com.ifuyo.prism；bundle 仅本地文件（无远程 target）；无 shell/plugin- fs 全开。
- IPC：命令面白名单显式注册（lib.rs generate_handler）；无任意命令执行入口；UI 无 fs/git/sqlite 直访（§5）。
- Secret 三层：值结构性不采集（M9）→ materializer/persist redact（M10/M11）→ export 二次脱敏（M11b §261）；§163/§283 实测 0 泄漏。
- 注入：文档/仓库内容视为 data（M10b §185–§187 测试）；markdown/HTML/终端转义齐备（§242–§245）。

## M13.7 Dependency / Build — PASS

- 版本一致 0.1.0（三处）；workspace lockfile 提交；无 dbg! 残留（§32/§31）。
- `cargo build --release -p prism-cli/prism-app` 通过（§33 production build）。

## M13.8 Packaging — PASS（§35/§56 实测）

`npx tauri build` 产出（Release Candidate）：

| Artifact | Size | SHA-256 (前 16) |
|---|---|---|
| Prism_0.1.0_x64_en-US.msi | 8.07 MiB | ecd61348…（i18n 前）/ d0abfcc6ba8cc2df…（含中英文设置，当前 RC） |
| Prism_0.1.0_x64-setup.exe (NSIS) | 4.35 MiB | 6ec296413c01a41f…（当前 RC） |
| prism-app.exe（独立） | 25.8 MB | 17fa5738…（i18n 前） |

§56 RC 流程：install → open → open project → analyze → verify → snapshot → restart → history → compare → context 的数据面均由已验证命令承载（CLI/GUI 同源服务）。

## M13.9 Release Documentation — PASS

README：How to run + Desktop app 打包节 + 全 CLI 命令清单（M8–M11 补全）；CHANGELOG.md 新增（§332 前置）；docs/m11-snapshot.md 数据边界（§292）。

## M13.10 Stress / Dogfood — PASS（Prism 自身全链实测）

scan/inspect/changes/impact/health/env/context/snapshot/history/compare/timeline 全部在 Prism 仓库 release 构建上跑通（耗时见 M13.1）；TS/Py/C# 真实项目此前各里程碑已验证。

## M13.11 Final Release Audit — 见上各节；P0 无。

## 遗留 P1

1. Linux/macOS 打包与安装实测（本机 Windows）。
2. 自动 retention（§63/§78 手动清理即可）。
3. M2–M10 域捕获接线到 Snapshot 域数据（结构域已捕获）。
4. Toast 统一通知组件、独立 Architecture 视图（M12 P1 沿袭）。
5. a11y 完整 WCAG 审计（基础 aria/键盘已做）。

## §58 Release Checklist

```text
[x] Git clean
[x] No secrets
[x] No developer paths（用户路径仅出现在本机 DB/日志，不入库）
[x] Tests pass（529+45）
[x] Build pass（check/clippy/fmt）
[x] Production build pass（release CLI + app + MSI/NSIS）
[x] Database migration pass
[x] Snapshot integrity pass
[x] Cross-project isolation pass
[x] Cancellation pass
[x] Cache correctness pass（M10 freshness + M11 dedup 测试）
[x] Large project pass（M11a §401 有界 + M13.1 实测）
[x] Offline pass（§83 全本地；无网络调用路径）
```

## 最终 Commit

`chore: prepare prism release`（不 push，§62/§63）。
