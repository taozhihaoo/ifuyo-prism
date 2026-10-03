import io

# health_command.rs: 追加完整分析入口
p = 'crates/prism-cli/src/health_command.rs'
s = io.open(p, encoding='utf-8').read()
s = s.replace(
    'use prism_core::config::PrismConfig;\nuse prism_health::HealthRuleRegistry;\nuse std::path::Path;',
    '''use prism_analysis::{CodeAnalysisOptions, CodeAnalysisService};
use prism_changes::{ChangeScope, ChangeSetOptions, ChangeSetService};
use prism_core::config::PrismConfig;
use prism_core::runtime::command::{CommandRunner, StdCommandRunner};
use prism_core::task::CancellationToken;
use prism_graph::DependencyService;
use prism_health::{
    render_html, render_markdown, render_text, HealthAnalysis, HealthOptions, HealthRuleRegistry,
    HealthScope,
};
use std::path::Path;
use std::sync::Arc;''')
s += '''
/// `--fail-on` 词表（§79/§80：默认 findings 不导致失败；显式 opt-in 才非零）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum FailOn {
    Critical,
    Warning,
}

/// `prism health <path>`（无子命令）——完整健康分析（M7b §75-§80）。
#[allow(clippy::too_many_arguments)] // CLI 参数直传
pub fn run_analyze(
    path: &Path,
    format: crate::cli::ChangesFormatArg,
    changed: bool,
    rule: Option<&str>,
    category: Option<&str>,
    fail_on: Option<FailOn>,
    max_files: usize,
) -> i32 {
    if !path.exists() {
        eprintln!("prism: path does not exist: {}", path.to_string_lossy());
        return crate::EXIT_INVALID_INPUT;
    }
    let runner: Arc<dyn CommandRunner> = Arc::new(StdCommandRunner::new());
    let cancel = CancellationToken::new();

    let discovery = match prism_project::DiscoveryService::new().discover(
        path,
        prism_project::ScanOptions::default(),
        &cancel,
        |_| {},
    ) {
        Ok(discovery) => discovery,
        Err(err) => {
            eprintln!("prism: health failed building discovery: {err}");
            return crate::EXIT_ERROR;
        }
    };
    let model = match CodeAnalysisService::new().analyze(
        &discovery,
        CodeAnalysisOptions::default(),
        &cancel,
        |_| {},
    ) {
        Ok(model) => model,
        Err(err) => {
            eprintln!("prism: health failed building code model: {err}");
            return crate::EXIT_ERROR;
        }
    };
    let graph = match DependencyService::new().analyze(&discovery, &model) {
        Ok(graph) => graph,
        Err(err) => {
            eprintln!("prism: health failed building graph: {err}");
            return crate::EXIT_ERROR;
        }
    };

    let (change_set, impact) = if changed {
        let changeset = ChangeSetService::new(runner.clone())
            .build(
                path,
                ChangeScope::WorkingTree,
                None,
                None,
                &ChangeSetOptions { max_files, ..ChangeSetOptions::default() },
                &cancel,
            )
            .ok()
            .flatten();
        let impact = changeset.as_ref().and_then(|changeset| {
            prism_impact::ImpactAnalysisService
                .analyze(
                    changeset,
                    &model,
                    &graph,
                    &prism_impact::ImpactOptions::default(),
                    &cancel,
                )
                .ok()
        });
        (changeset, impact)
    } else {
        (None, None)
    };

    let project_config_path = path.join(PrismConfig::config_file_name());
    let (project_config, _loaded) = match PrismConfig::load(&project_config_path) {
        Ok(loaded) => loaded,
        Err(err) => {
            eprintln!("prism: config invalid: {err}");
            return crate::EXIT_ERROR;
        }
    };

    let scope = if changed { HealthScope::ImpactedItems } else { HealthScope::FullProject };
    let options = HealthOptions { scope, ..HealthOptions::default() };
    let context = prism_health::HealthContext {
        project: &discovery.project,
        code_model: Some(&model),
        discovery: Some(&discovery),
        graph: Some(&graph),
        git: None,
        change_set: change_set.as_ref(),
        impact: impact.as_ref(),
        root: path,
        derived: None,
    };
    match prism_health::analyze(
        &HealthRuleRegistry::with_builtins(),
        &project_config.health,
        &context,
        &options,
        &cancel,
    ) {
        Ok(mut analysis) => {
            if let Some(rule_id) = rule {
                analysis.findings.retain(|finding| finding.rule_id == rule_id);
            }
            if let Some(category) = category {
                analysis.findings.retain(|finding| {
                    format!("{:?}", finding.category).to_ascii_lowercase()
                        == category.to_ascii_lowercase()
                });
            }
            analysis.summary.total_findings = analysis.findings.len() as u64;
            match format {
                crate::cli::ChangesFormatArg::Json => {
                    match serde_json::to_string_pretty(&analysis) {
                        Ok(text) => println!("{text}"),
                        Err(err) => {
                            eprintln!("prism: failed to serialize health analysis: {err}");
                            return crate::EXIT_ERROR;
                        }
                    }
                }
                crate::cli::ChangesFormatArg::Text => print!("{}", render_text(&analysis)),
                crate::cli::ChangesFormatArg::Markdown => {
                    print!("{}", render_markdown(&analysis))
                }
            }
            match fail_on {
                None => crate::EXIT_OK,
                Some(FailOn::Critical) if analysis.summary.critical_count > 0 => {
                    eprintln!(
                        "prism: --fail-on critical: {} critical finding(s)",
                        analysis.summary.critical_count
                    );
                    crate::EXIT_ERROR
                }
                Some(FailOn::Warning)
                    if analysis.summary.critical_count > 0
                        || analysis.summary.warning_count > 0 =>
                {
                    eprintln!(
                        "prism: --fail-on warning: {} critical / {} warning finding(s)",
                        analysis.summary.critical_count, analysis.summary.warning_count
                    );
                    crate::EXIT_ERROR
                }
                Some(_) => crate::EXIT_OK,
            }
        }
        Err(prism_core::PrismError::Cancelled) => {
            eprintln!("prism: health cancelled");
            crate::EXIT_CANCELLED
        }
        Err(err) => {
            eprintln!("prism: health failed: {err}");
            crate::EXIT_ERROR
        }
    }
}
'''
io.open(p, 'w', encoding='utf-8', newline='\n').write(s)

