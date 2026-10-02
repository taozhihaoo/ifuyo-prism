interface StatusBarProps {
  version: string | null;
  error: string | null;
}

/** 全局状态栏：隐私立场常驻展示；Core 不可达时如实报告，不伪造状态。 */
export default function StatusBar({ version, error }: StatusBarProps) {
  return (
    <footer className="status-bar">
      <span className="status-privacy">local-first · no telemetry</span>
      <span className="status-spacer" />
      {error ? (
        <span className="status-error" role="alert">
          core unavailable: {error}
        </span>
      ) : (
        <span className="status-version">{version ? `Prism v${version}` : "connecting to core…"}</span>
      )}
    </footer>
  );
}
