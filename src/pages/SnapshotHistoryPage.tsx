import { useEffect, useState } from "react";
import { useProjectStore } from "../app/ProjectStore";
import { useUiSettings } from "../app/SettingsProvider";
import {
  compareSnapshots,
  deleteSnapshot,
  snapshotCapture,
  snapshotHistory,
  snapshotTimeline,
  toPrismError,
} from "../services/ipc";
import type { DomainDiffDto, SnapshotDiffDto, SnapshotRecordDto, TimelineDto } from "../types/prism";

/**
 * Snapshot History 页（M11b §127–§133/§49–§51/§129–§130）：
 * 列表（date/basis/revision/state/counts）→ 选择 A/B → Compare（分域 facts）→
 * Timeline（git + snapshot 派生）。数据全部来自 SnapshotStore（§126）。
 */
export default function SnapshotHistoryPage() {
  const { project } = useProjectStore();
  const { t } = useUiSettings();
  const [snapshots, setSnapshots] = useState<SnapshotRecordDto[] | null>(null);
  const [timeline, setTimeline] = useState<TimelineDto | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [selectedA, setSelectedA] = useState<string | null>(null);
  const [selectedB, setSelectedB] = useState<string | null>(null);
  const [diff, setDiff] = useState<SnapshotDiffDto | null>(null);
  const [comparing, setComparing] = useState(false);
  const [capturing, setCapturing] = useState(false);

  const load = () => {
    if (!project) return;
    void snapshotHistory()
      .then((list) => {
        setSnapshots(list);
        // 默认选最新两条（§251 latest/previous）。
        const first = list[0]?.id ?? null;
        const second = list[1]?.id ?? null;
        setSelectedB(first);
        setSelectedA(second);
      })
      .catch((err) => setError(toPrismError(err).message));
    void snapshotTimeline()
      .then(setTimeline)
      .catch(() => setTimeline(null));
  };

  useEffect(() => {
    if (!project) return;
    load();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [project?.root_path]);

  const capture = () => {
    setCapturing(true);
    void snapshotCapture(false)
      .then(() => {
        setCapturing(false);
        load();
      })
      .catch((err) => {
        setCapturing(false);
        setError(toPrismError(err).message);
      });
  };

  const runCompare = () => {
    if (!selectedA || !selectedB) return;
    setComparing(true);
    void compareSnapshots(selectedA, selectedB)
      .then(setDiff)
      .catch((err) => setError(toPrismError(err).message))
      .finally(() => setComparing(false));
  };

  const removeSnapshot = (id: string) => {
    void deleteSnapshot(id)
      .then(load)
      .catch((err) => setError(toPrismError(err).message));
  };

  if (!project) {
    return (
      <div className="card" data-testid="history-needs-project">
        <p className="muted">Open a project to view its snapshot history.</p>
      </div>
    );
  }
  if (error) {
    return (
      <div className="card" data-testid="history-error">
        <p className="status-error">Snapshot history failed: {error}</p>
      </div>
    );
  }
  if (snapshots === null) {
    return (
      <div className="card" data-testid="history-loading">
        <p className="muted">Loading snapshot history…</p>
      </div>
    );
  }
  if (snapshots.length === 0) {
    return (
      <div className="card" data-testid="history-empty">
        <h2 className="card-title">{t("history.title")}</h2>
        <p className="muted">{t("history.empty.first")}</p>
        <button type="button" className="linklike" onClick={capture} disabled={capturing}>
          {capturing ? t("history.capturing") : t("history.empty.capture")}
        </button>
      </div>
    );
  }

  return (
    <section className="stack">
      <div className="card" data-testid="history-list">
        <h2 className="card-title">{t("history.title")}</h2>
        <p className="muted small">{t("history.notBackups")}</p>
        <ul>
          {snapshots.map((snapshot) => (
            <li key={snapshot.id} className="small">
              <input
                type="radio"
                name="compare-a"
                aria-label={`A: ${snapshot.id}`}
                checked={selectedA === snapshot.id}
                onChange={() => setSelectedA(snapshot.id)}
              />
              <input
                type="radio"
                name="compare-b"
                aria-label={`B: ${snapshot.id}`}
                checked={selectedB === snapshot.id}
                onChange={() => setSelectedB(snapshot.id)}
              />
              <span className="mono">
                {new Date(snapshot.captured_at).toLocaleString()}
              </span>{" "}
              [{snapshot.basis}] {snapshot.working_tree_state}
              {snapshot.git_revision && (
                <span className="muted"> · {snapshot.git_revision.slice(0, 8)}</span>
              )}{" "}
              <span className="pill">{snapshot.state_fingerprint.slice(0, 8)}</span>
              {snapshot.note && <span className="muted"> · {snapshot.note}</span>}
              <button
                type="button"
                className="linklike"
                onClick={() => removeSnapshot(snapshot.id)}
                aria-label={`delete ${snapshot.id}`}
              >
                delete
              </button>
            </li>
          ))}
        </ul>
        <button
          type="button"
          className="linklike"
          data-testid="history-compare"
          disabled={!selectedA || !selectedB || selectedA === selectedB || comparing}
          onClick={runCompare}
        >
          {comparing ? t("history.capturing") : t("history.compare")}
        </button>
        <button type="button" className="linklike" onClick={capture} disabled={capturing}>
          Capture now
        </button>
      </div>

      {diff && (
        <div className="card" data-testid="history-compare">
          <h2 className="card-title">{t("history.comparison")} ({diff.compatibility})</h2>
          {(Object.entries({
            Structure: diff.structure,
            Code: diff.code,
            Architecture: diff.architecture,
            Git: diff.git,
            Change: diff.change,
            Impact: diff.impact,
            Health: diff.health,
            Verification: diff.verification,
            Environment: diff.environment,
            Context: diff.context_health,
          }) as [string, DomainDiffDto][]).map(([title, domain]) => (
            <div key={domain.domain} className="small">
              <strong>{title}</strong>: {domain.state}
              <ul>
                {domain.facts.map((fact, index) => (
                  <li key={index} className="muted small">
                    {fact}
                  </li>
                ))}
              </ul>
            </div>
          ))}
          {diff.finding_lifecycles.length > 0 && (
            <div className="small">
              <strong>Findings</strong>
              <ul>
                {diff.finding_lifecycles.map((entry) => (
                  <li key={entry.finding_fingerprint}>
                    {entry.rule_id}: {entry.lifecycle}
                  </li>
                ))}
              </ul>
            </div>
          )}
        </div>
      )}

      {timeline && (
        <div className="card" data-testid="history-timeline">
          <h2 className="card-title">{t("history.timeline")}</h2>
          <ul>
            {timeline.events.map((event) => (
              <li key={event.id} className="small">
                <span className="muted">
                  {new Date(event.timestamp).toLocaleString()}
                </span>{" "}
                [{event.source}] {event.title} — {event.summary}
              </li>
            ))}
            {timeline.events.length === 0 && <li className="muted">{t("history.noEvents")}</li>}
          </ul>
          {timeline.limitations.map((limitation) => (
            <p key={limitation} className="muted small">
              {limitation}
            </p>
          ))}
        </div>
      )}
    </section>
  );
}
