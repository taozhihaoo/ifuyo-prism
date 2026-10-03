# Prism M7 Final Health Audit

日期：2026-10-04 ｜ HEAD：见 git log（最终 audit 提交） ｜ 规格：`Spec/Prism M7a`+`M7b`
（M7a gate 自查见 `docs/audits/M7a-gate-check.md`；本文档为 M7 最终验收 §250/§251。）

## A. Baseline

- **FACT（CONFIRMED）**：M0 Severity = Info/Warning/Critical 三级、Confidence 独立三级——M7 复用未造第二套（§11）。FindingId 在 M0 ids 预留。
- **FACT（CONFIRMED）**：M1-M6 全部可消费；M3 `graph.cycles` 已预计算 SCC（ARCH001 直接复用，§132）；`Dependency.source→target`=依赖方向（§135 fixture 验证）。
- **RESOLVED**：M3 `CallEdge.caller_id` 格式缺陷（M6 修复 b4c5b23）。

## B. Health Architecture

- **FACT**：`prism-health` crate（model/derived/engine/registry/rules/render/config 归 core）。M3 owns relationship facts / M5 change facts / M6 impact facts / M7 health findings——M7 只消费（§175/§176）。无 M6→M7 反向依赖（§177）。引擎顺序执行 + collect-sort-dedup（§95–§98）；无全局状态（§204）。

## C. Finding Model

- **FACT**：确定性 `finding:{rule_id}:{stable_key}`（§6/§198）；category/severity/confidence/location(+multi)/related_files/symbols/modules/evidence/status/suggested_action/metadata/change_relation 全字段；§78/§200 JSON 契约字段齐备（CLI 测试断言）。

## D. Rule Engine

- **FACT**：registry 13 条内建规则按 id 排序；precedence CLI>config>default（§221 单测）；单规则失败→诊断+failed_rules+Partial（§163 单测）；per-rule/global caps→Truncated+计数保留（§223/§224 单测）；取消→Err(Cancelled)（§102/§160 单测 + §103 合成 5000 findings 测试）；确定性三次运行逐字节一致（§161 单测 + CLI 测试）。

## E. Architecture Rules

- **FACT**：ARCH001 环（M3 SCC，canonical，A↔B 一条不重复、无环不报——e2e §149）；ARCH002/003 fan-out/in 阈值边界精确（16>15 报、fan-in 16≤20 不报——e2e §150）。ARCH004/005（需用户配置）**未实现，推迟**（§113/§134 不臆测架构）。

## F. Dependency Rules

- **FACT**：DEP001 unresolved（requested-target evidence，e2e §32）；DEP002 ambiguous（§33）。DEP003 同源 ARCH001；DEP004/005 推迟。

## G. Code Rules

- **FACT**：CODE001 复杂度（算法文档化：1+分支词计数，§37）；CODE002 函数行数（threshold 覆盖 500 ⇒ finding 消失——e2e §218）；CODE003 类型行数；CODE004 花括号嵌套（Python NotApplicable，§115）。均为词法近似、非完整静态分析器（§36）。

## H. Security Rules

- **FACT**：SEC001 凭证形态赋值（占位词跳过 §110）；SEC002 PEM（Critical/High，§109）。**Redaction 验证**：`sk-live` 值不出现在序列化输出，evidence 一律 `[REDACTED]`（e2e 断言 §46/§244）。无 auto-delete（§227）、无网络（§229）。

## I. Maintenance Rules

- **FACT**：MAINT001 TODO/FIXME/HACK/XXX（大写整词+注释引导符+非 URL）；噪声由 cap/过滤承担（§111）；§154 五类负例单测。

## J. Documentation Rules

- **FACT**：DOC001 命令漂移（package.json scripts 对账 + fenced/inline-code 扫描，Node 项目门控 §114；e2e §245）；DOC002 路径漂移（markdown 链接+fenced，散文不扫 §112/§53）。措辞 "appears inconsistent"（§215）。

## K. Configuration

- **FACT**：单一 `prism.config.json` `health` 节（§70）；deny_unknown_fields（threshhold 报错单测 §72）；invalid config ⇒ CLI fail-loud exit 1（契约测试）；precedence disabled_rules > rules.<id>.enabled > default > CLI（§221 单测）。

## L. Scope

- **FACT**：FullProject / ChangedFiles / ImpactedItems；scope 由 ChangeSet/ImpactAnalysis 决定不扩大（§191）；finding.change_relation = DirectlyChanged/Impacted/Unrelated（§137）；e2e：--changed 下 scope=impacted_items 且计数正确（§247）。

