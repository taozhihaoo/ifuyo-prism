//! 环境变量采集与敏感分类（规格 M9a §54–§57/§61/§168/§199–§200/§315）。
//!
//! 红线：**绝不采集值**（§54/§56/§168：name + present + classification +
//! source）。日志与 `--json` 输出天然无 secret（§200）；展示层策略归 M9b。

use crate::model::{VariableFact, VariableSource, VARIABLE_CAP};
use prism_core::task::CancellationToken;

/// 保守敏感模式（§55）：命中即 sensitive（宁可误标，绝不漏标）。
const SENSITIVE_FRAGMENTS: [&str; 12] = [
    "token",
    "password",
    "passwd",
    "secret",
    "key",
    "credential",
    "cred",
    "private",
    "auth",
    "bearer",
    "pass",
    "database",
];

/// §199 分类语义：API_KEY/PASSWORD/TOKEN/SECRET/DATABASE_URL ⇒ sensitive；
/// PORT/NODE_ENV/LANG ⇒ 非 sensitive。
pub fn is_possibly_sensitive(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    SENSITIVE_FRAGMENTS
        .iter()
        .any(|fragment| lower.contains(fragment))
}

/// 采集进程环境变量名（§57 ProcessEnvironment 来源；§168 不做全量值 dump；
/// §315 有界 + §395 按名字稳定排序）。
pub fn collect_variables(cancel: &CancellationToken) -> Vec<VariableFact> {
    let mut names: Vec<String> = std::env::vars_os()
        .filter_map(|(name, _value)| name.into_string().ok())
        .collect();
    names.sort();
    names.dedup();
    let mut facts = Vec::new();
    for name in names {
        if cancel.is_cancelled() || facts.len() >= VARIABLE_CAP {
            break;
        }
        facts.push(VariableFact {
            possibly_sensitive: is_possibly_sensitive(&name),
            name,
            present: true,
            source: VariableSource::ProcessEnvironment,
        });
    }
    facts
}

#[cfg(test)]
mod tests {
    use super::*;

    // §199：分类必须正确。
    #[test]
    fn sensitive_classification_matches_spec_examples() {
        for name in [
            "API_KEY",
            "MY_PASSWORD",
            "GITHUB_TOKEN",
            "CLIENT_SECRET",
            "DATABASE_URL",
            "AWS_SECRET_ACCESS_KEY",
            "PRIVATE_KEY_PATH",
            "AUTH_HEADER",
            "DB_PASS",
        ] {
            assert!(is_possibly_sensitive(name), "{name} must be sensitive");
        }
        for name in ["PORT", "NODE_ENV", "LANG", "CI", "PATH", "HOME"] {
            assert!(!is_possibly_sensitive(name), "{name} must not be sensitive");
        }
    }

    #[test]
    fn facts_never_carry_values() {
        // 采集函数只产生 name/classification；结构上无值字段（§54/§200）。
        let fact = VariableFact {
            name: "SHOULD_NOT_LEAK".to_string(),
            present: true,
            possibly_sensitive: true,
            source: VariableSource::ProcessEnvironment,
        };
        let serialized = serde_json::to_string(&fact).unwrap();
        assert!(!serialized.to_ascii_lowercase().contains("value"));
    }
}
