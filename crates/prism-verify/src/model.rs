//! Verify 核心模型（规格 M8 §7–§12/§28/§56–§59/§72–§74/§85–§86/§126–§128/§159/§237/§241）。

use serde::{Deserialize, Serialize};

/// 对外 JSON 契约版本。
pub const VERIFY_SCHEMA_VERSION: u32 = 1;

/// 验证种类（§8）。CI/Release 第一版是编排占位（§8 注意事项）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum CheckKind {
    Build,
    Test,
    Lint,
    Format,
    Typecheck,
    StaticAnalysis,
    Custom,
}

impl CheckKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            CheckKind::Build => "build",
            CheckKind::Test => "test",
            CheckKind::Lint => "lint",
            CheckKind::Format => "format",
            CheckKind::Typecheck => "typecheck",
            CheckKind::StaticAnalysis => "static_analysis",
            CheckKind::Custom => "custom",
        }
    }

    /// 字符串 → kind（配置解析用；未知 ⇒ None，§54 unknown kind 检测）。
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "build" => Some(CheckKind::Build),
            "test" => Some(CheckKind::Test),
            "lint" => Some(CheckKind::Lint),
            "format" => Some(CheckKind::Format),
            "typecheck" => Some(CheckKind::Typecheck),
            "static_analysis" | "staticanalysis" => Some(CheckKind::StaticAnalysis),
            "custom" => Some(CheckKind::Custom),
            _ => None,
        }
    }
}

/// 执行状态（§9：现实环境远不止 Success/Failure）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum CheckState {
    Queued,
    Running,
    Passed,
    Failed,
    Skipped,
    Cancelled,
    TimedOut,
    Blocked,
    Unavailable,
    Error,
}

impl CheckState {
    pub const fn as_str(self) -> &'static str {
        match self {
            CheckState::Queued => "queued",
            CheckState::Running => "running",
            CheckState::Passed => "passed",
            CheckState::Failed => "failed",
            CheckState::Skipped => "skipped",
            CheckState::Cancelled => "cancelled",
            CheckState::TimedOut => "timed_out",
            CheckState::Blocked => "blocked",
            CheckState::Unavailable => "unavailable",
            CheckState::Error => "error",
        }
    }
}

/// 命令写策略（§85–§87）：默认只允许 ReadOnly / BuildArtifacts。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum WritePolicy {
    ReadOnly,
    BuildArtifacts,
    ArbitraryWrite,
}

impl WritePolicy {
    pub const fn as_str(self) -> &'static str {
        match self {
            WritePolicy::ReadOnly => "read_only",
            WritePolicy::BuildArtifacts => "build_artifacts",
            WritePolicy::ArbitraryWrite => "arbitrary_write",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "read_only" | "readonly" => Some(WritePolicy::ReadOnly),
            "build_artifacts" | "buildartifacts" => Some(WritePolicy::BuildArtifacts),
            "arbitrary_write" | "arbitrarywrite" => Some(WritePolicy::ArbitraryWrite),
            _ => None,
        }
    }
}

/// check 来源（§56/§241：让用户知道为什么执行这条命令）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum CheckSource {
    /// prism.config.json 显式配置。
    Configured,
    /// package.json scripts（Node 生态）。
    PackageScript,
    /// Cargo 项目约定。
    Cargo,
    /// pyproject/pytest 约定。
    Pyproject,
    /// 内置。
    BuiltIn,
}

impl CheckSource {
    pub const fn as_str(self) -> &'static str {
        match self {
            CheckSource::Configured => "configured",
            CheckSource::PackageScript => "package_script",
            CheckSource::Cargo => "cargo",
            CheckSource::Pyproject => "pyproject",
            CheckSource::BuiltIn => "built_in",
        }
    }
}

/// 发现置信度（§57/§237）：第一版只自动执行 Exact/HighConfidence（§238）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum CheckConfidence {
    Exact,
    HighConfidence,
    Probable,
    Ambiguous,
}

/// check 定义（§12/§127：稳定定义，与一次执行 CheckRun 分离）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckSpec {
    /// 稳定 check id（§128，如 "test"）。
    pub id: String,
    pub kind: CheckKind,
    pub name: String,
    /// 程序（无 shell 字符串，§13）。
    pub program: String,
    pub args: Vec<String>,
    /// 项目相对工作目录；None = 项目根（§16）。
    pub working_directory: Option<String>,
    pub timeout_ms: u64,
    /// 输出采集上限（透传 M0 CommandSpec.max_output_bytes；§201/§202）。
    pub max_output_bytes: Option<usize>,
    pub required: bool,
    pub write_policy: WritePolicy,
    /// 依赖的其他 check id（§28）。
    pub depends_on: Vec<String>,
    pub source: CheckSource,
    pub confidence: CheckConfidence,
    /// 来源明细（§236：如 "package.json scripts.test"，不用无信息描述）。
    pub detected_from: String,
}

