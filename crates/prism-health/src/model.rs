//! Finding 数据模型（规格 M7 §5/§6/§13/§14/§68/§121–§125/§139/§140）。
//!
//! 身份契约（§6/§198）：FindingId = `finding:{rule_id}:{stable_key}`，
//! stable_key 由规则从 rule + 项目相对位置 + 相关实体身份确定性派生；
//! 禁随机 UUID（M11 Snapshot 依赖跨运行同一问题 = 同一 id）。
//!
//! 证据契约（§13/§14）：只存位置/身份/关系/小结构事实，绝不复制源码。

use prism_core::severity::{Confidence, Severity};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// 对外 JSON 契约版本。
pub const HEALTH_SCHEMA_VERSION: u32 = 1;

/// 稳定 Finding 身份（newtype；确定性派生入口见 [`Finding::derive_id`]）。
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FindingId(String);

impl FindingId {
    /// `finding:{rule_id}:{stable_key}`（§6）。
    pub fn derive(rule_id: &str, stable_key: &str) -> Self {
        FindingId(format!("finding:{rule_id}:{stable_key}"))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// 规则分类（§15：按实际规则数量，不为分类而分类）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum FindingCategory {
    Architecture,
    Dependency,
    Code,
    Security,
    Maintenance,
    Documentation,
    Configuration,
    Git,
    Environment,
}

/// 一处定位（§121：项目相对路径 + 1-based 行列；绝不用绝对路径做身份）。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FindingLocation {
    pub relative_path: String,
    pub line: u32,
    pub column: u32,
}

impl FindingLocation {
    pub fn new(relative_path: impl Into<String>, line: u32, column: u32) -> Self {
        FindingLocation {
            relative_path: relative_path.into(),
            line,
            column,
        }
    }
}

/// 证据种类（§13）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    /// 符号身份/范围事实。
    Symbol,
    /// 源码位置事实。
    SourceLocation,
    /// 文件元数据事实（语言/分类/大小）。
    FileMetadata,
    /// 关系边事实（import/依赖/调用，含解析状态）。
    Relationship,
    /// Git 事实。
    GitFact,
    /// 配置事实。
    ConfigFact,
}

/// 一条证据（§14：小结构事实；`detail` 一句话，不含源码/文件内容）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Evidence {
    pub kind: EvidenceKind,
    /// 证据指向的实体（相对路径/符号 id/边 id）。
    pub subject: String,
    pub detail: String,
    /// 可选精确定位。
    pub location: Option<FindingLocation>,
}

impl Evidence {
    pub fn new(kind: EvidenceKind, subject: impl Into<String>, detail: impl Into<String>) -> Self {
        Evidence {
            kind,
            subject: subject.into(),
            detail: detail.into(),
            location: None,
        }
    }

    pub fn with_location(mut self, location: FindingLocation) -> Self {
        self.location = Some(location);
        self
    }
}

/// Finding 生命周期（§68：第一版只有 Active/Suppressed/Ignored；
/// 不模拟 issue tracker 的 Open/Resolved 生命周期）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum FindingStatus {
    /// 检出且未被抑制。
    Active,
    /// 检出但被配置抑制（suppressed ≠ not detected，§67/§164）。
    Suppressed,
    /// 用户/基线明确忽略（预留；当前由 suppressed 配置驱动）。
    Ignored,
}

/// 一条健康发现（§5；字段按实际模型收敛）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Finding {
    /// 确定性 id（§6）。
    pub id: FindingId,
    /// 稳定规则 id（如 "MAINT001"；不绑实现文件，§16/§17）。
    pub rule_id: String,
    pub category: FindingCategory,
    /// M0 三级 severity（§11 复用）。
    pub severity: Severity,
    /// 与 severity 完全独立（§12）。
    pub confidence: Confidence,
    /// 一行标题（§140 措辞：Detected/Missing/Exceeds threshold…非评判词）。
    pub title: String,
    /// 一句话消息。
    pub message: String,
    /// 结构化解释（§139：What / Evidence / Why it matters——克制）。
    pub explanation: FindingExplanation,
    /// 主定位。
    pub location: FindingLocation,
    /// 多定位（§122：cycle 的 A/B/C）。
    pub related_locations: Vec<FindingLocation>,
    /// 相关实体（§123 第一版仅 id 列表，不做 finding graph）。
    pub related_symbols: Vec<String>,
    pub related_modules: Vec<String>,
    pub related_files: Vec<String>,
    pub evidence: Vec<Evidence>,
    /// 通用下一步（§124：克制；§125 无 auto-fix）。
    pub suggested_action: Option<String>,
    pub status: FindingStatus,
    /// 规则私有附加事实（确定性、小体量）。
    pub metadata: BTreeMap<String, String>,
    /// 与当前变更的关系（§58/§137；FullProject 全量时也有值，M10 可消费；
    /// 序列化时 None 省略，保持 M7a 契约向后兼容）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub change_relation: Option<ChangeRelation>,
}

/// What / Evidence / Why it matters 三段解释（§139）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FindingExplanation {
    pub what: String,
    pub evidence_summary: String,
    /// 克制的一句话影响说明（不得断言 "will break production"，§139/§140）。
    pub why_it_matters: Option<String>,
}

impl Finding {
    /// 确定性 id（§6）。
    pub fn derive_id(rule_id: &str, stable_key: &str) -> FindingId {
        FindingId::derive(rule_id, stable_key)
    }
}

