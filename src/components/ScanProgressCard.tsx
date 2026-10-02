import { useProjectStore } from "../app/ProjectStore";

/** 扫描中卡片：显示真实计数与取消按钮（纲领 §39/§40/§41）。 */
export default function ScanProgressCard() {
  const { scanStatus, cancelScan } = useProjectStore();
  if (!scanStatus) return null;

  return (
    <div className="card" data-testid="scan-progress">
      <h2 className="card-title">Scanning project…</h2>
      <dl className="fact-grid">
        <div>
          <dt>Directories</dt>
          <dd>{scanStatus.directories}</dd>
        </div>
        <div>
          <dt>Files</dt>
          <dd>{scanStatus.files}</dd>
        </div>
        <div>
          <dt>Ignored</dt>
          <dd>{scanStatus.ignored}</dd>
        </div>
        <div>
          <dt>Warnings</dt>
          <dd>{scanStatus.errors}</dd>
        </div>
      </dl>
      <button type="button" className="button" onClick={cancelScan}>
        Cancel scan
      </button>
    </div>
  );
}
