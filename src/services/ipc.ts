/**
 * IPC 服务层：前端访问 Core 的唯一通道。
 *
 * 页面组件不得直接 import `@tauri-apps/api/core`，
 * 必须经由本模块（便于 mock、错误归一化与未来替换传输层）。
 */
import { invoke } from "@tauri-apps/api/core";
import type { AppInfo, PrismErrorDto, RuntimeInfo } from "../types/prism";

/** 把 IPC rejection 归一化为 PrismErrorDto（对应 Rust PrismErrorDto）。 */
export function toPrismError(err: unknown): PrismErrorDto {
  if (typeof err === "object" && err !== null) {
    const candidate = err as { code?: unknown; message?: unknown };
    if (typeof candidate.code === "string" && typeof candidate.message === "string") {
      return { code: candidate.code, message: candidate.message };
    }
  }
  return { code: "internal", message: String(err) };
}

export function getAppInfo(): Promise<AppInfo> {
  return invoke<AppInfo>("get_app_info");
}

export function getPrismVersion(): Promise<string> {
  return invoke<string>("get_prism_version");
}

export function getRuntimeInfo(): Promise<RuntimeInfo> {
  return invoke<RuntimeInfo>("get_runtime_info");
}
