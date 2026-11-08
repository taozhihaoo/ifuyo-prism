import { useUiSettings } from "../app/SettingsProvider";

interface StatusBarProps {
  version: string | null;
  error: string | null;
}

/** 全局状态栏：隐私立场常驻展示；Core 不可达时如实报告，不伪造状态。 */
export default function StatusBar({ version, error }: StatusBarProps) {
  const { t } = useUiSettings();
  return (
    <footer className="status-bar">
      <span className="status-privacy">
        <span className="status-dot" aria-hidden="true" />
        {t("status.privacy")}
      </span>
      <span className="status-spacer" />
      {error ? (
        <span className="status-error" role="alert">
          {t("status.coreError")} {error}
        </span>
      ) : (
        <span className="status-version">
          {version ? `Prism v${version}` : t("status.connecting")}
        </span>
      )}
    </footer>
  );
}
