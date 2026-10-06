import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import IntelligenceSummaryCard from "./IntelligenceSummaryCard";
import type { IntelligenceSummaryDto } from "../types/prism";

const ipc = vi.hoisted(() => ({
  getIntelligenceSummary: vi.fn(),
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

describe("IntelligenceSummaryCard (P1-2)", () => {
  beforeEach(() => {
    ipc.getIntelligenceSummary.mockReset();
  });

  it("shows counts per domain (§71 source honesty)", async () => {
    ipc.getIntelligenceSummary.mockResolvedValue({
      health2: { critical: 0, warning: 2, info: 1 },
      health: 5,
      godot: { scenes: 3, resources: 1, nodes: 18 },
      deps: { declarations: 9, resolved: 8, cycles: 1 },
    } satisfies IntelligenceSummaryDto);

    render(<IntelligenceSummaryCard />);
    await waitFor(() => {
      expect(screen.getByTestId("intelligence-summary")).toBeInTheDocument();
    });
    expect(screen.getByTestId("intelligence-health2")).toHaveTextContent(
      "2 warning",
    );
    expect(screen.getByTestId("intelligence-godot")).toHaveTextContent("3 scenes");
    expect(screen.getByTestId("intelligence-deps")).toHaveTextContent("1 cycles");
    // Unknown 不应出现（所有域都有事实）。
    expect(screen.queryByText("Unknown")).not.toBeInTheDocument();
  });

  it("renders Unknown for domains without session facts (§46 not fake-normal)", async () => {
    ipc.getIntelligenceSummary.mockResolvedValue({
      health2: null,
      health: null,
      godot: null,
      deps: null,
    });

    render(<IntelligenceSummaryCard />);
    await waitFor(() => {
      expect(screen.getByText("Intelligence 摘要（2.0）")).toBeInTheDocument();
    });
    expect(screen.getAllByText("Unknown")).toHaveLength(4);
    expect(screen.getByText(/Manual First §147/)).toBeInTheDocument();
  });

  it("refresh re-queries the summary", async () => {
    ipc.getIntelligenceSummary.mockResolvedValue({
      health2: { critical: 1, warning: 0, info: 0 },
      health: null,
      godot: null,
      deps: null,
    });

    render(<IntelligenceSummaryCard />);
    await waitFor(() => {
      expect(screen.getByTestId("intelligence-health2")).toBeInTheDocument();
    });
    fireEvent.click(screen.getByTestId("intelligence-refresh"));
    await waitFor(() => {
      expect(ipc.getIntelligenceSummary).toHaveBeenCalledTimes(2);
    });
  });

  it("surfaces projection errors honestly", async () => {
    ipc.getIntelligenceSummary.mockRejectedValue(new Error("projection exploded"));

    render(<IntelligenceSummaryCard />);
    await waitFor(() => {
      expect(screen.getByText(/projection exploded/)).toBeInTheDocument();
    });
  });
});
