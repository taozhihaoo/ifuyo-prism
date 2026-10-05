//! Secret Candidate 检测（M29 §32-§36/§62/§67）。
//!
//! 原则：**候选而非确定**（evidence-first + conservative）。输出只含
//! redacted 上下文与内容寻址指纹——secret 原文绝不进入任何下游
//! （Finding/Evidence/日志/导出，§35/§36/§87）。

use prism_intel::meta::Confidence;
use serde::{Deserialize, Serialize};

/// 单条候选（无 secret 原文，§62：指纹不含内容）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretCandidate {
    /// `find:sec:{fnv1a16(rule|path|line|class)}`——稳定指纹，
    /// 与 M28 生命周期键同形（同位置同规则 ⇒ 同 id）。
    pub id: String,
    pub rule: String,
    pub path: String,
    pub line: usize,
    pub detection_class: String,
    /// 命中理由（pattern + 上下文，§77 Why 面向）。
    pub detection_reason: String,
    /// FP 抑制说明（§34：fixture/placeholder/低熵等，命中但降级）。
    pub exclusion_notes: Vec<String>,
    pub confidence: Confidence,
    /// redacted 上下文：变量名/前缀 + 长度——不含值。
    pub redacted_context: String,
}

/// 扫描上限（§68 资源限制：超限 ⇒ truncated，不假装扫完）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SecretScanLimits {
    /// 单文件最大扫描字节（超出跳过并记诊断）。
    pub max_file_bytes: usize,
    /// 候选总数上限（超出截断 ⇒ truncated）。
    pub max_candidates: usize,
}

impl Default for SecretScanLimits {
    fn default() -> Self {
        SecretScanLimits {
            max_file_bytes: 256 * 1024,
            max_candidates: 200,
        }
    }
}

/// 检测规则（规则名即 finding_type 后缀的稳定部分）。
struct Detector {
    rule: &'static str,
    class: &'static str,
    /// 逐行正则替代：手动谓词（无 regex 依赖；模式均为结构化特征）。
    kind: DetectorKind,
}

enum DetectorKind {
    /// 已知凭据前缀 + 固定长度字符集。
    Prefix {
        prefix: &'static str,
        min_len: usize,
        charset: fn(u8) -> bool,
    },
    /// 赋值形态：`name = "value"`（name 命中敏感词 + 值满足熵/长度）。
    Assignment { keywords: &'static [&'static str] },
    /// JWT 形态：三段 base64url 以 '.' 分隔且首段以 `eyJ` 开头。
    Jwt,
    /// PEM 私钥头。
    PrivateKeyHeader,
}

const BASE64ISH: fn(u8) -> bool = |b: u8| {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'-' || b == b'+' || b == b'/' || b == b'='
};

const DETECTORS: &[Detector] = &[
    Detector {
        rule: "aws_access_key_id",
        class: "cloud_credential",
        kind: DetectorKind::Prefix {
            prefix: "AKIA",
            min_len: 20,
            charset: |b| b.is_ascii_uppercase() || b.is_ascii_digit(),
        },
    },
    Detector {
        rule: "github_token",
        class: "token_like",
        kind: DetectorKind::Prefix {
            prefix: "ghp_",
            min_len: 40,
            charset: BASE64ISH,
        },
    },
    Detector {
        rule: "github_token",
        class: "token_like",
        kind: DetectorKind::Prefix {
            prefix: "github_pat_",
            min_len: 30,
            charset: BASE64ISH,
        },
    },
    Detector {
        rule: "slack_token",
        class: "token_like",
        kind: DetectorKind::Prefix {
            prefix: "xox",
            min_len: 18,
            charset: BASE64ISH,
        },
    },
    Detector {
        rule: "secret_assignment",
        class: "password_like_assignment",
        kind: DetectorKind::Assignment {
            keywords: &[
                "api_key",
                "apikey",
                "api-key",
                "secret",
                "password",
                "passwd",
                "access_token",
                "auth_token",
                "client_secret",
                "private_key",
            ],
        },
    },
    Detector {
        rule: "jwt_like_token",
        class: "jwt_like_token",
        kind: DetectorKind::Jwt,
    },
    Detector {
        rule: "private_key_block",
        class: "private_key",
        kind: DetectorKind::PrivateKeyHeader,
    },
];

