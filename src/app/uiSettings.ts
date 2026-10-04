/** M12 §48/§11：UI 偏好（theme/density）——localStorage 持久化，UI-only。 */
export type Theme = "dark" | "light";
export type Density = "comfortable" | "compact";

export interface UiSettings {
  theme: Theme;
  density: Density;
}

const KEY = "prism.ui.settings";

export function loadSettings(): UiSettings {
  try {
    const raw = localStorage.getItem(KEY);
    if (raw) {
      const parsed = JSON.parse(raw) as Partial<UiSettings>;
      return {
        theme: parsed.theme === "light" ? "light" : "dark",
        density: parsed.density === "compact" ? "compact" : "comfortable",
      };
    }
  } catch {
    // localStorage 不可用 ⇒ 默认值（§258 无密码、无网络）。
  }
  return { theme: "dark", density: "comfortable" };
}

export function saveSettings(settings: UiSettings): void {
  try {
    localStorage.setItem(KEY, JSON.stringify(settings));
  } catch {
    // 忽略：UI 偏好写失败不影响功能。
  }
}

/** 应用到 documentElement（data-theme / data-density）。 */
export function applySettings(settings: UiSettings): void {
  document.documentElement.dataset.theme = settings.theme;
  document.documentElement.dataset.density = settings.density;
}
