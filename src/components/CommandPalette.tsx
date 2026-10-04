import { useEffect, useMemo, useRef, useState } from "react";
import type { PageKey } from "../pages/registry";
import { PAGES } from "../pages/registry";
import { useUiSettings } from "../app/SettingsProvider";
import { translate } from "../app/i18n";

interface RecentProject {
  display_name: string;
  canonical_root: string;
}

interface CommandPaletteProps {
  active: PageKey;
  recentProjects: RecentProject[];
  onNavigate: (key: PageKey) => void;
  onOpenProject: (path: string) => void;
  onClose: () => void;
}

/**
 * Command Palette（M12 §34：Ctrl+K；页面导航 + 最近项目打开）。
 * 自实现浮层——不引入框架（§0.1）。
 */
export default function CommandPalette({
  active,
  recentProjects,
  onNavigate,
  onOpenProject,
  onClose,
}: CommandPaletteProps) {
  const { settings, t } = useUiSettings();
  const [query, setQuery] = useState("");
  const [activeIndex, setActiveIndex] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  const pageItems = useMemo(
    () =>
      PAGES.map((page) => ({
        kind: "page" as const,
        id: page.key,
        label: translate(settings.language, `page.${page.key}`),
        hint: page.key === active ? "current page" : "go to page",
      })),
    [active],
  );
  const projectItems = useMemo(
    () =>
      recentProjects.slice(0, 6).map((project) => ({
        kind: "project" as const,
        id: project.canonical_root,
        label: `${t("palette.openProject")}: ${project.display_name}`,
        hint: project.canonical_root,
      })),
    [recentProjects],
  );

  const query_lower = query.toLowerCase();
  const filtered = [...pageItems, ...projectItems].filter((item) =>
    item.label.toLowerCase().includes(query_lower),
  );

  const commit = (index: number) => {
    const item = filtered[index];
    if (!item) return;
    if (item.kind === "page") {
      onNavigate(item.id as PageKey);
    } else {
      onOpenProject(item.id);
    }
    onClose();
  };

  const on_key = (event: React.KeyboardEvent) => {
    if (event.key === "Escape") {
      onClose();
    } else if (event.key === "ArrowDown") {
      event.preventDefault();
      setActiveIndex((index) => Math.min(index + 1, filtered.length - 1));
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      setActiveIndex((index) => Math.max(index - 1, 0));
    } else if (event.key === "Enter") {
      event.preventDefault();
      commit(activeIndex);
    }
  };

  return (
    <div className="palette-overlay" data-testid="command-palette" onClick={onClose}>
      <div className="palette" role="dialog" aria-label="Command palette" onClick={(e) => e.stopPropagation()}>
        <input
          ref={inputRef}
          className="palette-input"
          placeholder={t("palette.placeholder")}
          value={query}
          onChange={(event) => {
            setQuery(event.target.value);
            setActiveIndex(0);
          }}
          onKeyDown={on_key}
        />
        <ul className="palette-list">
          {filtered.map((item, index) => (
            <li key={`${item.kind}:${item.id}`}>
              <button
                type="button"
                className="palette-item"
                data-active={index === activeIndex}
                onClick={() => commit(index)}
              >
                <span>{item.label}</span>
                <span className="palette-hint">{item.hint}</span>
              </button>
            </li>
          ))}
          {filtered.length === 0 && <li className="palette-item">{t("palette.noMatches")}</li>}
        </ul>
      </div>
    </div>
  );
}
