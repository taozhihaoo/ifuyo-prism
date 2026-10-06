import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import HealthPage from "./HealthPage";
import type { HealthAnalysisDto } from "../types/prism";

const ipc = vi.hoisted(() => ({
  startHealth: vi.fn(),
  getHealthStatus: vi.fn(),
  getHealthAnalysis: vi.fn(),
  cancelHealth: vi.fn(),
  startHealth2: vi.fn(),
  getHealth2Status: vi.fn(),
  getHealth2Results: vi.fn(),
  cancelHealth2: vi.fn(),
  getEvolutionReport: vi.fn(),
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

/** 与 CLI `prism health2 --json` 同构的最小 fixture（§71 断言用）。 */
const health2Fixture: import("../types/prism").Health2ReportDto = {
  schema_version: 1,
  metrics: {
    files_total: {
      key: "files_total",
      value: 2,
      unit: "files",
      definition: "number of inventoried source files",
      producer: "STRUCT/LARGE_FILE",
    },
  },
  findings: [
    {
      id: "find:abc123",
      finding_type: "STRUCT/LARGE_FILE",
      severity: "warning",
      status: "open",
      title: "Large source file: src/big.cs (1200 lines > 1000)",
      description: "file spans 1200 lines",
      origin: "fact",
      confidence: "high",
      target: "file:src/big.cs",
      evidence: ["ev:1"],
      remediation: "split the file",
      metadata: {},
    },
  ],
  evidence: [
    {
      id: "ev:1",
      kind: "metric",
      path: "src/big.cs",
      range: null,
      reference: null,
      note: "1200 lines counted from bounded read",
    },
  ],
  diagnostics: ["prerequisite unavailable for supply rules (not run in this rail)"],
  lifecycle: {
    new_findings: ["find:abc123"],
    persistent_findings: [],
    resolved_findings: [],
    regressed_findings: [],
    suppressed_findings: [],
  },
  quality_gate: {
    verdict: "fail",
    new_count: 1,
    breaking_count: 0,
    reasons: ["new findings exceed max_new=0"],
  },
  rule_source: { engine: "prism-health2", builtin_rules: 4 },
};

describe("HealthPage (M7 GUI)", () => {  beforeEach(() => {
    ipc.startHealth.mockReset().mockResolvedValue("started");
    ipc.getHealthStatus.mockReset();
    ipc.getHealthAnalysis.mockReset();
    ipc.cancelHealth.mockReset().mockResolvedValue(undefined);
    ipc.startHealth2.mockReset().mockResolvedValue("started");
    ipc.getHealth2Status.mockReset();
    ipc.getHealth2Results.mockReset();
    ipc.cancelHealth2.mockReset().mockResolvedValue(undefined);
    ipc.getEvolutionReport.mockReset();
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

  it("labels the rule source as the 1.0 builtin set (P2-4a)", async () => {
    ipc.getHealthStatus.mockResolvedValue({ status: "completed", error: null });
    ipc.getHealthAnalysis.mockResolvedValue(analysisFixture);

    render(<HealthPage />);
    await waitFor(() => {
      expect(screen.getByTestId("health-rule-source")).toBeInTheDocument();
    });
    const source = screen.getByTestId("health-rule-source");
    // checked(3) + disabled(0) = 共 3 条，全部检查；指针指向 2.0 CLI。
    expect(source).toHaveTextContent("1.0 内置（共 3 条 · 本次检查 3 条）");
    expect(source).toHaveTextContent("prism health2");

    // 禁用与失败的规则也计入标注。
    ipc.getHealthAnalysis.mockResolvedValue({
      ...analysisFixture,
      checked_rules: ["MAINT001", "SEC001", "DEP001", "CODE001"],
      disabled_rules: ["DOC001"],
      failed_rules: ["CODE001"],
    });
    render(<HealthPage />);
    await waitFor(() => {
      expect(screen.getAllByTestId("health-rule-source")).toHaveLength(2);
    });
    expect(screen.getAllByTestId("health-rule-source")[1]).toHaveTextContent(
      "共 5 条 · 本次检查 4 条 · 失败 1 条 · 禁用 1 条",
    );
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

  it("surfaces the data completeness banner when diagnostics exist (P2-5)", async () => {
    ipc.getHealthStatus.mockResolvedValue({ status: "completed", error: null });
    ipc.getHealthAnalysis.mockResolvedValue(analysisFixture);

    render(<HealthPage />);
    await waitFor(() => {
      expect(screen.getByTestId("health-completeness")).toBeInTheDocument();
    });
    const banner = screen.getByTestId("health-completeness");
    expect(banner).toHaveTextContent("数据完整度");
    expect(banner).toHaveTextContent("unresolved_relationship");
  });

  it("labels partial completeness and hides the banner when nothing is missing (P2-5)", async () => {
    ipc.getHealthStatus.mockResolvedValue({ status: "completed", error: null });
    // complete 且无诊断 → 横幅不渲染。
    ipc.getHealthAnalysis.mockResolvedValue({
      ...analysisFixture,
      diagnostics: [],
    });
    const { unmount } = render(<HealthPage />);
    await waitFor(() => {
      expect(screen.getByTestId("health-summary")).toBeInTheDocument();
    });
    expect(screen.queryByTestId("health-completeness")).not.toBeInTheDocument();
    unmount();

    // partial → 横幅标注「部分缺失」。
    ipc.getHealthAnalysis.mockResolvedValue({
      ...analysisFixture,
      analysis_completeness: "partial",
      diagnostics: [],
    });
    render(<HealthPage />);
    await waitFor(() => {
      expect(screen.getByTestId("health-completeness")).toHaveTextContent(
        "部分缺失",
      );
    });
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

  it("switches to the health2 rail and runs the same pipeline as the CLI (P1-1)", async () => {
    ipc.getHealthStatus.mockResolvedValue({ status: "completed", error: null });
    ipc.getHealthAnalysis.mockResolvedValue(analysisFixture);
    ipc.getHealth2Status
      .mockResolvedValueOnce({ status: "idle", error: null })
      .mockResolvedValue({ status: "completed", error: null });
    ipc.getHealth2Results.mockResolvedValue(health2Fixture);

    render(<HealthPage />);
    await waitFor(() => {
      expect(screen.getByTestId("health-rails")).toBeInTheDocument();
    });

    fireEvent.click(screen.getByTestId("health-rail-health2"));
    await waitFor(() => {
      expect(ipc.startHealth2).toHaveBeenCalled();
    });
    await waitFor(() => {
      expect(screen.getByTestId("health2-summary")).toBeInTheDocument();
    });
    // §71 标注：与 CLI 同一管线。
    expect(screen.getByTestId("health2-rule-source")).toHaveTextContent("prism health2");
    expect(screen.getByTestId("health2-rule-source")).toHaveTextContent("内置 4 条");
    // Gate 是策略判断不是评分。
    expect(screen.getByTestId("health2-gate")).toHaveTextContent("FAIL");
    // Findings 与 CLI --json 同构。
    expect(screen.getByTestId("health2-findings")).toHaveTextContent("STRUCT/LARGE_FILE");

    // 切回 1.0 轨道：health2 面板卸载（其取消由组件卸载钩子触发）。
    fireEvent.click(screen.getByTestId("health-rail-classic"));
    await waitFor(() => {
      expect(ipc.cancelHealth2).toHaveBeenCalled();
    });
    expect(screen.getByTestId("health-summary")).toBeInTheDocument();
  });

  it("shows the dual-track note on the health page (P2-4b)", () => {
    ipc.getHealthStatus.mockResolvedValue({ status: "idle", error: null });
    render(<HealthPage />);
    expect(screen.getByTestId("health-dual-track-note")).toHaveTextContent("双轨说明");
  });
});
