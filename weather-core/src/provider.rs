use crate::{
    Config, WeatherRequest, WeatherResponse,
    provider::{openweather::OpenWeatherProvider, weatherapi::WeatherApiProvider},
};
use async_trait::async_trait;
use std::{convert::TryFrom, fmt::Debug};

pub mod openweather;
pub mod weatherapi;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProviderId {
    OpenWeather,
    WeatherApi,
}

impl ProviderId {
    pub fn as_str(&self) -> &'static str {
        match self {
            ProviderId::OpenWeather => "openweather",
            ProviderId::WeatherApi => "weatherapi",
        }
    }

    pub const fn all() -> &'static [ProviderId] {
        &[ProviderId::OpenWeather, ProviderId::WeatherApi]
    }
}

impl std::fmt::Display for ProviderId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl TryFrom<&str> for ProviderId {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let lower = value.to_lowercase();

        match lower.as_str() {
            "openweather" => Ok(ProviderId::OpenWeather),
            "weatherapi" => Ok(ProviderId::WeatherApi),
            _ => Err(anyhow::anyhow!(
                "Unknown provider '{value}'. Supported providers: openweather, weatherapi."
            )),
        }
    }
}

#[async_trait]
pub trait WeatherProvider: Send + Sync + Debug {
    async fn get_weather(&self, request: &WeatherRequest) -> anyhow::Result<WeatherResponse>;
}

/// Construct a provider from config and explicit ProviderId.
pub fn provider_from_config(
    id: ProviderId,
    config: &Config,
) -> anyhow::Result<Box<dyn WeatherProvider>> {
    let api_key = config.provider_api_key(id).ok_or_else(|| {
        anyhow::anyhow!(
            "No API key configured for provider '{id}'.\n\
                 Hint: run `weather configure {id}` and enter your API key."
        )
    })?;

    let boxed: Box<dyn WeatherProvider> = match id {
        ProviderId::OpenWeather => Box::new(OpenWeatherProvider::new(api_key.to_owned())),
        ProviderId::WeatherApi => Box::new(WeatherApiProvider::new(api_key.to_owned())),
    };

    Ok(boxed)
}

/// Construct the default provider from config, using `default_provider` field.
pub fn default_provider_from_config(config: &Config) -> anyhow::Result<Box<dyn WeatherProvider>> {
    let id = config.default_provider_id()?;
    provider_from_config(id, config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    #[test]
    fn provider_id_as_str_roundtrip() {
        for id in ProviderId::all() {
            let s = id.as_str();
            let parsed = ProviderId::try_from(s).expect("roundtrip should succeed");
            assert_eq!(*id, parsed);
        }
    }

    #[test]
    fn unknown_provider_error() {
        let err = ProviderId::try_from("doesnotexist").unwrap_err();
        assert!(err.to_string().contains("Unknown provider"));
    }

    #[test]
    fn provider_from_config_errors_when_missing_api_key() {
        let cfg = Config::default();
        let err = provider_from_config(ProviderId::OpenWeather, &cfg).unwrap_err();
        assert!(err.to_string().contains("No API key configured for provider"));
    }

    #[test]
    fn default_provider_from_config_errors_when_not_set() {
        let cfg = Config::default();
        let err = default_provider_from_config(&cfg).unwrap_err();

        let msg = err.to_string();
        assert!(msg.contains("No default provider configured"));
        assert!(msg.contains("Hint: run `weather configure"));
    }

    #[test]
    fn default_provider_from_config_works_when_set_and_configured() {
        let mut cfg = Config::default();
        cfg.upsert_provider_api_key(ProviderId::OpenWeather, "KEY".to_string());

        let provider = default_provider_from_config(&cfg);
        assert!(provider.is_ok());
    }
}
