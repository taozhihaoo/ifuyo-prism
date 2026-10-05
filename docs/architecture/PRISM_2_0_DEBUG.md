# PRISM_2_0_DEBUG.md — Debug Intelligence 契约（2.0-M210）

> 2.0-M210 产物。§七十四 的九份文档（DEBUG_MODEL/PARSERS/STACK_TRACE/
> ERROR_CLUSTERING/DEBUG_REGRESSION/DEBUG_SECURITY/TEST_MATRIX）按既有
> 惯例合并为本契约 + `docs/audits/2.0-M210-final-audit.md`（Final
> Report）。命名约定：文档文件名 M210；审计/提交写 `2.0-M210`。

## 1. 定位（§一/§二/§五）

`prism-debug` crate（第 22 个成员）+ `prism-health2` Debug 规则 + CLI
`prism debug`。回答：发生了什么 / 在哪里 / 可能来自哪里 / 哪些是同一问题 /
证据是什么 / 修改后是否复现。

- **Evidence Intelligence，不是文本搜索**（§五 原则 1）：
  `Raw → Parser → Event → Error(签名) → Cluster → Frame → Symbol/Scene →
  Finding → Gate`。
- **不建第二套模型**（§二.10/§二十三/§二十六）：Finding/Gate/生命周期 =
  M28 原样；脱敏复用 M29 `secrets::redact_line`（§二十八 不重建扫描器）；
  Symbol = M23；Scene/Node = M24。
- **Unknown 一等公民**：无日志 ≠ 无错误（§二.7，CLI/规则双层显式
  "unavailable, not zero"）；Unsupported parser 不产"无错误"；栈存在但
  解析不到 Symbol 也是有效数据（§十）。
- **Root Cause 边界**（§六十七/§六十八）：v1 不产 RootCause——无充分
  证据时 Unknown 比猜测正确；时间接近 ≠ 因果（§十八）。

## 2. 数据模型（§六-§十）

```text
DebugSource  { source_id=src:{fnv(逻辑路径)}, source_type, logical/physical_path,
               content_hash, parser, parse_state, redactions, diagnostics }
DebugEvent   { event_id=evt:{fnv(source|line|normalized)}, source_id, line,
               timestamp?, level(标准化)+original_level, domain(Runtime/Build/
               Test/Import/Unknown), message(redacted), test_name?, code?, file? }
DebugError   { error_id=err:{fingerprint}, exception_type?, normalized_message,
               example_message(redacted), code?, top_frames(≤3), stack_trace?,
               occurrences, occurrences_detail(≤5 例证), first/last_observed }
StackTrace   { trace_id, frames: Vec<StackFrame> }
StackFrame   { module/type/method/file/line/column, resolution: FrameResolution }
ErrorCluster { cluster_id, error_ids, occurrences, explanation }
DebugRun     { run_id, run_type, source_ids, event_count }
```

- **三层身份**（§九）：Event（一次发生）/ Error（规范化签名）/ Cluster
  （同源分组）。指纹 = `fnv1a(domain|exception|normalized message|code|
  top frames)`——无 GUID/时间戳/行号（测试锁定：行号与具体数字不改变指纹）。
- **规范化只进指纹**（§八）：路径→`<PATH>`、IP/版本/数字→`<N>`、GUID→
  `<ID>`、长 hex→`<HEX>`；反例测试锁定不同问题不合并。
- **重复抑制**（§四十六）：1000 次异常 = 1 签名 = 1 finding + occurrences。
- ParseState（§三十三）：Complete/Partial/Malformed/Unsupported——空源
  Complete + "empty" 诊断；无标记文本按 Unknown 级事件如实记录。

## 3. 解析器（§三十-§三十三）

手写逐行、零 regex 依赖；`detect_parser` 内容嗅探路由；全部经
`redact_line`（§二十九）与 ANSI 剥离；malformed ⇒ 诊断不伪装。

| Parser | 形态 | 说明 |
| --- | --- | --- |
| generic-text-log/1 | `[LEVEL]`/`LEVEL:`/裸词级别 + ISO 时间戳前缀 | 无级别行并入上一条（多行消息）；续行独立成行 |
| dotnet-exception/1 | `Unhandled exception. TYPE: MSG` + `at Frame in path:line` + `---> inner` | 嵌套异常各成签名；帧行并入消息供帧解析 |
| godot-log/1 | `ERROR:`/`SCRIPT ERROR:`/`WARNING:` + `at: Func (res://path:line)` | 引擎横幅只是识别线索；`at:` 行恒为帧续行 |
| build-output/1 | `path(l,c): error CS0246: msg` + `error[E0308]:` + ` --> path:l:c` | cargo 位置行回填最近错误事件；code/file/line 结构化 |
| test-output/1 | `Failed X` / `FAIL x` + Expected/Actual/Exception 详情并入 | test_name 结构化 |

