//! Timeline 推导（规格 M11b §43–§51/§88–§90/§125/§152–§156/§232–§233）。
//!
//! TimelineEvent 是**派生 view model**（§45）：Snapshot + Git 为事实来源，
//! 不落第二张表（§232 可随时重建）。§88 不伪造历史——Prism 专属历史从
//! 第一次 snapshot 开始；§89 无 snapshot 时仍可显示 Git timeline。

use crate::model::ProjectSnapshot;
use serde::{Deserialize, Serialize};

/// 事件类型（§45）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimelineEventKind {
    GitCommit,
    SnapshotCaptured,
    ArchitectureChanged,
    HealthChanged,
    VerificationChanged,
    EnvironmentChanged,
    DocumentationChanged,
    ContextHealthChanged,
}

impl TimelineEventKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            TimelineEventKind::GitCommit => "git_commit",
            TimelineEventKind::SnapshotCaptured => "snapshot_captured",
            TimelineEventKind::ArchitectureChanged => "architecture_changed",
            TimelineEventKind::HealthChanged => "health_changed",
            TimelineEventKind::VerificationChanged => "verification_changed",
            TimelineEventKind::EnvironmentChanged => "environment_changed",
            TimelineEventKind::DocumentationChanged => "documentation_changed",
            TimelineEventKind::ContextHealthChanged => "context_health_changed",
        }
    }
}

/// 来源标签（§90：Git 原生事实 vs Prism 推导）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimelineSource {
    Git,
    PrismSnapshot,
    Derived,
}

impl TimelineSource {
    pub const fn as_str(self) -> &'static str {
        match self {
            TimelineSource::Git => "git",
            TimelineSource::PrismSnapshot => "prism_snapshot",
            TimelineSource::Derived => "derived",
        }
    }
}

/// TimelineEvent（§44）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimelineEvent {
    /// §152：确定性身份（timestamp|kind|ref）——派生对象不持久化独立 ID。
    pub id: String,
    pub timestamp: String,
    pub project_id: String,
    pub kind: TimelineEventKind,
    pub snapshot_id: Option<String>,
    pub git_commit: Option<String>,
    pub title: String,
    pub summary: String,
    pub source: TimelineSource,
}

/// Timeline（§125）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Timeline {
    pub project_id: String,
    pub events: Vec<TimelineEvent>,
    pub limitations: Vec<String>,
}

/// Git 提交事实（来自 M4，只读）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitCommitFact {
    pub id: String,
    pub timestamp: String,
    pub subject: String,
}

/// 从快照列表 + Git 提交推导时间线（§232：Snapshots + Git → TimelineBuilder）。
///
/// 规则：
/// - 每 snapshot ⇒ SnapshotCaptured 事件（§90 source = prism_snapshot）
/// - 快照间的域变化 ⇒ Derived 变化事件（§233：每个 (snapshot对, 域) 只一条）
/// - Git commits（§89：Git 原生事实；不冒充 ProjectSnapshot）
/// - 排序 timestamp ASC，同 timestamp 按 source/kind/id 稳定（§153）
pub fn build_timeline(
    project_id: &str,
    snapshots: &[ProjectSnapshot],
    git_commits: &[GitCommitFact],
) -> Timeline {
    let mut events: Vec<TimelineEvent> = Vec::new();

    // ---- Git 事件（§89：Git timeline 可独立于 snapshot 存在）----
    for commit in git_commits {
        events.push(TimelineEvent {
            id: deterministic_id(commit.timestamp.as_str(), "git_commit", &commit.id),
            timestamp: commit.timestamp.clone(),
            project_id: project_id.to_string(),
            kind: TimelineEventKind::GitCommit,
            snapshot_id: None,
            git_commit: Some(commit.id.clone()),
            title: format!("Commit {}", commit.id),
            summary: commit.subject.clone(),
            source: TimelineSource::Git,
        });
    }

    // ---- 快照事件（按 captured_at 升序处理，便于 diff 派生）----
    let mut ordered: Vec<&ProjectSnapshot> = snapshots.iter().collect();
    ordered.sort_by(|a, b| a.captured_at.cmp(&b.captured_at).then_with(|| a.id.cmp(&b.id)));

    let mut previous: Option<&ProjectSnapshot> = None;
    for snapshot in &ordered {
        events.push(TimelineEvent {
            id: deterministic_id(snapshot.captured_at.as_str(), "snapshot_captured", &snapshot.id),
            timestamp: snapshot.captured_at.clone(),
            project_id: project_id.to_string(),
            kind: TimelineEventKind::SnapshotCaptured,
            snapshot_id: Some(snapshot.id.clone()),
            git_commit: snapshot.git_revision.clone(),
            title: "Prism snapshot".to_string(),
            summary: format!(
                "basis={} state={} files={} findings={}",
                snapshot.basis.as_str(),
                &snapshot.state_fingerprint[..8.min(snapshot.state_fingerprint.len())],
                snapshot.counts.get("files").copied().unwrap_or(0),
                snapshot.counts.get("findings").copied().unwrap_or(0),
            ),
            source: TimelineSource::PrismSnapshot,
        });

        // §47/§233：变化事件从相邻 snapshot 对派生（一个域一条，不重复）。
        if let Some(previous_snapshot) = previous {
            let changed_domains: Vec<&str> = DOMAINS_CHANGED
                .iter()
                .copied()
                .filter(|domain| {
                    fingerprint_of(previous_snapshot, domain) != fingerprint_of(snapshot, domain)
                })
                .collect();
            for domain in changed_domains {
                let (kind, source) = domain_event(domain);
                events.push(TimelineEvent {
                    id: deterministic_id(
                        snapshot.captured_at.as_str(),
                        kind.as_str(),
                        &format!("{}->{}", previous_snapshot.id, snapshot.id),
                    ),
                    timestamp: snapshot.captured_at.clone(),
                    project_id: project_id.to_string(),
                    kind,
                    snapshot_id: Some(snapshot.id.clone()),
                    git_commit: None,
                    title: format!("{domain} changed"),
                    summary: format!(
                        "between snapshots {} -> {}",
                        short(&previous_snapshot.id),
                        short(&snapshot.id)
                    ),
                    source,
                });
            }
        }
        previous = Some(snapshot);
    }

    events.sort_by(|a, b| {
        a.timestamp
            .cmp(&b.timestamp)
            .then_with(|| source_rank(a.source).cmp(&source_rank(b.source)))
            .then_with(|| a.kind.as_str().cmp(b.kind.as_str()))
            .then_with(|| a.id.cmp(&b.id))
    });

    let mut limitations = Vec::new();
    if snapshots.is_empty() {
        // §88：不伪造 Prism 历史。
        limitations.push(
            "No Prism snapshots yet — Git timeline only; Prism-specific history \
             starts with the first snapshot."
                .to_string(),
        );
    }
    Timeline {
        project_id: project_id.to_string(),
        events,
        limitations,
    }
}

