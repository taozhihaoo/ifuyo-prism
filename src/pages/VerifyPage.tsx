import { useEffect, useState } from "react";
import { useProjectStore } from "../app/ProjectStore";
import {
  cancelVerify,
  getVerifyPlan,
  getVerifyResults,
  getVerifyStatus,
  startVerify,
  toPrismError,
} from "../services/ipc";
import type {
  CheckRunDto,
  PlannedCheckDto,
  VerifyPlanDto,
  VerificationReportDto,
} from "../types/prism";

type Phase =
  | { kind: "loading" }
  | { kind: "plan"; plan: VerifyPlanDto }
  | { kind: "running" }
  | { kind: "done"; plan: VerifyPlanDto; report: VerificationReportDto | null }
  | { kind: "noProject" }
  | { kind: "error"; message: string };

function stateLabel(state: string): string {
  return state.replace("_", " ").toUpperCase();
}

/**
 * Verify 页（M8，规格 §112-§118/§268-§277/§297-§298）：
 * 先展示计划（§117），用户显式 Run（§118/§297）；check 卡片 + 输出查看器
 * （§113-§115）；结果只来自真实执行（§3/§6）；无健康分（§211）。
 */
export default function VerifyPage() {
  const { project } = useProjectStore();
  const [phase, setPhase] = useState<Phase>({ kind: "loading" });
  const [expanded, setExpanded] = useState<string | null>(null);
  const [selected, setSelected] = useState<string[]>([]);
  // 触发轮询重启：Run 后 plan 阶段的轮询已停止，需要重新拉起。
  const [pollTick, setPollTick] = useState(0);

  useEffect(() => {
    if (!project) {
      setPhase({ kind: "noProject" });
      return;
    }
    setPhase({ kind: "loading" });
    return () => {
      void Promise.resolve(cancelVerify()).catch(() => {
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
        const status = await getVerifyStatus();
        if (disposed) return;
        if (status.status === "running") {
          setPhase((prev) => (prev.kind === "plan" ? prev : { kind: "running" }));
          timer = setTimeout(poll, 400);
          return;
        }
        if (status.status === "idle") {
          // §298 First-run：发现入口 + 展示计划，不自动执行。
          const plan = await getVerifyPlan();
          if (!disposed) setPhase({ kind: "plan", plan });
          return;
        }
        if (status.status === "failed") {
          if (!disposed) {
            setPhase({ kind: "error", message: status.error ?? "verification failed" });
          }
          return;
        }
        const results = await getVerifyResults();
        if (!disposed) {
          if (results) {
            setPhase({ kind: "done", plan: results.plan, report: results.report });
          } else {
            setPhase({ kind: "error", message: "verification completed without results" });
          }
        }
      } catch (err) {
        if (!disposed) setPhase({ kind: "error", message: toPrismError(err).message });
      }
    };
    void poll();

    return () => {
      disposed = true;
      if (timer) clearTimeout(timer);
    };
  }, [project?.root_path, pollTick]);

  const runVerification = (ids: string[] | null) => {
    void startVerify(ids ?? undefined)
      .then(() => {
        setPhase({ kind: "running" });
        setPollTick((tick) => tick + 1);
      })
      .catch((err) => setPhase({ kind: "error", message: toPrismError(err).message }));
  };

  if (!project) {
    return (
      <div className="card" data-testid="verify-needs-project">
        <p className="muted">Open a project to verify it here.</p>
      </div>
    );
  }

  if (phase.kind === "loading") {
    return (
      <div className="card" data-testid="verify-loading">
        <p className="muted">Discovering checks…</p>
      </div>
    );
  }

  if (phase.kind === "error") {
    return (
      <div className="card" data-testid="verify-error">
        <p className="status-error">Verification unavailable: {phase.message}</p>
      </div>
    );
  }

  if (phase.kind === "plan") {
    // §298 First-run：展示检测到的入口与计划，不偷偷执行。
    return (
      <section className="stack">
        <div className="card" data-testid="verify-plan">
          <h2 className="card-title">Verify Plan</h2>
          <p className="muted small">
            Detected verification entry points. Nothing has been executed yet (§298).
          </p>
          <ul>
            {phase.plan.checks.map((check: PlannedCheckDto) => (
              <li key={check.id} className="small">
                <span className="mono">{check.id}</span> — {check.program}{" "}
                {check.args.join(" ")}{" "}
                <span className="muted">
                  [source: {check.source} · {check.detected_from}]
                </span>
              </li>
            ))}
          </ul>
          {phase.plan.checks.length === 0 && (
            <p className="muted">No verification entry points detected.</p>
          )}
          <button
            type="button"
            className="linklike"
            disabled={phase.plan.checks.length === 0}
            onClick={() => runVerification(null)}
          >
            Run verification
          </button>
        </div>
      </section>
    );
  }

  if (phase.kind !== "done") {
    return (
      <div className="card" data-testid="verify-loading">
        <p className="muted">Awaiting verification results…</p>
      </div>
    );
  }
  const report = phase.report;
  if (!report) {
    return (
      <div className="card" data-testid="verify-loading">
        <p className="muted">Awaiting verification results…</p>
      </div>
    );
  }
  const counts = report.counts;
  const visibleRuns: CheckRunDto[] = selected.length > 0
    ? report.runs.filter((run) => selected.includes(run.check_id))
    : report.runs;

  return (
    <section className="stack">
      <div className="card" data-testid="verify-summary">
        <h2 className="card-title">
          Verification · {report.conclusion.toUpperCase()}
        </h2>
        <dl className="fact-grid">
          <div><dt>Passed</dt><dd>{counts.passed}</dd></div>
          <div><dt>Failed</dt><dd>{counts.failed}</dd></div>
          <div><dt>Unavailable</dt><dd>{counts.unavailable}</dd></div>
          <div><dt>Blocked</dt><dd>{counts.blocked}</dd></div>
          <div><dt>Skipped</dt><dd>{counts.skipped}</dd></div>
          <div><dt>Cancelled</dt><dd>{counts.cancelled}</dd></div>
          <div><dt>Timed out</dt><dd>{counts.timed_out}</dd></div>
          <div><dt>Error</dt><dd>{counts.error}</dd></div>
        </dl>
        <p className="muted small">
          Real execution results only — no health score (§3/§211).
        </p>
        {report.verified_state?.kind === "dirty_workspace" && (
          <p className="muted small" data-testid="verify-dirty-note">
            {report.verified_state.changed_files} working-tree changes present at
            verification time.
          </p>
        )}
        {report.verified_state?.kind === "clean_at_commit" && (
          <p className="muted small" data-testid="verify-clean-note">
            Working tree clean
            {report.verified_state.commit
              ? ` at commit ${report.verified_state.commit.slice(0, 12)}`
              : " (no commits yet)"}
            .
          </p>
        )}
      </div>

      <div className="card" data-testid="verify-runs">
        <h2 className="card-title">Check runs</h2>
        <div className="chip-row">
          <button type="button" className="chip" onClick={() => setSelected([])}>all</button>
          <button type="button" className="chip" onClick={() => setSelected(["build"])}>build</button>
          <button type="button" className="chip" onClick={() => setSelected(["test"])}>test</button>
          <button type="button" className="chip" onClick={() => setSelected(["lint"])}>lint</button>
        </div>
        <ul>
          {visibleRuns.map((run) => (
            <li key={run.run_id} className="small">
              <button
                type="button"
                className="linklike"
                onClick={() => setExpanded(expanded === run.run_id ? null : run.run_id)}
              >
                <span className={`sev sev-${run.state === "passed" ? "info" : "warning"}`}>
                  [{stateLabel(run.state)}]
                </span>{" "}
                <span className="mono">{run.command_display}</span>{" "}
                <span className="muted">({run.duration_ms}ms)</span>
                {run.output_truncated && (
                  <span className="muted"> · output truncated</span>
                )}
              </button>
              {expanded === run.run_id && (
                <div className="finding-detail">
                  <p>
                    <strong>Working directory:</strong>{" "}
                    <span className="mono">{run.working_directory}</span>
                  </p>
                  {run.exit_code !== null && (
                    <p>
                      <strong>Exit code:</strong> {run.exit_code}
                    </p>
                  )}
                  {run.diagnostics.length > 0 && (
                    <ul>
                      {run.diagnostics.map((diagnostic, index) => (
                        <li key={index} className="small">
                          [{diagnostic.kind}] {diagnostic.detail}
                        </li>
                      ))}
                    </ul>
                  )}
                  {run.stdout && (
                    <>
                      <p className="muted small">stdout:</p>
                      <pre className="mono small">{run.stdout}</pre>
                    </>
                  )}
                  {run.stderr && (
                    <>
                      <p className="muted small">stderr:</p>
                      <pre className="mono small">{run.stderr}</pre>
                    </>
                  )}
                </div>
              )}
            </li>
          ))}
          {visibleRuns.length === 0 && <li className="muted">No runs.</li>}
        </ul>
      </div>

      <div className="card" data-testid="verify-limitations">
        <h2 className="card-title">Notes</h2>
        <p className="muted small">
          Results come from real command execution against the current working
          tree; they reflect the code as it was when run. Results apply only to
          this session.
        </p>
        <button type="button" className="linklike" onClick={() => runVerification(null)}>
          Run again
        </button>
      </div>
    </section>
  );
}
