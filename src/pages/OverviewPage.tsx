import type { AppInfo } from "../types/prism";

interface OverviewPageProps {
  appInfo: AppInfo | null;
}

/**
 * Overview：M0 唯一提供真实数据的页面。
 * 只展示来自 Core 的事实（应用/运行时信息）与各能力的真实交付状态。
 * 禁止伪造计数或评分（纲领 §38）。
 */
export default function OverviewPage({ appInfo }: OverviewPageProps) {
  if (!appInfo) {
    return (
      <section className="card">
        <p className="muted">Connecting to Prism core…</p>
      </section>
    );
  }

  return (
    <section className="stack">
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

      <div className="card">
        <h2 className="card-title">Capabilities</h2>
        <p className="muted">
          Prism grows milestone by milestone. Areas below are not implemented
          yet — the sidebar marks when each arrives.
        </p>
        <ul className="capability-list">
          <li>
            <span>Understand</span> <span className="pill">M1</span>
          </li>
          <li>
            <span>Changes</span> <span className="pill">M4</span>
          </li>
          <li>
            <span>Health</span> <span className="pill">M7</span>
          </li>
          <li>
            <span>Verify</span> <span className="pill">M8</span>
          </li>
          <li>
            <span>Environment</span> <span className="pill">M9</span>
          </li>
          <li>
            <span>Context</span> <span className="pill">M10</span>
          </li>
        </ul>
      </div>
    </section>
  );
}
