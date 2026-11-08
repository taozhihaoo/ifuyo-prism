/**
 * 仅用于本地 UI 预览/截图的最小 IPC 替身（不参与生产构建）。
 *
 * 背景：Prism 的真正数据来自 Rust Core（Tauri IPC）。在浏览器里跑 Vite
 * 时没有 `__TAURI_INTERNALS__`，页面只能停在空状态，无法用于文档截图。
 * 本模块在 `demo.html` 入口里安装一个返回**固定演示数据**的 invoke shim，
 * 让同一套 React 组件在浏览器中可完整渲染。
 *
 * 约束：
 * - 只由 `src/demo.tsx` 调用；`vite build` 只打包 index.html，因此不会
 *   进入发布产物。
 * - 数据全部是常量，不读文件、不联网、不写存储（除 UI 语言偏好）。
 */

type Handler = (args: Record<string, unknown>) => unknown;

const COMMIT = "9f3c1a2b7d4e5f60718293a4b5c6d7e8f9012345";
const CAPTURED_AT = "2026-11-07T18:42:00Z";

const PROJECT = {
  id: "proj-sandbook",
  name: "sandbook",
  root_path: "E:/WorkSpace/sandbook",
  discovered_at: "2026-11-07T18:41:00Z",
};

const DISCOVERY = {
  schema_version: 1,
  project: PROJECT,
  scan: {
    scan_id: "scan-demo-1",
    root_path: PROJECT.root_path,
    started_at: "2026-11-07T18:41:12Z",
    finished_at: "2026-11-07T18:41:14Z",
    duration_ms: 1840,
    files_scanned: 3128,
    directories_scanned: 214,
    files_ignored: 1180,
    errors_count: 0,
    status: "completed",
  },
  files: [],
  directories: [],
  languages: [
    { language: "C#", count: 612 },
    { language: "TypeScript", count: 188 },
    { language: "Rust", count: 96 },
    { language: "GDScript", count: 74 },
  ],
  project_types: [
    {
      name: "Godot 4",
      confidence: "high",
      evidence: [
        {
          source: "project_type_detector",
          path: "project.godot",
          reason: "Godot project file found",
        },
        {
          source: "project_type_detector",
          path: "SandBook.csproj",
          reason: ".NET C# project file",
        },
      ],
    },
    {
      name: "Node.js",
      confidence: "medium",
      evidence: [
        {
          source: "project_type_detector",
          path: "package.json",
          reason: "manifest found",
        },
      ],
    },
  ],
  configurations: [],
  entry_points: [
    {
      path: "Scripts/Main.gd",
      kind: "application",
      confidence: "high",
      evidence: [
        {
          source: "entry_point_detector",
          path: "Scripts/Main.gd",
          reason: "conventional application entry",
        },
      ],
    },
    {
      path: "SandBook.csproj",
      kind: "build",
      confidence: "high",
      evidence: [
        {
          source: "entry_point_detector",
          path: "SandBook.csproj",
          reason: "build manifest",
        },
      ],
    },
  ],
  statistics: {
    total_files: 3128,
    total_directories: 214,
    total_size_bytes: 486_000_000,
    source_files: 970,
    test_files: 167,
    config_files: 42,
    documentation_files: 38,
    asset_files: 1420,
    binary_files: 74,
    generated_files: 12,
    unknown_files: 405,
    ignored_files: 1180,
    error_count: 0,
  },
  errors: [],
};

const REPORT = {
  schema_version: 1,
  project_name: "sandbook",
  plan: { checks: [] },
  runs: [
    {
      run_id: "run-build",
      check_id: "build",
      kind: "build",
      name: "build",
      required: true,
      state: "passed",
      command_display: "dotnet build SandBook.csproj",
      working_directory: ".",
      started_at: null,
      finished_at: null,
      duration_ms: 8420,
      exit_code: 0,
      stdout: "",
      stderr: "",
      output_truncated: false,
      diagnostics: [],
      reason: null,
      side_effects: [],
    },
    {
      run_id: "run-test",
      check_id: "test",
      kind: "test",
      name: "test",
      required: true,
      state: "passed",
      command_display: "dotnet test",
      working_directory: ".",
      started_at: null,
      finished_at: null,
      duration_ms: 21400,
      exit_code: 0,
      stdout: "",
      stderr: "",
      output_truncated: false,
      diagnostics: [],
      reason: null,
      side_effects: [],
    },
    {
      run_id: "run-lint",
      check_id: "lint",
      kind: "lint",
      name: "lint",
      required: false,
      state: "passed",
      command_display: "dotnet format --verify-no-changes",
      working_directory: ".",
      started_at: null,
      finished_at: null,
      duration_ms: 3100,
      exit_code: 0,
      stdout: "",
      stderr: "",
      output_truncated: false,
      diagnostics: [],
      reason: null,
      side_effects: [],
    },
  ],
  conclusion: "pass",
  completeness: "complete",
  counts: {
    passed: 3,
    failed: 0,
    unavailable: 0,
    blocked: 0,
    skipped: 0,
    cancelled: 0,
    timed_out: 0,
    error: 0,
  },
  side_effects: [],
  impacted_test_candidates: [],
  verified_state: { kind: "clean_at_commit", commit: COMMIT, changed_files: 0 },
};

