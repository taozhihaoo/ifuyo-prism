import { useState } from "react";
import { useProjectStore } from "../app/ProjectStore";
import DirectoryTree from "../components/DirectoryTree";
import FileList from "../components/FileList";
import PlaceholderPage from "../components/PlaceholderPage";
import SymbolExplorer, { SymbolDetail } from "../components/SymbolExplorer";
import type { SymbolDto } from "../types/prism";

/**
 * Understand：项目结构（M1 树 + 文件列表）与代码符号（M2 符号浏览器）。
 * 数据一律来自真实 discovery / code model，未分析时如实说明。
 */
export default function UnderstandPage() {
  const { project, phase, discovery, codeModel, inspectStatus, startInspect } = useProjectStore();
  const [selected, setSelected] = useState<SymbolDto | null>(null);

  if (!project || !discovery) {
    return (
      <PlaceholderPage
        milestone="M1"
        description="Open and scan a project on the Overview page to see its structure, files and symbols here."
      />
    );
  }

  if (phase === "scanning") {
    return (
      <div className="card">
        <p className="muted">Scanning project…</p>
      </div>
    );
  }

  if (phase === "analyzing") {
    return (
      <div className="card" data-testid="analyzing">
        <p className="muted">Analyzing code…</p>
        {inspectStatus && (
          <p className="muted small">
            files {inspectStatus.files_done}/{inspectStatus.files_total} · symbols{" "}
            {inspectStatus.symbols} · errors {inspectStatus.errors}
          </p>
        )}
      </div>
    );
  }

  if (!codeModel) {
    return (
      <div className="card" data-testid="inspect-missing">
        <p className="muted">
          Code analysis has not completed yet. It runs automatically after a scan.
        </p>
        <button type="button" className="button" onClick={() => void startInspect()}>
          Run code analysis
        </button>
      </div>
    );
  }

  const summary = codeModel.summary;

  return (
    <section className="stack">
      <div className="card" data-testid="code-summary">
        <h2 className="card-title">Code understanding</h2>
        <dl className="fact-grid">
          <div>
            <dt>Source files</dt>
            <dd>{summary.files_total}</dd>
          </div>
          <div>
            <dt>Parsed</dt>
            <dd>{summary.files_parsed}</dd>
          </div>
          <div>
            <dt>With diagnostics</dt>
            <dd>{summary.files_with_diagnostics}</dd>
          </div>
          <div>
            <dt>Unsupported</dt>
            <dd>{summary.files_unsupported}</dd>
          </div>
          <div>
            <dt>Failed</dt>
            <dd>{summary.files_failed}</dd>
          </div>
          <div>
            <dt>Symbols</dt>
            <dd>{summary.symbols_total}</dd>
          </div>
          <div>
            <dt>Duration</dt>
            <dd>{summary.duration_ms} ms</dd>
          </div>
        </dl>
        {(summary.error_count > 0 || summary.warning_count > 0) && (
          <div className="warning-box" data-testid="parse-diagnostics">
            <p>
              {summary.error_count} error(s), {summary.warning_count} warning(s) —
              per-file details below. Partial results are preserved.
            </p>
            <ul>
              {codeModel.parse_results
                .flatMap((result) =>
                  result.diagnostics.map((diagnostic, index) => ({
                    path: result.relative_path,
                    diagnostic,
                    index,
                  })),
                )
                .slice(0, 20)
                .map(({ path, diagnostic, index }) => (
                  <li key={index} className="mono small">
                    {path}: [{diagnostic.severity}] {diagnostic.message}
                  </li>
                ))}
            </ul>
          </div>
        )}
      </div>

      <div className="card">
        <h2 className="card-title">Symbols</h2>
        {selected && <SymbolDetail symbol={selected} />}
        <SymbolExplorer
          results={codeModel.parse_results}
          selectedId={selected?.id ?? null}
          onSelect={setSelected}
        />
      </div>

      <div className="card">
        <h2 className="card-title">Project tree</h2>
        <p className="muted small">
          {discovery.project.name} — {discovery.statistics.total_directories} directories,{" "}
          {discovery.statistics.total_files} files (ignored entries are not listed).
        </p>
        <DirectoryTree directories={discovery.directories} files={discovery.files} />
      </div>
      <FileList files={discovery.files} />
    </section>
  );
}
