//! `.env` 元数据检查（规格 M9b §58–§60/§201/§59 Git 集成）。
//!
//! 红线：**绝不读取/存储值**（§58"不把完整 .env 内容加载"、§60 只解析安全
//! 必要的 key names）；secret candidate（.env/.env.local）与 template
//! （.env.example/.env.template）区分（§201 不误判）；tracked 事实来自
//! M4 GitStatus（§59，只读）。

use crate::model::{
    EnvFileKind, EnvFileInfo, EnvKeyInfo, ENV_FILE_KEY_CAP, ENV_FILE_READ_CAP_BYTES,
};
use prism_core::task::CancellationToken;
use std::path::Path;

/// 候选文件清单（§58：.env/.env.local/.env.development + §101 ConfigurationSource）。
pub const ENV_FILE_CANDIDATES: &[&str] = &[
    ".env",
    ".env.local",
    ".env.development",
    ".env.production",
    ".env.example",
    ".env.template",
];

/// 文件名 → 类别（§201：template/example 的占位值属预期，不当 secret 候选）。
pub fn env_file_kind(path: &str) -> EnvFileKind {
    match path {
        ".env.example" | ".env.template" => EnvFileKind::Template,
        ".env" | ".env.local" | ".env.development" | ".env.production" => {
            EnvFileKind::SecretCandidate
        }
        _ => EnvFileKind::Unknown,
    }
}

