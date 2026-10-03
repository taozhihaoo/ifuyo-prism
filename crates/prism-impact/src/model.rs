//! Impact 领域模型（规格 M6 §7/§8/§30/§31/§45/§126–§130）。
//!
//! 确定性契约：全部集合 serde snake_case + 稳定排序；id 从事实派生，
//! 禁随机 UUID；`distance` = 关系跳数（0 = 被改本身，§81）。

use serde::{Deserialize, Serialize};

/// 对外 JSON 契约版本。
pub const IMPACT_SCHEMA_VERSION: u32 = 1;

/// 分析选项（§125：少量参数、保守默认；§36/§37/§78 的界限所在）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImpactOptions {
    /// 反向遍历最大深度（关系跳数；达到 ⇒ TraversalDepthExceeded + Truncated）。
    pub max_depth: u32,
    /// 受影响节点上限（达到 ⇒ ImpactNodeLimitReached + Truncated）。
    pub max_nodes: u64,
    /// 证据路径上限。
    pub max_paths: u64,
    /// 每个风险信号的证据条数上限。
    pub max_evidence_per_signal: u64,
    /// 收集测试候选（§26）。
    pub include_tests: bool,
    /// 收集文档候选（§28；无证据则产出 NoDocumentationEvidence 诊断）。
    pub include_docs: bool,
}

impl Default for ImpactOptions {
    fn default() -> Self {
        // 默认值按当前真实项目规模评估（§36）：Prism 自身调用链 < 6 跳，
        // 10 跳足以覆盖传递影响；节点上限保护 GUI/内存（§37）。
        ImpactOptions {
            max_depth: 10,
            max_nodes: 2_000,
            max_paths: 10_000,
            max_evidence_per_signal: 20,
            include_tests: true,
            include_docs: true,
        }
    }
}

/// 分析完整度（§127；含义写入 schema 文档）：
/// - Complete：全部请求的遍历完成
/// - Partial：部分源/关系映射不可用（如不支持语言、解析失败），但仍有效
/// - Truncated：达到 depth/nodes/paths 上限被截断
/// - Failed：无法产生有效结果（如 change_set 缺失）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImpactCompleteness {
    Complete,
    Partial,
    Truncated,
    Failed,
}

/// 节点种类（§8）。不强制所有层级都存在。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    File,
    Symbol,
    FileModule,
    Package,
    Namespace,
    Project,
    Documentation,
}

impl NodeKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            NodeKind::File => "file",
            NodeKind::Symbol => "symbol",
            NodeKind::FileModule => "file_module",
            NodeKind::Package => "package",
            NodeKind::Namespace => "namespace",
            NodeKind::Project => "project",
            NodeKind::Documentation => "documentation",
        }
    }
}

/// 影响种类（§9/§84：同一节点允许多个分类，不强迫单一 enum）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum ImpactKind {
    /// 被改本身（M5 ChangeSet 事实；与 impacted 严格分离，§48）。
    DirectlyChanged,
    /// 直接反向依赖（distance 1）。
    DirectDependent,
    /// 传递反向依赖（distance ≥ 2）。
    TransitiveDependent,
    /// 调用了被改符号。
    Caller,
    /// 传递调用（caller of caller）。
    TransitiveCaller,
    /// 消费了被改模块（imports/depends on）。
    ModuleConsumer,
    /// 跨项目消费（workspace 内项目引用；外部包不扫描，§51/§52）。
    ProjectConsumer,
    /// 测试候选（§26/§54；候选语义，绝不等于"会失败"，§27/§55）。
    TestCandidate,
    /// 文档候选（§28/§29：仅当存在已知关系证据；无证据不猜）。
    DocumentationCandidate,
    /// 配置相关（§53：仅作为事实标记，不推断波及面）。
    ConfigurationCandidate,
}

/// 影响置信度（§45/§46/§47）：分类制，来自关系解析 × 变更映射置信的
/// 保守传播（取弱者），不制造伪精确分数。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum ImpactConfidence {
    Exact,
    HighConfidence,
    Probable,
    Ambiguous,
    Unknown,
}

impl ImpactConfidence {
    /// 保守传播：取两者中更弱的一个（§46；枚举序 Exact=0 最强 ⇒ weaker = 序号更大者）。
    pub fn weakest(self, other: ImpactConfidence) -> ImpactConfidence {
        if self >= other {
            self
        } else {
            other
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            ImpactConfidence::Exact => "exact",
            ImpactConfidence::HighConfidence => "high_confidence",
            ImpactConfidence::Probable => "probable",
            ImpactConfidence::Ambiguous => "ambiguous",
            ImpactConfidence::Unknown => "unknown",
        }
    }
}

