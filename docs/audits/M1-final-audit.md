# M1 Final Audit — Project Discovery

- 日期：2026-10-03
- 依据：`Spec/ifuyo Prism — M1 Project Discovery 完整开发提示词.md` §87–§92
- 基线：M0 HEAD `76079bd` → M1 HEAD `995711d`（8 个子任务 commit）
- 结论标记：FACT / HYPOTHESIS / INFERENCE

## A. Baseline

```text
M0 HEAD: 76079bd（工作树 clean，cargo check 通过）
M1 HEAD: 995711d（工作树 clean）
新增:    crates/prism-project（12 个模块）、fixture-multi-language、
         discovery 集成测试套件、GUI Discovery 视图、tauri-plugin-dialog
```

## B. Architecture（§88 B）

FACT — 职责分离（§69），无 God Object：

```text
scanner.rs（遍历）→ ignore.rs（是否忽略）→ file.rs（分类）
language.rs（语言）→ project_type.rs（技术栈）→ configuration.rs（配置）
entry_point.rs（入口候选）→ statistics.rs（统计）→ service.rs（编排）
```

FACT — 依赖方向单向：`prism-cli / src-tauri → prism-project → prism-core`；
prism-project 不依赖 tauri/前端。Project 从 prism-core 迁入 prism-project
（文档漂移以代码为准，用户决策 2026-10-03）。

## C. Discovery Accuracy（§88 C）

FACT — 自扫描（Prism 扫描自己，§81，`prism scan .`，exit 0）：

```text
Project: ifuyo-prism   Files: 165   Directories: 49   Ignored: 6   Errors: 0
Languages: Rust 40, Markdown 22, TypeScriptReact 14, TypeScript 12,
           JSON 9, TOML 7, JavaScript 1, Python 1, HTML 1, CSS 1
Project types: Node.js / Rust / Tauri(2 evidence) / TypeScript = High；
               React / Vite = Medium（全部带 Evidence）
Entry points: src-tauri/src/main.rs (high, RustBinary),
              src/main.tsx (high, Frontend), index.html (medium, Frontend)
```

FACT — fixture-multi-language：9 文件 / 4 目录全部正确发现；Tauri/Rust/
Node.js/TypeScript 候选 + 证据、入口 `src/main.ts` 与
`src-tauri/src/main.rs`、5 个配置识别、Test/Documentation 分类均由
`multi_language_fixture_full_discovery` 断言通过。

## D. Ignore Accuracy（§88 D）

- included：`.gitignore`、`.env.example` 等 hidden 文件默认包含（include_hidden=true）
- ignored：`.git/node_modules/target/bin/obj/.venv/venv/__pycache__/dist/build/coverage/.cache` 等 24 项系统安全规则 + gitignore + 用户规则
- FACT：系统规则不可被 `.gitignore` 取反（`system_rules_override_gitignore_whitelist` 测试）
- symlink：记录为条目（is_symlink=true，不可分析），目录链接不递归（含 Windows 权限不足时的跳过逻辑）
- large file：3MB 文件 discovered=true / analyzable=false / reason=too_large（集成测试）
- binary/generated：按扩展名与目录段分类，reason 可解释

## E. Error Handling（§88 E）

- Fatal：根不存在（NotFound→exit 3）、根不可读（Io→exit 1）
- Non-fatal：read_dir/metadata 失败累积为 `ScanError{path,kind,message}`，
  status 仍为 Completed（warnings>0 → exit 5 / GUI "completed with warnings"）
- Cancelled：保留部分事实，status=Cancelled（exit 4）

## F. Determinism（§88 F）

FACT — 对 Prism 自身连续两次 `scan --format json`，剥离运行期字段
（scan_id/时间戳/duration/project id）后 JSON 完全相等（deterministic: YES）；
fixture 的 `repeated_scans_are_structurally_deterministic` 集成测试同过。
文件/目录按相对路径字典序、语言按数量降序+名字、类型/入口按置信度降序+名字。

## G. Performance（§88 G）

```text
Prism 自身：165 文件 / 49 目录 / 忽略 6 项；CLI JSON 输出 < 1s（含 cargo run 启动）
GUI 扫描在后台线程执行（UI 不冻结）；进度为真实计数，无伪百分比
```

INFERENCE：十万文件级项目的性能优化（并发 walker、缓存）未做也无阻塞迹象——
按纲领 §54 先保证正确/取消/确定性。

## H. CLI（§88 H）

```text
prism scan .                → 人类可读摘要，exit 0      PASS
prism scan . --format json  → schema_version=1 完整契约  PASS
prism scan <不存在路径>      → exit 3（invalid input）   PASS
退出码契约：0 ok / 1 fatal / 2 usage / 3 invalid / 4 cancelled / 5 warnings
```

## I. GUI（§88 I）

FACT：`tauri dev` 启动成功（进程运行、Vite 200、后端日志正常）；
Overview 空状态/扫描卡（真实计数+取消）/结果摘要（类型+证据展开、语言、入口）、
Understand 目录树+文件列表（过滤/排序、500 行截断如实说明）已实现，
前端组件测试 8/8 通过（含打开→扫描→摘要的 mock 流程、取消选择、
未扫描时 Understand 的诚实提示）。

FACT（诚实声明）：真实目录选择对话框（tauri-plugin-dialog）的端到端点击流
未做自动化验证——需人工在 `npm run tauri dev` 中确认一次。
INFERENCE：对话框为官方插件 + 标准 capability 配置，风险低。

## J. Privacy（§88 J）

FACT：无网络调用（无任何网络依赖引入）；不读取文件内容（仅 metadata）；
日志不含源码；扫描只访问用户指定的 root（symlink 不跟随）；
无 telemetry/upload。

## K. Git Hygiene（§88 K）

```text
branch: main   工作树 clean   未 push
M1 提交（8 个，子任务粒度）：
  b36e737 feat: add prism-project domain models
  baad0a3 feat: add project type and entry point detectors
  f407dea feat: add discovery scanner, statistics and result assembly
  fc3f78f feat: implement prism scan CLI
  5599cf1 feat: add scan IPC and discovery views in the GUI
  b6e7ad6 test: add multi-language fixture and discovery integration suite
  c4dbb99 docs: record M1 pipeline, ignore precedence and crate map updates
  995711d refactor: resolve M1 clippy warnings
secrets 扫描：无命中
```

## L. Tests

```text
Rust：prism-core 54 + prism-project 48 + prism-cli 10
      + M0 集成 10 + M1 集成 9 = 131 全部通过（cargo test --workspace）
前端：8/8 通过；typecheck / build 通过
clippy：0 warning；cargo fmt --check 通过
```

## M. Risks（仅 P0/P1）

- **P1** GUI 目录选择对话框端到端人工验证待做（见 I）
- **P1** 嵌套目录的 .gitignore 未读取（只读项目根）；多层 monorepo 场景后续里程碑处理
- **P1** package.json 的 main/bin/exports 入口未利用（M1 不解析内容，符合 §24 边界；
  M10 Context Health 可消费）
- FACT：无 P0（模型、遍历、ignore、确定性、Core 边界全部可靠）

## N. Deferred（§88 M）

AST / Symbols（M2）、Dependency/Architecture（M3）、Git 分析（M4）、
Health（M7）、Verify（M8）、Context（M10）、快照历史（M11）——
均未提前实现。LOC 统计按 §31 推迟。

## O. 结论

INFERENCE：M1 = COMPLETE（§87 清单 27 项全部满足；发现层可作为 M2
Code Understanding 的可信输入）。
