import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi, beforeEach } from "vitest";
import VizGraphPanel from "./VizGraphPanel";
import type { VizGraph } from "../types/viz";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

import { invoke } from "@tauri-apps/api/core";
const mockInvoke = vi.mocked(invoke);

function godotGraph(): VizGraph {
  return {
    schema_version: 1,
    view: "godot",
    status: "complete",
    nodes: [
      {
        id: "viz:project:godot-project",
        entity_id: "godot-project",
        kind: "project",
        label: "Godot Project",
        parent: null,
        status: "complete",
        metadata: {},
      },
      {
        id: "viz:scene:Scenes/Battle.tscn",
        entity_id: "Scenes/Battle.tscn",
        kind: "scene",
        label: "Scenes/Battle.tscn",
        parent: "viz:project:godot-project",
        status: "complete",
        metadata: {},
      },
      {
        id: "viz:script:Scripts/Battle.cs",
        entity_id: "Scripts/Battle.cs",
        kind: "script",
        label: "Scripts/Battle.cs",
        parent: "viz:scene:Scenes/Battle.tscn",
        status: "complete",
        metadata: {},
      },
    ],
    edges: [
      {
        id: "edge:contains:x#y",
        relation: "contains",
        source: "viz:scene:Scenes/Battle.tscn",
        target: "viz:script:Scripts/Battle.cs",
        status: "complete",
        metadata: {},
      },
    ],
    legend: [{ key: "scene", description: "Godot .tscn scene file" }],
    truncated: false,
    diagnostics: [],
  };
}

beforeEach(() => {
  mockInvoke.mockReset();
});

describe("VizGraphPanel (M212)", () => {
  it("renders godot projection nodes and legend from real projection data", async () => {
    mockInvoke.mockResolvedValue(godotGraph());
    render(<VizGraphPanel />);
    await waitFor(() => {
      expect(screen.getByTestId("viz-node-Scenes/Battle.tscn")).toBeTruthy();
    });
    expect(screen.getByTestId("viz-node-Scripts/Battle.cs")).toBeTruthy();
    expect(screen.getByTestId("viz-legend").textContent).toContain("scene");
    expect(mockInvoke).toHaveBeenCalledWith(
      "get_visualization_graph",
      expect.objectContaining({ view: "godot" }),
    );
  });

  it("selection opens inspector with entity + relations (§36 Evidence-first)", async () => {
    mockInvoke.mockResolvedValue(godotGraph());
    render(<VizGraphPanel />);
    await waitFor(() => {
      expect(screen.getByTestId("viz-node-Scripts/Battle.cs")).toBeTruthy();
    });
    fireEvent.click(screen.getByTestId("viz-node-Scripts/Battle.cs"));
    const inspector = screen.getByTestId("viz-inspector");
    expect(inspector.textContent).toContain("Scripts/Battle.cs");
    expect(inspector.textContent).toContain("relations: 1");
  });

  it("shows partial view status banner when projection is partial (§46)", async () => {
    const partial = { ...godotGraph(), status: "partial" as const };
    mockInvoke.mockResolvedValue(partial);
    render(<VizGraphPanel />);
    await waitFor(() => {
      expect(screen.getByTestId("viz-status").textContent).toContain("partial");
    });
  });

  it("shows truncation banner when limits hit (§28)", async () => {
    const truncated = { ...godotGraph(), truncated: true };
    mockInvoke.mockResolvedValue(truncated);
    render(<VizGraphPanel />);
    await waitFor(() => {
      expect(screen.getByTestId("viz-truncated")).toBeTruthy();
    });
  });

  it("shows unavailable state on IPC failure (§66, not blank)", async () => {
    mockInvoke.mockRejectedValue({ code: "internal", message: "boom" });
    render(<VizGraphPanel />);
    await waitFor(() => {
      expect(screen.getByTestId("viz-error").textContent).toContain("unavailable");
    });
  });

  it("switches view refetches projection (cross-view §35)", async () => {
    mockInvoke.mockResolvedValue(godotGraph());
    render(<VizGraphPanel />);
    await waitFor(() => {
      expect(screen.getByTestId("viz-tree")).toBeTruthy();
    });
    fireEvent.click(screen.getByRole("tab", { name: "Architecture" }));
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith(
        "get_visualization_graph",
        expect.objectContaining({ view: "architecture" }),
      );
    });
  });
});
