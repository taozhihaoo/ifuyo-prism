//! Debug 数据模型（M210 §六-§十/§十七/§四十五）。
//!
//! 三层身份（§九）：Event（某一次发生）/ Error（规范化签名）/ Cluster
//! （同源分组）。指纹稳定：无 GUID/时间戳/行号/自增 id。

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// M210 输出契约版本。
pub const DEBUG_SCHEMA_VERSION: u32 = 1;

/// 输入源类型（§六：不强制全实现，能力如实）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceType {
    LogFile,
    BuildOutput,
    TestOutput,
    GodotLog,
    DotNetException,
    PastedText,
    Unknown,
}

/// 标准化级别 + 原始级别分离（§七：不强制 Debug/Info/Warning/Error 四级）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NormalizedLevel {
    Trace,
    Debug,
    Info,
    Warning,
    Error,
    Critical,
    Unknown,
}

impl NormalizedLevel {
    /// 原样级别词 → 标准化（ERR/FATAL/CRITICAL/VERBOSE/NOTICE…）。
    pub fn normalize(raw: &str) -> Self {
        let lowered = raw.trim().to_ascii_lowercase();
        match lowered.as_str() {
            "trace" | "verbose" | "v" => NormalizedLevel::Trace,
            "debug" | "dbg" | "d" => NormalizedLevel::Debug,
            "info" | "notice" | "i" => NormalizedLevel::Info,
            "warning" | "warn" | "w" => NormalizedLevel::Warning,
            "error" | "err" | "e" | "script error" => NormalizedLevel::Error,
            "fatal" | "critical" | "crash" | "panic" => NormalizedLevel::Critical,
            _ => NormalizedLevel::Unknown,
        }
    }
}

/// 事件域（§十四：Build/Test/Runtime 来源区别保留，不混为一种 Error）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventDomain {
    Runtime,
    Build,
    Test,
    Import,
    Unknown,
}

/// 解析完整度（§三十三：Partial/Malformed 如实，不伪装 Complete）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParseState {
    Complete,
    Partial,
    Malformed,
    Unsupported,
}

/// StackFrame → Symbol 解析状态（§十：栈存在但解析不到本地 Symbol 也是
/// 有效 Debug 数据）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResolutionStatus {
    Resolved,
    Unresolved,
    Ambiguous,
    Unsupported,
    Unavailable,
}

/// 匹配强度（§十一：Exact=FACT，Probable=INFERENCE——Origin 分离）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchState {
    Exact,
    Strong,
    Probable,
    Ambiguous,
    Unresolved,
    Unsupported,
}

/// 输入源（§六）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebugSource {
    /// `src:{fnv1a16(逻辑路径)}`——轮转片段（app.log.1）归并到同一逻辑 id。
    pub source_id: String,
    pub source_type: SourceType,
    /// 项目相对路径（逻辑源：不带轮转后缀）。
    pub logical_path: String,
    /// 物理文件（本次实际读取的）。
    pub physical_path: String,
    pub content_hash: String,
    pub bytes: u64,
    /// 解析器标识（如 "godot-log/1"；String 以可序列化，§71 契约）。
    pub parser: String,
    pub parse_state: ParseState,
    /// 本源解析中发生的脱敏替换数（§二十九）。
    pub redactions: u64,
    /// 解析诊断（malformed/unsupported 原因）。
    pub diagnostics: Vec<String>,
}

/// 单条规范化事件（§七）。消息为 redacted 表示——原文不进入模型（§二十九）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebugEvent {
    /// `evt:{fnv1a16(source_id|line|normalized_message)}`。
    pub event_id: String,
    pub source_id: String,
    /// 源文件内 1-based 行号（证据定位，非身份）。
    pub line: usize,
    pub timestamp: Option<String>,
    pub level: NormalizedLevel,
    pub original_level: Option<String>,
    pub domain: EventDomain,
    /// redacted 消息（有界，§四十七 max message length）。
    pub message: String,
    /// 测试名（Test 域，§十六）。
    pub test_name: Option<String>,
    /// 错误码（Build 域，如 CS0246，§十五）。
    pub code: Option<String>,
    /// 编译诊断/构建错误关联的文件（Build 域）。
    pub file: Option<String>,
    pub line_in_file: Option<u32>,
}