const DOMAINS_CHANGED: [&str; 8] = [
    "architecture",
    "health",
    "verification",
    "environment",
    "documentation",
    "context_health",
    "change",
    "code",
];

fn fingerprint_of(snapshot: &ProjectSnapshot, domain: &str) -> String {
    match domain {
        "architecture" => snapshot.architecture_fingerprint.clone(),
        "health" => snapshot.health_fingerprint.clone(),
        "verification" => snapshot.verification_fingerprint.clone(),
        "environment" => snapshot.environment_fingerprint.clone(),
        "context_health" => snapshot.context_health_fingerprint.clone(),
        "code" => snapshot.code_fingerprint.clone(),
        "change" => snapshot.change_fingerprint.clone(),
        "structure" => snapshot.structure_fingerprint.clone(),
        "git" => snapshot.git_fingerprint.clone(),
        "impact" => snapshot.impact_fingerprint.clone(),
        _ => String::new(),
    }
}

/// 域 → 事件类型/来源（§45）。
fn domain_event(domain: &str) -> (TimelineEventKind, TimelineSource) {
    match domain {
        "architecture" => (TimelineEventKind::ArchitectureChanged, TimelineSource::Derived),
        "health" => (TimelineEventKind::HealthChanged, TimelineSource::Derived),
        "verification" => (TimelineEventKind::VerificationChanged, TimelineSource::Derived),
        "environment" => (TimelineEventKind::EnvironmentChanged, TimelineSource::Derived),
        "documentation" => (TimelineEventKind::DocumentationChanged, TimelineSource::Derived),
        "context_health" => (TimelineEventKind::ContextHealthChanged, TimelineSource::Derived),
        _ => (TimelineEventKind::SnapshotCaptured, TimelineSource::Derived),
    }
}

fn source_rank(source: TimelineSource) -> u8 {
    match source {
        TimelineSource::Git => 0,
        TimelineSource::PrismSnapshot => 1,
        TimelineSource::Derived => 2,
    }
}

fn deterministic_id(timestamp: &str, kind: &str, reference: &str) -> String {
    crate::model::fnv1a_hex(&format!("{timestamp}|{kind}|{reference}"))
}

