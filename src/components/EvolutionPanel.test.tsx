import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import EvolutionPanel from "./EvolutionPanel";
import type { EvolutionReportDto } from "../types/prism";

const ipc = vi.hoisted(() => ({
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

const report: EvolutionReportDto = {
  available: true,
  unavailable_reason: null,
  window: "recent:200-commits",
  commits_scanned: 42,
  hotspots: [
    {
      path: "src/battle.gd",
      commit_count: 12,
      change_count: 12,
      churn: 340,
      lines_added: 220,
      lines_deleted: 120,
      author_count: 2,
      last_committed_at: "2026-10-01T00:00:00Z",
    },
  ],
  couplings: [
    {
      path_a: "src/a.gd",
      path_b: "src/a_test.gd",
      co_change_count: 5,
      evidence_commits: ["abc1234", "def5678"],
    },
  ],
};

describe("EvolutionPanel (P1-1 M211 GUI)", () => {
  beforeEach(() => {
    ipc.getEvolutionReport.mockReset();
  });

  it("runs on explicit request (Manual First §147) and renders tables", async () => {
    ipc.getEvolutionReport.mockResolvedValue(report);

    render(<EvolutionPanel />);
    expect(screen.getByTestId("evolution-run")).toBeInTheDocument();
    expect(ipc.getEvolutionReport).not.toHaveBeenCalled();

    fireEvent.click(screen.getByTestId("evolution-run"));
    await waitFor(() => {
      expect(screen.getByTestId("evolution-hotspots")).toBeInTheDocument();
    });
    expect(ipc.getEvolutionReport).toHaveBeenCalledTimes(1);
    expect(screen.getByTestId("evolution-window")).toHaveTextContent(
      "recent:200-commits",
    );
    expect(screen.getByTestId("evolution-hotspots")).toHaveTextContent("src/battle.gd");
    expect(screen.getByTestId("evolution-hotspots")).toHaveTextContent("340");
    expect(screen.getByTestId("evolution-couplings")).toHaveTextContent(
      "src/a.gd ↔ src/a_test.gd",
    );
    // §26/§27 边界声明。
    expect(screen.getByText(/≠ 语义依赖/)).toBeInTheDocument();
  });

  it("shows unavailability honestly for non-git projects (§46)", async () => {
    ipc.getEvolutionReport.mockResolvedValue({
      available: false,
      unavailable_reason: "not a git repository",
      window: "",
      commits_scanned: 0,
      hotspots: [],
      couplings: [],
    });

    render(<EvolutionPanel />);
    fireEvent.click(screen.getByTestId("evolution-run"));
    await waitFor(() => {
      expect(screen.getByTestId("evolution-unavailable")).toHaveTextContent(
        "not a git repository",
      );
    });
  });

  it("surfaces IPC errors instead of fake-empty tables", async () => {
    ipc.getEvolutionReport.mockRejectedValue(new Error("git exploded"));

    render(<EvolutionPanel />);
    fireEvent.click(screen.getByTestId("evolution-run"));
    await waitFor(() => {
      expect(screen.getByTestId("evolution-error")).toHaveTextContent("git exploded");
    });
  });
});
