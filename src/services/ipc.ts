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
  ChangeSetDto,
  ChangesetStatusDto,
  HealthAnalysisDto,
  HealthStatusDto,
  VerifyPlanDto,
  VerificationReportDto,
  VerifyStatusDto,
  EnvironmentSnapshotDto,
  ContextPackDto,
  ImpactAnalysisDto,
  ImpactStatusDto,
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

// ---------- M5：ChangeSet ----------

export function startChangeset(
  scope: "working_tree" | "staged" | "revision_range",
  from?: string,
  to?: string,
  maxFiles?: number,
): Promise<string> {
  return invoke<string>("start_changeset", {
    scope,
    from: from ?? null,
    to: to ?? null,
    maxFiles: maxFiles ?? null,
  });
}

export function getChangesetStatus(): Promise<ChangesetStatusDto> {
  return invoke<ChangesetStatusDto>("get_changeset_status");
}

export function getChangeset(): Promise<ChangeSetDto | null> {
  return invoke<ChangeSetDto | null>("get_changeset");
}

export function cancelChangeset(): Promise<void> {
  return invoke<void>("cancel_changeset");
}

// ---------- M6：Impact Analysis ----------

export function startImpact(
  scope: "working_tree" | "staged" | "revision_range",
  from?: string,
  to?: string,
  maxDepth?: number,
  maxNodes?: number,
): Promise<string> {
  return invoke<string>("start_impact", {
    scope,
    from: from ?? null,
    to: to ?? null,
    maxDepth: maxDepth ?? null,
    maxNodes: maxNodes ?? null,
  });
}

export function getImpactStatus(): Promise<ImpactStatusDto> {
  return invoke<ImpactStatusDto>("get_impact_status");
}

export function getImpact(): Promise<ImpactAnalysisDto | null> {
  return invoke<ImpactAnalysisDto | null>("get_impact");
}

export function cancelImpact(): Promise<void> {
  return invoke<void>("cancel_impact");
}

// ---------- M7：Health ----------

export function startHealth(): Promise<string> {
  return invoke<string>("start_health");
}

export function getHealthStatus(): Promise<HealthStatusDto> {
  return invoke<HealthStatusDto>("get_health_status");
}

export function getHealthAnalysis(): Promise<HealthAnalysisDto | null> {
  return invoke<HealthAnalysisDto | null>("get_health_analysis");
}

export function cancelHealth(): Promise<void> {
  return invoke<void>("cancel_health");
}

// ---------- M8：Verify ----------

export function getVerifyPlan(): Promise<VerifyPlanDto> {
  return invoke<VerifyPlanDto>("verify_plan");
}

export function startVerify(checks?: string[], changed?: boolean): Promise<string> {
  return invoke<string>("start_verify", {
    checks: checks ?? null,
    changed: changed ?? null,
  });
}

export function getVerifyStatus(): Promise<VerifyStatusDto> {
  return invoke<VerifyStatusDto>("get_verify_status");
}

export interface VerifyResults {
  plan: VerifyPlanDto;
  report: VerificationReportDto | null;
}

export function getVerifyResults(): Promise<VerifyResults | null> {
  return invoke<VerifyResults | null>("get_verify_results");
}

export function cancelVerify(): Promise<void> {
  return invoke<void>("cancel_verify");
}

// ---------- M9：Environment ----------

export function startEnvironmentScan(allTools?: boolean): Promise<string> {
  return invoke<string>("start_environment_scan", { allTools: allTools ?? null });
}

export function getEnvironmentStatus(): Promise<{ status: string; error: string | null }> {
  return invoke<{ status: string; error: string | null }>("get_environment_status");
}

export function getEnvironmentResults(): Promise<EnvironmentSnapshotDto | null> {
  return invoke<EnvironmentSnapshotDto | null>("get_environment_results");
}

export function cancelEnvironmentScan(): Promise<void> {
  return invoke<void>("cancel_environment_scan");
}

// ---------- M10：Context ----------

export function startContextGeneration(options?: { task?: string; scope?: string; maxTokens?: number }): Promise<string> {
  return invoke<string>("start_context_generation", {
    task: options?.task ?? null,
    scope: options?.scope ?? null,
    maxTokens: options?.maxTokens ?? null,
  });
}

export function getContextStatus(): Promise<{ status: string; error: string | null }> {
  return invoke<{ status: string; error: string | null }>("get_context_status");
}

export function getContextResults(): Promise<ContextPackDto | null> {
  return invoke<ContextPackDto | null>("get_context_results");
}

export function cancelContextGeneration(): Promise<void> {
  return invoke<void>("cancel_context_generation");
}

export function exportContext(format: "json" | "markdown" | "text"): Promise<string> {
  return invoke<string>("export_context", { format });
}
