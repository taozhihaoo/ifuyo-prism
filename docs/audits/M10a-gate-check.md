# Prism M10a Gate Check（Context Selection Engine — Part 1/2 Exit Gate）

日期：2026-10-03 · 依据：`Spec/Prism M10a — Context Selection Engine 完整开发提示词.md`【阶段性完成门槛】
复核基线：`cargo test --workspace` **485 passed / 0 failed**（58 个测试二进制）+ vitest 41 passed，clippy --workspace --all-targets **0 warning**。

> Findings-first。上一次（SARIF、.env）审计-代码不符的教训已吸取：本表每项 [x] 均指认实现文件与测试。

## 门槛逐项核对

| 门槛项 | 状态 | 实现与测试证据 |
| --- | --- | --- |
| M0–M9 regression 全部通过 | ✅ | 485 Rust（prism-context 21 lib + 5 CLI 契约新增）+ 41 vitest，clippy 0 |
| ContextRequest / TaskKind（14）/ ContextFocus（12）/ ContextTarget（Resolved/Ambiguous/NotFound/None 不猜） | ✅ | model.rs + task.rs；§16 歧义测试（同名符号两处 ⇒ Ambiguous 全列出）；§18 Unknown ⇒ Generic 声明测试 |
| ContextSource 分类 + Source types（15） | ✅ | model.rs SourceType + ContextSource{source_type/source_id/path/location/provenance §110} |
| ContextItem / ContextGroup / ContextRange / ContextManifest | ✅ | model.rs（item 含 §112 compact metadata + §35 truncation facts + §218 content_hash） |
| ContextSelection / SelectionTrace（含 excluded_reason） | ✅ | selector.rs SelectionTrace + ExcludedCandidate（reason enum §373 全 8 种） |
| CandidateCollector 体系隔离 | ✅ | collectors.rs：11 个 collector 只产候选；ranking 在 selector.rs、budget 在 selector、render 在 service.rs（§89 职责分离） |
| 归一化与去重（身份 = Project + path + SymbolId/ModuleId/FindingId/CheckRun 实例） | ✅ | selector::normalize 身份键 (source_type, path\|source_id)；§92 多理由保留测试；§94–§97 沿用 M2 SymbolId / M7 FindingId / M8 run_id |
| 确定性优先级带（Critical/High/Normal/Supporting/Low；无浮点/weights） | ✅ | selector::prioritize 规则化（§144）；无任何 f32/f64 排序键（§24/§143–§145） |
| 确定性排序 + 稳定 tie-breaker | ✅ | (band, source_type, path, location) 排序（§139/§140）；reproducible CLI 测试（两次运行 selected ids + bytes 全等，§138/§220） |
| Change/Risk/Impact-aware（复用 M5/M6/M7/M8/M9，不重新实现） | ✅ | FactsInputs 由调用方注入既有结果（§263/§48/§50–§53）；anti-pattern §463–§467 无第二套引擎 |
| ContextBudget：max_items/max_bytes/max_tokens_estimate/max_source_files | ✅ | model.rs ContextBudget（默认 64/512KiB/24k tokens/24 files） |
| Critical reservation + must-include | ✅ | selector::select：超预算 Critical 仍入选（§198），其余记 BudgetExcluded（§141）；测试 budget_reserves_critical |
| 预算边界行为 | ✅ | 零预算 ⇒ InvalidBudget ⇒ CLI exit 3（§200 测试）；极小预算 ⇒ Critical-only + limitations；无 panic |
| lazy materialization + 四级策略（Full/RelevantSymbols/Skeleton/MetadataOnly） | ✅ | materializer.rs：selector 产出 metadata-only 占位，物化阶段才读文件（§202）；四级按优先级/形态落位 |
| 截断沿 symbol/heading/line 边界 + truncated 事实 | ✅ | symbol range 切片（M2 range）/行边界/markdown heading 回退（§36/§176）；truncated + original_size + included_size + truncation_reason（§35 测试） |
| Documentation/Instruction 选择、path scope、precedence、不执行命令 | ✅ | documentation_candidates：AGENTS/CLAUDE 为 instruction（ProjectInstruction）、目标目录 ⇒ specificity evidence（§58–§60）、"data, never executed; conflicts retained" 写入 evidence（§57/§56/§459——M10b 再做防御性渲染） |
| provenance / reason / excluded_reason / source locator | ✅ | ContextSource.provenance + reasons + trace.excluded + location（§110/§146/§147） |
| `prism context` 基础生成（`<path>`/`--task`/`--json`） | ✅ | context_command.rs（§283/§347/§348：stdout 纯数据；另有 --scope/--target/--max-tokens） |
| 本篇范围单测通过 | ✅ | prism-context 21 lib（task 5 / selector 5 / materializer 5 / collectors+service 经 CLI 5 契约；selection/dedup/budget/truncation/reproducibility 全覆盖；完整测试规格 §395–§407 归 M10b） |
| 无 AI / embeddings / vector DB / RAG / MCP / 网络依赖 | ✅ | prism-context Cargo 依赖仅 workspace 内 crate + serde/uuid；代码无任何网络/AI 路径 |
| 只读（不修改源码/文档；header 不写回） | ✅ | prism-context 全部为 fs::read；无 write 路径 |
| 未重新实现 parser / graph / git / health / verify / environment | ✅ | 全部经 FactsInputs 消费既有结果（§468/§469 integration matrix） |
| 无 P0，无未解决阻塞 P1 | ✅ | P1 列表如下，均为增强性质 |

## P1（带入 M10b，非阻塞）

1. **§451–§457 反模式面**的渲染层防御（prompt injection 转义、仓库内容标注为 data）归 M10b privacy/redaction 主题（本篇 collector evidence 已声明 instructions-are-data）。
2. **§150–§152 相邻 range 合并**：v1 每 item 单 range；跨符号相邻合并留 M10b。
3. **§203 heading-range 文档节选择**（DocumentSection 模型）：v1 只做 heading 边界回退截断；按相关性挑节留 M10b（文档健康同主题）。
4. **§213 ContextRelation 关系图**：模型未建（selected items 间关系）；M10b。
5. **§266–§267 session 缓存 / stale 检测**：v1 每次全新生成（天然无 stale）；缓存优化留 M10b。
6. **§57 instruction conflict 的结构化 InstructionConflict 类型**：v1 以 evidence 声明保留全部来源；结构化冲突类型归 M10b。
7. **§427 impact summary 计数 item**（Direct/Transitive dependents 计数行）：impact items 已带 distance，汇总行留 M10b。

## 红线自查

- 无 AI/LLM/embedding/vector/MCP/网络（§3/§4/§253/§364/§368）：结构保证（无此类依赖）。
- read-only（§156–§158）：prism-context 无任何写路径；context header 只存在于 pack 渲染。
- 不重新实现引擎（§463–§468）：全部 FactsInputs 消费。
- 确定性（§138–§140）：BTree/sort 稳定键；无 HashMap 迭代序依赖。
- No score（§27/§136/§462）：无任何分数输出。

结论：**M10a 阶段性完成门槛通过**，按门槛指令以
`feat: add context selection engine (M10 part 1/2)` 收口（不 push）；
Final Audit / 完整验收 / 最终 commit 在 M10b。
