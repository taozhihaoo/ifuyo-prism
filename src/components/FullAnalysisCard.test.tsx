import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import FullAnalysisCard from "./FullAnalysisCard";
import type { FullAnalysisStatusDto } from "../types/prism";

const ipc = vi.hoisted(() => ({
  startFullAnalysis: vi.fn(),
  getFullAnalysisStatus: vi.fn(),
  cancelFullAnalysis: vi.fn(),
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

const idleStatus: FullAnalysisStatusDto = {
  status: "idle",
  stages: [],
  error: null,
};

const runningStatus: FullAnalysisStatusDto = {
  status: "running",
  stages: [
    { key: "scan", status: "completed", duration_ms: 12, error: null },
    { key: "inspect", status: "running", duration_ms: null, error: null },
    { key: "graph", status: "pending", duration_ms: null, error: null },
    { key: "health", status: "pending", duration_ms: null, error: null },
  ],
  error: null,
};

const cancelledStatus: FullAnalysisStatusDto = {
  status: "cancelled",
  stages: [
    { key: "scan", status: "completed", duration_ms: 12, error: null },
    { key: "inspect", status: "completed", duration_ms: 340, error: null },
    { key: "graph", status: "cancelled", duration_ms: 20, error: null },
    { key: "health", status: "skipped", duration_ms: null, error: null },
  ],
  error: null,
};

describe("FullAnalysisCard (P2-1)", () => {
  beforeEach(() => {
    ipc.startFullAnalysis.mockReset().mockResolvedValue("started");
    ipc.getFullAnalysisStatus.mockReset();
    ipc.cancelFullAnalysis.mockReset().mockResolvedValue(undefined);
  });

  it("shows the start button when idle and starts the pipeline on click", async () => {
    ipc.getFullAnalysisStatus
      .mockResolvedValueOnce(idleStatus)
      .mockResolvedValue(runningStatus);

    render(<FullAnalysisCard />);
    await waitFor(() => {
      expect(screen.getByTestId("full-analysis-start")).toBeInTheDocument();
    });
    expect(ipc.startFullAnalysis).not.toHaveBeenCalled();

    fireEvent.click(screen.getByTestId("full-analysis-start"));
    await waitFor(() => {
      expect(ipc.startFullAnalysis).toHaveBeenCalledTimes(1);
    });
    // 逐步进度可见：四阶段 + 各自状态。
    await waitFor(() => {
      expect(screen.getByTestId("full-analysis-stage-scan")).toHaveTextContent("completed");
    });
    expect(screen.getByTestId("full-analysis-stage-inspect")).toHaveTextContent("running");
    expect(screen.getByTestId("full-analysis-stage-graph")).toHaveTextContent("pending");
    expect(screen.getByTestId("full-analysis-stage-health")).toHaveTextContent("pending");
    expect(screen.getByTestId("full-analysis-cancel")).toBeInTheDocument();
  });

  it("keeps completed stages visible after cancellation (acceptance)", async () => {
    ipc.getFullAnalysisStatus.mockResolvedValue(cancelledStatus);

    render(<FullAnalysisCard />);
    await waitFor(() => {
      expect(screen.getByTestId("full-analysis-cancelled")).toHaveTextContent(
        "completed stages above are kept",
      );
    });
    expect(screen.getByTestId("full-analysis-stage-scan")).toHaveTextContent("completed");
    expect(screen.getByTestId("full-analysis-stage-inspect")).toHaveTextContent("completed");
    expect(screen.getByTestId("full-analysis-stage-graph")).toHaveTextContent("cancelled");
    expect(screen.getByTestId("full-analysis-stage-health")).toHaveTextContent("skipped");
  });

  it("surfaces failure honestly", async () => {
    ipc.getFullAnalysisStatus.mockResolvedValue({
      status: "failed",
      stages: [
        { key: "scan", status: "completed", duration_ms: 12, error: null },
        {
          key: "inspect",
          status: "failed",
          duration_ms: 5,
          error: "engine exploded",
        },
        { key: "graph", status: "skipped", duration_ms: null, error: null },
        { key: "health", status: "skipped", duration_ms: null, error: null },
      ],
      error: "stage inspect failed: engine exploded",
    });

    render(<FullAnalysisCard />);
    await waitFor(() => {
      expect(screen.getByTestId("full-analysis-failed")).toHaveTextContent(
        "stage inspect failed",
      );
    });
    expect(screen.getByTestId("full-analysis-stage-inspect")).toHaveTextContent("failed");
  });

  it("surfaces start errors instead of fake success", async () => {
    ipc.getFullAnalysisStatus.mockResolvedValue(idleStatus);
    ipc.startFullAnalysis.mockRejectedValue(new Error("a domain analysis is already running"));

    render(<FullAnalysisCard />);
    await waitFor(() => {
      expect(screen.getByTestId("full-analysis-start")).toBeInTheDocument();
    });
    fireEvent.click(screen.getByTestId("full-analysis-start"));
    await waitFor(() => {
      expect(screen.getByTestId("full-analysis-error")).toHaveTextContent(
        "a domain analysis is already running",
      );
    });
  });
});
