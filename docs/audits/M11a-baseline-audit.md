# M11 Baseline Audit（§188–§190，实施前审计）

日期：2026-10-03 · 只列与 M11a 相关的事实与结论。

## A. FACT（真实代码证据）

1. `prism_project::Project.id` = `ProjectId::new()`（运行期 UUID，project.rs:39）——**每次打开都变**，不满足 §8 稳定身份要求。
2. `prism_core::ids::define_id!` 提供 `ProjectId/SnapshotId`（后者 M0 已预留，M11 启用）。`SnapshotId::new()/from_string/as_str` 可直接复用（§151）。
3. Cargo.lock **无 rusqlite**——需新增依赖（build-time crates.io 拉取；运行时仍 100% local，§83 不冲突）。
4. M0 `task.rs`（CancellationToken/TaskState）与 `progress.rs` 存在，可复用（§106/§107）。
5. M1 `DiscoveryService`（文件/语言统计）、M4 `GitService.status/log`、M5 `ChangeSetService`、M9 `EnvironmentService::scan` 均有可独立调用的 API；M2/M3/M7/M10 在 CLI 层已有成熟组装先例（context_command.rs）。
6. M8 Verification 与 M7 Health 仅存在于 tauri session/需要长时运行——CLI capture 默认按 §109 标记 domain `NotCaptured/Unavailable`，不做自动运行。
7. 中央脱敏 `prism_core::privacy::redact_text` 已有（§28 SnapshotRedactor 复用，不新建 detector）。
8. CLI 已有 `prism snapshot`?? **否**——`Command` enum 无 snapshot/history；placeholder 测试列表里也没有（M0 列表只含 report）。需新增两个真实命令。
9. 无任何既有 storage 抽象/repo pattern——按 §186 新建 `prism-store`。
10. 错误体系 `PrismError`（core/error.rs）可扩展变体（§185）。

## B. HYPOTHESIS

- rusqlite `bundled` feature 可在 Windows/MSVC 编译（自带 sqlite3.c）。
- `Project::from_root` canonicalize 后的路径可作为 canonical_root 身份依据（§8 默认依据）。
- 测试可用环境变量覆盖 DB 路径（§220 Database Path Tests），正式路径用 `dirs`-style 平台 API——为避免新依赖，自实现 `home_dir + AppData/Prism`（Windows `%APPDATA%\Prism`，Unix `~/.local/share/Prism`）。

## C. INFERENCE

- prism-store（SQLite + migrations + store trait + in-memory test double）与 prism-snapshot（domain model + canonical serialization + fingerprints + capture orchestration）两个新 crate 即可覆盖 M11a P0-1..P0-9/§194 前段；Timeline/compare 归 M11b（P0-10 留接口）。
- capture 默认只跑 M1+M4（快），M2/M3/M5/M6/M7/M8/M9/M10 域按 §108/§109 标 `NotCaptured`（可用 `--refresh-analysis` 扩展为后续项；本篇以 Unavailable 诚实降级）。

## D. Current Architecture

见 A.4/A.5/A.7：M1–M10 服务层无 DB 依赖（§104 要求满足）；Application 层在 CLI main.rs + tauri。

## E. Existing Reusable Infrastructure

ids、task、privacy、error、project/git/changeset 服务、CLI runner 模式、integration test crate。

## F. Gaps

稳定 ProjectIdentity/Registry、SQLite、canonical serializer、domain fingerprints、dedup、`prism snapshot`/`prism history` 命令、restart/multi-project/isolation 测试。

## G. P0

§192 P0-1..P0-9 + P0-11/12（Timeline P0-10 归 M11b）。

## H. P1

GUI history、export、retention 自动化、其余域的 capture（随服务层组合）。
