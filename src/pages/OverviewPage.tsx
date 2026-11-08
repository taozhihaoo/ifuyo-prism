import { useEffect, useState } from "react";
import { formatBytesLabel } from "../services/format";
import { useProjectStore } from "../app/ProjectStore";
import { useUiSettings } from "../app/SettingsProvider";
import { format } from "../app/i18n";
import ScanProgressCard from "../components/ScanProgressCard";
import ProjectTypesCard from "../components/ProjectTypesCard";
import IntelligenceSummaryCard from "../components/IntelligenceSummaryCard";
import FullAnalysisCard from "../components/FullAnalysisCard";
import {
  getGitFacts,
  getHealthStatus,
  getVerifyResults,
  getVerifyStatus,
  snapshotHistory,
} from "../services/ipc";
import type { SnapshotRecordDto } from "../types/prism";
import type { CheckRunDto, VerificationReportDto } from "../types/prism";
import type { AppInfo } from "../types/prism";

interface OverviewPageProps {
  appInfo: AppInfo | null;
}

/** 本 session 的验证摘要（§212/§213/§275/§276/§277）。 */
interface VerifyOverview {
  report: VerificationReportDto;
  runs: CheckRunDto[];
  stale: boolean;
  staleReason: string | null;
}

function staleReasonFor(
  report: VerificationReportDto,
  headCommit: string | null,
  dirtyNow: number,
  t: (key: string) => string,
): { stale: boolean; reason: string | null } {
  const state = report.verified_state;
  if (!state) return { stale: false, reason: null };
  if (state.kind === "clean_at_commit") {
    if (headCommit !== null && state.commit !== null && headCommit !== state.commit) {
      return { stale: true, reason: t("overview.staleHead") };
    }
    if (dirtyNow > 0) {
      return { stale: true, reason: t("overview.staleWorktree") };
    }
  }
  return { stale: false, reason: null };
}