/// 解析 key names（§60）：`KEY=...` 只取左侧；注释/空行跳过；
/// key 形态保守校验（无空白、不以 '=' 开头）；值直接丢弃。
pub fn parse_env_keys(content: &str) -> Vec<EnvKeyInfo> {
    let mut keys = Vec::new();
    for line in content.lines() {
        if keys.len() >= ENV_FILE_KEY_CAP {
            break;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let Some((name, _value)) = trimmed.split_once('=') else {
            continue; // 无 '=' 的行不猜。
        };
        let name = name.trim();
        if name.is_empty()
            || name.contains(char::is_whitespace)
            || !name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
        {
            continue;
        }
        keys.push(EnvKeyInfo {
            possibly_sensitive: crate::variables::is_possibly_sensitive(name),
            name: name.to_string(),
        });
    }
    keys
}

/// 采集 .env 系文件元数据。`tracked` 经 M4 GitStatus 判定（§59）：
/// 条目 untracked ⇒ false；条目非 untracked ⇒ true；不在条目中且仓库存在
/// ⇒ true（干净 = 已跟踪）；无仓库 ⇒ None。
pub fn collect_env_files(
    project_root: &Path,
    git: Option<&prism_git::GitService>,
    cancel: &CancellationToken,
) -> Vec<EnvFileInfo> {
    // 一次性取 GitStatus（只读，§83；§334 不做全量重扫——status 是 M4 常规操作）。
    let status = git.and_then(|git| git.status(project_root, false, cancel).ok());
    let mut files = Vec::new();
    for candidate in ENV_FILE_CANDIDATES {
        if cancel.is_cancelled() {
            break;
        }
        let path = project_root.join(candidate);
        if !path.is_file() {
            continue;
        }
        // §58：只读必要部分（64KB head 足够 key 名清单）。
        let content = read_head(&path, ENV_FILE_READ_CAP_BYTES);
        let keys = content
            .map(|content| parse_env_keys(&content))
            .unwrap_or_default();
        // §209：非 git 仓库 ⇒ 无法判定（None），不猜。
        let tracked = status.as_ref().and_then(|status| {
            let repository = status.repository.as_ref()?;
            let _ = repository;
            Some(match status
                .entries
                .iter()
                .find(|entry| entry.path == *candidate)
            {
                Some(entry) => !entry.is_untracked,
                None => true, // 干净文件 = 已跟踪。
            })
        });
        files.push(EnvFileInfo {
            path: candidate.to_string(),
            kind: env_file_kind(candidate),
            keys,
            tracked,
        });
    }
    files
}

fn read_head(path: &Path, max_bytes: u64) -> Option<String> {
    let metadata = std::fs::metadata(path).ok()?;
    if metadata.len() > max_bytes * 4 {
        // 异常巨大的 "env 文件" 不像 env——保守跳过而不是读入。
        return None;
    }
    let content = std::fs::read_to_string(path).ok()?;
    Some(content.chars().take(max_bytes as usize).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn temp(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "prism-env-envfile-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.subsec_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    // §201：secret candidate 与 template/example 不误判。
    #[test]
    fn secret_candidate_and_template_distinguished() {
        assert_eq!(env_file_kind(".env"), EnvFileKind::SecretCandidate);
        assert_eq!(env_file_kind(".env.local"), EnvFileKind::SecretCandidate);
        assert_eq!(env_file_kind(".env.example"), EnvFileKind::Template);
        assert_eq!(env_file_kind(".env.template"), EnvFileKind::Template);
        assert_eq!(env_file_kind(".env.production"), EnvFileKind::SecretCandidate);
    }

    // §60：只解析 key names，值绝不入库（结构性 + 序列化双重断言）。
    #[test]
    fn keys_parsed_values_never_stored() {
        let keys = parse_env_keys(
            "# comment\nDATABASE_URL=postgres://user:pass@host/db\nAPI_TOKEN=abc123secret\nNODE_ENV=development\ninvalid line\n=empty-key\nBAD KEY=1\n",
        );
        let names: Vec<&str> = keys.iter().map(|key| key.name.as_str()).collect();
        assert_eq!(names, vec!["DATABASE_URL", "API_TOKEN", "NODE_ENV"]);
        assert!(keys[0].possibly_sensitive);
        assert!(keys[1].possibly_sensitive);
        assert!(!keys[2].possibly_sensitive);
        let serialized = serde_json::to_string(&keys).unwrap();
        assert!(!serialized.contains("abc123secret"));
        assert!(!serialized.contains("postgres://"));
    }

    // §58 + §59：tracked 事实（真实 git 仓库）。
    #[test]
    fn tracked_fact_from_real_git_repo() {
        let dir = temp("tracked");
        let git = |args: &[&str]| {
            let output = std::process::Command::new("git")
                .args(args)
                .current_dir(&dir)
                .output()
                .expect("git available");
            assert!(output.status.success(), "{args:?}");
        };
        std::fs::write(dir.join(".env"), "API_TOKEN=real-secret\n").unwrap();
        std::fs::write(dir.join(".env.example"), "API_KEY=\n").unwrap();
        git(&["init", "-b", "main"]);
        git(&["config", "user.name", "Prism Test"]);
        git(&["config", "user.email", "test@prism.local"]);
        // 只跟踪 .env（untracked .env.example 留作对照）。
        git(&["add", ".env"]);
        git(&["commit", "-m", "base"]);

        let runner = Arc::new(prism_core::runtime::command::StdCommandRunner::new());
        let service = prism_git::GitService::new(runner);
        let cancel = CancellationToken::new();
        let files = collect_env_files(&dir, Some(&service), &cancel);

        let env_file = files.iter().find(|file| file.path == ".env").unwrap();
        assert_eq!(env_file.kind, EnvFileKind::SecretCandidate);
        assert_eq!(env_file.tracked, Some(true), "§59: git add 过 ⇒ tracked");
        assert_eq!(env_file.keys.len(), 1);
        assert!(env_file.keys[0].possibly_sensitive);

        let example = files.iter().find(|file| file.path == ".env.example").unwrap();
        assert_eq!(example.kind, EnvFileKind::Template);
        assert_eq!(example.tracked, Some(false), "未 add ⇒ untracked");

        // 非 git 目录 ⇒ None（§209 不猜）。
        let plain = temp("nogit");
        std::fs::write(plain.join(".env"), "A=1\n").unwrap();
        let files = collect_env_files(&plain, Some(&service), &cancel);
        assert_eq!(files[0].tracked, None);
    }
}
