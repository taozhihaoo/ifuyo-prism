import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import CompletenessBanner, { isComplete } from "./CompletenessBanner";

const diag = (index: number) => ({
  kind: `kind_${index}`,
  subject: `subject_${index}`,
  detail: `detail ${index}`,
});

describe("CompletenessBanner (P2-5)", () => {
  it("renders nothing when complete without diagnostics", () => {
    const { container } = render(
      <CompletenessBanner
        data={{ domain: "测试域", completeness: "complete", diagnostics: [] }}
      />,
    );
    expect(container).toBeEmptyDOMElement();
    expect(
      isComplete({ domain: "d", completeness: "complete", diagnostics: [] }),
    ).toBe(true);
  });

  it("renders a banner with label and diagnostics when data is partial", () => {
    render(
      <CompletenessBanner
        data={{
          domain: "测试域",
          completeness: "partial",
          diagnostics: [diag(1)],
        }}
      />,
    );
    const banner = screen.getByTestId("completeness-banner");
    expect(banner).toHaveTextContent("测试域");
    expect(banner).toHaveTextContent("部分缺失");
    expect(banner).toHaveTextContent("kind_1");
    expect(banner).toHaveTextContent("[subject_1]");
  });

  it("collapses long diagnostic lists behind an expand toggle", () => {
    render(
      <CompletenessBanner
        data={{
          domain: "测试域",
          completeness: "complete",
          diagnostics: [diag(1), diag(2), diag(3), diag(4), diag(5)],
        }}
      />,
    );
    // complete 但带诊断 → 仍要展示（诊断即缺失事实）。
    expect(screen.getByTestId("completeness-banner")).toBeInTheDocument();
    expect(screen.getByText("kind_1")).toBeInTheDocument();
    expect(screen.queryByText("kind_4")).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /展开其余 2 条/ }));
    expect(screen.getByText("kind_4")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "收起" }));
    expect(screen.queryByText("kind_4")).not.toBeInTheDocument();
  });

  it("falls back to the raw completeness string for unknown values", () => {
    render(
      <CompletenessBanner
        data={{ domain: "测试域", completeness: "weird_state", diagnostics: [] }}
      />,
    );
    expect(screen.getByTestId("completeness-banner")).toHaveTextContent(
      "weird_state",
    );
  });
});
