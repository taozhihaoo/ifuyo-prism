/**
 * 轻量 i18n（M12 §240/§241 边界的落地）：只翻译 Prism 生成的界面标签；
 * 源码/文档/后端事实内容原样保留（§241）。无框架依赖（§0.1）。
 */

export type LocalizedText = { en: string; zh: string };

const DICT: Record<string, LocalizedText> = {
  // 导航分组（§7 IA）
  "nav.group.project": { en: "Project", zh: "项目" },
  "nav.group.change": { en: "Change", zh: "变更" },
  "nav.group.quality": { en: "Quality", zh: "质量" },
  "nav.group.aiContext": { en: "AI Context", zh: "AI 上下文" },
  "nav.group.history": { en: "History", zh: "历史" },
  "nav.group.system": { en: "System", zh: "系统" },
  // 页面名（导航/标题/调色板共用）。P1-3：任何两个条目的中文名不得相同
  // （组名「变更」+页名「变更」曾重名；组名保留、页名去重）。
  "page.overview": { en: "Overview", zh: "概览" },
  "page.understand": { en: "Understand", zh: "代码理解" },
  "page.changes": { en: "Change Analysis", zh: "变更分析" },
  "page.health": { en: "Health", zh: "健康" },
  "page.verify": { en: "Verify", zh: "验证" },
  "page.environment": { en: "Environment", zh: "环境" },
  "page.context": { en: "Context", zh: "上下文" },
  "page.reports": { en: "Snapshots", zh: "快照历史" },
  "page.settings": { en: "Settings", zh: "设置" },
  // 页头副标题
  "page.overview.subtitle": {
    en: "Project facts at a glance — evidence, not scores.",
    zh: "一眼看清项目事实——只呈现证据，不做评分。",
  },
  "page.understand.subtitle": {
    en: "Structure, symbols, dependencies and visualization.",
    zh: "结构、符号、依赖关系与可视化投影。",
  },
  "page.changes.subtitle": {
    en: "What changed in the working tree, and what it touches.",
    zh: "工作区变了什么，以及这些变化波及到哪里。",
  },
  "page.health.subtitle": {
    en: "Evidence-backed engineering findings with lifecycle tracking.",
    zh: "有证据支撑、可追溯生命周期的工程发现。",
  },
  "page.verify.subtitle": {
    en: "Plan first, then run build / test / lint on explicit intent.",
    zh: "先生成计划，再由用户显式执行构建 / 测试 / 检查。",
  },
  "page.environment.subtitle": {
    en: "Project requirements compared with real machine facts.",
    zh: "把项目要求与机器上的真实事实逐项对照。",
  },
  "page.context.subtitle": {
    en: "Risk-weighted, task-aware AI context packs — deterministic.",
    zh: "风险加权、任务感知的 AI 上下文包——确定性生成。",
  },
  "page.reports.subtitle": {
    en: "Engineering snapshots, timeline and comparison.",
    zh: "工程状态快照、时间线与差异对比。",
  },
  "page.settings.subtitle": {
    en: "Appearance and local, UI-only preferences.",
    zh: "外观设置与仅保存在本地的界面偏好。",
  },
  // 通用
  "palette.openProject": { en: "Open", zh: "打开" },
  "palette.placeholder": {
    en: "Search pages and projects…",
    zh: "搜索页面与项目…",
  },
  "palette.noMatches": { en: "No matches.", zh: "无匹配项。" },
  "palette.currentPage": { en: "current page", zh: "当前页面" },
  "palette.goToPage": { en: "go to page", zh: "跳转到页面" },
  "common.search": { en: "Search… Ctrl+K", zh: "搜索… Ctrl+K" },
  "common.searchLabel": { en: "Search", zh: "搜索" },
  "common.live": { en: "live", zh: "在线" },
  "common.loading": { en: "Loading…", zh: "加载中…" },
  "common.unknown": { en: "Unknown", zh: "未知" },
  // Shell / 状态栏
  "shell.tagline": { en: "LOCAL WORKBENCH", zh: "本地工程工作台" },
  "shell.localOnly": { en: "Local only · no telemetry", zh: "仅本地 · 无遥测" },
  "shell.offlineReady": { en: "Offline ready", zh: "离线可用" },
  "status.privacy": { en: "local-first · no telemetry", zh: "本地优先 · 无遥测" },
  "status.coreError": { en: "core unavailable:", zh: "核心服务不可用：" },
  "status.connecting": { en: "connecting to core…", zh: "正在连接核心服务…" },
  // 设置页
  "settings.appearance": { en: "Appearance", zh: "外观" },
  "settings.language": { en: "Language", zh: "语言" },
  "settings.theme": { en: "Theme", zh: "主题" },
  "settings.density": { en: "Density", zh: "密度" },
  "settings.theme.dark": { en: "Dark", zh: "深色" },
  "settings.theme.light": { en: "Light", zh: "浅色" },
  "settings.density.comfortable": { en: "Comfortable", zh: "舒适" },
  "settings.density.compact": { en: "Compact", zh: "紧凑" },
  "settings.storage": { en: "Storage", zh: "存储" },
  "settings.storageHint": {
    en: "Snapshot database location (local-only, §21):",
    zh: "快照数据库位置（仅本地，§21）：",
  },
  "settings.currentProject": { en: "Current project:", zh: "当前项目：" },
  "settings.uiOnly": {
    en: "UI-only preferences, stored locally (§48).",
    zh: "仅界面偏好，本地存储（§48）。",
  },
  "settings.exists": { en: "(exists)", zh: "（已创建）" },
  "settings.notCreated": { en: "(not created yet)", zh: "（尚未创建）" },
  // 历史页（§127–§133）
  "history.title": { en: "Snapshot History", zh: "快照历史" },
  "history.notBackups": {
    en: "Engineering state snapshots — not source backups (§177).",
    zh: "工程状态快照——不是源码备份（§177）。",
  },
  "history.compare": { en: "Compare A vs B", zh: "比较 A 与 B" },
  "history.captureNow": { en: "Capture now", zh: "立即捕获" },
  "history.capturing": { en: "Capturing…", zh: "捕获中…" },
  "history.timeline": { en: "Timeline", zh: "时间线" },
  "history.noEvents": { en: "No events.", zh: "暂无事件。" },
  "history.comparison": { en: "Comparison", zh: "比较结果" },
  "history.empty.title": { en: "Snapshot History", zh: "快照历史" },
  "history.empty.first": {
    en: "No snapshots for this project yet — this will be the first recorded Prism snapshot (§246).",
    zh: "该项目还没有快照——这将是第一条 Prism 快照记录（§246）。",
  },
  "history.empty.capture": {
    en: "Create first snapshot",
    zh: "创建第一条快照",
  },
  "history.state": { en: "state", zh: "状态" },
  "history.files": { en: "files", zh: "文件" },
  "history.findings": { en: "findings", zh: "发现" },
  // 概览状态卡（§14）
  "overview.status": { en: "Status", zh: "状态" },
  "overview.health": { en: "Health", zh: "健康检查" },
  "overview.verify": { en: "Verify", zh: "验证" },
  "overview.lastSnapshot": { en: "Last snapshot", zh: "最近快照" },
  "overview.notRun": { en: "not run", zh: "未运行" },
  "overview.none": { en: "none", zh: "无" },
  "overview.sessionNote": {
    en: "Session facts — Health/Verify reflect whether they ran in this session; snapshot is the latest capture time (§14).",
    zh: "会话事实——健康检查 / 验证反映本次会话是否运行过；快照为最近一次捕获时间（§14）。",
  },
  "overview.noProject": { en: "No project opened.", zh: "尚未打开项目。" },
  "overview.noProjectHint": {
    en: "Open a local project to begin. Prism scans it locally — nothing leaves your machine.",
    zh: "先打开一个本地项目。Prism 只在本地扫描，数据不会离开你的机器。",
  },
  "overview.openProject": { en: "Open Project", zh: "打开项目" },
  "overview.project": { en: "Project", zh: "项目" },
  "overview.name": { en: "Name", zh: "名称" },
  "overview.root": { en: "Root", zh: "根目录" },
  "overview.files": { en: "Files", zh: "文件" },
  "overview.directories": { en: "Directories", zh: "目录" },
  "overview.size": { en: "Size", zh: "大小" },
  "overview.scanDuration": { en: "Scan duration", zh: "扫描耗时" },
  "overview.scanCancelled": {
    en: "Scan was cancelled — partial results below.",
    zh: "扫描已取消——下方为部分结果。",
  },
  "overview.scanWarnings": {
    en: "Scan completed with warnings ({n}).",
    zh: "扫描完成，但有 {n} 条警告。",
  },
  "overview.verification": { en: "Verification", zh: "验证" },
  "overview.noVerificationRun": {
    en: "No verification run in this session.",
    zh: "本次会话尚未运行验证。",
  },
  "overview.verificationStale": {
    en: "Verification is stale —",
    zh: "验证结果已过期——",
  },
  "overview.staleHead": {
    en: "HEAD moved since the last verification.",
    zh: "自上次验证后 HEAD 已移动。",
  },
  "overview.staleWorktree": {
    en: "Working tree changed since the last verification.",
    zh: "自上次验证后工作区已变更。",
  },
  "overview.workingTreeChanges": {
    en: "{n} working-tree changes present at verification time.",
    zh: "验证时工作区存在 {n} 处变更。",
  },
  "overview.sessionOnly": {
    en: "Session facts only — nothing is claimed about past sessions.",
    zh: "仅呈现本次会话事实，不对历史会话作任何推断。",
  },
  "overview.languages": { en: "Languages", zh: "语言" },
  "overview.languageCount": { en: "{n} languages", zh: "{n} 种语言" },
  "overview.noLanguages": { en: "No recognized source files.", zh: "未识别到源码文件。" },
  "overview.fileCountNote": {
    en: "File counts, not lines of code.",
    zh: "统计的是文件数量，不是代码行数。",
  },
  "overview.entryPoints": { en: "Candidate entry points", zh: "候选入口" },
  "overview.noEntryPoints": {
    en: "No conventional entry points found.",
    zh: "未发现约定俗成的入口点。",
  },
  "overview.candidatesNote": {
    en: "Candidates with reasons — not a single verdict. See Understand for structure.",
    zh: "候选入口均附理由，而非单一结论。结构信息见「代码理解」。",
  },
  "overview.application": { en: "Application", zh: "应用信息" },
  "overview.version": { en: "Version", zh: "版本" },
  "overview.os": { en: "OS", zh: "操作系统" },
  "overview.arch": { en: "Arch", zh: "架构" },
  "overview.buildProfile": { en: "Build profile", zh: "构建配置" },
  "overview.generatedAt": { en: "Generated at", zh: "生成时间" },
  "overview.scanCancelledTitle": { en: "Scan cancelled", zh: "扫描已取消" },
  "overview.scanFailedTitle": { en: "Scan failed", zh: "扫描失败" },
  "overview.readyTitle": { en: "Ready to scan", zh: "可以开始扫描" },
  "overview.readyHint": {
    en: "Run a scan to discover this project's structure.",
    zh: "运行扫描以发现该项目的结构。",
  },
  "overview.scanFailedHint": {
    en: "The scan failed — see logs for details.",
    zh: "扫描失败——详情见日志。",
  },
  // 概览子卡片
  "projectTypes.title": { en: "Detected project types", zh: "识别到的项目类型" },
  "projectTypes.empty": {
    en: "No known project type signatures found.",
    zh: "未发现已知的项目类型特征。",
  },
  "projectTypes.note": {
    en: "Candidates are evidence-based, not a verdict.",
    zh: "这些是附证据的候选类型，不是单一结论。",
  },
  "fullAnalysis.title": { en: "Full analysis", zh: "全面分析" },
  "fullAnalysis.start": { en: "Run full analysis", zh: "全面分析" },
  "fullAnalysis.cancel": { en: "Cancel", zh: "取消" },
  "fullAnalysis.hint": {
    en: "Runs structure scan → code understanding → dependency graph → health in sequence. Cancelling keeps completed stages (§46).",
    zh: "依次执行 结构扫描 → 代码理解 → 依赖图 → 健康分析。中途取消会保留已完成阶段（§46）。",
  },
  "fullAnalysis.stage.scan": { en: "Structure scan", zh: "结构扫描" },
  "fullAnalysis.stage.inspect": { en: "Code understanding", zh: "代码理解" },
  "fullAnalysis.stage.graph": { en: "Dependency graph", zh: "依赖图" },
  "fullAnalysis.stage.health": { en: "Health analysis", zh: "健康分析" },
  "fullAnalysis.status.pending": { en: "pending", zh: "待执行" },
  "fullAnalysis.status.running": { en: "running…", zh: "运行中…" },
  "fullAnalysis.status.completed": { en: "completed", zh: "完成" },
  "fullAnalysis.status.failed": { en: "failed", zh: "失败" },
  "fullAnalysis.status.cancelled": { en: "cancelled", zh: "已取消" },
  "fullAnalysis.status.skipped": { en: "skipped", zh: "跳过" },
  "fullAnalysis.done": {
    en: "All stages completed · {ms}ms total — see each domain page for results.",
    zh: "全部完成 · 总耗时 {ms}ms——各域结果见对应页面。",
  },
  "fullAnalysis.cancelledNote": {
    en: "Cancelled — completed stages above are kept. You can run it again.",
    zh: "已取消——上方已完成阶段的结果保留，可重新运行。",
  },
  "intelligence.title": { en: "Intelligence summary (2.0)", zh: "Intelligence 摘要（2.0）" },
  "intelligence.refresh": { en: "Refresh", zh: "刷新" },
  "intelligence.loading": {
    en: "Projecting intelligence facts…",
    zh: "正在投影工程智能事实…",
  },
  "intelligence.error": {
    en: "Intelligence summary unavailable:",
    zh: "工程智能摘要不可用：",
  },
  "intelligence.health2": { en: "health2 findings", zh: "health2 发现" },
  "intelligence.health1": { en: "1.0 findings", zh: "1.0 发现" },
  "intelligence.godot": { en: "Godot scenes", zh: "Godot 场景" },
  "intelligence.deps": { en: "Dependencies", zh: "依赖" },
  "intelligence.units.critical": { en: "critical", zh: "严重" },
  "intelligence.units.warning": { en: "warning", zh: "警告" },
  "intelligence.units.info": { en: "info", zh: "提示" },
  "intelligence.units.scenes": { en: "scenes", zh: "场景" },
  "intelligence.units.nodes": { en: "nodes", zh: "节点" },
  "intelligence.units.resources": { en: "resources", zh: "资源" },
  "intelligence.units.declarations": { en: "declarations", zh: "声明" },
  "intelligence.units.resolved": { en: "resolved", zh: "已解析" },
  "intelligence.units.cycles": { en: "cycles", zh: "环" },
  "intelligence.note": {
    en: "Unknown = this domain has not run in the current session (Manual First §147). Full analysis lives on each domain page.",
    zh: "Unknown = 本次会话尚未运行该域（Manual First §147）——如实标注，不画成正常（§46）。完整分析入口在各域页面。",
  },
  // 数据完整度横幅
  "completeness.title": { en: "Data completeness", zh: "数据完整度" },
  "completeness.detailCount": { en: "{n} details", zh: "{n} 条明细" },
  "completeness.expand": { en: "Show {n} more", zh: "展开其余 {n} 条" },
  "completeness.collapse": { en: "Collapse", zh: "收起" },
  "completeness.complete": { en: "complete", zh: "完整" },
  "completeness.partial": { en: "partial", zh: "部分缺失" },
  "completeness.truncated": { en: "truncated", zh: "已截断" },
  "completeness.failed": { en: "failed", zh: "失败" },
  "completeness.unknown": { en: "unknown", zh: "未知" },
};

/** 按语言取翻译；未知键回退英文，再回退键名本身。 */
export function translate(language: "en" | "zh", key: string): string {
  const entry = DICT[key];
  if (!entry) return key;
  return language === "zh" ? entry.zh : entry.en;
}

/** 带占位符的插值：`format(t("key"), { n: 3 })` → 替换 `{n}`。 */
export function format(template: string, values: Record<string, string | number>): string {
  return template.replace(/\{(\w+)\}/g, (match, name: string) =>
    name in values ? String(values[name]) : match,
  );
}
