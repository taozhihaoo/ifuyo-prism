import { useEffect, useState } from "react";
import { useProjectStore } from "../app/ProjectStore";
import { getIntelligenceSummary, toPrismError } from "../services/ipc";
import type { IntelligenceSummaryDto } from "../types/prism";

type PanelState =
  | { kind: "loading" }
  | { kind: "loaded"; summary: IntelligenceSummaryDto }
  | { kind: "error"; message: string };

function Unknown() {
  return <dd className="muted">Unknown</dd>;
}

/**
 * 概览 Intelligence 摘要卡（P1-2）。
 *
 * health2/health 计数来自本会话内存（没跑过 ⇒ Unknown，§14/§46 不假扮）；
 * godot 场景/deps 计数为按需 bounded 投影（缺 godot/manifest 文件 ⇒
 * Unknown）。Manual First：本卡只读既有事实与亚秒级投影，不触发重分析；
 * 各域完整分析入口在对应页面。
 */
export default function IntelligenceSummaryCard() {
  const { project } = useProjectStore();
  const [state, setState] = useState<PanelState>({ kind: "loading" });

  useEffect(() => {
    if (!project?.root_path) return;
    let disposed = false;
    setState({ kind: "loading" });
    const load = () => {
      getIntelligenceSummary()
        .then((summary) => {
          if (!disposed) setState({ kind: "loaded", summary });
        })
        .catch((err) => {
          if (!disposed) setState({ kind: "error", message: toPrismError(err).message });
        });
    };
    load();
    return () => {
      disposed = true;
    };
  }, [project?.root_path]);

  if (!project) return null;

  return (
    <div className="card" data-testid="intelligence-summary">
      <div className="context-row">
        <h2 className="card-title">Intelligence 摘要（2.0）</h2>
        <button
          type="button"
          className="linklike small"
          onClick={() => {
            setState({ kind: "loading" });
            getIntelligenceSummary()
              .then((summary) => setState({ kind: "loaded", summary }))
              .catch((err) => setState({ kind: "error", message: toPrismError(err).message }));
          }}
          data-testid="intelligence-refresh"
        >
          刷新
        </button>
      </div>
      {state.kind === "loading" && <p className="muted small">Projecting intelligence facts…</p>}
      {state.kind === "error" && (
        <p className="status-error">Intelligence summary unavailable: {state.message}</p>
      )}
      {state.kind === "loaded" && (
        <>
          <dl className="fact-grid">
            <div>
              <dt>health2 findings</dt>
              {state.summary.health2 ? (
                <dd data-testid="intelligence-health2">
                  {state.summary.health2.critical} critical · {state.summary.health2.warning}{" "}
                  warning · {state.summary.health2.info} info
                </dd>
              ) : (
                <Unknown />
              )}
            </div>
            <div>
              <dt>1.0 findings</dt>
              {state.summary.health !== null ? (
                <dd>{state.summary.health}</dd>
              ) : (
                <Unknown />
              )}
            </div>
            <div>
              <dt>Godot 场景</dt>
              {state.summary.godot ? (
                <dd data-testid="intelligence-godot">
                  {state.summary.godot.scenes} scenes · {state.summary.godot.nodes} nodes ·{" "}
                  {state.summary.godot.resources} resources
                </dd>
              ) : (
                <Unknown />
              )}
            </div>
            <div>
              <dt>依赖</dt>
              {state.summary.deps ? (
                <dd data-testid="intelligence-deps">
                  {state.summary.deps.declarations} declarations ·{" "}
                  {state.summary.deps.resolved} resolved · {state.summary.deps.cycles} cycles
                </dd>
              ) : (
                <Unknown />
              )}
            </div>
          </dl>
          <p className="muted small">
            Unknown = 本会话尚未运行该域（Manual First §147）——如实标注，不画成正常（§46）。
            完整分析入口：健康页（1.0/health2）、理解页 Godot 投影、环境页。
          </p>
        </>
      )}
    </div>
  );
}
