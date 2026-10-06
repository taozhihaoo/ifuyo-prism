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
});
