import { useState } from "react";
import { useProjectStore } from "../app/ProjectStore";
import { getEvolutionReport, toPrismError } from "../services/ipc";
import type { EvolutionReportDto } from "../types/prism";

type PanelState =
  | { kind: "idle" }
  | { kind: "loading" }
  | { kind: "loaded"; report: EvolutionReportDto }
  | { kind: "error"; message: string };

/**
 * 演化面板（P1-1，M211）：变更热点 + 变更耦合表格。
 *
 * Manual First（§147）：git 历史可能较慢，点击按钮才运行。数据 =
 * bounded `scan_repo`（单次 `log --numstat`，最近 200 commit，§47/§52）；
 * 热点是原始指标、无 Score（§25）；耦合是历史共同变更、不是语义依赖
 * （§26/§27）。非 git 目录 ⇒ available:false + 原因（§46 不伪装）。
 */
export default function EvolutionPanel() {
  const { project } = useProjectStore();
  const [state, setState] = useState<PanelState>({ kind: "idle" });

  if (!project) return null;

  const run = () => {
    setState({ kind: "loading" });
    getEvolutionReport()
      .then((report) => setState({ kind: "loaded", report }))
      .catch((err) => setState({ kind: "error", message: toPrismError(err).message }));
  };

  return (
    <div className="card" data-testid="evolution-panel">
      <h2 className="card-title">演化分析（Evolution）</h2>
      {state.kind === "idle" && (
        <>
          <p className="muted small">
            变更热点与耦合来自 git 历史（最近 200 commit，bounded）。只读
            git，不修改工作区。文件级单文件演化用 CLI：<code>prism git . evolution &lt;file&gt;</code>
          </p>
          <button type="button" className="button" onClick={run} data-testid="evolution-run">
            运行演化分析
          </button>
        </>
      )}
      {state.kind === "loading" && (
        <p className="muted" data-testid="evolution-loading">
          Reading recent git history…
        </p>
      )}
      {state.kind === "error" && (
        <p className="status-error" data-testid="evolution-error">
          Evolution analysis failed: {state.message}
        </p>
      )}
      {state.kind === "loaded" && !state.report.available && (
        <div className="warning-box" data-testid="evolution-unavailable">
          <p>演化分析不可用：{state.report.unavailable_reason ?? "unknown reason"}</p>
          <button type="button" className="linklike small" onClick={run}>
            重试
          </button>
        </div>
      )}
      {state.kind === "loaded" && state.report.available && (
        <>
          <p className="muted small" data-testid="evolution-window">
            窗口：{state.report.window || "unknown"} · 扫描 {state.report.commits_scanned} commit ·
            热点显示前 {state.report.hotspots.length} · 耦合显示 {state.report.couplings.length} 对
          </p>

          <h3 className="card-subtitle">变更热点（原始指标，无分数 §25）</h3>
          {state.report.hotspots.length === 0 ? (
            <p className="muted small">窗口内没有文件变更。</p>
          ) : (
            <table className="data-table" data-testid="evolution-hotspots">
              <thead>
                <tr>
                  <th>文件</th>
                  <th>commits</th>
                  <th>churn</th>
                  <th>contributors</th>
                </tr>
              </thead>
              <tbody>
                {state.report.hotspots.map((hotspot) => (
                  <tr key={hotspot.path}>
                    <td className="mono">{hotspot.path}</td>
                    <td>{hotspot.commit_count}</td>
                    <td>
                      {hotspot.churn} (+{hotspot.lines_added}/-{hotspot.lines_deleted})
                    </td>
                    <td>{hotspot.author_count}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}

          <h3 className="card-subtitle">变更耦合（历史共同变更，≠ 语义依赖 §26/§27）</h3>
          {state.report.couplings.length === 0 ? (
            <p className="muted small">窗口内没有 ≥3 次共同变更的文件对。</p>
          ) : (
            <table className="data-table" data-testid="evolution-couplings">
              <thead>
                <tr>
                  <th>文件对</th>
                  <th>共同变更</th>
                  <th>证据（commit）</th>
                </tr>
              </thead>
              <tbody>
                {state.report.couplings.map((coupling) => (
                  <tr key={`${coupling.path_a}|${coupling.path_b}`}>
                    <td className="mono">
                      {coupling.path_a} ↔ {coupling.path_b}
                    </td>
                    <td>{coupling.co_change_count}</td>
                    <td className="muted small">
                      {coupling.evidence_commits.slice(0, 3).join(", ") || "—"}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
          <button type="button" className="linklike small" onClick={run}>
            重新运行
          </button>
        </>
      )}
    </div>
  );
}
