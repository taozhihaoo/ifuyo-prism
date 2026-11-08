import { useEffect, useRef, useState } from "react";
import { getAppInfo, listRecentProjects, openProject, toPrismError } from "../services/ipc";
import type { AppInfo } from "../types/prism";
import Sidebar, { NAV_GROUPS } from "../components/Sidebar";
import StatusBar from "../components/StatusBar";
import CommandPalette from "../components/CommandPalette";
import { PAGES, type PageKey } from "../pages/registry";
import { ProjectProvider } from "./ProjectStore";
import { SettingsProvider, useUiSettings } from "./SettingsProvider";

interface RecentProject {
  display_name: string;
  canonical_root: string;
}

/** 输入框/文本域/可编辑区域内的快捷键不应劫持用户正在输入的文本。 */
function isEditableTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  if (target.isContentEditable) return true;
  const tag = target.tagName.toLowerCase();
  return tag === "input" || tag === "textarea" || tag === "select";
}

/**
 * 应用 Shell（M12 §8/§33/§34/§46）：固定导航 + 主内容 + 状态栏 +
 * Command Palette（Ctrl+K）+ 页面快捷键（Ctrl+1..9）。
 * 页面切换用本地 state；不引入路由依赖（M0 无深层路由需求）。
 */
function Shell() {
  const [active, setActive] = useState<PageKey>("overview");
  const [appInfo, setAppInfo] = useState<AppInfo | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [paletteOpen, setPaletteOpen] = useState(false);
  const [recentProjects, setRecentProjects] = useState<RecentProject[]>([]);
  const { settings, t } = useUiSettings();
  const aliveRef = useRef(true);

  // theme/density 应用到 root（§11/§48）。
  useEffect(() => {
    document.documentElement.dataset.theme = settings.theme;
    document.documentElement.dataset.density = settings.density;
  }, [settings]);

  // 卸载后不再写入 state（StrictMode 双挂载时重新置活）。
  useEffect(() => {
    aliveRef.current = true;
    return () => {
      aliveRef.current = false;
    };
  }, []);

  const refreshRecent = () => {
    listRecentProjects()
      .then((list) => {
        if (aliveRef.current) setRecentProjects(list);
      })
      .catch(() => {
        if (aliveRef.current) setRecentProjects([]);
      });
  };

  useEffect(() => {
    let cancelled = false;
    getAppInfo()
      .then((info) => {
        if (!cancelled) {
          setAppInfo(info);
          setError(null);
        }
      })
      .catch((err) => {
        if (!cancelled) setError(toPrismError(err).message);
      });
    refreshRecent();
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const openByPath = (path: string) => {
    openProject(path)
      .then(() => {
        if (!aliveRef.current) return;
        setError(null);
        refreshRecent();
      })
      .catch((err) => {
        if (aliveRef.current) setError(toPrismError(err).message);
      });
  };

  // §46：Ctrl/Cmd+K palette；Ctrl/Cmd+1..9 页面切换。
  // 可编辑控件内不劫持快捷键，避免在搜索框/文本框中误切页面。
  useEffect(() => {
    const on_key = (event: KeyboardEvent) => {
      const mod = event.ctrlKey || event.metaKey;
      if (!mod || isEditableTarget(event.target)) return;
      if (event.key.toLowerCase() === "k") {
        event.preventDefault();
        setPaletteOpen((open) => !open);
        return;
      }
      if (event.key >= "1" && event.key <= "9") {
        const page_index = Number(event.key) - 1;
        const target = PAGES[page_index];
        if (target) {
          event.preventDefault();
          setActive(target.key);
        }
      }
    };
    window.addEventListener("keydown", on_key);
    return () => window.removeEventListener("keydown", on_key);
  }, []);

  const page = PAGES.find((p) => p.key === active) ?? PAGES[0];
  const page_label = page ? t(`page.${page.key}`) : "";
  const group = NAV_GROUPS.find((entry) => entry.keys.includes(active));
  const group_label = group ? t(group.titleKey) : "";

  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="brand">
          <span className="brand-mark" aria-hidden="true">
            <svg viewBox="0 0 32 32" fill="none">
              <path d="M16 3 29 25H3L16 3Z" stroke="currentColor" strokeWidth="1.8" strokeLinejoin="round" />
              <path d="M16 3v22M9 16h14" stroke="currentColor" strokeWidth="1.2" strokeOpacity=".55" />
            </svg>
          </span>
          <span className="brand-text">
            <strong>Prism</strong>
            <small>{t("shell.tagline")}</small>
          </span>
        </div>

        <nav className="sidebar-nav" aria-label="Primary">
          <Sidebar active={active} onSelect={setActive} />
        </nav>

        <div className="sidebar-footer">
          <button
            type="button"
            className="palette-trigger"
            data-testid="open-palette"
            aria-label={t("common.search")}
            onClick={() => setPaletteOpen(true)}
          >
            <span className="palette-trigger-icon" aria-hidden="true">⌕</span>
            <span className="palette-trigger-label">{t("common.searchLabel")}</span>
            <kbd>Ctrl K</kbd>
          </button>
          <div className="sidebar-badge">
            <span className="status-dot" aria-hidden="true" />
            {t("shell.localOnly")}
          </div>
        </div>
      </aside>

      <main className="main-content">
        <header className="page-header">
          <div className="page-header-main">
            <div className="page-eyebrow">
              <span>{group_label}</span>
              <span className="eyebrow-sep" aria-hidden="true">/</span>
              <span className="mono">{page?.key ?? ""}</span>
            </div>
            <h1 className="page-title">{page_label}</h1>
            <p className="page-subtitle">{t(`page.${page?.key}.subtitle`)}</p>
          </div>
          <div className="page-header-meta">
            <span className="header-chip">
              <span className="status-dot" aria-hidden="true" />
              {t("shell.offlineReady")}
            </span>
            {appInfo && <span className="header-chip mono">v{appInfo.version}</span>}
          </div>
        </header>
        <div className="page-body">{page?.render(appInfo)}</div>
      </main>

      <StatusBar version={appInfo?.version ?? null} error={error} />
      {paletteOpen && (
        <CommandPalette
          active={active}
          recentProjects={recentProjects}
          onNavigate={setActive}
          onOpenProject={openByPath}
          onClose={() => setPaletteOpen(false)}
        />
      )}
    </div>
  );
}

export default function App() {
  return (
    <SettingsProvider>
      <ProjectProvider>
        <Shell />
      </ProjectProvider>
    </SettingsProvider>
  );
}
