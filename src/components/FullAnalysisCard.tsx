import { useEffect, useState } from "react";
import { useProjectStore } from "../app/ProjectStore";
import {
  cancelFullAnalysis,
  getFullAnalysisStatus,
  startFullAnalysis,
  toPrismError,
} from "../services/ipc";
import type { FullAnalysisStatusDto } from "../types/prism";

const STAGE_LABEL: Record<string, string> = {
  scan: "结构扫描（scan）",
  inspect: "代码理解（inspect）",
  graph: "依赖图（graph）",
  health: "健康分析（health）",
};

const STATUS_LABEL: Record<string, string> = {
  pending: "待执行",
  running: "运行中…",
  completed: "完成",
  failed: "失败",
  cancelled: "已取消",
  skipped: "跳过",
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
  const [status, setStatus] = useState<FullAnalysisStatusDto | null>(null);
  const [error, setError] = useState<string | null>(null);
  // 启动/取消后重启轮询循环（挂载时的循环在 idle 即退出）。
  const [pollTick, setPollTick] = useState(0);

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
      .then(() => setPollTick((tick) => tick + 1))
      .catch((err) => setError(toPrismError(err).message));
  };

  return (
    <div className="card" data-testid="full-analysis-card">
      <div className="context-row">
        <h2 className="card-title">全面分析</h2>
        {running ? (
          <button
            type="button"
            className="button"
            onClick={() => void cancelFullAnalysis()}
            data-testid="full-analysis-cancel"
          >
            取消
          </button>
        ) : (
          <button
            type="button"
            className="button button-primary"
            onClick={start}
            data-testid="full-analysis-start"
          >
            全面分析
          </button>
        )}
      </div>
      <p className="muted small">
        一键串行执行 结构扫描 → 代码理解 → 依赖图 → 健康分析（health 复用前三步产物，
        不重复扫描）。中途取消保留已完成部分。
      </p>
      {error && (
        <p className="status-error" data-testid="full-analysis-error">
          {error}
        </p>
      )}
      {status && status.stages.length > 0 && (
        <ul className="row-list" data-testid="full-analysis-stages">
          {status.stages.map((stage) => (
            <li
              key={stage.key}
              className="row-static"
              data-testid={`full-analysis-stage-${stage.key}`}
            >
              <span>{STAGE_LABEL[stage.key] ?? stage.key}</span>
              <span className="pill">
                {STATUS_LABEL[stage.status] ?? stage.status}
                {stage.duration_ms !== null && stage.status === "completed"
                  ? ` · ${stage.duration_ms}ms`
                  : ""}
              </span>
            </li>
          ))}
        </ul>
      )}
      {status?.status === "completed" && (
        <p className="muted small" data-testid="full-analysis-done">
          全部完成 · 总耗时 {totalMs}ms——各域结果见对应页面。
        </p>
      )}
      {status?.status === "cancelled" && (
        <p className="muted small" data-testid="full-analysis-cancelled">
          已取消——上方已完成阶段的结果保留，可重新运行。
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
