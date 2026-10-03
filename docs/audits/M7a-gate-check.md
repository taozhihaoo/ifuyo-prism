# M7a Gate Self-Check — Health Foundation and Findings Engine

日期：2026-10-04 ｜ HEAD：见 git log ｜ 规格：`Spec/Prism M7a — Health Foundation and Findings Engine 完整开发提示词.md`
（M7 最终验收以 M7b §250/§251 为准；本文档只是 M7a 前置门槛检查。）

## Baseline（§1/§2）

- **FACT**：Finding 此前不存在；M0 `Severity` = Info/Warning/Critical 三级（总纲领 §11 锚定，决策 #3）、`Confidence` = Low/Medium/High 独立类型——**复用，未造第二套**（§11）。
- **FACT**：`FindingId` 已在 M0 ids.rs 预留（"必须稳定可复现"）；配置系统为唯一 `prism.config.json`（分节 + validate + schema_version）——`health` 节并入该系统（§70 只选一种）。
- **FACT**：M1-M6 事实层全部可消费（M6 刚做过全链路 e2e）；CLI `health` 占位存在；各层 diagnostics convention 一致（kind/subject/detail）。

## 门槛逐项证据

| 门槛项 | 证据 |
| --- | --- |
| M0–M6 regression（§175/§176） | 全工作区 **323 tests 绿**、clippy 0；M2 仅"选择器纳入 Test 文件"（M6 已交付）与 M0 config 增加 `health` 节两处扩展，均为加法 |
| Baseline Audit FACT/HYPOTHESIS/P0-P1 | 本文档 Baseline 节 |
| Finding model + 稳定 FindingId（§5/§6/§121–§124/§198） | `Finding{id: finding:{rule_id}:{stable_key}, locations, related_*, evidence, status}`；id 确定性单测（同输入同 id、异规则异 id） |
| Severity × Confidence 分离（§11/§12） | 复用 core 两独立类型；JSON 中独立字段（模型序列化测试断言） |
| Evidence First、不复制源码（§13/§14） | `Evidence{kind, subject, detail, location}` 六类 kind；序列化测试断言无 source_content；MAINT 证据只存 120 字符截断行摘录 |
| HealthRule/Registry/Engine 骨架（§7/§8/§9/§16/§17） | trait + `HealthRuleMeta`（含 threshold 语义声明 §73）+ 按 id 排序注册表；规则 id 与实现文件解耦 |
| Context/Scope/Options（§19/§63/§222） | `HealthContext`（全借用不复制）、`HealthScope`（M7a 实现 FullProject）、`HealthOptions{disabled/enabled/max_findings/max_findings_per_rule}` |
| Result/immutability/thread-safety/无全局状态（§201–§204） | `evaluate(&context, &config, &cancel) -> PrismResult<RuleResult>`；无 static mut；顺序执行确定性（并行留 M7b，§97 允许） |
| Determinism + ordering + collect-sort-dedup（§95–§98/§161） | 三次运行 JSON 逐字节一致（单测）；§96 排序单测；dedup by 确定性 id |
| Partial failure / completeness（§99/§100/§163） | 失败规则 → `rule_failed` 诊断 + failed_rules + Partial，其余规则存活（单测）；Truncated > Partial 优先级 |
| Cancellation/timeout/large-safety/真实 progress（§101–§105/§160） | timeout **未单设线程终止**——复用 `CancellationToken` 协作取消（§101「优先复用 M0 Task/cancellation」，规则内逐文件/逐行检查）；取消 ⇒ `Err(Cancelled)` 不伪装 0 findings（单测）；§103 合成大项目安全测试：注入规则产出 5000 findings/1000 文件 ⇒ no crash、确定性（两次运行逐字节一致）、耗时受界断言、全局 cap ⇒ Truncated（4500 截断计数保留）、协作取消中途生效；stats 为真实计数（无假百分比）。内存以「完成运行 + 引擎无跨运行累积结构」为界，未做峰值测量（如实记录） |
| Parser failure / Unsupported 降级（§116/§117/§157） | e2e：解析失败文件仍被词法规则扫描；未知扩展名/不支持语言安静跳过、无伪造 |
| suppression：suppressed ≠ not detected（§65–§69/§164） | `suppressed_findings` 配置命中 ⇒ 仍检出、status=Suppressed、stats 分计（引擎单测 + e2e） |
| health: 配置/schema 校验/unknown/precedence（§70–§72/§219–§221） | core `HealthConfig`（deny_unknown_fields ⇒ threshhold 报错单测）；precedence disabled_rules > rules.enabled > default 单测；CLI > config 单测；unknown rule/option 引擎期诊断（threshold 语义单测） |
| max_findings[_per_rule] 截断标记 Truncated（§223/§224） | 单测：cap ⇒ Truncated + "reported N of at least M" 诊断 |
| `prism health rules`（§74/§129） | CLI 文本/JSON 双格式；5 个契约测试（含 config 禁用生效、无效配置 fail-loud） |
| MAINT001 满足 §42–§43/§111/§154/§148 | TODO/FIXME/HACK/XXX 大写整词 + 注引导符（行首或行尾）+ URL/字符串/标识符负例；unknown 扩展名跳过；噪音由 cap+过滤承担；positive/negative/edge/configuration/determinism 五类测试齐 |
| 无 AI/网络/遥测/Git mutation/auto-fix/远程规则（§125–§128/§182/§183） | 代码审计：新代码零网络零 AI 零 git 调用；suggested_action 仅通用文字 |
| 无 M7 专用持久化（§188） | 无数据库；复用 M1/M2 事实 |
| 措辞符合 §139/§140 | explanation 三段结构；标题用 "Detected …"，无评判性词汇 |

## Risks（仅 P0/P1）

- **P1**：`health rules` 的 CLI 覆盖显示（§220 CLI override 后的最终状态）属于 M7b 的完整 `prism health` 命令面——当前 config 合并状态正确显示。
- **无 P0**。无 M0–M6 语义破坏（§175）。

## Deferred → M7b

完整规则目录（ARCH/DEP/CODE/SEC/DOC）、HealthSummary/HealthReport、`prism health .` 完整分析命令、GUI Health 数据流、误报抽样审计、真实项目验证、Final Audit（§250）与 Completion Gate（§251）。
