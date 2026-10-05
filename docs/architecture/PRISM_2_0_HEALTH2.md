# PRISM_2_0_HEALTH2.md — Health & Engineering Findings 2.0 契约（2.0-M28）

> 2.0-M28 产物。四者分离是本文档的唯一主题：**Finding 是分析结果；Metric
> 是测量结果；Health 聚合是视图；Quality Gate 是策略判断**（§3/§21）。
> 命名约定：文档文件名 M28；审计/提交写 `2.0-M28`。

## 1. 定位与边界

- prism-health2 crate：消费既有引擎事实（L0 清单行数 / M23-M25 C# 抽取与
  语义 / M24 Godot IR），产出 Metric + Finding + Evidence 报告。
- **不建第二套 Finding 模型**（§90 禁止 1）：唯一 SoT = M21
  `prism_intel::Finding`（severity/confidence/status 三轴分离）。
- 不吞并 Debug/Security/Dependency Intelligence（M29/M210 域，§2）。
- 无单一神奇 Health Score（§90 禁止 2）；无评分伪装（§21/§38）。
- 纯内存分析：无 shell、无网络、无遥测；有界读取（单文件 4 MiB 上限）。

## 2. 模型（§5-§7/§20-§23）

```text
Metric          { key, value: u64, unit: String, definition, producer }
Health2Report   { schema_version=1, metrics: BTreeMap, findings, evidence, diagnostics }
LifecycleDiff   { new, persistent, resolved, regressed, suppressed }
QualityGatePolicy { max_new, max_breaking }        // 默认 0 容忍
QualityGateResult { verdict, new_count, breaking_count, reasons }
GateVerdict     { Pass | Warn | Fail | Unknown }
```

- Metric 有值/单位/可解释定义/producer（§12：无黑盒；没有数据不伪造）。
- `Health2Report.evidence`：证据库（M21 `Evidence`，内容寻址 `ev:*`）；
  finding.evidence 引用其 id——每个正式 Finding 都有可追溯证据（§37）。
- Finding id：M21 内容寻址、无行号先例（`find:{hash16(type|title|target)}`；
  行内移动不换 id；符号重命名 ⇒ 新 id，如实）。

## 3. Rule 基建（§89 Phase C，"独立但不过度抽象"）

```text
RuleDefinition { id, name, category }                  // Structural|Complexity|Semantic|Godot
RuleOutcome    { findings, metrics, evidence, diagnostics }
CSharpFacts    { extractions: &BTreeMap<String, FileExtraction>, semantics: Option<&SemanticEngine> }
RuleContext    { files, thresholds, observed_at, csharp: Option<_>, godot: Option<&GodotAnalysis>, cancel }
RuleFn         = fn(&RuleContext) -> RuleOutcome        // 纯函数；不扫描文件系统、不重跑分析器
RuleRegistry   .register(def, fn) / .with_builtins() / .evaluate(ctx) -> Health2Report
```

- 评估按规则 id 确定性排序（§41）；输出 Finding 按 id 排序、Evidence 按 id
  排序——同输入 ⇒ 同报告（指纹层，observed_at 除外）。
- 事实未注入（如非 C# 项目）⇒ 规则记 diagnostics 跳过，**不产出空结论**
  （Unsupported 正确处理）。
- 取消：registry 逐规则检查 + 规则内长循环自查；保留已完成部分并如实记
  诊断（partial 不伪装 success）。

## 4. 内建规则（第一批，§A-§D 裁剪到真实能力）

