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
  // 页面名（导航/标题/调色板共用）
  "page.overview": { en: "Overview", zh: "概览" },
  "page.understand": { en: "Understand", zh: "代码理解" },
  "page.changes": { en: "Changes", zh: "变更" },
  "page.health": { en: "Health", zh: "健康" },
  "page.verify": { en: "Verify", zh: "验证" },
  "page.environment": { en: "Environment", zh: "环境" },
  "page.context": { en: "Context", zh: "上下文" },
  "page.history": { en: "History", zh: "历史" },
  "page.reports": { en: "History", zh: "历史" },
  "page.settings": { en: "Settings", zh: "设置" },
  // 通用
  "palette.openProject": { en: "Open", zh: "打开" },
  "palette.placeholder": {
    en: "Search pages and projects…",
    zh: "搜索页面与项目…",
  },
  "palette.noMatches": { en: "No matches.", zh: "无匹配项。" },
  "common.search": { en: "Search… Ctrl+K", zh: "搜索… Ctrl+K" },
  "common.loading": { en: "Loading…", zh: "加载中…" },
  // 设置页
  "settings.appearance": { en: "Appearance", zh: "外观" },
  "settings.language": { en: "Language", zh: "语言" },
  "settings.theme": { en: "Theme", zh: "主题" },
  "settings.density": { en: "Density", zh: "密度" },
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
    zh: "会话事实——Health/Verify 反映本次会话是否运行过；快照为最近一次捕获时间（§14）。",
  },
};

/** 按语言取翻译；未知键回退英文，再回退键名本身。 */
export function translate(language: "en" | "zh", key: string): string {
  const entry = DICT[key];
  if (!entry) return key;
  return language === "zh" ? entry.zh : entry.en;
}
