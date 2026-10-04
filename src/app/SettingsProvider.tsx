import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import { applySettings, loadSettings, saveSettings, type UiSettings } from "./uiSettings";

/**
 * UI 设置上下文（M12 §48/§11）：theme/density——UI-only 偏好，
 * localStorage 持久化；不引入状态管理框架（§0.1）。
 */
interface SettingsContextValue {
  settings: UiSettings;
  update: (patch: Partial<UiSettings>) => void;
}

const SettingsContext = createContext<SettingsContextValue | null>(null);

export function SettingsProvider({ children }: { children: ReactNode }) {
  const [settings, setSettings] = useState<UiSettings>(() => loadSettings());

  useEffect(() => {
    applySettings(settings);
    saveSettings(settings);
  }, [settings]);

  const update = useCallback((patch: Partial<UiSettings>) => {
    setSettings((previous) => ({ ...previous, ...patch }));
  }, []);

  const value = useMemo(() => ({ settings, update }), [settings, update]);
  return <SettingsContext.Provider value={value}>{children}</SettingsContext.Provider>;
}

export function useUiSettings(): SettingsContextValue {
  const context = useContext(SettingsContext);
  if (!context) {
    throw new Error("useUiSettings must be used within SettingsProvider");
  }
  return context;
}
