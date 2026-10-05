# PRISM_2_0_DEPLOYMENT.md — Developer Deploy 契约（2.0-M214）

> 2.0-M214 产物。§74 的十份文档（DEPLOYMENT_MODEL/SECURITY/POLICY/
> ARTIFACT_MODEL/TARGET_MODEL/DEPLOYMENT_STATE/RECOVERY/TEST_MATRIX）按既有
> 惯例合并为本契约 + `docs/audits/2.0-M214-final-audit.md`（Final Report）。

## 1. 定位（§一/§二/§119）

`prism-deploy` crate（第 26 个成员）+ CLI `prism deploy` 命令族。**受控
部署系统，不是远程 Shell**（§二）：把已分析/验证的 Artifact 以可审计、
可验证、可回溯的方式部署到开发者控制的目标。

- **Credential Reference 边界**（§十，P0）：只存 `os-keychain://` /
  `env://` / `ssh-agent` / `Unavailable` 引用——**绝不**发明明文密码库。
- **零任意远程执行**（§二十/§四十五）：Local transport 零 shell；
  SFTP/SSH = Unsupported 如实（ssh2 离线不可用——P1 升级路径，Local 是
  §63 要求的真实 transport integration path）。
- **Atomic 如实**（§二十二）：Local = releases/current 布局；Windows
  rename 非严格原子 ⇒ 如实（capability 报告）。
- **Plan 与 Execute 分离**（§五十）：`plan` 零动作；`run` 须 `--confirm`；
  policy FAIL ⇒ 阻断不执行。

## 2. 数据契约（§五-§十一）

```text
ArtifactIdentity { artifact_id, kind(File|Directory), sha256(§六 SHA-256),
                   git_commit(§16 注入，不猜) }
ArtifactManifest { artifact, name, source, files[](§七 sorted+hashed),
                   total_bytes, generated_at, diagnostics }
Target           { target_id=fnv(kind|host|root), kind(Local|Sftp),
                   remote_root(§46 一切路径解析于此), environment,
                   credential: Option<CredentialRef>, transport_capability }
DeploymentProfile{ require_clean_git/require_artifact_hash/
                   block_critical_findings }              // §61 data-driven
DeploymentPlan   { preflight, policy_decision(PASS/WARN/FAIL/UNKNOWN §61),
                   confirmation_required, reasons[](§80 可展开), summaries }
DeploymentRecord { deployment_id, release_id, artifact_sha256, state(14 态
                   §54), failure_stage(13 阶段 §26), reasons }   // §30/§41 一等
```

## 3. 状态机与执行（§十二/§五十四/§五十七）

`Planned → Preflight → (AwaitingConfirmation) → Transferring → Applying →
Verifying → Succeeded`；异常 ⇒ Failed/PartiallyApplied/Cancelled/RolledBack。

`execute`（§50）：policy FAIL ⇒ Preflight 阶段阻断、**零 transfer**；
Transfer 逐文件 `TransferItem`（§五十七：73 成功 27 失败可回答）；
Apply = 原子 activate（releases/current）；Verify = hash match +
activation。取消 ⇒ Cancelled/PartiallyApplied（§27 不得写 Cancelled=Safe）。

## 4. Path Safety（§四十六）

`resolve_under_root`：`../`/绝对路径/`C:\`/空路径全部拒绝（测试矩阵锁定）；
release_path/current_path 有界；symlink 逃逸 → skip + diagnostic（§46）。

## 5. Policy（§六十一）

data-driven：`require_clean_git` / `require_artifact_hash` /
`block_critical_security_findings` / `block_unknown_preflight`（§14
UNKNOWN 默认确认不伪装 FAIL——除非 policy 要求 BLOCK）。结论 reasons 可
展开（§80）。M28/M29 findings 经 critical_findings 参数消费（§39 不重做
Quality Gate）。

## 6. History / Rollback / Diff（§二十九-§三十一/§四十一）

- History 一等：每次部署（含 Rollback）追加 DeploymentRecord；JSON 落盘
  重启保留（测试锁定）；SQLite DEFER P1（沿先例）。
- Rollback（§二十九）：回退到 last_successful 的 release，生成**新**
  Record（RolledBack），不删历史；`--confirm` 破坏性确认。
- Diff（§三十一）：A↔B 的 artifact/release/state 变化。

## 7. CLI（§49/§50/§55）

```text
prism deploy . artifact <source> [--name] [--git-commit] [--json]
prism deploy . plan <source> --target-root R [--environment dev]
                     [--git-commit] [--git-dirty] [--critical-findings N] [--json]
prism deploy . run <source> --target-root R --confirm [...] [--json]
prism deploy . history [--json]
prism deploy . rollback <release> --confirm [--reason]
```

## 8. 边界与遗留（§二/§111 DoD 对照）

- 已满足：Artifact 模型/Manifest/稳定身份/Git linkage/Target/Profile/
  Credential 边界/Preflight/Plan/Policy/Local transport/状态机/结构化
  失败/取消/Partial/Rollback/History/Diff/确定性/路径安全/redaction 边界
  （secret 经上游 redaction）/CLI/垂直切片。
- P1（有证据）：① SFTP/SSH transport（离线缓存无 ssh2——Local 是真实
  transport path §63）；② GUI Deploy View（§48）；③ MCP 只读 inspection；
  ④ SQLite 持久化 + migration（§40）；⑤ health-check verification
  provider（复用 M213 §24）；⑥ Snapshot Deployment 域（§32）；⑦ M27
  Context 接线（§17）。
- 不做（§二）：CI/CD SaaS、K8s、云平台、AI 自主部署、任意远程 Shell。

## 9. 实测证据（§92 垂直切片）

`prism deploy . artifact dist` ⇒ sha256 + sorted manifest ⇒ `plan`（policy
评估 + preflight 3 checks + reasons）⇒ `run --confirm` ⇒ Succeeded 2/2
files atomic=true verified=true ⇒ history 落盘；无 `--confirm` ⇒
"requires --confirm" 门控；policy FAIL ⇒ 零 transfer 阻断。
- prism-deploy 21 项单测（manifest 确定性/内容变化身份/symlink/路径逃逸
  矩阵/roundtrip/SFTP Unsupported/FAIL 阻断/dirty git 门控/UNKNOWN 默认
  确认/critical 阻断/执行成功链/policy 阻断执行/历史重启保留/last_
  successful/diff）。
- 全量：clippy 0 / fmt 0 / **804 Rust**（M213 末 783 + deploy 21）+
  61 vitest。
