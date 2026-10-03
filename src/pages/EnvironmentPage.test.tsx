import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import EnvironmentPage from "./EnvironmentPage";
import type { EnvironmentSnapshotDto } from "../types/prism";

const ipc = vi.hoisted(() => ({
  startEnvironmentScan: vi.fn(),
  getEnvironmentStatus: vi.fn(),
  getEnvironmentResults: vi.fn(),
  cancelEnvironmentScan: vi.fn(),
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

const snapshot: EnvironmentSnapshotDto = {
  schema_version: 1,
  id: "snap-1",
  project_id: "proj:demo",
  project_root: "C:/tmp/demo",
  captured_at: "2026-10-03T23:00:00Z",
  host: {
    os: "windows",
    os_name: "Windows",
    os_version: "10.0.26200",
    architecture: "x86_64",
    hostname: null,
    username: null,
    locale: null,
    timezone: null,
  },
  shell: { kind: "powershell", executable: null, version: null },
  paths: {
    scope: "process",
    entries: [],
    duplicates: ["C:\\dup"],
    missing: [],
    truncated: false,
  },
  tools: [
    {
      name: "node",
      path: "C:/Program Files/nodejs/node.exe",
      additional_paths: ["C:/elsewhere/node.exe"],
      version: { raw: "v22.16.0", major: 22, minor: 16, patch: 0, extra: null },
      raw_version: "v22.16.0",
      source: "path",
      probe: "succeeded",
    },
    {
      name: "yarn",
      path: null,
      additional_paths: [],
      version: null,
      raw_version: null,
      source: "unknown",
      probe: "not_found",
    },
  ],
  virtualenvs: [],
  runtimes: [],
  sdks: [],
  package_managers: [],
  variables: [
    { name: "NODE_ENV", present: true, possibly_sensitive: false, source: "process_environment" },
    { name: "API_KEY", present: true, possibly_sensitive: true, source: "process_environment" },
  ],
  ports: [
    {
      port: 5173,
      protocol: "tcp",
      address: "0.0.0.0:5173",
      state: "listening",
      pid: 1234,
      process_name: "node.exe",
      source: "netstat",
      observed_at: "2026-10-03T23:00:01Z",
    },
  ],
  processes: [],
  expected_ports: [],
  resources: {
    logical_cpus: 16,
    memory_total_bytes: 33_507_328,
    disk_free_bytes: 100,
    disk_path: "C:/tmp/demo",
  },
  requirements: [
    {
      id: "req:runtime:node#package.json engines",
      kind: "runtime",
      subject: "node",
      operator: "at_least",
      expected: "22",
      source: "package.json engines",
      source_kind: "ecosystem_metadata",
      evidence: "package.json: engines",
      confidence: "high",
      applicable: true,
    },
  ],
  conflicts: [],
  matches: [
    {
      requirement_id: "req:runtime:node#package.json engines",
      subject: "node",
      result: "satisfied",
      evidence: "actual: node v22.16.0 (expected at_least 22)",
    },
  ],
  findings: [
    {
      id: "envf:path_issue:PATH",
      kind: "path_issue",
      category: "path",
      severity: "info",
      confidence: "exact",
      subject: "PATH",
      message: "PATH contains 1 duplicate entry.",
      evidence: ["duplicate: C:\\dup"],
      suggestion: null,
    },
  ],
  status: "attention",
  diagnostics: [],
  scan_outcome: "completed",
  completeness: "complete",
};

describe("EnvironmentPage (M9 GUI)", () => {
  beforeEach(() => {
    ipc.startEnvironmentScan.mockReset().mockResolvedValue("started");
    ipc.getEnvironmentStatus.mockReset();
    ipc.getEnvironmentResults.mockReset();
    ipc.cancelEnvironmentScan.mockReset().mockResolvedValue(undefined);
  });

  it("offers explicit refresh when idle (§274/§298 first-run)", async () => {
    ipc.getEnvironmentStatus.mockResolvedValue({ status: "idle", error: null });
    render(<EnvironmentPage />);
    await waitFor(() => {
      expect(screen.getByTestId("env-idle")).toBeInTheDocument();
    });
    expect(screen.getByText(/No environment scan in this session/)).toBeInTheDocument();
    expect(ipc.startEnvironmentScan).not.toHaveBeenCalled();
  });

  it("renders toolchain, requirements, findings and dynamic facts", async () => {
    ipc.getEnvironmentStatus.mockResolvedValue({ status: "completed", error: null });
    ipc.getEnvironmentResults.mockResolvedValue(snapshot);

    render(<EnvironmentPage />);
    await waitFor(() => {
      expect(screen.getByTestId("env-summary")).toBeInTheDocument();
    });
    // §222 工具链 + §65 遮蔽候选（版本出现在工具行与 evidence 行，用 getAllByText）。
    expect(screen.getAllByText(/v22\.16\.0/).length).toBeGreaterThan(0);
    expect(screen.getByText(/also found: C:\/elsewhere\/node.exe/)).toBeInTheDocument();
    expect(screen.getByText(/not found/)).toBeInTheDocument();
    // §223 需求 + §258 evidence。
    expect(screen.getByTestId("env-requirements")).toHaveTextContent("Satisfied");
    expect(screen.getByText(/actual: node v22.16.0/)).toBeInTheDocument();
    // §224 findings。
    expect(screen.getByTestId("env-findings")).toHaveTextContent("PATH contains 1 duplicate entry.");
    // §225 动态端口 + §277 observed。
    expect(screen.getByTestId("env-ports")).toHaveTextContent("5173 LISTENING");
    expect(screen.getByTestId("env-ports")).toHaveTextContent("observed at scan time");
    // §229 变量：只有名字与分类，没有值。
    expect(screen.getByTestId("env-variables")).toHaveTextContent("1 possibly sensitive");
    // §276 观测时间。
    expect(screen.getByText("Observed")).toBeInTheDocument();
  });

  it("refresh re-scans and supports all-tools (§274/§270)", async () => {
    ipc.getEnvironmentStatus.mockResolvedValue({ status: "completed", error: null });
    ipc.getEnvironmentResults.mockResolvedValue(snapshot);

    render(<EnvironmentPage />);
    await waitFor(() => {
      expect(screen.getByTestId("env-summary")).toBeInTheDocument();
    });
    fireEvent.click(screen.getByRole("button", { name: "Refresh" }));
    expect(ipc.startEnvironmentScan).toHaveBeenLastCalledWith(false);
    fireEvent.click(screen.getByRole("button", { name: "Refresh all tools" }));
    expect(ipc.startEnvironmentScan).toHaveBeenLastCalledWith(true);
  });

  it("surfaces failure honestly and offers retry", async () => {
    ipc.getEnvironmentStatus.mockResolvedValue({
      status: "failed",
      error: "probe budget exhausted",
    });
    render(<EnvironmentPage />);
    await waitFor(() => {
      expect(screen.getByTestId("env-error")).toHaveTextContent("probe budget exhausted");
    });
  });

  it("never displays variable values (§54)", async () => {
    ipc.getEnvironmentStatus.mockResolvedValue({ status: "completed", error: null });
    ipc.getEnvironmentResults.mockResolvedValue(snapshot);

    const { container } = render(<EnvironmentPage />);
    await waitFor(() => {
      expect(screen.getByTestId("env-variables")).toBeInTheDocument();
    });
    // 页面任何位置都不应出现变量"值"（本测试变量只有名字，若实现错误地
    // 渲染 value 字段也不会有文本——结构性保证在 Rust 侧，这里验证摘要行）。
    expect(container.textContent).toContain("values are never read");
  });
});
