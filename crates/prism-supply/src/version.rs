//! 版本比较与范围匹配（M29 §11/§30）。
//!
//! 只做工程事实对照（字符串原样 + 数值分段比较），不做语义化版本
//! 全规范；不支持的操作符 ⇒ Unknown（诚实降级）。

/// 分段比较：按 `.`/`-`/`+` 切分，数字段数值比较，非数字段按字典序。
/// 前缀相等时较短的视为更小（1.2 < 1.2.0）。
pub fn compare_versions(a: &str, b: &str) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    let split = |v: &str| -> Vec<String> {
        v.trim()
            .split(['.', '-', '+'])
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect()
    };
    let (sa, sb) = (split(a), split(b));
    let len = sa.len().max(sb.len());
    for index in 0..len {
        let pa = sa.get(index);
        let pb = sb.get(index);
        match (pa, pb) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(x), Some(y)) => {
                let ord = match (x.parse::<u64>(), y.parse::<u64>()) {
                    (Ok(nx), Ok(ny)) => nx.cmp(&ny),
                    _ => x.cmp(y),
                };
                if ord != Ordering::Equal {
                    return ord;
                }
            }
        }
    }
    Ordering::Equal
}

/// `version_satisfies(resolved, request)`：支持精确相等、`*`/`x` 通配段、
/// 前缀通配（`8.0.*`）、以及逗号分隔的比较链（`>=1.0,<1.2.5`）。
/// 无法解析的请求 ⇒ false（调用方按 Unknown 表达，不当作满足）。
pub fn version_satisfies(resolved: &str, request: &str) -> bool {
    let request = request.trim();
    if request.is_empty() {
        return false;
    }
    if request.contains(',') {
        return request
            .split(',')
            .all(|part| single_satisfies(resolved, part.trim()));
    }
    single_satisfies(resolved, request)
}

fn single_satisfies(resolved: &str, request: &str) -> bool {
    let request = request.trim();
    let (op, target) = if let Some(rest) = request.strip_prefix(">=") {
        (">=", rest.trim())
    } else if let Some(rest) = request.strip_prefix("<=") {
        ("<=", rest.trim())
    } else if let Some(rest) = request.strip_prefix('>') {
        (">", rest.trim())
    } else if let Some(rest) = request.strip_prefix('<') {
        ("<", rest.trim())
    } else if let Some(rest) = request.strip_prefix("!=") {
        ("!=", rest.trim())
    } else if let Some(rest) = request.strip_prefix('=') {
        ("=", rest.trim())
    } else {
        ("=", request)
    };
    if target == "*" || target.is_empty() {
        return true;
    }
    match op {
        "=" => wildcard_matches(resolved, target),
        ">=" => compare_versions(resolved, target) != std::cmp::Ordering::Less,
        "<=" => compare_versions(resolved, target) != std::cmp::Ordering::Greater,
        ">" => compare_versions(resolved, target) == std::cmp::Ordering::Greater,
        "<" => compare_versions(resolved, target) == std::cmp::Ordering::Less,
        "!=" => !wildcard_matches(resolved, target),
        _ => false,
    }
}

/// `8.0.*` / `1.x`：通配段之后不再比较；`*`/`x` 段必须与已比较前缀一致。
fn wildcard_matches(resolved: &str, pattern: &str) -> bool {
    let pattern = pattern.trim();
    let split = |v: &str| -> Vec<String> {
        v.split('.')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect()
    };
    let (rv, pv) = (split(resolved), split(pattern));
    for (index, part) in pv.iter().enumerate() {
        if part == "*" || part.eq_ignore_ascii_case("x") {
            return true;
        }
        match rv.get(index) {
            Some(actual) if actual == part => continue,
            _ => return false,
        }
    }
    // 模式比解析版本更长且无通配 ⇒ 前缀一致即视为满足（1.2 ≈ 1.2.0）。
    rv.len() >= pv.len() || rv.starts_with(&rv[..rv.len().min(pv.len())])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordering_and_wildcards() {
        assert_eq!(
            compare_versions("1.2.3", "1.2.10"),
            std::cmp::Ordering::Less
        );
        assert_eq!(
            compare_versions("8.0.11", "8.0.11"),
            std::cmp::Ordering::Equal
        );
        assert!(version_satisfies("8.0.11", "8.0.*"));
        assert!(version_satisfies("8.0.11", "*"));
        assert!(!version_satisfies("9.0.0", "8.0.*"));
        assert!(version_satisfies("1.2.5", ">=1.0,<1.2.6"));
        assert!(!version_satisfies("1.3.0", ">=1.0,<1.2.6"));
        assert!(version_satisfies("13.0.3", "13.0.3"));
        assert!(!version_satisfies("2.0.0", "1.0.0"));
    }
}
