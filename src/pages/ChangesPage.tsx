import { useEffect, useState } from "react";
import { useProjectStore } from "../app/ProjectStore";
import {
  cancelChangeset,
  getChangeset,
  getChangesetStatus,
  getGitFacts,
  startChangeset,
  toPrismError,
} from "../services/ipc";
import type {
  ChangeSetDto,
  FileChangeDto,
  GitFactsDto,
  SymbolChangeDto,
} from "../types/prism";

type LoadState =
  | { kind: "loading" }
  | { kind: "loaded"; facts: GitFactsDto | null; changeset: ChangeSetDto }
  | { kind: "notGit" }
  | { kind: "error"; message: string };

const FILE_GROUPS: { type: string; label: string }[] = [
  { type: "added", label: "Added" },
  { type: "modified", label: "Modified" },
  { type: "deleted", label: "Deleted" },
  { type: "renamed", label: "Renamed" },
  { type: "copied", label: "Copied" },
  { type: "type_changed", label: "Type changed" },
];

const SYMBOL_GROUPS: { type: string; label: string }[] = [
  { type: "added", label: "Added" },
  { type: "modified", label: "Modified" },
  { type: "deleted", label: "Deleted" },
  { type: "renamed", label: "Renamed" },
  { type: "moved", label: "Moved" },
];

function filePath(file: FileChangeDto): string {
  return file.new_path ?? file.old_path ?? "";
}

/**
 * Changes：ChangeSet 页（M5，规格 §67–§70）。
 * Git facts → FileChange → SymbolChange → ModuleChange → Diagnostics。
 * 只展示事实与证据，不显示风险/影响（M6 边界，§69/§70）。
 */
