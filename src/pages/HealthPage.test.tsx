import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import HealthPage from "./HealthPage";
import type { HealthAnalysisDto } from "../types/prism";

const ipc = vi.hoisted(() => ({
  startHealth: vi.fn(),
  getHealthStatus: vi.fn(),
  getHealthAnalysis: vi.fn(),
  cancelHealth: vi.fn(),
  toPrismError: (err: unknown) => ({ code: "internal", message: String(err) }),
}));

vi.mock("../services/ipc", () => ipc);

const projectFixture = {
  id: "p1",
  name: "demo",
  root_path: "C:/tmp/demo",
  discovered_at: "2026-10-04T00:00:00Z",
};

vi.mock("../app/ProjectStore", () => ({
  useProjectStore: () => ({ project: projectFixture }),
}));

const finding = (
  rule: string,
  severity: "info" | "warning" | "critical",
  category: string,
  path: string,
  relation: "directly_changed" | "impacted" | "unrelated" = "unrelated",
) => ({
  id: `finding:${rule}:${path}:${line_of(path)}`,
  rule_id: rule,
  category,
  severity,
  confidence: "high",
  title: `Detected issue (${rule})`,
  message: `${rule} at ${path}`,
  explanation: {
    what: `A ${rule} finding was produced.`,
    evidence_summary: `${path} evidence`,
    why_it_matters: null,
  },
  location: { relative_path: path, line: line_of(path), column: 1 },
  related_locations: [],
  related_symbols: [],
  related_modules: [],
  related_files: [path],
  evidence: [{ kind: "source_location", subject: path, detail: "test evidence" }],
  suggested_action: null,
  status: "active",
  change_relation: relation,
});

function line_of(path: string): number {
  return path.includes("maint") ? 3 : 7;
}

const analysisFixture: HealthAnalysisDto = {
  schema_version: 1,
  project_name: "demo",
  scope: "full_project",
  change_related_findings: 2,
  findings: [
    finding("MAINT001", "info", "maintenance", "src/maint.ts"),
    finding("SEC001", "warning", "security", "src/secret.ts", "directly_changed"),
    finding("DEP001", "warning", "dependency", "src/dep.ts", "impacted"),
  ],
  impact_paths: [],
  summary: {
    total_findings: 3,
    critical_count: 0,
    warning_count: 2,
    info_count: 1,
    files_added: 0,
    files_modified: 0,
    files_deleted: 0,
    files_renamed: 0,
    files_copied: 0,
    files_type_changed: 0,
    files_other: 0,
    symbols_total: 0,
    symbols_added: 0,
    symbols_deleted: 0,
    symbols_modified: 0,
    symbols_renamed: 0,
    symbols_moved: 0,
    modules_changed: 0,
    direct_impacted_modules: 0,
    direct_impacted_symbols: 0,
    transitive_impacted_modules: 0,
    transitive_impacted_symbols: 0,
    potentially_affected_tests: 0,
    documentation_candidates: 0,
    unresolved_relationships: 0,
    ambiguous_relationships: 0,
    max_distance: 0,
    diagnostics_total: 1,
    by_category: { maintenance: 1, security: 1, dependency: 1 },
    by_rule: { MAINT001: 1, SEC001: 1, DEP001: 1 },
    files_with_findings: 3,
    modules_with_findings: 3,
    symbols_with_findings: 0,
  } as never,
  diagnostics: [
    {
      kind: "unresolved_relationship",
      subject: null,
      detail: "2 unresolved relationship(s) adjacent to changed entities",
    },
  ],
  analysis_completeness: "complete",
  analysis_incomplete: false,
  changed_items: [],
  impacted_items: [],
  checked_rules: ["MAINT001", "SEC001", "DEP001"],
  disabled_rules: [],
  failed_rules: [],
  risk_signals: [],
} as never;

