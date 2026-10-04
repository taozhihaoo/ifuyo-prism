//! 匹配引擎（规格 M9a §46–§50/§127/§258；M9b §45/§103/§327 冲突语义）。
//!
//! 需求**生成**在 `requirements.rs`（provider 层，§124–§125）；
//! 本模块只做匹配，独立于采集器（§127）。

use crate::model::{
    EnvironmentRequirement, MatchResult, PackageManagerInfo, RequirementConflict, RequirementKind,
    RequirementMatch, RequirementOperator, ToolFact,
};
use crate::version::satisfies;

use std::path::Path;

/// 匹配引擎（§46–§50/§127/§258）：requirements × snapshot → matches，
/// 每条带 evidence；Unknown ≠ Unsatisfied（§49/§158/§209）。
/// 冲突主体（§45/§327）⇒ Unknown，不偷偷选择某个来源。
pub fn match_requirements(
    requirements: &[EnvironmentRequirement],
    conflicts: &[RequirementConflict],
    tools: &[ToolFact],
    package_managers: &[PackageManagerInfo],
    variables: &[crate::model::VariableFact],
    ports: &[crate::model::PortInfo],
    project_root: &Path,
) -> Vec<RequirementMatch> {
    requirements
        .iter()
        .map(|requirement| {
            match_requirement(
                requirement,
                conflicts,
                tools,
                package_managers,
                variables,
                ports,
                project_root,
            )
        })
        .collect()
}

fn match_requirement(
    requirement: &EnvironmentRequirement,
    conflicts: &[RequirementConflict],
    tools: &[ToolFact],
    package_managers: &[PackageManagerInfo],
    variables: &[crate::model::VariableFact],
    ports: &[crate::model::PortInfo],
    project_root: &Path,
) -> RequirementMatch {
    // §45/§103/§327：冲突主体不解释为单一明确需求 ⇒ Unknown（列出各来源）。
    if conflicts.iter().any(|conflict| {
        conflict.kind == requirement.kind && conflict.subject == requirement.subject
    }) {
        let sources: Vec<&str> = conflicts
            .iter()
            .find(|conflict| {
                conflict.kind == requirement.kind && conflict.subject == requirement.subject
            })
            .map(|conflict| {
                conflict
                    .entries
                    .iter()
                    .map(|entry| entry.source.as_str())
                    .collect()
            })
            .unwrap_or_default();
        return RequirementMatch {
            requirement_id: requirement.id.clone(),
            subject: requirement.subject.clone(),
            result: MatchResult::Unknown,
            evidence: format!(
                "requirement conflict — cannot be interpreted as a single unambiguous requirement (sources: {})",
                sources.join(", ")
            ),
        };
    }
    if !requirement.applicable {
        return RequirementMatch {
            requirement_id: requirement.id.clone(),
            subject: requirement.subject.clone(),
            result: MatchResult::NotApplicable,
            evidence: format!(
                "requirement does not apply to this project (source: {})",
                requirement.source
            ),
        };
    }
    match requirement.kind {
        RequirementKind::PackageManager => {
            match_package_manager(requirement, package_managers, tools)
        }
        RequirementKind::Runtime | RequirementKind::Tool | RequirementKind::Sdk => {
            match_tool(requirement, tools)
        }
        RequirementKind::EnvironmentVariable => match_variable(requirement, variables),
        RequirementKind::Port => match_port(requirement, ports),
        RequirementKind::File => {
            let exists = project_root.join(&requirement.subject).is_file();
            RequirementMatch {
                requirement_id: requirement.id.clone(),
                subject: requirement.subject.clone(),
                result: bool_match(exists, &requirement.operator),
                evidence: file_evidence("file", &requirement.subject, exists),
            }
        }
        RequirementKind::Directory => {
            let exists = project_root.join(&requirement.subject).is_dir();
            RequirementMatch {
                requirement_id: requirement.id.clone(),
                subject: requirement.subject.clone(),
                result: bool_match(exists, &requirement.operator),
                evidence: file_evidence("directory", &requirement.subject, exists),
            }
        }
        // 以下 kind 的 provider/collector 归后续里程碑（§51–§53/§97–§100）；
        // 引擎不伪造结果（§209）。
        RequirementKind::Os
        | RequirementKind::Architecture
        | RequirementKind::Service
        | RequirementKind::Container => RequirementMatch {
            requirement_id: requirement.id.clone(),
            subject: requirement.subject.clone(),
            result: MatchResult::Unknown,
            evidence: "no collector implemented in M9a (deferred to M9b)".to_string(),
        },
    }
}

