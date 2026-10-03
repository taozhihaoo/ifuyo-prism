//! 文本渲染（规格 M9a §296/§297；机器可读 JSON 由 CLI 直接序列化 snapshot）。
//!
//! 只呈现事实与匹配结果，不做 Ready/分数式总结（§209 的展示面延伸）。

use crate::model::{EnvironmentSnapshot, MatchResult, ProbeOutcome};

/// 状态标签（§296 示例形态：OK / MISMATCH / NOT FOUND / UNKNOWN）。
fn status_label(fact: &crate::model::ToolFact) -> &'static str {
    match fact.probe {
        ProbeOutcome::NotFound => "NOT FOUND",
        ProbeOutcome::Succeeded => "OK",
        ProbeOutcome::Failed => "PROBE FAILED",
        ProbeOutcome::NotProbed => "NOT PROBED",
    }
}

fn match_label(result: MatchResult) -> &'static str {
    match result {
        MatchResult::Satisfied => "OK",
        MatchResult::Unsatisfied => "MISMATCH",
        MatchResult::Unknown => "UNKNOWN",
        MatchResult::NotApplicable => "N/A",
    }
}

/// 终端安全（§343）：环境变量名/工具输出可能含 ANSI 控制序列；
/// 渲染前剥除 C0 控制字符（保留 tab），防止恶意终端注入。
pub fn sanitize(text: &str) -> String {
    text.chars()
        .filter(|c| !c.is_control() || *c == '\u{9}')
        .collect()
}

