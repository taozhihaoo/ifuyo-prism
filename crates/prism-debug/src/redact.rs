//! 脱敏入口（M210 §二十八/§二十九）：复用 M29 `prism-supply` 的检测器
//! 与 redact 形态——**不重新实现 Secret Scanner**。

/// 行级脱敏：返回 (redacted 文本, 替换数)。原文不返回（§二十九：
/// 存储与展示只允许 redacted 表示）。
pub fn redact_line(line: &str) -> (String, usize) {
    prism_supply::secrets::redact_line(line)
}