/// 栈帧（§十）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StackFrame {
    pub index: usize,
    pub module: Option<String>,
    pub type_name: Option<String>,
    pub method: Option<String>,
    pub file_path: Option<String>,
    pub line: Option<u32>,
    pub column: Option<u32>,
    /// M23/M24 关联结果（§十一/§十二；未注入引擎 ⇒ Unavailable）。
    pub resolution: FrameResolution,
}

/// 帧解析结论（§十一：Exact=FACT，Probable=INFERENCE，含依据）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrameResolution {
    pub status: ResolutionStatus,
    pub match_state: MatchState,
    /// M23 逻辑符号 id（Resolved 时）。
    pub symbol_id: Option<String>,
    /// M24 关联：脚本文件 → 挂接节点 → 场景（有证据才填，§十二）。
    pub scene: Option<String>,
    pub node_path: Option<String>,
    /// 判定依据（可解释 §六十六）。
    pub reasoning: String,
}

/// 栈（§十）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StackTrace {
    /// `trace:{fnv1a16(frames 归一表示)}`——同栈同 id。
    pub trace_id: String,
    pub frames: Vec<StackFrame>,
}

/// 规范化错误签名（§八/§九）：同指纹的事件聚合，保留次数与例证。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebugError {
    /// `err:{fingerprint}`。
    pub error_id: String,
    pub fingerprint: String,
    pub domain: EventDomain,
    /// 异常类型原样（`System.NullReferenceException`）；非异常错误为 None。
    pub exception_type: Option<String>,
    /// 规范化消息（§八：动态变量归一，不过度合并）。
    pub normalized_message: String,
    /// 首个例证的 redacted 原始消息。
    pub example_message: String,
    pub code: Option<String>,
    /// 归一 top 帧（参与指纹，cap 3）。
    pub top_frames: Vec<String>,
    pub stack_trace: Option<StackTrace>,
    pub occurrences: u64,
    /// 例证位置（source_id + line，cap 5；§五 证据链）。
    pub occurrences_detail: Vec<(String, usize)>,
    pub first_observed: Option<String>,
    pub last_observed: Option<String>,
    pub redaction_count: u64,
}

/// 聚类解释（§十七：为什么归为一类——逐项依据，不是裸相似度）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClusterExplanation {
    pub same_exception_type: bool,
    pub same_normalized_message: bool,
    pub same_top_frames: bool,
    pub same_error_code: bool,
    /// 人类可读归因（"identical fingerprint components: type+message+frames"）。
    pub reasoning: String,
}

/// 错误簇（§十七）：v1 = 精确指纹分组（保守；相似度打分 = Unsupported，
/// 如实记诊断，§17 不产出无解释的相似度数字）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorCluster {
    /// `cluster:{fingerprint}`——v1 与精确签名 1:1（保守聚类）。
    pub cluster_id: String,
    pub fingerprint: String,
    pub error_ids: Vec<String>,
    pub occurrences: u64,
    pub first_observed: Option<String>,
    pub last_observed: Option<String>,
    pub explanation: ClusterExplanation,
}

/// 一次分析运行（§四十五：build/test/import/manual capture）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebugRun {
    /// `run:{fnv1a16(source_ids|started)}`。
    pub run_id: String,
    pub run_type: EventDomain,
    pub started: String,
    pub source_ids: Vec<String>,
    pub event_count: u64,
}

/// Godot 运行时场景关联（§十二/§十三：仅 M24 证据支持时建立）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GodotMapping {
    pub scene: String,
    pub node_path: String,
}

/// 分析报告（M28 规则层/健康层的注入对象）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct DebugReport {
    pub schema_version: u32,
    pub sources: Vec<DebugSource>,
    /// 事件（有界；total 如实记录截断前数量）。
    pub events: Vec<DebugEvent>,
    pub events_total: u64,
    pub events_truncated: bool,
    pub errors: Vec<DebugError>,
    pub clusters: Vec<ErrorCluster>,
    pub runs: Vec<DebugRun>,
    /// 引擎注入是否可用于符号解析（§十：Unavailable 如实）。
    pub symbol_resolution_available: bool,
    pub redaction_count: u64,
    pub parser_failures: u64,
    pub unsupported_sources: u64,
    /// 解析/聚类诊断（§四十七：截断详情，绝不静默丢弃）。
    pub diagnostics: Vec<String>,
}

