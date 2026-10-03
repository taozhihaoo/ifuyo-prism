//! `prism health rules` — 规则注册表检视（规格 M7 §74/§129/§220）。
//!
//! 输出每条规则的 id/name/category 与**合并配置后的最终开关状态**
//! （precedence：built-in defaults → prism.config.json `health` 节；
//! CLI 覆盖属于完整 `prism health .` 命令，M7b 提供）。

use prism_core::config::PrismConfig;
use prism_health::HealthRuleRegistry;
use std::path::Path;

pub fn run_rules(path: &Path, json: bool) -> i32 {
    if !path.exists() {
        eprintln!("prism: path does not exist: {}", path.to_string_lossy());
        return crate::EXIT_INVALID_INPUT;
    }

    // 项目配置（不存在 ⇒ 默认配置；§70 单一配置系统）。
    let config_path = path.join(PrismConfig::config_file_name());
    let (config, _loaded) = PrismConfig::load(&config_path).unwrap_or_else(|err| {
        eprintln!("prism: config invalid: {err}");
        std::process::exit(crate::EXIT_ERROR);
    });

    let registry = HealthRuleRegistry::with_builtins();
    let rows: Vec<_> = registry
        .rules()
        .map(|rule| {
            let meta = rule.meta();
            (
                meta.id.to_string(),
                meta.name.to_string(),
                meta.category,
                config
                    .health
                    .is_rule_enabled(meta.id, meta.default_enabled)
                    .unwrap_or(meta.default_enabled),
            )
        })
        .collect();

    if json {
        let value: Vec<serde_json::Value> = rows
            .iter()
            .map(|(id, name, category, enabled)| {
                serde_json::json!({
                    "id": id,
                    "name": name,
                    "category": format!("{category:?}").to_ascii_lowercase(),
                    "enabled": enabled,
                })
            })
            .collect();
        match serde_json::to_string_pretty(&value) {
            Ok(text) => println!("{text}"),
            Err(err) => {
                eprintln!("prism: failed to serialize rules: {err}");
                return crate::EXIT_ERROR;
            }
        }
    } else {
        for (id, name, category, enabled) in &rows {
            let state = if *enabled { "enabled" } else { "disabled" };
            println!("{id:<9} {name:<28} {category:?}  {state}");
        }
    }
    crate::EXIT_OK
}