const HANDLERS: Record<string, unknown | Handler> = {
  get_app_info: {
    name: "Prism",
    version: "0.1.0",
    os: "windows",
    arch: "x86_64",
    family: "windows",
    build_profile: "release",
    generated_at: "2026-11-07T00:00:00Z",
  },
  get_prism_version: "0.1.0",
  get_runtime_info: {
    platform: "windows",
    arch: "x86_64",
    family: "windows",
    prism_version: "0.1.0",
  },
  list_recent_projects: [
    {
      id: "proj-sandbook",
      display_name: "sandbook",
      canonical_root: PROJECT.root_path,
      last_seen_at: CAPTURED_AT,
    },
  ],
  get_current_project_path: { root: PROJECT.root_path },
  get_storage_info: {
    db_path: "C:/Users/demo/AppData/Roaming/ifuyo-prism/prism.db",
    db_exists: true,
  },
  "plugin:dialog|open": PROJECT.root_path,
  open_project: PROJECT,
  start_scan: "task-scan",
  start_inspect: "task-inspect",
  start_graph: "task-graph",
  start_health: "task-health",
  start_health2: "task-health2",
  start_full_analysis: "task-full",
  start_changeset: "task-changes",
  start_impact: "task-impact",
  start_context_generation: "task-context",
  start_environment_scan: "task-env",
  cancel_scan: null,
  cancel_inspect: null,
  cancel_health: null,
  cancel_health2: null,
  cancel_full_analysis: null,
  cancel_verify: null,
  terminate: null,
  get_scan_status: {
    project: PROJECT,
    status: "completed",
    directories: 214,
    files: 3128,
    ignored: 1180,
    errors: 0,
    has_warnings: false,
    error: null,
  },
  get_discovery: DISCOVERY,
  get_inspect_status: {
    status: "completed",
    files_total: 970,
    files_done: 970,
    symbols: 18420,
    errors: 0,
    has_warnings: false,
    error: null,
  },
  get_code_model: null,
  get_graph_status: {
    status: "completed",
    node_count: 412,
    edge_count: 1180,
    cycle_count: 2,
    unresolved_count: 14,
    error: null,
  },
  get_dependency_graph: null,
  get_git_facts: {
    repository: { path: PROJECT.root_path, worktree: PROJECT.root_path },
    head: {
      kind: "branch",
      commit_id: COMMIT,
      branch_name: "main",
      symbolic_ref: "refs/heads/main",
    },
    status: { repository: null, head: null, entries: [], ignored: [] },
    recent_commits: [],
    refs: [],
  },
  get_verify_plan: { checks: [] },
  get_verify_status: { status: "completed", error: null },
  get_verify_results: { plan: { checks: [] }, report: REPORT },
  get_health_status: { status: "completed", error: null },
  get_health_analysis: null,
  get_health2_status: { status: "completed", error: null },
  get_health2_results: null,
  get_evolution_report: null,
  get_full_analysis_status: { status: "idle", stages: [], error: null },
  get_environment_status: { status: "idle", error: null },
  get_environment_results: null,
  get_context_status: { status: "idle", error: null },
  get_context_results: null,
  get_changeset_status: { status: "idle", error: null },
  get_changeset: null,
  get_impact_status: { status: "idle", error: null },
  get_impact: null,
  snapshot_history: [
    {
      id: "snap-1",
      project_id: PROJECT.id,
      captured_at: CAPTURED_AT,
      state: "complete",
      file_count: 3128,
      finding_count: 19,
    },
  ],
  snapshot_timeline: { events: [] },
  get_cached_domain: null,
  get_intelligence_summary: {
    health2: { critical: 2, warning: 9, info: 14 },
    health: 7,
    godot: { scenes: 148, nodes: 1240, resources: 612 },
    deps: { declarations: 96, resolved: 91, cycles: 2 },
  },
  get_visualization_graph: {
    schema_version: 1,
    view: "architecture",
    status: "complete",
    nodes: [],
    edges: [],
    legend: [],
    truncated: false,
    diagnostics: [],
  },
  register_project_open: null,
};

/** 读取 `?lang=zh|en` 并写入与设置页一致的 localStorage 键。 */
export function applyDemoLanguage(): void {
  const params = new URLSearchParams(window.location.search);
  const language = params.get("lang") === "zh" ? "zh" : "en";
  const theme = params.get("theme") === "light" ? "light" : "dark";
  try {
    window.localStorage.setItem(
      "prism.ui.settings",
      JSON.stringify({ theme, density: "comfortable", language }),
    );
  } catch {
    /* localStorage 不可用时用默认值 */
  }
  document.documentElement.lang = language === "zh" ? "zh-CN" : "en";
}

export function installDemoIpc(): void {
  const invoke = async (cmd: string, args?: Record<string, unknown>) => {
    const entry = HANDLERS[cmd];
    if (typeof entry === "function") return (entry as Handler)(args ?? {});
    if (entry === undefined) return null;
    return entry;
  };

  const internals = {
    invoke,
    transformCallback: (callback: unknown) => callback,
    convertFileSrc: (path: string) => path,
  };
  (window as unknown as { __TAURI_INTERNALS__: typeof internals }).__TAURI_INTERNALS__ =
    internals;
}