impl HealthSummary {
    /// 从 findings 重建全部计数（过滤后重算用；§10）。
    pub fn from_findings<'a>(findings: impl Iterator<Item = &'a Finding>) -> Self {
        let mut summary = HealthSummary::default();
        let mut files_with = std::collections::BTreeSet::new();
        let mut modules_with = std::collections::BTreeSet::new();
        let mut symbols_with = std::collections::BTreeSet::new();
        for finding in findings {
            summary.total_findings += 1;
            match finding.severity {
                Severity::Critical => summary.critical_count += 1,
                Severity::Warning => summary.warning_count += 1,
                Severity::Info => summary.info_count += 1,
            }
            *summary
                .by_category
                .entry(format!("{:?}", finding.category).to_ascii_lowercase())
                .or_insert(0) += 1;
            *summary.by_rule.entry(finding.rule_id.clone()).or_insert(0) += 1;
            files_with.insert(finding.location.relative_path.clone());
            for module in &finding.related_modules {
                modules_with.insert(module.clone());
            }
            for symbol in &finding.related_symbols {
                symbols_with.insert(symbol.clone());
            }
        }
        summary.files_with_findings = files_with.len() as u64;
        summary.modules_with_findings = modules_with.len() as u64;
        summary.symbols_with_findings = symbols_with.len() as u64;
        summary
    }
}

/// 汇总统计（§10：全部为事实计数；禁止折算成任何 0-100 健康分，§4）。
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct HealthSummary {
    pub total_findings: u64,
    /// 三级 severity 计数（M0 词表；对外契约稳定）。
    pub critical_count: u64,
    pub warning_count: u64,
    pub info_count: u64,
    /// 分类 → 计数（按分类名排序，§79 确定性）。
    pub by_category: BTreeMap<String, u64>,
    /// 规则 → 计数。
    pub by_rule: BTreeMap<String, u64>,
    pub files_with_findings: u64,
    pub modules_with_findings: u64,
    pub symbols_with_findings: u64,
}

/// Finding 与当前变更的关系（§58/§137；只读事实，不改变 finding 本体）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeRelation {
    /// finding 位置在当前 ChangeSet 的变更文件内。
    DirectlyChanged,
    /// finding 位置在 M6 ImpactAnalysis 的受影响范围内（未直接变更）。
    Impacted,
    /// 与当前变更无关（scope=FullProject 时的默认）。
    Unrelated,
}

/// 单条规则的执行结果（§201：不直接改全局 HealthAnalysis）。
#[derive(Debug, Clone, Default)]
pub struct RuleResult {
    pub findings: Vec<Finding>,
    pub diagnostics: Vec<crate::model::HealthDiagnostic>,
}

/// 健康诊断（§208：只在意外局限/配置错误/分析失败时产生；
/// "规则不适用"不产生用户可见诊断）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthDiagnostic {
    /// 稳定种类串（如 "rule_failed" / "config_unknown_rule"）。
    pub kind: String,
    /// 相关规则 id 或路径；全局为 None。
    pub subject: Option<String>,
    pub detail: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Finding {
        Finding {
            id: Finding::derive_id("MAINT001", "src/a.ts:3:5:TODO"),
            rule_id: "MAINT001".to_string(),
            category: FindingCategory::Maintenance,
            severity: Severity::Info,
            confidence: Confidence::High,
            title: "Detected TODO marker".to_string(),
            message: "TODO marker at src/a.ts:3".to_string(),
            explanation: FindingExplanation {
                what: "A TODO marker was detected in a comment.".to_string(),
                evidence_summary: "line 3: // TODO: extract helper".to_string(),
                why_it_matters: Some("Marker comments often track unfinished work.".to_string()),
            },
            location: FindingLocation::new("src/a.ts", 3, 5),
            related_locations: vec![],
            related_symbols: vec![],
            related_modules: vec![],
            related_files: vec![],
            evidence: vec![Evidence::new(
                EvidenceKind::SourceLocation,
                "src/a.ts:3:5",
                "TODO marker in comment",
            )
            .with_location(FindingLocation::new("src/a.ts", 3, 5))],
            suggested_action: Some("Resolve or schedule the marked work.".to_string()),
            status: FindingStatus::Active,
            change_relation: None,
            metadata: BTreeMap::new(),
        }
    }

    #[test]
    fn finding_id_is_deterministic_and_rule_scoped() {
        let a = Finding::derive_id("MAINT001", "src/a.ts:3:5:TODO");
        let b = Finding::derive_id("MAINT001", "src/a.ts:3:5:TODO");
        let c = Finding::derive_id("MAINT002", "src/a.ts:3:5:TODO");
        assert_eq!(a, b);
        assert_ne!(a, c, "different rules ⇒ different ids");
        assert!(a.as_str().starts_with("finding:MAINT001:"));
    }

    #[test]
    fn finding_serializes_snake_case_with_independent_severity_confidence() {
        let json = serde_json::to_value(sample()).unwrap();
        assert_eq!(json["category"], "maintenance");
        assert_eq!(json["status"], "active");
        assert_eq!(json["severity"], "info");
        assert_eq!(json["confidence"], "high");
        assert_eq!(json["location"]["relative_path"], "src/a.ts");
        assert_eq!(
            json["explanation"]["what"],
            "A TODO marker was detected in a comment."
        );
        assert!(
            json.get("source_content").is_none(),
            "§14: no source in findings"
        );
    }

    #[test]
    fn evidence_stays_small_and_located() {
        let evidence = Evidence::new(
            EvidenceKind::Relationship,
            "dep:src/a.ts#./b#unresolved",
            "import './b' unresolved",
        )
        .with_location(FindingLocation::new("src/a.ts", 1, 10));
        let json = serde_json::to_value(&evidence).unwrap();
        assert_eq!(json["kind"], "relationship");
        assert_eq!(json["location"]["line"], 1);
    }
}
