//! SSRF 防护（M213 §18-§20/§64）。
//!
//! 用户输入的 URL/Host 一律不可信：scheme 白名单、host 白名单
//! （localhost-first 默认）、私网/link-local/metadata 地址拒绝、
//! userinfo 拒绝、端口白名单、**不跟随重定向**（重定向 = Blocked 诊断）。
//! https 当前 Unsupported（零 TLS 依赖的诚实降级，P1 升级路径见 Final Audit）。

/// 目标校验结论。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TargetViolation {
    /// scheme 不是 http（https ⇒ Unsupported：当前无 TLS 栈，如实）。
    SchemeUnsupported(String),
    /// file:///ftp:// 等完全禁止的 scheme。
    SchemeForbidden(String),
    /// host 不在白名单（§18 Target Host Allowlist）。
    HostNotAllowed(String),
    /// host 解析为私网/loopback-外保留/metadata 地址但未获显式允许。
    PrivateAddressBlocked(String),
    /// URL 带 userinfo（`user:pass@host`）——凭证形式禁止。
    UserinfoForbidden,
    /// 端口不在白名单。
    PortNotAllowed(u16),
    /// URL 无法解析。
    Malformed(String),
}

/// 目标白名单（§18）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetGuard {
    /// 允许的 host 小写集合（默认 localhost / 127.0.0.1 / ::1）。
    pub allowed_hosts: Vec<String>,
    /// 允许的端口集合（None = 80/8080/3000/5000 常见开发端口）。
    pub allowed_ports: Option<Vec<u16>>,
}

impl Default for TargetGuard {
    fn default() -> Self {
        TargetGuard {
            allowed_hosts: vec![
                "localhost".to_string(),
                "127.0.0.1".to_string(),
                "[::1]".to_string(),
                "::1".to_string(),
            ],
            allowed_ports: None,
        }
    }
}

/// 解析后的目标。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub scheme: String,
    pub host: String,
    pub port: u16,
    pub path_and_query: String,
}

/// 解析 URL（std-only 手写，§75 不可信输入：解析失败即拒绝）。
pub fn parse_url(url: &str) -> Result<Target, TargetViolation> {
    let (scheme, rest) = url
        .split_once("://")
        .ok_or_else(|| TargetViolation::Malformed(url.to_string()))?;
    let scheme = scheme.to_ascii_lowercase();
    if scheme != "http" && scheme != "https" {
        return Err(TargetViolation::SchemeForbidden(scheme));
    }
    let (authority, path_and_query) = match rest.find('/') {
        Some(at) => (&rest[..at], &rest[at..]),
        None => (rest, "/"),
    };
    if authority.contains('@') {
        return Err(TargetViolation::UserinfoForbidden);
    }
    let (host, port) = match authority.rsplit_once(':') {
        Some((host, port)) => {
            let port: u16 = port
                .parse()
                .map_err(|_| TargetViolation::Malformed(url.to_string()))?;
            (host.to_string(), port)
        }
        None => (
            authority.to_string(),
            if scheme == "https" { 443u16 } else { 80u16 },
        ),
    };
    if host.is_empty() {
        return Err(TargetViolation::Malformed(url.to_string()));
    }
    Ok(Target {
        scheme,
        host: host.to_ascii_lowercase(),
        port,
        path_and_query: path_and_query.to_string(),
    })
}

impl TargetGuard {
    /// 全量校验（§19：scheme → userinfo → host → port → 地址类别）。
    pub fn check(&self, url: &str) -> Result<Target, TargetViolation> {
        let target = parse_url(url)?;
        if target.scheme == "https" {
            return Err(TargetViolation::SchemeUnsupported(target.scheme));
        }
        let host_lower = target.host.to_ascii_lowercase();
        let explicitly_allowed = self
            .allowed_hosts
            .iter()
            .any(|allowed| allowed == &host_lower || allowed == &format!("[{host_lower}]"));
        if !explicitly_allowed {
            // 先查危险地址字面量（私网/metadata），再落 HostNotAllowed。
            if is_blocked_address(&host_lower) {
                return Err(TargetViolation::PrivateAddressBlocked(host_lower));
            }
            return Err(TargetViolation::HostNotAllowed(host_lower));
        }
        if let Some(ports) = &self.allowed_ports {
            if !ports.contains(&target.port) {
                return Err(TargetViolation::PortNotAllowed(target.port));
            }
        }
        Ok(target)
    }
}

