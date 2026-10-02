import { useEffect, useState } from "react";
import { getAppInfo, toPrismError } from "../services/ipc";
import type { AppInfo } from "../types/prism";
import Sidebar from "../components/Sidebar";
import StatusBar from "../components/StatusBar";
import { PAGES, type PageKey } from "../pages/registry";
import { ProjectProvider } from "./ProjectStore";

/**
 * 应用 Shell：固定导航 + 主内容 + 状态栏。
 * 页面切换用本地 state；不引入路由依赖（M0 无深层路由需求）。
 * 项目/扫描状态集中在 ProjectProvider（纲领 §67）。
 */
export default function App() {
  const [active, setActive] = useState<PageKey>("overview");
  const [appInfo, setAppInfo] = useState<AppInfo | null>(null);
  const [error, setError] = useState<string | null>(null);

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
    return () => {
      cancelled = true;
    };
  }, []);

  const page = PAGES.find((p) => p.key === active) ?? PAGES[0];

  return (
    <ProjectProvider>
      <div className="app-shell">
        <aside className="sidebar">
          <div className="brand">
            <span className="brand-mark" aria-hidden="true" />
            <span>Prism</span>
          </div>
          <nav aria-label="Primary">
            <Sidebar active={active} onSelect={setActive} />
          </nav>
        </aside>
        <main className="main-content">
          <h1 className="page-title">{page?.label}</h1>
          {page?.render(appInfo)}
        </main>
        <StatusBar version={appInfo?.version ?? null} error={error} />
      </div>
    </ProjectProvider>
  );
}
