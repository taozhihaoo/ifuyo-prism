import { render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import OverviewPage from "./OverviewPage";
import type { GitFactsDto, VerificationReportDto } from "../types/prism";

const ipc = vi.hoisted(() => ({
  getVerifyResults: vi.fn(),
  getGitFacts: vi.fn(),
}));

vi.mock("../services/ipc", () => ({
  ...ipc,
  formatBytesLabel: (n: number) => `${n} B`,
}));

const projectFixture = {
  id: "p1",
  name: "demo",
  root_path: "C:/tmp/demo",
  discovered_at: "2026-10-04T00:00:00Z",
};

const discoveryFixture = {
  project: { name: "demo", root_path: "C:/tmp/demo" },
  statistics: {
    total_files: 3,
    total_directories: 1,
    total_size_bytes: 300,
  },
  scan: { duration_ms: 5, errors_count: 0 },
  errors: [],
  project_types: [],
  languages: [],
  entry_points: [],
};

vi.mock("../app/ProjectStore", () => ({
  useProjectStore: () => ({
    project: projectFixture,
    phase: "completed",
    scanStatus: null,
    discovery: discoveryFixture,
    pickAndOpenProject: vi.fn(),
    lastError: null,
  }),
}));

vi.mock("../components/ScanProgressCard", () => ({
  default: () => <div />,
}));
vi.mock("../components/ProjectTypesCard", () => ({
  default: () => <div />,
}));

const appInfo = {
  name: "prism",
  version: "0.1.0",
  os: "windows",
  arch: "x64",
  family: "desktop",
  build_profile: "debug",
  generated_at: "2026-10-04T00:00:00Z",
};

const gitFacts = (commitId: string | null, dirty: number): GitFactsDto =>
  ({
    repository: commitId === null ? null : { path: "C:/tmp/demo", worktree: "C:/tmp/demo" },
    head:
      commitId === null
        ? null
        : {
            kind: "branch",
            commit_id: commitId,
            branch_name: "main",
            symbolic_ref: "refs/heads/main",
          },
    status: {
      repository: null,
      head: null,
      entries: Array.from({ length: dirty }, (_, i) => ({
        path: `f${i}.ts`,
        original_path: null,
        index_state: "unmodified",
        working_tree_state: "modified",
        is_untracked: false,
        is_ignored: false,
        is_renamed: false,
        is_copied: false,
        is_conflicted: false,
      })),
      ignored: [],
    },
    recent_commits: [],
    refs: [],
  }) as never;

const report = (
  verifiedState: VerificationReportDto["verified_state"],
): VerificationReportDto =>
  ({
    schema_version: 1,
    project_name: "demo",
    plan: { checks: [] },
    runs: [
      {
        run_id: "r1",
        check_id: "build",
        kind: "build",
        name: "build",
        required: true,
        state: "passed",
        command_display: "npm run build",
        working_directory: ".",
        started_at: null,
        finished_at: null,
        duration_ms: 10,
        exit_code: 0,
        stdout: "",
        stderr: "",
        output_truncated: false,
        diagnostics: [],
        reason: null,
        side_effects: [],
      },
    ],
    conclusion: "pass",
    completeness: "complete",
    counts: {
      passed: 1,
      failed: 0,
      unavailable: 0,
      blocked: 0,
      skipped: 0,
      cancelled: 0,
      timed_out: 0,
      error: 0,
    },
    side_effects: [],
    impacted_test_candidates: [],
    verified_state: verifiedState,
  }) as never;

describe("OverviewPage verification card (M8 §212/§213/§275)", () => {
  beforeEach(() => {
    ipc.getVerifyResults.mockReset();
    ipc.getGitFacts.mockReset();
  });

  it("says 'no verification run in this session' when none exists (§275)", async () => {
    ipc.getVerifyResults.mockResolvedValue(null);
    ipc.getGitFacts.mockResolvedValue(gitFacts("c1", 0));

    render(<OverviewPage appInfo={appInfo} />);
    await waitFor(() => {
      expect(screen.getByTestId("verify-no-run")).toHaveTextContent(
        "No verification run in this session.",
      );
    });
  });

  it("lists session results with per-kind states (§212)", async () => {
    ipc.getVerifyResults.mockResolvedValue({
      plan: { checks: [] },
      report: report({ kind: "clean_at_commit", commit: "c1" }),
    });
    ipc.getGitFacts.mockResolvedValue(gitFacts("c1", 0));

    render(<OverviewPage appInfo={appInfo} />);
    await waitFor(() => {
      expect(screen.getByText("build")).toBeInTheDocument();
    });
    expect(screen.getByText("passed")).toBeInTheDocument();
    expect(screen.queryByTestId("verify-stale")).not.toBeInTheDocument();
  });

  it("marks results stale when HEAD moved (§213)", async () => {
    ipc.getVerifyResults.mockResolvedValue({
      plan: { checks: [] },
      report: report({ kind: "clean_at_commit", commit: "c1" }),
    });
    ipc.getGitFacts.mockResolvedValue(gitFacts("c2", 0));

    render(<OverviewPage appInfo={appInfo} />);
    await waitFor(() => {
      expect(screen.getByTestId("verify-stale")).toHaveTextContent(/stale/i);
    });
  });

  it("marks results stale when the working tree became dirty (§213)", async () => {
    ipc.getVerifyResults.mockResolvedValue({
      plan: { checks: [] },
      report: report({ kind: "clean_at_commit", commit: "c1" }),
    });
    ipc.getGitFacts.mockResolvedValue(gitFacts("c1", 3));

    render(<OverviewPage appInfo={appInfo} />);
    await waitFor(() => {
      expect(screen.getByTestId("verify-stale")).toHaveTextContent(
        /Working tree changed/,
      );
    });
  });

  it("shows the dirty-at-verification note (§277)", async () => {
    ipc.getVerifyResults.mockResolvedValue({
      plan: { checks: [] },
      report: report({ kind: "dirty_workspace", changed_files: 8 }),
    });
    ipc.getGitFacts.mockResolvedValue(gitFacts("c1", 8));

    render(<OverviewPage appInfo={appInfo} />);
    await waitFor(() => {
      expect(
        screen.getByText(/8 working-tree changes present at verification time/),
      ).toBeInTheDocument();
    });
  });
});
