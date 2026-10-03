//! prism-verify — Verification Facts 层（M8a：Verify Engine Core）。
//!
//! ```text
//! CheckDiscovery → VerifyPlan → prerequisites → CheckScheduler → CheckExecutor
//!   → OutputParser/ResultNormalizer → VerificationReport
//! ```
//!
//! 定位（§0/§3/§6）：Prism 是 **Verify Orchestrator**，不是工具作者；
//! 只回答实际执行事实（"npm test 以 exit code 1 结束"），不做预测。
//!
//! 边界：复用 M0 `CommandRunner`（§2，无第二套 runner）；无 shell 字符串
//! 执行（§13）；secret 脱敏（§77/§173-§176）；输出有界采集（§201/§202）；
//! WritePolicy 默认拒绝源码变更类命令（§85-§87）；Git 只读（§83）。

pub mod ci;
pub mod discovery;
pub mod engine;
pub mod executor;
pub mod fake;
pub mod model;
pub mod parsers;
pub mod release;
pub mod planner;
pub mod redact;
pub mod render;
pub mod tool;

pub use discovery::discover_checks;
pub use engine::{build_plan, run_verification};
pub use executor::{execute_check, execute_plan, execute_plan_observed, execute_plan_with, execute_plan_without_prereqs};
pub use fake::{FakeCommandRunner, FakeResponse};
pub use model::{
    CheckConfidence, CheckKind, CheckPolicy, CheckRun, CheckSource, CheckSpec, CheckState,
    OverallConclusion, PlannedCheck, Prerequisite, PrerequisiteOutcome, VerifiedState,
    VerifyCounts, VerifyDiagnostic, VerifyOptions, VerifyPlan, VerificationReport, WritePolicy,
    VERIFY_SCHEMA_VERSION,
};
pub use ci::{compare_with_local, inspect_github_actions, CICheck, CIInspection, VerificationCoverage};
pub use parsers::{parse_dotnet_line, parse_junit, parse_pytest_line, ParseOutcome, ToolDiagnostic};
pub use planner::plan_checks;
pub use release::{build_test_conclusions, evaluate_release, ReleaseCheck, ReleaseReport};
pub use redact::{command_display, redact_args, redact_text};
pub use render::{render_html, render_text};
pub use tool::resolve_executable;
