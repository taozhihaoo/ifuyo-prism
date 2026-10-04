import { useEffect, useState } from "react";
import { getAppInfo, listRecentProjects, openProject, toPrismError } from "../services/ipc";
import type { AppInfo } from "../types/prism";
import Sidebar from "../components/Sidebar";
import StatusBar from "../components/StatusBar";
import CommandPalette from "../components/CommandPalette";
import { PAGES, type PageKey } from "../pages/registry";
import { ProjectProvider } from "./ProjectStore";
import { SettingsProvider, useUiSettings } from "./SettingsProvider";

interface RecentProject {
  display_name: string;
  canonical_root: string;
}

/**
 * 应用 Shell（M12 §8/§33/§34/§46）：固定导航 + 主内容 + 状态栏 +
 * Command Palette（Ctrl+K）+ 页面快捷键（Ctrl+1..8）。
 * 页面切换用本地 state；不引入路由依赖（M0 无深层路由需求）。
 */
function Shell() {
  const [active, setActive] = useState<PageKey>("overview");
  const [appInfo, setAppInfo] = useState<AppInfo | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [paletteOpen, setPaletteOpen] = useState(false);
  const [recentProjects, setRecentProjects] = useState<RecentProject[]>([]);
  const { settings, t } = useUiSettings();

  // theme/density 应用到 root（§11/§48）。
  useEffect(() => {
    document.documentElement.dataset.theme = settings.theme;
    document.documentElement.dataset.density = settings.density;
  }, [settings]);

  const refreshRecent = () => {
    listRecentProjects()
      .then(setRecentProjects)
      .catch(() => setRecentProjects([]));
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
  }, []);

  const openByPath = (path: string) => {
    openProject(path)
      .then(() => {
        setError(null);
        refreshRecent();
      })
      .catch((err) => setError(toPrismError(err).message));
  };

  // §46：Ctrl+K palette；Ctrl+1..9 页面切换。
  useEffect(() => {
    const on_key = (event: KeyboardEvent) => {
      if (event.ctrlKey && event.key.toLowerCase() === "k") {
        event.preventDefault();
        setPaletteOpen((open) => !open);
        return;
      }
      if (event.ctrlKey && event.key >= "1" && event.key <= "9") {
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

  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="brand">
          <span className="brand-mark" aria-hidden="true" />
          <span>Prism</span>
        </div>
        <nav aria-label="Primary">
          <Sidebar active={active} onSelect={setActive} />
        </nav>
        <button
          type="button"
          className="linklike palette-trigger"
          data-testid="open-palette"
          onClick={() => setPaletteOpen(true)}
        >
          Search… Ctrl+K
        </button>
      </aside>
      <main className="main-content">
        <h1 className="page-title">{page_label}</h1>
        {page?.render(appInfo)}
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
