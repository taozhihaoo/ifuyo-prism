# Prism M9a Gate Check（Environment Facts and Matching — Part 1/2 Exit Gate）

日期：2026-10-03 · 依据：`Spec/Prism M9a — Environment Facts and Matching 完整开发提示词.md`【阶段性完成门槛】
验证基线：`cargo test --workspace` 431 passed / 0 failed（54 个测试二进制）+ vitest 36 passed，clippy --workspace --all-targets 0 warning。

## 门槛逐项核对

| 门槛项 | 状态 | 证据 |
| --- | --- | --- |
| EnvironmentSnapshot/Requirement/Match/ToolProbe 模型 + schema_version 稳定序列化（§11–§13/§38–§41/§292–§294/§337–§340） | ✅ | `model.rs`：EnvironmentSnapshot{schema_version=1, id(uuid 观测 id §13), scan_outcome, completeness}；EnvironmentRequirement{kind 12 态, operator 6 种, source/evidence/confidence/applicable}；RequirementMatch 4 态；ToolProbe（allowlist probe 定义）；全部 serde snake_case + 单测 |
| Host/OS/Architecture/Shell/PATH 采集与规范化（§14–§21/§108–§111/§191–§193） | ✅ | `host.rs`（OS family 规范化、Windows 注册表版本 FFI、最小身份 §14——username 不采集；shell 启发式不启动 profile §161）；`paths.rs`（normalize/dedupe 保留首项/duplicate+missing 诊断/大写不敏感 identity §192/上限截断 §314；Unicode 测试 §193） |
| 可执行文件发现、ToolProbe allowlist、结构化版本比较（§22–§30/§112–§117/§155–§158/§204） | ✅ | `tools.rs`（23 项 allowlist，version 形态参数断言；PATH 解析 + 同名候选 §65；EnvironmentProfile 惰性选择 §120–§123）；`probe.rs`（M0 runner、5s 超时、取消传播、64KiB 有界输出、cmd.exe /C wrap）；`version.rs`（20.1<20.10<22 数值比较、raw-only 保守、AtLeast/Equals 语义 + Unknown） |
| Runtime/SDK/PackageManager 发现（§31–§37/§104–§107/§159） | ✅ | `runtime.rs`：runtime 别名去重；dotnet --list-sdks / rustup toolchain list 解析（非语义化 raw-only）；lockfile 关联 + packageManager 字段优先（evidence 字段回答 §37 provenance；§105/§106 匹配语义）；corepack 只记录不启用（§107）；venv 识别 + interpreter 探测 + active 标记（§69）；resolve_project_local（§62/§68） |
| RequirementMatch 语义：Satisfied/Unsatisfied/Unknown/NotApplicable；Unknown ≠ Missing ≠ Broken（§46–§50/§209） | ✅ | `requirement.rs` 6 个单测覆盖四态；§158 version unknown ⇒ Unknown；§105 lookup 确认缺失才 Unsatisfied；§50 applicable 标记；§54 值类 operator ⇒ Unknown；NotFound 仅在 lookup 确认后（§209） |
| 敏感变量识别与 redaction（§54–§57/§61/§199–§200/§248/§251–§252/§346/§348） | ✅ | `variables.rs`：12 个保守片段（含 §199 样例断言）；**值从不采集**（结构无 value 字段 + JSON 断言）；probe 原始输出经 `prism_core::privacy::redact_text`；中央隐私层复用（M8 redact 已下沉 prism-core，§346） |
| Probe 超时/取消/有界并发/失败隔离（§115–§117/§146–§150/§308–§311/§392–§394） | ✅ | probe 5s/个 + 总预算 30s（耗尽 ⇒ NotProbed + 诊断，不失败）；max_parallel_probes=4；单工具失败不影响其余（discover_tool 不返回错误）；取消返回部分快照（service 测试）；EnvironmentCapability 模型（§147）+ Unsupported ⇒ None/诊断（§307） |
| EnvironmentService 独立运行；依赖方向无环；M8 可消费（§4/§264–§268） | ✅ | prism-env 依赖 core/project/git 方向（workspace Cargo.toml）；prism-verify 未依赖 prism-env（消费留 M9b+，方向合法）；`prism env .` 独立于 M8 运行 |
| `prism env` + `--json` 基础输出；exit code 与 strictness 已文档化（§240–§242/§295–§297） | ✅ | `env_command.rs`：stdout 仅数据；exit 0/1/3；--strict 文档化（cli.rs help + 本审计）；--all-tools（§270）；5 个 CLI 契约测试 |
| 本篇范围单测（§196/§198–§200/§203–§205） | ✅ | tool discovery（found/missing/probe-failed/profile）、version comparison（§204 四组）、matching 四态、sensitive 分类（§199）、redaction（无值结构断言 + 中央层单测）、path normalization（§192/§193）、provenance（lockfile/字段来源）、fixtures（node/generic/venv/unsatisfied） |
| 无 P0；遗留 P1 记录并带入 M9b | ✅ | 见下表 |