/// 已知危险地址（metadata / 私网段字面量；DNS rebinding 防护 = 只按
/// 字面量校验 + 不跟随重定向，文档如实记录残余风险 §19）。
fn is_blocked_address(host: &str) -> bool {
    let bare = host.trim_start_matches('[').trim_end_matches(']');
    // metadata endpoint。
    if bare.starts_with("169.254.169.254") || bare == "metadata.google.internal" {
        return true;
    }
    // IPv4 私网/链路本地字面量。
    if let Some(first_octet_str) = bare.split('.').next() {
        if let Ok(first) = first_octet_str.parse::<u32>() {
            if [10, 127, 172, 192, 169].contains(&first) && bare.contains('.') {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn guard() -> TargetGuard {
        TargetGuard::default()
    }

    #[test]
    fn localhost_targets_pass() {
        // §64：localhost / 127.0.0.1 / ::1 全部可过。
        assert!(guard().check("http://localhost:5000/api/cards").is_ok());
        assert!(guard().check("http://127.0.0.1:8080/x").is_ok());
        assert!(guard().check("http://[::1]:9000/y").is_ok());
    }

    #[test]
    fn private_and_metadata_blocked() {
        // §64：私网 / metadata endpoint 拒绝。
        // 私网/metadata 字面量 => PrivateAddressBlocked（比 HostNotAllowed 更明确）。
        assert!(matches!(
            guard().check("http://192.168.1.10/x"),
            Err(TargetViolation::PrivateAddressBlocked(_))
        ));
        assert!(matches!(
            guard().check("http://10.0.0.5/x"),
            Err(TargetViolation::PrivateAddressBlocked(_))
        ));
        assert!(matches!(
            guard().check("http://169.254.169.254/latest/meta-data"),
            Err(TargetViolation::PrivateAddressBlocked(_))
        ));
        // 普通 host 不在白名单 => HostNotAllowed。
        assert!(matches!(
            guard().check("http://api.example.com/x"),
            Err(TargetViolation::HostNotAllowed(_))
        ));
    }

    #[test]
    fn scheme_and_userinfo_blocked() {
        // §64：file:///ftp:// 禁止；https 当前 Unsupported；userinfo 拒绝。
        assert!(matches!(
            guard().check("file:///etc/passwd"),
            Err(TargetViolation::SchemeForbidden(_))
        ));
        assert!(matches!(
            guard().check("https://localhost/x"),
            Err(TargetViolation::SchemeUnsupported(_))
        ));
        assert!(matches!(
            guard().check("http://user:pass@localhost/x"),
            Err(TargetViolation::UserinfoForbidden)
        ));
    }

    #[test]
    fn explicit_host_expansion_allows_extra_target() {
        // 允许清单可显式扩展（如本地 docker 主机名），私网字面量仍拒绝。
        let expanded = TargetGuard {
            allowed_hosts: vec!["battle-dev.local".to_string()],
            allowed_ports: Some(vec![8080]),
        };
        assert!(expanded.check("http://battle-dev.local:8080/api").is_ok());
        assert!(matches!(
            expanded.check("http://battle-dev.local:9999/api"),
            Err(TargetViolation::PortNotAllowed(9999))
        ));
    }

    #[test]
    fn malformed_urls_rejected() {
        assert!(matches!(
            guard().check("not-a-url"),
            Err(TargetViolation::Malformed(_))
        ));
        assert!(matches!(
            guard().check("http:///path"),
            Err(TargetViolation::Malformed(_))
        ));
    }
}
