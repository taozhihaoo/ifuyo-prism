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
