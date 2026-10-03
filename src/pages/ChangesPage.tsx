import { useEffect, useState } from "react";
import { useProjectStore } from "../app/ProjectStore";
import { getGitFacts, toPrismError } from "../services/ipc";
import type { GitFactsDto } from "../types/prism";

type LoadState =
  | { kind: "loading" }
  | { kind: "loaded"; facts: GitFactsDto }
  | { kind: "notGit" }
  | { kind: "error"; message: string };

/**
 * Changes：Git facts 真实数据页（M4，规格 §32/§33/§57/§58）。
 * 只展示事实；状态不解释成风险；非 Git 项目显示友好状态而非错误堆栈。
 */
export default function ChangesPage() {
  const { project } = useProjectStore();
  const [state, setState] = useState<LoadState>({ kind: "loading" });

  useEffect(() => {
    if (!project) {
      setState({ kind: "loading" });
      return;
    }
    let cancelled = false;
    setState({ kind: "loading" });
    getGitFacts(false)
      .then((facts) => {
        if (!cancelled) {
          setState(facts.repository ? { kind: "loaded", facts } : { kind: "notGit" });
        }
      })
      .catch((err) => {
        if (!cancelled) setState({ kind: "error", message: toPrismError(err).message });
      });
    return () => {
      cancelled = true;
    };
  }, [project]);

  if (!project) {
    return (
      <div className="card" data-testid="git-needs-project">
        <p className="muted">Open a project to see its Git facts here.</p>
      </div>
    );
  }

  if (state.kind === "loading") {
    return (
      <div className="card" data-testid="git-loading">
        <p className="muted">Reading Git facts…</p>
      </div>
    );
  }

  if (state.kind === "error") {
    return (
      <div className="card" data-testid="git-error">
        <p className="status-error">Git facts unavailable: {state.message}</p>
      </div>
    );
  }

  if (state.kind === "notGit") {
    return (
      <div className="card" data-testid="not-git">
        <h2 className="card-title">Git repository not detected</h2>
        <p className="muted">
          This project is not a Git repository, so there is no Git state to
          show. Prism does not treat this as an error.
        </p>
      </div>
    );
  }

  const { facts } = state;
  const status = facts.status;
  const groups: { label: string; paths: string[] }[] = [
    {
      label: "Staged (index)",
      paths: status.entries
        .filter((e) => e.index_state !== "unmodified" && e.index_state !== "untracked")
        .map((e) =>
          e.original_path
            ? `${e.index_state}: ${e.original_path} -> ${e.path}`
            : `${e.index_state}: ${e.path}`,
        ),
    },
    {
      label: "Working tree",
      paths: status.entries
        .filter((e) => e.working_tree_state !== "unmodified" && !e.is_untracked)
        .map((e) => `${e.working_tree_state}: ${e.path}`),
    },
    {
      label: "Untracked",
      paths: status.entries.filter((e) => e.is_untracked).map((e) => e.path),
    },
  ];

  return (
    <section className="stack">
      <div className="card" data-testid="repo-header">
        <h2 className="card-title">Repository</h2>
        {facts.head && (
          <dl className="fact-grid">
            <div>
              <dt>Branch</dt>
              <dd>
                {facts.head.kind === "detached"
                  ? "(detached)"
                  : facts.head.kind === "unborn"
                    ? `${facts.head.branch_name ?? "?"} (unborn)`
                    : facts.head.branch_name ?? "—"}
              </dd>
            </div>
            <div>
              <dt>HEAD</dt>
              <dd className="mono">
                {facts.head.commit_id
                  ? facts.head.commit_id.slice(0, 12)
                  : "(no commits)"}
              </dd>
            </div>
            <div>
              <dt>Root</dt>
              <dd>{facts.repository?.root_path}</dd>
            </div>
          </dl>
        )}
      </div>

      <div className="card" data-testid="working-tree">
        <h2 className="card-title">Working tree</h2>
        {groups.every((group) => group.paths.length === 0) ? (
          <p className="muted">Working tree clean.</p>
        ) : (
          groups.map((group) =>
            group.paths.length === 0 ? null : (
              <div key={group.label}>
                <p className="muted small">{group.label}</p>
                <ul>
                  {group.paths.map((path, index) => (
                    <li key={index} className="mono small">
                      {path}
                    </li>
                  ))}
                </ul>
              </div>
            ),
          )
        )}
        {status.ignored.length > 0 && (
          <p className="muted small">
            {status.ignored.length} ignored entries (hidden by default).
          </p>
        )}
      </div>

      <div className="card" data-testid="history">
        <h2 className="card-title">Recent commits</h2>
        {facts.recent_commits.length === 0 ? (
          <p className="muted">No commits yet.</p>
        ) : (
          <ul>
            {facts.recent_commits.map((commit) => (
              <li key={commit.id} className="small">
                <span className="mono">{commit.id.slice(0, 7)}</span> {commit.subject}{" "}
                <span className="muted">({commit.author.name})</span>
              </li>
            ))}
          </ul>
        )}
        <p className="muted small">Facts only — no health interpretation (spec §58).</p>
      </div>
    </section>
  );
}