| 规则 id | 类别 | 输入 | 产出 | 默认阈值 |
| --- | --- | --- | --- | --- |
| `STRUCT/LARGE_FILE` | Structural | L0 清单行数 | Warning finding + File 证据 + files_total/lines_total/large_files | >1000 行 |
| `SYMBOL/SIZE` | Complexity | M23 抽取 range | `COMPLEX/LARGE_TYPE` / `COMPLEX/LARGE_METHOD` + max_type_lines/max_method_lines | 类型 >400 行 / 方法 >100 行 |
| `SEMANTIC/<kind>` | Semantic | M25 诊断 | 按 (kind,file) 聚合 finding（爆炸防护组上限 200，每组 ≤5 条 SourceRange 证据）；ambiguous_reference=Warning，其余=Info | — |
| `GODOT/BROKEN_WIRING` | Godot | M24 IR | `GODOT/MISSING_SCRIPT` / `GODOT/MISSING_INSTANCE`（Warning）+ `GODOT/MALFORMED_SCENE`（Info）+ scenes/nodes 计数 | — |

阈值结构 `RuleThresholds { large_file_lines, large_type_lines,
large_method_lines, max_semantic_groups }`——可配置，不写死裸数字策略（§11A）。

## 5. 生命周期（§23-§33）

- 指纹集 = finding id 集合；`diff_against_baseline(current, baseline)`：
  New / Persistent / Resolved。
- `mark_regressed(diff, baseline_new_ids, current)`：基线轮标记为 New 且当前
  仍存在 ⇒ Regressed（未修复确认）。baseline_new_ids 来自基线文件自带的
  `lifecycle.new`（上次 CLI 输出）。
- **Suppression**：当前侧 status=Suppressed 的 finding 归入
  `suppressed_findings`，不计 New/Persistent，不因消失记 Resolved
  （M21 语义：用户显式忽略，不可自动复活）。Gate 计数同样排除。
- History：v1 = 基线文件链（每次输出可作下次基线）；SQLite 持久化为
  后续项（见 Final Audit §20）。

## 6. Quality Gate（§29-§30/§65）

- 策略判断与引擎分析分离：`evaluate_gate(Option<&LifecycleDiff>,
  breaking_count, policy)` **不重新执行分析器**。
- 判定阶梯：超预算 ⇒ FAIL；预算内非零 ⇒ WARN；全零 ⇒ PASS；
  无对比依据（基线 schema 不兼容）⇒ UNKNOWN（不伪造结论）。
- 无基线的首轮 = 与空基线对比 ⇒ 全 New，可判定（FAIL if > max_new）。
- breaking_count 由调用方按类别统计注入（v1 CLI 恒 0——无 breaking 类规则）。

## 7. CLI（§73 Vertical Slice）

```text
prism health2 <path> [--baseline <file>] [--max-file-lines N] [--json]
```

- 基线文件 = 上一次 `prism health2 --json` 的完整输出（报告字段 +
  lifecycle；serde 忽略未知字段，可直接回喂）。schema 不匹配 ⇒ Gate UNKNOWN。
- 单次有界读取同时喂行数统计与 C# 引擎（不二次读盘）；Godot 文件存在才
  调 M24 Adapter。
- 退出码沿 CLI 契约：0 成功（findings 不导致失败，沿 1.0 §79 先例；
  非零 fail-on 语义留后续）。

## 8. 实测证据（本契约写入时的 Gate）

- prism-health2：13 lib + 7 integration 测试（阈值/确定性/生命周期
  new-persistent-resolved-regressed-suppressed/Gate 四态/序列化/三规则）。
- 端到端（fixture：500 行类型 + dynamic 参数 + 场景悬挂脚本）：
  首轮 3 finding 全 New + Gate FAIL ⇒ 二轮 persistent=3/regressed=3 +
  PASS ⇒ 修复后 resolved=1 + PASS。
- 全量回归：76 suites / **654 Rust**（M27 末 634 + health2 新增 20）+
  55 vitest；clippy 0；fmt 0。

## 9. 后续（不在 M28 裁剪面内）

- SQLite Finding history / Snapshot 内嵌 health2 摘要（§34：复用既有
  Snapshot，不建第二套 Health Snapshot）。
- Git Change → Findings Regression 直连（当前经基线文件两轮对比达成）。
- fail-on / policy 文件化配置；GUI/MCP 只读查询面（MCP = 新建子系统，
  按矩阵排期）。
