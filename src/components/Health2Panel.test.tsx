import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import Health2Panel from "./Health2Panel";
import type { Health2ReportDto } from "../types/prism";

const ipc = vi.hoisted(() => ({
  startHealth2: vi.fn(),
  getHealth2Status: vi.fn(),
  getHealth2Results: vi.fn(),
  cancelHealth2: vi.fn(),
  getCachedDomain: vi.fn(),
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

const report: Health2ReportDto = {
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
  diagnostics: [],
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
  baseline: { kind: "empty" },
};

describe("Health2Panel (P1-1 GUI health2 rail)", () => {
  beforeEach(() => {
    ipc.startHealth2.mockReset().mockResolvedValue("started");
    ipc.getHealth2Status.mockReset();
    ipc.getHealth2Results.mockReset();
    ipc.cancelHealth2.mockReset().mockResolvedValue(undefined);
    ipc.getCachedDomain.mockReset().mockResolvedValue(null);
  });

  it("auto-starts and renders findings, gate, metrics and lifecycle", async () => {
    ipc.getHealth2Status
      .mockResolvedValueOnce({ status: "idle", error: null })
      .mockResolvedValue({ status: "completed", error: null });
    ipc.getHealth2Results.mockResolvedValue(report);

    render(<Health2Panel />);
    await waitFor(() => {
      expect(ipc.startHealth2).toHaveBeenCalled();
    });
    await waitFor(() => {
      expect(screen.getByTestId("health2-summary")).toBeInTheDocument();
    });

    expect(screen.getByTestId("health2-rule-source")).toHaveTextContent("内置 4 条");
    expect(screen.getByTestId("health2-rule-source")).toHaveTextContent("§71");
    expect(screen.getByTestId("health2-gate")).toHaveTextContent("FAIL");
    expect(screen.getByTestId("health2-gate")).toHaveTextContent("不是健康分");
    expect(screen.getByTestId("health2-metrics")).toHaveTextContent("files_total");
    expect(screen.getByTestId("health2-lifecycle")).toHaveTextContent("New");
    expect(screen.getByTestId("health2-findings")).toHaveTextContent("STRUCT/LARGE_FILE");
    // 空基线 ⇒ New 标注出现在 finding 行。
    expect(screen.getByTestId("health2-findings")).toHaveTextContent("[new]");
    // 无诊断 ⇒ 完整度横幅不渲染。
    expect(screen.queryByTestId("health2-completeness")).not.toBeInTheDocument();
  });

  it("expands finding detail with evidence notes", async () => {
    ipc.getHealth2Status.mockResolvedValue({ status: "completed", error: null });
    ipc.getHealth2Results.mockResolvedValue(report);

    render(<Health2Panel />);
    await waitFor(() => {
      expect(screen.getByTestId("health2-findings")).toBeInTheDocument();
    });
    fireEvent.click(screen.getByText(/STRUCT\/LARGE_FILE/));
    expect(screen.getByText(/1200 lines counted from bounded read/)).toBeInTheDocument();
    expect(screen.getByText(/split the file/)).toBeInTheDocument();
  });

  it("surfaces diagnostics as a completeness banner (P2-5 reuse)", async () => {
    ipc.getHealth2Status.mockResolvedValue({ status: "completed", error: null });
    ipc.getHealth2Results.mockResolvedValue({
      ...report,
      diagnostics: ["skipped line count for huge.cs (5 MiB > cap)"],
    });

    render(<Health2Panel />);
    await waitFor(() => {
      expect(screen.getByTestId("health2-completeness")).toBeInTheDocument();
    });
    expect(screen.getByTestId("health2-completeness")).toHaveTextContent(
      "skipped line count for huge.cs",
    );
  });

  it("surfaces failure honestly", async () => {
    ipc.getHealth2Status.mockResolvedValue({ status: "failed", error: "engine exploded" });
    render(<Health2Panel />);
    await waitFor(() => {
      expect(screen.getByTestId("health2-error")).toHaveTextContent("engine exploded");
    });
  });

  it("labels a persisted previous-run baseline (batch A)", async () => {
    ipc.getHealth2Status.mockResolvedValue({ status: "completed", error: null });
    ipc.getHealth2Results.mockResolvedValue({
      ...report,
      baseline: { kind: "previous_run", captured_at: "2026-10-07T01:00:00Z" },
      lifecycle: {
        ...report.lifecycle,
        persistent_findings: ["find:abc123"],
        new_findings: [],
      },
    });

    render(<Health2Panel />);
    await waitFor(() => {
      expect(screen.getByTestId("health2-rule-source")).toHaveTextContent(
        "上次持久化结果",
      );
    });
    // Persistent 在本轮有了真实语义：跨会话仍存在。
    expect(screen.getByTestId("health2-findings")).toHaveTextContent("[persistent]");
  });

  it("cancels on unmount (§102)", async () => {
    ipc.getHealth2Status.mockResolvedValue({ status: "completed", error: null });
    ipc.getHealth2Results.mockResolvedValue(report);

    const { unmount } = render(<Health2Panel />);
    await waitFor(() => {
      expect(screen.getByTestId("health2-summary")).toBeInTheDocument();
    });
    unmount();
    await waitFor(() => {
      expect(ipc.cancelHealth2).toHaveBeenCalled();
    });
  });
});

describe("Health2Panel cached rail (P2-2)", () => {
  beforeEach(() => {
    ipc.startHealth2.mockReset().mockResolvedValue("started");
    ipc.getHealth2Status.mockReset();
    ipc.getHealth2Results.mockReset();
    ipc.cancelHealth2.mockReset().mockResolvedValue(undefined);
    ipc.getCachedDomain.mockReset().mockResolvedValue(null);
  });

  it("shows cached health2 results with rerun instead of auto-running", async () => {
    ipc.getHealth2Status.mockResolvedValue({ status: "idle", error: null });
    ipc.getCachedDomain.mockResolvedValue({
      domain: "health2",
      completed_at: "2026-10-04T00:00:00Z",
      basis: "abc|0",
      stale: false,
      data: { kind: "health2", report: report },
    });

    render(<Health2Panel />);
    await waitFor(() => {
      expect(screen.getByTestId("health2-cached-banner")).toBeInTheDocument();
    });
    expect(ipc.startHealth2).not.toHaveBeenCalled();
    expect(screen.getByTestId("health2-summary")).toBeInTheDocument();
    expect(screen.getByTestId("health2-cached-banner")).toHaveTextContent("基线未变化");

    fireEvent.click(screen.getByTestId("health2-rerun"));
    await waitFor(() => {
      expect(ipc.startHealth2).toHaveBeenCalled();
    });
    expect(screen.queryByTestId("health2-cached-banner")).not.toBeInTheDocument();
  });
});
