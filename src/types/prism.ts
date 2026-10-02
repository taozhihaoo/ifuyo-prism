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

// ---------- M2：Code Understanding 契约 ----------

export type SymbolKind =
  | "file"
  | "module"
  | "namespace"
  | "class"
  | "struct"
  | "interface"
  | "enum"
  | "function"
  | "method"
  | "constructor"
  | "property"
  | "field"
  | "variable"
  | "constant"
  | "parameter"
  | "type_alias"
  | "import"
  | "export"
  | "decorator"
  | "unknown";

export type SymbolVisibility =
  | "public"
  | "private"
  | "protected"
  | "internal"
  | "package"
  | "module"
  | "unknown";

export interface SourceLocationDto {
  line: number;
  column: number;
  byte_offset: number;
}

export interface SourceRangeDto {
  start: SourceLocationDto;
  end: SourceLocationDto;
}

export interface SymbolDto {
  id: string;
  name: string;
  qualified_name: string;
  kind: SymbolKind;
  language: string;
  file_id: string;
  parent_id: string | null;
  visibility: SymbolVisibility;
  range: SourceRangeDto;
  signature: string | null;
  documentation_present: boolean;
}

export type ParseStatus =
  | "successful"
  | "successful_with_diagnostics"
  | "failed"
  | "unsupported"
  | "skipped";

export interface ParseDiagnosticDto {
  severity: "info" | "warning" | "error";
  message: string;
  location: SourceLocationDto | null;
  code: string;
}

export interface ParseResultDto {
  file_id: string;
  relative_path: string;
  language: string | null;
  status: ParseStatus;
  symbols: SymbolDto[];
  diagnostics: ParseDiagnosticDto[];
  duration_ms: number;
  parser_version: string;
  reason: string | null;
}

export interface ParseSummaryDto {
  files_total: number;
  files_parsed: number;
  files_with_diagnostics: number;
  files_failed: number;
  files_unsupported: number;
  files_skipped: number;
  symbols_total: number;
  error_count: number;
  warning_count: number;
  duration_ms: number;
  status: "queued" | "running" | "completed" | "failed" | "cancelled";
}

export interface ProjectCodeModelDto {
  schema_version: number;
  project: ProjectInfo;
  files: FileInfoDto[];
  parse_results: ParseResultDto[];
  summary: ParseSummaryDto;
}

export interface InspectStatus {
  status: "idle" | "running" | "completed" | "failed" | "cancelled";
  files_total: number;
  files_done: number;
  symbols: number;
  errors: number;
  has_warnings: boolean;
  error: string | null;
}

// ---------- M3：Dependency & Architecture 契约 ----------

export type ModuleKindDto =
  | "project"
  | "package"
  | "namespace"
  | "file_module"
  | "external_package"
  | "builtin"
  | "unknown";

export interface ModuleDto {
  id: string;
  name: string;
  kind: ModuleKindDto;
  file_ids: string[];
  parent_id: string | null;
  language: string | null;
  path: string | null;
  evidence: Evidence[];
}

export type ResolutionStatusDto =
  | "resolved"
  | "ambiguous"
  | "unresolved"
  | "external"
  | "builtin"
  | "ignored"
  | "not_applicable";

export interface DependencyDto {
  id: string;
  source_id: string;
  source_path: string;
  raw_target: string;
  kind: string;
  resolution: ResolutionStatusDto;
  confidence: Confidence;
  target_id: string | null;
  target_path: string | null;
  imported_names: string[];
  location: SourceLocationDto | null;
  evidence: Evidence[];
}

export type GraphNodeKindDto =
  | "project"
  | "package"
  | "namespace"
  | "file_module"
  | "external_package"
  | "builtin_module";

export interface GraphNodeDto {
  id: string;
  kind: GraphNodeKindDto;
  label: string;
  entity: string;
  language: string | null;
}

export interface GraphEdgeDto {
  id: string;
  source: string;
  target: string;
  kind: string;
  resolution: ResolutionStatusDto;
  confidence: Confidence;
  evidence: Evidence[];
}

export interface CycleDto {
  id: string;
  nodes: string[];
}

export interface GraphMetricsDto {
  node_count: number;
  edge_count: number;
  resolved_edges: number;
  ambiguous_edges: number;
  unresolved_edges: number;
  external_edges: number;
  builtin_edges: number;
  cycle_count: number;
  connected_components: number;
  most_referenced: [string, number][];
  most_dependent: [string, number][];
}

export interface ResolutionDiagnosticDto {
  source_path: string;
  raw_target: string;
  resolver: string;
  status: ResolutionStatusDto;
  reason: string;
  location: SourceLocationDto | null;
}

export interface DependencyGraphResultDto {
  schema_version: number;
  project_name: string;
  modules: ModuleDto[];
  dependencies: DependencyDto[];
  nodes: GraphNodeDto[];
  edges: GraphEdgeDto[];
  calls: CallEdgeDto[];
  cycles: CycleDto[];
  metrics: GraphMetricsDto;
  diagnostics: ResolutionDiagnosticDto[];
}

export interface GraphStatus {
  status: "idle" | "running" | "completed" | "failed" | "cancelled";
  node_count: number;
  edge_count: number;
  cycle_count: number;
  unresolved_count: number;
  error: string | null;
}

export interface CallEdgeDto {
  id: string;
  caller_id: string;
  caller_qualified: string;
  callee_text: string;
  callee_name: string;
  callee_id: string | null;
  callee_qualified: string | null;
  resolution: ResolutionStatusDto;
  confidence: Confidence;
  location: SourceLocationDto | null;
}
