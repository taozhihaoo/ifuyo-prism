import "@testing-library/jest-dom/vitest";
import { afterEach } from "vitest";
import { cleanup } from "@testing-library/react";

// vitest 未启用 globals 时 RTL 不会自动注册 cleanup，需显式执行，
// 否则前一个用例的 DOM 会泄漏到下一个用例。
afterEach(() => {
  cleanup();
});
