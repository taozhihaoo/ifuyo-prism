//! Context 模型（规格 M10a §7–§9/§19–§25/§31–§42/§114–§116/§138–§153/
//! §210–§220/§318/§372–§373/§384–§388/§418–§428）。
//!
//! 核心红线：无神秘浮点分（§24/§27/§143–§145，只有 ordinal 优先级带）、
//! 无 LLM 依赖（§253/§364）、ContextPack 只 in-memory（§9）、
//! 每个 item 必须能回答"为什么我在这里"（§22）。

/// Context manifest/pack schema 版本（§217/§292 思路）。
pub const CONTEXT_SCHEMA_VERSION: u32 = 1;

use serde::{Deserialize, Serialize};

// ---------- 请求（§10–§18） ----------

/// Context 请求（§10）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextRequest {
    pub project_root: String,
    /// 自由文本任务；None ⇒ Generic overview（§18）。
    pub task: Option<String>,
    pub scope: ContextScope,
    /// 显式目标（file/symbol/module/commit 文本，§15）。
    pub target: Option<String>,
    pub budget: ContextBudget,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextScope {
    /// §83：按 task + changes + project state 自动选择。
    #[default]
    Auto,
    /// §84：只围绕当前 ChangeSet/Impact。
    CurrentChange,
    /// §85：围绕显式 file/symbol/module。
    Target,
    /// §86：更广（budget 仍生效）。
    Full,
}

pub fn scope_str(scope: ContextScope) -> &'static str {
    match scope {
        ContextScope::Auto => "auto",
        ContextScope::CurrentChange => "current_change",
        ContextScope::Target => "target",
        ContextScope::Full => "full",
    }
}

/// 任务分类（§12/§13：允许多 kind）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskKind {
    Change,
    BugInvestigation,
    Refactor,
    Feature,
    Build,
    Test,
    Ci,
    Architecture,
    Security,
    Documentation,
    Release,
    Environment,
    Review,
    Unknown,
}

impl TaskKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            TaskKind::Change => "change",
            TaskKind::BugInvestigation => "bug_investigation",
            TaskKind::Refactor => "refactor",
            TaskKind::Feature => "feature",
            TaskKind::Build => "build",
            TaskKind::Test => "test",
            TaskKind::Ci => "ci",
            TaskKind::Architecture => "architecture",
            TaskKind::Security => "security",
            TaskKind::Documentation => "documentation",
            TaskKind::Release => "release",
            TaskKind::Environment => "environment",
            TaskKind::Review => "review",
            TaskKind::Unknown => "unknown",
        }
    }
}

/// 任务关注面（§14）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextFocus {
    Code,
    Architecture,
    Changes,
    Impact,
    Health,
    Verification,
    Environment,
    Documentation,
    Git,
    Configuration,
    Tests,
    Ci,
}

/// 任务具体性（§18：Unknown task ⇒ Generic overview + 明确声明）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskSpecificity {
    Specific,
    Generic,
}

/// 确定性任务分类结果（§5/§12：无 LLM）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskClassification {
    pub kinds: Vec<TaskKind>,
    pub focus: Vec<ContextFocus>,
    pub specificity: TaskSpecificity,
    /// 未解析出的自然语言原样保留（§17 parsing boundary）。
    pub raw_task: Option<String>,
}

/// 目标解析（§16：Resolved/Ambiguous 不猜）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextTarget {
    pub raw: String,
    pub resolution: TargetResolution,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TargetResolution {
    /// 唯一命中。
    Resolved {
        paths: Vec<String>,
        symbols: Vec<String>,
    },
    /// 多个候选 ⇒ Ambiguous，全部列出（§16 不猜）。
    Ambiguous { candidates: Vec<String> },
    /// 提到了但项目里找不到。
    NotFound,
    /// 任务没有提取出目标。
    None,
}

// ---------- 来源与候选（§19–§20/§88） ----------