fn short(id: &str) -> String {
    id.chars().take(13).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn snapshot(id: &str, captured_at: &str, architecture: &str) -> ProjectSnapshot {
        let mut counts = BTreeMap::new();
        counts.insert("files".to_string(), 10u64);
        counts.insert("findings".to_string(), 3u64);
        let mut fingerprints = BTreeMap::new();
        for domain in [
            "structure", "code", "architecture", "git", "change", "impact", "health",
            "verification", "environment", "context_health",
        ] {
            let value = if domain == "architecture" { architecture } else { "same" };
            fingerprints.insert(domain.to_string(), crate::model::fnv1a_hex(value));
        }
        ProjectSnapshot {
            id: id.to_string(),
            project_id: "proj:test".to_string(),
            schema_version: 1,
            captured_at: captured_at.to_string(),
            duration_ms: 0,
            basis: crate::model::SnapshotBasis::WorkingTree,
            git_revision: None,
            git_branch: None,
            working_tree_state: "dirty".to_string(),
            project_name: "demo".to_string(),
            project_root: "C:/tmp/demo".to_string(),
            state_fingerprint: "state".to_string(),
            structure_fingerprint: crate::model::fnv1a_hex("same"),
            code_fingerprint: crate::model::fnv1a_hex("same"),
            architecture_fingerprint: crate::model::fnv1a_hex(architecture),
            git_fingerprint: crate::model::fnv1a_hex("same"),
            change_fingerprint: crate::model::fnv1a_hex("same"),
            impact_fingerprint: crate::model::fnv1a_hex("same"),
            health_fingerprint: crate::model::fnv1a_hex("same"),
            verification_fingerprint: crate::model::fnv1a_hex("same"),
            environment_fingerprint: crate::model::fnv1a_hex("same"),
            context_health_fingerprint: crate::model::fnv1a_hex("same"),
            source_versions: BTreeMap::new(),
            counts,
            domains: vec![],
            limitations: vec![],
            note: None,
        }
    }

    // §45/§47：snapshot + git 都进入 timeline。
    #[test]
    fn timeline_combines_git_and_snapshots() {
        let snapshots = vec![snapshot("snap:1", "2026-10-01T00:00:00Z", "arch-a")];
        let commits = vec![GitCommitFact {
            id: "abc123".to_string(),
            timestamp: "2026-09-30T00:00:00Z".to_string(),
            subject: "base commit".to_string(),
        }];
        let timeline = build_timeline("proj:test", &snapshots, &commits);
        assert_eq!(timeline.events.len(), 2);
        assert_eq!(timeline.events[0].kind, TimelineEventKind::GitCommit);
        assert_eq!(timeline.events[0].source, TimelineSource::Git);
        assert_eq!(timeline.events[1].kind, TimelineEventKind::SnapshotCaptured);
        assert_eq!(timeline.events[1].source, TimelineSource::PrismSnapshot);
    }

    // §47/§233：architecture 指纹变化 ⇒ 恰好一条 ArchitectureChanged（不重复）。
    #[test]
    fn domain_change_derives_single_event() {
        let snapshots = vec![
            snapshot("snap:1", "2026-10-01T00:00:00Z", "arch-a"),
            snapshot("snap:2", "2026-10-02T00:00:00Z", "arch-b"),
        ];
        let timeline = build_timeline("proj:test", &snapshots, &[]);
        let arch_events: Vec<_> = timeline
            .events
            .iter()
            .filter(|event| event.kind == TimelineEventKind::ArchitectureChanged)
            .collect();
        assert_eq!(arch_events.len(), 1, "§233 dedup");
        assert_eq!(arch_events[0].source, TimelineSource::Derived, "§90 source label");
    }

    // §153：排序稳定且 timestamp ASC。
    #[test]
    fn timeline_ordering_is_stable() {
        let snapshots = vec![
            snapshot("snap:2", "2026-10-02T00:00:00Z", "arch-b"),
            snapshot("snap:1", "2026-10-01T00:00:00Z", "arch-a"),
        ];
        let commits = vec![GitCommitFact {
            id: "abc".to_string(),
            timestamp: "2026-10-02T00:00:00Z".to_string(),
            subject: "s".to_string(),
        }];
        let timeline = build_timeline("proj:test", &snapshots, &commits);
        let timestamps: Vec<&str> = timeline
            .events
            .iter()
            .map(|event| event.timestamp.as_str())
            .collect();
        let mut sorted = timestamps.clone();
        sorted.sort();
        assert_eq!(timestamps, sorted, "§153 timestamp ASC");
    }

    // §88：无 snapshot ⇒ 只有 git + 明确 limitation。
    #[test]
    fn no_snapshots_yields_git_only_with_limitation() {
        let commits = vec![GitCommitFact {
            id: "abc".to_string(),
            timestamp: "2026-09-30T00:00:00Z".to_string(),
            subject: "base".to_string(),
        }];
        let timeline = build_timeline("proj:test", &[], &commits);
        assert_eq!(timeline.events.len(), 1);
        assert!(timeline
            .limitations
            .iter()
            .any(|limitation| limitation.contains("starts with the first snapshot")),
            "§88 no fabricated Prism history");
    }
}
