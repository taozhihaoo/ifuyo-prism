# PRISM_2_0_MIGRATION.md

> 2.0-M20 产物（M20 §46/§27）。Current → Future 映射 + 迁移/兼容策略。

## 1. 迁移总策略

1. **演进而非重写**（纲领 §十四/M20 §51）：1.0 域 crate 全部保留；2.0 以 EXTEND/MIGRATE 为主，无 REMOVE 级组件。
2. **兼容层过渡**：M21+ 统一模型上线时，各域 CLI/GUI JSON 契约不变（§三十八 数据兼容）；新模型并行产出，按域逐个切换。
3. **数据兼容**：SQLite 无 schema 变更即为默认态；每次变更走 prism-store migration（顺序、fail-stop、事务）。

## 2. 组件映射表

| 1.0 组件 | 所有权（现） | 数据（现） | 2.0 去向 | 迁移方式 | 兼容策略 | 破坏风险 | 里程碑 |
|---|---|---|---|---|---|---|---|
| prism-project（发现/扫描） | prism-project | 内存 + session | L0 结构层 | **Extend** | 命令/JSON 不变 | 低 | M22 |
| prism-git（只读事实） | prism-git | 无持久 | Change/History 事实源 | **Retain** | — | 无 | （既役）/M211 消费 |
| prism-env（环境） | prism-env | 无持久（报告即产） | Environment Intelligence | **Extend** | 命令/JSON 不变 | 低 | M29 |
| prism-parser/analysis（Python L1） | prism-parser/analysis | 无持久 | L1 Symbol Engine 参照实现 | **Migrate**（收编为 adapter 形态） | 现有 health/context 行为回归保护 | 低 | M23 |
| prism-graph（模块依赖） | prism-graph | 无持久 | 统一 Relation 层 | **Migrate**（依赖事实转为 Relation+Evidence） | graph 命令 JSON 保持 | 中 | M23/M26 |
| prism-changes（M5） | prism-changes | 无持久 | Change 域 | **Extend** | — | 低 | M26 前 |
| prism-impact（M6） | prism-impact | 无持久 | Evidence-backed Impact | **Migrate**（结论补 origin/evidence） | impact 命令 JSON 增量加字段 | 中 | M26 |
| prism-health（M7） | prism-health | 无持久 | Finding Engine | **Extend**（Finding→统一模型映射） | 13+ 规则行为回归保护 | 低 | M28 |
| prism-verify（M8） | prism-verify | 无持久 | Validate | **Retain** | — | 无 | （既役） |
| prism-context（M10） | prism-context | 导出文件 | Context Intelligence | **Extend**（消费 Core 关系图） | --json/--markdown 契约不变 | 低 | M27 |
| prism-snapshot（M11） | prism-snapshot | SQLite 三表 | Versioned Snapshot | **Extend**（新域 = 新 DomainRow + migration） | 历史快照可读（已有迁移测试） | 低 | M22+ 持续 |
| prism-store（SQLite） | prism-store | 4 表 | Persistence（独占） | **Retain** | schema_migrations 框架 | 低 | 持续 |
| 路径身份 | capture/CLI/GUI | canonical_root 列 | 统一 canonical 口径 | **Already migrated（M20）** | 非规范历史行按原字面量仍可查（见 §4） | 低 | ✅ 39c639b |
| MCP | —（不存在） | — | read-only MCP server | **新建** | 无历史包袱 | — | M27 后候选 |
| Godot 支持 | —（不存在） | — | Godot Adapter | **新建** | — | — | M24 |
| C# 支持 | Language 枚举仅识别 | — | C# L1+L2 | **新建** | — | — | M23/M25 |
| `report` 占位命令 | prism-cli | — | 待评估（可能被 history export 取代） | **Defer** | exit 1 现状 | 无 | 2.0 后段 |

## 3. Identity / Freshness / Cache / Snapshot 策略（M20 §十五/§32/§33）

- **Identity**：Project = canonical root（2.0-M20 已统一）；File = 项目相对路径（规范化分隔符）；Symbol = 沿 1.0 `symbol:{path}#{qualified}#{kind}` 模式扩展 signature 分量（M23 定稿）；Relation/Finding = 稳定 id（fnv1a，沿 finding_fingerprint 先例——无行号、内容寻址）。禁止 display name / array index / 自增主键作通用身份。
- **Freshness**：统一四态 fresh/stale/invalid/unknown（EVIDENCE_CONTRACT §5）；所有权 = 数据生产者；新分析层必须答五问（复用/重算/stale/invalidate/取消）。
- **Cache**：session 同源复用（现状 RETAIN）；跨进程持久缓存仅在证明必要后引入（YAGNI，§52）。
- **Snapshot**：新域走 DomainRow + 域指纹既有模式；schema_version 变更 ⇒ compare 报 limited（§71 先例）；Manual First 不变。

## 4. 一次性迁移说明（2.0-M20 路径归一化）

- FACT：commit 39c639b 起，snapshot 注册/查询统一 canonical 根（canonicalize + 去 verbatim）。
- 兼容性：**以历史字面量（如 `.`、正斜杠形态）写入的旧项目行不会自动改写**——按原字符串查询仍可见；用规范绝对路径的新捕获会建立规范身份并向前收敛。无 schema 变更；无需数据回填（本地缓存性质数据，Manual delete 可清理旧行）。

## 5. 回滚 / 失败策略

- 代码迁移：每域独立 PR/commit，回归不过即回退单域，不阻塞其他域。
- 数据迁移：schema_migrations fail-stop + 损坏库显式报错（§157 测试在案）；迁移前无自动备份义务（本地缓存），但 migration 必须可重入幂等。