/// 来源类型（§20）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceType {
    File,
    Symbol,
    Module,
    Dependency,
    ArchitectureNode,
    GitCommit,
    GitDiff,
    ChangeSet,
    ImpactAnalysis,
    HealthFinding,
    CheckRun,
    EnvironmentFact,
    Documentation,
    Configuration,
    ProjectMetadata,
}

impl SourceType {
    pub const fn as_str(self) -> &'static str {
        match self {
            SourceType::File => "file",
            SourceType::Symbol => "symbol",
            SourceType::Module => "module",
            SourceType::Dependency => "dependency",
            SourceType::ArchitectureNode => "architecture_node",
            SourceType::GitCommit => "git_commit",
            SourceType::GitDiff => "git_diff",
            SourceType::ChangeSet => "change_set",
            SourceType::ImpactAnalysis => "impact_analysis",
            SourceType::HealthFinding => "health_finding",
            SourceType::CheckRun => "check_run",
            SourceType::EnvironmentFact => "environment_fact",
            SourceType::Documentation => "documentation",
            SourceType::Configuration => "configuration",
            SourceType::ProjectMetadata => "project_metadata",
        }
    }
}

/// 统一来源描述（§19）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextSource {
    pub source_type: SourceType,
    /// 稳定来源 id（沿用 M2 SymbolId / M3 ModuleId / M7 FindingId /
    /// M8 run 实例；文件类 = 项目相对路径，§93–§97）。
    pub source_id: String,
    /// 项目相对路径（适用时）。
    pub path: Option<String>,
    /// 位置（"L12-L40" / "heading:Build" 等）。
    pub location: Option<String>,
    /// 出处说明（"M5 ChangeSet" / "task target" / "M6 ImpactAnalysis"）。
    pub provenance: String,
}

// ---------- 优先级与理由（§24–§27/§143–§145/§372–§373） ----------

/// Ordinal 优先级带（§24/§25/§144）：无浮点、无 magic weights。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Priority {
    /// 显式目标 / review 任务的当前 diff / 失败且相关的 check。
    Critical,
    /// 直接变更 / 直接影响。
    High,
    /// 强依赖 / 相关测试 / 相关告警。
    Normal,
    /// 约定文档 / 架构概览 / 环境。
    Supporting,
    /// 远距离依赖。
    Low,
}

impl Priority {
    pub const fn as_str(self) -> &'static str {
        match self {
            Priority::Critical => "critical",
            Priority::High => "high",
            Priority::Normal => "normal",
            Priority::Supporting => "supporting",
            Priority::Low => "low",
        }
    }
}

/// 入选理由（§22/§372：可多理由并存 §92）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SelectionReason {
    ExplicitTaskTarget,
    DirectlyChanged,
    DirectlyImpacted,
    ReverseDependency,
    Dependency,
    RelatedTest,
    FailedVerification,
    HealthFinding,
    RelevantDocumentation,
    ProjectInstruction,
    RequiredEnvironment,
    TaskMatch,
    ArchitectureOverview,
    CurrentDiff,
}

impl SelectionReason {
    pub const fn as_str(self) -> &'static str {
        match self {
            SelectionReason::ExplicitTaskTarget => "explicit_task_target",
            SelectionReason::DirectlyChanged => "directly_changed",
            SelectionReason::DirectlyImpacted => "directly_impacted",
            SelectionReason::ReverseDependency => "reverse_dependency",
            SelectionReason::Dependency => "dependency",
            SelectionReason::RelatedTest => "related_test",
            SelectionReason::FailedVerification => "failed_verification",
            SelectionReason::HealthFinding => "health_finding",
            SelectionReason::RelevantDocumentation => "relevant_documentation",
            SelectionReason::ProjectInstruction => "project_instruction",
            SelectionReason::RequiredEnvironment => "required_environment",
            SelectionReason::TaskMatch => "task_match",
            SelectionReason::ArchitectureOverview => "architecture_overview",
            SelectionReason::CurrentDiff => "current_diff",
        }
    }
}

