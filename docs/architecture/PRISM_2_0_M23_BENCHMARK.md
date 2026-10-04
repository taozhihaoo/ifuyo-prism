# PRISM_2_0_M23_BENCHMARK.md

> 2.0-M23 §71 要求。环境：Windows / 同机 / release 构建 / `prism intel --domain symbols --json` 端到端（含 L0 文件清单 + 解析 + 抽取 + 索引 + Intel 翻译 + JSON 序列化）。
> **项目为 synthetic**（§71：生成 fixture，已如实标注，不冒充真实项目）——每文件约 26 行、10 个符号（class+interface+enum+3 enum members+ctor+2 overloads+property+field+event+indexer+operator 覆盖）。

## 结果（best of 2, ms）

| 规模 | Files | Symbols(实体) | Cold | Warm |
|---|---|---|---|---|
| small | 5 | 95 | 39ms | 39ms |
| medium | 50 | 950 | 65ms | 65ms |
| large | 300 | 5700 | 317ms | 317ms |

- 扩展性近似线性（≈1ms/file @ release）；无 O(N²) 迹象。
- Debug 构建对照：large 3540ms（构建模式影响 ~10×——基准以 release 为准）。
- Search 延迟：BTreeMap 前缀/子串过滤，亚毫秒级（未单独列表——与项目规模对数+命中数相关）。
- Memory：未单独采样（bundle 序列化体量为间接代理——large 全 bundle JSON 正常输出）；如未来出现规模需求再引入正式内存测量（§42 YAGNI）。
- Incremental：per-file hash 复用 + 全量索引重建，重建 = large 317ms 上限（change 场景实测不超过 cold）。

## 与既有基线关系

M20/M21/M22 命令零改动（intel 默认域不变），既有性能基线不受影响（全量回归绿）。
