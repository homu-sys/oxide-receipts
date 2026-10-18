use serde::Deserialize;
use std::path::Path;

pub const DEFAULT_REGISTRY: &str = "https://github.com/homu-sys/oxide-receipts.git";

#[derive(Default, Deserialize)]
pub struct ToolConfig {
    pub registry: Option<RegistrySection>,
}

#[derive(Default, Deserialize)]
pub struct RegistrySection {
    pub url: Option<String>,
}

pub fn resolve_registry_url(cli_override: Option<&str>, config_path: &Path) -> String {
    if let Some(url) = cli_override {
        return url.to_string();
    }
    if let Ok(content) = std::fs::read_to_string(config_path) {
        if let Ok(cfg) = toml::from_str::<ToolConfig>(&content) {
            if let Some(url) = cfg.registry.and_then(|r| r.url) {
                return url;
            }
        }
    }
    DEFAULT_REGISTRY.to_string()
}
