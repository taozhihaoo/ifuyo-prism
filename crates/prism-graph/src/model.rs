//! 关系层数据模型（规格 §5–§10、§21、§25、§26、§33）。
//!
//! 身份契约：ModuleId / GraphNodeId / 边 ID 全部从稳定事实派生
//! （相对路径 / 命名空间 / 包名），禁随机 UUID（规格 §7/§27）。
//! ProjectId 在 M1 是运行期 UUID，故模块身份暂不含 project 维度；
//! 稳定项目身份属 M11（偏离已记录于 docs/architecture.md）。

pub use prism_core::ids::ModuleId;
use prism_core::Confidence;
use prism_parser::SourceLocation;
use prism_project::{Evidence, Language};
use serde::{Deserialize, Serialize};

/// 对外 JSON 契约版本。
pub const GRAPH_SCHEMA_VERSION: u32 = 1;

/// 模块种类（规格 §6）。不强行统一语义。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModuleKind {
    Project,
    Package,
    Namespace,
    FileModule,
    ExternalPackage,
    Builtin,
    Unknown,
}

impl ModuleKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            ModuleKind::Project => "project",
            ModuleKind::Package => "package",
            ModuleKind::Namespace => "namespace",
            ModuleKind::FileModule => "file_module",
            ModuleKind::ExternalPackage => "external_package",
            ModuleKind::Builtin => "builtin",
            ModuleKind::Unknown => "unknown",
        }
    }
}

/// 一个模块（Module ≠ File ≠ Symbol，规格 §6）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Module {
    pub id: ModuleId,
    pub name: String,
    pub kind: ModuleKind,
    pub file_ids: Vec<prism_core::ids::FileId>,
    pub parent_id: Option<ModuleId>,
    pub language: Option<Language>,
    /// 项目相对路径（文件模块 = 文件路径；包模块 = 目录路径）。
    pub path: Option<String>,
    pub evidence: Vec<Evidence>,
}

/// 关系解析状态（规格 §10；含义必须显式，禁止猜测补全）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResolutionStatus {
    /// 明确解析到项目内实体。
    Resolved,
    /// 存在多个合理候选。
    Ambiguous,
    /// 存在关系但当前 resolver 无法解析。
    Unresolved,
    /// 项目外部 package。
    External,
    /// 语言/runtime 内置模块。
    Builtin,
    /// 依据 ignore 规则或项目规则主动排除。
    Ignored,
    /// 该字段对此关系不适用。
    NotApplicable,
}

/// 依赖种类（规格 §8 的 M3 可靠子集）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencyKind {
    Import,
    ReExport,
    NamespaceReference,
    ProjectReference,
    PackageReference,
    ModuleReference,
    Unknown,
}

impl DependencyKind {
    /// 稳定的对外字符串（与 serde 序列化一致）。
    pub const fn as_str(self) -> &'static str {
        match self {
            DependencyKind::Import => "import",
            DependencyKind::ReExport => "re_export",
            DependencyKind::NamespaceReference => "namespace_reference",
            DependencyKind::ProjectReference => "project_reference",
            DependencyKind::PackageReference => "package_reference",
            DependencyKind::ModuleReference => "module_reference",
            DependencyKind::Unknown => "unknown",
        }
    }
}

/// 一条模块级依赖（文件 → 目标），含完整证据链。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dependency {
    /// 稳定 ID：`dep:{source_path}#{raw_target}#{kind}`。
    pub id: String,
    /// 来源 FileModule ID。
    pub source_id: ModuleId,
    pub source_path: String,
    /// 原样 specifier（未加工）。
    pub raw_target: String,
    pub kind: DependencyKind,
    pub resolution: ResolutionStatus,
    pub confidence: Confidence,
    /// Resolved 时的目标模块 ID。
    pub target_id: Option<ModuleId>,
    /// Resolved 时的目标项目相对路径。
    pub target_path: Option<String>,
    pub imported_names: Vec<String>,
    pub location: Option<SourceLocation>,
    pub evidence: Vec<Evidence>,
}

