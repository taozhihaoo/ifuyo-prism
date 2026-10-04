# Prism Snapshot（M11）

> **Prism 的项目长期记忆层：对已分析的工程事实做持久化，并支持跨时间比较、追踪和回顾。**
> Snapshot 是"记忆"，不是"备份"（§177）。

## What Snapshot is（§291）

一次 capture 冻结 Prism 已分析出的工程事实为一个历史状态：

- 稳定项目身份（canonical root 首见注册，ProjectRegistry）
- 十域状态 + 域指纹 + 组合状态指纹
- basis：GitRevision（干净树）或 WorkingTree（含未提交变更）
- 捕获时间、git revision/branch、counts（事实统计）

## What it stores

- 规范化域 JSON（structure 为 M1 事实；其余域由调用方注入时捕获）
- 十域指纹 + state_fingerprint（canonical JSON → fnv1a64）
- limits/limitations/note（用户备注）

## What it does NOT store（§29/§30/§292）

- **完整源码**——Git revision 快照记录 SHA，历史源码用 `git show`；dirty 快照只存结构事实
- secrets（写入前过中央 redaction）
- raw env values / credential stores
- 完整 ContextPack 内容（只存 manifest/fingerprints/summary）

**Working Tree Snapshot 无法保证未来恢复当时未提交源码**（§292）。

## Where data is stored（§21）

`%APPDATA%\Prism\prism.db`（Windows）/ `XDG_DATA_HOME|~/.local/share/Prism/prism.db`（Unix）。
`PRISM_DB_PATH` 可覆盖。**不在项目仓库里**（§21：不污染、不被提交）。

## How history works

- Manual Snapshot First（§147）：无后台自动快照
- 去重：状态未变 ⇒ 返回既有 snapshot（§17），`--force` 强制新 observation（§18）
- 删除：`prism history delete --yes`（§62），只动 SQLite（§116）

## How comparison works（§31–§42/§113）

两个持久化快照的规范化事实直比（不 checkout、不重跑分析器）：

- 每域 facts：added/removed/scalar 变化
- Finding 生命周期：fingerprint = rule + subject + path（**不含行号**，§37/§134）
- verification 状态迁移只列事实（§73）
- Unavailable ≠ empty（§245）；Unavailable/Stale ≠ Failed（§13）

## Privacy boundaries（§28/§163/§261）

- 值结构性不采集（M9 变量 keys-only）
- persist/export 前过中央 redaction（[REDACTED] 保留行结构）
- export 含 metadata（exported_at/schema/project id），用户显式操作（§260）

## CLI

```bash
prism snapshot <path> [--force] [--note "text"] [--json]
prism history <path> [--json] [--limit N]
prism history export <path> [--output history.json]
prism history delete <path> (--snapshot <id> | --all) --yes
prism compare <path> <snapshot-a> <snapshot-b> [--json|--markdown]
prism timeline <path> [--limit N] [--json|--markdown]
```

Exit：0 成功/去重；3 路径不存在/参数错误；1 存储或生成失败（§119/§120：storage 失败 ≠ 分析不可用）。

## Limitations（§171）

- M2–M10 域默认 NotCaptured（调用方注入即捕获；M11b 后续接线）
- dirty 快照的历史源码不可恢复（只存结构事实）
- CTX 全量规则/自动 retention/加密：见 M11a/M11b gate P1
