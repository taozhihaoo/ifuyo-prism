# PRISM_2_0_M29.md — Dependency / Security / Environment Intelligence 契约（2.0-M29）

> 2.0-M29 产物。§109 的七份文档按既有惯例（§73 允许）合并为本契约 +
> `PRISM_2_0_M29_BASELINE.md`（现状矩阵）+ `docs/audits/2.0-M29-final-audit.md`
>（Final Report）。命名约定：文档文件名 M29；审计/提交写 `2.0-M29`。

## 1. 定位（§1/§2/§100）

`prism-supply` crate（第 21 个成员）+ `prism-health2` 供给规则 + CLI
`prism supply`。M29 让 Prism 从"理解代码本身"扩展到"理解代码依赖的整个
工程运行条件与供应链"。明确不做（§2/§115-§117）：自动修复/升级/安装、
远程扫描、LLM 解释、Debug/Git-Evolution/Visualization 提前实现。

- **不建第二套任何模型**（§3/§98）：Finding = M21 SoT；规则挂 M28
  RuleRegistry（§47）；生命周期/Gate = M28 原样复用（§53/§59/§60）；
  policy 只是 Gate 的数据（§65）；Impact 沿 M26（§51，本里程碑未接线=P1）。
- **Unknown 一等公民**（§105）：advisory 库不可用 ⇒ `Vulnerability
  analysis = Unavailable`，绝不显示 0 漏洞；依赖侧 license 离线 =
  UNKNOWN；声明了但未观测到工具 ⇒ Missing（不是 Error）。
- **offline-first**（§72/§73）：零网络、零 shell 新增（工具探测沿 M9
  allowlist 体系）；advisory/provider 走 trait 边界，Core 不见 HTTP。

## 2. Dependency Intelligence（§7-§19）

```text
DependencyDeclaration { id="{ecosystem}:{package}", requested_version,
                        version_is_range, kind, source, scope,
                        manifest_path, line, version_source }
ResolvedDependency    { id, resolved_version, direct, from_lockfile, lockfile_path }
VersionComparison     { state: DriftState, detail }
DriftState            { NormalResolution | Expected | Drift | Mismatch | Conflict
                      | Unresolved | Unknown }        // §11：不自动判错
```

- **Declared ≠ Resolved 是核心设计**（§9）；解析状态不是"失败=没有"（§17）。
- **CPM**（§13）：`Directory.Packages.props`（须 `ManagePackageVersionsCentrally
  =true` 才视为中央版本）补全无内联版本的 PackageReference；
  `version_source = central` 如实标注。
- 解析范围按真实生态裁剪（§12）：packages.lock.json（serde，Direct/
  Transitive）+ Cargo.lock（行扫去重）；npm/pnpm/yarn 锁只记录存在（M22
  先例）。malformed 锁 ⇒ Err 诊断，不是空结果。