## 4. Symbol / Godot 关联（§十一/§十二）

- 优先级：文件 + 方法 + 行号在声明范围 ⇒ **Exact (FACT)**；文件 + 方法 ⇒
  **Strong**；仅方法名全局唯一 ⇒ **Probable (INFERENCE，依据写明)**；多候选
  ⇒ Ambiguous 不猜；引擎未注入 ⇒ Unavailable。
- Godot：**仅** M24 `attached_script` 证据映射 脚本→节点→场景（§十二
  禁止类名/文件名猜测）；`res://` 前缀即项目相对路径。

## 5. Clustering / Regression（§十七/§二十一/§三十九）

- v1 = 精确指纹分组（保守）；`ClusterExplanation` 逐项列出
  same_exception_type/message/top_frames/code + reasoning；**相似度打分 =
  Unsupported**（不产出无解释的 93.17%）。
- Regression = M28 生命周期原样（`--baseline` 链：New/Persistent/Resolved/
  Regressed/Suppressed）；"第一次观察到 ≠ 第一次发生"——metadata 用
  first/last_observed（观测范围内）。

## 6. Findings / Metrics（§二十四/§六十九）

| 规则 | 严重度 | 条件 |
| --- | --- | --- |
| `DEBUG/REPEATED_RUNTIME_ERROR` | Warning | Runtime 签名次数 ≥ 阈值（默认 10） |
| `DEBUG/CRASH_SIGNATURE` | Critical | 原文含 `Unhandled exception.` |
| `DEBUG/BUILD_FAILURE` / `DEBUG/TEST_FAILURE` | Warning | 域 = Build / Test |
| `DEBUG/UNRESOLVED_STACK_FRAME` | Info | 符号引擎在场仍解析不到的帧 |

Metrics：`debug.unique_runtime_error/build_failure/test_failure.count`、
`debug.stack_frame.total/resolved`（§七十 覆盖率分子/分母口径写进
definition）。空源 ⇒ 诊断 "analysis unavailable (not zero errors)"。

## 7. CLI（§五十）

```text
prism debug <path> [--source <file>...] [--baseline <file>] [--json]
```

- 自动发现 `*.log` 与轮转片段（§三十五：`app.log.1` 归并逻辑源 app.log）；
  单源 8 MiB / 20 源 / 50k 事件上限（§四十七，截断可见）。
- 文本输出 = §50 形态（Sources/Events/unique errors/clusters/unresolved
  frames/findings/lifecycle/gate + diag）；数值全部来自真实分析。

## 8. 边界与遗留

- GUI（§五十二）/MCP（§五十三）/SQLite（§四十九）/Snapshot 域（§三十八）/
  M27 Context 候选（§四十）/M26 Impact 接线（§四十二）：DEFER（P1，见
  Final Audit §36）；库形态（DebugReport/Health2Report）即共享服务面。
- 不做（§四）：API/网络捕获、部署、自动修复、APM/SIEM/SAST、遥测。
- 日志 = 不可信输入（§二十八/§五十五）：DATA 非指令；redaction 在解析
  层强制（事件消息绝不存原文）。

## 9. 实测证据

- prism-debug 25 项单测（嗅探/各级别/续行/ANSI+redaction/嵌套异常/双形态
  帧/MSBuild+cargo/测试输出/空源与 malformed/轮转/位置格式/规范化反例/
  聚类确定性/指纹跨行号稳定/截断可见/Exact/Strong/Probable/Ambiguous/
  Unavailable/Godot 映射）。
- health2 +6 项（前置诊断/50 次=1 finding/Critical/低于阈值不产/空源
  not-zero/build+test 分离）。
- e2e（§六十二 Godot+C# fixture）：12× SCRIPT ERROR NullReferenceException
  ⇒ 1 签名 ⇒ REPEATED_RUNTIME_ERROR Warning；帧 **Exact** 解析
  `cs:App.Battle.BattleController.ResolveCard#method#…`；M24 证据 ⇒
  `Scenes/Battle.tscn` + `/BattleRoot`；baseline 轮 persistent=1/
  regressed=1 + Pass；空项目 ⇒ Unavailable。
- 全量：clippy 0 / fmt 0 / **727 Rust**（M29 末 695 + 32）+ 55 vitest。
