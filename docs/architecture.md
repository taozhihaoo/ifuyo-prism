# Prism Architecture (M0)

本文档回答 M0 验收（§40 Architecture）要求的问题：
Core 在哪里？Application Layer 在哪里？Tauri 在哪里？Frontend 在哪里？
CLI 在哪里？未来模块在哪里？依赖方向是什么？

## 分层与依赖方向

```text
Frontend (src/, React + TS)
   ↓  invoke()  [IPC]
Tauri Shell (src-tauri/, prism-app crate)
   ↓
Application Layer (prism-core::application)
   ↓
Core Domain (prism-core::error/ids/severity/task/config/...)
   ↓
Runtime Abstractions (prism-core::runtime, trait + std 实现)
```

禁止的反向依赖：

```text
Core → Tauri / React / Browser API   ✗
CLI 自建业务逻辑（不经 Application/Core）✗
```

## Crate 地图（M0 现状）

| Crate | 角色 | 说明 |
| --- | --- | --- |
| `crates/prism-core` | Core + Application 层 | 领域类型、错误/配置/任务/日志、CommandRunner/FileSystem 抽象、AppInfoService |
| `crates/prism-cli` | CLI 壳 | clap 参数 → Application/Core；退出码契约 0/1/2 |
| `src-tauri`（prism-app） | Tauri 壳 | IPC commands → Application/Core；配置与日志初始化 |
| `tests/integration` | 集成测试 | 跨 crate 验收场景 + prism-test-helper |

### prism-core 内部结构

```text
src/
├─ error.rs        # PrismError / PrismErrorCode / PrismErrorDto（对外 JSON 契约）
├─ ids.rs          # ProjectId..TaskId 统一 ID（serde 透明字符串）
├─ severity.rs     # Severity(Info/Warning/Critical) + Confidence（分类式）
├─ timestamp.rs    # RFC3339 UTC 时间戳（毫秒精度）
├─ project.rs      # Project 最小骨架（id/name/root_path）
├─ task.rs         # TaskState/TaskRecord/TaskTracker/CancellationToken
├─ progress.rs     # 通用 ProgressEvent
├─ config.rs       # PrismConfig（prism.config.json）+ 跨平台路径
├─ logging.rs      # 单行 RFC3339 日志（stderr + 可选文件）
├─ runtime/        # CommandRunner / FileSystem trait + std 实现
└─ application/    # AppInfoService（GUI 与 CLI 的共同事实源）
```

### 设计决策记录（M0 拍板，后续里程碑跟随）

1. **M0 只建三个真实 crate**（core/cli/app）。`prism-project/parser/analysis/...`
   等未来 crate 随各自里程碑创建，不预建空壳（纲领 §36 反对过度工程）。
2. **prism-core 同时承载 Application 层**（`application` 模块）。规模变大后再拆
   `prism-app-services` 之类的独立 crate；模块边界现在已固定。
3. **Severity 锚定为 Info / Warning / Critical**（纲领 §11）。M7 Health 若需要
   更细粒度，允许在内部加 P0–P3，但对外枚举不改。
4. **配置文件定名 `prism.config.json`**（符合 M7 规范允许的 `prism.config.*`
   形态；选 JSON 避免引入 YAML 依赖）。schema_version 从 1 起。
5. **JSON 契约统一 snake_case**；时间戳一律 RFC3339 UTC。
6. **CLI 用 clap v4**：M1–M11 要长出约十个子命令与大量 flag，标准解析器
   优于手写（依赖决策见 development.md）。
7. **TS 契约类型暂为手写镜像**（数量极少，src/types/prism.ts）；契约膨胀后
   再引入自动生成（如 tauri-specta），避免现在制造重复类型（§28 允许）。
8. **CommandRunner 超时用轮询**（25ms tick），管道由独立线程读取防死锁；
   进程树终止（Windows job object）留给 M8 Verify 执行器。
9. **ID 生成策略**：运行期身份用 UUIDv4（TaskId 等）；需要跨运行稳定的
   语义身份（M2 SymbolId、M7 FindingId）由对应里程碑用 `from_string`
   确定性派生，禁止随机。

## Tauri IPC（M0 基础集）

```text
get_app_info / get_prism_version / get_runtime_info   # 真实数据，来自 AppInfoService
```

前端不直接 `import { invoke }`，统一经 `src/services/ipc.ts`；
错误统一归一化为 `PrismErrorDto { code, message }`。

## CLI 契约

- `prism --version` / `prism --help`：真实数据
- `prism info [--format json|text]`：真实应用/运行时信息
- `scan|inspect|changes|health|verify|env|context|report`：占位，明确输出
  `Not implemented in current milestone.` 并以退出码 1 结束（不伪造数据）
- 退出码：`0` 成功 / `1` 错误或未实现 / `2` 用法错误

## 未来模块落位（随里程碑创建）

```text
M1  prism-project     文件扫描 / ignore / 语言与项目类型识别
M2  prism-parser      Language Adapter + 符号抽取（TS/JS/Python/C#）
M2  prism-analysis    ProjectCodeModel / SymbolIndex
M3  （prism-analysis 内）模块/依赖/调用图、环检测
M4  prism-git         只读 Git 事实层（经 CommandRunner 调系统 git）
M5  prism-changes     ChangeSet（或并入 prism-analysis，M5 开工时定案）
M6  prism-impact      影响分析（反向遍历 M3 图）
M7  prism-health      Finding 体系 + 规则引擎
M8  prism-verify      Check 发现/计划/调度/执行/结果归一
M9  prism-environment 环境采集/匹配/解释
M10 prism-context     ContextPack 组装（无 LLM）
M11 prism-snapshot + prism-store   快照与 SQLite 持久化
```

依赖方向不变：所有新 crate 只允许依赖 prism-core（及声明过的兄弟 crate），
禁止依赖 tauri / 前端。
