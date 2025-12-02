use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub api_key: String,
    // later: base URL, extra fields per provider, etc.
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DefaultProvider {
    OpenWeather,
    WeatherApi,
    // later: AccuWeather, etc.
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    pub default_provider: Option<DefaultProvider>,
    pub providers: std::collections::HashMap<String, ProviderConfig>,
}

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        // TODO: implement reading from config file
        Ok(Self::default())
    }

    pub fn save(&self) -> anyhow::Result<()> {
        // TODO: implement writing to config file
        Ok(())
    }

    pub fn config_path() -> anyhow::Result<PathBuf> {
        // TODO: use `directories` to locate config directory
        todo!()
    }
}
