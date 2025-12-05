use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, NaiveDateTime, Utc};
use reqwest::Client;
use serde::Deserialize;

use crate::model::{WeatherRequest, WeatherResponse};

use super::WeatherProvider;

#[derive(Debug, Clone)]
pub struct OpenWeatherProvider {
    api_key: String,
    http: Client,
}

impl OpenWeatherProvider {
    pub fn new(api_key: String) -> Self {
        Self { api_key, http: Client::new() }
    }
}

#[derive(Debug, Deserialize)]
struct OwMain {
    temp: f64,
    feels_like: f64,
    humidity: u8,
}

#[derive(Debug, Deserialize)]
struct OwWeather {
    description: String,
}

#[derive(Debug, Deserialize)]
struct OwWind {
    speed: f64,
}

#[derive(Debug, Deserialize)]
struct OwResponse {
    name: String,
    dt: i64,
    main: OwMain,
    weather: Vec<OwWeather>,
    wind: OwWind,
}

#[async_trait]
impl WeatherProvider for OpenWeatherProvider {
    async fn get_weather(&self, request: &WeatherRequest) -> Result<WeatherResponse> {
        // For now we ignore request.when (date) and fetch current conditions.
        let url = "https://api.openweathermap.org/data/2.5/weather";

        let res = self
            .http
            .get(url)
            .query(&[
                ("q", request.address.as_str()),
                ("appid", self.api_key.as_str()),
                ("units", "metric"),
            ])
            .send()
            .await
            .context("Failed to send request to OpenWeather")?;

        let status = res.status();
        let body = res.text().await.context("Failed to read OpenWeather response body")?;

        if !status.is_success() {
            return Err(anyhow::anyhow!(
                "OpenWeather request failed with status {}: {}",
                status,
                truncate_body(&body),
            ));
        }

        let parsed: OwResponse =
            serde_json::from_str(&body).context("Failed to parse OpenWeather JSON")?;

        let observation_time = unix_to_utc(parsed.dt).unwrap_or_else(Utc::now);

        let condition = parsed
            .weather
            .first()
            .map(|w| w.description.clone())
            .unwrap_or_else(|| "Unknown".to_string());

        Ok(WeatherResponse {
            provider: "openweather".to_string(),
            location_name: parsed.name,
            temperature_c: parsed.main.temp,
            feels_like_c: parsed.main.feels_like,
            condition,
            humidity_pct: parsed.main.humidity,
            wind_speed_mps: parsed.wind.speed,
            observation_time,
        })
    }
}

fn unix_to_utc(ts: i64) -> Option<DateTime<Utc>> {
    NaiveDateTime::from_timestamp_opt(ts, 0).map(|ndt| DateTime::<Utc>::from_utc(ndt, Utc))
}

fn truncate_body(body: &str) -> String {
    const MAX: usize = 200;
    if body.len() > MAX { format!("{}...", &body[..MAX]) } else { body.to_string() }
}
