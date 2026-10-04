import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import {
  applySettings,
  loadSettings,
  saveSettings,
  type UiSettings,
} from "./uiSettings";
import { translate } from "./i18n";

/**
 * UI 设置上下文（M12 §48/§11 + 语言设置）：theme/density/language——
 * UI-only 偏好，localStorage 持久化；不引入状态管理框架（§0.1）。
 */
interface SettingsContextValue {
  settings: UiSettings;
  /** 当前语言的界面文案翻译函数。 */
  t: (key: string) => string;
  update: (patch: Partial<UiSettings>) => void;
}

const SettingsContext = createContext<SettingsContextValue | null>(null);

export function SettingsProvider({ children }: { children: ReactNode }) {
  const [settings, setSettings] = useState<UiSettings>(() => loadSettings());

  useEffect(() => {
    applySettings(settings);
    saveSettings(settings);
  }, [settings]);

  const t = useCallback(
    (key: string) => translate(settings.language, key),
    [settings.language],
  );

  const update = useCallback((patch: Partial<UiSettings>) => {
    setSettings((previous) => ({ ...previous, ...patch }));
  }, []);

  const value = useMemo(() => ({ settings, t, update }), [settings, t, update]);
  return <SettingsContext.Provider value={value}>{children}</SettingsContext.Provider>;
}

export function useUiSettings(): SettingsContextValue {
  // 无 Provider 时回退默认（en/dark/comfortable）：页面测试与独立渲染
  // 不依赖外层 Shell（§0.1 最小改动）。
  const context = useContext(SettingsContext);
  if (context) return context;
  return {
    settings: { theme: "dark", density: "comfortable", language: "en" },
    t: (key: string) => translate("en", key),
    update: () => {},
  };
}
