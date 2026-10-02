# M2 Final Audit — Code Understanding

- 日期：2026-10-03
- 依据：`Spec/ifuyo Prism — M2 Code Understanding 开发提示词.md` §64/§69/§70
- 基线：M1 HEAD `ba6d461` → M2 HEAD `f8c0cf7`（9 个子任务 commit）
- 结论标记：FACT / HYPOTHESIS / INFERENCE

## 1. Baseline

```text
branch: main   基线 HEAD: ba6d461（M1 完成态，工作树 clean）
当前 HEAD: f8c0cf7   工作树 clean
新增: crates/prism-parser（模型+registry+4 语言 adapter）、
      crates/prism-analysis（编排+CodeModel+Index）、
      11 个 M2 fixtures、10 个集成测试、GUI 符号浏览器
```

## 2. Architecture（§69 B）

FACT — 依赖方向单向：
`prism-cli / src-tauri → prism-analysis → prism-parser → prism-project → prism-core`。
语言私有 AST 只存在于 prism-parser 的 adapter 模块内，不外泄（规格 §9 无万能 AST）；
parser 不依赖 Tauri/React；前端经 IPC 访问 Code Model，不触文件系统。

FACT — 复用而非重写（§2.D）：文件选择直接消费 M1 `ProjectDiscovery` +
`FileClassification::Source && analyzable`（§5）；Generated 跳过策略复用 M1
分类（§46）；max_file_size 复用 M1（§29，too_large 集成测试验证）；
Evidence/SymbolId 复用 M0 ids + M1 模型锚定（§43）。

HYPOTHESIS：qualified_name 采用"路径根式"（如 `src/nested.Deep.Outer.inner`）
而非 C# 惯例的纯命名空间根——为稳定身份与跨语言一致性所做的偏离（规格 §17
示例为命名空间根式）；M5 匹配时可能需要按 language 特化比较逻辑。

## 3. Parser（§69 C）

```text
supported:    TypeScript / TypeScriptReact / JavaScript / Python / C#（5 个 parser，
              TS/TSX/JS 共享一个 walker，不同 grammar）
unsupported:  Rust / Go / Java / C / C++ 等已识别语言 → Unsupported/no_parser
              （§35，M1 未识别的语言如 Ruby 不进入代码模型）
parse success: 自扫描 70 源文件 → 20 parsed / 50 unsupported / 0 failed（FACT）
failure cases: 语法错误 → SuccessfulWithDiagnostics + 部分符号保留（§24）；
               非 UTF-8 → Failed/unsupported_encoding（§45）；python MISSING
               节点诊断 + has_error 兜底（不吞错误）
```

## 4. Symbol Model（§69 D）

FACT：Symbol = id/name/qualified_name/kind/language/file_id/parent_id/
visibility/range/signature/documentation_present（20 种 SymbolKind 全集定义，
adapter 只映射可靠识别的 P0 子集）；children 由 SymbolIndex 提供（无双份簿记）。

FACT — 稳定 SymbolId：`symbol:{relative_path}#{qualified_path}#{kind}`
（单元测试锁定确定性 + kind/file 敏感性）。

FACT — 坐标契约（§14/§15）：line 1-based / column 1-based UTF-8 字节列 /
byte_offset 绝对字节；`python_unicode_locations_are_byte_based_and_consistent`
（中文函数名列=1、字节=0）与 unicode fixture 集成测试锁定。

FACT — 索引完整性：SymbolIndex 五类查询（id/file/parent/children/qualified/name）
有测试；index 为派生内存结构不参与序列化。

## 5. Diagnostics（§69 E）

FACT：fatal 仅限项目根无效（exit 3）；文件级 read/编码/解析失败 → 非致命
ParseResult；partial success 可表达（SuccessfulWithDiagnostics + 保留符号，
syntax-errors fixture 集成测试验证 Broken 类 + ok 方法保留）；unsupported/
skipped 均带 reason 字段（no_parser / generated / too_large / cancelled）。

## 6. CLI（§69 I）

```text
prism inspect .                 → 文本摘要（§40 格式），exit 0        PASS
prism inspect . --format json   → ProjectCodeModel（schema_version=1）PASS
prism inspect <不存在路径>       → exit 3                             PASS
```

## 7. GUI（§69 J）

