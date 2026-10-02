import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import App from "./App";
import type { AppInfo } from "../types/prism";

const appInfoFixture: AppInfo = {
  name: "Prism",
  version: "0.1.0",
  os: "windows",
  arch: "x86_64",
  family: "windows",
  build_profile: "debug",
  generated_at: "2026-10-03T00:00:00Z",
};

const ipc = vi.hoisted(() => ({
  getAppInfo: vi.fn(),
  getPrismVersion: vi.fn(),
  getRuntimeInfo: vi.fn(),
  toPrismError: (err: unknown) => ({
    code: "internal",
    message: String(err),
  }),
}));

vi.mock("../services/ipc", () => ipc);

describe("App shell", () => {
  beforeEach(() => {
    ipc.getAppInfo.mockReset();
    ipc.getAppInfo.mockResolvedValue(appInfoFixture);
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
      "Reports",
    ]) {
      expect(screen.getByRole("button", { name: new RegExp(label) })).toBeInTheDocument();
    }

    // Overview 页展示真实（mock 的 IPC 契约）应用信息。
    await waitFor(() => {
      expect(screen.getByText("0.1.0")).toBeInTheDocument();
    });
    expect(screen.getByText("windows")).toBeInTheDocument();
  });

  it("marks unimplemented areas honestly with their milestone", async () => {
    render(<App />);

    const nav = within(screen.getByRole("navigation", { name: "Primary" }));
    fireEvent.click(nav.getByRole("button", { name: /Health/ }));
    expect(await screen.findByTestId("placeholder-page")).toHaveTextContent(
      "Coming in M7",
    );
  });

  it("shows a core error in the status bar instead of fake data", async () => {
    ipc.getAppInfo.mockRejectedValueOnce({ code: "io", message: "disk gone" });
    render(<App />);
    expect(await screen.findByRole("alert")).toHaveTextContent(/core unavailable/);
  });
});
