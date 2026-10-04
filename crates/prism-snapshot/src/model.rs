//! ProjectSnapshot 领域模型（规格 M11a §6–§9/§13/§27/§67–§70/§123/§137/§151）。
//!
//! 域身份（§6）：十个域指纹 + 组合 state fingerprint；域状态（§13）沿用
//! M9/M10 Unknown 语义（Unavailable/Stale ≠ Failed）。

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Snapshot schema 版本（§68：显式版本，禁止假设永久兼容）。
pub const SNAPSHOT_SCHEMA_VERSION: u32 = 1;

/// 捕获基线（§5：两种主要状态 + Unknown）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SnapshotBasis {
    /// Git revision（干净树 ⇒ HEAD SHA）。
    GitRevision,
    /// 工作树（含未提交变更）。
    WorkingTree,
    /// 无法判定。
    Unknown,
}

impl SnapshotBasis {
    pub const fn as_str(self) -> &'static str {
        match self {
            SnapshotBasis::GitRevision => "git_revision",
            SnapshotBasis::WorkingTree => "working_tree",
            SnapshotBasis::Unknown => "unknown",
        }
    }
}

/// 域状态（§13）：Unavailable/Stale ≠ Failed（保持 Unknown 语义）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DomainStatus {
    /// 本次 capture 已捕获规范化事实。
    Available,
    /// 调用方未提供该域事实（如 CLI 无 session 的 M7/M8）。
    NotCaptured,
    /// 尝试过但不可用（权限/部分扫描失败）。
    Unavailable,
    /// 事实来自更早的状态（如 M8 在旧 revision 上执行）。
    Stale,
    Partial,
    /// 域与当前项目无关（如非 git 项目的 GitState）。
    NotApplicable,
}

impl DomainStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            DomainStatus::Available => "available",
            DomainStatus::NotCaptured => "not_captured",
            DomainStatus::Unavailable => "unavailable",
            DomainStatus::Stale => "stale",
            DomainStatus::Partial => "partial",
            DomainStatus::NotApplicable => "not_applicable",
        }
    }
}

/// 新鲜度（§12/§74：signal，不是正确性）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DomainFreshness {
    Fresh,
    Stale,
    Unknown,
}

impl DomainFreshness {
    pub const fn as_str(self) -> &'static str {
        match self {
            DomainFreshness::Fresh => "fresh",
            DomainFreshness::Stale => "stale",
            DomainFreshness::Unknown => "unknown",
        }
    }
}

/// 单域状态（§13 SnapshotDomainState）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotDomainState {
    pub domain: String,
    pub status: DomainStatus,
    pub captured_at: String,
    pub source_revision: Option<String>,
    pub input_fingerprint: Option<String>,
    pub content_fingerprint: Option<String>,
    pub freshness: DomainFreshness,
    /// 规范化后的域内容（canonical JSON Value，参与 fingerprint）。
    pub data: serde_json::Value,
    pub limitations: Vec<String>,
}

/// ProjectSnapshot（§7；§298 domain purity——无 UI/SQL 类型）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectSnapshot {
    /// SnapshotId（§151：稳定不可变 UUID，非 index/timestamp）。
    pub id: String,
    pub project_id: String,
    pub schema_version: u32,
    pub captured_at: String,
    pub duration_ms: u64,
    pub basis: SnapshotBasis,
    pub git_revision: Option<String>,
    pub git_branch: Option<String>,
    pub working_tree_state: String,
    pub project_name: String,
    pub project_root: String,
    /// §6：组合状态指纹（十域 canonical JSON 的组合 hash）。
    pub state_fingerprint: String,
    pub structure_fingerprint: String,
    pub code_fingerprint: String,
    pub architecture_fingerprint: String,
    pub git_fingerprint: String,
    pub change_fingerprint: String,
    pub impact_fingerprint: String,
    pub health_fingerprint: String,
    pub verification_fingerprint: String,
    pub environment_fingerprint: String,
    pub context_health_fingerprint: String,
    /// §69/§70：各域模型版本（判定新旧 snapshot 知识模型一致性）。
    pub source_versions: BTreeMap<String, String>,
    /// §27：事实统计（counts 是事实，不是评分）。
    pub counts: BTreeMap<String, u64>,
    pub domains: Vec<SnapshotDomainState>,
    pub limitations: Vec<String>,
    /// §91：用户备注（不自动生成主观标签）。
    pub note: Option<String>,
}

