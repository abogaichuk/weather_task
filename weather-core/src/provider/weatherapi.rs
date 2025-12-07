use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, NaiveDateTime, Utc, Timelike};
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

#[derive(Debug, Deserialize)]
struct WaForecastHour {
    time_epoch: i64,
    temp_c: f64,
    feelslike_c: f64,
    humidity: u8,
    wind_kph: f64,
    condition: WaCondition,
}

#[derive(Debug, Deserialize)]
struct WaForecastDay {
    hour: Vec<WaForecastHour>,
}

#[derive(Debug, Deserialize)]
struct WaForecast {
    forecastday: Vec<WaForecastDay>,
}

#[derive(Debug, Deserialize)]
struct WaForecastResponse {
    location: WaLocation,
    forecast: WaForecast,
}

#[async_trait]
impl WeatherProvider for WeatherApiProvider {
    async fn get_weather(&self, request: &WeatherRequest) -> Result<WeatherResponse> {
        let now = Utc::now();
        let target = request.when.unwrap_or(now);
        let today = now.date_naive();
        let target_date = target.date_naive();

        if request.when.is_none() || target_date == today {
            // ==== CURRENT WEATHER ====
            let url = "http://api.weatherapi.com/v1/current.json";

            let res = self
                .http
                .get(url)
                .query(&[
                    ("key", self.api_key.as_str()),
                    ("q", request.address.as_str()),
                ])
                .send()
                .await
                .context("Failed to send request to WeatherAPI.com (current)")?;

            let status = res.status();
            let body = res
                .text()
                .await
                .context("Failed to read WeatherAPI current response body")?;

            if !status.is_success() {
                return Err(anyhow::anyhow!(
                    "WeatherAPI current request failed with status {}: {}",
                    status,
                    truncate_body(&body),
                ));
            }

            let parsed: WaResponse =
                serde_json::from_str(&body).context("Failed to parse WeatherAPI current JSON")?;

            let ts = parsed
                .current
                .last_updated_epoch
                .or(parsed.location.localtime_epoch);
            let observation_time = ts
                .and_then(unix_to_utc)
                .unwrap_or_else(Utc::now);

            // let location_name = format!("{}, {}", parsed.location.name);
            let wind_speed_mps = parsed.current.wind_kph / 3.6;

            return Ok(WeatherResponse {
                provider: "weatherapi".to_string(),
                location_name: parsed.location.name,
                temperature_c: parsed.current.temp_c,
                feels_like_c: parsed.current.feelslike_c,
                condition: parsed.current.condition.text,
                humidity_pct: parsed.current.humidity,
                wind_speed_mps,
                observation_time,
            });
        }

        // ==== FORECAST / HISTORY ====
        // If target_date > today => forecast, else => history.
        let base_url = if target_date > today {
            "http://api.weatherapi.com/v1/forecast.json"
        } else {
            "http://api.weatherapi.com/v1/history.json"
        };

        let unixdt = target.timestamp();
        let hour = target.hour(); // 0–23

        let res = self
            .http
            .get(base_url)
            .query(&[
                ("key", self.api_key.as_str()),
                ("q", request.address.as_str()),
                // Restrict date/time to a specific hour via unixdt/hour, per docs.
                ("unixdt", &unixdt.to_string()),
                ("hour", &hour.to_string()),
            ])
            .send()
            .await
            .with_context(|| {
                format!(
                    "Failed to send request to WeatherAPI.com ({})",
                    if target_date > today { "forecast" } else { "history" }
                )
            })?;

        let status = res.status();
        let body = res
            .text()
            .await
            .context("Failed to read WeatherAPI forecast/history response body")?;

        if !status.is_success() {
            return Err(anyhow::anyhow!(
                "WeatherAPI {} request failed with status {}: {}",
                if target_date > today { "forecast" } else { "history" },
                status,
                truncate_body(&body),
            ));
        }

        let parsed: WaForecastResponse = serde_json::from_str(&body).with_context(|| {
            format!(
                "Failed to parse WeatherAPI {} JSON",
                if target_date > today { "forecast" } else { "history" }
            )
        })?;

        // let location_name = format!("{}, {}", parsed.location.name, parsed.location.country);

        let day = parsed
            .forecast
            .forecastday.first()
            .ok_or_else(|| anyhow::anyhow!("WeatherAPI response contained no forecastday data"))?;

        let hour_entry = day
            .hour
            .iter()
            .min_by_key(|h| (h.time_epoch - unixdt).abs())
            .ok_or_else(|| anyhow::anyhow!("WeatherAPI response contained no hourly data"))?;

        let observation_time = unix_to_utc(hour_entry.time_epoch).unwrap_or_else(Utc::now);
        let wind_speed_mps = hour_entry.wind_kph / 3.6;

        Ok(WeatherResponse {
            provider: "weatherapi".to_string(),
            location_name: parsed.location.name,
            temperature_c: hour_entry.temp_c,
            feels_like_c: hour_entry.feelslike_c,
            condition: hour_entry.condition.text.clone(),
            humidity_pct: hour_entry.humidity,
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
    if body.len() > MAX {
        format!("{}...", &body[..MAX])
    } else {
        body.to_string()
    }
}