- **ProjectReference**（§15/§16）：`..\` 路径规范化为项目相对路径；
  环检测 DFS（深度 32、cap 10，旋转归一到最小元素开头，证据 = 环路径
  `A → B → C → A`）。
- 身份（§14）：`{ecosystem}:{package_id}` 内部结构化；
  `nuget:X@13.0.3` 只是展示形式。

## 3. License（§20-§22）

- `LicenseFact { subject, expression, spdx_known, source }`：来源 = 项目
  自身 manifest（Cargo.toml `license`、package.json `license`）。
  **依赖侧元数据离线不可得 ⇒ SBOM 中记 UNKNOWN 并计数**（§21：UNKNOWN
  ≠ 没有 license）。
- SPDX 保守词表 22 项（MIT/Apache-2.0/BSD-*/GPL-*/LGPL-*/MPL-2.0/ISC/
  Unlicense/CC0/0BSD）；复合表达式按 token 全查。
- **Policy**（§65）：`LicensePolicy { blocked, allowed, require_known }`；
  判定产出 `reasoning = "fact: GPL-3.0 (manifest) × policy.blocked=[...]"`
  ——fact × policy ⇒ finding，可解释（§22）；不产生法律结论（§1）。

## 4. Security（§26-§36/§61-§62/§67-§69）

- **Known ≠ Heuristic**（§28）：只有 advisory 明确命中才产出
  `SEC/KNOWN_VULNERABILITY`；`Package appears old` 类启发式 v1 不产 finding。
- **AdvisoryProvider trait** + `LocalAdvisoryDatabase`（本地 JSON，版本化；
  匹配 = 受影响范围与解析版本相交 ⇒ "Potentially affected"，§30）。
  Provider 不可用 ⇒ 规则记诊断跳过（§48），**绝不等于 0 漏洞**。
- **SecretCandidate**（§32-§36）：候选而非确定。检测器 = 已知凭据前缀
  （AKIA/ghp_/github_pat_/xox）/凭据形态赋值（敏感词 + 长度≥12 + 含数字）/
  JWT 三段 base64url / PEM 私钥头。降级：测试/fixture/示例/docs 路径 ⇒
  Low confidence + exclusion_notes。
- **Redaction 全链**（§36/§87）：候选只含前 4 字符 + `…(N chars)`；
  finding 标题/描述/证据/e2e JSON 全文实测无原文。指纹 = fnv1a(rule|path|
  line|class)——**不含 secret 内容**（§62）。
- 资源限制（§68/§69）：单文件 256 KiB、候选 cap 200 ⇒ truncated 标志 +
  诊断；扫描面 = 调用方决定的 Source/Test 文本（§67，沿 M22 scope）。

## 5. Environment（§37-§43/§84）

- **Declared vs Observed**（§38）：声明来自 global.json（dotnet SDK）/
  csproj TargetFramework / project.godot config/features（Godot 版本提示，
  §84：仅声明，机器版本必须来自观测）。观测 = M9 allowlist 探测
  （`dotnet --version`，有界 + 取消；本机无 dotnet ⇒ raw 空）。
- **ToolchainState**（§41）：Match / RangeSatisfied / Mismatch（主版本
  不同）/ Missing（声明了但未观测）/ Unknown（不可比）——不统一
  EnvironmentError；`8.0.100 vs 9.0.100` 是 Mismatch 事实，不是 Build
  Broken 判决（§38）。
- **Capability**（§39）：CanParse / CanResolveDependencies /
  CanCompile（=Unavailable，v1 无构建集成，如实）/ CanGenerateSBOM /
  VulnerabilityAnalysis（Available|Unavailable|Unknown）。
- 与 M28 Gate 的连接（§43）：供给 findings 进 lifecycle ⇒ Gate 原样消费；
  M29 不做第二套 Gate。

## 6. SBOM（§23-§25）

- `prism-sbom/1`：确定性导出（BTreeMap + 排序数组，**无时间戳/UUID 进
  内容**，§24）；组件 `{name: eco:pkg@ver, relationship: direct|transitive,
  license: 表达式|UNKNOWN, evidence_ref: lockfile 路径}`——可追溯（§25）。
- SPDX/CycloneDX 明确 DEFER（§24：不因标准名存在就假设完成）。
- `prism supply --sbom <file>` 实测两轮逐字节一致。

## 7. Findings / Metrics（§44-§46/§54）

| 规则 id | 类别 | 产出 |
| --- | --- | --- |
| `DEP/UNRESOLVED` `DEP/VERSION_MISMATCH` | Dependency | 声明无锁 / Mismatch+Conflict（Drift/NormalResolution **不**产 finding，§11） |
| `DEP/CYCLE` | Dependency | 环路径证据 |
| `SEC/SECRET_CANDIDATE` | Security | severity 按 confidence（High⇒Critical/Medium⇒Warning/Low⇒Info） |
| `SEC/KNOWN_VULNERABILITY` | Security | severity 按 advisory hint（critical⇒Critical/high|moderate⇒Warning）；remediation：有 fixed_version ⇒ `Update to >= X`，否则泛化建议（§80） |
| `LIC/POLICY_VIOLATION` `LIC/UNKNOWN_LICENSE` | License | policy 判定 + require_known |
| `ENV/TOOL_MISSING` `ENV/TOOLCHAIN_MISMATCH` `ENV/VERSION_UNKNOWN` | Environment | Missing/Mismatch=Warning，Unknown=Info |

Metrics（§54 词表）：`dependency.declared/resolved/unresolved/cycle.count`、
`security.secret_candidate/known_vulnerability.count`、
`license.unknown/policy_violation.count`、`environment.mismatch/missing_tool.count`
——全部有 definition/unit/producer（复用 M28 Metric 模型）。

## 8. CLI（§75）

```text
prism supply <path> [--advisories <db.json>] [--policy <policy.json>]
                   [--sbom <out.json>] [--baseline <file>] [--json]
```

文本输出 = §104 最低可用产品体验形态（dependencies/security/licenses/
environment/findings/lifecycle/quality gate + diag）。退出码沿 CLI 契约
（findings 不导致失败）。基线文件 = 上次 `--json` 输出（M28 生命周期链
复用：New/Persistent/Resolved/Regressed/Suppressed + Gate 四态）。

## 9. 边界与遗留

- GUI/MCP：沿 M21-M28 先例 DEFER（MCP=新建子系统，矩阵 M27 行）。
- SQLite 持久化 / Snapshot 域 / Context 候选 / Dependency→Impact 接线：
  P1（见 Final Audit §24）。
- 完整规则族（OutdatedDependency 需版本比较基准、LockfileMismatch 深核对
  等）按 §44"规则数量不是 KPI"裁剪，规则族可经 `registry.register` 无限
  扩展。

## 10. 实测证据

- prism-supply 36 项单测（解析器/版本/漂移/环归一/secret FP 降级/
  advisory 范围匹配/SBOM 确定性/声明对照/能力派生）。
- health2 +6 项（前置诊断/unavailable≠0/secret 渲染无原文/severity+
  remediation/drift 不产 finding）。
- e2e（Godot+C# fixture）：DEP/CYCLE 1 + KNOWN_VULNERABILITY Critical
  （remediation "Update to >= 1.2.5"）+ SECRET_CANDIDATE 2 + ENV 3 ⇒
  new=7 + FAIL ⇒ 基线轮 persistent=7/regressed=7 + Pass；JSON 无原文；
  SBOM 两轮一致。
- 全量：clippy 0 / fmt 0 / **695 Rust**（M28 末 654 + 41）+ 55 vitest。
