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
| `crates/prism-core` | 地基层 | ID/错误/配置/任务/日志、CommandRunner/FileSystem 抽象、AppInfoService |
| `crates/prism-project` | Project Discovery 领域层（M1） | scanner/ignore/language/file/configuration/project_type/entry_point/statistics + DiscoveryService |
| `crates/prism-parser` | Code Understanding（M2） | Symbol 模型与坐标契约（UTF-8 字节）/LanguageParser/Registry + tree-sitter adapters（TS/TSX/JS/Python/C#） |
| `crates/prism-analysis` | 分析编排（M2） | 源文件选择/有界并发/取消/ProjectCodeModel/SymbolIndex |
| `crates/prism-graph` | 关系层（M3） | Module/Dependency/ResolutionStatus/GraphNode·Edge、四语言 import resolver、SCC 环检测、指标、DependencyService |
| `crates/prism-cli` | CLI 壳 | clap 参数 → 领域服务；退出码契约见 development.md |
| `src-tauri`（prism-app） | Tauri 壳 | IPC commands → 领域服务；扫描在后台线程、前端轮询 |
| `tests/integration` | 集成测试 | M0/M1 验收场景 + prism-test-helper |

## M1 Discovery 管线（纲领 §70）

```text
Project Root → Walker（确定性 DFS）→ IgnoreEngine → Metadata（只读 metadata，不读内容）
  → FileClassification → Language → Configuration → Project Type → Entry Point
  → Statistics → ProjectDiscovery（schema_version=1）
```

- 文件身份 = `project root + relative_path`（`/` 分隔）；绝对路径仅在执行 IO 时使用
- Ignore 优先级：hidden 过滤 + 系统安全规则/用户排除（按名匹配，不可被取反）
  > 项目 `.gitignore`（`ignore` crate 提供完整语义含 `!` 取反）
- gitignore 匹配依赖：`ignore` crate（BurntSushi/ripgrep 生态，MIT/Apache-2.0，
  维护良好；决策记录于 M1 commit b36e737）
- 取消语义：取消不是故障——保留已完成的部分事实，`status = cancelled`
- 非致命错误：逐条累积为 `ScanError{path,kind,message}`；
  `completed with warnings = Completed + errors_count > 0`
- GUI 轮询（get_scan_status）而非事件流（纲领 §68 允许）；数据全部来自真实扫描

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
7. **TS 契约类型暂为手写镜像**（src/types/prism.ts，M1 已覆盖 Discovery 契约）；
   契约膨胀后再引入自动生成（如 tauri-specta），避免现在制造重复类型（§28 允许）。
8. **Evidence 模型锚定于 prism-project（M1）**：`Evidence{source,path,reason}`；
   M2/M3 复用或扩展此定义（文档漂移以代码为准）。
9. **Application 层拆分**：prism-core::application 承载跨域基础服务（AppInfo），
   各域服务住在各自 crate（如 prism-project::DiscoveryService、
   prism-analysis::CodeAnalysisService）——保持 core 零业务依赖，依赖方向仍为
   CLI/Tauri → prism-project/prism-parser/prism-analysis → prism-core。
10. **M2 坐标契约**：line 1-based；column 1-based 按行内 UTF-8 字节；
    byte_offset 0-based 绝对字节。SymbolId =
    `symbol:{relative_path}#{qualified_path}#{kind}`（确定性派生）。
    qualified_name 为路径根式（跨语言方案统一、稳定可复现），不做伪跨语言语义。
11. **M2 解析技术**：tree-sitter 0.25 + 官方 grammar 0.23（统一 ABI，MIT/Apache）；
    ERROR 节点受限恢复（关键字序列合成），Python MISSING 节点诊断为必查项。
12. **M3 关系解析边界**：Resolution 七态显式建模；external 判定只基于本地
    元数据（package.json deps / 内置表 / 框架前缀），不做在线 lookup；
    qualified_name 路径根式；C# using 解析经命名空间索引（exact > child >
    framework prefix > unresolved）；环 = SCC（>1 节点）仅为记录。
13. **M5 ChangeSet 归属**：ChangeSet 是集成层（Git facts × 代码事实 × 模块事实），
    独立 crate `prism-changes`，不并入 prism-git/prism-analysis —— 未来可从
    Snapshot/Patch 产生 ChangeSource（M5 §103/§104）。确定性身份 =
    `changeset:{repository}#{base}#{target}#{scope}`（base/target 为解析后 SHA，
    repository 为目录名；绝对路径不入契约）。符号匹配分级保守：稳定 ID →
    git rename local identity → 同文件 body 证据（唯一候选才成立，多候选
    Ambiguous）；对侧解析带 error 时抑制 Added/Deleted 孤儿（§42）。
    历史源只读：`git cat-file`（含 `:0` index 侧），blob 字节长度核对防
    非 UTF-8 假空文件；绝不改写用户工作区。
14. **M6 影响语义**：只输出 Potential Impact；Changed 与 Impacted 集合严格分离；
    反向索引一次构建不复制 M3 模型；只遍历 Resolved 边，未解析/歧义保持不确定；
    deleted 符号经名字索引（精确限定名 HighConfidence / 同名 Probable）；
    confidence weakest-wins 分类传播；风险信号只有计数+证据、无阈值无总分；
    Added 符号不做消费者遍历。CallEdge.caller_id 必须与 M2 SymbolId 同构
    （含 #kind 段）——M6 曾因格式断裂丢失传递调用者。
13. **M3 ModuleId**：`module:{kind}:{identity}`（相对路径/命名空间/包名），
    不含 project 维度——M1 ProjectId 是运行期 UUID，稳定项目身份属 M11。
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
M3  ✓ 已实现（prism-graph：模块/依赖/解析/图/环/指标/有限调用图——
    直接调用经名字解析，同文件 High、resolved-import Medium，其余 Unresolved）
M4  ✓ 已实现（prism-git：porcelain v2 -z status、for-each-ref、log、
    三作用域 diff、只读 cat-file/rev-parse 历史源）
M5  ✓ 已实现（prism-changes：ChangeSet 集成层 —— GitDiff × 代码模型 ×
    M3 模块事实 → FileChange/SymbolChange/ModuleChange + 证据/诊断/汇总）
M6  ✓ 已实现（prism-impact：反向索引 + 有界 BFS + 证据路径 + 事实风险信号；
    ChangeSet × 代码模型 × M3 图 → ImpactAnalysis，candidate 语义）
M7  prism-health      Finding 体系 + 规则引擎
M8  prism-verify      Check 发现/计划/调度/执行/结果归一
M9  prism-environment 环境采集/匹配/解释
M10 prism-context     ContextPack 组装（无 LLM）
M11 prism-snapshot + prism-store   快照与 SQLite 持久化
```

依赖方向不变：所有新 crate 只允许依赖 prism-core（及声明过的兄弟 crate），
禁止依赖 tauri / 前端。
