/**
 * Rust → 前端的 JSON 契约类型（M0 基础集）。
 *
 * 契约规则：
 * - 字段名与 Rust 侧 serde 序列化保持一致（snake_case）；
 * - 类型只描述稳定契约，不泄漏 Rust 实现细节；
 * - M0 采用手写镜像（数量极少）；契约规模变大后再引入自动生成（记录于 docs/development.md）。
 */

/** AppInfoService::AppInfo */
export interface AppInfo {
  name: string;
  version: string;
  os: string;
  arch: string;
  family: string;
  build_profile: string;
  generated_at: string;
}

/** AppInfoService::RuntimeInfo */
export interface RuntimeInfo {
  os: string;
  arch: string;
  family: string;
  build_profile: string;
  prism_version: string;
}

/** PrismErrorDto — 所有 IPC 错误的统一形态 */
export interface PrismErrorDto {
  code: string;
  message: string;
}

/** Prism TaskState（task.rs） */
export type TaskState = "queued" | "running" | "completed" | "failed" | "cancelled";

/** Prism Severity（纲领 §11：仅三级） */
export type Severity = "info" | "warning" | "critical";