/// Capture 结果（§123：analysis 与 persistence 分离，§120/§122）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptureStatus {
    /// 新 snapshot 已创建。
    Created,
    /// §17：状态未变 ⇒ 去重，返回既有 snapshot id。
    Deduplicated,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotCaptureResult {
    pub status: CaptureStatus,
    /// 新建时为新 id；去重时为既有 snapshot id。
    pub snapshot_id: String,
    pub state_fingerprint: String,
    pub created: bool,
    pub deduplicated: bool,
    pub duration_ms: u64,
    pub domain_results: Vec<String>,
    pub warnings: Vec<String>,
    pub limitations: Vec<String>,
    /// §120：analysis 成功但持久化失败时为 false（不吞错误 §122）。
    pub persisted: bool,
}

/// Canonical JSON（§15）：serde_json Value 默认 BTreeMap ⇒ key 有序；
/// 数组元素排序（§158 集合顺序无关）、其余类型原样。
pub fn canonicalize(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Null => serde_json::Value::Null,
        serde_json::Value::Bool(b) => serde_json::Value::Bool(*b),
        serde_json::Value::Number(n) => serde_json::Value::Number(n.clone()),
        serde_json::Value::String(text) => serde_json::Value::String(text.clone()),
        serde_json::Value::Array(items) => {
            // Value 未实现 Ord：以 canonical 字符串为排序键（等价于对
            // canonical 形态排序，稳定性一致）。
            let mut canonical: Vec<serde_json::Value> =
                items.iter().map(canonicalize).collect();
            canonical.sort_by_key(|value| value.to_string());
            serde_json::Value::Array(canonical)
        }
        serde_json::Value::Object(map) => {
            let mut out = serde_json::Map::new();
            for (key, item) in map {
                out.insert(key.clone(), canonicalize(item));
            }
            serde_json::Value::Object(out)
        }
    }
}

/// FNV-1a 64-bit（无外部依赖；fingerprint 用途 §16——非密码学）。
pub fn fnv1a_hex(text: &str) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in text.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

/// 值的域指纹：canonical JSON 文本 → hash。
pub fn fingerprint_value(value: &serde_json::Value) -> String {
    fnv1a_hex(&canonicalize(value).to_string())
}

/// 组合状态指纹（§6）：按域名字典序拼接 "domain=hash" 再 hash。
pub fn combine_fingerprints(domain_fingerprints: &BTreeMap<String, String>) -> String {
    let mut combined = String::new();
    for (domain, hash) in domain_fingerprints {
        combined.push_str(domain);
        combined.push('=');
        combined.push_str(hash);
        combined.push(';');
    }
    fnv1a_hex(&combined)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // §158：same logical state → same fingerprint（集合顺序无关）。
    #[test]
    fn canonical_serialization_is_order_insensitive() {
        let a = json!({ "files": ["b.ts", "a.ts"], "counts": { "x": 1, "y": 2 } });
        let b = json!({ "counts": { "y": 2, "x": 1 }, "files": ["a.ts", "b.ts"] });
        assert_eq!(fingerprint_value(&a), fingerprint_value(&b));
    }

    // §205/§206：内容变化 ⇒ fingerprint 变化。
    #[test]
    fn fingerprint_changes_when_content_changes() {
        let before = json!({ "dependencies": ["a", "b"] });
        let after = json!({ "dependencies": ["a", "b", "c"] });
        assert_ne!(fingerprint_value(&before), fingerprint_value(&after));
    }

    // §6：组合指纹按域名字典序稳定（BTreeMap 已保证；双保险测试）。
    #[test]
    fn combined_fingerprint_is_stable_across_insertion_order() {
        let mut first = BTreeMap::new();
        first.insert("health".to_string(), "h1".to_string());
        first.insert("code".to_string(), "c1".to_string());
        let mut second = BTreeMap::new();
        second.insert("code".to_string(), "c1".to_string());
        second.insert("health".to_string(), "h1".to_string());
        assert_eq!(combine_fingerprints(&first), combine_fingerprints(&second));
    }

    #[test]
    fn canonical_handles_null_and_nesting() {
        let a = json!({ "x": null, "y": [{ "b": 2, "a": 1 }] });
        let canonical = canonicalize(&a);
        assert_eq!(canonical["y"][0]["a"], json!(1));
        assert!(canonical["x"].is_null());
    }
}
