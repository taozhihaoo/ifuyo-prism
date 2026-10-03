# Prism Health Rules — Rule Catalog (M7)

> 每条规则：ID / Title / Category / Description / Default severity / Confidence
> semantics / Inputs / Detection logic / False-positive notes / Configuration /
> Example（规格 M7 §130/§131）。全部规则确定性、离线、无 AI（§126/§127/§182/§183）。
> 阈值均为**产品默认值**（§216：记录来源为 product default，非工程法则），可用
> `prism.config.json` 的 `health.rules.<ID>.threshold` 覆盖（§218/§221）。

## MAINT001 — Maintenance Marker
- Category: Maintenance · Default severity: Info · Confidence: High
- Inputs: 代码模型文件内容（DerivedFacts 共享读取）
- Detection: 行内出现大写整词 `TODO`/`FIXME`/`HACK`/`XXX`，且 marker 之前行内
  出现该语言注释引导符（按扩展名），marker 前导字符不是 `/`（排除 URL/路径）。
- False positives: 字符串字面量中的 TODO 若同行无注释引导符则不报；`TODOS`、
  `TODOs` 等非整词不报。Markdown（无注释引导符映射）不扫描。
- Configuration: `enabled`；无 threshold。噪声由引擎 per-rule cap 与 GUI 过滤承担（§111）。
- Example: `// TODO: extract helper` ⇒ finding @file:line。

## ARCH001 — Circular Module Dependency
- Category: Architecture · Default severity: Warning · Confidence: High
- Inputs: M3 预计算 SCC（`graph.cycles`，§132 不重造）。
- Detection: 成员数 ≥ 2 的每个 SCC 输出一条 finding（canonical cycle id，§24/§25）。
- False positives: 静态循环 ≠ 错误架构（§23 措辞："detected static dependency cycle"）。
- Configuration: `enabled`。
- Example: A→B→C→A ⇒ 一条 finding，members = [a, b, c]，evidence = 成员列表。

## ARCH002 — Excessive Module Fan-out
- Category: Architecture · Default severity: Info · Confidence: High
- Inputs: M3 resolved 依赖派生的 fan-out 统计（§94）。
- Detection: 模块 fan-out > threshold（默认 15，product default）。
- False positives: 高 fan-out ≠ 设计错误（§26），仅事实信号。
- Configuration: `enabled` + `threshold`。

## ARCH003 — Excessive Module Fan-in
- 同 ARCH002，方向为 incoming（默认阈值 20）。fan-in 高 ≠ 设计错误（§27）。

## DEP001 — Unresolved Dependency
- Category: Dependency · Default severity: Warning · Confidence: High
- Inputs: M3 依赖解析事实（§34 不重写 resolver）。
- Detection: `ResolutionStatus == Unresolved` 的每条依赖。
- Evidence: source 位置 + requested target。
- Example: `import { thing } from "@shared/payment"` 无法解析 ⇒ finding。

## DEP002 — Ambiguous Dependency
- Category: Dependency · Default severity: Info · Confidence: Medium
- Detection: `ResolutionStatus == Ambiguous`。措辞 "could not be uniquely
  resolved"，不说 wrong（§33）。

## CODE001 — High Complexity
- Category: Code · Default severity: Warning · Confidence: High
- Inputs: CodeMetrics（每函数一次计算，§93）。
- Detection: `complexity = 1 + 分支关键词/操作符计数`（if/for/while/case/catch/
  &&/||/?; Python: if/elif/for/while/except/and/or）> threshold（默认 10）。
  **词法近似，非标准 McCabe**（§37 明确算法）。
- False positives: 字符串字面量中的关键词计入（文档化近似）；未支持语言
  NotApplicable（§115）。
- Configuration: `threshold`（§218，evidence 携带 value/threshold）。

## CODE002 — Very Large Function
- 函数行数（start..=end 行）> threshold（默认 200，§39 优先 source lines）。

## CODE003 — Very Large Type
- 类型（class/struct/interface/enum）行数 > threshold（默认 400，§40）。

## CODE004 — Excessive Nesting
- 花括号语言函数体内最大 `{` 深度 > threshold（默认 5）。Python 缩进块
  NotApplicable（§41/§115）。

## SEC001 — Possible Hard-coded Secret
- Category: Security · Default severity: Warning · Confidence: Medium（§48 保守）
- Inputs: 非 Generated 的全部可读文件（§49：secret detection 对生成文件单独定义，
  默认排除；§120 安全面=全文件）。
- Detection: 行含凭证键名（password/passwd/secret/token/api_key/apikey/access_key/
  client_secret/auth_token/private_token）+ `=`/`:` 分隔符 + ≥12 字符值，
  且值不含占位词（example/test/dummy/sample/placeholder/changeme/your_/xxxx）
  且非 URL（§45/§48/§110）。
- **Redaction（§46/§233）**: evidence 只存 `pattern` 与 `[REDACTED]`——值本身
  永不进入 finding/报告/日志（§244 验收）。
- False positives: camel-case 键名（AuthToken）暂不匹配（词法限制，已文档化）。
- Configuration: `enabled`。无 auto-fix（§227）、无网络查询（§229）。

## SEC002 — Private Key Material Detected
- Detection: 行含 `-----BEGIN ... PRIVATE KEY -----` PEM 标记（§109 强证据）。
- Default severity: Critical · Confidence: High。Redaction 同 SEC001。

## DOC001 — Documented Command Not Found
- Category: Documentation · Default severity: Info · Confidence: Medium
- Inputs: README.md 内容 + package.json scripts（仅 Node 项目启用，§114）。
- Detection: README fenced 块或行内代码中的 `npm/yarn/pnpm run <name>`，
  `package.json` 未定义 `<name>`（§52/§245）。
- False positives: README 可能故意引用 legacy 命令——措辞 "appears inconsistent"
  （§215），不用 "documentation is wrong"。

## DOC002 — Documented Path Not Found
- Detection: README markdown 链接（`](path)`，排除 URL/锚点/邮箱）与 fenced 块
  引用的项目相对路径在磁盘不存在（§53/§156）。普通散文路径不扫（§112）。

## 未实现/推迟（规格允许的范围裁剪，见 M7 Final Audit §V）
- ARCH004/005（forbidden/layer）：需用户配置（§28/§113/§134：不臆测架构）——配置
  schema 已预留，规则未实现（第一版目录裁剪）。
- DEP003（与 ARCH001 同源）、DEP004/005（需生态依赖模型）、CODE005、CONF001-003、
  GIT001-003、ENV001（M9）。