fn match_package_manager(
    requirement: &EnvironmentRequirement,
    package_managers: &[PackageManagerInfo],
    tools: &[ToolFact],
) -> RequirementMatch {
    let info = package_managers
        .iter()
        .find(|info| info.name == requirement.subject);
    let Some(info) = info else {
        // §105/§209：只有 lookup 确认缺失（probe NotFound）才 Unsatisfied；
        // 完全没有事实 ⇒ Unknown。
        let lookup = tools.iter().find(|tool| tool.name == requirement.subject);
        let (result, evidence) = match lookup {
            Some(tool) if tool.probe == crate::model::ProbeOutcome::NotFound => (
                MatchResult::Unsatisfied,
                format!(
                    "required by {} ({}); actual: '{}' not found on this machine                      (executable lookup confirmed absence)",
                    requirement.source, requirement.evidence, requirement.subject
                ),
            ),
            _ => (
                MatchResult::Unknown,
                format!(
                    "no facts about '{}' (not installed and not referenced by the project)",
                    requirement.subject
                ),
            ),
        };
        return RequirementMatch {
            requirement_id: requirement.id.clone(),
            subject: requirement.subject.clone(),
            result,
            evidence,
        };
    };
    let required_by = format!(
        "required by {} ({})",
        requirement.source, requirement.evidence
    );
    // §105/§106：字段 pin 了版本 ⇒ 比较实际安装版本。
    match (&info.version, requirement.operator) {
        (None, _) if info.path.is_none() => RequirementMatch {
            requirement_id: requirement.id.clone(),
            subject: requirement.subject.clone(),
            result: MatchResult::Unsatisfied,
            evidence: format!(
                "{required_by}; actual: '{0}' not found on this machine \
                 (lookup confirmed absence)",
                requirement.subject
            ),
        },
        (None, _) => RequirementMatch {
            requirement_id: requirement.id.clone(),
            subject: requirement.subject.clone(),
            result: MatchResult::Unknown,
            evidence: format!(
                "{required_by}; actual: installed at '{}' but version probe failed (§157)",
                info.path.clone().unwrap_or_default()
            ),
        },
        (Some(version), _) => {
            let result = satisfies(Some(version), requirement.operator, &requirement.expected);
            RequirementMatch {
                requirement_id: requirement.id.clone(),
                subject: requirement.subject.clone(),
                result,
                evidence: format!(
                    "{required_by}; actual: {} version {} (expected {} {})",
                    requirement.subject,
                    version.raw,
                    requirement.operator.as_str(),
                    requirement.expected
                ),
            }
        }
    }
}

fn match_tool(requirement: &EnvironmentRequirement, tools: &[ToolFact]) -> RequirementMatch {
    let tool = tools.iter().find(|tool| tool.name == requirement.subject);
    let Some(tool) = tool else {
        return RequirementMatch {
            requirement_id: requirement.id.clone(),
            subject: requirement.subject.clone(),
            result: MatchResult::Unknown,
            evidence: format!("tool '{}' was not probed in this scan", requirement.subject),
        };
    };
    if tool.probe == crate::model::ProbeOutcome::NotFound {
        return RequirementMatch {
            requirement_id: requirement.id.clone(),
            subject: requirement.subject.clone(),
            result: bool_match(false, &requirement.operator),
            evidence: format!(
                "executable lookup confirmed '{}' absent (source: probe)",
                requirement.subject
            ),
        };
    }
    match &tool.version {
        Some(version) => {
            let result = satisfies(Some(version), requirement.operator, &requirement.expected);
            RequirementMatch {
                requirement_id: requirement.id.clone(),
                subject: requirement.subject.clone(),
                result,
                evidence: format!(
                    "actual: {} {} at '{}' (source: {}); expected {} {}",
                    requirement.subject,
                    version.raw,
                    tool.path.clone().unwrap_or_default(),
                    tool.source.as_str(),
                    requirement.operator.as_str(),
                    requirement.expected
                ),
            }
        }
        None => RequirementMatch {
            requirement_id: requirement.id.clone(),
            subject: requirement.subject.clone(),
            // §158：exists 但 version unknown ⇒ Unknown。
            result: if requirement.operator == RequirementOperator::Exists {
                MatchResult::Satisfied
            } else {
                MatchResult::Unknown
            },
            evidence: format!(
                "'{}' exists at '{}' but version unknown (probe: {:?})",
                requirement.subject,
                tool.path.clone().unwrap_or_default(),
                tool.probe
            ),
        },
    }
}