/// 扫描结果：候选（按 path+line 排序）+ truncated 标记 + 诊断。
pub struct SecretScanOutcome {
    pub candidates: Vec<SecretCandidate>,
    pub truncated: bool,
    pub diagnostics: Vec<String>,
}

/// 扫描一批文本文件（content 已由调用方有界读取）。
pub fn scan_secrets(files: &[(String, String)], limits: &SecretScanLimits) -> SecretScanOutcome {
    let mut candidates = Vec::new();
    let mut truncated = false;
    let mut diagnostics = Vec::new();
    'files: for (path, content) in files {
        if content.len() > limits.max_file_bytes {
            diagnostics.push(format!(
                "secret scan skipped {path}: {} bytes > cap {}",
                content.len(),
                limits.max_file_bytes
            ));
            continue;
        }
        let looks_like_fixture = path_looks_like_fixture(path);
        for (line_number, line) in content.lines().enumerate() {
            for detector in DETECTORS {
                if let Some(hit) = detect(detector, line) {
                    if candidates.len() >= limits.max_candidates {
                        truncated = true;
                        break 'files;
                    }
                    candidates.push(build_candidate(
                        detector,
                        path,
                        line_number + 1,
                        &hit,
                        looks_like_fixture,
                    ));
                }
            }
        }
    }
    candidates.sort_by(|a, b| {
        a.path
            .cmp(&b.path)
            .then_with(|| a.line.cmp(&b.line))
            .then_with(|| a.id.cmp(&b.id))
    });
    SecretScanOutcome {
        candidates,
        truncated,
        diagnostics,
    }
}

struct Hit {
    /// 命中 token 在行内的字节区间（供行级 redaction 替换）。
    span: (usize, usize),
    /// redacted 表示（不含原文）。
    redacted_context: String,
    reason: String,
}

fn detect(detector: &Detector, line: &str) -> Option<Hit> {
    match &detector.kind {
        DetectorKind::Prefix {
            prefix,
            min_len,
            charset,
        } => {
            let mut search_from = 0usize;
            while let Some(at) = line[search_from..].find(prefix) {
                let start = search_from + at;
                let mut end = start + prefix.len();
                while end < line.len() && charset(line.as_bytes()[end]) {
                    end += 1;
                }
                let token = &line[start..end];
                if token.len() >= *min_len {
                    return Some(Hit {
                        span: (start, end),
                        redacted_context: redact(token),
                        reason: format!(
                            "known credential prefix `{prefix}` followed by {} chars",
                            token.len() - prefix.len()
                        ),
                    });
                }
                search_from = end.max(start + prefix.len());
            }
            None
        }
        DetectorKind::Assignment { keywords } => {
            let lowered = line.to_ascii_lowercase();
            let keyword = keywords.iter().copied().find(|kw| lowered.contains(kw))?;
            // 分隔符后取值段（引号内或 = : 后的连续字符）。
            let separator_at = lowered.find(keyword)? + keyword.len();
            let rest = &line[separator_at..];
            let value_start = rest
                .find(|c: char| c.is_ascii_alphanumeric())
                .map(|at| separator_at + at)?;
            let value: &str = line[value_start..]
                .split(|c: char| {
                    c.is_whitespace() || c == '"' || c == '\'' || c == ';' || c == ',' || c == ')'
                })
                .next()
                .unwrap_or("");
            if value.len() < 12 || !value.bytes().any(|b| b.is_ascii_digit()) {
                return None;
            }
            Some(Hit {
                span: (value_start, value_start + value.len()),
                redacted_context: format!("{} = {}", keyword, redact(value)),
                reason: format!(
                    "credential-like assignment `{keyword}=` with {}-char value",
                    value.len()
                ),
            })
        }
        DetectorKind::Jwt => {
            // 按 token 检测（行前缀如 `Authorization: Bearer` 不参与）。
            let mut offset = 0usize;
            for token in line.split_whitespace() {
                let token_start = offset + line[offset..].find(token).unwrap_or(0);
                offset = token_start + token.len();
                let parts: Vec<&str> = token.split('.').collect();
                if parts.len() == 3
                    && parts[0].starts_with("eyJ")
                    && parts[0].len() > 8
                    && parts[1].len() > 8
                    && parts[2].len() > 4
                    && parts.iter().all(|p| {
                        p.bytes()
                            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
                    })
                {
                    return Some(Hit {
                        span: (token_start, token_start + token.len()),
                        redacted_context: format!("JWT {} chars", token.len()),
                        reason: "three-segment base64url token with `eyJ` header".to_string(),
                    });
                }
            }
            None
        }
        DetectorKind::PrivateKeyHeader => {
            let trimmed = line.trim();
            if trimmed.starts_with("-----BEGIN")
                && trimmed.contains("PRIVATE KEY")
                && trimmed.ends_with("-----")
            {
                let lead = line.len() - line.trim_start().len();
                return Some(Hit {
                    span: (lead, lead + trimmed.len()),
                    redacted_context: trimmed.to_string(),
                    reason: "PEM private key header".to_string(),
                });
            }
            None
        }
    }
}

