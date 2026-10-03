//! 统一配置系统。
//!
//! - 配置文件：`prism.config.json`（命名遵循后续里程碑约定的 `prism.config.*` 形态；
//!   选择 JSON 而非 YAML 以避免引入额外依赖，M0 就锁定该决策）。
//! - 默认值齐全、允许未知字段（向前兼容）、加载时校验。
//! - 路径解析一律使用跨平台 API 与环境变量，禁止硬编码 Windows 路径。

use crate::error::{PrismError, PrismResult};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// 配置文件名（放在 [`config_dir`] 下）。
pub const CONFIG_FILE_NAME: &str = "prism.config.json";
/// 当前配置结构版本；不兼容变更时递增。
pub const CONFIG_SCHEMA_VERSION: u32 = 1;

/// 日志级别（JSON 为小写字符串，与 `log` crate 语义对应）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

impl LogLevel {
    pub fn to_log_level(self) -> log::Level {
        match self {
            LogLevel::Debug => log::Level::Debug,
            LogLevel::Info => log::Level::Info,
            LogLevel::Warn => log::Level::Warn,
            LogLevel::Error => log::Level::Error,
        }
    }
}

/// CLI 的默认输出形态（真实生效：`prism info --format json`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    Text,
    Json,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct GeneralConfig {
    pub default_output_format: OutputFormat,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        GeneralConfig {
            default_output_format: OutputFormat::Text,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct PathsConfig {
    /// 覆盖缓存/数据目录；None 表示使用平台默认（见 [`data_dir`]）。
    pub data_dir: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AnalysisConfig {
    /// 单文件分析大小上限（字节）。M1 扫描将消费该默认值。
    pub max_file_size_bytes: u64,
}

impl Default for AnalysisConfig {
    fn default() -> Self {
        AnalysisConfig {
            max_file_size_bytes: 2 * 1024 * 1024,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct LoggingConfig {
    pub level: LogLevel,
    /// 是否同时写入日志文件（`data_dir`/`logs/prism.log`）。
    pub file_enabled: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        LoggingConfig {
            level: LogLevel::Info,
            file_enabled: true,
        }
    }
}

/// 单条健康规则的配置覆盖（§71）。键 = 稳定规则 id；
/// `deny_unknown_fields` 把拼写错误（如 threshhold）变成解析错误而非静默忽略（§72）。
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct HealthRuleConfig {
    /// None = 使用规则默认开关（precedence：built-in → config → CLI，§221）。
    pub enabled: Option<bool>,
    /// 覆盖规则默认 severity（§219；必须用 M0 Severity 词表）。
    pub severity: Option<crate::severity::Severity>,
    /// 规则自定义阈值；各规则在元数据中声明语义，未声明阈值语义的规则
    /// 收到 threshold ⇒ 引擎期 diagnostic（§72 unknown option）。
    pub threshold: Option<u64>,
}

/// health 配置节（M7）。复用唯一配置系统 prism.config.json（§70 只选一种）。
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct HealthConfig {
    /// 规则级覆盖，键 = 稳定规则 id（如 "MAINT001"）。
    pub rules: std::collections::BTreeMap<String, HealthRuleConfig>,
    /// 整规则禁用列表（§65 快捷方式；与 rules.<id>.enabled=false 等效）。
    pub disabled_rules: Vec<String>,
    /// Finding 级抑制（§69：精确 FindingId 字符串；命中 ⇒ 检出但标记
    /// Suppressed——suppressed ≠ not detected，§67/§164）。
    pub suppressed_findings: Vec<String>,
}

impl HealthConfig {
    /// 规则的最终开关（config 合并后；CLI 覆盖由引擎再叠加，§221）。
    pub fn is_rule_enabled(&self, rule_id: &str, default_enabled: bool) -> Option<bool> {
        if self.disabled_rules.iter().any(|id| id == rule_id) {
            return Some(false);
        }
        self.rules
            .get(rule_id)
            .and_then(|rule| rule.enabled)
            .or(Some(default_enabled))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct PrivacyConfig {
    /// 生成上下文/日志时对疑似秘密脱敏。Prism 的默认立场，允许提高不允许关闭到 0 信任。
    pub redact_secrets: bool,
    /// 显式出网能力总开关。M0 阶段恒为 false（校验强制），未来任何网络能力
    /// 必须以 opt-in 设计并显式经用户触发。
    pub allow_outbound_network: bool,
}

impl Default for PrivacyConfig {
    fn default() -> Self {
        PrivacyConfig {
            redact_secrets: true,
            allow_outbound_network: false,
        }
    }
}

/// Prism 顶层配置。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct PrismConfig {
    pub schema_version: u32,
    pub general: GeneralConfig,
    pub paths: PathsConfig,
    pub analysis: AnalysisConfig,
    pub logging: LoggingConfig,
    pub privacy: PrivacyConfig,
    /// M7 健康规则配置（规则开关/覆盖 severity/阈值；§70 复用唯一配置系统）。
    #[serde(default)]
    pub health: HealthConfig,
}

impl Default for PrismConfig {
    fn default() -> Self {
        PrismConfig {
            schema_version: CONFIG_SCHEMA_VERSION,
            general: GeneralConfig::default(),
            paths: PathsConfig::default(),
            analysis: AnalysisConfig::default(),
            logging: LoggingConfig::default(),
            privacy: PrivacyConfig::default(),
            health: HealthConfig::default(),
        }
    }
}

impl PrismConfig {
    /// 结构校验。加载与保存前都应调用。
    pub fn validate(&self) -> PrismResult<()> {
        if self.schema_version != CONFIG_SCHEMA_VERSION {
            return Err(PrismError::Config {
                message: format!(
                    "unsupported config schema_version {} (expected {})",
                    self.schema_version, CONFIG_SCHEMA_VERSION
                ),
            });
        }
        if self.analysis.max_file_size_bytes == 0 {
            return Err(PrismError::Config {
                message: "analysis.max_file_size_bytes must be greater than 0".to_string(),
            });
        }
        if self.privacy.allow_outbound_network {
            return Err(PrismError::Config {
                message: "privacy.allow_outbound_network is not available in this build; \
                          outbound network is a future opt-in capability"
                    .to_string(),
            });
        }
        Ok(())
    }

    /// 从指定路径加载配置（文件不存在返回默认值与 `false`）。
    pub fn load(path: &Path) -> PrismResult<(Self, bool)> {
        if !path.exists() {
            return Ok((Self::default(), false));
        }
        let text = std::fs::read_to_string(path)?;
        // 未知字段被忽略（向前兼容），缺失字段取默认值。
        let config: PrismConfig = serde_json::from_str(&text)?;
        config.validate()?;
        Ok((config, true))
    }

    /// 保存配置（pretty JSON，稳定字段顺序由 serde 结构定义保证）。
    pub fn save(&self, path: &Path) -> PrismResult<()> {
        self.validate()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let text = serde_json::to_string_pretty(self)?;
        std::fs::write(path, text)?;
        Ok(())
    }
}

/// 用户级配置目录（跨平台）：
/// - Windows: `%APPDATA%\prism`
/// - macOS:   `~/Library/Application Support/prism`
/// - 其他:    `$XDG_CONFIG_HOME/prism` 或 `~/.config/prism`
pub fn config_dir() -> PrismResult<PathBuf> {
    if cfg!(target_os = "windows") {
        let base = std::env::var("APPDATA").map_err(|_| PrismError::Config {
            message: "APPDATA environment variable is not set".to_string(),
        })?;
        Ok(PathBuf::from(base).join("prism"))
    } else if cfg!(target_os = "macos") {
        let home = home_dir()?;
        Ok(home
            .join("Library")
            .join("Application Support")
            .join("prism"))
    } else {
        let base = match std::env::var("XDG_CONFIG_HOME") {
            Ok(dir) if !dir.is_empty() => PathBuf::from(dir),
            _ => home_dir()?.join(".config"),
        };
        Ok(base.join("prism"))
    }
}

/// 用户级数据/缓存根目录（跨平台，同理不硬编码盘符）：
/// - Windows: `%LOCALAPPDATA%\prism`
/// - macOS:   `~/Library/Application Support/prism`
/// - 其他:    `$XDG_DATA_HOME/prism` 或 `~/.local/share/prism`
pub fn default_data_dir() -> PrismResult<PathBuf> {
    if cfg!(target_os = "windows") {
        let base = std::env::var("LOCALAPPDATA").map_err(|_| PrismError::Config {
            message: "LOCALAPPDATA environment variable is not set".to_string(),
        })?;
        Ok(PathBuf::from(base).join("prism"))
    } else if cfg!(target_os = "macos") {
        let home = home_dir()?;
        Ok(home
            .join("Library")
            .join("Application Support")
            .join("prism"))
    } else {
        let base = match std::env::var("XDG_DATA_HOME") {
            Ok(dir) if !dir.is_empty() => PathBuf::from(dir),
            _ => home_dir()?.join(".local").join("share"),
        };
        Ok(base.join("prism"))
    }
}

/// 解析实际使用的数据目录：配置覆盖优先，否则平台默认。
pub fn resolve_data_dir(config: &PrismConfig) -> PrismResult<PathBuf> {
    match &config.paths.data_dir {
        Some(dir) => Ok(dir.clone()),
        None => default_data_dir(),
    }
}

pub fn default_config_path() -> PrismResult<PathBuf> {
    Ok(config_dir()?.join(CONFIG_FILE_NAME))
}

fn home_dir() -> PrismResult<PathBuf> {
    // Windows 上 USERPROFILE，Unix 上 HOME；均为标准环境变量。
    let key = if cfg!(target_os = "windows") {
        "USERPROFILE"
    } else {
        "HOME"
    };
    std::env::var(key)
        .map(PathBuf::from)
        .map_err(|_| PrismError::Config {
            message: format!("{key} environment variable is not set"),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_passes_validation() {
        PrismConfig::default().validate().unwrap();
    }

    #[test]
    fn health_config_merges_deterministically() {
        let config = HealthConfig {
            rules: [(
                "MAINT001".to_string(),
                HealthRuleConfig {
                    enabled: Some(false),
                    severity: None,
                    threshold: None,
                },
            )]
            .into_iter()
            .collect(),
            disabled_rules: vec!["SEC001".to_string()],
            suppressed_findings: vec![],
        };
        // §221 precedence：disabled_rules > rules.<id>.enabled > default。
        assert_eq!(config.is_rule_enabled("SEC001", true), Some(false));
        assert_eq!(config.is_rule_enabled("MAINT001", true), Some(false));
        assert_eq!(config.is_rule_enabled("UNKNOWN999", true), Some(true));
        assert_eq!(config.is_rule_enabled("OTHER", false), Some(false));
    }

    #[test]
    fn health_rule_config_rejects_unknown_fields() {
        // §72：threshhold 拼写错误必须报错，不得静默接受。
        let result: Result<HealthRuleConfig, _> = serde_json::from_str(r#"{"threshhold": 200}"#);
        assert!(result.is_err(), "unknown option must fail parsing");
        let ok: HealthRuleConfig =
            serde_json::from_str(r#"{"enabled": false, "severity": "warning", "threshold": 5}"#)
                .unwrap();
        assert_eq!(ok.enabled, Some(false));
        assert_eq!(ok.severity, Some(crate::severity::Severity::Warning));
    }

    #[test]
    fn roundtrip_preserves_values() {
        let mut config = PrismConfig::default();
        config.logging.level = LogLevel::Debug;
        config.analysis.max_file_size_bytes = 1024;
        let text = serde_json::to_string_pretty(&config).unwrap();
        let back: PrismConfig = serde_json::from_str(&text).unwrap();
        assert_eq!(back, config);
    }

    #[test]
    fn unknown_fields_are_tolerated() {
        let text = r#"{
            "schema_version": 1,
            "future_section": { "whatever": true }
        }"#;
        let config: PrismConfig = serde_json::from_str(text).unwrap();
        assert_eq!(config, PrismConfig::default());
    }

    #[test]
    fn wrong_schema_version_is_rejected() {
        let text = r#"{ "schema_version": 999 }"#;
        let config: PrismConfig = serde_json::from_str(text).unwrap();
        let err = config.validate().unwrap_err();
        assert_eq!(err.code(), crate::error::PrismErrorCode::Config);
    }

    #[test]
    fn outbound_network_flag_is_rejected() {
        let text = r#"{ "schema_version": 1, "privacy": { "allow_outbound_network": true } }"#;
        let config: PrismConfig = serde_json::from_str(text).unwrap();
        let err = config.validate().unwrap_err();
        assert!(err.message().contains("outbound"));
    }

    #[test]
    fn load_missing_file_returns_defaults() {
        let path = std::env::temp_dir().join("prism-core-config-missing.json");
        let _ = std::fs::remove_file(&path);
        let (config, loaded) = PrismConfig::load(&path).unwrap();
        assert!(!loaded);
        assert_eq!(config, PrismConfig::default());
    }

    #[test]
    fn save_and_load_roundtrip() {
        let dir = std::env::temp_dir().join(format!("prism-core-config-{}", std::process::id()));
        let path = dir.join(CONFIG_FILE_NAME);
        let mut config = PrismConfig::default();
        config.analysis.max_file_size_bytes = 4096;
        config.save(&path).unwrap();
        let (back, loaded) = PrismConfig::load(&path).unwrap();
        assert!(loaded);
        assert_eq!(back.analysis.max_file_size_bytes, 4096);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn resolve_data_dir_prefers_override() {
        let mut config = PrismConfig::default();
        config.paths.data_dir = Some(PathBuf::from("/tmp/prism-data"));
        assert_eq!(
            resolve_data_dir(&config).unwrap(),
            PathBuf::from("/tmp/prism-data")
        );
    }
}
