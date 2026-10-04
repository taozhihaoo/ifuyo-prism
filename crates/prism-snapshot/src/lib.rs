pub mod capture;
pub mod cli_render;
pub mod compare;
pub mod model;
pub mod report;
pub mod timeline;
pub use cli_render::{load_snapshot_json, render_compare, render_history_export, render_timeline};
pub use capture::{
    resolve_project_identity, DomainFacts, SnapshotCaptureOptions, SnapshotCaptureService,
};
pub use model::{
    canonicalize, combine_fingerprints, fingerprint_value, fnv1a_hex, CaptureStatus,
    DomainFreshness, DomainStatus, ProjectSnapshot, SnapshotBasis, SnapshotCaptureResult,
    SnapshotDomainState, SNAPSHOT_SCHEMA_VERSION,
};
pub use prism_git;
pub use compare::{compare_domain, set_diff, DomainDiff, FindingDiffEntry, FindingLifecycle, SnapshotDiff, VerificationTransition};
pub use timeline::{build_timeline, GitCommitFact, Timeline, TimelineEvent, TimelineEventKind, TimelineSource};
pub use report::{counts_from_json, evolution_summary, render_compare_markdown, render_history_export_json, render_timeline_markdown, EvolutionEntry, EvolutionSummary, HistoryExport};
