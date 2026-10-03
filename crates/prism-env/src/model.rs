//! 环境模型（规格 M9a §11–§21/§54–§57/§138/§150/§163/§292–§294/§337–§340）。
//!
//! EnvironmentSnapshot 是**一次环境观测**（§12/§13：session 内对象，
//! 非持久化历史；M11 归属持久化）。关键事实必须带 source + evidence；
//! 采不到的字段是 None/Unknown，绝不虚构（§209）。

/// 快照 schema 版本（§292/§294：显式版本，向后兼容从 M9b 开始考虑）。
pub const ENVIRONMENT_SCHEMA_VERSION: u32 = 1;

use serde::{Deserialize, Serialize};

/// 一次环境扫描产出的完整快照（§11，按现有架构裁剪；M9a 不含
/// ports/processes/container/requirements-provider 全量——M9b 承接）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnvironmentSnapshot {
    pub schema_version: u32,
    /// 观测 id（§13：每次主动重扫新建，不要求确定性）。
    pub id: String,
    /// 项目 id（M1 Project::id 透传）。
    pub project_id: String,
    /// 项目根（展示用；环境事实本身与项目无强绑定）。
    pub project_root: String,
    /// 采集时间（§289：只记录 captured_at，不做"当前时间"稳定事实）。
    pub captured_at: String,
    pub host: HostInfo,
    pub shell: ShellInfo,
    pub paths: PathFacts,
    /// 项目相关工具链事实（§22–§30；lazily 按 profile 选择，§120）。
    pub tools: Vec<ToolFact>,
    /// 项目虚拟环境（§69：exists/interpreter/version/active，绝不 activate）。
    pub virtualenvs: Vec<VirtualEnvInfo>,
    pub runtimes: Vec<RuntimeInfo>,
    pub sdks: Vec<SdkInfo>,
    pub package_managers: Vec<PackageManagerInfo>,
    /// 环境变量名 + 分类（§54：绝不采集值）。
    pub variables: Vec<VariableFact>,
    /// 监听端口事实（§81–§85；动态事实，每次 refresh 重采 §211/§217）。
    #[serde(default)]
    pub ports: Vec<PortInfo>,
    /// 相关进程事实（§86–§93；动态事实）。
    #[serde(default)]
    pub processes: Vec<ProcessInfo>,
    /// 期望端口候选（§94：来源 + 置信度）。
    #[serde(default)]
    pub expected_ports: Vec<ExpectedPort>,
    pub resources: ResourceFacts,
    /// 项目环境需求（§42–§45/§71–§77 provider 层产出）。
    pub requirements: Vec<EnvironmentRequirement>,
    /// 同主体多来源且期望不一致的冲突（§45/§103：不静默择一）。
    #[serde(default)]
    pub conflicts: Vec<RequirementConflict>,
    /// 需求 × 实际环境 的匹配结果（§46）。
    pub matches: Vec<RequirementMatch>,
    /// 单个采集器/探测失败隔离后的诊断（§149/§150/§392–§393）。
    pub diagnostics: Vec<EnvironmentDiagnostic>,
    /// 扫描过程结果（§337/§338：scan 状态与 requirement 状态严格分离）。
    pub scan_outcome: ScanOutcome,
    /// 环境完整性（§138：分项事实各自可 Unknown，整体取 Partial）。
    pub completeness: EnvironmentCompleteness,
}

/// 扫描生命周期（§337：复用已有 task 语义的精简形态）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScanOutcome {
    Completed,
    Partial,
    Cancelled,
}

/// 环境完整性（§138：Complete/Partial/Unknown，不是分数）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnvironmentCompleteness {
    Complete,
    Partial,
    Unknown,
}