export default function ChangesPage() {
  const { project } = useProjectStore();
  const [state, setState] = useState<LoadState>({ kind: "loading" });
  const [selectedFile, setSelectedFile] = useState<string | null>(null);

  const projectPath = project?.root_path;

  useEffect(() => {
    if (!projectPath) {
      setState({ kind: "loading" });
      return;
    }
    setState({ kind: "loading" });
    setSelectedFile(null);

    // 页面关闭/切换时取消后台分析（§79）。
    return () => {
      Promise.resolve(cancelChangeset()).catch(() => {
        /* idle 状态下取消是无操作 */
      });
    };
  }, [projectPath]);

  useEffect(() => {
    if (!projectPath) return;
    let disposed = false;
    let timer: ReturnType<typeof setTimeout> | null = null;

    const poll = async () => {
      try {
        const status = await getChangesetStatus();
        if (disposed) return;
        if (status.status === "running" || status.status === "idle") {
          if (status.status === "idle") {
            await startChangeset("working_tree");
          }
          timer = setTimeout(poll, 300);
          return;
        }
        if (status.status === "failed") {
          if (!disposed) {
            setState({
              kind: "error",
              message: status.error ?? "changeset analysis failed",
            });
          }
          return;
        }
        // completed / cancelled：取模型（cancelled 保留已完成部分或为空）。
        const changeset = await getChangeset();
        if (disposed) return;
        if (!changeset) {
          const facts = await getGitFacts(false).catch(() => null);
          if (disposed) return;
          setState(
            facts?.repository
              ? { kind: "error", message: "changeset analysis was cancelled" }
              : { kind: "notGit" },
          );
          return;
        }
        const facts = await getGitFacts(false).catch(() => null);
        if (!disposed) setState({ kind: "loaded", facts, changeset });
      } catch (err) {
        if (!disposed) setState({ kind: "error", message: toPrismError(err).message });
      }
    };
    void poll();

    return () => {
      disposed = true;
      if (timer) clearTimeout(timer);
    };
  }, [projectPath]);

  if (!project) {
    return (
      <div className="card" data-testid="changes-needs-project">
        <p className="muted">Open a project to see its changes here.</p>
      </div>
    );
  }

  if (state.kind === "loading") {
    return (
      <div className="card" data-testid="changes-loading">
        <p className="muted">Analyzing changes…</p>
      </div>
    );
  }

  if (state.kind === "error") {
    return (
      <div className="card" data-testid="changes-error">
        <p className="status-error">Change analysis unavailable: {state.message}</p>
      </div>
    );
  }

  if (state.kind === "notGit") {
    return (
      <div className="card" data-testid="not-git">
        <h2 className="card-title">Git repository not detected</h2>
        <p className="muted">
          This project is not a Git repository, so there are no changes to
          analyze. Prism does not treat this as an error.
        </p>
      </div>
    );
  }

  const { changeset, facts } = state;
  const summary = changeset.summary;
  const head = facts?.head ?? null;
  const visibleSymbols: SymbolChangeDto[] = selectedFile
    ? changeset.symbols.filter((symbol) => symbol.path === selectedFile)
    : changeset.symbols;

  return (
    <section className="stack">
      <div className="card" data-testid="changeset-header">
        <h2 className="card-title">Changes · {changeset.scope.replace("_", " ")}</h2>
        <dl className="fact-grid">
          {head && (
            <div>
              <dt>Branch</dt>
              <dd>
                {head.kind === "detached"
                  ? "(detached)"
                  : head.kind === "unborn"
                    ? `${head.branch_name ?? "?"} (unborn)`
                    : head.branch_name ?? "—"}
              </dd>
            </div>
          )}
          <div>
            <dt>Base</dt>
            <dd className="mono">{changeset.base?.slice(0, 12) ?? "—"}</dd>
          </div>
          <div>
            <dt>Target</dt>
            <dd className="mono">{changeset.target ?? "—"}</dd>
          </div>
          <div>
            <dt>Repository</dt>
            <dd>{changeset.repository ?? "—"}</dd>
          </div>
        </dl>
      </div>

      <div className="card" data-testid="change-summary">
        <h2 className="card-title">Change summary</h2>
        <dl className="fact-grid">
          <div>
            <dt>Files</dt>
            <dd>
              {summary.files_total} ({summary.files_added} added, {summary.files_modified}{" "}
              modified, {summary.files_deleted} deleted, {summary.files_renamed} renamed)
            </dd>
          </div>
          <div>
            <dt>Symbols</dt>
            <dd>
              {summary.symbols_total} ({summary.symbols_added} added, {summary.symbols_modified}{" "}
              modified, {summary.symbols_deleted} deleted)
            </dd>
          </div>
          <div>
            <dt>Modules</dt>
            <dd>{summary.modules_changed} changed</dd>
          </div>
          <div>
            <dt>Diagnostics</dt>
            <dd>{summary.diagnostics_total}</dd>
          </div>
          {summary.test_files_changed > 0 && (
            <div>
              <dt>Test files</dt>
              <dd>{summary.test_files_changed}</dd>
            </div>
          )}
        </dl>
        {changeset.truncated && (
          <p className="muted small">Result truncated at the file limit.</p>
        )}
        <p className="muted small">
          Facts only — &quot;what may be affected&quot; belongs to M6 (spec §56).
        </p>
      </div>

      <div className="card" data-testid="changed-files">
        <h2 className="card-title">Files</h2>
        {changeset.files.length === 0 ? (
          <p className="muted">No file changes — working tree is clean.</p>
        ) : (
          FILE_GROUPS.map((group) => {
            const files = changeset.files.filter((file) => file.change_type === group.type);
            if (files.length === 0) return null;
            return (
              <div key={group.type}>
                <p className="muted small">
                  {group.label} ({files.length})
                </p>
                <ul>
                  {files.map((file) => {
                    const path = filePath(file);
                    const selected = selectedFile === path;
                    return (
                      <li key={file.id}>
                        <button
                          type="button"
                          className={`mono small linklike${selected ? " selected" : ""}`}
                          onClick={() => setSelectedFile(selected ? null : path)}
                        >
                          {path}
                          {file.old_path && file.new_path && file.old_path !== file.new_path
                            ? ` (from ${file.old_path})`
                            : ""}
                          {file.mapping_status !== "mapped" ? ` · ${file.mapping_status}` : ""}
                        </button>
                      </li>
                    );
                  })}
                </ul>
              </div>
            );
          })
        )}
        {selectedFile && (
          <p className="muted small">
            Showing symbols for <span className="mono">{selectedFile}</span> — click again to
            clear.
          </p>
        )}
      </div>

      <div className="card" data-testid="changed-symbols">
        <h2 className="card-title">Symbols</h2>
        {visibleSymbols.length === 0 ? (
          <p className="muted">
            No symbol-level changes resolved
            {selectedFile ? " for this file" : ""}.
          </p>
        ) : (
          SYMBOL_GROUPS.map((group) => {
            const symbols = visibleSymbols.filter((symbol) => symbol.change_type === group.type);
            if (symbols.length === 0) return null;
            return (
              <div key={group.type}>
                <p className="muted small">
                  {group.label} ({symbols.length})
                </p>
                <ul>
                  {symbols.map((symbol, index) => (
                    <li key={`${symbol.after_symbol_id ?? symbol.before_symbol_id ?? index}`} className="small">
                      <span className="mono">{symbol.name}</span>{" "}
                      <span className="muted">({symbol.kind})</span>
                      {symbol.characteristics.length > 0 && (
                        <span className="muted"> · {symbol.characteristics.join(", ")}</span>
                      )}
                      {(symbol.location_after ?? symbol.location_before) && (
                        <span className="muted small">
                          {" "}
                          L{(symbol.location_after ?? symbol.location_before)!.start_line}-
                          {(symbol.location_after ?? symbol.location_before)!.end_line}
                        </span>
                      )}
                    </li>
                  ))}
                </ul>
              </div>
            );
          })
        )}
      </div>

      <div className="card" data-testid="changed-modules">
        <h2 className="card-title">Modules</h2>
        {changeset.modules.length === 0 ? (
          <p className="muted">No module changes.</p>
        ) : (
          <ul>
            {changeset.modules.map((module) => (
              <li key={module.id} className="small">
                <span className="mono">{module.module_id}</span> {module.change_type}{" "}
                <span className="muted">
                  ({module.changed_files.length} file(s), {module.changed_symbol_count} symbol(s))
                </span>
              </li>
            ))}
          </ul>
        )}
      </div>

      <div className="card" data-testid="changes-diagnostics">
        <h2 className="card-title">Diagnostics</h2>
        {changeset.diagnostics.length === 0 ? (
          <p className="muted">None.</p>
        ) : (
          <ul>
            {changeset.diagnostics.map((diagnostic, index) => (
              <li key={index} className="small">
                <span className="mono">{diagnostic.kind}</span>{" "}
                <span className="muted">{diagnostic.path ?? ""}</span> — {diagnostic.detail}
              </li>
            ))}
          </ul>
        )}
        <p className="muted small">
          Mapping failures do not invalidate the ChangeSet (partial success, spec §34).
        </p>
      </div>
    </section>
  );
}
