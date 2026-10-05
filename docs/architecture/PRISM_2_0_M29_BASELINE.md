# PRISM_2_0_M29_BASELINE.md — M29 现状审计矩阵（2.0-M29 §5）

> 2026-10-06。逐域标注 Current / Source / Action（RETAIN/EXTEND/MIGRATE/
> REFACTOR/DEFER）；不以"架构不够漂亮"为由重构（§5）。

| Domain | Current Implementation | Data Source | Persistence | CLI | GUI | MCP | Test | Cache | M29 Action |
|---|---|---|---|---|---|---|---|---|---|
| Project dependency | M25 semantic relations（ProjectReference 未抽取——csproj 不在 M23/M25 解析面） | .cs 语法/索引 | 内存 IntelBundle | `intel --domain semantic` | 无 | 无（MCP=待建） | 有（M25） | extraction hash | **EXTEND**：prism-supply 解析 csproj ProjectReference |
| Package dependency | M22 l0 声明抽取：package.json/Cargo.toml/requirements.txt（csproj **未**覆盖）；declared≠resolved 已分离（§27/§77） | manifest 文本 | 内存 | `intel --domain structural` | 无 | 无 | 有（l0） | 无（delta 已备） | **EXTEND**：prism-supply csproj/packages.lock.json/Directory.Packages.props/global.json |
| Lock file | M22 只记录存在（5 种），不解析内容 | 存在事实 | 内存 | 同上 | 无 | 无 | 有 | 无 | **EXTEND**：解析 packages.lock.json（resolved）+ Cargo.lock（name/version 行扫） |
| License | 无 | — | — | — | — | — | — | — | **NEW**：LicenseFact（项目自身 manifest 可得；依赖侧离线=UNKNOWN，§21）+ policy 判定 |
| Security | 无漏洞/SCA 能力 | — | — | — | — | — | — | — | **NEW**：AdvisoryProvider（本地 JSON 库，offline-first）+ 匹配规则 |
| Secret detection | M9 envfiles：.env key 名分类 + SensitiveFileTracked finding（不读值） | .env 文件 | SQLite(env) | `env` | EnvironmentPage | 无 | 有 | 无 | **EXTEND**：源码 SecretCandidate 扫描（pattern+entropy+context，redaction 全链） |
| Environment | M9 prism-env：19 工具 allowlist 探测（含 dotnet）/RequirementProvider 7 层/EnvironmentSnapshot | probe（allowlist 命令） | SQLite(env) | `env` | EnvironmentPage | 无 | 有 | 无（每次采集） | **EXTEND**：Declared(vs) Observed 对比 + Capability + ToolchainDrift |
| SDK/toolchain | global.json/TargetFramework/project.godot features **未**解析 | — | — | — | — | — | — | — | **NEW**：声明抽取 + 与 observed 对比 |
| SBOM | 无 | — | — | — | — | — | — | — | **NEW**：prism-sbom/1 确定性导出（SPDX/CycloneDX DEFER，§24） |
| Findings | M28 prism-health2：Finding=prism_intel::Finding SoT + RuleRegistry + 生命周期五态 + Gate 四态 | 注入事实 | 报告 JSON 链 | `health2` | 无 | 无 | 20 项 | 报告基线 | **RETAIN + EXTEND**：新增 Dependency/Security/License/Environment category 与规则 |
| Quality Gate | M28 evaluate_gate（PASS/WARN/FAIL/UNKNOWN） | LifecycleDiff | 同上 | 同上 | 无 | 无 | 有 | 同上 | **RETAIN**（M29 policy 只是其 Policy 数据，§65） |
| Snapshot | M11b 七域捕获（无 M29 域） | AnalysisBundle | SQLite | snapshot/compare | SnapshotHistoryPage | 无 | 有 | 指纹 | **DEFER**（P1：M29 域入 snapshot） |
| Context | M27 evidence collector（语义边） | IntelBundle | 内存 | `context --impact-aware` | ContextPage | 无 | 有 | 无 | **DEFER**（P1：M29 findings 进候选） |
| Persistence | prism-store SQLite（snapshots/env domains）；IntelBundle 不持久 | — | SQLite | — | — | — | 迁移测试有 | — | **DEFER**（P1：沿 M28 先例，报告 JSON 链为 v1 持久化） |

## 结论

- 骨架（Finding/Registry/Gate/Lifecycle/证据/取消/有界读取）M28 全部就位——M29 增量 = **事实供给**（dependency/security/license/env 声明与扫描）+ **规则挂载**，不建第二套任何模型（§98）。
- Godot+C# 第一优先（§83）：csproj / packages.lock.json / Directory.Packages.props / global.json / project.godot features 本里程碑必须真实解析。
- 明确不做（§2/§115-§117）：自动修复/升级/安装、远程扫描、LLM 解释、M210-M212 提前实现。