/// 前置条件种类（§159）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Prerequisite {
    /// 可执行文件在 PATH/项目内可解析（§20）。
    ExecutableExists { program: String },
    /// 项目相对文件存在。
    FileExists { path: String },
}

/// 前置条件评估结果（§160）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrerequisiteOutcome {
    Satisfied,
    Unsatisfied,
    Unknown,
}

/// 执行计划中的条目（§25）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlannedCheck {
    pub id: String,
    pub kind: CheckKind,
    pub name: String,
    pub program: String,
    pub args: Vec<String>,
    pub required: bool,
    pub source: CheckSource,
    pub detected_from: String,
    pub depends_on: Vec<String>,
}

/// VerifyPlan（§25/§269：创建后执行期不可变）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerifyPlan {
    /// 执行顺序（拓扑分层后按层展开）。
    pub checks: Vec<PlannedCheck>,
}

/// 一次实际运行的完整记录（§7/§126：run_id 为执行实例唯一 id，§128）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckRun {
    /// 执行实例 id（UUID，§126/§127）。
    pub run_id: String,
    pub check_id: String,
    pub kind: CheckKind,
    pub name: String,
    pub required: bool,
    pub state: CheckState,
    /// 脱敏后的命令展示（§173/§174）。
    pub command_display: String,
    pub working_directory: String,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub duration_ms: u64,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub output_truncated: bool,
    /// 环境元数据：变量名 + present/absent（§18/§175：不输出值）。
    pub environment_metadata: Vec<String>,
    pub diagnostics: Vec<VerifyDiagnostic>,
    /// 写策略执行结果备注（如 "skipped: arbitrary_write not allowed"）。
    pub reason: Option<String>,
    /// 执行期间该 check 观测到的验证副作用（§167/§168：仅记录，不回滚、
    /// 不归类为问题）。M8a 为层粒度归因（同层并发共享同一窗口）。
    pub side_effects: Vec<String>,
}

/// Verify 诊断（§95/§154：分类明确，不都变成 "Command failed"）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerifyDiagnostic {
    /// "timeout" / "spawn_failed" / "program_not_found" / "write_policy_refused" / …
    pub kind: String,
    pub detail: String,
}

/// 整体验证结论（§71：不声称 "Project = Verified"，除非 required 全过）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OverallConclusion {
    /// required 全部 Passed（无 required 时默认，除非有 failed）。
    Pass,
    /// 有 required check Failed（或 required Unavailable/Blocked 按策略）。
    Fail,
    /// 既有 Passed 又有 Failed/Blocked/Unavailable 的混合，或全部 optional。
    Partial,
    /// 无任何 check 被执行。
    NotRun,
}

/// 计数（§10）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct VerifyCounts {
    pub passed: u64,
    pub failed: u64,
    pub unavailable: u64,
    pub blocked: u64,
    pub skipped: u64,
    pub cancelled: u64,
    pub timed_out: u64,
    pub error: u64,
}

/// 完整验证报告（§141 输出契约）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationReport {
    pub schema_version: u32,
    pub project_name: String,
    pub plan: VerifyPlan,
    /// 按 VerifyPlan 顺序排列的运行结果（§225：执行顺序不影响展示顺序）。
    pub runs: Vec<CheckRun>,
    pub conclusion: OverallConclusion,
    pub completeness: prism_core::PrismCompleteness,
    pub counts: VerifyCounts,
    /// 整次验证观测到的副作用路径（执行前后 Git status 差异；§167/§198）。
    pub side_effects: Vec<String>,
    /// 受影响的测试候选路径（§34/§247：Candidate impacted tests——
    /// 绝不冒充 "Verified affected tests"）。
    pub impacted_test_candidates: Vec<String>,
}

/// 引擎选项。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct VerifyOptions {
    /// 选中的 check id；None = 全部（§31/§147）。
    pub selected: Option<Vec<String>>,
    pub dry_run: bool,
    /// 有 Warning 级及以上 finding 时非零退出（沿用 health 的 fail-on 语义，
    /// 这里按 state：required failed ⇒ 1；该字段为 verify 特有的宽松模式开关）。
    pub fail_on_unavailable: bool,
}



/// check 策略（§74：第一版保持简单）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct CheckPolicy {
    /// conclusion 判定基于 required 集（§72）。
    pub required_only_for_fail: bool,
}

