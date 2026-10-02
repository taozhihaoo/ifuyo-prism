# -*- coding: utf-8 -*-
"""Mechanically split the M11 spec into two lossless parts."""
import io
import re

SRC = "Prism M11 — Project Snapshot - Timeline - Evolution Engine 完整开发提示词.md"
OUT_A = "Prism M11a — Snapshot Capture and Storage 完整开发提示词.md"
OUT_B = "Prism M11b — Snapshot Compare Timeline and Evolution 完整开发提示词.md"

text = io.open(SRC, encoding="utf-8", newline="").read()
lines = text.splitlines(keepends=True)

# 1) section starts: only level-1 headings `# N. `
starts = []
for i, line in enumerate(lines):
    m = re.match(r"^# (\d+)\. ", line)
    if m:
        starts.append((int(m.group(1)), i))
nums = [n for n, _ in starts]
assert nums == list(range(len(nums))), f"numbering not contiguous: {len(nums)}"

first = starts[0][1]
preamble = "".join(lines[:first])

spans = {}
for idx, (no, i) in enumerate(starts):
    end = starts[idx + 1][1] if idx + 1 < len(starts) else len(lines)
    spans[no] = "".join(lines[i:end])

# 2) part assignment by content domain
a_ranges = [
    (0, 30), (63, 70), (81, 84), (91, 92), (103, 111), (114, 114), (116, 123),
    (137, 137), (141, 147), (149, 151), (157, 159), (164, 169), (171, 171),
    (176, 177), (183, 194), (196, 220), (222, 224), (226, 231), (239, 239),
    (241, 244), (248, 248), (252, 257), (270, 271), (275, 275), (296, 300),
    (323, 323),
]
a_set = set()
for lo, hi in a_ranges:
    a_set.update(range(lo, hi + 1))
b_set = set(nums) - a_set
assert not (a_set & b_set)
print(f"sections: total={len(nums)} a={len(a_set)} b={len(b_set)}")

HEADER_A = """# Prism M11a — Snapshot Capture and Storage（M11 拆分 Part 1/2）
## 完整、详细、专业开发提示词

> **【系列说明】** 本文档由《Prism M11 — Project Snapshot - Timeline - Evolution Engine 完整开发提示词》拆分而来（Part 1/2）。姊妹篇：`Prism M11b — Snapshot Compare Timeline and Evolution 完整开发提示词.md`（Part 2/2）。两篇合起来覆盖原文 §0–§335 全部内容，章节编号与标题保持原文不变（正文内 "§xx" 交叉引用继续有效）。本篇为独立可执行的子里程碑：完成【阶段性完成门槛】后即可按本篇指令本地 commit（不 push）。

**归属映射（原文 § → 本篇）：**

| 原文区间 | 内容域 |
| --- | --- |
| 引言、§0–§30 | 使命/原则/Git 边界/两种状态/ProjectSnapshot/Identity/Registry/Capture Pipeline/freshness/Domain State/Canonical Serialization/Fingerprint/去重/Storage/SQLite/存储位置/Storage 模块/表与 Schema/SnapshotManifest/持久化边界 |
| §63–§70 | Retention、Migration、Transaction Safety、Atomic Snapshot、Snapshot State、Schema Version、数据版本、Ruleset Version |
| §81–§84 | Persistence Policy、项目级 Disable、Offline First、AI Boundary |
| §91–§92 | Snapshot Note、Baseline Snapshot |
| §103–§111 | 依赖方向、建议架构、Snapshot Service API、Cancellation、Progress、Performance、Verification/Environment 默认行为、Capture Options |
| §114、§116–§123 | 不修改工作树、Delete 不等于修改、Rename/Move、Corruption、Read-only fallback、Persistence Failure、不吞错误、Capture Result |
| §137、§141–§147、§149–§151 | Snapshot Tags、Size/SQLite Index/Cleanup/Concurrency/Capture Lock/No Auto-Snapshot/Manual First、API 不依赖 React、Serialization Contract、Snapshot IDs |
| §157–§159、§164–§169、§171、§176–§177、§183–§194 | Migration/Dedup/Serialization 测试、Working Tree/Rename/跨平台/Symlink/Permission/Partial 测试、Limitations、Dirty Source/Purpose、Logging/Observability/Error Taxonomy/Domain Layer、执行纪律与 P0/P1/P2 |
| §196–§220、§222–§224 | Restart Persistence、多项目、同名、Isolation、并发、Crash、Migration、Old Snapshot、Fingerprint、Database Size、fixtures、可重复、Clock/Path/Database Path、File Mutation、No Network、No AI |
| §226–§231、§239、§241–§244、§248 | Store/Repository/SQL 边界/JSON Boundary/Immutability/Deletion、Branch Dimension、Branch Rename/Detached/No Git Repo/Gitless、Baseline Semantics |
| §252–§257、§270–§271、§275、§296–§300、§323 | Retention Warning/Database Health/Recovery/Schema Backup/Migration Logging/Encryption、Testing Philosophy/Test Count、.gitignore、向后兼容/稳定 API/Core Purity/Infra Isolation/Data Contract、Store 不主动运行分析器 |

**归 M11b 的部分（本篇不做）**：§31–§62（Snapshot Diff/各域 Diff/Finding Identity/History/Timeline/Compare/Evolution/Historical Query/全部 CLI 命令面）、§71–§80、§85–§90、§93–§102、§112–§113、§115、§124–§136、§138–§140、§148、§152–§156、§160–§163、§170、§172–§175、§178–§182、§195、§208–§215、§221、§232–§238、§240、§245–§247、§249–§251、§258–§269、§272–§274、§276–§295、§301–§322、§324–§335（含完整验收、Final Audit、最终 commit）。

---

"""

