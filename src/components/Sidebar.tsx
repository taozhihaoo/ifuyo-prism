import { PAGES, type PageKey } from "../pages/registry";

/** §7 信息架构：导航分组（policy，非硬编码功能）。 */
const NAV_GROUPS: { title: string; keys: PageKey[] }[] = [
  { title: "Project", keys: ["overview", "understand"] },
  { title: "Change", keys: ["changes"] },
  { title: "Quality", keys: ["health", "verify", "environment"] },
  { title: "AI Context", keys: ["context"] },
  { title: "History", keys: ["reports"] },
];

interface SidebarProps {
  active: PageKey;
  onSelect: (key: PageKey) => void;
}

/** 固定主导航（纲领 §13 八项），按 §7 IA 分组呈现。 */
export default function Sidebar({ active, onSelect }: SidebarProps) {
  return (
    <ul className="nav-list">
      {NAV_GROUPS.map((group) => {
        const pages = group.keys
          .map((key) => PAGES.find((page) => page.key === key))
          .filter((page): page is (typeof PAGES)[number] => Boolean(page));
        if (pages.length === 0) return null;
        return (
          <li key={group.title}>
            <div className="nav-group-title" aria-hidden="true">
              {group.title}
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
                    {page.label}
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
