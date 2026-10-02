import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Tauri 前端：固定端口、退出清屏交给 Tauri CLI。
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
    watch: {
      // Rust 构建产物与 Tauri 后端不归前端 HMR 管；Windows 上 fs.watch
      // 监视被 cargo 锁定的 exe/dll 会同步抛 EBUSY 使 vite 崩溃。
      ignored: (path: string) => /[\\/]target[\\/]|[\\/]src-tauri[\\/]/.test(path),
    },
  },
  build: {
    target: "es2022",
  },
});
