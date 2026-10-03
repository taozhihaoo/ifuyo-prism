//! 安全规则（规格 M7 §44–§50/§107–§110/§155/§226/§227）。
//!
//! 极度保守（§45）：凭证形态赋值（键名 + 分隔符 + 长值 + 非占位符）
//! 与 PEM 私钥标记；**绝不输出值本身**（§46：evidence 只存位置、
//! 模式种类与 [REDACTED] 预览）；example/test/dummy/sample/placeholder
//! 等占位值直接跳过（§48/§110）；Generated 文件不在扫描面（M2 管线
//! 策略 + derived 文件集不含 Skipped）；无 auto-fix（§227）、无网络（§229）。

use crate::model::{
    Evidence, EvidenceKind, Finding, FindingCategory, FindingExplanation, FindingLocation,
    FindingStatus, RuleResult,
};
use crate::registry::{HealthRule, HealthRuleMeta};
use prism_core::severity::{Confidence, Severity};
use prism_core::task::CancellationToken;
use prism_core::PrismError;
use std::sync::LazyLock;

const KEY_NAMES: [&str; 10] = [
    "password", "passwd", "secret", "token", "api_key", "apikey", "access_key",
    "client_secret", "auth_token", "private_token",
];
const PLACEHOLDERS: [&str; 9] = [
    "example", "test", "dummy", "sample", "placeholder", "changeme", "change-me",
    "your_", "xxxx",
];
const MIN_VALUE_LEN: usize = 12;

/// 凭证形态行识别：键名 + (=|:) + 长值（非 URL、非占位符）。
fn credential_line(line: &str) -> Option<(String, usize)> {
    let lower = line.to_ascii_lowercase();
    // 先找键名出现位置（自左向右），再找其后的赋值分隔符。
    for key in KEY_NAMES {
        let mut cursor = 0;
        while let Some(found) = lower[cursor..].find(key) {
            let key_start = cursor + found;
            let key_end = key_start + key.len();
            // 整词（前不是字母/下划线，避免匹配 my_password_token 之类复合词太宽）。
            let before_ok = key_start == 0
                || !(lower.as_bytes()[key_start - 1].is_ascii_alphanumeric()
                    || lower.as_bytes()[key_start - 1] == b'_');
            cursor = key_end;
            if !before_ok || key_end >= lower.len() {
                continue;
            }
            // 键名后的第一个非空白必须是 = 或 :（容忍两端空格与引号）。
            let rest = lower[key_end..].trim_start();
            let sep_relative = rest.chars().next()?;
            if sep_relative != '=' && sep_relative != ':' {
                continue;
            }
            let value_part = line[key_end + line[key_end..].find(sep_relative).unwrap() + 1..]
                .trim()
                .trim_matches(|c| c == '"' || c == '\'' || c == '`');
            let lower_value = value_part.to_ascii_lowercase();
            let is_placeholder = PLACEHOLDERS.iter().any(|p| lower_value.contains(p));
            let looks_url = value_part.starts_with("http://") || value_part.starts_with("https://");
            if !is_placeholder && !looks_url && value_part.len() >= MIN_VALUE_LEN {
                let column = line.find(key).map(|i| i + 1).unwrap_or(1);
                return Some((key.to_string(), column));
            }
        }
    }
    None
}

fn pem_line(line: &str) -> bool {
    line.contains("-----BEGIN") && line.contains("PRIVATE KEY") && line.contains("-----")
}

#[allow(clippy::too_many_arguments)] // finding 字段直传；参数即 finding 形状
fn secret_finding(
    rule_id: &'static str,
    title: &str,
    what: &str,
    path: &str,
    line_number: usize,
    column: usize,
    pattern_kind: &str,
    severity: Severity,
    confidence: Confidence,
) -> Finding {
    Finding {
        id: Finding::derive_id(rule_id, &format!("{path}:{line_number}:{pattern_kind}")),
        rule_id: rule_id.to_string(),
        category: FindingCategory::Security,
        severity,
        confidence,
        title: title.to_string(),
        message: format!("Possible credential-like value detected at {path}:{line_number} (value redacted)."),
        explanation: FindingExplanation {
            what: what.to_string(),
            evidence_summary: format!("pattern = {pattern_kind}; value = [REDACTED]"),
            why_it_matters: Some(
                "Credential-like values in source or config files can leak through reports and history; verify manually.".to_string(),
            ),
        },
        location: FindingLocation::new(path.to_string(), line_number as u32, column as u32),
        related_locations: vec![],
        related_symbols: vec![],
        related_modules: vec![],
        related_files: vec![path.to_string()],
        evidence: vec![Evidence::new(
            EvidenceKind::SourceLocation,
            format!("{path}:{line_number}:{column}"),
            format!("pattern = {pattern_kind}; value = [REDACTED]"),
        )
        .with_location(FindingLocation::new(
            path.to_string(),
            line_number as u32,
            column as u32,
        ))],
        suggested_action: Some("Verify the value and move real credentials out of tracked files.".to_string()),
        status: FindingStatus::Active,
        metadata: [("pattern".to_string(), pattern_kind.to_string())]
            .into_iter()
            .collect(),
    }
}

