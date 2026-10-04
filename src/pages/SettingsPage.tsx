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
    void getStorageInfo().then(setStorage).catch(() => setStorage(null));
    void getCurrentProjectPath()
      .then((value) => setProjectRoot(value?.root ?? null))
      .catch(() => setProjectRoot(null));
  }, []);

  const languages: { value: Language; label: string }[] = [
    { value: "en", label: "English" },
    { value: "zh", label: "中文" },
  ];
  const themes: { value: Theme; label: string }[] = [
    { value: "dark", label: "Dark" },
    { value: "light", label: "Light" },
  ];
  const densities: { value: Density; label: string }[] = [
    { value: "comfortable", label: "Comfortable" },
    { value: "compact", label: "Compact" },
  ];

  return (
    <section className="stack">
      <div className="card" data-testid="settings-appearance">
        <h2 className="card-title">{t("settings.appearance")}</h2>
        <div className="small">
          <label>{t("settings.language")}: </label>
          {languages.map(({ value, label }) => (
            <label key={value} style={{ marginRight: 12 }}>
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
        <div className="small">
          <label>{t("settings.theme")}: </label>
          {themes.map(({ value, label }) => (
            <label key={value} style={{ marginRight: 12 }}>
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
        <div className="small">
          <label>{t("settings.density")}: </label>
          {densities.map(({ value, label }) => (
            <label key={value} style={{ marginRight: 12 }}>
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
        <p className="muted small">{t("settings.uiOnly")}</p>
      </div>

      <div className="card" data-testid="settings-storage">
        <h2 className="card-title">{t("settings.storage")}</h2>
        <p className="muted small">
          {t("settings.storageHint")}{" "}
          <span className="mono" data-testid="settings-db-path">
            {storage?.db_path ?? "unknown"}
          </span>{" "}
          {storage?.db_exists ? "(exists)" : "(not created yet)"}
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
