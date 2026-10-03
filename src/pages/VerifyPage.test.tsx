import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import VerifyPage from "./VerifyPage";
import type { PlannedCheckDto, VerificationReportDto, VerifyPlanDto } from "../types/prism";

const ipc = vi.hoisted(() => ({
  getVerifyPlan: vi.fn(),
  startVerify: vi.fn(),
  getVerifyStatus: vi.fn(),
  getVerifyResults: vi.fn(),
  cancelVerify: vi.fn(),
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

const check = (
  id: string,
  kind: string,
  program: string,
  dependsOn: string[] = [],
): PlannedCheckDto => ({
  id,
  kind,
  name: `${id} check`,
  program,
  args: ["--silent"],
  required: kind !== "lint",
  source: "package_script",
  detected_from: "package.json",
  depends_on: dependsOn,
});

const planFixture: VerifyPlanDto = {
  checks: [
    check("build", "build", "npm"),
    check("test", "test", "npm", ["build"]),
    check("lint", "lint", "npx"),
  ],
};

const runOf = (
  runId: string,
  checkId: string,
  state: string,
  extra: Partial<{ stdout: string; exit_code: number | null; diagnostics: { kind: string; detail: string }[] }> = {},
) => ({
  run_id: runId,
  check_id: checkId,
  kind: checkId,
  name: `${checkId} check`,
  required: true,
  state,
  command_display: `npm run ${checkId}`,
  working_directory: "C:/tmp/demo",
  started_at: "2026-10-04T00:00:00Z",
  finished_at: "2026-10-04T00:00:01Z",
  duration_ms: 1200,
  exit_code: extra.exit_code ?? 0,
  stdout: extra.stdout ?? "",
  stderr: "",
  output_truncated: false,
  diagnostics: extra.diagnostics ?? [],
  reason: null,
  side_effects: [],
});

const counts = {
  passed: 2,
  failed: 1,
  unavailable: 0,
  blocked: 0,
  skipped: 0,
  cancelled: 0,
  timed_out: 0,
  error: 0,
};

const reportFixture: VerificationReportDto = {
  schema_version: 1,
  project_name: "demo",
  plan: planFixture,
  runs: [
    runOf("r1", "build", "passed"),
    runOf("r2", "test", "failed", {
      exit_code: 1,
      stdout: "FAIL src/a.test.ts\nexpected 1 to be 2",
      diagnostics: [{ kind: "test_failure", detail: "a.test.ts:3 expected 1 to be 2" }],
    }),
    runOf("r3", "lint", "passed"),
  ],
  conclusion: "fail",
  completeness: "complete",
  counts,
  side_effects: [],
  impacted_test_candidates: [],
};

describe("VerifyPage (M8 GUI)", () => {
  beforeEach(() => {
    ipc.getVerifyPlan.mockReset().mockResolvedValue(planFixture);
    ipc.startVerify.mockReset().mockResolvedValue("started");
    ipc.getVerifyStatus.mockReset();
    ipc.getVerifyResults.mockReset();
    ipc.cancelVerify.mockReset().mockResolvedValue(undefined);
  });

  it("shows the plan first and never executes on its own (§298)", async () => {
    ipc.getVerifyStatus.mockResolvedValue({ status: "idle", error: null });

    render(<VerifyPage />);

    await waitFor(() => {
      expect(screen.getByTestId("verify-plan")).toBeInTheDocument();
    });
    expect(screen.getByText(/Nothing has been executed yet/)).toBeInTheDocument();
    expect(screen.getAllByText(/--silent/)).toHaveLength(3);
    expect(screen.getAllByText(/npm --silent/)).toHaveLength(2);
    expect(screen.getByText(/npx --silent/)).toBeInTheDocument();
    expect(screen.getByText("build")).toBeInTheDocument();
    expect(ipc.startVerify).not.toHaveBeenCalled();
  });

  it("runs on explicit request and renders real results (§118/§3)", async () => {
    ipc.getVerifyStatus
      .mockResolvedValueOnce({ status: "idle", error: null })
      .mockResolvedValueOnce({ status: "running", error: null })
      .mockResolvedValue({ status: "completed", error: null });
    ipc.getVerifyResults.mockResolvedValue({ plan: planFixture, report: reportFixture });

    render(<VerifyPage />);
    await waitFor(() => {
      expect(screen.getByTestId("verify-plan")).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole("button", { name: /run verification/i }));
    expect(ipc.startVerify).toHaveBeenCalledWith(undefined);

    await waitFor(
      () => {
        expect(screen.getByTestId("verify-summary")).toBeInTheDocument();
      },
      { timeout: 3000 },
    );
    expect(screen.getByText("Verification · FAIL")).toBeInTheDocument();
    expect(screen.getByText("[FAILED]")).toBeInTheDocument();
    expect(screen.getByText(/npm run test/)).toBeInTheDocument();
    expect(screen.getByText("2")).toBeInTheDocument(); // passed count
  });

  it("filters runs by check kind chips", async () => {
    ipc.getVerifyStatus.mockResolvedValue({ status: "completed", error: null });
    ipc.getVerifyResults.mockResolvedValue({ plan: planFixture, report: reportFixture });

    render(<VerifyPage />);
    await waitFor(() => {
      expect(screen.getByTestId("verify-runs")).toBeInTheDocument();
    });
    expect(screen.getByText(/npm run build/)).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "test" }));
    expect(screen.getByText(/npm run test/)).toBeInTheDocument();
    expect(screen.queryByText(/npm run build/)).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "all" }));
    expect(screen.getByText(/npm run build/)).toBeInTheDocument();
  });

  it("expands a failed run with output and diagnostics (§113-§115)", async () => {
    ipc.getVerifyStatus.mockResolvedValue({ status: "completed", error: null });
    ipc.getVerifyResults.mockResolvedValue({ plan: planFixture, report: reportFixture });

    render(<VerifyPage />);
    await waitFor(() => {
      expect(screen.getByTestId("verify-runs")).toBeInTheDocument();
    });

    fireEvent.click(screen.getByText(/npm run test/));
    expect(screen.getByText(/FAIL src\/a\.test\.ts/)).toBeInTheDocument(); // stdout
    expect(screen.getByText(/a\.test\.ts:3/)).toBeInTheDocument(); // diagnostic
    expect(screen.getByText(/Exit code:/)).toBeInTheDocument();
    expect(screen.getByText(/test_failure/)).toBeInTheDocument();
  });

  it("awaits honestly when completion carries no report", async () => {
    ipc.getVerifyStatus.mockResolvedValue({ status: "completed", error: null });
    ipc.getVerifyResults.mockResolvedValue({ plan: planFixture, report: null });

    render(<VerifyPage />);
    await waitFor(() => {
      expect(screen.getByText(/Awaiting verification results/)).toBeInTheDocument();
    });
  });

  it("surfaces a failed verification honestly", async () => {
    ipc.getVerifyStatus.mockResolvedValue({
      status: "failed",
      error: "plan construction exploded",
    });

    render(<VerifyPage />);
    await waitFor(() => {
      expect(screen.getByTestId("verify-error")).toHaveTextContent(
        "plan construction exploded",
      );
    });
  });

  it("reports completion without results as an error, not fake success", async () => {
    ipc.getVerifyStatus.mockResolvedValue({ status: "completed", error: null });
    ipc.getVerifyResults.mockResolvedValue(null);

    render(<VerifyPage />);
    await waitFor(() => {
      expect(screen.getByTestId("verify-error")).toHaveTextContent(
        /without results/,
      );
    });
  });

  it("cancels a running verification on unmount (§93)", async () => {
    ipc.getVerifyStatus.mockResolvedValue({ status: "idle", error: null });

    const { unmount } = render(<VerifyPage />);
    await waitFor(() => {
      expect(screen.getByTestId("verify-plan")).toBeInTheDocument();
    });
    unmount();
    await waitFor(() => {
      expect(ipc.cancelVerify).toHaveBeenCalled();
    });
  });

  it("keeps the empty-state hidden while a project is open", () => {
    ipc.getVerifyStatus.mockResolvedValue({ status: "idle", error: null });
    render(<VerifyPage />);
    expect(screen.queryByTestId("verify-needs-project")).not.toBeInTheDocument();
  });
});
