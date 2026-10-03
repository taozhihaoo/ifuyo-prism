import { render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import ChangesPage from "./ChangesPage";
import type { ChangeSetDto, GitFactsDto } from "../types/prism";

const ipc = vi.hoisted(() => ({
  getGitFacts: vi.fn(),
  startChangeset: vi.fn(),
  getChangesetStatus: vi.fn(),
  getChangeset: vi.fn(),
  cancelChangeset: vi.fn(),
  startImpact: vi.fn(),
  getImpactStatus: vi.fn(),
  getImpact: vi.fn(),
  cancelImpact: vi.fn(),
  toPrismError: (err: unknown) => ({
    code: "internal",
    message: String(err),
  }),
}));

vi.mock("../services/ipc", () => ipc);

const projectFixture = {
  id: "p1",
  name: "demo",
  root_path: "C:/tmp/demo",
  discovered_at: "2026-10-03T00:00:00Z",
};

vi.mock("../app/ProjectStore", () => ({
  useProjectStore: () => ({ project: projectFixture }),
}));

const changesetFixture: ChangeSetDto = {
  schema_version: 1,
  id: "changeset:demo#aaa#worktree#working_tree",
  source: "git",
  scope: "working_tree",
  base: "aaa",
  target: "worktree",
  repository: "demo",
  files: [
    {
      id: "fc:src/greet.ts",
      old_path: null,
      new_path: "src/greet.ts",
      change_type: "modified",
      additions: 1,
      deletions: 1,
      old_exists: true,
      new_exists: true,
      old_language: "typescript",
      new_language: "typescript",
      old_file_kind: "source",
      new_file_kind: "source",
      language_changed: false,
      is_binary: false,
      similarity: null,
      mapping_status: "mapped",
    },
    {
      id: "fc:assets/logo.png",
      old_path: null,
      new_path: "assets/logo.png",
      change_type: "modified",
      additions: null,
      deletions: null,
      old_exists: true,
      new_exists: true,
      old_language: null,
      new_language: null,
      old_file_kind: "asset",
      new_file_kind: "asset",
      language_changed: false,
      is_binary: true,
      similarity: null,
      mapping_status: "not_applicable",
    },
  ],
  symbols: [
    {
      file_change_id: "fc:src/greet.ts",
      path: "src/greet.ts",
      before_symbol_id: "symbol:src/greet.ts#greet#function",
      after_symbol_id: "symbol:src/greet.ts#greet#function",
      before_qualified_name: "src/greet.greet",
      after_qualified_name: "src/greet.greet",
      name: "greet",
      kind: "function",
      change_type: "modified",
      characteristics: ["body_changed"],
      location_before: { start_line: 1, end_line: 3 },
      location_after: { start_line: 1, end_line: 3 },
      mapping_status: "mapped",
      confidence: "exact",
    },
  ],
  modules: [
    {
      id: "mc:module:file:src/greet.ts",
      module_id: "module:file:src/greet.ts",
      module_name: "greet.ts",
      module_kind: "file_module",
      change_type: "modified",
      changed_files: ["src/greet.ts"],
      changed_symbol_count: 1,
      mapping_status: "mapped",
    },
  ],
  summary: {
    files_total: 2,
    files_added: 0,
    files_modified: 2,
    files_deleted: 0,
    files_renamed: 0,
    files_copied: 0,
    files_type_changed: 0,
    files_other: 0,
    symbols_total: 1,
    symbols_added: 0,
    symbols_deleted: 0,
    symbols_modified: 1,
    symbols_renamed: 0,
    symbols_moved: 0,
    modules_changed: 1,
    files_with_ambiguous_symbols: 0,
    test_files_changed: 0,
    documentation_files_changed: 0,
    diagnostics_total: 2,
  },
  diagnostics: [
    {
      kind: "binary_file",
      path: "assets/logo.png",
      detail: "binary content; symbol mapping not applicable",
    },
    {
      kind: "unsupported_language",
      path: "assets/logo.png",
      detail: "after side has no parser for this language",
    },
  ],
  truncated: false,
};

const factsFixture: GitFactsDto = {
  repository: {
    root_path: "C:/tmp/demo",
    git_dir: "C:/tmp/demo/.git",
    common_git_dir: "C:/tmp/demo/.git",
    is_bare: false,
    is_worktree: false,
  },
  head: {
    kind: "branch",
    commit_id: "aaa",
    branch_name: "main",
    symbolic_ref: "refs/heads/main",
  },
  status: {
    repository: null,
    head: null,
    entries: [],
    ignored: [],
  },
  recent_commits: [],
  refs: [],
};

describe("ChangesPage (M5 ChangeSet)", () => {
  beforeEach(() => {
    ipc.getGitFacts.mockReset().mockResolvedValue(factsFixture);
    ipc.startChangeset.mockReset().mockResolvedValue("started");
    ipc.getChangesetStatus.mockReset();
    ipc.getChangeset.mockReset();
    ipc.cancelChangeset.mockReset().mockResolvedValue(undefined);
    // M6 默认 idle：不触发 impact 链（专项测试里再覆盖）。
    ipc.startImpact.mockReset().mockResolvedValue("started");
    ipc.getImpactStatus.mockReset().mockResolvedValue({ status: "idle", error: null });
    ipc.getImpact.mockReset().mockResolvedValue(null);
    ipc.cancelImpact.mockReset().mockResolvedValue(undefined);
  });

  it("renders summary, files, symbols, modules and diagnostics", async () => {
    ipc.getChangesetStatus.mockResolvedValue({ status: "completed", error: null });
    ipc.getChangeset.mockResolvedValue(changesetFixture);

    render(<ChangesPage />);

    await waitFor(() => {
      expect(screen.getByTestId("change-summary")).toBeInTheDocument();
    });
    expect(screen.getByText(/2 \(0 added, 2 modified/)).toBeInTheDocument();
    expect(screen.getByText("src/greet.ts")).toBeInTheDocument();
    expect(screen.getByText("greet")).toBeInTheDocument();
    expect(screen.getByText(/body_changed/)).toBeInTheDocument();
    expect(screen.getByText("module:file:src/greet.ts")).toBeInTheDocument();
    expect(screen.getByText(/binary_file/)).toBeInTheDocument();
    // M6 边界（§69）：不显示风险/影响。
    expect(screen.queryByText(/risk/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/impact/i)).not.toBeInTheDocument();
  });

  it("starts the working-tree analysis and polls until completed", async () => {
    ipc.getChangesetStatus
      .mockResolvedValueOnce({ status: "idle", error: null })
      .mockResolvedValue({ status: "completed", error: null });
    ipc.getChangeset.mockResolvedValue(changesetFixture);

    render(<ChangesPage />);
    await waitFor(() => {
      expect(ipc.startChangeset).toHaveBeenCalledWith("working_tree");
    });
    await waitFor(() => {
      expect(screen.getByTestId("changed-files")).toBeInTheDocument();
    });
  });

  it("shows the friendly not-git state", async () => {
    ipc.getChangesetStatus.mockResolvedValue({ status: "completed", error: null });
    ipc.getChangeset.mockResolvedValue(null);
    ipc.getGitFacts.mockResolvedValue({ ...factsFixture, repository: null });

    render(<ChangesPage />);
    await waitFor(() => {
      expect(screen.getByTestId("not-git")).toBeInTheDocument();
    });
  });

  it("surfaces failure honestly", async () => {
    ipc.getChangesetStatus.mockResolvedValue({
      status: "failed",
      error: "git exploded",
    });
    render(<ChangesPage />);
    await waitFor(() => {
      expect(screen.getByTestId("changes-error")).toHaveTextContent("git exploded");
    });
  });
});

describe("ChangesPage (M6 Impact)", () => {
  beforeEach(() => {
    ipc.getGitFacts.mockReset().mockResolvedValue(factsFixture);
    ipc.startChangeset.mockReset().mockResolvedValue("started");
    ipc.getChangesetStatus.mockReset().mockResolvedValue({ status: "completed", error: null });
    ipc.getChangeset.mockReset().mockResolvedValue(changesetFixture);
    ipc.startImpact.mockReset().mockResolvedValue("started");
    ipc.getImpactStatus.mockReset().mockResolvedValue({ status: "completed", error: null });
    ipc.getImpact.mockReset().mockResolvedValue({
      schema_version: 1,
      id: "impact:cs#10#2000",
      change_set_id: "changeset:demo#1",
      scope: "working_tree",
      analysis_completeness: "complete",
      analysis_incomplete: false,
      changed_items: [],
      impacted_items: [
        {
          node_id: "symbol:src/checkout.ts#src/checkout.checkout#function",
          node_kind: "symbol",
          impact_kinds: ["direct_dependent", "caller"],
          distance: 1,
          confidence: "high_confidence",
          reasons: ["calls_changed_symbol"],
          evidence_path_count: 1,
          additional_paths: 0,
          path: "src/checkout.ts",
          name: "checkout",
        },
        {
          node_id: "module:file:src/checkout.test.ts",
          node_kind: "file_module",
          impact_kinds: ["transitive_dependent", "test_candidate"],
          distance: 2,
          confidence: "high_confidence",
          reasons: ["transitive_dependency"],
          evidence_path_count: 1,
          additional_paths: 0,
          path: "src/checkout.test.ts",
          name: "checkout.test.ts",
        },
      ],
      impact_paths: [
        {
          id: "ip:p1",
          source_change: "symbol:src/lib.ts#lib.pay#function",
          target: "symbol:src/checkout.ts#src/checkout.checkout#function",
          distance: 1,
          hops: [
            {
              from: "symbol:src/lib.ts#lib.pay#function",
              relation: "calls",
              to: "symbol:src/checkout.ts#src/checkout.checkout#function",
              resolution: "resolved",
              confidence: "high_confidence",
            },
          ],
        },
      ],
      summary: {
        changed_files: 1,
        changed_symbols: 1,
        changed_modules: 1,
        direct_impacted_modules: 0,
        direct_impacted_symbols: 1,
        transitive_impacted_modules: 1,
        transitive_impacted_symbols: 0,
        potentially_affected_tests: 1,
        documentation_candidates: 0,
        unresolved_relationships: 0,
        ambiguous_relationships: 0,
        max_distance: 2,
        diagnostics_total: 1,
      },
      diagnostics: [
        {
          kind: "no_documentation_evidence",
          subject: null,
          detail: "no resolvable document reference relationships",
        },
      ],
      risk_signals: [
        {
          kind: "cross_module_impact",
          count: 1,
          detail: "impact extends beyond the changed files themselves",
          evidence: [],
        },
      ],
      truncated: false,
    } as never);
    ipc.cancelImpact.mockReset().mockResolvedValue(undefined);
  });

  it("renders impact summary, filterable items, paths and limitations", async () => {
    const { render, screen, waitFor } = await import("@testing-library/react");
    render(<ChangesPage />);
    await waitFor(() => {
      expect(screen.getByTestId("impact-summary")).toBeInTheDocument();
    });
    expect(screen.getByText(/1 modules, 1 symbols, 1 tests/)).toBeInTheDocument();
    expect(screen.getByText("checkout")).toBeInTheDocument();
    // §84：测试候选双分类可见；§27 措辞是 candidate 不是 will-fail。
    expect(screen.getByText(/test candidate/)).toBeInTheDocument();
    expect(screen.queryByText(/will fail/i)).not.toBeInTheDocument();
    // §73：Why impacted 路径。
    expect(screen.getByTestId("impact-paths")).toBeInTheDocument();
    // §74/§131：局限显式。
    expect(screen.getByTestId("impact-risk-signals")).toBeInTheDocument();
    expect(screen.getByText(/cross_module_impact/)).toBeInTheDocument();
    expect(screen.getByText(/based on resolvable static relationships/i)).toBeInTheDocument();
  });

  it("filters impacted items by test candidate chip", async () => {
    const { render, screen, waitFor, fireEvent } = await import("@testing-library/react");
    render(<ChangesPage />);
    await waitFor(() => {
      expect(screen.getByTestId("impact-items")).toBeInTheDocument();
    });
    fireEvent.click(screen.getByRole("button", { name: "tests" }));
    expect(screen.getByText("checkout.test.ts")).toBeInTheDocument();
    expect(screen.queryByText("checkout")).not.toBeInTheDocument();
  });
});