FACT：`tauri dev` 启动成功（进程 + Vite 200 + 后端日志）；新命令
start_inspect/get_inspect_status/get_code_model/cancel_inspect；
Understand 页面成为第一个完全真实数据页面：code summary →（扫描完成后自动
链式 inspect）→ 每文件符号树（父子缩进、点击符号显示 Kind/Visibility/
QualifiedName/Location/Signature/Documented 详情卡）→ 解析诊断列表 →
M1 目录树与文件列表保留。渲染上限 500 符号如实说明。

FACT（诚实声明）：符号点击→详情卡的端到端交互由组件层实现、前端测试 8/8
覆盖 store/summary 流程，人工可视化确认待做（同 M1 对话框项）。

## 8. Tests（§69 H）

```text
Rust：prism-core 54 + prism-project 48 + prism-parser 12(+7 contract) +
      prism-analysis 5 + prism-cli 13 + M0 集成 10 + M1 集成 9 +
      M2 集成 10 = 168 全部通过（cargo test --workspace）
前端：8/8；typecheck/build 通过；clippy 0 warning；fmt --check 通过
contract tests（§52）：5 语言 ×（language/valid/invalid/location/
hierarchy/recovery/determinism/cancellation）由参数化套件统一执行
```

## 9. Performance（§69 G）

```text
自扫描：70 源文件（读 20 个）→ 269 符号，CLI 总时长 < 2s（含 cargo run）
每次解析 read once / parse once / extract once（§54 禁止模式未出现）
有界并发：default min(cores,4) workers；12 文件 / 3 worker 集成测试通过
```

INFERENCE：十万文件级项目需缓存（M13/性能阶段），当前架构已留
AnalysisCacheKey 接口（§32）。

## 10. Security / Privacy（§69 K）

FACT：无网络依赖新增；文件内容按需读取且解析后丢弃（AST 不驻留，§55）；
日志不含源码/密钥；JSON 输出全部项目相对路径；无 telemetry/upload/AI
（§61：Symbol truth 来自 parser，无 LLM）。

## 11. Git（§69 L）

```text
branch: main   工作树 clean   未 push
M2 提交（9 个，子任务粒度）：
  748098b feat: add prism-parser model layer
  b2aa10c feat: add tree-sitter language adapters（TS/TSX/JS/Python/C# 五个
          parser 与统一契约测试；TS 子任务的独立提交 eab441c 因随后一次
          amend 失误被并入本提交——内容完整，历史不再改写，如实记录）
  896f054 feat: add prism-analysis orchestration layer
  896f054 feat: add prism-analysis orchestration layer
  de1bd35 feat: implement prism inspect CLI
  045e49f feat: add inspect IPC and symbol explorer in the GUI
  c506789 test: add M2 fixtures and code understanding integration suite
  4876e37 docs: record M2 pipeline, coordinate contract and crate map updates
  f8c0cf7 chore: remove leftover debug probe example
secrets 扫描：无命中（CRLF fixture 按字节入库，.gitattributes 豁免）
```

## 12. Risks

### P0
- 无

### P1
- GUI 符号交互的人工可视化确认待做（同 M1 对话框项）
- Rust parser 未实现（M2 范围外）→ 自扫描 50 文件 unsupported；Prism 自身
  Rust 代码的符号理解需后续语言 adapter（架构已就绪，新语言 = 新 adapter）
- qualified_name 的路径根式与规格 §17 示例（命名空间根式）有偏离（记录于
  architecture.md 决策 10）；M5 ChangeSet 匹配时再评估
- 参数/局部变量/decorator/常量抽取为 P1 deferred（§65 允许）

## 13. Deferred

依赖图/架构（M3）、Git（M4）、ChangeSet（M5）、Impact（M6）、Health（M7）、
Verify（M8）、Environment（M9）、Context（M10）、Snapshot（M11）——
均未提前实现；更多语言（Rust/Go/Java/C/C++）、完整局部变量、signature
normalization 属 P1（§65）。

## 结论

INFERENCE：M2 = COMPLETE（§64 P0 清单 32 项全部满足）。代码事实层
（Files → Language → Parser → Symbols → Hierarchy → Index → CodeModel）
稳定、可测试、可取消、可序列化、无 AI 依赖，可直接作为 M3 依赖分析的输入。