describe("HealthPage (M7 GUI)", () => {
  beforeEach(() => {
    ipc.startHealth.mockReset().mockResolvedValue("started");
    ipc.getHealthStatus.mockReset();
    ipc.getHealthAnalysis.mockReset();
    ipc.cancelHealth.mockReset().mockResolvedValue(undefined);
  });

  it("renders summary, category groups and findings list", async () => {
    ipc.getHealthStatus.mockResolvedValue({ status: "completed", error: null });
    ipc.getHealthAnalysis.mockResolvedValue(analysisFixture);

    render(<HealthPage />);

    await waitFor(() => {
      expect(screen.getByTestId("health-summary")).toBeInTheDocument();
    });
    // §81 severity counts（Total/Warning/Info 用唯一锚点断言）。
    expect(screen.getByText("Total")).toBeInTheDocument();
    expect(screen.getByText("Critical")).toBeInTheDocument();
    expect(screen.getByText("Warning")).toBeInTheDocument();
    expect(screen.getByText("Info")).toBeInTheDocument();
    // §136 category groups（chips 也含分类名，用 groups 容器内断言）。
    const groups = screen.getByTestId("health-groups");
    expect(within(groups).getByText("Architecture")).toBeInTheDocument();
    expect(within(groups).getByText("Maintenance")).toBeInTheDocument();
    // finding 行。
    expect(screen.getByText(/MAINT001/)).toBeInTheDocument();
    expect(screen.getByText(/SEC001/)).toBeInTheDocument();
  });

  it("severity filter narrows the findings list", async () => {
    ipc.getHealthStatus.mockResolvedValue({ status: "completed", error: null });
    ipc.getHealthAnalysis.mockResolvedValue(analysisFixture);

    render(<HealthPage />);
    await waitFor(() => {
      expect(screen.getByTestId("health-findings")).toBeInTheDocument();
    });
    fireEvent.click(screen.getByRole("button", { name: "warning" }));
    expect(screen.getByText(/SEC001/)).toBeInTheDocument();
    expect(screen.queryByText(/MAINT001/)).not.toBeInTheDocument();
  });

  it("current-changes chip filters by change relation", async () => {
    ipc.getHealthStatus.mockResolvedValue({ status: "completed", error: null });
    ipc.getHealthAnalysis.mockResolvedValue(analysisFixture);

    render(<HealthPage />);
    await waitFor(() => {
      expect(screen.getByTestId("health-findings")).toBeInTheDocument();
    });
    fireEvent.click(screen.getByRole("button", { name: /current changes/i }));
    expect(screen.getByText(/SEC001/)).toBeInTheDocument(); // directly_changed
    expect(screen.getByText(/DEP001/)).toBeInTheDocument(); // impacted
    expect(screen.queryByText(/MAINT001/)).not.toBeInTheDocument(); // unrelated
  });

  it("expands finding detail with what/why/evidence (§83)", async () => {
    ipc.getHealthStatus.mockResolvedValue({ status: "completed", error: null });
    ipc.getHealthAnalysis.mockResolvedValue(analysisFixture);

    render(<HealthPage />);
    await waitFor(() => {
      expect(screen.getByTestId("health-findings")).toBeInTheDocument();
    });
    fireEvent.click(screen.getByText(/MAINT001/));
    expect(screen.getByText(/What:/)).toBeInTheDocument();
    expect(screen.getByText(/test evidence/)).toBeInTheDocument();
  });

  it("shows analysis limitations card (§131)", async () => {
    ipc.getHealthStatus.mockResolvedValue({ status: "completed", error: null });
    ipc.getHealthAnalysis.mockResolvedValue(analysisFixture);

    render(<HealthPage />);
    await waitFor(() => {
      expect(screen.getByTestId("health-limitations")).toBeInTheDocument();
    });
    expect(screen.getByText(/unresolved_relationship/)).toBeInTheDocument();
    expect(
      screen.getByText(/based on resolvable static relationships/i)
    ).toBeInTheDocument();
  });

  it("cancels on unmount (§102)", async () => {
    ipc.getHealthStatus.mockResolvedValue({ status: "completed", error: null });
    ipc.getHealthAnalysis.mockResolvedValue(analysisFixture);

    const { unmount } = render(<HealthPage />);
    await waitFor(() => {
      expect(screen.getByTestId("health-summary")).toBeInTheDocument();
    });
    unmount();
    await waitFor(() => {
      expect(ipc.cancelHealth).toHaveBeenCalled();
    });
  });

  it("surfaces error state honestly", async () => {
    ipc.getHealthStatus.mockResolvedValue({
      status: "failed",
      error: "graph exploded",
    });
    render(<HealthPage />);
    await waitFor(() => {
      expect(screen.getByTestId("health-error")).toHaveTextContent("graph exploded");
    });
  });
});
