import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import ContextPage from "./ContextPage";
import type { ContextPackDto } from "../types/prism";

const ipc = vi.hoisted(() => ({
  startContextGeneration: vi.fn(),
  getContextStatus: vi.fn(),
  getContextResults: vi.fn(),
  cancelContextGeneration: vi.fn(),
  exportContext: vi.fn(),
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

const pack: ContextPackDto = {
  manifest: {
    schema_version: 1,
    id: "pack-1",
    project_root: "C:/tmp/demo",
    task: "重构 BattleManager",
    classification: {
      kinds: ["refactor"],
      focus: ["code"],
      specificity: "specific",
      raw_task: "重构 BattleManager",
    },
    target: {
      raw: "BattleManager",
      resolution: { kind: "resolved", paths: ["src/BattleManager.ts"] },
    },
    scope: "auto",
    generated_at: "2026-10-03T23:00:00Z",
    status: "complete",
    summary: { files: 1, symbols: 1, docs: 0, findings: 0, checks: 0, items: 3, ranges: 1 },
    budget: {
      selected_items: 3,
      excluded_items: 2,
      used_bytes: 2048,
      used_tokens_estimate: 512,
      limit_tokens_estimate: 24576,
    },
    limitations: [],
  },
  state: {
    head_commit: null,
    branch: null,
    dirty: true,
    generated_at: "2026-10-03T23:00:00Z",
  },
  risk_signals: ["verification_stale"],
  items: [
    {
      id: "ctx:file|file:src/BattleManager.ts",
      source: {
        source_type: "file",
        source_id: "file:src/BattleManager.ts",
        path: "src/BattleManager.ts",
        location: null,
        provenance: "task target",
      },
      group: "target_code",
      title: "Target file: src/BattleManager.ts",
      content: "class BattleManager {\n  resolve(i) { return i; }\n}",
      location: null,
      priority: "critical",
      reasons: ["explicit_task_target"],
      evidence: ["explicit task target"],
      trust_level: "prism_fact",
      language: null,
      change_state: null,
      impact_distance: null,
      truncated: false,
      original_size: null,
      included_size: 55,
      truncation_reason: null,
      content_level: "full",
      detail_level: "full",
      ranges: [],
      unavailable_reason: null,
      content_hash: "abc123",
    },
  ],
  selection: {
    candidates: 5,
    selected: ["ctx:file|file:src/BattleManager.ts"],
    excluded: [
      {
        id: "x1",
        title: "unrelated.md",
        reason: "outside_scope",
        detail: "outside auto scope",
      },
    ],
  },
  health: {
    status: "attention",
    findings: [
      {
        rule_id: "CTX005",
        severity: "Warning",
        title: "Verification result stale",
        message: "Verification facts captured on a different workspace state.",
        evidence: [],
      },
    ],
    coverage: {
      target: "covered",
      impact: "available",
      verification: "available",
      docs: "partial",
      environment: "unavailable",
    },
    limitations: [],
  },
  diagnostics: [],
};

describe("ContextPage (M10b GUI)", () => {
  beforeEach(() => {
    ipc.startContextGeneration.mockReset().mockResolvedValue("started");
    ipc.getContextStatus.mockReset();
    ipc.getContextResults.mockReset();
    ipc.cancelContextGeneration.mockReset().mockResolvedValue(undefined);
    ipc.exportContext.mockReset();
  });

  it("offers explicit generate when idle (§293/§298 no auto-run)", async () => {
    ipc.getContextStatus.mockResolvedValue({ status: "idle", error: null });
    render(<ContextPage />);
    await waitFor(() => {
      expect(screen.getByTestId("context-idle")).toBeInTheDocument();
    });
    expect(ipc.startContextGeneration).not.toHaveBeenCalled();
  });

  it("renders summary, health, budget, excluded and grouped items (§295-§301)", async () => {
    ipc.getContextStatus.mockResolvedValue({ status: "completed", error: null });
    ipc.getContextResults.mockResolvedValue(pack);

    render(<ContextPage />);
    await waitFor(() => {
      expect(screen.getByTestId("context-summary")).toBeInTheDocument();
    });
    // §295 generated summary。
    expect(screen.getByTestId("context-generated-summary")).toHaveTextContent(
      "3 items",
    );
    // §300 health + §274 边界声明。
    expect(screen.getByTestId("context-health")).toHaveTextContent("CTX005");
    expect(screen.getByTestId("context-health")).toHaveTextContent(
      "Context Health ≠ project health",
    );
    // §301 budget（非 score）。
    expect(screen.getByTestId("context-budget")).toHaveTextContent("512 /");
    // §299 excluded 可见。
    expect(screen.getByTestId("context-excluded")).toHaveTextContent(
      "outside_scope",
    );
    // 分组 + item + why-included（§298）。
    expect(screen.getByTestId("context-group-target_code")).toHaveTextContent(
      "explicit_task_target",
    );
    expect(screen.getByText(/working tree dirty/)).toBeInTheDocument();
  });

  it("generate posts task and scope (§293/§294)", async () => {
    ipc.getContextStatus
      .mockResolvedValueOnce({ status: "idle", error: null })
      .mockResolvedValue({ status: "completed", error: null });
    ipc.getContextResults.mockResolvedValue(pack);

    render(<ContextPage />);
    await waitFor(() => {
      expect(screen.getByTestId("context-idle")).toBeInTheDocument();
    });
    fireEvent.change(screen.getByTestId("context-task-input"), {
      target: { value: "修复 CI" },
    });
    fireEvent.click(screen.getByRole("button", { name: /generate/i }));
    expect(ipc.startContextGeneration).toHaveBeenCalledWith({
      task: "修复 CI",
      scope: "auto",
    });
  });

  it("copy exports markdown via export IPC (§302/§303 final markdown not JSON)", async () => {
    ipc.getContextStatus.mockResolvedValue({ status: "completed", error: null });
    ipc.getContextResults.mockResolvedValue(pack);
    ipc.exportContext.mockResolvedValue("# Prism Context Pack\n");

    Object.assign(navigator, {
      clipboard: { writeText: vi.fn().mockResolvedValue(undefined) },
    });

    render(<ContextPage />);
    await waitFor(() => {
      expect(screen.getByTestId("context-summary")).toBeInTheDocument();
    });
    fireEvent.click(screen.getByTestId("context-copy-markdown"));
    await waitFor(() => {
      expect(ipc.exportContext).toHaveBeenCalledWith("markdown");
    });
    expect(navigator.clipboard.writeText).toHaveBeenCalledWith(
      "# Prism Context Pack\n",
    );
  });
});
