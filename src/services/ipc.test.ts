import { describe, expect, it } from "vitest";
import { toPrismError } from "./ipc";

describe("toPrismError", () => {
  it("maps a PrismErrorDto-shaped rejection", () => {
    expect(toPrismError({ code: "config", message: "bad config" })).toEqual({
      code: "config",
      message: "bad config",
    });
  });

  it("wraps plain values as internal errors", () => {
    expect(toPrismError("boom")).toEqual({ code: "internal", message: "boom" });
    expect(toPrismError(undefined).code).toBe("internal");
  });

  it("rejects objects missing code or message", () => {
    const mapped = toPrismError({ code: "config" });
    expect(mapped.code).toBe("internal");
  });
});
