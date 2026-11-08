import { describe, expect, it } from "vitest";
import { translate } from "./i18n";
import { NAV_GROUPS } from "../components/Sidebar";
import { PAGES } from "../pages/registry";

describe("i18n sidebar labels (P1-3)", () => {
  it("keeps every zh nav entry name unique (groups + pages)", () => {
    const labels = [
      ...NAV_GROUPS.map((group) => translate("zh", group.titleKey)),
      ...PAGES.map((page) => translate("zh", `page.${page.key}`)),
    ];
    const duplicates = labels.filter(
      (label, index) => labels.indexOf(label) !== index,
    );
    expect(duplicates).toEqual([]);
  });

  it("keeps every en nav entry name unique (groups + pages)", () => {
    const labels = [
      ...NAV_GROUPS.map((group) => translate("en", group.titleKey)),
      ...PAGES.map((page) => translate("en", `page.${page.key}`)),
    ];
    const duplicates = labels.filter(
      (label, index) => labels.indexOf(label) !== index,
    );
    expect(duplicates).toEqual([]);
  });

  it("names the reports page distinctly from its group", () => {
    expect(translate("zh", "page.reports")).toBe("快照历史");
    expect(translate("en", "page.reports")).toBe("Snapshots");
  });

  it("localizes the shell and overview labels in both languages", () => {
    expect(translate("en", "overview.project")).toBe("Project");
    expect(translate("zh", "overview.project")).toBe("项目");
    expect(translate("en", "overview.noProject")).toBe("No project opened.");
    expect(translate("zh", "overview.noProject")).toBe("尚未打开项目。");
    expect(translate("en", "completeness.partial")).toBe("partial");
    expect(translate("zh", "completeness.partial")).toBe("部分缺失");
    expect(translate("zh", "fullAnalysis.status.completed")).toBe("完成");
    expect(translate("en", "intelligence.title")).toBe("Intelligence summary (2.0)");
    expect(translate("zh", "intelligence.title")).toBe("Intelligence 摘要（2.0）");
  });
});
