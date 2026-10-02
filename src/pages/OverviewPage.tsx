import { formatBytesLabel } from "../services/format";
import { useProjectStore } from "../app/ProjectStore";
import ScanProgressCard from "../components/ScanProgressCard";
import ProjectTypesCard from "../components/ProjectTypesCard";
import type { AppInfo } from "../types/prism";

interface OverviewPageProps {
  appInfo: AppInfo | null;
}

/** Overview：打开项目 → 扫描 → 真实 Discovery 摘要（纲领 §38/§42/§74/§75）。 */
export default function OverviewPage({ appInfo }: OverviewPageProps) {
  const { project, phase, scanStatus, discovery, pickAndOpenProject, lastError } =
    useProjectStore();

  return (
    <section className="stack">
      {phase === "noProject" && (
        <div className="card" data-testid="empty-state">
          <h2 className="card-title">No project opened.</h2>
          <p className="muted">
            Open a local project to begin. Prism scans it locally — nothing leaves your
            machine.
          </p>
          <button
            type="button"
            className="button button-primary"
            onClick={() => void pickAndOpenProject()}
          >
            Open Project
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
          <div className="card" data-testid="project-summary">
            <h2 className="card-title">Project</h2>
            <dl className="fact-grid">
              <div>
                <dt>Name</dt>
                <dd>{discovery.project.name}</dd>
              </div>
              <div>
                <dt>Root</dt>
                <dd>{discovery.project.root_path}</dd>
              </div>
              <div>
                <dt>Files</dt>
                <dd>{discovery.statistics.total_files}</dd>
              </div>
              <div>
                <dt>Directories</dt>
                <dd>{discovery.statistics.total_directories}</dd>
              </div>
              <div>
                <dt>Size</dt>
                <dd>{formatBytesLabel(discovery.statistics.total_size_bytes)}</dd>
              </div>
              <div>
                <dt>Scan duration</dt>
                <dd>{discovery.scan.duration_ms} ms</dd>
              </div>
            </dl>
            {(phase === "completedWithWarnings" || phase === "cancelled") && (
              <div className="warning-box" data-testid="warnings">
                <p>
                  {phase === "cancelled"
                    ? "Scan was cancelled — partial results below."
                    : `Scan completed with warnings (${discovery.scan.errors_count}).`}
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

          <ProjectTypesCard candidates={discovery.project_types} />

          <div className="card">
            <h2 className="card-title">Languages</h2>
            {discovery.languages.length === 0 ? (
              <p className="muted">No recognized source files.</p>
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
            <p className="muted small">File counts, not lines of code.</p>
          </div>

          <div className="card">
            <h2 className="card-title">Candidate entry points</h2>
            {discovery.entry_points.length === 0 ? (
              <p className="muted">No conventional entry points found.</p>
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
            <p className="muted small">
              Candidates with reasons — not a single verdict. See Understand for structure.
            </p>
          </div>
        </>
      )}

      {project && !discovery && phase !== "scanning" && (
        <div className="card" data-testid="scan-terminal-state">
          <h2 className="card-title">
            {phase === "cancelled"
              ? "Scan cancelled"
              : phase === "failed"
                ? "Scan failed"
                : "Ready to scan"}
          </h2>
          <p className="muted">
            {phase === "failed"
              ? (scanStatus?.error ?? "The scan failed — see logs for details.")
              : "Run a scan to discover this project's structure."}
          </p>
        </div>
      )}

      {appInfo && (
        <div className="card">
          <h2 className="card-title">Application</h2>
          <dl className="fact-grid">
            <div>
              <dt>Name</dt>
              <dd>{appInfo.name}</dd>
            </div>
            <div>
              <dt>Version</dt>
              <dd>{appInfo.version}</dd>
            </div>
            <div>
              <dt>OS</dt>
              <dd>{appInfo.os}</dd>
            </div>
            <div>
              <dt>Arch</dt>
              <dd>{appInfo.arch}</dd>
            </div>
            <div>
              <dt>Build profile</dt>
              <dd>{appInfo.build_profile}</dd>
            </div>
            <div>
              <dt>Generated at</dt>
              <dd>{appInfo.generated_at}</dd>
            </div>
          </dl>
        </div>
      )}
    </section>
  );
}
