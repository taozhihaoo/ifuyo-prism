import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import SnapshotHistoryPage from "./SnapshotHistoryPage";
import type { SnapshotRecordDto, TimelineDto } from "../types/prism";

const ipc = vi.hoisted(() => ({
  snapshotCapture: vi.fn(),
  snapshotHistory: vi.fn(),
  compareSnapshots: vi.fn(),
  snapshotTimeline: vi.fn(),
  deleteSnapshot: vi.fn(),
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

const record = (id: string, fingerprint: string, note?: string): SnapshotRecordDto =>
  ({
    id,
    captured_at: "2026-10-03T00:00:00Z",
    basis: "working_tree",
    git_revision: "abc123def",
    git_branch: "main",
    working_tree_state: "dirty",
    state_fingerprint: fingerprint,
    counts: { files: 10, findings: 3 },
    note: note ?? null,
  }) as never;

const timeline: TimelineDto = {
  project_id: "proj:demo",
  events: [
    {
      id: "e1",
      timestamp: "2026-10-03T00:00:00Z",
      kind: "git_commit",
      snapshot_id: null,
      git_commit: "abc123",
      title: "Commit abc123",
      summary: "base commit",
      source: "git",
    },
    {
      id: "e2",
      timestamp: "2026-10-03T01:00:00Z",
      kind: "snapshot_captured",
      snapshot_id: "snap:1",
      git_commit: null,
      title: "Prism snapshot",
      summary: "basis=working_tree state=abcd1234 files=10 findings=3",
      source: "prism_snapshot",
    },
  ],
  limitations: [],
};

const listA = record("snap:1", "aaaa");
const listB = record("snap:2", "bbbb", "before refactor");

describe("SnapshotHistoryPage (M11b GUI §127-§133)", () => {
  beforeEach(() => {
    ipc.snapshotCapture.mockReset().mockResolvedValue({});
    ipc.snapshotHistory.mockReset();
    ipc.compareSnapshots.mockReset();
    ipc.snapshotTimeline.mockReset().mockResolvedValue(timeline);
    ipc.deleteSnapshot.mockReset().mockResolvedValue({ deleted: true });
  });

  it("empty project shows first-snapshot guidance (§246)", async () => {
    ipc.snapshotHistory.mockResolvedValue([]);
    ipc.snapshotTimeline.mockResolvedValue({
      project_id: "proj",
      events: [],
      limitations: ["No Prism snapshots yet."],
    });

    render(<SnapshotHistoryPage />);
    await waitFor(() => {
      expect(screen.getByTestId("history-empty")).toBeInTheDocument();
    });
    expect(screen.getByText(/first recorded Prism snapshot/)).toBeInTheDocument();
  });

  it("renders list with basis/revision/state/counts and timeline (§130/§129)", async () => {
    ipc.snapshotHistory.mockResolvedValue([listA, listB]);
    ipc.snapshotTimeline.mockResolvedValue(timeline);

    render(<SnapshotHistoryPage />);
    await waitFor(() => {
      expect(screen.getByTestId("history-list")).toBeInTheDocument();
    });
    expect(screen.getAllByText(/working_tree/).length).toBeGreaterThan(0);
    expect(screen.getAllByText(/abc123de/).length).toBeGreaterThan(0);
    expect(screen.getByTestId("history-timeline")).toHaveTextContent("Commit abc123");
    expect(screen.getByTestId("history-timeline")).toHaveTextContent("Prism snapshot");
  });

  it("compares selected A vs B with per-domain facts (§49/§95)", async () => {
    const diff = {
      snapshot_a: "snap:1",
      snapshot_b: "snap:2",
      compatibility: "compatible",
      structure: {
        domain: "structure",
        state: "changed",
        facts: ["files added: \"c.ts\""],
      },
      code: { domain: "code", state: "unchanged", facts: [] },
      architecture: { domain: "architecture", state: "unchanged", facts: [] },
      git: { domain: "git", state: "unchanged", facts: [] },
      change: { domain: "change", state: "unchanged", facts: [] },
      impact: { domain: "impact", state: "unchanged", facts: [] },
      health: { domain: "health", state: "unchanged", facts: [] },
      verification: { domain: "verification", state: "unchanged", facts: [] },
      environment: { domain: "environment", state: "unchanged", facts: [] },
      context_health: { domain: "context_health", state: "unchanged", facts: [] },
      finding_lifecycles: [
        {
          finding_fingerprint: "fp1",
          rule_id: "CTX005",
          lifecycle: "new",
          evidence: [],
        },
      ],
      verification_transitions: [],
      limitations: [],
    } as never;
    ipc.snapshotHistory.mockResolvedValue([listA, listB]);
    ipc.snapshotTimeline.mockResolvedValue(timeline);
    ipc.compareSnapshots.mockResolvedValue(diff);

    render(<SnapshotHistoryPage />);
    await waitFor(() => {
      expect(screen.getByTestId("history-list")).toBeInTheDocument();
    });
    // A/B 已默认选最新两条；A=snap:1, B=snap:2。
    const compareButton = screen.getByTestId("history-compare");
    expect(compareButton).not.toBeDisabled();
    fireEvent.click(compareButton);

    await waitFor(() => {
      expect(screen.getByTestId("history-compare")).toBeInTheDocument();
    });
    expect(ipc.compareSnapshots).toHaveBeenCalledWith("snap:2", "snap:1");
    expect(screen.getByText(/files added/)).toBeInTheDocument();
    expect(screen.getByText(/CTX005/)).toBeInTheDocument();
  });

  it("capture button creates a new snapshot (§147 manual first)", async () => {
    ipc.snapshotHistory.mockResolvedValue([]);
    ipc.snapshotTimeline.mockResolvedValue({
      project_id: "proj",
      events: [],
      limitations: [],
    });

    render(<SnapshotHistoryPage />);
    await waitFor(() => {
      expect(screen.getByTestId("history-empty")).toBeInTheDocument();
    });
    fireEvent.click(screen.getByRole("button", { name: /create first snapshot/i }));
    await waitFor(() => {
      expect(ipc.snapshotCapture).toHaveBeenCalled();
    });
  });

  it("delete requires the per-row button and refreshes (§62)", async () => {
    ipc.snapshotHistory
      .mockResolvedValueOnce([listA])
      .mockResolvedValue([]);
    ipc.snapshotTimeline.mockResolvedValue({
      project_id: "proj",
      events: [],
      limitations: [],
    });

    render(<SnapshotHistoryPage />);
    await waitFor(() => {
      expect(screen.getByTestId("history-list")).toBeInTheDocument();
    });
    fireEvent.click(screen.getByRole("button", { name: /delete snap:1/ }));
    await waitFor(() => {
      expect(ipc.deleteSnapshot).toHaveBeenCalledWith("snap:1");
    });
    await waitFor(() => {
      expect(screen.getByTestId("history-empty")).toBeInTheDocument();
    });
  });
});
