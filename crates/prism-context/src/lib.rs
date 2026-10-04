pub mod collectors;
pub mod doc_health;
pub mod health;
pub mod materializer;
pub mod model;
pub mod recipe;
pub mod render;
pub mod selector;
pub mod service;
pub mod task;

pub use collectors::{collect_candidates, FactsInputs};
pub use doc_health::{
    command_drift, extract_documented_commands, extract_documented_paths, normalize_command,
    path_drift, DocumentedCommand,
};
pub use health::{
    evaluate, ContextCoverage, ContextHealth, ContextHealthFinding, ContextHealthStatus,
    HealthInputs,
};
pub use materializer::{classify_path, materialize, MaterializeReport};
pub use model::{
    BudgetReport, ContextBudget, ContextDiagnostic, ContextFocus, ContextGroup, ContextItem,
    ContextManifest, ContextPack, ContextRange, ContextRequest, ContextScope, ContextSource,
    ContextSummary, ContextTarget, DetailLevel, ExcludedCandidate, ExcludedReason,
    FileContentLevel, PackStatus, Priority, SelectionReason, SelectionTrace, SourceType,
    TargetResolution, TaskClassification, TaskKind, TaskSpecificity, CONTEXT_SCHEMA_VERSION,
};
pub use recipe::ContextRecipe;
pub use render::{render_markdown, render_plain_text, strip_control};
pub use selector::{
    apply_scope, estimate_candidate, normalize, prioritize, select, InvalidBudget, SelectionOutput,
};
pub use service::{render_text, ContextService};
pub use task::{classify, extract_target, resolve_target};
