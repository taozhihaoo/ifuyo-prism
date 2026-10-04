import { useEffect, useState } from "react";
import { useProjectStore } from "../app/ProjectStore";
import {
  cancelContextGeneration,
  exportContext,
  getContextResults,
  getContextStatus,
  startContextGeneration,
  toPrismError,
} from "../services/ipc";
import type { ContextItemDto, ContextPackDto } from "../types/prism";

type Phase =
  | { kind: "loading" }
  | { kind: "idle" }
  | { kind: "running" }
  | { kind: "done"; pack: ContextPackDto }
  | { kind: "noProject" }
  | { kind: "error"; message: string };

/**
 * Context 页（M10b，规格 §292–§303/§354–§363）：
 * task 输入 → Generate → groups/items（why-included §298）/excluded（§299）/
 * health（§300）/budget（§301）/Copy/Export（§302，无自动 AI 上传 §308-§311）。
 */
export default function ContextPage() {
  const { project } = useProjectStore();
  const [phase, setPhase] = useState<Phase>({ kind: "loading" });
  const [task, setTask] = useState("");
  const [scope, setScope] = useState("auto");
  const [expanded, setExpanded] = useState<string | null>(null);
  const [copied, setCopied] = useState<string | null>(null);
  const [pollTick, setPollTick] = useState(0);

  useEffect(() => {
    if (!project) {
      setPhase({ kind: "noProject" });
      return;
    }
    return () => {
      void Promise.resolve(cancelContextGeneration()).catch(() => {
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
        const status = await getContextStatus();
        if (disposed) return;
        if (status.status === "running") {
          setPhase({ kind: "running" });
          timer = setTimeout(poll, 500);
          return;
        }
        if (status.status === "failed") {
          setPhase({ kind: "error", message: status.error ?? "context generation failed" });
          return;
        }
        if (status.status === "idle") {
          setPhase({ kind: "idle" });
          return;
        }
        const pack = await getContextResults();
        if (!disposed) {
          if (pack) {
            setPhase({ kind: "done", pack });
          } else {
            setPhase({ kind: "error", message: "generation completed without results" });
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

  const generate = () => {
    void startContextGeneration({
      task: task || undefined,
      scope,
    })
      .then(() => {
        setPhase({ kind: "running" });
        setPollTick((tick) => tick + 1);
      })
      .catch((err) => setPhase({ kind: "error", message: toPrismError(err).message }));
  };

  const copy = async (format: "markdown" | "text", label: string) => {
    try {
      const content = await exportContext(format);
      await navigator.clipboard.writeText(content);
      setCopied(label);
    } catch (err) {
      setPhase({ kind: "error", message: toPrismError(err).message });
    }
  };

  if (!project) {
    return (
      <div className="card" data-testid="context-needs-project">
        <p className="muted">Open a project to generate a context pack.</p>
      </div>
    );
  }
  if (phase.kind === "loading" || phase.kind === "running") {
    return (
      <div className="card" data-testid="context-loading">
        <p className="muted">
          {phase.kind === "running"
            ? "Generating context (task → collect → select → materialize)…"
            : "Loading context…"}
        </p>
      </div>
    );
  }
  if (phase.kind === "error") {
    return (
      <div className="card" data-testid="context-error">
        <p className="status-error">Context generation failed: {phase.message}</p>
      </div>
    );
  }
  if (phase.kind === "idle") {
    return (
      <div className="card" data-testid="context-idle">
        <h2 className="card-title">Context</h2>
        <p className="muted">
          No context pack in this session. Generation is deterministic and
          read-only — nothing is sent anywhere (§308/§311).
        </p>
        <textarea
          data-testid="context-task-input"
          placeholder="What are you working on?"
          value={task}
          onChange={(event) => setTask(event.target.value)}
          rows={2}
        />
        <select
          data-testid="context-scope"
          value={scope}
          onChange={(event) => setScope(event.target.value)}
        >
          <option value="auto">auto</option>
          <option value="current_change">current_change</option>
          <option value="target">target</option>
          <option value="full">full</option>
        </select>
        <button type="button" className="linklike" onClick={generate}>
          Generate
        </button>
      </div>
    );
  }

  if (phase.kind !== "done") {
    return (
      <div className="card" data-testid="context-loading">
        <p className="muted">Awaiting context…</p>
      </div>
    );
  }

  const pack = phase.pack;
  const manifest = pack.manifest;

  const grouped = new Map<string, ContextItemDto[]>();
  for (const item of pack.items) {
    const list = grouped.get(item.group) ?? [];
    list.push(item);
    grouped.set(item.group, list);
  }

  return (
    <section className="stack">
      <div className="card" data-testid="context-summary">
        <h2 className="card-title">Context · {manifest.status}</h2>
        <p className="muted small" data-testid="context-generated-summary">
          {manifest.budget.selected_items} items ·{" "}
          {(manifest.budget.used_tokens_estimate / 1024).toFixed(1)}k estimated
          tokens / {(manifest.budget.limit_tokens_estimate / 1024).toFixed(0)}k ·{" "}
          {pack.health.findings.length} health findings ·{" "}
          {manifest.limitations.length} limitations
        </p>
        <p className="muted small">
          Observed {new Date(pack.state.generated_at).toLocaleTimeString()} ·{" "}
          {pack.state.dirty === true
            ? "working tree dirty"
            : pack.state.dirty === false
              ? "working tree clean"
              : "git state unknown"}
        </p>
        <div className="chip-row">
          <button
            type="button"
            className="chip"
            onClick={() => void copy("markdown", "markdown")}
            data-testid="context-copy-markdown"
          >
            Copy Markdown{copied === "markdown" ? " ✓" : ""}
          </button>
          <button
            type="button"
            className="chip"
            onClick={() => void copy("text", "text")}
          >
            Copy Text{copied === "text" ? " ✓" : ""}
          </button>
        </div>
        <button type="button" className="linklike" onClick={generate}>
          Refresh
        </button>
      </div>

      <div className="card" data-testid="context-health">
        <h2 className="card-title">Context Health · {pack.health.status}</h2>
        <ul>
          {pack.health.findings.map((finding) => (
            <li key={finding.rule_id} className="small">
              <span
                className={`sev ${finding.severity === "Info" ? "sev-info" : "sev-warning"}`}
              >
                [{finding.rule_id}]
              </span>{" "}
              {finding.message}
            </li>
          ))}
          {pack.health.findings.length === 0 && (
            <li className="muted">No context health findings.</li>
          )}
        </ul>
        <p className="muted small">
          Context Health ≠ project health (§274): it only describes this
          generated pack.
        </p>
      </div>

      <div className="card" data-testid="context-budget">
        <h2 className="card-title">Budget</h2>
        <p className="muted small">
          {manifest.budget.used_tokens_estimate} /{" "}
          {manifest.budget.limit_tokens_estimate} estimated tokens ·{" "}
          {manifest.budget.selected_items} selected ·{" "}
          {manifest.budget.excluded_items} excluded
        </p>
      </div>

      {manifest.limitations.length > 0 && (
        <div className="warning-box" data-testid="context-limitations">
          {manifest.limitations.map((limitation) => (
            <p key={limitation} className="small">
              {limitation}
            </p>
          ))}
        </div>
      )}

      <div className="card" data-testid="context-excluded">
        <h2 className="card-title">Excluded ({pack.selection.excluded.length})</h2>
        <ul>
          {pack.selection.excluded.slice(0, 10).map((excluded) => (
            <li key={excluded.id} className="small muted">
              {excluded.title} — {excluded.reason}
            </li>
          ))}
          {pack.selection.excluded.length === 0 && (
            <li className="muted">Nothing excluded.</li>
          )}
        </ul>
      </div>

      {[...grouped.entries()].map(([group, items]) => (
        <div className="card" key={group} data-testid={`context-group-${group}`}>
          <h2 className="card-title">{group}</h2>
          <ul>
            {items.map((item) => (
              <li key={item.id} className="small">
                <button
                  type="button"
                  className="linklike"
                  onClick={() =>
                    setExpanded(expanded === item.id ? null : item.id)
                  }
                >
                  <span
                    className={`sev ${item.priority === "critical" ? "sev-warning" : "sev-info"}`}
                  >
                    [{item.priority}]
                  </span>{" "}
                  {item.title}
                  {item.truncated && (
                    <span className="muted"> · truncated</span>
                  )}
                </button>
                <div className="muted small">
                  why: {item.reasons.join(", ")} · trust: {item.trust_level}
                </div>
                {expanded === item.id && (
                  <pre className="mono small">{item.content ?? "(metadata only)"}</pre>
                )}
              </li>
            ))}
          </ul>
        </div>
      ))}
    </section>
  );
}
