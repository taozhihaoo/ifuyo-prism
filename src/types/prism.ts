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

// ---------- M1：Project Discovery 契约 ----------

/** prism_project::Project */
export interface ProjectInfo {
  id: string;
  name: string;
  root_path: string;
  discovered_at: string;
}

/** 分类置信度（分类式，非百分比） */
export type Confidence = "low" | "medium" | "high";

/** FileKind（分类，与语言正交） */
export type FileKind =
  | "source"
  | "test"
  | "config"
  | "documentation"
  | "asset"
  | "generated"
  | "binary"
  | "unknown";

export interface Evidence {
  source: string;
  path: string;
  reason: string;
}

export interface FileClassification {
  kind: FileKind;
  confidence: Confidence;
  reason: string;
}

export interface FileInfoDto {
  id: string;
  relative_path: string;
  file_name: string;
  extension: string | null;
  size_bytes: number;
  language: string | null;
  classification: FileClassification;
  analyzable: boolean;
  non_analyzable_reason: string | null;
  modified_at: string | null;
  is_hidden: boolean;
  is_symlink: boolean;
}

export interface DirectoryInfoDto {
  id: string;
  relative_path: string;
  name: string;
  depth: number;
  is_hidden: boolean;
}

export interface LanguageCount {
  language: string;
  count: number;
}

export interface ProjectTypeCandidate {
  name: string;
  confidence: Confidence;
  evidence: Evidence[];
}

export interface ConfigurationInfoDto {
  name: string;
  path: string;
  kind: string;
  confidence: Confidence;
}

export interface EntryPointDto {
  path: string;
  kind: string;
  confidence: Confidence;
  evidence: Evidence[];
}

export interface ProjectStatisticsDto {
  total_files: number;
  total_directories: number;
  total_size_bytes: number;
  source_files: number;
  test_files: number;
  config_files: number;
  documentation_files: number;
  asset_files: number;
  binary_files: number;
  generated_files: number;
  unknown_files: number;
  ignored_files: number;
  error_count: number;
}

export interface ScanErrorDto {
  path: string;
  kind: string;
  message: string;
}

export interface ScanInfoDto {
  scan_id: string;
  root_path: string;
  started_at: string;
  finished_at: string | null;
  duration_ms: number;
  files_scanned: number;
  directories_scanned: number;
  files_ignored: number;
  errors_count: number;
  status: "queued" | "running" | "completed" | "failed" | "cancelled";
}

export interface ProjectDiscovery {
  schema_version: number;
  project: ProjectInfo;
  scan: ScanInfoDto;
  files: FileInfoDto[];
  directories: DirectoryInfoDto[];
  languages: LanguageCount[];
  project_types: ProjectTypeCandidate[];
  configurations: ConfigurationInfoDto[];
  entry_points: EntryPointDto[];
  statistics: ProjectStatisticsDto;
  errors: ScanErrorDto[];
}

/** get_scan_status 的返回（实时轮询） */
export interface ScanStatus {
  project: ProjectInfo | null;
  status: "idle" | "queued" | "running" | "completed" | "failed" | "cancelled";
  directories: number;
  files: number;
  ignored: number;
  errors: number;
  has_warnings: boolean;
  error: string | null;
}