impl DebugReport {
    /// 按指纹聚簇（§十七）：v1 精确指纹分组 + 逐项解释。
    pub fn build_clusters(errors: &[DebugError]) -> Vec<ErrorCluster> {
        let mut by_fingerprint: BTreeMap<&str, Vec<&DebugError>> = BTreeMap::new();
        for error in errors {
            by_fingerprint
                .entry(error.fingerprint.as_str())
                .or_default()
                .push(error);
        }
        let mut clusters = Vec::new();
        for (fingerprint, group) in by_fingerprint {
            let first = group[0];
            let explanation = ClusterExplanation {
                same_exception_type: group
                    .iter()
                    .all(|error| error.exception_type == first.exception_type),
                same_normalized_message: group
                    .iter()
                    .all(|error| error.normalized_message == first.normalized_message),
                same_top_frames: group.iter().all(|error| error.top_frames == first.top_frames),
                same_error_code: group.iter().all(|error| error.code == first.code),
                reasoning: "exact fingerprint match (type+normalized message+top frames); similarity scoring unsupported in this version".to_string(),
            };
            clusters.push(ErrorCluster {
                cluster_id: format!("cluster:{fingerprint}"),
                fingerprint: fingerprint.to_string(),
                error_ids: group.iter().map(|error| error.error_id.clone()).collect(),
                occurrences: group.iter().map(|error| error.occurrences).sum(),
                first_observed: group
                    .iter()
                    .filter_map(|error| error.first_observed.clone())
                    .min(),
                last_observed: group
                    .iter()
                    .filter_map(|error| error.last_observed.clone())
                    .max(),
                explanation,
            });
        }
        clusters.sort_by(|a, b| a.cluster_id.cmp(&b.cluster_id));
        clusters
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn error(fingerprint: &str, exception: Option<&str>, message: &str) -> DebugError {
        DebugError {
            error_id: format!("err:{fingerprint}"),
            fingerprint: fingerprint.to_string(),
            domain: EventDomain::Runtime,
            exception_type: exception.map(str::to_string),
            normalized_message: message.to_string(),
            example_message: message.to_string(),
            code: None,
            top_frames: vec!["App.Battle.Resolve()".to_string()],
            stack_trace: None,
            occurrences: 2,
            occurrences_detail: vec![],
            first_observed: None,
            last_observed: None,
            redaction_count: 0,
        }
    }

    #[test]
    fn level_normalization_covers_extended_vocabulary() {
        // §七：ERR/FATAL/VERBOSE/NOTICE 都要落到标准化级别。
        assert_eq!(NormalizedLevel::normalize("ERR"), NormalizedLevel::Error);
        assert_eq!(
            NormalizedLevel::normalize("Fatal"),
            NormalizedLevel::Critical
        );
        assert_eq!(
            NormalizedLevel::normalize("VERBOSE"),
            NormalizedLevel::Trace
        );
        assert_eq!(NormalizedLevel::normalize("notice"), NormalizedLevel::Info);
        assert_eq!(
            NormalizedLevel::normalize("weird"),
            NormalizedLevel::Unknown
        );
    }

    #[test]
    fn clustering_is_deterministic_and_explained() {
        let errors = vec![
            error("f1", Some("System.NullReferenceException"), "msg a"),
            error("f1", Some("System.NullReferenceException"), "msg a"),
            error("f2", Some("System.IO.IOException"), "msg b"),
        ];
        let clusters = DebugReport::build_clusters(&errors);
        assert_eq!(clusters.len(), 2);
        let big = clusters
            .iter()
            .find(|cluster| cluster.fingerprint == "f1")
            .unwrap();
        assert_eq!(big.occurrences, 4);
        assert!(big.explanation.same_exception_type);
        assert!(big.explanation.same_normalized_message);
        assert!(big.explanation.reasoning.contains("exact fingerprint"));
        // 同输入同顺序（§四十八）。
        let again = DebugReport::build_clusters(&errors);
        assert_eq!(clusters, again);
    }
}