/// Host / OS / Architecture 事实（§14–§16）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostInfo {
    pub os: OsFamily,
    /// 规范化后的 OS 展示名（GUI 不依赖原始字符串，§15）。
    pub os_name: String,
    /// OS 版本；平台 API 不可用时为 None（§307 Unsupported，不猜）。
    pub os_version: Option<String>,
    pub architecture: Architecture,
    /// 最小化身份信息（§14）：仅 hostname，来源进程环境；不作为核心产品数据。
    pub hostname: Option<String>,
    /// v1 不采集 username（§14 默认尽量减少身份信息）。
    pub username: Option<String>,
    /// locale 名（§286：仅名字；v1 Unix 读 LANG/LC_ALL，Windows 未实现）。
    pub locale: Option<String>,
    /// 时区名（§287：非核心；v1 未实现 ⇒ None）。
    pub timezone: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OsFamily {
    Windows,
    MacOS,
    Linux,
    Other,
}

impl OsFamily {
    pub const fn as_str(self) -> &'static str {
        match self {
            OsFamily::Windows => "windows",
            OsFamily::MacOS => "macos",
            OsFamily::Linux => "linux",
            OsFamily::Other => "other",
        }
    }

    /// 从 std::env::consts::OS 规范化（§15）。
    pub fn from_const(os_const: &str) -> Self {
        match os_const {
            "windows" => OsFamily::Windows,
            "macos" => OsFamily::MacOS,
            "linux" => OsFamily::Linux,
            _ => OsFamily::Other,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Architecture {
    X86,
    X86_64,
    Arm64,
    Arm,
    Unknown,
}

impl Architecture {
    pub const fn as_str(self) -> &'static str {
        match self {
            Architecture::X86 => "x86",
            Architecture::X86_64 => "x86_64",
            Architecture::Arm64 => "arm64",
            Architecture::Arm => "arm",
            Architecture::Unknown => "unknown",
        }
    }

    /// 从 std::env::consts::ARCH 规范化（§16）。
    pub fn from_const(arch_const: &str) -> Self {
        match arch_const {
            "x86" => Architecture::X86,
            "x86_64" => Architecture::X86_64,
            "aarch64" => Architecture::Arm64,
            "arm" => Architecture::Arm,
            _ => Architecture::Unknown,
        }
    }
}

/// 当前 shell 事实（§17–§18：只记录，不做执行引擎）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShellInfo {
    pub kind: ShellKind,
    pub executable: Option<String>,
    /// "version if available"；v1 不为 shell 发起 probe ⇒ None。
    pub version: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShellKind {
    PowerShell,
    Pwsh,
    Cmd,
    Bash,
    Zsh,
    Fish,
    Unknown,
}

impl ShellKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            ShellKind::PowerShell => "powershell",
            ShellKind::Pwsh => "pwsh",
            ShellKind::Cmd => "cmd",
            ShellKind::Bash => "bash",
            ShellKind::Zsh => "zsh",
            ShellKind::Fish => "fish",
            ShellKind::Unknown => "unknown",
        }
    }
}

/// 事实作用域（§163：Process/User/System/Project 不混成一个 PATH）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FactScope {
    Process,
    User,
    System,
    Project,
    Unknown,
}

/// PATH 事实（§19–§21：parse/normalize/dedupe/diagnostics，有界 §314）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PathFacts {
    pub scope: FactScope,
    /// 顺序保留（§20 ordered entries）。
    pub entries: Vec<PathEntry>,
    /// 重复条目（按 identity；§21 duplicate PATH entries）。
    pub duplicates: Vec<String>,
    /// 不存在的目录（§21 nonexistent PATH directory）。
    pub missing: Vec<String>,
    /// 超出上限被截断（§314）。
    pub truncated: bool,
}

/// PATH 条目上限（§314：异常巨大的 PATH 有界采集）。
pub const PATH_ENTRY_CAP: usize = 200;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PathEntry {
    /// 原样条目（去引号/空白后）。
    pub raw: String,
    /// 平台规范化表示（§108/§109/§340）。
    pub normalized: String,
    /// 身份键：Windows 大小写不敏感（§192），其他平台原样。
    pub identity: String,
    pub exists: bool,
}