/// 行级脱敏（M210 §28/§29 复用点）：一行内所有候选 token 替换为
/// redacted 表示，返回 (脱敏文本, 替换数)。secret 原文不返回。
pub fn redact_line(line: &str) -> (String, usize) {
    let mut hits: Vec<(usize, usize, String)> = Vec::new();
    for detector in DETECTORS {
        if let Some(hit) = detect(detector, line) {
            hits.push((hit.span.0, hit.span.1, hit.redacted_context));
        }
    }
    if hits.is_empty() {
        return (line.to_string(), 0);
    }
    hits.sort_by(|a, b| b.0.cmp(&a.0).then(b.1.cmp(&a.1)));
    hits.dedup_by(|a, b| a.0 == b.0 && a.1 == b.1);
    let mut redacted = line.to_string();
    let mut count = 0usize;
    for (start, end, context) in hits {
        if start <= end && end <= redacted.len() && redacted.is_char_boundary(start) {
            redacted.replace_range(start..end, &context);
            count += 1;
        }
    }
    (redacted, count)
}

/// redact（§36）：保留前 4 字符 + 长度——不足以还原，足以人工定位。
fn redact(value: &str) -> String {
    let prefix: String = value.chars().take(4).collect();
    format!("{prefix}…({} chars)", value.chars().count())
}

/// 文件名/路径形态提示（§34 FP 考量：测试/示例/文档降级）。
fn path_looks_like_fixture(path: &str) -> bool {
    let lowered = path.to_ascii_lowercase();
    [
        "test", "fixture", "example", "sample", "spec", "mock", "docs/", "readme",
    ]
    .iter()
    .any(|marker| lowered.contains(marker))
}

