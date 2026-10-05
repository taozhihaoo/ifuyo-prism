//! 消息规范化（M210 §八）：动态变量归一用于指纹，**不过度归一**——
//! 归一只发生在指纹层，原始消息（redacted 后）保留在事件上。

/// 归一化：路径 token（含 `/` 或 `\` 或盘符）→ `<PATH>`；纯数字段 →
/// `<N>`；GUID 形态 → `<ID>`；长十六进制（≥16）→ `<HEX>`。
/// 引号内的路径同样替换；其余文本原样（避免不同问题被错误合并）。
pub fn normalize_message(message: &str) -> String {
    let mut out = String::with_capacity(message.len());
    for (index, word) in message.split_whitespace().enumerate() {
        if index > 0 {
            out.push(' ');
        }
        out.push_str(&normalize_token(word));
    }
    out
}

fn normalize_token(word: &str) -> String {
    // 剥离常见包裹符后再判断（引号内的路径也归一）。
    let trimmed = word
        .trim_matches(|c: char| matches!(c, '"' | '\'' | '(' | ')' | '[' | ']' | ',' | ':' | ';'));
    if is_guid(trimmed) {
        return word.replace(trimmed, "<ID>");
    }
    if is_long_hex(trimmed) {
        return word.replace(trimmed, "<HEX>");
    }
    if is_path_like(trimmed) {
        // 保留 res:// 前缀语义（Godot 资源引用），路径本体归一。
        if let Some(rest) = trimmed.strip_prefix("res://") {
            return word.replace(rest, "<PATH>");
        }
        return word.replace(trimmed, "<PATH>");
    }
    if is_numeric_like(trimmed) {
        return word.replace(trimmed, "<N>");
    }
    word.to_string()
}

fn is_path_like(token: &str) -> bool {
    if token.len() < 3 {
        return false;
    }
    token.starts_with("res://")
        || token.starts_with("user://")
        || token.contains('\\')
        || token.contains('/')
        || token.contains("~/.")
        || (token.as_bytes().get(1) == Some(&b':') && token.as_bytes()[0].is_ascii_alphabetic())
}

fn is_guid(token: &str) -> bool {
    let bytes = token.as_bytes();
    bytes.len() == 36
        && bytes.iter().enumerate().all(|(index, b)| {
            if matches!(index, 8 | 13 | 18 | 23) {
                *b == b'-'
            } else {
                b.is_ascii_hexdigit()
            }
        })
}

fn is_long_hex(token: &str) -> bool {
    token.len() >= 16 && token.bytes().all(|b| b.is_ascii_hexdigit())
}

/// 纯数字或点分数字（IP/版本/计数形态）⇒ 动态量纲，指纹归一。
fn is_numeric_like(token: &str) -> bool {
    if token.is_empty() {
        return false;
    }
    // 数字/点分（IP、版本、计数、`123.dat` 形态的数字文件名）。
    let body = token.rsplit_once('.').map_or(token, |(head, _)| head);
    !body.is_empty()
        && body.bytes().all(|b| b.is_ascii_digit() || b == b'.')
        && token.bytes().any(|b| b.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_numbers_guids_normalize() {
        assert_eq!(
            normalize_message("File not found: C:\\Users\\Alice\\game\\save\\12345.dat"),
            "File not found: <PATH>"
        );
        assert_eq!(
            normalize_message("loading res://Scripts/Battle/BattleController.cs failed"),
            "loading res://<PATH> failed"
        );
        assert_eq!(
            normalize_message("retry 3 after 500 ms"),
            "retry <N> after <N> ms"
        );
        assert_eq!(
            normalize_message("session 550e8400-e29b-41d4-a716-446655440000 ended"),
            "session <ID> ended"
        );
        assert_eq!(
            normalize_message("hash deadbeefdeadbeef mismatch"),
            "hash <HEX> mismatch"
        );
        assert_eq!(
            normalize_message("connection refused to 10.0.0.1"),
            "connection refused to <N>"
        );
    }

    #[test]
    fn distinct_problems_stay_distinct() {
        // §八：不允许过度归一化——不同错误文本仍然不同。
        assert_ne!(
            normalize_message("File not found: a.dat"),
            normalize_message("Access denied: a.dat")
        );
        // 数字归一让同一问题的不同实例合并（期望行为）。
        assert_eq!(
            normalize_message("File not found: 1.dat"),
            normalize_message("File not found: 2.dat")
        );
    }
}