/// 资源事实（§140–§145：snapshot 时点快照，不做监控）。
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ResourceFacts {
    pub logical_cpus: Option<u64>,
    pub memory_total_bytes: Option<u64>,
    /// 项目根所在文件系统的可用空间（§143）。
    pub disk_free_bytes: Option<u64>,
    pub disk_path: Option<String>,
}



/// 环境变量事实（§54：name + present + classification + source；无值）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VariableFact {
    pub name: String,
    pub present: bool,
    pub possibly_sensitive: bool,
    pub source: VariableSource,
}

/// 变量来源（§57；v1 只采集 ProcessEnvironment，不读 shell history）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VariableSource {
    ProcessEnvironment,
    ProjectEnvFile,
    ShellProfile,
    CIConfig,
    PrismConfig,
    Unknown,
}

/// 环境变量名上限（§315：不读取无界 env 数据）。
pub const VARIABLE_CAP: usize = 500;

/// 端口条目上限（§313）。
pub const PORT_CAP: usize = 100;

/// 进程条目上限（§312）。
pub const PROCESS_CAP: usize = 200;

/// 监听端口事实（§81/§83 listening-only；§211 observed_at 必带）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortInfo {
    pub port: u16,
    /// v1 只有 TCP listening（§83）。
    pub protocol: PortProtocol,
    /// 绑定地址（"0.0.0.0:5173" / "[::]:5173"）。
    pub address: String,
    pub state: PortState,
    /// 归属 PID；平台不允许/解析失败 ⇒ None（§85 不能猜）。
    pub pid: Option<u32>,
    pub process_name: Option<String>,
    /// 采集来源（"netstat"/"ss"）。
    pub source: String,
    pub observed_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PortProtocol {
    Tcp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PortState {
    Listening,
}

/// 相关进程事实（§86–§89；§87 v1 不采集 command line）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessInfo {
    /// §212：PID 会变，不作稳定身份。
    pub pid: u32,
    pub name: String,
    /// 可执行路径；平台不提供 ⇒ None（§92/§213）。
    pub executable: Option<String>,
    /// 该进程监听的端口（与 ports 关联，§86）。
    pub ports: Vec<u16>,
    /// 证据式分类（§89：基于 name/executable evidence）。
    pub classification: ProcessKind,
    /// 项目关联证据（§91/§330）；v1 无 cwd/cmdline API ⇒ None（不猜）。
    pub project_association: Option<String>,
    pub observed_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcessKind {
    Node,
    Python,
    Dotnet,
    Java,
    Rust,
    Docker,
    Database,
    Browser,
    Unknown,
}

impl ProcessKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            ProcessKind::Node => "node",
            ProcessKind::Python => "python",
            ProcessKind::Dotnet => "dotnet",
            ProcessKind::Java => "java",
            ProcessKind::Rust => "rust",
            ProcessKind::Docker => "docker",
            ProcessKind::Database => "database",
            ProcessKind::Browser => "browser",
            ProcessKind::Unknown => "unknown",
        }
    }
}

/// 期望端口候选（§94：provenance + confidence 必须明确）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExpectedPort {
    pub port: u16,
    /// 如 "docker-compose.yml ports"。
    pub source: String,
    pub confidence: prism_core::Confidence,
}

/// 平台能力模型（§146–§147）：能力不可用 ⇒ 对应事实 Unknown/缺省 +
/// 诊断，而不是引擎失败（§307 Unsupported）。M9a 实际覆盖前四项。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnvironmentCapability {
    ExecutableDiscovery,
    ToolVersionProbe,
    EnvironmentVariableInspection,
    ResourceInspection,
    ProcessInspection,
    PortInspection,
}

/// 诊断（§150：复用 Core 的 Diagnostic 形态；部分失败 ≠ 整体 Failed）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnvironmentDiagnostic {
    pub kind: String,
    pub subject: Option<String>,
    pub detail: String,
}

// ---------- 工具链 / 运行时 / SDK / 包管理器（§22–§37） ----------