/** Overview：打开项目 → 扫描 → 真实 Discovery 摘要（纲领 §38/§42/§74/§75）。 */
export default function OverviewPage({ appInfo }: OverviewPageProps) {
  const { project, phase, scanStatus, discovery, pickAndOpenProject, lastError } =
    useProjectStore();
  const { t } = useUiSettings();
  const [verifyOverview, setVerifyOverview] = useState<VerifyOverview | null>(null);
  const [verifyChecked, setVerifyChecked] = useState(false);
  const [latestSnapshot, setLatestSnapshot] = useState<SnapshotRecordDto | null>(null);
  const [healthState, setHealthState] = useState<string | null>(null);
  const [verifyState, setVerifyState] = useState<string | null>(null);

  useEffect(() => {
    if (!project?.root_path) return;
    let disposed = false;
    // §212/§275：只报告本 session 事实；无结果就直说，不假扮历史。
    void (async () => {
      try {
        const [results, facts] = await Promise.all([
          getVerifyResults(),
          getGitFacts(false),
        ]);
        // §14 Overview Status Cards：真实 session 事实（无则如实 idle）。
        void snapshotHistory()
          .then((list) => {
            if (!disposed) setLatestSnapshot(list[0] ?? null);
          })
          .catch(() => {});
        void getHealthStatus()
          .then((status) => {
            if (!disposed) setHealthState(status.status);
          })
          .catch(() => {});
        void getVerifyStatus()
          .then((status) => {
            if (!disposed) setVerifyState(status.status);
          })
          .catch(() => {});
        if (disposed) return;
        if (results?.report) {
          const headCommit = facts.head?.commit_id ?? null;
          const dirtyNow = facts.status.entries.length;
          const { stale, reason } = staleReasonFor(results.report, headCommit, dirtyNow, t);
          setVerifyOverview({
            report: results.report,
            runs: results.report.runs,
            stale,
            staleReason: reason,
          });
        } else {
          setVerifyOverview(null);
        }
      } catch {
        if (!disposed) setVerifyOverview(null);
      } finally {
        if (!disposed) setVerifyChecked(true);
      }
    })();
    return () => {
      disposed = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [project?.root_path]);

  const openProject = () => {
    // ProjectStore 内部记录 lastError；这里只吞掉 rejection，避免未处理异常。
    void pickAndOpenProject().catch(() => {});
  };

  return (
    <section className="stack">
      {phase === "noProject" && (
        <div className="card empty-hero" data-testid="empty-state">
          <span className="empty-hero-mark" aria-hidden="true">◇</span>
          <h2 className="card-title">{t("overview.noProject")}</h2>
          <p className="muted">{t("overview.noProjectHint")}</p>
          <button
            type="button"
            className="button button-primary"
            onClick={openProject}
          >
            {t("overview.openProject")}
          </button>
          {lastError && (
            <p className="status-error" role="alert">
              {lastError}
            </p>
          )}
        </div>
      )}

      {phase === "scanning" && <ScanProgressCard />}

      {project && phase !== "scanning" && discovery && (
        <>
          <div className="card hero-card" data-testid="project-summary">
            <div className="card-head">
              <h2 className="card-title">{t("overview.project")}</h2>
              <span className="pill pill-accent">
                {format(t("overview.languageCount"), { n: discovery.languages.length })}
              </span>
            </div>
            <dl className="fact-grid">
              <div>
                <dt>{t("overview.name")}</dt>
                <dd>{discovery.project.name}</dd>
              </div>
              <div className="fact-path">
                <dt>{t("overview.root")}</dt>
                <dd title={discovery.project.root_path}>{discovery.project.root_path}</dd>
              </div>
              <div>
                <dt>{t("overview.files")}</dt>
                <dd>{discovery.statistics.total_files}</dd>
              </div>
              <div>
                <dt>{t("overview.directories")}</dt>
                <dd>{discovery.statistics.total_directories}</dd>
              </div>
              <div>
                <dt>{t("overview.size")}</dt>
                <dd>{formatBytesLabel(discovery.statistics.total_size_bytes)}</dd>
              </div>
              <div>
                <dt>{t("overview.scanDuration")}</dt>
                <dd>{discovery.scan.duration_ms} ms</dd>
              </div>
            </dl>
            {(phase === "completedWithWarnings" || phase === "cancelled") && (
              <div className="warning-box" data-testid="warnings">
                <p>
                  {phase === "cancelled"
                    ? t("overview.scanCancelled")
                    : format(t("overview.scanWarnings"), { n: discovery.scan.errors_count })}
                </p>
                {discovery.errors.length > 0 && (
                  <ul>
                    {discovery.errors.slice(0, 20).map((error, index) => (
                      <li key={index} className="mono small">
                        {error.path}: {error.message}
                      </li>
                    ))}
                  </ul>
                )}
              </div>
            )}
          </div>

          {(latestSnapshot || healthState || verifyState) && (
            <div className="card" data-testid="overview-status-cards">
              <h2 className="card-title">{t("overview.status")}</h2>
              <dl className="fact-grid">
                <div>
                  <dt>{t("overview.health")}</dt>
                  <dd>{healthState ?? t("overview.notRun")}</dd>
                </div>
                <div>
                  <dt>{t("overview.verify")}</dt>
                  <dd>{verifyState ?? t("overview.notRun")}</dd>
                </div>
                <div>
                  <dt>{t("overview.lastSnapshot")}</dt>
                  <dd>
                    {latestSnapshot
                      ? new Date(latestSnapshot.captured_at).toLocaleString()
                      : t("overview.none")}
                  </dd>
                </div>
              </dl>
              <p className="muted small">{t("overview.sessionNote")}</p>
            </div>
          )}

          <ProjectTypesCard candidates={discovery.project_types} />

          <FullAnalysisCard />

          <IntelligenceSummaryCard />

          <div className="card" data-testid="overview-verify">
            <h2 className="card-title">{t("overview.verification")}</h2>
            {!verifyChecked || verifyOverview === null ? (
              <p className="muted small" data-testid="verify-no-run">
                {t("overview.noVerificationRun")}
              </p>
            ) : (
              <>
                <ul className="row-list">
                  {verifyOverview.runs.map((run) => (
                    <li key={run.run_id} className="row-static">
                      <span>{run.kind}</span>
                      <span className="pill">{run.state}</span>
                    </li>
                  ))}
                </ul>
                {verifyOverview.stale && (
                  <p className="warning-box small" data-testid="verify-stale">
                    {t("overview.verificationStale")} {verifyOverview.staleReason}
                  </p>
                )}
                {verifyOverview.report.verified_state?.kind === "dirty_workspace" && (
                  <p className="muted small">
                    {format(t("overview.workingTreeChanges"), {
                      n: verifyOverview.report.verified_state.changed_files,
                    })}
                  </p>
                )}
                <p className="muted small">{t("overview.sessionOnly")}</p>
              </>
            )}
          </div>

          <div className="card">
            <h2 className="card-title">{t("overview.languages")}</h2>
            {discovery.languages.length === 0 ? (
              <p className="muted">{t("overview.noLanguages")}</p>
            ) : (
              <ul className="row-list">
                {discovery.languages.map((entry) => (
                  <li key={entry.language} className="row-static">
                    <span>{entry.language}</span>
                    <span className="pill">{entry.count}</span>
                  </li>
                ))}
              </ul>
            )}
            <p className="muted small">{t("overview.fileCountNote")}</p>
          </div>

          <div className="card">
            <h2 className="card-title">{t("overview.entryPoints")}</h2>
            {discovery.entry_points.length === 0 ? (
              <p className="muted">{t("overview.noEntryPoints")}</p>
            ) : (
              <ul className="row-list">
                {discovery.entry_points.map((entry) => (
                  <li key={entry.path} className="row-static">
                    <span className="mono">{entry.path}</span>
                    <span className="pill">
                      {entry.confidence} · {entry.kind}
                    </span>
                  </li>
                ))}
              </ul>
            )}
            <p className="muted small">{t("overview.candidatesNote")}</p>
          </div>
        </>
      )}

      {project && !discovery && phase !== "scanning" && (
        <div className="card" data-testid="scan-terminal-state">
          <h2 className="card-title">
            {phase === "cancelled"
              ? t("overview.scanCancelledTitle")
              : phase === "failed"
                ? t("overview.scanFailedTitle")
                : t("overview.readyTitle")}
          </h2>
          <p className="muted">
            {phase === "failed"
              ? (scanStatus?.error ?? t("overview.scanFailedHint"))
              : t("overview.readyHint")}
          </p>
        </div>
      )}

      {appInfo && (
        <div className="card">
          <h2 className="card-title">{t("overview.application")}</h2>
          <dl className="fact-grid">
            <div>
              <dt>{t("overview.name")}</dt>
              <dd>{appInfo.name}</dd>
            </div>
            <div>
              <dt>{t("overview.version")}</dt>
              <dd>{appInfo.version}</dd>
            </div>
            <div>
              <dt>{t("overview.os")}</dt>
              <dd>{appInfo.os}</dd>
            </div>
            <div>
              <dt>{t("overview.arch")}</dt>
              <dd>{appInfo.arch}</dd>
            </div>
            <div>
              <dt>{t("overview.buildProfile")}</dt>
              <dd>{appInfo.build_profile}</dd>
            </div>
            <div>
              <dt>{t("overview.generatedAt")}</dt>
              <dd>{appInfo.generated_at}</dd>
            </div>
          </dl>
        </div>
      )}
    </section>
  );
}