/// 影响理由（§33：一节点可有多条 reasons）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum ImpactReason {
    /// 关系边：calls 被改符号。
    CallsChangedSymbol,
    /// 关系边：传递调用链。
    TransitiveCall,
    /// 引用了被改符号（当前 M3 的符号级关系即调用；保留语义位）。
    ReferencesChangedSymbol,
    /// 依赖了被改模块。
    DependsOnChangedModule,
    /// 传递模块依赖。
    TransitiveDependency,
    /// 项目引用（workspace 内跨项目）。
    ProjectReference,
    /// 被改符号已删除且存在指向其名字的调用点（§20）。
    CallsDeletedSymbol,
    /// 文档中存在对该实体的已知关系证据。
    DocumentsChangedEntity,
}

/// 关系种类（§61：定义 impact 含义，不假设"重要性"）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum RelationKind {
    Calls,
    DependsOn,
    ProjectReferences,
}

/// 路径上的一跳（§30/§73：Why impacted 的展开单元）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PathHop {
    /// 起点（模块或符号 id）。
    pub from: String,
    pub relation: RelationKind,
    /// 终点。
    pub to: String,
    /// 该边在 M3 中的解析状态（"resolved"/"unresolved"/…，原样保留 §5）。
    pub resolution: String,
    pub confidence: ImpactConfidence,
}

/// 一条影响路径：Changed → relationship…→ target（§30）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImpactPath {
    /// 确定性 id：`ip:{source}#{target}#{distance}#{ordinal}`。
    pub id: String,
    /// 路径起点 = 被改实体（符号或模块 id）。
    pub source_change: String,
    /// 路径终点 = 受影响实体 id。
    pub target: String,
    pub distance: u32,
    pub hops: Vec<PathHop>,
}

/// 受影响（或被改）条目（§8/§83：多来源合并、reasons 多条、min distance）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImpactableNode {
    /// 实体 id：SymbolId / ModuleId / 文件模块 id / 文档相对路径。
    pub node_id: String,
    pub node_kind: NodeKind,
    /// 排序去重后的分类集合（§84）。
    pub impact_kinds: Vec<ImpactKind>,
    /// 最短影响距离（§81/§82：0=被改，N=关系跳数）。
    pub distance: u32,
    /// 保守传播后的置信度（§46）。
    pub confidence: ImpactConfidence,
    /// 排序去重后的理由集合（§33）。
    pub reasons: Vec<ImpactReason>,
    /// 到达该节点的不同证据路径条数（§34：合并重复目标，不重复列节点）。
    pub evidence_path_count: u64,
    /// 其余非最短路径条数（§82 additional_paths）。
    pub additional_paths: u64,
    /// 项目相对路径（展示用；项目级节点为空串）。
    pub path: String,
    /// 展示名（符号名/模块名/文档文件名）。
    pub name: String,
}

/// 一个风险信号（§40/§85/§86/§130）：事实计数 + 有界证据，无阈值、无总分。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RiskSignal {
    pub kind: RiskSignalKind,
    pub count: u64,
    /// 一句话事实依据。
    pub detail: String,
    /// 有界证据 id 列表（符号/模块/文件 id，按序截断）。
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum RiskSignalKind {
    /// public/可见性变更或导出变更的符号被改（§42；不是 BreakingChange）。
    PublicApiChanged,
    /// 被删除符号仍有指向其名字的调用点（§20/§96）。
    DeletedReferencedSymbol,
    /// 影响跨出被改文件所属模块（§40）。
    CrossModuleImpact,
    /// 存在未解析关系（§44）。
    UnresolvedRelationship,
    /// M5 符号映射存在歧义（§83 术语 AmbiguousSymbolMapping）。
    AmbiguousSymbolMapping,
    /// 配置文件发生变更（§53：仅事实信号）。
    ConfigurationFileChanged,
}

/// 影响诊断（§128；复用 M5 命名约定）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum ImpactDiagnosticKind {
    UnresolvedRelationship,
    AmbiguousRelationship,
    UnmappedChange,
    UnsupportedLanguage,
    ParseFailure,
    TraversalDepthExceeded,
    ImpactNodeLimitReached,
    Cancelled,
    HistoricalModelUnavailable,
    IncompleteGraph,
    NoDocumentationEvidence,
}

impl ImpactDiagnosticKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            ImpactDiagnosticKind::UnresolvedRelationship => "unresolved_relationship",
            ImpactDiagnosticKind::AmbiguousRelationship => "ambiguous_relationship",
            ImpactDiagnosticKind::UnmappedChange => "unmapped_change",
            ImpactDiagnosticKind::UnsupportedLanguage => "unsupported_language",
            ImpactDiagnosticKind::ParseFailure => "parse_failure",
            ImpactDiagnosticKind::TraversalDepthExceeded => "traversal_depth_exceeded",
            ImpactDiagnosticKind::ImpactNodeLimitReached => "impact_node_limit_reached",
            ImpactDiagnosticKind::Cancelled => "cancelled",
            ImpactDiagnosticKind::HistoricalModelUnavailable => "historical_model_unavailable",
            ImpactDiagnosticKind::IncompleteGraph => "incomplete_graph",
            ImpactDiagnosticKind::NoDocumentationEvidence => "no_documentation_evidence",
        }
    }
}