/// 工具事实（§22 ExecutableInfo + §26 source + §28 version）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolFact {
    /// 稳定逻辑名（§23 allowlist 中的名字，如 "node"）。
    pub name: String,
    /// 解析到的可执行文件路径；未找到 ⇒ None（§209：NotFound 需 lookup 确认）。
    pub path: Option<String>,
    /// 同名多个候选（§65 PATH shadowing 事实；active 排最前）。
    pub additional_paths: Vec<String>,
    /// 版本；probe 失败/不可解析 ⇒ None（§157：exists 但 version unknown）。
    pub version: Option<crate::version::ToolVersion>,
    /// 版本 probe 的原始输出首行（未脱敏顾虑：--version 输出非 secret；
    /// 仍过中央 redaction 后保存，§346）。
    pub raw_version: Option<String>,
    pub source: ToolSource,
    /// probe 状态（§156–§157：found-but-probe-failed ≠ missing）。
    pub probe: ProbeOutcome,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolSource {
    Path,
    KnownLocation,
    ProjectLocal,
    VersionManager,
    Unknown,
}

impl ToolSource {
    pub const fn as_str(self) -> &'static str {
        match self {
            ToolSource::Path => "path",
            ToolSource::KnownLocation => "known_location",
            ToolSource::ProjectLocal => "project_local",
            ToolSource::VersionManager => "version_manager",
            ToolSource::Unknown => "unknown",
        }
    }
}

/// probe 结果语义（§156–§158）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProbeOutcome {
    /// 未 probe（不需要版本）。
    NotProbed,
    /// `--version` 成功。
    Succeeded,
    /// 可执行文件存在但 probe 失败（非零/超时/取消）。
    Failed,
    /// 可执行文件未找到（lookup 确认）。
    NotFound,
}

/// 虚拟环境（§69：只识别记录；不切换、不 activate）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VirtualEnvInfo {
    /// "venv" / "virtualenv" / "conda"（按目录特征）。
    pub kind: String,
    pub path: String,
    pub interpreter: Option<String>,
    pub version: Option<crate::version::ToolVersion>,
    pub raw_version: Option<String>,
    /// 是否是当前进程激活的环境（VIRTUAL_ENV 指向此处）。
    pub active: bool,
}

/// 运行时（§31：Node.js/Python/.NET/JVM/Go/Rust…）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeInfo {
    pub name: String,
    pub version: Option<crate::version::ToolVersion>,
    pub raw_version: Option<String>,
    pub path: Option<String>,
    pub architecture: Option<Architecture>,
    pub source: ToolSource,
}

/// SDK（§32–§34：与 runtime 严格分开；如 .NET Runtime vs .NET SDK）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SdkInfo {
    pub name: String,
    pub version: Option<crate::version::ToolVersion>,
    pub raw_version: Option<String>,
    pub source: ToolSource,
}

/// 包管理器（§35：name/version/path/project association/source）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageManagerInfo {
    pub name: String,
    pub version: Option<crate::version::ToolVersion>,
    pub raw_version: Option<String>,
    pub path: Option<String>,
    /// 与当前项目的关联（§36/§37：来自 lockfile/packageManager 字段，带证据）。
    pub project_association: Option<PackageManagerAssociation>,
    pub source: ToolSource,
}

/// 项目-包管理器关联证据（§37 provenance：必须回答"为什么判断是 pnpm"）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageManagerAssociation {
    pub manager: String,
    /// "package-lock.json" / "packageManager field" / "pnpm-lock.yaml" 等。
    pub evidence: String,
    /// packageManager 字段携带的版本要求（§106，如 "pnpm@10.17.0"）。
    pub required_version: Option<String>,
}