fn build_candidate(
    detector: &Detector,
    path: &str,
    line: usize,
    hit: &Hit,
    looks_like_fixture: bool,
) -> SecretCandidate {
    let mut exclusion_notes = Vec::new();
    let mut confidence = Confidence::Medium;
    if looks_like_fixture {
        exclusion_notes.push("path matches test/fixture/example/docs pattern".to_string());
        confidence = Confidence::Low;
    }
    let fingerprint_input = format!("{}|{path}|{line}|{}", detector.rule, detector.class);
    SecretCandidate {
        id: format!(
            "find:sec:{}",
            prism_intel::identity::fnv1a_hex16(&fingerprint_input)
        ),
        rule: detector.rule.to_string(),
        path: path.to_string(),
        line,
        detection_class: detector.class.to_string(),
        detection_reason: hit.reason.clone(),
        exclusion_notes,
        confidence,
        redacted_context: hit.redacted_context.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scan(lines: &[(&str, &str)]) -> SecretScanOutcome {
        let files: Vec<(String, String)> = lines
            .iter()
            .map(|(path, content)| ((*path).to_string(), (*content).to_string()))
            .collect();
        scan_secrets(&files, &SecretScanLimits::default())
    }

    #[test]
    fn known_prefix_is_candidate_with_redacted_context() {
        // TEST FIXTURE ONLY — 合成假凭据（§93）。
        let outcome = scan(&[("src/app.cs", "var key = \"AKIAABCDEFGHIJKLMNOP\";")]);
        assert_eq!(outcome.candidates.len(), 1);
        assert_eq!(outcome.candidates[0].rule, "aws_access_key_id");
        assert_eq!(outcome.candidates[0].detection_class, "cloud_credential");
        let context = &outcome.candidates[0].redacted_context;
        assert!(context.contains('…'), "redacted：{} ", context);
        assert!(!outcome.candidates[0]
            .detection_reason
            .contains("ABCDEFGHIJKLMNOP"));
    }

    #[test]
    fn assignment_requires_length_and_digit() {
        let outcome = scan(&[("cfg.txt", "MY_SECRET = s3cr3tValueWithDigits42")]);
        assert_eq!(outcome.candidates.len(), 1);
        assert_eq!(outcome.candidates[0].rule, "secret_assignment");
        // 短值/无数字 ⇒ 不报（§33 不以"字符串长"定罪）。
        let quiet = scan(&[("cfg.txt", "SECRET = short")]);
        assert!(quiet.candidates.is_empty());
    }

    #[test]
    fn jwt_and_private_key_detected() {
        let outcome = scan(&[(
            "req.txt",
            "Authorization: Bearer eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U",
        )]);
        assert!(outcome
            .candidates
            .iter()
            .any(|c| c.rule == "jwt_like_token"));
        let pem = scan(&[("key.pem", "-----BEGIN RSA PRIVATE KEY-----")]);
        assert_eq!(pem.candidates[0].detection_class, "private_key");
    }

    #[test]
    fn fixture_path_downgrades_confidence_with_note() {
        let real = scan(&[(
            "src/a.cs",
            "token = \"ghp_AbcdefGHijklmnOpqrstuvWXYZ1234567890\"",
        )]);
        let fixture = scan(&[(
            "tests/fixture_a.cs",
            "token = \"ghp_AbcdefGHijklmnOpqrstuvWXYZ1234567890\"",
        )]);
        assert_eq!(real.candidates[0].confidence, Confidence::Medium);
        assert_eq!(fixture.candidates[0].confidence, Confidence::Low);
        assert!(!fixture.candidates[0].exclusion_notes.is_empty());
    }

    #[test]
    fn cap_truncates_and_flags() {
        let mut files = Vec::new();
        for index in 0..5 {
            files.push((format!("f{index}.txt"), "AKIAABCDEFGHIJKLMNOP".to_string()));
        }
        let limits = SecretScanLimits {
            max_file_bytes: 1024,
            max_candidates: 2,
        };
        let outcome = scan_secrets(&files, &limits);
        assert_eq!(outcome.candidates.len(), 2);
        assert!(outcome.truncated, "§69：截断必须可见");
    }

    #[test]
    fn redact_line_replaces_without_leaking() {
        let (redacted, count) = redact_line("token = AKIAABCDEFGHIJKLMNOP; ok");
        assert_eq!(count, 1);
        assert!(!redacted.contains("ABCDEFGHIJKLMNOP"));
        assert!(redacted.contains("…(20 chars)"));
        let (plain, zero) = redact_line("nothing suspicious here");
        assert_eq!(zero, 0);
        assert_eq!(plain, "nothing suspicious here");
    }

    #[test]
    fn deterministic_fingerprints() {
        let a = scan(&[("src/a.cs", "AKIAABCDEFGHIJKLMNOP")]);
        let b = scan(&[("src/a.cs", "AKIAABCDEFGHIJKLMNOP")]);
        assert_eq!(a.candidates[0].id, b.candidates[0].id);
    }
}
