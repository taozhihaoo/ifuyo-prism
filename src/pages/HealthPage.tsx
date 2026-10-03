import { useEffect, useState } from "react";
import { useProjectStore } from "../app/ProjectStore";
import {
  cancelHealth,
  getHealthAnalysis,
  getHealthStatus,
  startHealth,
  toPrismError,
} from "../services/ipc";
import type { FindingDto, HealthAnalysisDto } from "../types/prism";

type LoadState =
  | { kind: "loading" }
  | { kind: "loaded"; analysis: HealthAnalysisDto }
  | { kind: "error"; message: string };

type SeverityFilter = "all" | "critical" | "warning" | "info";

const CATEGORIES: { key: string; label: string }[] = [
  { key: "architecture", label: "Architecture" },
  { key: "dependency", label: "Dependency" },
  { key: "code", label: "Code" },
  { key: "security", label: "Security" },
  { key: "maintenance", label: "Maintenance" },
  { key: "documentation", label: "Documentation" },
];

/**
 * Health 页（M7，规格 §81–§86/§141–§144）。
 * 证据化 findings + 过滤 + 明细展开；无健康分、无游戏化（§4/§145）。
 */
export default function HealthPage() {
  const { project } = useProjectStore();
  const [state, setState] = useState<LoadState>({ kind: "loading" });
  const [severityFilter, setSeverityFilter] = useState<SeverityFilter>("all");
  const [categoryFilter, setCategoryFilter] = useState<string>("all");
  const [changedOnly, setChangedOnly] = useState(false);
  const [expanded, setExpanded] = useState<string | null>(null);
  const [showAll, setShowAll] = useState(false);

  useEffect(() => {
    if (!project) {
      setState({ kind: "loading" });
      return;
    }
    setState({ kind: "loading" });
    // 页面卸载时取消后台分析（§102/§160）。
    return () => {
      void Promise.resolve(cancelHealth()).catch(() => {
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
        const status = await getHealthStatus();
        if (disposed) return;
        if (status.status === "running") {
          timer = setTimeout(poll, 300);
          return;
        }
        if (status.status === "idle") {
          await startHealth();
          timer = setTimeout(poll, 300);
          return;
        }
        if (status.status === "failed") {
          if (!disposed) {
            setState({ kind: "error", message: status.error ?? "health analysis failed" });
          }
          return;
        }
        const analysis = await getHealthAnalysis();
        if (!disposed) {
          if (analysis) {
            setState({ kind: "loaded", analysis });
          } else {
            setState({ kind: "error", message: "health analysis was cancelled" });
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
  }, [project?.root_path]);

  if (!project) {
    return (
      <div className="card" data-testid="health-needs-project">
        <p className="muted">Open a project to see its health findings here.</p>
      </div>
    );
  }

  if (state.kind === "loading") {
    return (
      <div className="card" data-testid="health-loading">
        <p className="muted">Running health rules…</p>
      </div>
    );
  }

  if (state.kind === "error") {
    return (
      <div className="card" data-testid="health-error">
        <p className="status-error">Health analysis unavailable: {state.message}</p>
      </div>
    );
  }

  const { analysis } = state;
  const summary = analysis.summary;
  const filtered = analysis.findings.filter((finding) => {
    if (severityFilter !== "all" && finding.severity !== severityFilter) return false;
    if (categoryFilter !== "all" && finding.category !== categoryFilter) return false;
    if (changedOnly && finding.change_relation === "unrelated") return false;
    return true;
  });
  const visible = showAll ? filtered : filtered.slice(0, 50);

  return (
    <section className="stack">
      <div className="card" data-testid="health-summary">
        <h2 className="card-title">Health Findings · {analysis.analysis_completeness}</h2>
        <dl className="fact-grid">
          <div>
            <dt>Total</dt>
            <dd>{summary.total_findings}</dd>
          </div>
          <div>
            <dt>Critical</dt>
            <dd>{summary.critical_count}</dd>
          </div>
          <div>
            <dt>Warning</dt>
            <dd>{summary.warning_count}</dd>
          </div>
          <div>
            <dt>Info</dt>
            <dd>{summary.info_count}</dd>
          </div>
          <div>
            <dt>Change-related</dt>
            <dd>{analysis.change_related_findings}</dd>
          </div>
        </dl>
        <p className="muted small">
          Findings first — no health score (spec §4). Facts only, every finding traceable to
          evidence.
        </p>
      </div>

      <div className="card" data-testid="health-groups">
        <h2 className="card-title">Categories</h2>
        <dl className="fact-grid">
          {CATEGORIES.map((category) => (
            <div key={category.key}>
              <dt>{category.label}</dt>
              <dd>{summary.by_category[category.key] ?? 0}</dd>
            </div>
          ))}
        </dl>
      </div>

      <div className="card" data-testid="health-findings">
        <h2 className="card-title">Findings ({filtered.length})</h2>
        <div className="chip-row">
          {(["all", "critical", "warning", "info"] as SeverityFilter[]).map((sev) => (
            <button
              key={sev}
              type="button"
              className={`chip${severityFilter === sev ? " selected" : ""}`}
              onClick={() => setSeverityFilter(sev)}
            >
              {sev}
            </button>
          ))}
          <span className="muted">|</span>
          <button
            type="button"
            className={`chip${categoryFilter === "all" ? " selected" : ""}`}
            onClick={() => setCategoryFilter("all")}
          >
            all categories
          </button>
          {CATEGORIES.map((category) => (
            <button
              key={category.key}
              type="button"
              className={`chip${categoryFilter === category.key ? " selected" : ""}`}
              onClick={() => setCategoryFilter(category.key)}
            >
              {category.label}
            </button>
          ))}
          <span className="muted">|</span>
          <button
            type="button"
            className={`chip${changedOnly ? " selected" : ""}`}
            onClick={() => setChangedOnly(!changedOnly)}
          >
            current changes ({analysis.change_related_findings})
          </button>
        </div>
        {filtered.length === 0 ? (
          <p className="muted">No findings match the current filters.</p>
        ) : (
          <ul>
            {visible.map((finding: FindingDto) => (
              <li key={finding.id} className="small">
                <button
                  type="button"
                  className="linklike"
                  onClick={() => setExpanded(expanded === finding.id ? null : finding.id)}
                >
                  <span className={`sev sev-${finding.severity}`}>[{finding.severity.toUpperCase()}]</span>{" "}
                  {finding.rule_id} — {finding.title} ·{" "}
                  <span className="mono">
                    {finding.location.relative_path}:{finding.location.line}
                  </span>
                  {finding.status !== "active" ? ` [${finding.status}]` : ""}
                </button>
                {expanded === finding.id && (
                  <div className="finding-detail">
                    <p>{finding.message}</p>
                    <p>
                      <strong>What:</strong> {finding.explanation?.what}
                    </p>
                    {finding.explanation?.why_it_matters && (
                      <p>
                        <strong>Why it matters:</strong> {finding.explanation.why_it_matters}
                      </p>
                    )}
                    <p>
                      <strong>Confidence:</strong> {finding.confidence}
                      {finding.suggested_action && (
                        <>
                          {" · "}
                          <strong>Suggested:</strong> {finding.suggested_action}
                        </>
                      )}
                    </p>
                    <ul>
                      {finding.evidence.map((evidence, index) => (
                        <li key={index} className="mono small">
                          {evidence.detail}
                        </li>
                      ))}
                    </ul>
                  </div>
                )}
              </li>
            ))}
            {filtered.length > 50 && !showAll && (
              <li>
                <button
                  type="button"
                  className="linklike small"
                  onClick={() => setShowAll(true)}
                >
                  View all {filtered.length} findings
                </button>
              </li>
            )}
          </ul>
        )}
        {filtered.length > 50 && showAll && (
          <p className="muted small">Showing all {filtered.length} findings.</p>
        )}
      </div>

      <div className="card" data-testid="health-limitations">
        <h2 className="card-title">Analysis Limitations</h2>
        <p className="muted small">
          Health analysis is based on resolvable static relationships; some relationships could
          not be resolved, so results may be incomplete (spec §131).
        </p>
        {analysis.diagnostics.length > 0 && (
          <ul>
            {analysis.diagnostics.map((diagnostic, index) => (
              <li key={index} className="small">
                <span className="mono">{diagnostic.kind}</span>{" "}
                <span className="muted">{diagnostic.subject ?? ""}</span> — {diagnostic.detail}
              </li>
            ))}
          </ul>
        )}
        <p className="muted small">
          Rules checked: {analysis.checked_rules.length}
          {analysis.disabled_rules.length > 0 &&
            ` · Disabled: ${analysis.disabled_rules.join(", ")}`}
        </p>
      </div>
    </section>
  );
}