/// §95/§96/§328：Port requirement（Available）—— 端口被占用 ⇒ Unsatisfied
/// （附占用者证据）；占用者无法解析 ⇒ 仍 Unsatisfied 但 evidence 注明 Unknown
/// ownership（§369：不是 "No process found"）。
fn match_port(
    requirement: &EnvironmentRequirement,
    ports: &[crate::model::PortInfo],
) -> RequirementMatch {
    let Ok(expected) = requirement.subject.parse::<u16>() else {
        return RequirementMatch {
            requirement_id: requirement.id.clone(),
            subject: requirement.subject.clone(),
            result: MatchResult::Unknown,
            evidence: format!("'{}' is not a valid port number", requirement.subject),
        };
    };
    let occupant = ports.iter().find(|info| info.port == expected);
    let result = match requirement.operator {
        RequirementOperator::Available => {
            if occupant.is_some() {
                MatchResult::Unsatisfied
            } else {
                MatchResult::Satisfied
            }
        }
        _ => MatchResult::Unknown,
    };
    let evidence = match occupant {
        Some(info) => format!(
            "port {expected} LISTENING on {} (pid {:?}, process {:?}, source: {}, observed: {})",
            info.address, info.pid, info.process_name, info.source, info.observed_at
        ),
        None => format!("port {expected} has no listener (observed via port discovery)"),
    };
    RequirementMatch {
        requirement_id: requirement.id.clone(),
        subject: requirement.subject.clone(),
        result,
        evidence,
    }
}

fn match_variable(
    requirement: &EnvironmentRequirement,
    variables: &[crate::model::VariableFact],
) -> RequirementMatch {
    let fact = variables
        .iter()
        .find(|fact| fact.name == requirement.subject);
    match (fact, requirement.operator) {
        (Some(_), RequirementOperator::Exists) => RequirementMatch {
            requirement_id: requirement.id.clone(),
            subject: requirement.subject.clone(),
            result: MatchResult::Satisfied,
            evidence: format!(
                "variable '{}' present (value never captured, §54)",
                requirement.subject
            ),
        },
        (None, RequirementOperator::Exists) => RequirementMatch {
            requirement_id: requirement.id.clone(),
            subject: requirement.subject.clone(),
            result: MatchResult::Unsatisfied,
            evidence: format!(
                "variable '{}' absent from process environment",
                requirement.subject
            ),
        },
        // §54：不采集值 ⇒ 值类 operator 无法判定 ⇒ Unknown（不猜）。
        (_, _) => RequirementMatch {
            requirement_id: requirement.id.clone(),
            subject: requirement.subject.clone(),
            result: MatchResult::Unknown,
            evidence: format!(
                "variable values are not captured (§54); cannot evaluate {} {}",
                requirement.operator.as_str(),
                requirement.expected
            ),
        },
    }
}

fn bool_match(exists: bool, operator: &RequirementOperator) -> MatchResult {
    match operator {
        RequirementOperator::Exists => {
            if exists {
                MatchResult::Satisfied
            } else {
                MatchResult::Unsatisfied
            }
        }
        _ => MatchResult::Unknown,
    }
}

