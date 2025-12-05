use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, NaiveDateTime, Utc};
use reqwest::Client;
use serde::Deserialize;

use crate::model::{WeatherRequest, WeatherResponse};

use super::WeatherProvider;

#[derive(Debug, Clone)]
pub struct WeatherApiProvider {
    api_key: String,
    http: Client,
}

impl WeatherApiProvider {
    pub fn new(api_key: String) -> Self {
        Self { api_key, http: Client::new() }
    }
}

#[derive(Debug, Deserialize)]
struct WaLocation {
    name: String,
    country: String,
    localtime_epoch: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct WaCondition {
    text: String,
}

#[derive(Debug, Deserialize)]
struct WaCurrent {
    temp_c: f64,
    feelslike_c: f64,
    humidity: u8,
    wind_kph: f64,
    condition: WaCondition,
    last_updated_epoch: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct WaResponse {
    location: WaLocation,
    current: WaCurrent,
}

#[async_trait]
impl WeatherProvider for WeatherApiProvider {
    async fn get_weather(&self, request: &WeatherRequest) -> Result<WeatherResponse> {
        // For now we ignore request.when and always fetch current conditions.
        let url = "http://api.weatherapi.com/v1/current.json";

        let res = self
            .http
            .get(url)
            .query(&[("key", self.api_key.as_str()), ("q", request.address.as_str())])
            .send()
            .await
            .context("Failed to send request to WeatherAPI.com")?;

        let status = res.status();
        let body = res.text().await.context("Failed to read WeatherAPI response body")?;

        if !status.is_success() {
            return Err(anyhow::anyhow!(
                "WeatherAPI request failed with status {}: {}",
                status,
                truncate_body(&body),
            ));
        }

        let parsed: WaResponse =
            serde_json::from_str(&body).context("Failed to parse WeatherAPI JSON")?;

        // Prefer last_updated_epoch, fall back to localtime_epoch, then now.
        let ts = parsed.current.last_updated_epoch.or(parsed.location.localtime_epoch);
        let observation_time = ts.and_then(unix_to_utc).unwrap_or_else(Utc::now);

        let location_name = format!("{}, {}", parsed.location.name, parsed.location.country);
        let wind_speed_mps = parsed.current.wind_kph / 3.6;

        Ok(WeatherResponse {
            provider: "weatherapi".to_string(),
            location_name,
            temperature_c: parsed.current.temp_c,
            feels_like_c: parsed.current.feelslike_c,
            condition: parsed.current.condition.text,
            humidity_pct: parsed.current.humidity,
            wind_speed_mps,
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
