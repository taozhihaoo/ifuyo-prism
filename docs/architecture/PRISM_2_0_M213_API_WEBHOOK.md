# PRISM_2_0_M213_API_WEBHOOK.md — API / Webhook Debugging 契约（2.0-M213）

> 2.0-M213 产物。§85 的九份文档（API_MODEL/WEBHOOK_MODEL/NETWORK_SECURITY/
> API_CORRELATION/API_REPLAY/API_FINDINGS/API_TEST_MATRIX）按既有惯例合并为
> 本契约 + `docs/audits/2.0-M213-final-audit.md`（Final Report）。

## 1. 定位与安全边界（§0/§3/§18-§20/§42）

`prism-api` crate（第 25 个成员）：把一次 API / Webhook 运行事件作为**工程
事实**纳入统一知识体系。零新重量依赖——HTTP/1.1 客户端/接收器基于
std::net（https ⇒ Unsupported 如实，P1 reqwest）；HMAC-SHA256 按 RFC 2104
手写（sha2 唯一新依赖，RFC 4231 三向量锁定）。

- **Network Safety P0**（§18-§20）：SSRF guard——scheme 白名单（http，
  https⇒Unsupported）+ host 白名单（localhost/127.0.0.1/::1 first）+
  私网/metadata 字面量 PrivateAddressBlocked + userinfo 拒绝 + 不跟随
  重定向；一切网络动作显式 CLI 调用。
- **Secret Safety P0**（§12-§14）：header 名单 + M29 `redact_line` 全链；
  fingerprint 不含 secret；签名六态（NotPresent/Present/Validated/Invalid/
  Unsupported/Unknown——§11 Unknown ≠ Invalid）。
- **不建第二模型**（§77/§79/§80）：Finding/Correlation/Redaction/Debug
  全复用 M21/M28/M210/M29；无 God Service（按能力分 discover/observe/
  receive/correlate）。

## 2. 模型（§6-§11/§71 数据边界）

```text
ApiEndpoint     { endpoint_id=api:{fnv}, method, route_template(归一化 {id}),
                  source(SourceCode/Config/Runtime/OpenApi/UserInput), evidence=文件:行,
                  handler_symbol(INFERENCE) }
ApiObservation  { observation_id, method, url, matched_endpoint(INFERENCE),
                  status, failure(六类:None/Transport/HttpStatus/Parsing/
                  Timeout/Cancelled——§9 不统一), latency_ms, observed_at,
                  correlation_id }
WebhookEvent    { event_record_id, event_type, payload(redacted),
                  signature(六态 §11), processing(Received/Parsed/Rejected/
                  Succeeded/TimedOut/Duplicate/Unknown), correlation_id,
                  handler_symbol }
ReplaySafety    { Safe(GET/HEAD/OPTIONS) | PotentiallyUnsafe(POST/PUT/PATCH)
                  | Blocked(DELETE) | Unknown }                    // §31
```

## 3. Endpoint Discovery（§15/§16）

- **ASP.NET Core**（第一优先，§15 按真实框架）：`[HttpGet]`/`[HttpPost]`/
  `[Route]` 属性 + 类级前缀组合（`[Route("api/[controller]")]` ⇒ 类名去
  Controller 后缀）+ 路径参数归一 `{id}`/`:id`/`<id>` ⇒ `{id}`（§7）。
- **Express**：`app.get("/x")` / `router.post` / `server.`。
- **FastAPI**：`@app.get("/x")` / `@router.post`。
- 证据 = `file:line`（§16 Source=SourceCode）；确定性排序 (method, route)。

## 4. HTTP/1.1 有界观测（§17-§20/§47）

`observe(guard, method, url, headers, body, limits, cancel)`：目标先过
TargetGuard（§19 SSRF 拒绝矩阵测试）→ TcpStream connect（refused/timedout
即时失败，其他瞬时错误重试至 deadline）→ HTTP/1.1 请求/有界响应读取
（256 KiB ⇒ truncated §47）→ redact headers + body（§12）→ FailureKind
分类（Transport/HttpStatus/Parsing/Timeout/Cancelled——§9 不统一）。
重定向**不跟随**（§19 redirect chain）。https ⇒ Transport + "scheme
unsupported" 诊断（§18）。

## 5. Webhook Receiver（§21/§22/§63）

