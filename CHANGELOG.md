# Changelog

所有显著变更记录于此。格式基于 Keep a Changelog。

## [Unreleased]

### Fixed

- **Verify 结果契约**：`get_verify_results` 原先返回 Rust tuple，serde 序列化为
  JSON 数组，而前端按 `{ plan, report }` 对象读取，导致 Verify 页与概览验证摘要
  永远为空。改为具名 `VerifyResultsDto`，并补充序列化回归测试。
- **全局快捷键**：Ctrl/Cmd+K 与 Ctrl/Cmd+1..9 不再在输入框/文本域内劫持输入；
  同时支持 macOS 的 Cmd 键。
- **项目切换竞态**：`ProjectStore` 的扫描/代码理解/依赖图轮询改为链式单次在飞，
  切换项目会停止旧轮询并用 generation 丢弃旧结果，避免重复启动与串项目。
- **可恢复的错误态**：可视化面板出错时仍保留视图切换与重试，不再卡在纯文本页。
- **暗色主题可读性**：severity 配色与 `.finding-detail` 改用主题变量，暗色下不再
  近乎不可见；清掉 `--accent` / `--border` 等未定义变量。
- **上下文健康**：`severity === "Info"` 大小写错误导致全部 finding 被着色为 warning。
- **命令面板**：语言切换后标签不刷新；空结果时光标索引被钳到 -1。

### Changed

- **界面重做**：新的设计系统（分层表面、渐变强调、图标导航、页头信息层级、
  卡片/指标块/按钮/主题感知状态色），深色与浅色主题、两种密度均适配。
- **i18n 补全**：外壳、概览页、设置页与关键子卡片全部走 `t()`，中英文案对齐。
- 新增浏览器内 UI 预览入口 `demo.html`（固定演示数据，不进入生产构建），
  用于文档截图与前端联调。

### Added

- 许可证：`LICENSE`（双许可说明）、`LICENSE-APACHE`、`LICENSE-MIT`；
  `package.json` / `workspace.package` 增加 `MIT OR Apache-2.0`。
- `SECURITY.md`（安全模型 + 漏洞上报流程）。
- `README.md` 重写为英文主文档、`README.zh-CN.md` 中文版，各自带对应语言界面截图。

## [0.1.0] — 2026-10-05（M0–M12 交付 + M13 Release Hardening）

### Engineering Intelligence（M1–M4）

- Project Discovery：文件扫描、ignore 引擎、语言/项目类型/入口点识别
- Code Understanding：多语言 Symbol 模型（TS/JS/Python/C#）
- Dependency & Architecture：模块依赖、环检测、指标

### Change / Impact（M5–M6）

- ChangeSet：GitDiff → FileChange → SymbolChange → ModuleChange
- Change Impact Analysis：反向图遍历、距离/置信度、blast radius

### Health（M7）

- 13 条确定性规则 + ARCH004 禁止依赖对
- Finding 体系（Evidence-first，无评分）
- `prism health` 完整 CLI（JSON/MD/HTML）

### Verify（M8）

- Check 发现（Node/Python/C#/Rust/CI/自定义）+ 计划/执行
- WritePolicy 默认拒绝、超时取消、输出截断
- 结构化输出解析（pytest/dotnet/junit/SARIF 嗅探）
- verified_state + `prism verify checks/ci/release`

### Environment（M9）

- host/shell/PATH 采集与规范化
- 工具发现（allowlist ToolProbe）+ 结构化版本比较
- runtime/SDK/包管理器/venv 发现
- Requirement 匹配（四态语义）+ `prism env` 完整 CLI

### Context（M10）

- 确定性任务分类 + 目标解析（不猜）
- CandidateCollector + 确定性优先级带 + 预算/Critical 预留
- Context Health（CTX 规则）+ trust/risk signals
- `prism context`（dry-run/explain/check-health/profile）

### Snapshot / Timeline（M11）

- 稳定项目身份（ProjectRegistry）+ SQLite 持久化（迁移/事务）
- 去重（状态指纹）+ SnapshotComparator（finding 生命周期）
- Timeline 派生（snapshots + git，确定性排序）
- `prism snapshot`/`prism history`/`prism compare`/`prism timeline`

### Product UI（M12）

- Desktop Shell（导航分组：Project/Change/Quality/AI Context/History）
- 全部页面真实 IPC（Overview 状态卡/Understand/Changes/Health/Verify/
  Environment/Context/History）
- Theme（dark/light）+ Density + Settings 页
- Command Palette（Ctrl+K）+ 页面快捷键（Ctrl+1..9）

### Hardening（M13）

- 性能基线实测（scan 2.3s / snapshot 0.23s / history 48ms @ Prism 仓库）
- SQLite 事务原子写 + 迁移测试 + corrupt DB 防护
- Secret 扫描（tracked 文件 + 持久化 JSON + export 三层）
- 生产构建（release profile）验证

### 边界（永久）

- 本地优先：无网络、无遥测、无 AI 依赖、无云端
- Git 只读：不 checkout/reset/commit 用户仓库
- 不保存源码副本、不保存 secrets
