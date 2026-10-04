import { useUiSettings } from "../app/SettingsProvider";
import { PAGES, type PageKey } from "../pages/registry";

/** §7 信息架构：导航分组（policy）；标签随语言设置翻译。 */
const NAV_GROUPS: { titleKey: string; keys: PageKey[] }[] = [
  { titleKey: "nav.group.project", keys: ["overview", "understand"] },
  { titleKey: "nav.group.change", keys: ["changes"] },
  { titleKey: "nav.group.quality", keys: ["health", "verify", "environment"] },
  { titleKey: "nav.group.aiContext", keys: ["context"] },
  { titleKey: "nav.group.history", keys: ["reports"] },
  { titleKey: "nav.group.system", keys: ["settings"] },
];

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
          <li key={group.titleKey}>
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
                    {t(`page.${page.key}`)}
                    {page.milestoneTag ? (
                      <span className="nav-tag">{page.milestoneTag}</span>
                    ) : (
                      <span className="nav-tag nav-tag-live">live</span>
                    )}
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
