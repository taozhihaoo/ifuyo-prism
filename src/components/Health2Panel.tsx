import { useEffect, useState } from "react";
import { useProjectStore } from "../app/ProjectStore";
import CompletenessBanner from "./CompletenessBanner";
import {
  cancelHealth2,
  getCachedDomain,
  getHealth2Results,
  getHealth2Status,
  startHealth2,
  toPrismError,
} from "../services/ipc";
import type { Health2FindingDto, Health2ReportDto } from "../types/prism";

type LoadState =
  | { kind: "loading" }
  | { kind: "running" }
  | { kind: "loaded"; report: Health2ReportDto }
  | { kind: "cached"; report: Health2ReportDto; completedAt: string; stale: boolean }
  | { kind: "error"; message: string };

const GATE_LABEL: Record<string, string> = {
  pass: "PASS",
  warn: "WARN",
  fail: "FAIL",
  unknown: "UNKNOWN",
};

const LIFECYCLE_ROWS: { key: keyof Health2ReportDto["lifecycle"]; label: string }[] = [
  { key: "new_findings", label: "New" },
  { key: "persistent_findings", label: "Persistent" },
  { key: "resolved_findings", label: "Resolved" },
  { key: "regressed_findings", label: "Regressed" },
  { key: "suppressed_findings", label: "Suppressed" },
];

/**
 * 健康页 2.0 health2 轨道（P1-1）。
 *
 * 数据 = `prism_health2::analyze_project`——与 CLI `prism health2` **同一
 * 函数**（§71：GUI 与 CLI 输出一致由构造保证）。离面板取消（§102/§160）。
 * P2-2：会话无结果时先查会话缓存——命中显示"上次结果 + 重新运行"，
 * 不自动重跑（重开项目后秒级回显）；未命中才自动启动。
 */
export default function Health2Panel() {
  const { project } = useProjectStore();
  const [state, setState] = useState<LoadState>({ kind: "loading" });
  // 「重新运行」后跳过缓存（否则 idle 分支又命中缓存，永远不真跑）。
  const [skipCache, setSkipCache] = useState(false);
  // 触发轮询重启。
  const [pollTick, setPollTick] = useState(0);

  useEffect(() => {
    if (!project) {
      setState({ kind: "loading" });
      return;
    }
    setState({ kind: "loading" });
    return () => {
      void Promise.resolve(cancelHealth2()).catch(() => {
        /* idle 取消为无操作 */
      });
    };
  }, [project?.root_path]);

  useEffect(() => {
    if (!project?.root_path) return;
    let disposed = false;
    let timer: ReturnType<typeof setTimeout> | null = null;
    const poll = async () => {
      try {
        const status = await getHealth2Status();
        if (disposed) return;
        if (status.status === "running") {
          timer = setTimeout(poll, 300);
          return;
        }
        if (status.status === "idle" || status.status === "cancelled") {
          if (!skipCache) {
            try {
              const cached = await getCachedDomain("health2");
              if (!disposed && cached?.data.kind === "health2") {
                setState({
                  kind: "cached",
                  report: cached.data.report,
                  completedAt: cached.completed_at,
                  stale: cached.stale,
                });
                return;
              }
            } catch {
              /* 缓存读取失败 ⇒ 走原自动启动路径 */
            }
          }
          await startHealth2();
          timer = setTimeout(poll, 300);
          return;
        }
        if (status.status === "failed") {
          if (!disposed) {
            setState({ kind: "error", message: status.error ?? "health2 analysis failed" });
          }
          return;
        }
        const report = await getHealth2Results();
        if (!disposed) {
          if (report) {
            setState({ kind: "loaded", report });
          } else {
            setState({ kind: "error", message: "health2 analysis was cancelled" });
          }
        }
      } catch (err) {
        if (!disposed) setState({ kind: "error", message: toPrismError(err).message });
      }
    };
    void poll();
    return () => {
      disposed = true;
      if (timer) clearTimeout(timer);
    };
  }, [project?.root_path, pollTick, skipCache]);

  if (!project) {
    return (
      <div className="card" data-testid="health2-needs-project">
        <p className="muted">Open a project to run health2 here.</p>
      </div>
    );
  }
  if (state.kind === "loading" || state.kind === "running") {
    return (
      <div className="card" data-testid="health2-loading">
        <p className="muted">Running health2 rules (same pipeline as CLI `prism health2`)…</p>
      </div>
    );
  }
  if (state.kind === "error") {
    return (
      <div className="card" data-testid="health2-error">
        <p className="status-error">Health2 analysis unavailable: {state.message}</p>
      </div>
    );
  }

  return (
    <section className="stack">
      {state.kind === "cached" && (
        <div className="warning-box" data-testid="health2-cached-banner">
          <p>
            上次 health2 结果（缓存，可能来自上一会话） · 完成于{" "}
            {new Date(state.completedAt).toLocaleString()}
            {state.stale
              ? " · 工作区已变化，结果可能过期"
              : " · 工作区基线未变化"}
          </p>
          <button
            type="button"
            className="button"
            onClick={() => {
              setSkipCache(true);
              setState({ kind: "loading" });
              setPollTick((tick) => tick + 1);
            }}
            data-testid="health2-rerun"
          >
            重新运行
          </button>
        </div>
      )}
      <Health2Results report={state.report} />
    </section>
  );
}

