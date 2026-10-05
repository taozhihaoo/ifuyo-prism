//! Remote Path Safety（M214 §四十六）：一切相对路径解析于 RemoteRoot
//! 之下；`..`/绝对路径/symlink 逃逸拒绝；无法保证时如实 Unknown。

pub use prism_intel::identity::fnv1a_hex16;

/// 路径违规类型。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathViolation {
    /// 绝对路径（`/x`、`C:\x`）——相对路径必须相对 RemoteRoot。
    AbsolutePath(String),
    /// `..` 逃逸 RemoteRoot。
    ParentEscape(String),
    /// 空路径。
    Empty,
}

/// 校验并把相对路径解析为 RemoteRoot 之下的路径（`/` 分隔，不触盘）。
/// 返回规范化后的相对路径（供 transport 组合绝对路径）。
pub fn resolve_under_root(remote_root: &str, relative: &str) -> Result<String, PathViolation> {
    if relative.is_empty() {
        return Err(PathViolation::Empty);
    }
    if relative.starts_with('/') || relative.starts_with('\\') {
        return Err(PathViolation::AbsolutePath(relative.to_string()));
    }
    if relative.len() >= 2 && relative.as_bytes()[1] == b':' {
        return Err(PathViolation::AbsolutePath(relative.to_string()));
    }
    let mut segments: Vec<&str> = Vec::new();
    for segment in relative.split(['/', '\\']) {
        match segment {
            "" | "." => {}
            ".." => {
                segments
                    .pop()
                    .ok_or_else(|| PathViolation::ParentEscape(relative.to_string()))?;
            }
            other => segments.push(other),
        }
    }
    if segments.is_empty() {
        return Err(PathViolation::ParentEscape(relative.to_string()));
    }
    Ok(format!(
        "{}/{}",
        remote_root.trim_end_matches('/').trim_end_matches('\\'),
        segments.join("/")
    ))
}

/// 组合 release 路径：`<root>/releases/<release_id>`（§二十一）。
pub fn release_path(remote_root: &str, release_id: &str) -> Result<String, PathViolation> {
    if release_id.contains("..") || release_id.is_empty() {
        return Err(PathViolation::ParentEscape(release_id.to_string()));
    }
    resolve_under_root(remote_root, &format!("releases/{release_id}"))
}

/// 组合 current 指针路径。
pub fn current_path(remote_root: &str) -> String {
    format!(
        "{}/current",
        remote_root.trim_end_matches('/').trim_end_matches('\\')
    )
}

/// release id 生成（确定性：时间戳 + artifact hash 前缀，§六十四 fixtures
/// 可注入时间）。
pub fn make_release_id(timestamp_compact: &str, artifact_sha: &str) -> String {
    format!("{}-{}", timestamp_compact, fnv1a_hex16(artifact_sha))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_paths_resolve_under_root() {
        assert_eq!(
            resolve_under_root("/srv/app", "releases/r1/app.bin").unwrap(),
            "/srv/app/releases/r1/app.bin"
        );
        assert_eq!(
            resolve_under_root("C:/deploy", "releases\\r1\\x.bin").unwrap(),
            "C:/deploy/releases/r1/x.bin"
        );
    }

    #[test]
    fn parent_escape_blocked() {
        // §四十六：../ 逃逸拒绝。
        assert!(matches!(
            resolve_under_root("/srv/app", "../../../etc/passwd"),
            Err(PathViolation::ParentEscape(_))
        ));
        assert!(matches!(
            resolve_under_root("/srv/app", "releases/../../etc"),
            Err(PathViolation::ParentEscape(_))
        ));
    }

    #[test]
    fn absolute_paths_blocked() {
        assert!(matches!(
            resolve_under_root("/srv/app", "/etc/passwd"),
            Err(PathViolation::AbsolutePath(_))
        ));
        assert!(matches!(
            resolve_under_root("/srv/app", "C:\\Windows\\system32"),
            Err(PathViolation::AbsolutePath(_))
        ));
        assert!(matches!(
            resolve_under_root("/srv/app", ""),
            Err(PathViolation::Empty)
        ));
    }

    #[test]
    fn release_paths_are_bounded() {
        assert_eq!(
            release_path("/srv/app", "20261006-abc").unwrap(),
            "/srv/app/releases/20261006-abc"
        );
        assert!(release_path("/srv/app", "../../evil").is_err());
    }
}
