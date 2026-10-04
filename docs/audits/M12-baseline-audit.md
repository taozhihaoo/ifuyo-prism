# M12.0 Baseline Audit — Existing Capability Matrix（§4）

日期：2026-10-05 · 只读审计，以当前代码为准（§4/§189）。分类 FACT/HYPOTHESIS/INFERENCE；状态 CONFIRMED/PARTIAL/MISSING。

## A. FACT — 现有能力矩阵

| Capability | Backend | IPC | Frontend Consumer | Current UI | Status | Gap |
|---|---|---|---|---|---|---|
| M1 Discovery | prism_project::DiscoveryService | `start_scan`/`get_scan_status`/`get_discovery` | ProjectStore.discovery | Overview + Understand | CONFIRMED | — |
| M2 Code | CodeAnalysisService | `start_inspect`/`get_inspect_status`/`get_code_model` | ProjectStore.codeModel | Understand（树+符号） | CONFIRMED | — |
| M3 Architecture | DependencyService | `start_dependencies`/`get_dependency_graph` | ProjectStore.dependencyGraph | Understand（DependencyView） | CONFIRMED | 独立 Architecture 视图缺（§16） |
| M4 Git | prism_git (CLI) | tauri **MISSING**（M5/M6 在 GUI 内部组装） | — | — | PARTIAL | 独立 git facts IPC 缺 |
| M5 ChangeSet | ChangeSetService | changes_commands（start_changeset/get_changeset） | ChangesPage | Changes 页 | CONFIRMED | — |
| M6 Impact | ImpactAnalysisService | impact_commands | ChangesPage（impact 链） | Changes 页 | CONFIRMED | — |
| M7 Health | HealthAnalysisService | health_commands（analyze/rules/status） | HealthPage | Health 页 | CONFIRMED | — |
| M8 Verify | SnapshotCaptureService 之外——verify_commands | verify_commands | VerifyPage | Verify 页 | CONFIRMED | — |
| M9 Environment | EnvironmentService | env_commands（analyze/status/sections） | EnvironmentPage | Environment 页 | CONFIRMED | — |
| M10 Context | ContextService | context_commands（generate/status/results/export） | ContextPage | Context 页 | CONFIRMED | — |
| M11 Snapshot | SnapshotCaptureService + SnapshotStore | snapshot_commands（capture/history/compare/timeline/delete） | SnapshotHistoryPage | History 页 | CONFIRMED | Snapshot Detail 分 tab（P1） |
| Project 打开 | Project::from_root | `open_project`（file dialog pick） | ProjectStore.pickAndOpenProject | Overview | CONFIRMED | **Recent projects / 手输路径 / 全局 Selector 缺（§12/§68）** |
| Global Search / Palette | — | — | — | — | MISSING（§33/§34） |
| Settings / Theme | — | — | — | — | MISSING（§48/§11） |
| Keyboard shortcuts | — | — | — | — | MISSING（§46） |
| Toast/Notifications | — | — | 内联错误 | 各页 | PARTIAL（§47 统一 Toast 缺） |

## B. HYPOTHESIS

- nav 分组（§7 IA：PROJECT/CHANGE/QUALITY/AI CONTEXT/HISTORY/SYSTEM）可用现有 Sidebar 加 section 标题实现，无路由依赖引入。
- Recent projects 可复用 prism-store `projects` 表（canonical_root/display_name/last_seen_at）——新增 tauri 命令 `list_recent_projects` + `open_project` 后 upsert last_seen。
- Command Palette 只需页面导航 + 打开项目两个动作，自实现浮层（§0 不引入框架）。

## C. INFERENCE

M12 范围内不需要任何 core 改动（§0.1）；全部为 tauri 命令补充 + UI 结构/交互。


---

## M12 交付后复核（同日补充）

以下缺口在 M12 实施中补齐：

| 缺口 | 处置 | 证据 |
|---|---|---|
| 导航分组（§7 IA） | CONFIRMED 已实现 | Sidebar.tsx NAV_GROUPS：Project/Change/Quality/AI Context/History |
| Recent Projects（§12/§68） | CONFIRMED 已实现 | prism-store `list_projects`（last_seen 降序）+ `list_recent_projects` IPC + Command Palette 项目项 |
| Command Palette（§34） | CONFIRMED 已实现 | CommandPalette.tsx（Ctrl+K，方向键+Enter，页面+项目双类） |
| Keyboard shortcuts（§46） | CONFIRMED 已实现 | Ctrl+K、Ctrl+1..9 页面直达（App.tsx） |
| Settings/Theme/Density（§48/§11） | CONFIRMED 已实现 | SettingsProvider + SettingsPage + CSS data-theme/data-density |
| Overview Status Cards（§14） | CONFIRMED 已实现 | OverviewPage Status 卡（health/verify/last-snapshot session 事实） |
| GUI History（§127-§133） | CONFIRMED 已实现（M11b 末段交付） | SnapshotHistoryPage（list/compare A-B/timeline/delete） |

**仍为 P1（记录，不在 M12 范围强做）**：Toast 统一通知组件（§47，当前内联错误如实显示）、
独立 Architecture 页（DependencyView 已在 Understand 内，§16 数据能力完整）、
Full-text search（§33 v1 只做导航/项目搜索）。
