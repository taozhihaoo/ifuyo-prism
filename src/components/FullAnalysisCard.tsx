import { useEffect, useRef, useState } from "react";
import { useProjectStore } from "../app/ProjectStore";
import { useUiSettings } from "../app/SettingsProvider";
import { format } from "../app/i18n";
import {
  cancelFullAnalysis,
  getFullAnalysisStatus,
  startFullAnalysis,
  toPrismError,
} from "../services/ipc";
import type { FullAnalysisStatusDto } from "../types/prism";

const STAGE_KEY: Record<string, string> = {
  scan: "fullAnalysis.stage.scan",
  inspect: "fullAnalysis.stage.inspect",
  graph: "fullAnalysis.stage.graph",
  health: "fullAnalysis.stage.health",
};

const STATUS_KEY: Record<string, string> = {
  pending: "fullAnalysis.status.pending",
  running: "fullAnalysis.status.running",
  completed: "fullAnalysis.status.completed",
  failed: "fullAnalysis.status.failed",
  cancelled: "fullAnalysis.status.cancelled",
  skipped: "fullAnalysis.status.skipped",
};

/**
 * 全面分析编排卡（P2-1）：一键串行 scan → inspect → graph → health。
 *
 * 后端按依赖序在单一线程串行执行（health 复用前三阶段产物，不重复
 * 扫描）；每阶段完成后页面轮询自然看到结果。逐步进度 + 随时可取消，
 * 中途取消保留已完成阶段（§46：未跑的阶段显式 skipped，不画成正常）。
 */
export default function FullAnalysisCard() {
  const { project } = useProjectStore();
  const { t } = useUiSettings();
  const [status, setStatus] = useState<FullAnalysisStatusDto | null>(null);
  const [error, setError] = useState<string | null>(null);
  // 启动/取消后重启轮询循环（挂载时的循环在 idle 即退出）。
  const [pollTick, setPollTick] = useState(0);
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
    let timer: ReturnType<typeof setTimeout> | null = null;
    const poll = async () => {
      try {
        const next = await getFullAnalysisStatus();
        if (disposed) return;
        setStatus(next);
        setError(null);
        if (next.status === "running") {
          timer = setTimeout(poll, 400);
        }
      } catch (err) {
        if (!disposed) setError(toPrismError(err).message);
      }
    };
    void poll();
    return () => {
      disposed = true;
      if (timer) clearTimeout(timer);
    };
  }, [project?.root_path, pollTick]);

  if (!project) return null;

  const running = status?.status === "running";
  const totalMs = status?.stages.reduce(
    (sum, stage) => sum + (stage.duration_ms ?? 0),
    0,
  );

  const start = () => {
    setError(null);
    startFullAnalysis()
      .then(() => {
        if (aliveRef.current) setPollTick((tick) => tick + 1);
      })
      .catch((err) => {
        if (aliveRef.current) setError(toPrismError(err).message);
      });
  };

  return (
    <div className="card" data-testid="full-analysis-card">
      <div className="card-head">
        <h2 className="card-title">{t("fullAnalysis.title")}</h2>
        {running ? (
          <button
            type="button"
            className="button"
            onClick={() => void cancelFullAnalysis()}
            data-testid="full-analysis-cancel"
          >
            {t("fullAnalysis.cancel")}
          </button>
        ) : (
          <button
            type="button"
            className="button button-primary"
            onClick={start}
            data-testid="full-analysis-start"
          >
            {t("fullAnalysis.start")}
          </button>
        )}
      </div>
      <p className="muted small">{t("fullAnalysis.hint")}</p>
      {error && (
        <p className="status-error" data-testid="full-analysis-error">
          {error}
        </p>
      )}
      {status && status.stages.length > 0 && (
        <ul className="row-list" data-testid="full-analysis-stages">
          {status.stages.map((stage) => {
            const stageKey = STAGE_KEY[stage.key];
            const statusKey = STATUS_KEY[stage.status];
            return (
              <li
                key={stage.key}
                className="row-static"
                data-testid={`full-analysis-stage-${stage.key}`}
              >
                <span>{stageKey ? t(stageKey) : stage.key}</span>
                <span className="pill">
                  {statusKey ? t(statusKey) : stage.status}
                  {stage.duration_ms !== null && stage.status === "completed"
                    ? ` · ${stage.duration_ms}ms`
                    : ""}
                </span>
              </li>
            );
          })}
        </ul>
      )}
      {status?.status === "completed" && (
        <p className="muted small" data-testid="full-analysis-done">
          {format(t("fullAnalysis.done"), { ms: totalMs ?? 0 })}
        </p>
      )}
      {status?.status === "cancelled" && (
        <p className="muted small" data-testid="full-analysis-cancelled">
          {t("fullAnalysis.cancelledNote")}
        </p>
      )}
      {status?.status === "failed" && (
        <p className="status-error small" data-testid="full-analysis-failed">
          {status.error ?? "full analysis failed"}
        </p>
      )}
    </div>
  );
}
