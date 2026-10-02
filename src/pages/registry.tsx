import type { ReactNode } from "react";
import type { AppInfo } from "../types/prism";
import OverviewPage from "./OverviewPage";
import PlaceholderPage from "../components/PlaceholderPage";

/** 主导航页面键（纲领 §13 固定八个，不随功能膨胀）。 */
export type PageKey =
  | "overview"
  | "understand"
  | "changes"
  | "health"
  | "verify"
  | "environment"
  | "context"
  | "reports";

export interface PageDef {
  key: PageKey;
  label: string;
  /** 占位页显示的里程碑标记；undefined 表示 M0 已提供真实内容。 */
  milestoneTag?: string;
  render: (appInfo: AppInfo | null) => ReactNode;
}

export const PAGES: PageDef[] = [
  {
    key: "overview",
    label: "Overview",
    render: (appInfo) => <OverviewPage appInfo={appInfo} />,
  },
  {
    key: "understand",
    label: "Understand",
    milestoneTag: "M1",
    render: () => (
      <PlaceholderPage
        milestone="M1"
        description="Project discovery and code understanding: structure, files, languages, entry points, symbols."
      />
    ),
  },
  {
    key: "changes",
    label: "Changes",
    milestoneTag: "M4",
    render: () => (
      <PlaceholderPage
        milestone="M4"
        description="Git facts, ChangeSet and change impact analysis: what changed, what it may affect."
      />
    ),
  },
  {
    key: "health",
    label: "Health",
    milestoneTag: "M7",
    render: () => (
      <PlaceholderPage
        milestone="M7"
        description="Evidence-based findings: architecture, dependency, secret, documentation and maintenance risks."
      />
    ),
  },
  {
    key: "verify",
    label: "Verify",
    milestoneTag: "M8",
    render: () => (
      <PlaceholderPage
        milestone="M8"
        description="Build, test, lint and type-check orchestration with unified, evidence-preserving results."
      />
    ),
  },
  {
    key: "environment",
    label: "Environment",
    milestoneTag: "M9",
    render: () => (
      <PlaceholderPage
        milestone="M9"
        description="Runtime, SDK and toolchain detection: whether this machine fits this project."
      />
    ),
  },
  {
    key: "context",
    label: "Context",
    milestoneTag: "M10",
    render: () => (
      <PlaceholderPage
        milestone="M10"
        description="Risk-weighted, task-aware context packs for AI — deterministic, redacted, budgeted."
      />
    ),
  },
  {
    key: "reports",
    label: "Reports",
    milestoneTag: "M7",
    render: () => (
      <PlaceholderPage
        milestone="M7"
        description="Saved JSON / Markdown / HTML reports for health, changes, verification and context."
      />
    ),
  },
];
