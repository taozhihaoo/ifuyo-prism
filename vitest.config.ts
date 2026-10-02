import { defineConfig, defaultExclude } from "vitest/config";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  test: {
    environment: "jsdom",
    setupFiles: ["./src/test/setup.ts"],
    // 仓库的 tests/ 目录是 Rust 集成测试与合成 fixture（fixture-basic 里的
    // *.test.ts 供 M8 verify 测试发现使用），不是前端测试。
    exclude: [...defaultExclude, "tests/**"],
  },
});
