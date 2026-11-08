import { useEffect, useRef, useState } from "react";
import { useProjectStore } from "../app/ProjectStore";
import { useUiSettings } from "../app/SettingsProvider";
import { getIntelligenceSummary, toPrismError } from "../services/ipc";
import type { IntelligenceSummaryDto } from "../types/prism";

type PanelState =
  | { kind: "loading" }
  | { kind: "loaded"; summary: IntelligenceSummaryDto }
  | { kind: "error"; message: string };

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
  const { t } = useUiSettings();
  const [state, setState] = useState<PanelState>({ kind: "loading" });
  const aliveRef = useRef(true);

  useEffect(() => {
    aliveRef.current = true;
    return () => {
      aliveRef.current = false;
    };
  }, []);

  useEffect(() => {
    if (!project?.root_path) return;
    let disposed = false;
    setState({ kind: "loading" });
    getIntelligenceSummary()
      .then((summary) => {
        if (!disposed) setState({ kind: "loaded", summary });
      })
      .catch((err) => {
        if (!disposed) setState({ kind: "error", message: toPrismError(err).message });
      });
    return () => {
      disposed = true;
    };
  }, [project?.root_path]);

  if (!project) return null;

  const refresh = () => {
    setState({ kind: "loading" });
    getIntelligenceSummary()
      .then((summary) => {
        if (aliveRef.current) setState({ kind: "loaded", summary });
      })
      .catch((err) => {
        if (aliveRef.current) setState({ kind: "error", message: toPrismError(err).message });
      });
  };

  const Unknown = () => <dd className="muted">{t("common.unknown")}</dd>;

  return (
    <div className="card" data-testid="intelligence-summary">
      <div className="card-head">
        <h2 className="card-title">{t("intelligence.title")}</h2>
        <button
          type="button"
          className="linklike small"
          onClick={refresh}
          data-testid="intelligence-refresh"
        >
          {t("intelligence.refresh")}
        </button>
      </div>
      {state.kind === "loading" && <p className="muted small">{t("intelligence.loading")}</p>}
      {state.kind === "error" && (
        <p className="status-error">
          {t("intelligence.error")} {state.message}
        </p>
      )}
      {state.kind === "loaded" && (
        <>
          <dl className="fact-grid">
            <div>
              <dt>{t("intelligence.health2")}</dt>
              {state.summary.health2 ? (
                <dd data-testid="intelligence-health2">
                  {state.summary.health2.critical} {t("intelligence.units.critical")} ·{" "}
                  {state.summary.health2.warning} {t("intelligence.units.warning")} ·{" "}
                  {state.summary.health2.info} {t("intelligence.units.info")}
                </dd>
              ) : (
                <Unknown />
              )}
            </div>
            <div>
              <dt>{t("intelligence.health1")}</dt>
              {state.summary.health !== null ? (
                <dd>{state.summary.health}</dd>
              ) : (
                <Unknown />
              )}
            </div>
            <div>
              <dt>{t("intelligence.godot")}</dt>
              {state.summary.godot ? (
                <dd data-testid="intelligence-godot">
                  {state.summary.godot.scenes} {t("intelligence.units.scenes")} ·{" "}
                  {state.summary.godot.nodes} {t("intelligence.units.nodes")} ·{" "}
                  {state.summary.godot.resources} {t("intelligence.units.resources")}
                </dd>
              ) : (
                <Unknown />
              )}
            </div>
            <div>
              <dt>{t("intelligence.deps")}</dt>
              {state.summary.deps ? (
                <dd data-testid="intelligence-deps">
                  {state.summary.deps.declarations} {t("intelligence.units.declarations")} ·{" "}
                  {state.summary.deps.resolved} {t("intelligence.units.resolved")} ·{" "}
                  {state.summary.deps.cycles} {t("intelligence.units.cycles")}
                </dd>
              ) : (
                <Unknown />
              )}
            </div>
          </dl>
          <p className="muted small">{t("intelligence.note")}</p>
        </>
      )}
    </div>
  );
}