`WebhookReceiver::start(config, cancel)`：**恒绑 127.0.0.1**（§21 永不
0.0.0.0）+ 显式 start/stop + 有界 payload(256 KiB)/events(100)/时长；
accepted stream **set_nonblocking(false)**（修复：listener nonblocking
继承导致假 EOF 丢请求——§63 disconnect 场景测试锁定）。签名 HMAC-SHA256
校验（constant-time eq §11）→ 六态；Rejected ⇒ 500 + Connection: close。
payload redact 后存储（§12 测试断言无原文）。无 secret ⇒ 签名 Unknown
（§11 不是 Invalid）。

## 6. Correlation（§23-§27）

- `match_endpoint`：method + path 段匹配（大小写不敏感 §24 ASP.NET 语义；
  `{id}` 通配；INFERENCE 非 FACT）。
- `correlate_request_logs`：correlation id 匹配 M210 日志行 ⇒ (source,line)。
- `correlate_error_cluster`：correlation id 匹配 M210 Error Cluster。
- `correlate_handler_symbol`：端点证据文件 → M23 by_file 首个 class/
  struct/record 类型符号（namespace 不算）。
- `correlate_recent_change`：M211 file_history 最近 commit（**temporal
  correlation not causal** §27，CLI 输出注明）。

## 7. health2 API 规则（§28/§41/§63）

| 规则 | 严重度 | 条件 |
| --- | --- | --- |
| `API/UNEXPECTED_STATUS` | Warning | HTTP 失败观测（status ≥ 500 或非状态类失败）；Origin=Fact（§5 HTTP 500 是 Runtime FACT） |
| `API/REPEATED_FAILURE` | Warning | 同 (method,url) 失败 ≥ api_repeated_failures(3)；聚合 §46 |

Metrics：`api.failure_observation.count`、`api.repeated_failure.count`。
with_all_builtins = **19 条内建规则**（10 categories）。

## 8. CLI（§54/§55）

```text
prism api . endpoints [--json]                        # 发现（零网络）
prism api . observe --url http://127.0.0.1:5000/x     # 显式观测（SSRF guard）
           [--method GET] [--header name=value] [--json]
prism api . correlate --url http://.../api/cards/42   # 垂直切片 §70
           [--method GET] [--max N]
prism api . webhook-listen --port 8085 [--secret S]   # 显式接收器（§21）
           [--signature-header H] [--duration-secs N] [--json]
```

垂直切片实测（§92 场景 A/B）见 Final Audit §16-§17。

## 9. Replay Safety（§31）

`HttpMethod::replay_safety()`：GET/HEAD/OPTIONS = Safe；POST/PUT/PATCH =
PotentiallyUnsafe；DELETE = Blocked。v1 observe 命令固定 GET（不自动发
写请求）；replay 命令 DEFER P1（§31 完整语义需 idempotency/auth 上下文）。

## 10. 边界与遗留（§3/§46/§53/§57/§77）

- GUI/MCP：DEFER（P1，沿 M21-M212 先例）；CLI 是 §54 命令族的可用面。
- SQLite 持久化 / Snapshot 域 / retention（§46/§47/§174）：DEFER P1
  （报告 JSON 即 v1 持久化）。
- https 观测 / OpenAPI 导入 / 签名 header 多格式 / Timing 分解（§35
  Observed/Unsupported 分离已建模）：DEFER/P1。
- 不可信输入（§75）：所有解析为数据切分；CLI 输出走 println（自动转义
  ANSI/HTML 不适用——终端文本）；export 未实现（P1 实现时须 §58 redaction）。

## 11. 实测证据（§92 验收场景）

- 场景 A（API Debug）：ASP.NET fixture 3 端点发现（[controller] 前缀组合 +
  路径参数归一）→ correlate GET /api/cards/42 ⇒ GET /api/Cards/{id} ⇒
  handler Game.Api.CardsController ⇒ recent change 548b7aa（temporal
  not causal）。
- 场景 B（Webhook Debug）：listen --secret ⇒ valid sig ⇒ 200 Validated +
  invalid sig ⇒ 500 Rejected + missing sig ⇒ NotPresent；payload redact
  后存储（测试断言无原文）；nonblocking 继承 bug 修复（§63 disconnect）。
- 全量：clippy 0 / fmt 0 / **783 Rust**（M212 末 751 + api 32）+ 61 vitest。
