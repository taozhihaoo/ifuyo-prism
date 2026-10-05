# PRISM_2_0_M25_BENCHMARK.md

> 2.0-M25 §63 要求。环境：Windows / release 构建 / `prism intel --domain semantic --json` 端到端（L0 清单 + L1 符号引擎 + L2 语义 + Intel 翻译 + JSON）。
> **项目为 synthetic**（每文件 ~10 行、1 类 2 方法，如实标注）。

## 结果（best of 2, ms，release，relations = CALLS+CONSTRUCTS 语义边）

| 规模 | Files | Relations | Cold(=Warm) |
|---|---|---|---|
| small | 5 | 5 | 39ms |
| medium | 50 | 50 | 64ms |
| large | 300 | 300 | 222ms |

线性 ≈0.7ms/file。**修复记录（§67 O(N²) 红线）**：首版 `update_file` 每文件
深拷贝整个 extractions 表建 Resolver——300 文件 54s（debug）；重构为
`update_source`（O(1) 登记）+ 单遍 `rebuild`（单 Resolver 借用遍历）后
线性恢复。增量 = rebuild 内 unchanged 文件复用抽取（§37 文件级增量；
语义级 per-file 增量 deferred——符号表变更影响任意文件解析，v1 全量重算
为正确性优先，§六十二）。

## 关系计数（large 300 files）

resolved + unresolved 关系全部进入 bundle；unresolved 以 `semantic_unresolved` 事实呈现（外部/框架引用，非错误）。

## 增量

SemanticEngine per-file content_hash 复用——unchanged 文件重 update 为 O(hash) 直返（测试覆盖）；单文件变更重析 = 单文件解析耗时（毫秒级）。
