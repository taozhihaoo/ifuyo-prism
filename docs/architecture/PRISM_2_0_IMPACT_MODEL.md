# PRISM_2_0_IMPACT_MODEL.md

> 2.0-M26 产物（§85 允许合并：Impact Model + Impact Rules + Impact Path + Change Mapping + Test Matrix 合并本文；Evidence Graph 见 PRISM_2_0_INTELLIGENCE_CORE.md §graph）。
> 实现 = `prism-intel` graph.rs/impact.rs + `prism-csharp` change_mapping.rs + CLI `--domain impact`（commit 5b6ce91）。

## 1. Impact Model（§5/§6/§34）

- **ImpactType 四类**：Direct（1 hop）/ Transitive（≥2 hops）/ Potential（unresolved 边或名字面候选）/ Unknown——与 Confidence 分离。
- **ImpactCategory 六类**：Breaking（binding 失效风险）/ Behavioral（行为可能变化）/ Structural / Test / Runtime / Unknown。
- **Resolution**：沿 M25（Resolved/Ambiguous/Unresolved/...）——unresolved 边 ⇒ Potential + Low（§63 不作高可信传播）。

## 2. Impact Rules（§16 Change-aware 表）

| ChangeType | 关系面 | 类别 | 继续传播 |
|---|---|---|---|
| ContractRemoved | calls/constructs/references | Breaking | 否（binding 已失效，无需深层） |
| ContractRemoved | inherits/implements/backed_by | Behavioral | 是（派生面行为影响） |
| SignatureChanged | calls | Breaking | 是 |
| SignatureChanged | references/hierarchy | Behavioral | 是 |
| BodyChanged | calls / hierarchy | Behavioral | 是（§19 Body-only ≠ broken） |
| BaseOrInterfaceChanged | hierarchy | Behavioral | 是；其余 Structural |
| Added | 全部 | Structural | 否（新面） |
| FileLevel | 全部 | Unknown | 是（scope Unknown §44） |

## 3. Change Mapping（§11-§13/§44-§45）

- Git status（working tree 优先 §43）→ 文件级 ChangedFile（added/removed/modified/renamed）。
- .cs 文件 ⇒ `map_file_change(old_extraction, new_extraction)`：以 **symbol_id**（含签名分量）为 diff 键——同 id ⇒ BodyChanged（面变化 ⇒ BaseOrInterfaceChanged）；旧有新无 ⇒ ContractRemoved；新有旧无 ⇒ Added。签名变化天然形成 Removed+Added 对（重载不折叠 §13）。
- 非 .cs ⇒ FileLevel（scope Unknown，§44 诚实降级）。
- Rename：沿 git rename 证据（M22/M4）；符号级 rename 识别 = 已记录限制。

## 4. Impact Path（§8/§9/§66）

primary_path = 变更起点 → … → 受影响实体的 entity id 链（BFS 最短）；alternative_paths 计数保留（不展开）。截断：max_hops/max_entities 触发 ⇒ TraversalMetadata 计数（§66 说明为何截断）。

## 5. 去重（§67/§68）

实体唯一（BTreeMap）；Direct 优先 Transitive（BFS 层序天然保证）；同层多父首见保留。

## 6. Traversal 保障（§33-§36/§57/§59）

cycle-safe（visited + 路径回环计数 cycles_detected）；有界（max_hops 3 / max_entities 500 默认）；确定性（BTreeMap + (impact, category, len, id) 排序）；起点屏蔽（变更实体不作为 affected）。

## 7. Test Matrix（§70 十用例实测）

| Case | 场景 | 结果 |
|---|---|---|
| 1 | CALLS B changed ⇒ A Direct/Breaking | ✓ |
| 2 | Body-only ⇒ Behavioral 非 Breaking | ✓ |
| 3 | Removed ⇒ Breaking | ✓ |
| 4 | 继承链 Direct+Transitive 分层 | ✓ |
| 5 | Interface ⇒ 实现者 Behavioral | ✓ |
| 10 | 环终止 + cycles_detected | ✓ |
| 附加 | max_hops 截断 / max_entities 截断 / unresolved⇒Low / 确定性 / 名字面种子 | ✓ |

## 8. Godot Impact（§25-§31）

M24 关系（attached_script/references_scene/backed_by/signal_connects/loads）同形可摄入 EvidenceGraph——CLI v1 已接 M25 语义边；Godot 边摄入 = 已支持（graph.push 通用），端到端用例随 M24 fixture 域扩展（记录于限制）。

## 9. CLI

`prism intel --domain impact [--json]`——工作区优先（§43）；输出 Changed/Direct/Transitive/Cycles/截断元数据 + --json 全量。

## 10. 支持边界（§86 摘要）

Supported：CALLS/INHERITS/IMPLEMENTS/OVERridES/CONSTRUCTS 影响、传递链、环防护、有界遍历、名字面 Potential。
Partially：跨项目（cross_project 标记就绪，分组 M26 后续）、Godot 端到端（图摄入就绪，场景级用例待扩）。
Unsupported（诚实）：运行时 dispatch 枚举、完整数据流、风险评分（M28）。