/// 无法解析（或部分解析）关系的可解释诊断（规格 §26）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolutionDiagnostic {
    pub source_path: String,
    pub raw_target: String,
    pub resolver: String,
    pub status: ResolutionStatus,
    pub reason: String,
    pub location: Option<SourceLocation>,
}

/// 图节点种类（规格 §21）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GraphNodeKind {
    Project,
    Package,
    Namespace,
    FileModule,
    ExternalPackage,
    BuiltinModule,
}

/// 图边种类（规格 §21；DependsOn 由 Imports 在模块级派生）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GraphEdgeKind {
    Contains,
    Imports,
    DependsOn,
    ReExports,
    ProjectReferences,
    PackageDependsOn,
}

/// 图节点。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphNode {
    /// 稳定 ID：`node:{kind}:{entity}`。
    pub id: String,
    pub kind: GraphNodeKind,
    pub label: String,
    /// 对应实体（模块 ID / 路径 / 包名）。
    pub entity: String,
    pub language: Option<Language>,
}

/// 图边。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphEdge {
    /// 稳定 ID：`edge:{source}#{target}#{kind}`。
    pub id: String,
    pub source: String,
    pub target: String,
    pub kind: GraphEdgeKind,
    pub resolution: ResolutionStatus,
    pub confidence: Confidence,
    pub evidence: Vec<Evidence>,
}

/// 一个循环依赖（SCC，>1 节点；仅为记录，不是健康判断，规格 §25）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cycle {
    /// 稳定 ID：`cycle:{排序后节点 id 连接}`。
    pub id: String,
    /// 环内节点（按 id 字典序，确定性）。
    pub nodes: Vec<String>,
}

/// 客观结构指标（事实，不做评价，规格 §24）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct GraphMetrics {
    pub node_count: u64,
    pub edge_count: u64,
    pub resolved_edges: u64,
    pub ambiguous_edges: u64,
    pub unresolved_edges: u64,
    pub external_edges: u64,
    pub builtin_edges: u64,
    pub cycle_count: u64,
    pub connected_components: u64,
    /// fan-in 最高的节点（降序，最多 10）。
    pub most_referenced: Vec<(String, u64)>,
    /// fan-out 最高的节点（降序，最多 10）。
    pub most_dependent: Vec<(String, u64)>,
}

/// 一条静态调用边（规格 §19 可靠子集；仅直接调用，且 callee 解析成功时
/// 才为 Resolved；动态/未知名保持 Unresolved，不猜测）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CallEdge {
    /// 稳定 ID：`call:{caller}#{callee_name}#{line}:{column}`。
    pub id: String,
    /// 调用方符号（限定路径所属）。
    pub caller_id: prism_core::ids::SymbolId,
    pub caller_qualified: String,
    /// 原样 callee 文本（"helper" / "obj.method"）。
    pub callee_text: String,
    /// 调用名的最后一段（解析键）。
    pub callee_name: String,
    /// Resolved 时的被调符号。
    pub callee_id: Option<prism_core::ids::SymbolId>,
    pub callee_qualified: Option<String>,
    pub resolution: ResolutionStatus,
    pub confidence: Confidence,
    pub location: Option<SourceLocation>,
}

/// M3 对外 JSON 契约（规格 §33）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DependencyGraphResult {
    pub schema_version: u32,
    /// 项目名（不含绝对路径——§33）。
    pub project_name: String,
    /// 按稳定 ID 排序。
    pub modules: Vec<Module>,
    /// 按稳定 ID 排序。
    pub dependencies: Vec<Dependency>,
    /// 按稳定 ID 排序。
    pub nodes: Vec<GraphNode>,
    /// 按稳定 ID 排序。
    pub edges: Vec<GraphEdge>,
    /// 静态调用边（符号级，规格 §19/§20），按稳定 ID 排序。
    #[serde(default)]
    pub calls: Vec<CallEdge>,
    /// 按 ID 排序。
    pub cycles: Vec<Cycle>,
    pub metrics: GraphMetrics,
    /// 按 (source_path, raw_target) 排序。
    pub diagnostics: Vec<ResolutionDiagnostic>,
}
