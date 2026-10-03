import { useEffect, useState } from "react";
import { useProjectStore } from "../app/ProjectStore";
import {
  cancelEnvironmentScan,
  getEnvironmentResults,
  getEnvironmentStatus,
  startEnvironmentScan,
  toPrismError,
} from "../services/ipc";
import type { EnvironmentSnapshotDto } from "../types/prism";

type Phase =
  | { kind: "loading" }
  | { kind: "idle" }
  | { kind: "running" }
  | { kind: "done"; snapshot: EnvironmentSnapshotDto }
  | { kind: "noProject" }
  | { kind: "error"; message: string };

const STATUS_LABEL: Record<string, string> = {
  satisfied: "SATISFIED",
  unsatisfied: "UNSATISFIED",
  attention: "ATTENTION",
  unknown: "UNKNOWN",
  conflict: "CONFLICT",
  partial: "PARTIAL",
};

const RESULT_LABEL: Record<string, string> = {
  satisfied: "Satisfied",
  unsatisfied: "Unsatisfied",
  unknown: "Unknown",
  not_applicable: "N/A",
};

/**
 * Environment 页（M9，规格 §220–§233/§271–§281/§298–§304）：
 * 全部事实来自 Rust EnvironmentService（§299/§300 无前端检测）；
 * 动态事实带 observed_at（§277）；无分数、无 "Ready"（§136/§137）。
 */