HEADER_B = """# Prism M11b — Snapshot Compare Timeline and Evolution（M11 拆分 Part 2/2）
## 完整、详细、专业开发提示词

> **【系列说明】** 本文档由《Prism M11 — Project Snapshot - Timeline - Evolution Engine 完整开发提示词》拆分而来（Part 2/2）。姊妹篇：`Prism M11a — Snapshot Capture and Storage 完整开发提示词.md`（Part 1/2）。两篇合起来覆盖原文 §0–§335 全部内容，章节编号与标题保持原文不变（正文内 "§xx" 交叉引用继续有效）。本篇是 M11 的收尾篇：完整验收清单、Final Audit 与最终 commit 指令在本篇。

> **【承接 M11a】** 本篇假设 Part 1 已实现并合入：ProjectSnapshot 与域 fingerprint、Canonical Serialization、prism-store（SQLite、migrations、事务与原子写）、捕获编排与去重、`prism snapshot` 与 `prism history` 基础面均已可用。本篇在其上完成比较、时间线、演化、导出与全部交付面。

**归属映射（原文 § → 本篇）：**

| 原文区间 | 内容域 |
| --- | --- |
| §31–§54 | Snapshot Diff、各域 Diff、Finding Identity、Health/Verification History、Environment/Context Health 历史、Timeline/TimelineEvent/事件类型、Compare UI/UX、Current Snapshot、Freshness-aware Comparison、Evolution Summary、禁单一 Score |
| §55–§62 | Historical Query、CLI 全命令面（snapshot/history/compare/timeline/export/delete） |
| §71–§80 | Compare Compatibility、历史 Finding/Verification 比较、各域 Evolution、Environment 与 Project Evolution 分离、Context History 隐私边界 |
| §85–§90 | 不允许 AI 重写历史、Provenance、Snapshot Provenance、不伪造历史、Git 历史补全、Timeline Source Label |
| §93–§102、§112–§113、§115 | 当前项目状态、Snapshot/Compare Detail、CLI JSON、Markdown 输出、Report Integration、M7/M8/M9/M10 关系、Diff 不重跑 M6、历史架构比较、Git Safety |
| §124–§136、§138–§140、§148 | Comparison/Timeline Result、历史数据不依赖 UI、GUI 页面与最低要求、Timeline UI、Snapshot List/Badge、Historical Health Detail、First Seen/Recurrence、Documentation/Context Health Evolution、No Cloud Sync/Collaboration/Upload、Recommended User Flow |
| §152–§156、§160–§163、§170、§172–§175、§178–§182 | Timeline Event IDs、Stable Ordering/Snapshot Ordering/Time Handling/Clock Uncertainty、Diff/历史 Finding/Freshness/Privacy 测试、Current 对比限制、Explainability/Evidence/Deep Link/Source Navigation、Backup/Export/Import/Data Ownership |
| §195、§208–§215、§221 | M11 完成标准、Diff Precision/History Query Performance/Privacy/CLI/GUI/Self-hosting/Real Project 测试、Git Safety Test |
| §232–§238、§240、§245–§247、§249–§251、§258–§269 | Timeline Derivation/Dedup/Historical Summary、No Hidden Inference、User Notes/System Facts、Provenance in UI、History Search、Cross-branch Compare、Unavailable/Zero Snapshot/First Diff、Current vs First、Timeline View/Historical Range、No Password Prompt/Project Privacy、Explicit/Sensitive Export/Export Metadata/History Restore、Release Semantics/M11 定位升级/用户价值/趋势图优先级/M12 输入 |
| §272–§274、§276–§295、§301–§322、§324–§335 | Required Test Matrix/Real-World/Git Commit 验收、Final Audit 全套、文档/ADR、ProjectSnapshot/SnapshotDiff/Timeline/EvolutionSummary 契约、开发顺序与 Phase 纪律、M5–M10 边界、最终结构/命名/清理/Git/Commit、最终汇报、Success Definition、最终定位、执行要求 |

**归 M11a 的部分（本篇不重复）**：§0–§30、§63–§70、§81–§84、§91–§92、§103–§111、§114、§116–§123、§137、§141–§147、§149–§151、§157–§159、§164–§169、§171、§176–§177、§183–§194、§196–§220、§222–§224、§226–§231、§239、§241–§244、§248、§252–§257、§270–§271、§275、§296–§300、§323。

---

"""

