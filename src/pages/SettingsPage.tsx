import { useEffect, useState } from "react";
import { getStorageInfo, getCurrentProjectPath } from "../services/ipc";
import { useUiSettings } from "../app/SettingsProvider";
import type { Theme, Density } from "../app/uiSettings";

/**
 * Settings 页（M12 §48/§11：theme/density 基础偏好；§48 不做复杂矩阵）。
 * 全部 UI-only——不写项目、不动 Snapshot DB（§73/§316）。
 */
export default function SettingsPage() {
  const { settings, update } = useUiSettings();
  const [storage, setStorage] = useState<{ db_path: string; db_exists: boolean } | null>(null);
  const [projectRoot, setProjectRoot] = useState<string | null>(null);

  useEffect(() => {
    void getStorageInfo().then(setStorage).catch(() => setStorage(null));
    void getCurrentProjectPath()
      .then((value) => setProjectRoot(value?.root ?? null))
      .catch(() => setProjectRoot(null));
  }, []);

  return (
    <section className="stack">
      <div className="card" data-testid="settings-appearance">
        <h2 className="card-title">Appearance</h2>
        <div className="small">
          <label>Theme: </label>
          {(["dark", "light"] as Theme[]).map((theme) => (
            <label key={theme} style={{ marginRight: 12 }}>
              <input
                type="radio"
                name="theme"
                checked={settings.theme === theme}
                onChange={() => update({ theme })}
              />{" "}
              {theme}
            </label>
          ))}
        </div>
        <div className="small">
          <label>Density: </label>
          {(["comfortable", "compact"] as Density[]).map((density) => (
            <label key={density} style={{ marginRight: 12 }}>
              <input
                type="radio"
                name="density"
                checked={settings.density === density}
                onChange={() => update({ density })}
              />{" "}
              {density}
            </label>
          ))}
        </div>
        <p className="muted small">UI-only preferences, stored locally (§48).</p>
      </div>

      <div className="card" data-testid="settings-storage">
        <h2 className="card-title">Storage</h2>
        <p className="muted small">
          Snapshot database location (local-only, §21):{" "}
          <span className="mono" data-testid="settings-db-path">
            {storage?.db_path ?? "unknown"}
          </span>{" "}
          {storage?.db_exists ? "(exists)" : "(not created yet)"}
        </p>
        {projectRoot && (
          <p className="muted small">
            Current project: <span className="mono">{projectRoot}</span>
          </p>
        )}
      </div>
    </section>
  );
}
