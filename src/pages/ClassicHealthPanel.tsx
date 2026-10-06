import { useEffect, useState } from "react";
import { useProjectStore } from "../app/ProjectStore";
import CompletenessBanner from "../components/CompletenessBanner";
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
 * 健康页 1.0 轨道（M7，规格 §81–§86/§141–§144；P1-1 双轨化后由 HealthPage
 * 以轨道切换挂载）。证据化 findings + 过滤 + 明细展开；无健康分（§4/§145）。
 * 规则来源标注见 health-rule-source（P2-4a/§71）。
 */
export default function ClassicHealthPanel() {
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
        if (status.status === "idle" || status.status === "cancelled") {
          // cancelled = 上次离开页面时按 §102/§160 取消的后台分析——
          // 回到本页应自动重启，而不是永久卡在 cancelled 错误上。
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
  // P2-4a：规则来源标注。1.0 注册表全量 = 本次检查（含失败）+ 禁用，
  // 由报告派生、不硬编码；与 CLI `prism health2`（2.0 增强规则集）区分。
  const ruleSetTotal = analysis.checked_rules.length + analysis.disabled_rules.length;
  const failedRules = analysis.failed_rules.length;
  const filtered = analysis.findings.filter((finding) => {
    if (severityFilter !== "all" && finding.severity !== severityFilter) return false;
    if (categoryFilter !== "all" && finding.category !== categoryFilter) return false;
    if (changedOnly && finding.change_relation === "unrelated") return false;
    return true;
  });
  const visible = showAll ? filtered : filtered.slice(0, 50);

  return (
    <section className="stack">
      <CompletenessBanner
        testId="health-completeness"
        data={{
          domain: "健康分析",
          completeness: analysis.analysis_completeness,
          diagnostics: analysis.diagnostics,
        }}
      />
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
        <p
          className="muted small"
          data-testid="health-rule-source"
        >
          规则集：1.0 内置（共 {ruleSetTotal} 条 · 本次检查{" "}
          {analysis.checked_rules.length} 条
          {failedRules > 0 && ` · 失败 ${failedRules} 条`}
          {analysis.disabled_rules.length > 0 &&
            ` · 禁用 ${analysis.disabled_rules.length} 条`}
          ）。2.0 增强规则集（prism-health2）请用 CLI：<code>prism health2</code>
          运行。
        </p>
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
          not be resolved, so results may be incomplete (spec §131). Missing pieces are listed in
          the data completeness banner above.
        </p>
        <p className="muted small">
          Rules checked: {analysis.checked_rules.length}
          {analysis.disabled_rules.length > 0 &&
            ` · Disabled: ${analysis.disabled_rules.join(", ")}`}
        </p>
      </div>
    </section>
  );
}
