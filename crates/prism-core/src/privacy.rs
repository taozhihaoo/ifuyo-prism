//! 中央隐私 / 脱敏层（规格 M8 §77/§173–§176；M9 §346 复用要求）。
//!
//! 覆盖 stdout/stderr 文本、命令 args、环境元数据三类（§174）。
//! 模式：`TOKEN/PASSWORD/SECRET/KEY/API_KEY` 形态赋值、`Authorization:
//! Bearer`、`--token xxx` 形态参数。值一律 `[REDACTED]`，只保留键名（§46/§175）。
//!
//! 各里程碑（verify / env / …）必须复用本层，禁止各造一套互不兼容的脱敏逻辑。

const KEY_FRAGMENTS: [&str; 7] = [
    "token", "password", "passwd", "secret", "api_key", "apikey", "_key",
];

/// 单行脱敏：`<键> = 值` / `<键>: 值` / `<键>=值` 命中键名词 ⇒ 值替换。
pub fn redact_text(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for (index, line) in text.lines().enumerate() {
        if index > 0 {
            out.push('\n');
        }
        out.push_str(&redact_line(line));
    }
    if text.ends_with('\n') {
        out.push('\n');
    }
    out
}

fn redact_line(line: &str) -> String {
    let lower = line.to_ascii_lowercase();
    // Authorization: Bearer xxx（§77）。
    if let Some(pos) = lower.find("authorization:") {
        let after = lower[pos..].contains("bearer");
        if after {
            return format!("{}[REDACTED]", &line[..pos + "authorization:".len()]);
        }
    }
    for fragment in KEY_FRAGMENTS {
        if let Some(pos) = lower.find(fragment) {
            // 键名必须紧邻分隔符（容忍空格/引号）。
            let after_key = &lower[pos + fragment.len()..];
            let trimmed = after_key.trim_start();
            if trimmed.starts_with('=') || trimmed.starts_with(':') {
                return format!("{}{} [REDACTED]", &line[..pos + fragment.len()], &trimmed[..1]);
            }
        }
    }
    line.to_string()
}

/// args 脱敏：`--token`/`--password` 等参数的**下一个**参数替换为 [REDACTED]（§173）。
pub fn redact_args(args: &[String]) -> Vec<String> {
    let mut out = Vec::with_capacity(args.len());
    let mut redact_next = false;
    for arg in args {
        let lower = arg.to_ascii_lowercase();
        if redact_next {
            out.push("[REDACTED]".to_string());
            redact_next = false;
            continue;
        }
        if lower.starts_with("--")
            && KEY_FRAGMENTS
                .iter()
                .any(|fragment| lower.contains(fragment))
        {
            redact_next = true;
        }
        out.push(arg.clone());
    }
    out
}

/// 命令展示（§172/§173：program + redacted args）。
pub fn command_display(program: &str, args: &[String]) -> String {
    let mut out = program.to_string();
    for arg in redact_args(args) {
        out.push(' ');
        out.push_str(&arg);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_key_value_lines() {
        assert_eq!(
            redact_text("NPM_TOKEN=eyJhbGciOi"),
            "NPM_TOKEN= [REDACTED]"
        );
        assert_eq!(
            redact_text("MY_PASSWORD: hunter22019"),
            "MY_PASSWORD: [REDACTED]"
        );
        assert_eq!(redact_text("normal line"), "normal line");
    }

    #[test]
    fn redacts_bearer_authorization() {
        let out = redact_text("Authorization: Bearer eyJhbGciOiJIUzI1");
        assert!(out.contains("[REDACTED]"));
        assert!(!out.contains("eyJ"));
    }

    #[test]
    fn redacts_secret_args() {
        let out = redact_args(&[
            "deploy".to_string(),
            "--token".to_string(),
            "abc123".to_string(),
            "--verbose".to_string(),
        ]);
        assert_eq!(out[2], "[REDACTED]");
        assert_eq!(out[1], "--token");
        assert_eq!(out[3], "--verbose");
    }
}
