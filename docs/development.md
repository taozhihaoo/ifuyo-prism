# Prism Development Guide

面向后续 AI Agent 与人类贡献者的工程手册。目标（纲领 §48）：任何后续 Agent
都能快速回答——这个文件属于哪一层？能依赖谁？业务逻辑放哪？错误如何传播？

## 环境要求

- **Rust**：stable，`x86_64-pc-windows-msvc`（本机默认）。需要 VS Build Tools
  （C++ 工作负载）。`rustup default stable-x86_64-pc-windows-msvc`
- **Node**：≥ 20（本机 v24），npm ≥ 10
- **WebView2**：Windows 11 自带
- 已知环境坑：Windows Defender 会误报隔离 rust-mingw 组件——建议给
  `%USERPROFILE%\.rustup`、`%USERPROFILE%\.cargo` 与本仓库添加排除目录

## 常用命令

```bash
# Rust
cargo check --workspace      # 快速类型检查
cargo test --workspace       # 单元 + 集成测试
cargo fmt --all              # 格式化（提交前必跑）

# 前端
npm install                  # 首次 / package.json 变更后
npm run typecheck            # tsc --noEmit
npm test                     # vitest run
npm run build                # tsc + vite build → dist/

# 桌面应用
npm run tauri dev            # 开发窗口（Vite :5173 + Tauri）
npm run tauri build          # 发布构建（M0 验证用 --no-bundle 亦可）

# 图标（改动品牌源图后）
npm run icon                 # 生成 PNG → tauri icon 全平台尺寸
```

## 分层规则（必须遵守）

1. **业务逻辑只写在 prism-core（或未来 prism-* crate）**；`src-tauri` commands
   与 `crates/prism-cli` 只做参数解析 → 调用服务 → 序列化输出。
2. **前端不直接 invoke**：所有 IPC 经 `src/services/ipc.ts`。
3. **错误一律 `PrismResult<T>`**：业务路径禁止 `unwrap()/expect()/panic!()`
   （测试与静态不可达分支除外，需注释依据）。
4. **新 JSON 契约字段用 snake_case**，时间戳用 `Timestamp`（RFC3339 UTC）。
5. **新 ID 必须用 `ids.rs` 的新类型**，不允许裸字符串。
6. **日志用 `log` 宏**，禁止 `println!` 调试输出进业务代码；不记录文件内容、
   密钥、token（privacy 红线，纲领 §15）。
7. **耗时操作**应登记 TaskTracker、接受 `CancellationToken`、报告 `ProgressEvent`。
8. **外部命令**经 `CommandRunner` trait（测试可 mock），不直接 `std::process`。

## 测试策略（纲领 §21）

```text
单元测试（各 crate #[cfg(test)]）→ 集成测试（tests/integration）
→ fixture 项目（tests/fixtures/）→ GUI 手动验证（tauri dev）
```

- fixture 项目保持极小且稳定；M1+ 扫描它，禁止引入真实依赖；
  `tests/fixtures/*/tests/*.test.ts` 这类占位文件不是前端测试
  （vitest 已排除仓库 `tests/` 目录）
- M2 起，重要 Analyzer 必须加“真实开源项目”验证（纲领 §22）

## Git 纪律（纲领 §8 / §20.2）

- 每个子任务一个清晰 commit（`feat:/test:/docs:/chore:` 前缀）
- 提交前：`cargo fmt` + `cargo test` + 对应前端检查；`git diff --stat` 目视
- **默认不 push**；推送是显式的人工动作
- 提交信息里避免反引号（bash 会做命令替换，用引号或改写）

## 依赖策略（纲领 §47）

新增任何依赖前先回答（写进 commit message 或文档）：

```text
Dependency / Why needed / Alternative considered / License / Maintenance impact
```

M0 已引入的全部运行时依赖：`serde`、`serde_json`、`uuid`、`time`、`log`、
`clap`、`tauri`。前端运行时依赖：`react`、`react-dom`、`@tauri-apps/api`。
其余（vite/vitest/testing-library/clippy 等）为开发依赖。

已知版本坑（2026-10 记录）：

- `log` 0.4.34 起 **std/alloc 特性默认关闭**，必须 `features = ["std"]`
  否则 `set_boxed_logger` 不存在
- `time` 的 RFC3339 在 `time::format_description::well_known::Rfc3339`，
  序列化辅助在 `time::serde::rfc3339`；`parse` 需要 `parsing` feature

## CLI 退出码契约（纲领 §78，M1 起细分）

`0` 成功 · `1` 运行错误或命令未实现 · `2` 用法错误 · `3` 输入无效 ·
`4` 已取消 · `5` 成功但有警告（completed with warnings）

## 技术决策台账

见 [architecture.md](architecture.md) 的“设计决策记录”。新决策必须追加，
不允许悄悄偏离。

## 后续里程碑怎么开工

1. 读 `Spec/<milestone> 开发提示词.md` 全文
2. 做 Baseline Audit（git 状态、全量测试、上一里程碑审计的 P0/P1）
3. 按提示词实现 → 测试 → Final Audit（FACT/HYPOTHESIS/INFERENCE）
4. 本地 commit，不 push
