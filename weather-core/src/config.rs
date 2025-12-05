use anyhow::{Context, Result, anyhow};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, path::PathBuf};

use crate::provider::ProviderId;

/// Configuration for a single provider (e.g., API key).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub api_key: String,
}

/// Top-level configuration stored on disk.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    /// Optional default provider id, e.g. "openweather" or "weatherapi".
    pub default_provider: Option<String>,

    /// Example TOML:
    /// [providers.openweather]
    /// api_key = "..."
    pub providers: HashMap<String, ProviderConfig>,
}

impl Config {
    /// Return the default provider as a strongly-typed ProviderId.
    pub fn default_provider_id(&self) -> Result<ProviderId> {
        let s = self.default_provider.as_ref().ok_or_else(|| {
            anyhow::anyhow!(
                "No default provider configured.\n\
                 Hint: run `weather configure <provider>` (e.g. `weather configure openweather`) first."
            )
        })?;

        ProviderId::try_from(s.as_str())
    }

    pub fn has_provider(&self, id: ProviderId) -> bool {
        self.providers.contains_key(id.as_str())
    }

    pub fn provider_config(&self, id: ProviderId) -> Option<&ProviderConfig> {
        self.providers.get(id.as_str())
    }

    /// Store default provider as string.
    pub fn set_default_provider(&mut self, id: ProviderId) {
        self.default_provider = Some(id.as_str().to_string());
    }

    /// Load config from disk, or return an empty default if it doesn't exist yet.
    pub fn load() -> Result<Self> {
        let path = Self::config_file_path()?;
        if !path.exists() {
            // First run: no config file, return empty.
            return Ok(Self::default());
        }

        let contents = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;

        let cfg: Config = toml::from_str(&contents)
            .with_context(|| format!("Failed to parse config file: {}", path.display()))?;

        Ok(cfg)
    }

    /// Save config to disk, creating parent directories as needed.
    pub fn save(&self) -> Result<()> {
        let path = Self::config_file_path()?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("Failed to create config directory: {}", parent.display())
            })?;
        }

        let toml =
            toml::to_string_pretty(self).context("Failed to serialize configuration to TOML")?;

        fs::write(&path, toml)
            .with_context(|| format!("Failed to write config file: {}", path.display()))?;

        Ok(())
    }

    /// Path to the config file.
    pub fn config_file_path() -> Result<PathBuf> {
        let dirs = ProjectDirs::from("dev", "weather-task", "weather-cli")
            .ok_or_else(|| anyhow!("Could not determine platform config directory"))?;

        Ok(dirs.config_dir().join("config.toml"))
    }

    /// Convenience helper: set/replace a provider API key and optionally set default provider.
    pub fn upsert_provider_api_key(&mut self, provider_id: ProviderId, api_key: String) {
        self.providers.insert(provider_id.as_str().to_string(), ProviderConfig { api_key });

        if self.default_provider.is_none() {
            self.default_provider = Some(provider_id.to_string());
        }
    }

    /// Returns API key for a provider, if present.
    pub fn provider_api_key(&self, provider_id: ProviderId) -> Option<&str> {
        self.providers.get(provider_id.as_str()).map(|cfg| cfg.api_key.as_str())
    }

    pub fn is_provider_configured(&self, provider_id: ProviderId) -> bool {
        self.provider_api_key(provider_id).is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::ProviderId;

    #[test]
    fn default_provider_id_errors_when_not_set() {
        let cfg = Config::default();
        let err = cfg.default_provider_id().unwrap_err();

        assert!(err.to_string().contains("No default provider configured"));
    }

    #[test]
    fn set_api_key_and_default_for_provider() {
        let mut cfg = Config::default();

        cfg.upsert_provider_api_key(ProviderId::OpenWeather, "OPEN_KEY".into());

        let default = cfg.default_provider_id().expect("default provider must exist");
        assert_eq!(default, ProviderId::OpenWeather);

        let key = cfg.provider_api_key(ProviderId::OpenWeather);
        assert_eq!(key, Some("OPEN_KEY"));
        assert!(cfg.is_provider_configured(ProviderId::OpenWeather));
    }

    #[test]
    fn upsert_does_not_override_existing_default() {
        let mut cfg = Config::default();

        cfg.upsert_provider_api_key(ProviderId::OpenWeather, "OPEN_KEY".into());
        cfg.upsert_provider_api_key(ProviderId::WeatherApi, "WEATHER_KEY".into());

        let default = cfg.default_provider_id().expect("default provider must exist");

        assert_eq!(default, ProviderId::OpenWeather);
        assert!(cfg.is_provider_configured(ProviderId::OpenWeather));
        assert!(cfg.is_provider_configured(ProviderId::WeatherApi));
    }

    #[test]
    fn set_default_provider_overrides_default() {
        let mut cfg = Config::default();

        cfg.upsert_provider_api_key(ProviderId::OpenWeather, "OPEN_KEY".into());
        cfg.upsert_provider_api_key(ProviderId::WeatherApi, "WEATHER_KEY".into());

        let default = cfg.default_provider_id().expect("default provider must exist");
        assert_eq!(default, ProviderId::OpenWeather);

        cfg.set_default_provider(ProviderId::WeatherApi);

        let default = cfg.default_provider_id().expect("default provider must exist");
        assert_eq!(default, ProviderId::WeatherApi);
    }
}