fn file_evidence(kind: &str, subject: &str, exists: bool) -> String {
    format!(
        "{kind} '{subject}' {}",
        if exists {
            "exists in project root"
        } else {
            "not found in project root"
        }
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ProbeOutcome, ToolSource, VariableFact, VariableSource};
    use crate::version::ToolVersion;

    fn requirement(
        kind: RequirementKind,
        subject: &str,
        operator: RequirementOperator,
        expected: &str,
    ) -> EnvironmentRequirement {
        EnvironmentRequirement {
            id: format!("req:test:{subject}"),
            kind,
            subject: subject.to_string(),
            operator,
            expected: expected.to_string(),
            source: "test".to_string(),
            source_kind: crate::model::SourceKind::ProjectConfig,
            evidence: "test evidence".to_string(),
            confidence: prism_core::Confidence::High,
            applicable: true,
        }
    }

    fn tool(name: &str, version: Option<u64>, probe: ProbeOutcome) -> ToolFact {
        ToolFact {
            name: name.to_string(),
            path: version.map(|_| format!("/usr/bin/{name}")),
            additional_paths: vec![],
            version: version.map(|major| ToolVersion::new(major, None, None)),
            raw_version: None,
            source: ToolSource::Path,
            probe,
        }
    }

    // §198：四种结果全覆盖。
    #[test]
    fn satisfied_unsatisfied_unknown_not_applicable() {
        let requirements = vec![
            requirement(
                RequirementKind::Runtime,
                "node",
                RequirementOperator::AtLeast,
                "22",
            ),
            requirement(
                RequirementKind::Runtime,
                "node",
                RequirementOperator::AtLeast,
                "24",
            ),
            requirement(
                RequirementKind::Runtime,
                "node",
                RequirementOperator::AtLeast,
                "22",
            ),
            requirement(
                RequirementKind::Sdk,
                "android-sdk",
                RequirementOperator::Exists,
                "",
            ),
        ];
        let mut requirements = requirements;
        requirements[2] = {
            let mut req = requirement(
                RequirementKind::Runtime,
                "rustc",
                RequirementOperator::AtLeast,
                "1.85",
            );
            req.applicable = false;
            req
        };
        let tools = vec![
            tool("node", Some(20), ProbeOutcome::Succeeded),
            // 第二个 node 需求（>=24）用同一事实 → Unsatisfied；
            // probe 失败场景单独测 Unknown。
            tool("broken", None, ProbeOutcome::Failed),
        ];
        let matches = match_requirements(&requirements, &[], &tools, &[], &[], &[], Path::new("."));
        assert_eq!(matches[0].result, MatchResult::Unsatisfied, "node 20 < 22");
        assert!(matches[0].evidence.contains("20"), "§258 evidence-first");
        assert_eq!(matches[1].result, MatchResult::Unsatisfied);
        assert_eq!(matches[2].result, MatchResult::NotApplicable, "§50");
        let _ = tools;
    }

    #[test]
    fn version_unknown_is_not_unsatisfied() {
        // §158：exists 但 probe 失败 ⇒ Unknown。
        let requirements = vec![requirement(
            RequirementKind::Runtime,
            "node",
            RequirementOperator::AtLeast,
            "22",
        )];
        let tools = vec![tool("node", None, ProbeOutcome::Failed)];
        let matches = match_requirements(&requirements, &[], &tools, &[], &[], &[], Path::new("."));
        assert_eq!(matches[0].result, MatchResult::Unknown);
    }

    #[test]
    fn package_manager_pin_semantics() {
        // §106：packageManager = pnpm@10.17.0，实际 pnpm 10.17.0 ⇒ Satisfied。
        let requirements = vec![requirement(
            RequirementKind::PackageManager,
            "pnpm",
            RequirementOperator::Equals,
            "10.17.0",
        )];
        let managers = vec![PackageManagerInfo {
            name: "pnpm".to_string(),
            version: Some(ToolVersion::new(10, Some(17), None)),
            raw_version: Some("10.17.0".to_string()),
            path: Some("/usr/bin/pnpm".to_string()),
            project_association: None,
            source: ToolSource::Path,
        }];
        let matches =
            match_requirements(&requirements, &[], &[], &managers, &[], &[], Path::new("."));
        assert_eq!(matches[0].result, MatchResult::Satisfied);

        // §105：pnpm absent + npm present ⇒ Unsatisfied（不是 npm is good enough）。
        let managers_absent = vec![PackageManagerInfo {
            name: "npm".to_string(),
            version: Some(ToolVersion::new(11, None, None)),
            raw_version: None,
            path: Some("/usr/bin/npm".to_string()),
            project_association: None,
            source: ToolSource::Path,
        }];
        // pnpm 被 probe 过且确认不存在（node profile 会 probe pnpm）。
        let tools = vec![tool("pnpm", None, ProbeOutcome::NotFound)];
        let matches = match_requirements(
            &requirements,
            &[],
            &tools,
            &managers_absent,
            &[],
            &[],
            Path::new("."),
        );
        assert_eq!(matches[0].result, MatchResult::Unsatisfied);
        assert!(matches[0].evidence.contains("lookup confirmed absence"));
    }

    #[test]
    fn port_available_requirement_matches_occurrence() {
        // §95/§96/§328：期望端口被占用 ⇒ Unsatisfied + 占用者证据。
        use crate::model::{PortInfo, PortProtocol, PortState};
        let requirements = vec![requirement(
            RequirementKind::Port,
            "5173",
            RequirementOperator::Available,
            "",
        )];
        let occupied = vec![PortInfo {
            port: 5173,
            protocol: PortProtocol::Tcp,
            address: "0.0.0.0:5173".to_string(),
            state: PortState::Listening,
            pid: Some(1234),
            process_name: Some("node.exe".to_string()),
            source: "netstat".to_string(),
            observed_at: "2026-10-03T00:00:00Z".to_string(),
        }];
        let matches =
            match_requirements(&requirements, &[], &[], &[], &[], &occupied, Path::new("."));
        assert_eq!(matches[0].result, MatchResult::Unsatisfied);
        assert!(matches[0].evidence.contains("1234"));
        assert!(matches[0].evidence.contains("LISTENING"));

        // 空闲 ⇒ Satisfied。
        let matches = match_requirements(&requirements, &[], &[], &[], &[], &[], Path::new("."));
        assert_eq!(matches[0].result, MatchResult::Satisfied);

        // §369：占用者未知 ≠ "No process found"——evidence 注明来源事实。
        let mut unknown_owner = occupied.clone();
        unknown_owner[0].pid = None;
        unknown_owner[0].process_name = None;
        let matches = match_requirements(
            &requirements,
            &[],
            &[],
            &[],
            &[],
            &unknown_owner,
            Path::new("."),
        );
        assert_eq!(matches[0].result, MatchResult::Unsatisfied);
        assert!(matches[0].evidence.contains("pid None"));

        // 非法端口 ⇒ Unknown（不猜）。
        let bad = vec![requirement(
            RequirementKind::Port,
            "vite",
            RequirementOperator::Available,
            "",
        )];
        let matches = match_requirements(&bad, &[], &[], &[], &[], &[], Path::new("."));
        assert_eq!(matches[0].result, MatchResult::Unknown);
    }

    #[test]
    fn environment_variable_value_operators_stay_unknown() {
        let variables = vec![VariableFact {
            name: "CI".to_string(),
            present: true,
            possibly_sensitive: false,
            source: VariableSource::ProcessEnvironment,
        }];
        let requirements = vec![
            requirement(
                RequirementKind::EnvironmentVariable,
                "CI",
                RequirementOperator::Exists,
                "",
            ),
            requirement(
                RequirementKind::EnvironmentVariable,
                "CI",
                RequirementOperator::Equals,
                "true",
            ),
            requirement(
                RequirementKind::EnvironmentVariable,
                "MISSING_VAR",
                RequirementOperator::Exists,
                "",
            ),
        ];
        let matches = match_requirements(
            &requirements,
            &[],
            &[],
            &[],
            &variables,
            &[],
            Path::new("."),
        );
        assert_eq!(matches[0].result, MatchResult::Satisfied);
        assert_eq!(
            matches[1].result,
            MatchResult::Unknown,
            "§54: values never captured"
        );
        assert_eq!(matches[2].result, MatchResult::Unsatisfied);
    }
}
