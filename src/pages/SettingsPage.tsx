import { useEffect, useState } from "react";
import { getStorageInfo, getCurrentProjectPath } from "../services/ipc";
import { useUiSettings } from "../app/SettingsProvider";
import type { Density, Language, Theme } from "../app/uiSettings";

/**
 * Settings 页（M12 §48/§11：theme/density/language 基础偏好；§48 不做复杂矩阵）。
 * 全部 UI-only——不写项目、不动 Snapshot DB（§73/§316）。
 */
export default function SettingsPage() {
  const { settings, update, t } = useUiSettings();
  const [storage, setStorage] = useState<{ db_path: string; db_exists: boolean } | null>(null);
  const [projectRoot, setProjectRoot] = useState<string | null>(null);

  useEffect(() => {
    let disposed = false;
    void getStorageInfo()
      .then((value) => {
        if (!disposed) setStorage(value);
      })
      .catch(() => {
        if (!disposed) setStorage(null);
      });
    void getCurrentProjectPath()
      .then((value) => {
        if (!disposed) setProjectRoot(value?.root ?? null);
      })
      .catch(() => {
        if (!disposed) setProjectRoot(null);
      });
    return () => {
      disposed = true;
    };
  }, []);

  const languages: { value: Language; label: string }[] = [
    { value: "en", label: "English" },
    { value: "zh", label: "中文" },
  ];
  const themes: { value: Theme; label: string }[] = [
    { value: "dark", label: t("settings.theme.dark") },
    { value: "light", label: t("settings.theme.light") },
  ];
  const densities: { value: Density; label: string }[] = [
    { value: "comfortable", label: t("settings.density.comfortable") },
    { value: "compact", label: t("settings.density.compact") },
  ];

  return (
    <section className="stack">
      <div className="card" data-testid="settings-appearance">
        <h2 className="card-title">{t("settings.appearance")}</h2>
        <div className="setting-row">
          <label className="setting-label">{t("settings.language")}: </label>
          <div className="setting-options">
            {languages.map(({ value, label }) => (
              <label key={value} className={`setting-option${settings.language === value ? " is-selected" : ""}`}>
                <input
                  type="radio"
                  name="language"
                  checked={settings.language === value}
                  onChange={() => update({ language: value })}
                />{" "}
                {label}
              </label>
            ))}
          </div>
        </div>
        <div className="setting-row">
          <label className="setting-label">{t("settings.theme")}: </label>
          <div className="setting-options">
            {themes.map(({ value, label }) => (
              <label key={value} className={`setting-option${settings.theme === value ? " is-selected" : ""}`}>
                <input
                  type="radio"
                  name="theme"
                  checked={settings.theme === value}
                  onChange={() => update({ theme: value })}
                />{" "}
                {label}
              </label>
            ))}
          </div>
        </div>
        <div className="setting-row">
          <label className="setting-label">{t("settings.density")}: </label>
          <div className="setting-options">
            {densities.map(({ value, label }) => (
              <label key={value} className={`setting-option${settings.density === value ? " is-selected" : ""}`}>
                <input
                  type="radio"
                  name="density"
                  checked={settings.density === value}
                  onChange={() => update({ density: value })}
                />{" "}
                {label}
              </label>
            ))}
          </div>
        </div>
        <p className="muted small setting-note">{t("settings.uiOnly")}</p>
      </div>

      <div className="card" data-testid="settings-storage">
        <h2 className="card-title">{t("settings.storage")}</h2>
        <p className="muted small">
          {t("settings.storageHint")}{" "}
          <span className="mono" data-testid="settings-db-path">
            {storage?.db_path ?? t("common.unknown")}
          </span>{" "}
          {storage?.db_exists ? t("settings.exists") : t("settings.notCreated")}
        </p>
        {projectRoot && (
          <p className="muted small">
            {t("settings.currentProject")} <span className="mono">{projectRoot}</span>
          </p>
        )}
      </div>
    </section>
  );
}
