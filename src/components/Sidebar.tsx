import type { ReactNode } from "react";
import { useUiSettings } from "../app/SettingsProvider";
import { PAGES, type PageKey } from "../pages/registry";

/** §7 信息架构：导航分组（policy）；标签随语言设置翻译。导出供 i18n 去重测试（P1-3）。 */
export const NAV_GROUPS: { titleKey: string; keys: PageKey[] }[] = [
  { titleKey: "nav.group.project", keys: ["overview", "understand"] },
  { titleKey: "nav.group.change", keys: ["changes"] },
  { titleKey: "nav.group.quality", keys: ["health", "verify", "environment"] },
  { titleKey: "nav.group.aiContext", keys: ["context"] },
  { titleKey: "nav.group.history", keys: ["reports"] },
  { titleKey: "nav.group.system", keys: ["settings"] },
];

/** 每个页面的线性图标（统一 24×24 viewBox，避免引入图标库）。 */
const PAGE_ICON: Record<PageKey, ReactNode> = {
  overview: <><rect x="3" y="3" width="7" height="7" rx="1.5" /><rect x="14" y="3" width="7" height="7" rx="1.5" /><rect x="3" y="14" width="7" height="7" rx="1.5" /><rect x="14" y="14" width="7" height="7" rx="1.5" /></>,
  understand: <><circle cx="11" cy="11" r="7" /><path d="m16.5 16.5 4 4" /><path d="M8 11h6" /></>,
  changes: <><path d="M4 7h16M4 12h10M4 17h13" /><path d="m17 4 3 3-3 3" /></>,
  health: <><path d="M3 12h4l2.5-6 4 12 2.5-6h5" /></>,
  verify: <><path d="M4 12.5 9 17l11-11" /><circle cx="12" cy="12" r="9" /></>,
  environment: <><rect x="3" y="4" width="18" height="7" rx="2" /><rect x="3" y="14" width="18" height="6" rx="2" /><path d="M7 7.5h.01M7 17h.01" /></>,
  context: <><path d="M5 4h10l4 4v12H5z" /><path d="M15 4v4h4M8 12h8M8 16h6" /></>,
  reports: <><circle cx="12" cy="12" r="9" /><path d="M12 7v5l3.5 2" /></>,
  settings: <><circle cx="12" cy="12" r="3" /><path d="M12 2.5v2.2M12 19.3v2.2M4.3 4.3l1.6 1.6M18.1 18.1l1.6 1.6M2.5 12h2.2M19.3 12h2.2M4.3 19.7l1.6-1.6M18.1 5.9l1.6-1.6" /></>,
};

interface SidebarProps {
  active: PageKey;
  onSelect: (key: PageKey) => void;
}

/** 固定主导航（纲领 §13 八项），按 §7 IA 分组呈现；标签随语言设置翻译。 */
export default function Sidebar({ active, onSelect }: SidebarProps) {
  const { t } = useUiSettings();

  return (
    <ul className="nav-list">
      {NAV_GROUPS.map((group) => {
        const pages = group.keys
          .map((key) => PAGES.find((page) => page.key === key))
          .filter((page): page is (typeof PAGES)[number] => Boolean(page));
        if (pages.length === 0) return null;
        return (
          <li key={group.titleKey} className="nav-group">
            <div className="nav-group-title" aria-hidden="true">
              {t(group.titleKey)}
            </div>
            <ul className="nav-group-list">
              {pages.map((page) => (
                <li key={page.key}>
                  <button
                    type="button"
                    className="nav-item"
                    aria-current={page.key === active ? "page" : undefined}
                    onClick={() => onSelect(page.key)}
                  >
                    <span className="nav-item-main">
                      <svg
                        className="nav-icon"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        strokeWidth="1.6"
                        strokeLinecap="round"
                        strokeLinejoin="round"
                        aria-hidden="true"
                      >
                        {PAGE_ICON[page.key]}
                      </svg>
                      <span>{t(`page.${page.key}`)}</span>
                    </span>
                    {page.milestoneTag ? (
                      <span className="nav-tag">{page.milestoneTag}</span>
                    ) : null}
                  </button>
                </li>
              ))}
            </ul>
          </li>
        );
      })}
    </ul>
  );
}