fn scan_contents(
    context: &crate::engine::HealthContext,
    cancel: &CancellationToken,
    mut on_line: impl FnMut(&str, usize, &str),
) -> prism_core::PrismResult<()> {
    let Some(derived) = context.derived else {
        return Ok(());
    };
    for (path, content) in &derived.file_contents {
        for (index, line) in content.lines().enumerate() {
            if cancel.is_cancelled() {
                return Err(PrismError::Cancelled);
            }
            on_line(path, index + 1, line);
        }
    }
    Ok(())
}

static SEC001: LazyLock<HealthRuleMeta> = LazyLock::new(|| HealthRuleMeta {
    id: "SEC001",
    name: "Possible Hard-coded Secret",
    category: FindingCategory::Security,
    description: "Credential-like assignment (key name + separator + long non-placeholder value); value is always redacted.",
    default_severity: Severity::Warning,
    default_enabled: true,
    threshold_meaning: None,
    needs_derived: true,
});

static SEC002: LazyLock<HealthRuleMeta> = LazyLock::new(|| HealthRuleMeta {
    id: "SEC002",
    name: "Private Key Material Detected",
    category: FindingCategory::Security,
    description: "PEM private-key marker detected; value is always redacted.",
    default_severity: Severity::Critical,
    default_enabled: true,
    threshold_meaning: None,
    needs_derived: true,
});

pub struct HardcodedSecretRule;

impl HealthRule for HardcodedSecretRule {
    fn meta(&self) -> &'static HealthRuleMeta {
        &SEC001
    }

    fn evaluate(
        &self,
        context: &crate::engine::HealthContext,
        _config: &prism_core::config::HealthRuleConfig,
        cancel: &CancellationToken,
    ) -> prism_core::PrismResult<RuleResult> {
        let mut result = RuleResult::default();
        scan_contents(context, cancel, |path, line_number, line| {
            let Some((key, column)) = credential_line(line) else {
                return;
            };
            result.findings.push(secret_finding(
                "SEC001",
                "Possible Hard-coded Secret",
                "A credential-like assignment was detected (key name + long non-placeholder value).",
                path,
                line_number,
                column,
                &format!("credential-like assignment ({key})"),
                Severity::Warning,
                Confidence::Medium,
            ));
        })?;
        Ok(result)
    }
}

pub struct PrivateKeyRule;

impl HealthRule for PrivateKeyRule {
    fn meta(&self) -> &'static HealthRuleMeta {
        &SEC002
    }

    fn evaluate(
        &self,
        context: &crate::engine::HealthContext,
        _config: &prism_core::config::HealthRuleConfig,
        cancel: &CancellationToken,
    ) -> prism_core::PrismResult<RuleResult> {
        let mut result = RuleResult::default();
        scan_contents(context, cancel, |path, line_number, line| {
            if !pem_line(line) {
                return;
            }
            result.findings.push(secret_finding(
                "SEC002",
                "Private Key Material Detected",
                "A PEM private-key marker was detected.",
                path,
                line_number,
                1,
                "pem private key marker",
                Severity::Critical,
                Confidence::High,
            ));
        })?;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn credential_line_detects_assignment_shapes() {
        assert!(credential_line("api_key = \"supersecret12345678\"").is_some());
        assert!(credential_line("client_secret: verylongsecretvalue12345").is_some());
        // 占位符（§48/§110）。
        assert!(credential_line("password = \"example-password-123456\"").is_none());
        assert!(credential_line("token: dummy-token-value-123456").is_none());
        // 短值。
        assert!(credential_line("password = \"short\"").is_none());
        // URL 值。
        assert!(credential_line("token = https://long-ignored-value.example").is_none());
        // 无键名。
        assert!(credential_line("let value = \"anything1234567890\";").is_none());
    }

    #[test]
    fn pem_marker_detected() {
        assert!(pem_line("-----BEGIN PRIVATE KEY-----"));
        assert!(pem_line("-----BEGIN RSA PRIVATE KEY-----"));
        assert!(!pem_line("-----BEGIN CERTIFICATE-----"));
    }
}