/// 人类可读文本（§296）。
pub fn render_text(snapshot: &EnvironmentSnapshot) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "Environment ({})\n",
        snapshot.project_root
    ));
    out.push_str(&format!(
        "  OS         {} {} ({})\n",
        snapshot.host.os_name,
        snapshot.host.os_version.as_deref().unwrap_or("version unknown"),
        snapshot.host.architecture.as_str()
    ));
    out.push_str(&format!(
        "  Shell      {}{}\n",
        snapshot.shell.kind.as_str(),
        snapshot
            .shell
            .executable
            .as_deref()
            .map(|path| format!(" ({path})"))
            .unwrap_or_default()
    ));
    if let Some(hostname) = &snapshot.host.hostname {
        out.push_str(&format!("  Hostname   {hostname}\n"));
    }

    out.push_str("\nTools\n");
    for fact in &snapshot.tools {
        let version = sanitize(fact.raw_version.as_deref().unwrap_or("version unknown"));
        let path = sanitize(fact.path.as_deref().unwrap_or("-"));
        out.push_str(&format!(
            "  {:<10} {:<32} {:<12} {}\n",
            fact.name, version, status_label(fact), path
        ));
        for additional in &fact.additional_paths {
            out.push_str(&format!("             also found: {additional}\n"));
        }
    }
    if snapshot.tools.is_empty() {
        out.push_str("  (no tools probed)\n");
    }

    if !snapshot.runtimes.is_empty() {
        out.push_str("\nRuntimes\n");
        for runtime in &snapshot.runtimes {
            out.push_str(&format!(
                "  {:<10} {}\n",
                runtime.name,
                runtime
                    .raw_version
                    .as_deref()
                    .unwrap_or("version unknown")
            ));
        }
    }
    if !snapshot.sdks.is_empty() {
        out.push_str("\nSDKs\n");
        for sdk in &snapshot.sdks {
            out.push_str(&format!(
                "  {:<16} {}\n",
                sdk.name,
                sdk.raw_version.as_deref().unwrap_or("version unknown")
            ));
        }
    }
    if !snapshot.virtualenvs.is_empty() {
        out.push_str("\nVirtual environments\n");
        for venv in &snapshot.virtualenvs {
            out.push_str(&format!(
                "  {} {} {}{}\n",
                venv.kind,
                venv.path,
                venv.raw_version.as_deref().unwrap_or("version unknown"),
                if venv.active { " [active]" } else { "" }
            ));
        }
    }
    if !snapshot.package_managers.is_empty() {
        out.push_str("\nPackage managers\n");
        for manager in &snapshot.package_managers {
            let association = manager
                .project_association
                .as_ref()
                .map(|association| {
                    format!(
                        " project: {} (evidence: {})",
                        association.manager, association.evidence
                    )
                })
                .unwrap_or_default();
            // §157：installed-but-version-unknown ≠ not installed。
            let installed = match (&manager.raw_version, &manager.path) {
                (Some(raw), _) => raw.clone(),
                (None, Some(path)) => format!("version unknown ({path})"),
                (None, None) => "not installed".to_string(),
            };
            out.push_str(&format!(
                "  {:<10} {}{}{}\n",
                manager.name,
                installed,
                association,
                manager
                    .project_association
                    .as_ref()
                    .and_then(|a| a.required_version.as_deref())
                    .map(|version| format!(" required {version}"))
                    .unwrap_or_default()
            ));
        }
    }

    out.push_str(&format!(
        "\nPATH      {} entries ({} duplicates, {} missing{})\n",
        snapshot.paths.entries.len(),
        snapshot.paths.duplicates.len(),
        snapshot.paths.missing.len(),
        if snapshot.paths.truncated {
            ", truncated"
        } else {
            ""
        }
    ));
    out.push_str(&format!(
        "Variables {} captured ({} possibly sensitive names; values never read)\n",
        snapshot.variables.len(),
        snapshot
            .variables
            .iter()
            .filter(|fact| fact.possibly_sensitive)
            .count()
    ));
    out.push_str(&format!(
        "Resources {} cpus · memory {} · disk free {}\n",
        snapshot
            .resources
            .logical_cpus
            .map(|value| value.to_string())
            .unwrap_or_else(|| "unknown".to_string()),
        format_bytes(snapshot.resources.memory_total_bytes),
        format_bytes(snapshot.resources.disk_free_bytes),
    ));

    if !snapshot.matches.is_empty() {
        out.push_str("\nRequirements\n");
        for (requirement, matched) in snapshot
            .requirements
            .iter()
            .zip(snapshot.matches.iter())
        {
            let expected = if requirement.expected.is_empty() {
                requirement.operator.as_str().to_string()
            } else {
                format!("{} {}", requirement.operator.as_str(), requirement.expected)
            };
            out.push_str(&format!(
                "  {:<10} {:<22} {:<10} [{}]\n    {}\n",
                requirement.kind.as_str(),
                format!("{} {expected}", requirement.subject),
                match_label(matched.result),
                requirement.source,
                matched.evidence
            ));
        }
    }

    if !snapshot.diagnostics.is_empty() {
        out.push_str("\nDiagnostics\n");
        for diagnostic in &snapshot.diagnostics {
            out.push_str(&format!(
                "  [{}] {}\n",
                diagnostic.kind, diagnostic.detail
            ));
        }
    }

    out.push_str(&format!(
        "\nSnapshot  {}  captured {}  scan={:?} completeness={:?}\n",
        snapshot.id, snapshot.captured_at, snapshot.scan_outcome, snapshot.completeness
    ));
    out.push_str(
        "This report describes the current local environment (§290); it does not \
         guarantee reproducibility (§291).\n",
    );
    out
}

fn format_bytes(bytes: Option<u64>) -> String {
    match bytes {
        Some(value) if value >= 1024 * 1024 * 1024 => {
            format!("{:.1} GiB", value as f64 / (1024.0 * 1024.0 * 1024.0))
        }
        Some(value) if value >= 1024 * 1024 => {
            format!("{:.1} MiB", value as f64 / (1024.0 * 1024.0))
        }
        Some(value) => format!("{value} B"),
        None => "unknown".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_strips_control_characters() {
        // §343：ANSI/控制序列不得进入终端输出。
        let esc = char::from_u32(0x1b).unwrap();
        let bel = char::from_u32(0x7).unwrap();
        let malicious = format!("{esc}[31mERROR{esc}[0m{bel}");
        assert_eq!(sanitize(&malicious), "[31mERROR[0m");
        assert_eq!(sanitize("normal"), "normal");
        assert!(sanitize("keep\ttab").contains('\t'));
    }
}