/// 排除理由（§141/§373/§147）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExcludedReason {
    OutsideScope,
    Duplicate,
    LowPriority,
    Generated,
    Vendor,
    BudgetExceeded,
    Unavailable,
    PrivacyRestricted,
}

impl ExcludedReason {
    pub const fn as_str(self) -> &'static str {
        match self {
            ExcludedReason::OutsideScope => "outside_scope",
            ExcludedReason::Duplicate => "duplicate",
            ExcludedReason::LowPriority => "low_priority",
            ExcludedReason::Generated => "generated",
            ExcludedReason::Vendor => "vendor",
            ExcludedReason::BudgetExceeded => "budget_exceeded",
            ExcludedReason::Unavailable => "unavailable",
            ExcludedReason::PrivacyRestricted => "privacy_restricted",
        }
    }
}

// ---------- 内容级别与范围（§38–§42/§152–§153） ----------

/// 文件内容四级策略（§40）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileContentLevel {
    Full,
    RelevantSymbols,
    Skeleton,
    MetadataOnly,
}

/// Context detail（§41/§42：代码默认 Focused，远距离 Structural/Metadata）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DetailLevel {
    Full,
    Focused,
    Structural,
    Metadata,
}

/// 选中范围（§152/§153：一个文件只出现一次，内部多 range）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextRange {
    /// 1-based 起止行。
    pub start: u64,
    pub end: u64,
    pub reason: SelectionReason,
    /// M2 SymbolId（适用时）。
    pub symbol_id: Option<String>,
    /// 符号名（header 呈现用）。
    pub symbol_name: Option<String>,
}

// ---------- 预算（§31–§35/§198–§201） ----------

/// Context 预算（§31；启发式 token 估算明确为 estimate §32/§388）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextBudget {
    pub max_items: usize,
    pub max_source_files: usize,
    pub max_bytes: u64,
    pub max_tokens_estimate: u64,
}

impl Default for ContextBudget {
    fn default() -> Self {
        ContextBudget {
            max_items: 64,
            max_source_files: 24,
            max_bytes: 512 * 1024,
            max_tokens_estimate: 96 * 1024 / 4, // 96 KiB ≈ 24k tokens（§32 启发式）
        }
    }
}

/// 预算消耗报告（§34/§386/§387）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BudgetReport {
    pub selected_items: usize,
    pub excluded_items: usize,
    pub used_bytes: u64,
    pub used_tokens_estimate: u64,
    pub limit_tokens_estimate: u64,
}

// ---------- 分组（§114–§116） ----------

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextGroup {
    Project,
    CurrentChange,
    TargetCode,
    Dependencies,
    Impact,
    Tests,
    Health,
    Verification,
    Environment,
    Documentation,
}

impl ContextGroup {
    pub const fn as_str(self) -> &'static str {
        match self {
            ContextGroup::Project => "project",
            ContextGroup::CurrentChange => "current_change",
            ContextGroup::TargetCode => "target_code",
            ContextGroup::Dependencies => "dependencies",
            ContextGroup::Impact => "impact",
            ContextGroup::Tests => "tests",
            ContextGroup::Health => "health",
            ContextGroup::Verification => "verification",
            ContextGroup::Environment => "environment",
            ContextGroup::Documentation => "documentation",
        }
    }

    /// §115/§116：默认推荐顺序（policy 可按 task 调整——M10b recipes 细化）。
    pub const fn default_rank(self) -> u8 {
        match self {
            ContextGroup::Project => 0,
            ContextGroup::CurrentChange => 1,
            ContextGroup::TargetCode => 2,
            ContextGroup::Dependencies => 3,
            ContextGroup::Impact => 4,
            ContextGroup::Tests => 5,
            ContextGroup::Health => 6,
            ContextGroup::Verification => 7,
            ContextGroup::Environment => 8,
            ContextGroup::Documentation => 9,
        }
    }
}