GATE_A = """

---

# 【阶段性完成门槛 — M11a】

在进入 M11b 之前，本篇必须满足：

1. 本篇归属章节中全部 P0 要求已实现：ProjectSnapshot 模型、域 fingerprint、
   Canonical Serialization、prism-store（schema/migrations/事务/原子写/并发锁）、
   捕获编排与去重、`prism snapshot` 与 `prism history` 基础命令。
2. Restart Persistence Test（§196）真实通过：建快照 → 重启进程 → 快照仍在。
3. 本篇范围测试全绿（§157–§159、§164–§171、§183–§194、§196–§224 中归本篇者）。
4. `cargo test --workspace`、clippy 0 warning、`cargo fmt --check` 通过；
   前端如涉及改动则 typecheck/test/build 通过。
5. Git Safety：捕获前后 `git status` / `HEAD` 完全一致；项目工作树零修改。
6. 输出【M11a 阶段审计】（FACT/HYPOTHESIS/INFERENCE，只列 P0/P1）。
7. 本地 commit：`feat: add snapshot capture and storage (M11 part 1/2)`。不 push。
   随后执行姊妹篇 `Prism M11b — Snapshot Compare Timeline and Evolution 完整开发提示词.md`。
"""

a_body = [preamble] + [spans[n] for n in sorted(a_set)]
b_body = [spans[n] for n in sorted(b_set)]
content_a = HEADER_A + "".join(a_body) + GATE_A
content_b = HEADER_B + "".join(b_body)

io.open(OUT_A, "w", encoding="utf-8", newline="").write(content_a)
io.open(OUT_B, "w", encoding="utf-8", newline="").write(content_b)

# 3) lossless verification: preamble + reassembled sections == original text
reassembled = "".join([spans[n] for n in nums])
assert preamble + reassembled == text, "LOSSY: reassembly mismatch"
assert sorted(a_set | b_set) == nums and not (a_set & b_set), "coverage error"
print("lossless reassembly: OK")
print("lines a:", content_a.count("\n"), "lines b:", content_b.count("\n"))
