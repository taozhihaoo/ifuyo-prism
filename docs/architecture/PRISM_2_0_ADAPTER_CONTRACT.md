# PRISM_2_0_ADAPTER_CONTRACT.md

> 2.0-M20 产物（M20 §47/§十一/§十三）。定义 Project Adapter 的能力契约；实现属 M23–M25。

## 1. 定位

Adapter 把特定语言/引擎的项目语义翻译成 Core 的通用模型（Entity/Fact/Relation/Evidence/Finding）。Core 不知道 Godot / Unity / TypeScript / Python 的存在（依赖方向：adapter → core，单向）。

```text
Prism Core（通用模型 + 契约）
        ↑ 实现
Project Adapters：Godot(M24) | C#(M23 L1 + M25 L2) | TypeScript(后续) | Python(收编既有 parser) | 未来
```

## 2. 能力声明（Capabilities）

每个 adapter 必须声明能力位（Core 据此调度，不做 if-语言 分支）：

```text
scanner             项目布局/文件分类（可复用 M1 ScanOptions/IgnoreEngine）
parser              L1 语法：symbol 抽取（声明 M23 能力矩阵）
symbol_provider     Symbol/成员/位置/规范 id
reference_provider  L2 引用/调用（可 None——诚实声明不支持）
semantic_provider   L2 类型/继承/实现/泛型/工程引用（可 None）
facts_provider      领域事实（如 Godot scene/node/signal/resource）
evidence_provider   每条结论的证据产出（对齐 EVIDENCE_CONTRACT）
fixture_strategy    确定性最小 fixture 集（自含于仓库，禁止提交用户真实项目）
versioning          adapter 版本 + 输出 schema_version（模型版本不匹配 ⇒ compare limited，沿 §71 先例）
error_handling      域错误 → core Error Contract 映射；部分失败 = 降级 + warnings（沿 collect_analysis 先例），不伪造空结果（§88）
```

## 3. 硬性规则

1. **禁止污染 Core**：adapter 概念（Scene/Node/Signal/NodePath/res://、C# partial 等）只存在于 adapter crate/模块；进 Core 只能是通用 Entity+metadata（收编判据见 ARCHITECTURE §4）。
2. **路径身份**：adapter 对外一律使用 Core 的项目相对身份 + canonical 文件位置（2.0-M20 §32 已统一 canonicalize 口径）；`res://` 等 engine 内路径是 adapter 层的映射产物，不得回流为通用身份。
3. **诚实降级**：解析失败/部分失败输出 limitations + warnings，永不虚构关系（FACT/INFERENCE/HYPOTHESIS 纪律）。
4. **无网/无遥测**：adapter 不得引入网络调用；新增第三方依赖（如 C# parser 后端）必须按 §45 清单论证并在对应里程碑评审（M23/M25 各自决定，M20 只记录候选：tree-sitter-c-sharp / Roslyn(需 .NET 运行时，存在环境依赖) / 自建）。
5. **只读**：adapter 对用户项目零写入。
6. **同源**：adapter 结果经 Core Service 暴露，CLI/GUI/MCP 三面同日可用。

## 4. Godot Adapter（第一优先，M24）

- 输入面：`project.godot`（application/autoload 等 section）、`*.tscn`（node/instance/connection/ext_resource/sub_resource）、`*.tres`、`res://` 路径、C#/GDScript 挂载。
- 产出关系（FACT 级，文本可直接证明）：Scene contains Node、Node mounts Script、Node emits/handles Signal（[connection]）、Scene references Resource、Autoload 映射脚本。
- 不做：场景图 ≠ 调用图（禁止把 mount 关系说成 semantic call——L2 归 C# adapter）。
- Fixture：极小合成 Godot 项目（project.godot + 2 scene + 1 tres + C# 占位脚本），覆盖上列关系各 ≥1 例。

## 5. C# Adapter（第一优先，M23/M25）

- M23（L1）：class/struct/interface/method/property/field/enum/namespace + 规范 Symbol Identity（≠display name；候选形式 `project+file+qualified name+kind+signature`，按解析器能力定稿）。
- M25（L2）：reference/call/type/inheritance/implementation/generic/project reference，逐项置信度 Exact/Probable/Unresolved；partial class、overload、generic 为已知难点，按证据诚实标注。
- 后端选型（Roslyn vs tree-sitter vs 自建）在 M25 开工时按评估矩阵决定；Roslyn 路线意味着 .NET 运行时环境依赖（Prism 为 Rust，需进程外 helper），必须在文档显式声明并给无 SDK 降级路径（2.0 提示词 M25 §七 已载）。

## 6. Python / TypeScript（既有/后续）

- Python：1.0 手写 parser（FACT：现支持语言仅 Python）作为 L1 参照实现收编进 adapter 形态（M23，行为保持 + 测试沿用）。
- TypeScript：后续 adapter，排在 Godot/C# 之后；复用 M1 语言识别（Language 枚举已支持）。

## 7. Fixture 策略（跨 adapter 统一）

- 全部 fixture 合成、极小、可重复、随仓库提交（`fixtures/` 或 crate 内 tests/fixtures，沿 1.0 惯例）。
- 每个 adapter 契约测试至少覆盖：能力声明、symbol 规范 id 稳定性、evidence 可追溯、错误降级、schema_version。