export default function EnvironmentPage() {
  const { project } = useProjectStore();
  const [phase, setPhase] = useState<Phase>({ kind: "loading" });
  const [pollTick, setPollTick] = useState(0);

  useEffect(() => {
    if (!project) {
      setPhase({ kind: "noProject" });
      return;
    }
    return () => {
      void Promise.resolve(cancelEnvironmentScan()).catch(() => {
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
        const status = await getEnvironmentStatus();
        if (disposed) return;
        if (status.status === "running") {
          setPhase({ kind: "running" });
          timer = setTimeout(poll, 500);
          return;
        }
        if (status.status === "failed") {
          setPhase({ kind: "error", message: status.error ?? "environment scan failed" });
          return;
        }
        if (status.status === "idle") {
          setPhase({ kind: "idle" });
          return;
        }
        const snapshot = await getEnvironmentResults();
        if (!disposed) {
          if (snapshot) {
            setPhase({ kind: "done", snapshot });
          } else {
            setPhase({ kind: "error", message: "scan completed without results" });
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

  const refresh = (allTools = false) => {
    void startEnvironmentScan(allTools)
      .then(() => {
        setPhase({ kind: "running" });
        setPollTick((tick) => tick + 1);
      })
      .catch((err) => setPhase({ kind: "error", message: toPrismError(err).message }));
  };

  if (!project) {
    return (
      <div className="card" data-testid="env-needs-project">
        <p className="muted">Open a project to inspect its environment.</p>
      </div>
    );
  }
  if (phase.kind === "loading" || phase.kind === "running") {
    return (
      <div className="card" data-testid="env-loading">
        <p className="muted">
          {phase.kind === "running"
            ? "Scanning environment (host, toolchains, requirements, dynamic facts)…"
            : "Loading environment…"}
        </p>
      </div>
    );
  }
  if (phase.kind === "error") {
    return (
      <div className="card" data-testid="env-error">
        <p className="status-error">Environment scan failed: {phase.message}</p>
        <button type="button" className="linklike" onClick={() => refresh()}>
          Retry
        </button>
      </div>
    );
  }
  if (phase.kind === "idle") {
    return (
      <div className="card" data-testid="env-idle">
        <h2 className="card-title">Environment</h2>
        <p className="muted">
          No environment scan in this session. Refresh runs host, toolchain and
          requirement discovery — nothing is executed or modified.
        </p>
        <button type="button" className="linklike" onClick={() => refresh()}>
          Refresh
        </button>
      </div>
    );
  }

  if (phase.kind !== "done") {
    return (
      <div className="card" data-testid="env-loading">
        <p className="muted">Awaiting environment…</p>
      </div>
    );
  }

  const snapshot = phase.snapshot;
  const matchBySubject = new Map(
    snapshot.matches.map((matched) => [matched.requirement_id, matched]),
  );

  return (
    <section className="stack">
      <div className="card" data-testid="env-summary">
        <h2 className="card-title">Environment · {STATUS_LABEL[snapshot.status] ?? snapshot.status}</h2>
        <dl className="fact-grid">
          <div>
            <dt>OS</dt>
            <dd>
              {snapshot.host.os_name}{" "}
              {snapshot.host.os_version ?? "version unknown"} ({snapshot.host.architecture})
            </dd>
          </div>
          <div>
            <dt>Shell</dt>
            <dd>{snapshot.shell.kind}</dd>
          </div>
          <div>
            <dt>Observed</dt>
            <dd>{new Date(snapshot.captured_at).toLocaleTimeString()}</dd>
          </div>
          <div>
            <dt>Scan</dt>
            <dd>
              {snapshot.scan_outcome} · {snapshot.completeness}
            </dd>
          </div>
        </dl>
        <p className="muted small">
          Facts about this machine only — not a health score, not a guarantee of
          reproducibility.
        </p>
        <button type="button" className="linklike" onClick={() => refresh()}>
          Refresh
        </button>
        <button type="button" className="linklike" onClick={() => refresh(true)}>
          Refresh all tools
        </button>
      </div>

      <div className="card" data-testid="env-tools">
        <h2 className="card-title">Toolchain</h2>
        <ul>
          {snapshot.tools.map((tool) => (
            <li key={tool.name} className="small">
              <span className="mono">{tool.name}</span>{" "}
              {tool.raw_version ?? "not found"}{" "}
              <span className={`sev ${tool.probe === "succeeded" ? "sev-info" : "sev-warning"}`}>
                [{tool.probe.replace("_", " ")}]
              </span>{" "}
              <span className="muted">{tool.path ?? ""}</span>
              {tool.additional_paths.map((path) => (
                <div key={path} className="muted small">
                  also found: {path}
                </div>
              ))}
            </li>
          ))}
          {snapshot.tools.length === 0 && <li className="muted">No tools probed.</li>}
        </ul>
        {snapshot.virtualenvs.map((venv) => (
          <p key={venv.path} className="muted small" data-testid="env-venv">
            {venv.kind}: {venv.path} {venv.raw_version ?? "version unknown"}
            {venv.active ? " [active]" : ""}
          </p>
        ))}
      </div>

      <div className="card" data-testid="env-requirements">
        <h2 className="card-title">Requirements</h2>
        <ul>
          {snapshot.requirements.map((requirement) => {
            const matched = matchBySubject.get(requirement.id);
            return (
              <li key={requirement.id} className="small">
                <span className="mono">{requirement.subject}</span> {requirement.operator}{" "}
                {requirement.expected || "—"}{" "}
                <span
                  className={`sev ${
                    matched?.result === "satisfied" ? "sev-info" : "sev-warning"
                  }`}
                >
                  [{RESULT_LABEL[matched?.result ?? "unknown"]}]
                </span>{" "}
                <span className="muted">
                  {requirement.source} ({requirement.source_kind})
                </span>
                {matched && <div className="muted small">{matched.evidence}</div>}
              </li>
            );
          })}
          {snapshot.requirements.length === 0 && (
            <li className="muted">No requirements discovered.</li>
          )}
        </ul>
        {snapshot.conflicts.map((conflict) => (
          <div key={conflict.subject} className="warning-box" data-testid="env-conflict">
            <p>
              Requirement conflict: {conflict.subject} — cannot be interpreted as a
              single unambiguous requirement.
            </p>
            {conflict.entries.map((entry) => (
              <p key={entry.source} className="muted small">
                {entry.source} ({entry.source_kind}): {entry.operator} {entry.expected}
              </p>
            ))}
          </div>
        ))}
      </div>

      <div className="card" data-testid="env-findings">
        <h2 className="card-title">Findings</h2>
        <ul>
          {snapshot.findings.map((finding) => (
            <li key={finding.id} className="small">
              <span className={`sev ${finding.severity === "info" ? "sev-info" : "sev-warning"}`}>
                [{finding.severity}]
              </span>{" "}
              {finding.message}
              <ul>
                {finding.evidence.map((evidence, index) => (
                  <li key={index} className="muted small">
                    {evidence}
                  </li>
                ))}
              </ul>
              {finding.suggestion && (
                <div className="muted small">suggestion (not executed): {finding.suggestion}</div>
              )}
            </li>
          ))}
          {snapshot.findings.length === 0 && <li className="muted">No environment findings.</li>}
        </ul>
      </div>

      <div className="card" data-testid="env-variables">
        <h2 className="card-title">Variables</h2>
        <p className="muted small">
          {snapshot.variables.length} captured ·{" "}
          {snapshot.variables.filter((fact) => fact.possibly_sensitive).length} possibly
          sensitive — values are never read.
        </p>
      </div>

      <div className="card" data-testid="env-ports">
        <h2 className="card-title">Ports</h2>
        <p className="muted small">Dynamic state — observed at scan time, not live.</p>
        <ul>
          {snapshot.expected_ports.map((expected) => (
            <li key={`expected-${expected.port}`} className="small muted">
              expected {expected.port} ({expected.source})
            </li>
          ))}
          {snapshot.ports.map((port) => (
            <li key={`${port.address}-${port.port}`} className="small mono">
              {port.port} LISTENING {port.address} pid={port.pid ?? "?"}{" "}
              {port.process_name ?? "unknown owner"}
            </li>
          ))}
          {snapshot.ports.length === 0 && <li className="muted">No listeners observed.</li>}
        </ul>
      </div>

      <div className="card" data-testid="env-path">
        <h2 className="card-title">PATH</h2>
        <p className="muted small">
          {snapshot.paths.entries.length} entries · {snapshot.paths.duplicates.length}{" "}
          duplicates · {snapshot.paths.missing.length} missing
          {snapshot.paths.truncated ? " · truncated" : ""}
        </p>
      </div>
    </section>
  );
}