真实运行记录（Windows 11，2026-10-03）：

- `prism env .`（Prism 自身，polyglot）：node v24.19.0 / npm 11.17.0 / pnpm 12.8.1 / corepack 0.35.0 / git 2.55.0 / cargo 1.99.0 / rustc 1.99.0 / rustup 1.29.1 全部 OK；bun/yarn 如实 NOT FOUND；PATH 27 条（4 重复 3 缺失）；74 个变量名（2 敏感名，无值）；16 cpu / 31.2 GiB / 磁盘可用；scan=Completed。
- `prism env <pin 项目>`：packageManager=pnpm@10.17.0 ⇒ pnpm shim 经 corepack 报告 pinned 版本，匹配 Satisfied（evidence 三段式）；--strict 同场景 exit 0；PATH 清空场景 ⇒ §105 Unsatisfied + exit 1；路径不存在 ⇒ exit 3。
- pnpm 冷启动 probe 超时的一次真实运行 ⇒ `PROBE FAILED / version unknown` + 匹配 Unknown + 默认 exit 0（§157/§242 现场验证）。

## 遗留 P1（带入 M9b）

1. **§110 symlink awareness**：只做规范化表示，未记录 resolved_path/original_path。
2. **§111 permission checks**（readable/executable/writable）未实现。
3. **§119 progress**：GUI/IPC 进度事件属 M9b IPC 面；CLI 扫描受预算约束（≤30s）。
4. **§160–§162 shell 环境差异探测**：v1 仅 Process scope（已标注）；User PATH（§165）平台 API 未接入。
5. **§155 probe batching**：dotnet --info 式单命令多事实仅用 --list-sdks；通用 batching 延后。
6. **§183/§184 Java/Android、CUDA/GPU**：按"依项目实际需求"延后；GpuInfo 接口未建模。
7. **需求 provider 仅 packageManager 字段**：per-生态需求识别与 source 层级/冲突即 M9b 主体（§42–§45/§71–§77，按拆分表归属）。
8. **EnvironmentCapability**：模型就位，但未按能力逐项 gate 采集器（当前以 None/诊断降级表达）。
9. **§205 fixtures**：conflicting requirements / port conflict fixtures 属 M9b 需求冲突与动态事实域。

## 红线自查

- 只读：无 install/upgrade/修改 PATH/切换版本/启停（§1/§176/§319/§388–§389）——代码路径无任何写操作。
- 无网络/遥测/AI/数据库（§253–§255/§384–§386）——prism-env 无网络依赖，无存储。
- allowlist：probe 只来自 `TOOL_ALLOWLIST`（§113），测试断言参数全部 version 形态。
- 全盘扫描禁令（§25/§334）：只查 PATH 目录 + 项目根单层文件。
- 终端注入（§343）：渲染前 sanitize 剥控制字符。

结论：**M9a 阶段性完成门槛通过**，按门槛指令以 `feat: add environment facts and matching (M9 part 1/2)` 收口（不 push），随后进入 M9b。
