# PRISM_2_0_VIZ.md — Visualization 契约（2.0-M212）

> 2.0-M212 产物。M212 = Intelligence Core 的**投影层**（§3/§119）：
> 图不是事实，图只是事实的可视表达；Edge 必须来自真实 Relation /
> Dependency / Signal 数据；Unknown 就是 Unknown。

## 1. 定位与迁移判定（§0/§87/§88）

- `prism-viz` crate（第 24 个成员）：确定性、有界的 View Model 投影，
  CLI / Tauri / GUI 共用同一数据面（§71）。
- 既有 `DependencyView`（M3 SVG）= **RETAIN**（未替换，无双数据源）；
  M212 在 Understand 页新增 `VizGraphPanel`（M212 卡片）消费新投影。
- 无 WebGL（§32：无性能证据不上）；纯 SVG/DOM + 既有 Design Token。
- 无新 Entity/Relation/Finding/Graph Database（§2 禁止清单全遵守）。

## 2. 数据契约（§5-§7/§86）

```text
VizGraph   { schema_version=1, view, status, nodes[], edges[], legend[],
             truncated, diagnostics }
VizNode    { id=viz:{kind}:{entity}, entity_id(原 Core 身份), kind(10 种),
             label, parent(层级), status(complete|partial|unknown|unavailable),
             metadata }
VizEdge    { id, relation, source, target, status, metadata }
VizLimits  { max_nodes=500, max_edges=2000 }
```

- `finalize`：去重 + (kind,label,id)/(relation,source,target) 稳定排序
  （§43/§44）+ 截断标记（§28）+ 空投影 ⇒ Unknown（§46）。
- 节点 `entity_id` 可回溯原 Entity/Relation（§4 Evidence-first）；渲染端
  不做任何分析（§86 禁止清单）。

## 3. 四投影（§8/§106）

| 视图 | 输入 | 内容 |
| --- | --- | --- |
| `godot`（§106 第一优先） | M24 GodotAnalysis + M23 SymbolIndex | Project→Scene→Node→Script→Symbol 跨层链（attached_script 证据）；signal 连接边；缺索引 ⇒ Partial+Unknown 节点 |
| `architecture` | M1 files + M3 DependencyGraphResult | 目录树层级（§9 聚合）+ file depends_on 边（两端在投影内；external 留 Dependency View §73） |
| `dependency` | M29 SupplyReport | declared/resolved 节点 + Unknown 版本/license（§21 UNKNOWN ≠ 无 license；§10 declared≠resolved 可分辨） |
| `impact` | M26 ImpactResult | ImpactType 驱动状态（Potential⇒Partial §46）+ primary_path 逐跳边（§22） |

## 4. 消费面（§60/§39）

- CLI：`prism viz . --view architecture|godot|dependency [--json]`（文本 =
  层级树；architecture 缺 M3 图 ⇒ 无边投影 + 诊断，§67 Partial 不空白）。
- Tauri：`get_visualization_graph(view, path)`——architecture 消费
  session.discovery+graph_model；godot/dependency 按需 bounded 采集
  （M24 analyze / M23 index / M29 collect，4 MiB/文件上限）。
- GUI：Understand 页「Visualization (M212)」卡片（VizGraphPanel）——视图
  切换 / 层级树 / 节点选择 Inspector（entity+metadata+relations §36）/
  Legend §48 / 截断与 partial banner（§28/§46）/ IPC 失败 unavailable
  （§66 不空白）。6 项 vitest 锁定。

## 5. 边界与遗留（§111 DoD 对照）

- 已满足：四投影确定性/有界/Partial/Unknown；godot 跨层垂直切片实测；
  关系图例；统一 Inspector 雏形；无渲染端分析；安全边界（零 shell/
  网络/secret——输入来自既有 redaction 后的引擎产物）。
- P1（有证据的后续）：① Inspector 深链 Evidence→Source 跳转（§36 完整
  形态）；② Impact 视图接 GUI（M26 analyze 按需运行）；③ Code City
  （§12：v1 以 Architecture 层级树承担规模/分布观察）；④ Git Replay
  时间轴视图（§14：数据面 M211 就绪，GUI 沿 Snapshot History 页 RETAIN）；
  ⑤ 大图 minimap（§51）；⑥ 性能 benchmark 报告（§79：投影均为亚秒级
  bounded 计算，实测自宿主）。
- M213 前置（§M213 边界）：`get_visualization_graph` 只读投影 API 即
  M213 可复用的 domain service 面。

## 6. 实测证据

- prism-viz 12 项单测：跨层链（label 含 App.Battle）/缺索引 Partial/
  signal 边端点解析（from=. ⇒ 根）/空投影 Unknown/层级聚合/external 隔离/
  declared-resolved 状态分离/Unknown 版本/ImpactType 状态驱动/路径边/
  finalize 截断+去重+排序/空=Unknown。
- GUI 6 项 vitest：渲染/选择 Inspector/partial banner/截断 banner/
  unavailable/视图切换重取。
- 自宿主：`viz --view architecture`（500 节点截断如实标记 + 层级树）；
  Godot 夹具（Project→Scene→/BattleRoot→Script→App.Battle 6 节点 5 边）；
  自宿主 godot 视图无 godot 文件 ⇒ Unknown 如实。
- 全量：clippy 0 / fmt 0 / **751 Rust**（M211 末 739 + viz 12）+
  **61 vitest**（55 + 6）。
