use crate::types::Config;
use anyhow::{Context, Result};
use dirs;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

const CONFIG_DIR_NAME: &str = "danavi";
const CONFIG_FILE_NAME: &str = "config.json";

pub fn get_config_path() -> Result<PathBuf> {
    let config_dir = dirs::config_dir()
        .context("Could not find config directory")?
        .join(CONFIG_DIR_NAME);

    Ok(config_dir.join(CONFIG_FILE_NAME))
}

pub fn get_default_config() -> Config {
    Config {
        base_url: "http://localhost:4533".to_string(),
        username: String::new(),
        password: String::new(),
        show_easter_eggs: true,
    }
}

pub fn load_config() -> Result<Config> {
    let config_path = get_config_path()?;

    if config_path.exists() {
        let content = fs::read_to_string(&config_path).context("Failed to read config file")?;

        // Try to parse with current format first
        match serde_json::from_str::<Config>(&content) {
            Ok(config) => Ok(config),
            Err(_) => {
                // Try to parse old TypeScript format and migrate
                #[derive(Deserialize)]
                struct OldConfig {
                    #[serde(alias = "baseUrl", alias = "base_url")]
                    base_url: Option<String>,
                    username: Option<String>,
                    password: Option<String>,
                    #[serde(alias = "showEasterEggs", alias = "show_easter_eggs")]
                    show_easter_eggs: Option<bool>,
                }

                let old: OldConfig = serde_json::from_str(&content)
                    .context("Failed to parse config file (neither old nor new format)")?;

                let new_config = Config {
                    base_url: old
                        .base_url
                        .unwrap_or_else(|| get_default_config().base_url),
                    username: old.username.unwrap_or_default(),
                    password: old.password.unwrap_or_default(),
                    show_easter_eggs: old.show_easter_eggs.unwrap_or(true),
                };

                // Save in new format
                save_config(&new_config)?;
                Ok(new_config)
            }
        }
    } else {
        let default_config = get_default_config();
        save_config(&default_config)?;
        Ok(default_config)
    }
}

pub fn save_config(config: &Config) -> Result<()> {
    let config_path = get_config_path()?;
    let config_dir = config_path.parent().context("Invalid config path")?;

    fs::create_dir_all(config_dir).context("Failed to create config directory")?;

    let content = serde_json::to_string_pretty(config).context("Failed to serialize config")?;

    fs::write(&config_path, content).context("Failed to write config file")?;

    Ok(())
}

pub fn config_needs_edit(config: &Config) -> bool {
    let default = get_default_config();
    config.base_url == default.base_url
        && config.username == default.username
        && config.password == default.password
}