/// 一条影响诊断。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImpactDiagnostic {
    pub kind: ImpactDiagnosticKind,
    /// 相关路径或 id；全局诊断为 None。
    pub subject: Option<String>,
    pub detail: String,
}

/// 汇总统计（§129；全部为事实计数，不含任何风险评价）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ImpactSummary {
    pub changed_files: u64,
    pub changed_symbols: u64,
    pub changed_modules: u64,
    pub direct_impacted_modules: u64,
    pub direct_impacted_symbols: u64,
    pub transitive_impacted_modules: u64,
    pub transitive_impacted_symbols: u64,
    pub potentially_affected_tests: u64,
    pub documentation_candidates: u64,
    pub unresolved_relationships: u64,
    pub ambiguous_relationships: u64,
    /// 关系跳数表示的最大影响距离（§81）。
    pub max_distance: u32,
    pub diagnostics_total: u64,
}

/// M6 根模型（§7/§126）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImpactAnalysis {
    pub schema_version: u32,
    /// 确定性 id：`impact:{change_set_id}#{depth}#{nodes}`。
    pub id: String,
    pub change_set_id: String,
    /// 变更范围（透传 M5 scope 字符串）。
    pub scope: String,
    pub analysis_completeness: ImpactCompleteness,
    /// 达到上限时为 true（§37/§78；valid but truncated ≠ failed，§90）。
    pub analysis_incomplete: bool,
    /// 被改条目（DirectlyChanged；与 impacted 严格分离，§48/§49）。
    pub changed_items: Vec<ImpactableNode>,
    /// 潜在受影响候选（Potential Impact，§0）。
    pub impacted_items: Vec<ImpactableNode>,
    /// 影响路径（§30；按 (source,target,distance,hops) 排序）。
    pub impact_paths: Vec<ImpactPath>,
    pub summary: ImpactSummary,
    /// 按 (kind, subject, detail) 排序。
    pub diagnostics: Vec<ImpactDiagnostic>,
    /// 按 kind 排序（§130）。
    pub risk_signals: Vec<RiskSignal>,
}

impl ImpactAnalysis {
    /// 确定性 id（§6 同源思路：同输入 ⇒ 同 id）。
    pub fn derive_id(change_set_id: &str, max_depth: u32, max_nodes: u64) -> String {
        format!("impact:{change_set_id}#{max_depth}#{max_nodes}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enums_serialize_snake_case() {
        assert_eq!(
            serde_json::to_string(&ImpactKind::TestCandidate).unwrap(),
            "\"test_candidate\""
        );
        assert_eq!(
            serde_json::to_string(&ImpactCompleteness::Truncated).unwrap(),
            "\"truncated\""
        );
        assert_eq!(
            serde_json::to_string(&ImpactReason::CallsDeletedSymbol).unwrap(),
            "\"calls_deleted_symbol\""
        );
        assert_eq!(
            ImpactDiagnosticKind::TraversalDepthExceeded.as_str(),
            "traversal_depth_exceeded"
        );
    }

    #[test]
    fn confidence_weakest_propagates_conservatively() {
        use ImpactConfidence::*;
        assert_eq!(Exact.weakest(HighConfidence), HighConfidence);
        assert_eq!(Probable.weakest(Exact), Probable);
        // Unknown 最弱：取弱得 Unknown，不得静默升为 Ambiguous（§46）。
        assert_eq!(Ambiguous.weakest(Unknown), Unknown);
        // 序：Exact < HighConfidence < Probable < Ambiguous < Unknown
        assert!(Exact < HighConfidence && HighConfidence < Probable);
    }

    #[test]
    fn identity_is_deterministic() {
        let a = ImpactAnalysis::derive_id("changeset:demo#1#2#working_tree", 10, 2000);
        let b = ImpactAnalysis::derive_id("changeset:demo#1#2#working_tree", 10, 2000);
        assert_eq!(a, b);
        assert!(a.starts_with("impact:changeset:demo"));
    }

    #[test]
    fn summary_defaults_to_zero_facts() {
        let summary = ImpactSummary::default();
        assert_eq!(summary.changed_files, 0);
        assert_eq!(summary.max_distance, 0);
        assert_eq!(summary.diagnostics_total, 0);
    }
}