/** 结果视图（fresh 与 cached 共用）。 */
function Health2Results({ report }: { report: Health2ReportDto }) {
  const [expanded, setExpanded] = useState<string | null>(null);
  const [showAllDiagnostics, setShowAllDiagnostics] = useState(false);

  // 规则执行诊断（前提跳过/读取跳过）⇒ 完整度横幅（P2-5 组件复用）。
  const diagnostics = report.diagnostics.map((detail) => ({
    kind: "health2-diagnostic",
    subject: null,
    detail,
  }));
  const evidenceById = new Map(report.evidence.map((evidence) => [evidence.id, evidence]));
  const lifecycleSets = new Map<string, Set<string>>(
    LIFECYCLE_ROWS.map((row) => [row.key, new Set(report.lifecycle[row.key])]),
  );
  const lifecycleOf = (finding: Health2FindingDto): string | null => {
    for (const [key, set] of lifecycleSets) {
      if (set.has(finding.id)) return key.replace("_findings", "");
    }
    return null;
  };
  const metrics = Object.values(report.metrics).slice(0, 8);
  const visibleDiagnostics = showAllDiagnostics
    ? report.diagnostics
    : report.diagnostics.slice(0, 5);

  return (
    <>
      <CompletenessBanner
        testId="health2-completeness"
        data={{
          domain: "health2 分析",
          completeness: report.diagnostics.length > 0 ? "partial" : "complete",
          diagnostics,
        }}
      />

      <div className="card" data-testid="health2-summary">
        <h2 className="card-title">Health2 Findings · {report.findings.length}</h2>
        <dl className="fact-grid">
          <div>
            <dt>Critical</dt>
            <dd>{report.findings.filter((f) => f.severity === "critical").length}</dd>
          </div>
          <div>
            <dt>Warning</dt>
            <dd>{report.findings.filter((f) => f.severity === "warning").length}</dd>
          </div>
          <div>
            <dt>Info</dt>
            <dd>{report.findings.filter((f) => f.severity === "info").length}</dd>
          </div>
          <div>
            <dt>New</dt>
            <dd>{report.lifecycle.new_findings.length}</dd>
          </div>
          <div>
            <dt>Persistent</dt>
            <dd>{report.lifecycle.persistent_findings.length}</dd>
          </div>
        </dl>
        <p className="muted small" data-testid="health2-rule-source">
          规则集：2.0 health2（engine <code>{report.rule_source.engine}</code> · 内置{" "}
          {report.rule_source.builtin_rules} 条）。与 CLI <code>prism health2</code> 同一管线
          （§71：输出一致）。
          {report.baseline?.kind === "previous_run" ? (
            <>
              {" "}生命周期基线 = 上次持久化结果（
              {new Date(report.baseline.captured_at).toLocaleString()}）——Persistent
              即跨会话仍然存在的 finding。
            </>
          ) : (
            <> 基线对比未提供 ⇒ 全部 New（首轮语义）。</>
          )}
        </p>
      </div>

      <div className="card" data-testid="health2-gate">
        <h2 className="card-title">
          Quality Gate · {GATE_LABEL[report.quality_gate.verdict] ?? report.quality_gate.verdict}
        </h2>
        <p className="muted small">
          new {report.quality_gate.new_count} · breaking {report.quality_gate.breaking_count} —
          Gate 是策略判断（默认 policy：max_new=0），不是健康分（§29-§30）。
        </p>
        {report.quality_gate.reasons.length > 0 && (
          <ul>
            {report.quality_gate.reasons.map((reason, index) => (
              <li key={index} className="muted small">
                {reason}
              </li>
            ))}
          </ul>
        )}
      </div>

      {metrics.length > 0 && (
        <div className="card" data-testid="health2-metrics">
          <h2 className="card-title">Metrics</h2>
          <dl className="fact-grid">
            {metrics.map((metric) => (
              <div key={metric.key} title={metric.definition}>
                <dt>{metric.key}</dt>
                <dd>
                  {metric.value} {metric.unit}
                </dd>
              </div>
            ))}
          </dl>
          <p className="muted small">Metric 是测量结果，不是分析结论（§20/§21）。</p>
        </div>
      )}

      <div className="card" data-testid="health2-lifecycle">
        <h2 className="card-title">Lifecycle（空基线对比）</h2>
        <dl className="fact-grid">
          {LIFECYCLE_ROWS.map((row) => (
            <div key={row.key}>
              <dt>{row.label}</dt>
              <dd>{report.lifecycle[row.key].length}</dd>
            </div>
          ))}
        </dl>
      </div>

      <div className="card" data-testid="health2-findings">
        <h2 className="card-title">Findings ({report.findings.length})</h2>
        {report.findings.length === 0 ? (
          <p className="muted">No findings from the health2 builtin rule set.</p>
        ) : (
          <ul>
            {report.findings.map((finding) => (
              <li key={finding.id} className="small">
                <button
                  type="button"
                  className="linklike"
                  onClick={() => setExpanded(expanded === finding.id ? null : finding.id)}
                >
                  <span className={`sev sev-${finding.severity}`}>
                    [{finding.severity.toUpperCase()}]
                  </span>{" "}
                  {finding.finding_type} — {finding.title}
                  {finding.target ? (
                    <span className="mono"> · {finding.target}</span>
                  ) : null}
                  {lifecycleOf(finding) ? ` [${lifecycleOf(finding)}]` : ""}
                </button>
                {expanded === finding.id && (
                  <div className="finding-detail">
                    {finding.description && <p>{finding.description}</p>}
                    <p>
                      <strong>Origin:</strong> {finding.origin} ·{" "}
                      <strong>Confidence:</strong> {finding.confidence}
                    </p>
                    {finding.remediation && (
                      <p>
                        <strong>Suggested:</strong> {finding.remediation}
                      </p>
                    )}
                    <ul>
                      {finding.evidence.map((evidenceId) => {
                        const evidence = evidenceById.get(evidenceId);
                        return (
                          <li key={evidenceId} className="mono small">
                            {evidence
                              ? `${evidence.path ?? evidence.reference ?? ""} — ${evidence.note}`
                              : evidenceId}
                          </li>
                        );
                      })}
                      {finding.evidence.length === 0 && (
                        <li className="muted small">no evidence entries</li>
                      )}
                    </ul>
                  </div>
                )}
              </li>
            ))}
          </ul>
        )}
      </div>

      {report.diagnostics.length > 0 && (
        <div className="card" data-testid="health2-diagnostics">
          <h2 className="card-title">Diagnostics ({report.diagnostics.length})</h2>
          <ul>
            {visibleDiagnostics.map((diagnostic, index) => (
              <li key={index} className="muted small mono">
                {diagnostic}
              </li>
            ))}
          </ul>
          {report.diagnostics.length > 5 && !showAllDiagnostics && (
            <button
              type="button"
              className="linklike small"
              onClick={() => setShowAllDiagnostics(true)}
            >
              View all {report.diagnostics.length} diagnostics
            </button>
          )}
        </div>
      )}
    </>
  );
}
