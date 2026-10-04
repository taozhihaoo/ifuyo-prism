# Changelog

所有显著变更记录于此。格式基于 Keep a Changelog。

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
