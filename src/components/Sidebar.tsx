import { PAGES, type PageKey } from "../pages/registry";

interface SidebarProps {
  active: PageKey;
  onSelect: (key: PageKey) => void;
}

/** 固定主导航（纲领 §13）：页面集合固定，不随功能无限增加。 */
export default function Sidebar({ active, onSelect }: SidebarProps) {
  return (
    <ul className="nav-list">
      {PAGES.map((page) => (
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
  );
}
