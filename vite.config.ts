import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Tauri 前端：固定端口、退出清屏交给 Tauri CLI。
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
  },
  build: {
    target: "es2022",
  },
});
