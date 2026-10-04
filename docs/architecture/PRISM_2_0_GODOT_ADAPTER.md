# PRISM_2_0_GODOT_ADAPTER.md

> 2.0-M24 产物（§85 31 节合并入本文件；Benchmark 并入 §24）。
> 实现 = 新 crate `prism-godot`（commit 2c71f4f）+ CLI `prism intel --domain godot`（ee6a9fe 后续提交）。

## 1. Executive Summary

Prism 第一个项目语义 Adapter 完成：project.godot / .tscn / .tres 静态解析 → Godot Project/Autoload/Scene/Node/Script/Resource 实体（全部 `EntityKind::other("godot.*")`）+ attached_script/loads/backed_by/signal_connects/contains 关系 → M21 IntelBundle。三条 Vertical Slice 全部真实可查询（§61-§63）。**Core 零 Godot 依赖（FACT：prism-godot 单向依赖 prism-intel/csharp/core）**。Gate：PASS。

## 2. Repository Baseline

main @ M23 之后；18 crate（新增 prism-godot）；73 suites / 606 Rust + 55 vitest、clippy 0、fmt 0。

## 3. Existing Godot Implementation Audit

审计结论（FACT）：1.0 无任何 Godot 专用实现——仅 M1 语言表识别 `.tscn/.tres` 为未知类型、M22 L0 把 project.godot 记为普通文件。分类：无 KEEP/WRAP 对象；M22 文件清单直接复用为输入（§84 迁移 = 纯新增 adapter）。

## 4. Adapter Architecture

`ir.rs`（轻量 IR：GodotProjectFile/TscnDocument——§32 控制规模）→ `parse.rs`（project.godot 最小 INI + .tscn/.tres 节驱动）→ `adapter.rs`（编排：失败隔离/限额/取消/环检测 + GodotIndex 查询）→ `intel.rs`（→ IntelBundle）。无 God Object：解析、查询、翻译各一模块。

## 5–7. Project Detection / project.godot / Autoload

project.godot = 当前唯一可靠信号（1.0 支持面下，诚实记录）；解析 config_version/name/main_scene/features（版本线索，无 ⇒ unknown，§33）/ `[autoload]`。Autoload = 一等实体 + `has_autoload`（project→autoload）+ `loads`（autoload→script），Evidence = project.godot 对应行（§9）。非 Godot 项目 ⇒ 空 + 诚实警告（无伪造检测事实，有测试）。

## 8–9. Scene / Node Model

.tscn → Scene 实体 + Node 实体（`EntityKind::other("godot.node")`；**Node Identity = 场景相对 NodePath**（`/Root/Parent/Child`，SceneId+path 编码进 canonical key §16）+ Virtual location（res:// 形态仅存显示））。树关系 = M21 `contains`（scene→root→children）；节点类型进 metadata `godot_type`（§37 不解析继承链）。

## 10–14. Resource / Script Mapping / C# Symbol / NodePath / Scene Reference

- Resource：.tres → `godot.resource` 实体；ext_resource 引用按类型连接（PackedScene→`references_scene`，其余→`references_resource`）；SubResource 记录在 IR 不作文件实体（§22）。
- Script：`godot.script` 实体（res:// 显示，项目相对路径身份）；`attached_script`（node→script）来自 tscn `script = ExtResource(id)`——Evidence 带 property 原文（§19）。
- **C# Symbol**：`backed_by`（script→M23 class Entity）——M23 索引按 declaration file 匹配，**不复制符号**（§11）；Origin=Inference（路径对应推导，§29）/High。M23 索引缺席 ⇒ 关系缺席 + 警告（不伪造）。
- NodePath：connection 的 from/to 归一化解析（`.`/`..`/多段）；**Resolved→关系 / Unresolved→保留 raw 事实**（§24/§28 三态，测试覆盖含不存在目标）。属性内 NodePath 字面量 = 已记录限制 #2。
- Scene Reference：PackedScene ext_resource + instance 节点 → `references_scene`；环检测 → SceneReferenceCycle 诊断（§44，visited-set + 深度 16 + 容量 8）。

