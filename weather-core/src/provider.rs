use crate::{WeatherRequest, WeatherResponse};
use async_trait::async_trait;

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
}

#[async_trait]
pub trait WeatherProvider: Send + Sync {
    async fn get_weather(&self, request: &WeatherRequest) -> anyhow::Result<WeatherResponse>;
}
