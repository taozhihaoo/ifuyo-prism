//! License 事实与 Policy 判定（M29 §20-§22）。
//!
//! License = 工程事实 + 风险提示，**不是法律意见**（§1/§20）：
//! `LicenseDeclared = MIT` 是 FACT；"兼容性潜在问题" 是 INFERENCE；
//! 法律结论禁止直接给出。依赖侧元数据离线不可得 ⇒ UNKNOWN（§21），
//! 不等于"没有 license"。

use prism_intel::evidence::EvidenceId;
use serde::{Deserialize, Serialize};

/// License 来源（§20 至少区分）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LicenseSource {
    /// 包/项目自身 manifest 的 license 字段或声明元素。
    PackageMetadata,
    Manifest,
    LicenseFile,
    SpdxIdentifier,
    RegistryMetadata,
    Unknown,
}

/// 一条 license 事实。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LicenseFact {
    /// 主体：项目自身（"project"）或 `{ecosystem}:{package_id}`。
    pub subject: String,
    /// SPDX 表达式原样（`MIT` / `Apache-2.0 OR MIT` / 自定义文本）。
    pub expression: String,
    /// 已知 SPDX 标识（保守词表命中 ⇒ true；否则 Custom/Unknown）。
    pub spdx_known: bool,
    pub source: LicenseSource,
    pub origin: prism_intel::meta::Origin,
    pub evidence: Vec<EvidenceId>,
}

/// License policy（§65：M28 Quality Gate 的 Policy 数据，非新框架）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct LicensePolicy {
    /// 命中即 PolicyViolation（GPL-3.0 等原样 SPDX id）。
    pub blocked: Vec<String>,
    /// 要求全部已知 license 落在允许集内（空 = 不启用白名单模式）。
    pub allowed: Vec<String>,
    /// true ⇒ license UNKNOWN 也产生 finding（MissingLicenseMetadata）。
    pub require_known: bool,
}

/// SPDX 已知标识保守词表（§20：实际支持范围由真实实现决定）。
const KNOWN_SPDX: &[&str] = &[
    "MIT",
    "Apache-2.0",
    "BSD-2-Clause",
    "BSD-3-Clause",
    "BSD-4-Clause",
    "ISC",
    "MPL-2.0",
    "GPL-2.0",
    "GPL-2.0-only",
    "GPL-2.0-or-later",
    "GPL-3.0",
    "GPL-3.0-only",
    "GPL-3.0-or-later",
    "LGPL-2.1",
    "LGPL-2.1-only",
    "LGPL-2.1-or-later",
    "LGPL-3.0",
    "LGPL-3.0-only",
    "LGPL-3.0-or-later",
    "Unlicense",
    "CC0-1.0",
    "0BSD",
];

/// 表达式是否命中已知 SPDX 词表（复合表达式的每个 token 都须已知；
/// OR/AND/WITH 分词后比对）。
pub fn is_known_spdx(expression: &str) -> bool {
    let trimmed = expression.trim();
    if trimmed.is_empty() {
        return false;
    }
    trimmed
        .split(|c: char| c.is_whitespace() || c == '(' || c == ')')
        .filter(|token| !token.is_empty() && !matches!(*token, "OR" | "AND" | "WITH"))
        .all(|token| {
            KNOWN_SPDX
                .iter()
                .any(|known| known.eq_ignore_ascii_case(token))
        })
}

/// policy 判定（§22 模型：fact × policy ⇒ finding 结论 + 解释链）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LicensePolicyOutcome {
    pub subject: String,
    pub violated: bool,
    /// 解释链原样：`fact: GPL-3.0 (manifest) × policy.blocked=[GPL-3.0]`。
    pub reasoning: String,
}

pub fn evaluate_license_policy(
    facts: &[LicenseFact],
    policy: &LicensePolicy,
) -> Vec<LicensePolicyOutcome> {
    let mut out = Vec::new();
    for fact in facts {
        let violated = policy.blocked.iter().any(|blocked| {
            fact.expression
                .split(|c: char| c.is_whitespace() || c == '(' || c == ')')
                .any(|token| token.eq_ignore_ascii_case(blocked))
        });
        if violated {
            out.push(LicensePolicyOutcome {
                subject: fact.subject.clone(),
                violated: true,
                reasoning: format!(
                    "fact: `{}` (source {:?}) × policy.blocked={:?}",
                    fact.expression, fact.source, policy.blocked
                ),
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fact(subject: &str, expression: &str) -> LicenseFact {
        LicenseFact {
            subject: subject.to_string(),
            expression: expression.to_string(),
            spdx_known: is_known_spdx(expression),
            source: LicenseSource::Manifest,
            origin: prism_intel::meta::Origin::Fact,
            evidence: Vec::new(),
        }
    }

    #[test]
    fn spdx_known_and_unknown() {
        assert!(is_known_spdx("MIT"));
        assert!(is_known_spdx("Apache-2.0 OR MIT"));
        assert!(!is_known_spdx("My-Custom-License-1.0"));
        assert!(!is_known_spdx(""));
    }

    #[test]
    fn blocked_license_produces_explainable_violation() {
        // §22 模型：fact GPL-3.0 × policy 禁止 ⇒ PolicyViolation（可解释）。
        let facts = vec![
            fact("nuget:Gpl.Pkg", "GPL-3.0-only"),
            fact("nuget:Ok.Pkg", "MIT"),
        ];
        let policy = LicensePolicy {
            blocked: vec!["GPL-3.0-only".into()],
            ..Default::default()
        };
        let outcomes = evaluate_license_policy(&facts, &policy);
        assert_eq!(outcomes.len(), 1);
        assert_eq!(outcomes[0].subject, "nuget:Gpl.Pkg");
        assert!(outcomes[0].reasoning.contains("GPL-3.0-only"));
        assert!(outcomes[0].reasoning.contains("policy.blocked"));
    }

    #[test]
    fn unknown_license_is_not_empty_license() {
        // §21：缺 license 元数据 ⇒ UNKNOWN 事实（spdx_known=false），
        // 不产生"没有 license"的表达。
        let unknown = fact("nuget:Mystery.Pkg", "");
        assert!(!unknown.spdx_known);
    }
}