## 15. Autoload
见 §5-7；E2E 实测 `GameState → Scripts/GameState.cs` 链。

## 16–18. Evidence / Origin / Confidence / Freshness

每个连接/事实带 Evidence（Config/File kind，tscn property 原文进 note §56）；关系 Origin=Fact（文本直证）/Inference（backed_by 推导）；Confidence 全 High（文本直读/唯一解析——§74 分级表）；Fresh at observation（§57 与 L0/L1 同规则）。Provenance = `prism-godot::godot_to_intel` + 版本。

## 19. Incremental Analysis

GodotAnalysis 为确定性全量观察（场景解析毫秒级）；`Incremental Result == Fresh Full Result` 有测试（§66 最后一条 invariant）；per-file 增量缓存按 §58 YAGNI 暂缓（与 L0 同判定）。

## 20. Cache / Snapshot

无新增缓存/无新 SQLite 表（§58/§60 红线）；bundle 可序列化可 delta——snapshot 域桥接按需评估（沿 M22 先例 deferred）。

## 21. CLI / GUI / MCP

CLI：`prism intel --domain godot [--json]`（消费 L0 清单 + M23 索引；godot-only 不吸收 symbols bundle；非 Godot 项目诚实空域 + 警告）。GUI/MCP：无新增（N/A 同前例；bundle JSON 为未来暴露形态）。

## 22. Failure / Partial / Cancellation

坏场景/超大/不可读 ⇒ 逐文件诊断 + 继续（§46）；截断节头 ⇒ well_formed=false 诊断；取消 ⇒ Cancelled + 已完成部分（测试覆盖）。

## 23. Security Audit

静态文本解析；无 shell/Godot Editor 执行/网络（§52/§70）；路径穿越面 = 无（清单来自 L0，res:// 映射纯字符串前缀剥离）；资源限额在案；Unicode 场景路径有测试（l0 层）。

## 24. Performance Benchmark

合成 Godot 项目（10 场景 × 20 节点 + 2 脚本 + autoload，标注 synthetic）：`prism intel --domain godot` 端到端 release < 100ms；环检测/NodePath 解析为线性（BTreeMap 直查，无 N×N——§67 红线遵守）。既有命令零改动。

## 25. Test Coverage

15 项（parse 4 / adapter 7：non-godot/nodepath/res 映射/normalize/环/cancel / lib 4：垂直切片/诚实空域/失败隔离/incremental==full）+ CLI E2E 实测。

## 26. Legacy Migration

无旧 Godot 模型存在（见 §3）——纯新增，无双真相风险。

## 27–28. FACT / INFERENCE / HYPOTHESIS 与问题状态

- FACT：15 测试数字、E2E 关系链、Core 零 Godot 依赖（依赖清单）。
- INFERENCE：性能线性外推；子项目/monorepo 结构已被 has_manifest+godot.scene 组合观察覆盖。
- HYPOTHESIS：多 Godot 项目共存的完整拆分策略（当前观察级，真实需求出现再扩展）。

## 29. P0 / P1

P0=0；P1=0。

## 30. M24 Remaining Risks

1. tscn 属性值多行数组仅取首行（限制已记录——影响显示字段，不影响关系链）。
2. instance 节点的子节点 parent 归属在 instanced 场景边界内的深度解析（v1 以 references_scene 表达，节点级穿透归 M25+ 需求评估）。
3. 属性内 NodePath 字面量未抽取（connection from/to 已覆盖主链）。

## 31. M25 Preconditions

- **M24 产出**：Scene/Node/Script 实体 + `attached_script`/`backed_by`（含 M23 class Entity id，跨 bundle merge 后同链）/`references_scene`/`signal_connects`（method 名进 metadata）关系——M25 的 C# 语义层可直接在这些 Entity id 上挂 references/calls。
- M25 不需要重扫场景/脚本（bundle 确定性可重产）。
- 边界不变量：Node→Script=M24 事实；Method A→Method B=M25。

（§83 三条关系链 + §87 反向审计：CLI E2E 已验证识别/列出/Node→Script→Symbol/Signal→Node 链；Snapshot/History 未破坏——全量回归绿。）
