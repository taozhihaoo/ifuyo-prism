import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it } from "vitest";
import SettingsPage from "./SettingsPage";
import { SettingsProvider, useUiSettings } from "../app/SettingsProvider";
import { applySettings, loadSettings } from "../app/uiSettings";

function Probe() {
  const { settings } = useUiSettings();
  return (
    <span data-testid="probe">
      {settings.theme}/{settings.density}
    </span>
  );
}

describe("SettingsPage (M12 §48/§11)", () => {
  beforeEach(() => {
    localStorage.removeItem("prism.ui.settings");
    document.documentElement.dataset.theme = "dark";
    document.documentElement.dataset.density = "comfortable";
  });

  it("renders storage info and current theme (§48)", async () => {
    render(
      <SettingsProvider>
        <SettingsPage />
      </SettingsProvider>,
    );
    await waitFor(() => {
      expect(screen.getByTestId("settings-db-path")).toBeInTheDocument();
    });
    expect(screen.getByText(/Theme:/)).toBeInTheDocument();
  });

  it("theme switch applies data-theme and persists (§11)", async () => {
    render(
      <SettingsProvider>
        <SettingsPage />
        <Probe />
      </SettingsProvider>,
    );
    fireEvent.click(screen.getByText("light"));
    expect(document.documentElement.dataset.theme).toBe("light");
    expect(screen.getByTestId("probe")).toHaveTextContent("light/comfortable");
    // §48 持久化：重新 loadSettings 应读回。
    expect(loadSettings().theme).toBe("light");
  });

  it("density switch applies data-density", async () => {
    render(
      <SettingsProvider>
        <SettingsPage />
        <Probe />
      </SettingsProvider>,
    );
    fireEvent.click(screen.getByText("compact"));
    expect(document.documentElement.dataset.density).toBe("compact");
    expect(screen.getByTestId("probe")).toHaveTextContent(/compact/);
  });
});

// applySettings 独立验证（§244 无副作用渲染路径）。
describe("applySettings", () => {
  it("sets dataset attributes", () => {
    applySettings({ theme: "light", density: "compact" });
    expect(document.documentElement.dataset.theme).toBe("light");
    expect(document.documentElement.dataset.density).toBe("compact");
  });
});