/// 环境需求（§38–§41 模型；provider 层级见 `requirements.rs`，
/// §42–§45/§71–§77/§102/§124–§125）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnvironmentRequirement {
    /// 确定性 id：`req:{kind}:{subject}#{source}`。
    pub id: String,
    pub kind: RequirementKind,
    /// 需求主体（如 "node" / "pnpm" / "PATH"）。
    pub subject: String,
    pub operator: RequirementOperator,
    /// 期望值（版本串 / 名字 / 存在性为空串）。
    pub expected: String,
    /// 来源描述（如 "package.json engines.node"）。
    pub source: String,
    /// 来源层级（§44/§102：显式配置 > 生态元数据 > CI > 版本文件 > 文档 > 启发式）。
    #[serde(default)]
    pub source_kind: SourceKind,
    /// 证据（§151：具体文件/字段）。
    pub evidence: String,
    pub confidence: prism_core::Confidence,
    /// 该需求是否适用于当前项目（§50 NotApplicable 的判定输入）。
    pub applicable: bool,
}

/// 需求来源层级（§44/§102）。数值越小优先级越高；
/// 冲突时不静默择一（§45），全部保留并报告。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    #[default]
    ProjectConfig,
    EcosystemMetadata,
    CiConfig,
    VersionFile,
    ContainerConfig,
    Documentation,
    Heuristic,
}

impl SourceKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            SourceKind::ProjectConfig => "project_config",
            SourceKind::EcosystemMetadata => "ecosystem_metadata",
            SourceKind::CiConfig => "ci_config",
            SourceKind::VersionFile => "version_file",
            SourceKind::ContainerConfig => "container_config",
            SourceKind::Documentation => "documentation",
            SourceKind::Heuristic => "heuristic",
        }
    }
}

/// 同一主体来自多个来源且期望不一致（§45/§103/§327）：
/// 不自动选择，全部保留。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequirementConflict {
    pub kind: RequirementKind,
    pub subject: String,
    /// 各来源条目（按 SourceKind 升序）。
    pub entries: Vec<ConflictEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConflictEntry {
    pub source: String,
    pub source_kind: SourceKind,
    pub expected: String,
    pub operator: RequirementOperator,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequirementKind {
    Runtime,
    Sdk,
    Tool,
    PackageManager,
    Os,
    Architecture,
    EnvironmentVariable,
    File,
    Directory,
    Port,
    Service,
    Container,
}

impl RequirementKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            RequirementKind::Runtime => "runtime",
            RequirementKind::Sdk => "sdk",
            RequirementKind::Tool => "tool",
            RequirementKind::PackageManager => "package_manager",
            RequirementKind::Os => "os",
            RequirementKind::Architecture => "architecture",
            RequirementKind::EnvironmentVariable => "environment_variable",
            RequirementKind::File => "file",
            RequirementKind::Directory => "directory",
            RequirementKind::Port => "port",
            RequirementKind::Service => "service",
            RequirementKind::Container => "container",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequirementOperator {
    Exists,
    Equals,
    NotEquals,
    AtLeast,
    AtMost,
    InRange,
    /// 端口可用（§95：Port requirement 专用）。
    Available,
}

impl RequirementOperator {
    pub const fn as_str(self) -> &'static str {
        match self {
            RequirementOperator::Exists => "exists",
            RequirementOperator::Equals => "equals",
            RequirementOperator::NotEquals => "not_equals",
            RequirementOperator::AtLeast => "at_least",
            RequirementOperator::AtMost => "at_most",
            RequirementOperator::InRange => "in_range",
            RequirementOperator::Available => "available",
        }
    }
}

/// 匹配结果（§46–§50 + §258 evidence-first）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequirementMatch {
    pub requirement_id: String,
    pub subject: String,
    pub result: MatchResult,
    /// 人类可读判定依据（§258：required/actual/result 三段式的基础）。
    pub evidence: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchResult {
    Satisfied,
    Unsatisfied,
    /// 采不到可靠事实（§49/§158）：absence of evidence ≠ evidence of absence。
    Unknown,
    /// 需求不适用于当前项目（§50）。
    NotApplicable,
}

impl MatchResult {
    pub const fn as_str(self) -> &'static str {
        match self {
            MatchResult::Satisfied => "satisfied",
            MatchResult::Unsatisfied => "unsatisfied",
            MatchResult::Unknown => "unknown",
            MatchResult::NotApplicable => "not_applicable",
        }
    }
}