// ---------- Item / Selection / Pack ----------

/// 上下文条目（§21/§111/§112：Header/Metadata/Content/Reason 齐备；
/// content 由 materializer **懒加载**填充 §202）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextItem {
    /// 去重身份（§93：`ctx:{source_type}:{path}#{symbol_or_location}`）。
    pub id: String,
    pub source: ContextSource,
    pub group: ContextGroup,
    pub title: String,
    /// 懒物化后的内容；MetadataOnly/读取失败 ⇒ None。
    pub content: Option<String>,
    pub location: Option<String>,
    pub priority: Priority,
    /// 多理由保留（§92）。
    pub reasons: Vec<SelectionReason>,
    pub evidence: Vec<String>,
    /// §112 metadata（compact，§113）。
    pub language: Option<String>,
    pub change_state: Option<String>,
    pub impact_distance: Option<u32>,
    /// §35/§385：截断事实。
    pub truncated: bool,
    pub original_size: Option<u64>,
    pub included_size: Option<u64>,
    pub truncation_reason: Option<String>,
    pub content_level: FileContentLevel,
    pub detail_level: DetailLevel,
    pub ranges: Vec<ContextRange>,
    /// §326/§329：不可用/不稳定标记。
    pub unavailable_reason: Option<String>,
    /// §218/§219：redacted content 的 hash（secret-safe）。
    pub content_hash: Option<String>,
}

/// 候选（§87–§89：collector 产物，未排序/未物化）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextCandidate {
    pub source: ContextSource,
    pub group: ContextGroup,
    pub title: String,
    pub reasons: Vec<SelectionReason>,
    pub evidence: Vec<String>,
    pub impact_distance: Option<u32>,
    pub change_state: Option<String>,
    pub language: Option<String>,
    /// M2 符号行范围（materializer 切片用）。
    pub symbol_range: Option<(u64, u64)>,
    pub symbol_id: Option<String>,
    pub symbol_name: Option<String>,
    pub severity: Option<String>,
    pub change_type: Option<String>,
    pub line_count_estimate: Option<u64>,
}

/// 排除记录（§141/§147）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExcludedCandidate {
    pub id: String,
    pub title: String,
    pub source: ContextSource,
    pub reason: ExcludedReason,
    pub detail: String,
}

/// SelectionTrace（§23–§24：不允许黑箱排序）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectionTrace {
    /// 参与选择的候选数（归一化后）。
    pub candidates: usize,
    pub selected: Vec<String>,
    pub excluded: Vec<ExcludedCandidate>,
}

/// 诊断（§258）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextDiagnostic {
    pub kind: String,
    pub subject: Option<String>,
    pub detail: String,
}

/// Pack 状态（§259）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PackStatus {
    Complete,
    Partial,
    Incomplete,
    Failed,
}

/// Manifest（§217/§318/§319：记录引用，不复制 source）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextManifest {
    pub schema_version: u32,
    pub id: String,
    pub project_root: String,
    pub task: Option<String>,
    pub classification: TaskClassification,
    pub target: ContextTarget,
    pub scope: ContextScope,
    pub generated_at: String,
    pub status: PackStatus,
    pub summary: ContextSummary,
    pub budget: BudgetReport,
    pub limitations: Vec<String>,
}

/// 汇总计数（§385/§386：files/items/ranges 分开）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ContextSummary {
    pub files: usize,
    pub symbols: usize,
    pub docs: usize,
    pub findings: usize,
    pub checks: usize,
    pub items: usize,
    pub ranges: usize,
}

/// ContextPack（§7/§8：一次任务视角的 Context View；§9 in-memory）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextPack {
    pub manifest: ContextManifest,
    /// 组内按 priority + 稳定 tie-break 排序；组间按 default_rank（§115）。
    pub items: Vec<ContextItem>,
    pub selection: SelectionTrace,
    pub diagnostics: Vec<ContextDiagnostic>,
}