## M. Change Integration

- **FACT**：CLI `--changed` 构建 M5 ChangeSet + M6 ImpactAnalysis 并按 ImpactedItems 过滤（§193/§194/§242 链路：changes → impact → health findings 全本地无 AI）。

## N. CLI

- **FACT**：`prism health .` / `--json` / `--changed` / `--rule` / `--category` / `--fail-on critical|warning` / `--format text|json|markdown|html` / `health rules [--json]`。exit 0=成功（findings 不失败 §79）、1=引擎失败或 --fail-on 命中、3=路径无效、4=取消。9+5 契约测试。

## O. GUI

- **FACT**：Health 页真实功能：summary（severity/category/change-related 计数）、过滤 chips（severity/category/current changes §141/§142）、findings 列表 50 条封顶+View all（§143/§225）、展开明细（What/Why/Evidence/Confidence §83）、Analysis Limitations 卡（§131）、卸载取消（§102）。14 前端测试；vite/tsc 构建通过。tauri IPC 四命令镜像既有模式。

## P. Reports

- **FACT**：JSON（契约 §200）、Markdown（§89 分类小节+limitations）、HTML（静态离线单文件 §90）；redaction 继承模型层（SEC 一律 [REDACTED]）；确定性：同输入同输出（§146，确定性单测覆盖 findings+ordering；报告无时间戳字段）。

## Q. Tests

- **FACT**：全工作区 **347 Rust 测试**（33 套件，含 prism-health 25 单测 + 3 health_e2e + 8 rules_e2e + 5 health_e2e-cli… 具体为 engine 17 + model/derived/render/config/registry 单测 + health_e2e 3 + rules_e2e 8 + CLI health 契约 14）**0 失败**；clippy 0；前端 **14 vitest** 通过；tsc/vite 构建通过。
- **FACT**：real projects（§167）：Prism 自身 + TS/Py/C# fixture 全部验证。

## R. Performance（§185/§186 实测，Windows debug 构建）

- **FACT**：Prism 自身（65 modules/103 deps/139 calls，13 rules，80 findings）：`prism health .` = **0.365s**；`--changed` = 0.783s（含 ChangeSet+Impact）；小 fixture = 0.049s。DerivedFacts 一次读取共享（§91）避免 13×全扫。峰值内存未测量（无失控迹象）。

## S. Privacy / Security

- **FACT**：无网络/遥测/AI/源码上传/运行时 tracing/Git mutation/auto-fix；secret 值被 [REDACTED]（e2e 断言输出不含 `sk-live`）；fixture 全部假值（§233）；日志不含源码/secret（§184）。

## T. Git

- **FACT**：工作树干净（audit 提交后）；无临时报告/生成文件误提交；未 push。

## U. Risks（仅 P0/P1）

- **P1**：SEC001 词法限制——camelCase 键名（AuthToken）不匹配；复杂度计入字符串字面量关键词（文档化近似）。均为保守方向（漏报优于误报），后续可加键名词表。
- **P1**：ARCH002/003 阈值语义需在 config 文档中更醒目（代码已文档化）。
- **无 P0**。无阻塞 M8 的 P1。

## V. Deferred（不偷偷实现）

- M8 Verify（build/test 执行）、M9 Environment（完整环境快照；ENV001 留 M9）、M10 Context（findings→ContextPack 选择）、M11 Snapshot（FindingId 跨运行对比 + first/last_seen）、M12 UI polish、M13 hardening。
- M7 内部推迟：ARCH004/005（需用户配置 schema）、DEP004/005、CODE005、CONF001-003、GIT001-003、rule 并行执行、SEC entropy 检测。

---

## M7 Completion Gate（§251）

M0-M6 回归 ✅（347 测试）｜Finding 稳定/evidence/双维度 ✅｜registry/config/determinism/cancellation/partial ✅｜cycle/unresolved/ambiguous/config-arch(裁剪声明)/complexity/large-function/nesting/maintenance/secret(保守+redaction 验证)/doc-drift ✅｜Full/Changed/Impacted scope + change-linked ✅｜CLI/JSON/Markdown/HTML/GUI ✅｜Prism self-scan + TS/Py/C# + real validation ✅｜无 AI/网络/遥测/数据库/运行时/自动修改/Git mutation/secret 泄漏 ✅｜无 P0 ✅｜无阻塞 P1 ✅
