//! Redaction（M213 §12-§14）：复用 M29 redaction，**不建第二套秘密处理**。
//!
//! 两层防线：
//! 1. **敏感 header 名单**（Authorization/Cookie/Set-Cookie/Proxy-
//!    Authorization/X-Api-Key/Signature 类）⇒ 值整体 `[REDACTED]`；
//! 2. 其余 header/body 行经 M29 `redact_line`（pattern+context 候选）。

use prism_supply::secrets::redact_line;

/// 敏感 header 名单（小写匹配）。
const SENSITIVE_HEADERS: &[&str] = &[
    "authorization",
    "proxy-authorization",
    "cookie",
    "set-cookie",
    "x-api-key",
    "api-key",
    "x-auth-token",
    "x-signature",
    "stripe-signature",
    "x-hub-signature-256",
    "x-webhook-secret",
];

/// 单个 header 值脱敏（§13：`Authorization: Bearer [REDACTED]`）。
pub fn redact_header(name: &str, value: &str) -> String {
    if SENSITIVE_HEADERS.contains(&name.to_ascii_lowercase().as_str()) {
        format!("{name}: [REDACTED]")
    } else {
        let (redacted, _) = redact_line(value);
        format!("{name}: {redacted}")
    }
}

/// header map 整体脱敏。
pub fn redact_headers(
    headers: &std::collections::BTreeMap<String, String>,
) -> std::collections::BTreeMap<String, String> {
    headers
        .iter()
        .map(|(name, value)| (name.clone(), redact_header(name, value)))
        .collect()
}

/// 任意文本（body/log 行）脱敏。
pub fn redact_value(text: &str) -> String {
    let (redacted, _) = redact_line(text);
    redacted
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sensitive_headers_fully_redacted() {
        // TEST FIXTURE ONLY — 合成假凭据。
        assert_eq!(
            redact_header("Authorization", "Bearer sk-live-abcdef123456"),
            "Authorization: [REDACTED]"
        );
        assert_eq!(
            redact_header("Cookie", "session=abc123"),
            "Cookie: [REDACTED]"
        );
        assert_eq!(
            redact_header("X-Hub-Signature-256", "sha256=deadbeef"),
            "X-Hub-Signature-256: [REDACTED]"
        );
    }

    #[test]
    fn non_sensitive_headers_pattern_redacted_only() {
        assert_eq!(
            redact_header("X-Request-Id", "req_12345"),
            "X-Request-Id: req_12345"
        );
        let redacted = redact_header("X-Client-Key", "AKIAABCDEFGHIJKLMNOP");
        assert!(redacted.contains("…("), "M29 pattern 层仍生效");
        assert!(!redacted.contains("ABCDEFGHIJKLMNOP"));
    }

    #[test]
    fn body_text_redacted() {
        let redacted = redact_value("{\"api_key\": \"AKIAABCDEFGHIJKLMNOP\"}");
        assert!(!redacted.contains("ABCDEFGHIJKLMNOP"));
    }
}
