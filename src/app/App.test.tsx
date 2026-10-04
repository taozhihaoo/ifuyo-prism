import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import App from "./App";
import type { AppInfo, ProjectDiscovery } from "../types/prism";

const appInfoFixture: AppInfo = {
  name: "Prism",
  version: "0.1.0",
  os: "windows",
  arch: "x86_64",
  family: "windows",
  build_profile: "debug",
  generated_at: "2026-10-03T00:00:00Z",
};

const projectFixture = {
  id: "proj-1",
  name: "demo",
  root_path: "C:/tmp/demo",
  discovered_at: "2026-10-03T00:00:00Z",
};

export function discoveryFixture(): ProjectDiscovery {
  return {
    schema_version: 1,
    project: projectFixture,
    scan: {
      scan_id: "scan-1",
      root_path: "C:/tmp/demo",
      started_at: "2026-10-03T00:00:00Z",
      finished_at: "2026-10-03T00:00:01Z",
      duration_ms: 12,
      files_scanned: 2,
      directories_scanned: 1,
      files_ignored: 1,
      errors_count: 0,
      status: "completed",
    },
    files: [
      {
        id: "file:src/main.ts",
        relative_path: "src/main.ts",
        file_name: "main.ts",
        extension: "ts",
        size_bytes: 10,
        language: "typescript",
        classification: { kind: "source", confidence: "high", reason: "TypeScript source extension" },
        analyzable: true,
        non_analyzable_reason: null,
        modified_at: null,
        is_hidden: false,
        is_symlink: false,
      },
    ],
    directories: [
      { id: "dir:src", relative_path: "src", name: "src", depth: 1, is_hidden: false },
    ],
    languages: [{ language: "typescript", count: 1 }],
    project_types: [
      {
        name: "Node.js",
        confidence: "high",
        evidence: [{ source: "project_type_detector", path: "package.json", reason: "manifest found" }],
      },
    ],
    configurations: [],
    entry_points: [
      {
        path: "src/main.ts",
        kind: "frontend",
        confidence: "high",
        evidence: [{ source: "entry_point_detector", path: "src/main.ts", reason: "conventional frontend entry" }],
      },
    ],
    statistics: {
      total_files: 2,
      total_directories: 1,
      total_size_bytes: 20,
      source_files: 1,
      test_files: 0,
      config_files: 1,
      documentation_files: 0,
      asset_files: 0,
      binary_files: 0,
      generated_files: 0,
      unknown_files: 0,
      ignored_files: 1,
      error_count: 0,
    },
    errors: [],
  };
}

const ipc = vi.hoisted(() => ({
  listRecentProjects: vi.fn().mockResolvedValue([]),
  getAppInfo: vi.fn(),
  getPrismVersion: vi.fn(),
  getRuntimeInfo: vi.fn(),
  openProject: vi.fn(),
  startScan: vi.fn(),
  cancelScan: vi.fn(),
  getScanStatus: vi.fn(),
  getDiscovery: vi.fn(),
  pickDirectory: vi.fn(),
  toPrismError: (err: unknown) => ({
    code: "internal",
    message: String(err),
  }),
}));

vi.mock("../services/ipc", () => ipc);

describe("App shell", () => {
  beforeEach(() => {
    ipc.getAppInfo.mockReset().mockResolvedValue(appInfoFixture);
    ipc.pickDirectory.mockReset();
    ipc.openProject.mockReset();
    ipc.startScan.mockReset().mockResolvedValue("task-1");
    ipc.cancelScan.mockReset().mockResolvedValue(undefined);
    ipc.getScanStatus.mockReset();
    ipc.getDiscovery.mockReset();
  });

  it("renders brand, full navigation and real app info", async () => {
    render(<App />);

    expect(screen.getByText("Prism")).toBeInTheDocument();
    for (const label of [
      "Overview",
      "Understand",
      "Changes",
      "Health",
      "Verify",
      "Environment",
      "Context",
      "History",
    ]) {
      expect(screen.getByRole("button", { name: new RegExp(label) })).toBeInTheDocument();
    }

    await waitFor(() => {
      expect(screen.getByText("0.1.0")).toBeInTheDocument();
    });
    expect(screen.getByText("windows")).toBeInTheDocument();
  });

  it("shows an honest empty state before a project is opened", () => {
    render(<App />);
    expect(screen.getByTestId("empty-state")).toHaveTextContent("No project opened.");
    expect(screen.getByRole("button", { name: "Open Project" })).toBeInTheDocument();
  });

  it("opens a project, scans, and shows the real discovery summary", async () => {
    ipc.pickDirectory.mockResolvedValue("C:/tmp/demo");
    ipc.openProject.mockResolvedValue(projectFixture);
    ipc.getScanStatus.mockResolvedValue({
      project: projectFixture,
      status: "completed",
      directories: 1,
      files: 2,
      ignored: 1,
      errors: 0,
      has_warnings: false,
      error: null,
    });
    ipc.getDiscovery.mockResolvedValue(discoveryFixture());

    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Open Project" }));

    expect(await screen.findByTestId("project-summary")).toBeInTheDocument();
    expect(screen.getByText("demo")).toBeInTheDocument();
    // 项目类型卡片展示证据来源。
    expect(screen.getByRole("button", { name: /Node\.js/ })).toBeInTheDocument();
  });

  it("keeps the empty state when the user cancels directory selection", async () => {
    ipc.pickDirectory.mockResolvedValue(null);
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Open Project" }));
    await waitFor(() => {
      expect(ipc.openProject).not.toHaveBeenCalled();
    });
    expect(screen.getByTestId("empty-state")).toBeInTheDocument();
  });

  it("Understand page explains itself before a scan exists", async () => {
    render(<App />);
    const nav = within(screen.getByRole("navigation", { name: "Primary" }));
    fireEvent.click(nav.getByRole("button", { name: /Understand/ }));
    expect(await screen.findByTestId("placeholder-page")).toHaveTextContent("Open and scan a project");
  });
});