# cli.rs
p = 'crates/prism-cli/src/cli.rs'
s = io.open(p, encoding='utf-8').read()
old = '''    /// 项目健康（M7a：规则检视；完整分析在 M7b）
    Health {
        #[arg(default_value = ".")]
        path: std::path::PathBuf,
        #[command(subcommand)]
        command: HealthCommand,
    },'''
new = '''    /// 项目健康体检（M7：完整分析；`rules` 子命令检视规则）
    Health {
        #[arg(default_value = ".")]
        path: std::path::PathBuf,
        /// 输出格式
        #[arg(long, value_enum, default_value_t = ChangesFormatArg::Text)]
        format: ChangesFormatArg,
        /// 只报告当前变更（及受影响范围）内的 findings（§61）
        #[arg(long)]
        changed: bool,
        /// 只输出指定规则 id 的 findings
        #[arg(long)]
        rule: Option<String>,
        /// 只输出指定分类的 findings（如 security）
        #[arg(long)]
        category: Option<String>,
        /// 存在 >= 指定严重级 finding 时以非零退出（默认 findings 不导致失败，§79）
        #[arg(long, value_enum)]
        fail_on: Option<crate::health_command::FailOn>,
        /// 变更文件数上限（--changed 时透传 ChangeSet）
        #[arg(long, default_value_t = 200)]
        max_files: usize,
        #[command(subcommand)]
        command: Option<HealthCommand>,
    },'''
assert old in s
s = s.replace(old, new)
io.open(p, 'w', encoding='utf-8', newline='\n').write(s)

# main.rs
p = 'crates/prism-cli/src/main.rs'
s = io.open(p, encoding='utf-8').read()
old = '''        Some(cli::Command::Health { path, command }) => match command {
            cli::HealthCommand::Rules { json } => {
                health_command::run_rules(&path, *json)
            }
        },'''
new = '''        Some(cli::Command::Health {
            path,
            format,
            changed,
            rule,
            category,
            fail_on,
            max_files,
            command,
        }) => match command {
            Some(cli::HealthCommand::Rules { json }) => {
                health_command::run_rules(&path, *json)
            }
            None => health_command::run_analyze(
                &path,
                format,
                *changed,
                rule.as_deref(),
                category.as_deref(),
                *fail_on,
                *max_files,
            ),
        },'''
assert old in s
s = s.replace(old, new)
io.open(p, 'w', encoding='utf-8', newline='\n').write(s)

# Cargo: prism-impact dep for --changed impact
p = 'crates/prism-cli/Cargo.toml'
s = io.open(p, encoding='utf-8').read()
if 'prism-impact' not in s:
    s = s.replace('prism-health.workspace = true', 'prism-health.workspace = true\nprism-impact.workspace = true')
io.open(p, 'w', encoding='utf-8', newline='\n').write(s)
print('wired')
