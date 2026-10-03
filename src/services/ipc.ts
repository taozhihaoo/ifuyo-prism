/**
 * IPC 服务层：前端访问 Core 的唯一通道。
 *
 * 页面组件不得直接 import `@tauri-apps/api/core`，
 * 必须经由本模块（便于 mock、错误归一化与未来替换传输层）。
 */
import { invoke } from "@tauri-apps/api/core";
import type {
  AppInfo,
  DependencyGraphResultDto,
  GitFactsDto,
  GraphStatus,
  InspectStatus,
  PrismErrorDto,
  ProjectCodeModelDto,
  ProjectDiscovery,
  ProjectInfo,
  RuntimeInfo,
  ScanStatus,
} from "../types/prism";

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

// ---------- M1：Project Discovery ----------

export function openProject(path: string): Promise<ProjectInfo> {
  return invoke<ProjectInfo>("open_project", { path });
}

/** 启动扫描，返回 task id；进度经 getScanStatus 轮询。 */
export function startScan(): Promise<string> {
  return invoke<string>("start_scan");
}

export function cancelScan(): Promise<void> {
  return invoke<void>("cancel_scan");
}

export function getScanStatus(): Promise<ScanStatus> {
  return invoke<ScanStatus>("get_scan_status");
}

export function getDiscovery(): Promise<ProjectDiscovery | null> {
  return invoke<ProjectDiscovery | null>("get_discovery");
}

/** 系统目录选择对话框（tauri-plugin-dialog）。取消返回 null。 */
export async function pickDirectory(): Promise<string | null> {
  const { open } = await import("@tauri-apps/plugin-dialog");
  const selected = await open({ directory: true, multiple: false, title: "Open project" });
  return typeof selected === "string" ? selected : null;
}

// ---------- M2：Code Understanding ----------

export function startInspect(): Promise<string> {
  return invoke<string>("start_inspect");
}

export function cancelInspect(): Promise<void> {
  return invoke<void>("cancel_inspect");
}

export function getInspectStatus(): Promise<InspectStatus> {
  return invoke<InspectStatus>("get_inspect_status");
}

export function getCodeModel(): Promise<ProjectCodeModelDto | null> {
  return invoke<ProjectCodeModelDto | null>("get_code_model");
}

// ---------- M3：Dependency & Architecture ----------

export function startGraph(): Promise<string> {
  return invoke<string>("start_graph");
}

export function getGraphStatus(): Promise<GraphStatus> {
  return invoke<GraphStatus>("get_graph_status");
}

export function getDependencyGraph(): Promise<DependencyGraphResultDto | null> {
  return invoke<DependencyGraphResultDto | null>("get_dependency_graph");
}

// ---------- M4：Git facts ----------

export function getGitFacts(includeIgnored: boolean): Promise<GitFactsDto> {
  return invoke<GitFactsDto>("get_git_facts", { includeIgnored });
}
