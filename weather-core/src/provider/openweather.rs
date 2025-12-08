use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, NaiveDateTime, Utc};
use reqwest::Client;
use serde::Deserialize;

use crate::{model::{WeatherRequest, WeatherResponse}, provider::{DateRequest, classify_date}};

use super::WeatherProvider;

#[derive(Debug, Clone)]
pub struct OpenWeatherProvider {
    api_key: String,
    http: Client,
}

impl OpenWeatherProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            http: Client::new(),
        }
    }

    async fn fetch_current(&self, address: &str) -> Result<WeatherResponse> {
        let url = "https://api.openweathermap.org/data/2.5/weather";

        let res = self
            .http
            .get(url)
            .query(&[
                ("q", address),
                ("appid", self.api_key.as_str()),
                ("units", "metric"),
            ])
            .send()
            .await
            .context("Failed to send request to OpenWeather (current weather)")?;

        let status = res.status();
        let body = res
            .text()
            .await
            .context("Failed to read OpenWeather current response body")?;

        if !status.is_success() {
            return Err(anyhow!(
                "OpenWeather current request failed with status {}: {}",
                status,
                truncate_body(&body),
            ));
        }

        let parsed: OwCurrentResponse =
            serde_json::from_str(&body).context("Failed to parse OpenWeather current JSON")?;

        let observation_time = unix_to_utc(parsed.dt).unwrap_or_else(Utc::now);

        let condition = parsed
            .weather.first()
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

    async fn fetch_forecast(&self, address: &str, when: DateTime<Utc>) -> Result<WeatherResponse> {
        let url = "https://api.openweathermap.org/data/2.5/forecast";

        let res = self
            .http
            .get(url)
            .query(&[
                ("q", address),
                ("appid", self.api_key.as_str()),
                ("units", "metric"),
            ])
            .send()
            .await
            .context("Failed to send request to OpenWeather (5-day forecast)")?;

        let status = res.status();
        let body = res
            .text()
            .await
            .context("Failed to read OpenWeather forecast response body")?;

        if !status.is_success() {
            return Err(anyhow!(
                "OpenWeather forecast request failed with status {}: {}",
                status,
                truncate_body(&body),
            ));
        }

        let parsed: OwForecastResponse =
            serde_json::from_str(&body).context("Failed to parse OpenWeather forecast JSON")?;

        let target_ts = when.timestamp();

        let entry = parsed
            .list
            .iter()
            .min_by_key(|e| (e.dt - target_ts).abs())
            .ok_or_else(|| anyhow!("OpenWeather forecast response contained no data"))?;

        let observation_time = unix_to_utc(entry.dt).unwrap_or_else(Utc::now);

        let condition = entry
            .weather.first()
            .map(|w| w.description.clone())
            .unwrap_or_else(|| "Unknown".to_string());

        let location_name = format!("{}, {}", parsed.city.name, parsed.city.country);

        Ok(WeatherResponse {
            provider: "openweather".to_string(),
            location_name,
            temperature_c: entry.main.temp,
            feels_like_c: entry.main.feels_like,
            condition,
            humidity_pct: entry.main.humidity,
            wind_speed_mps: entry.wind.speed,
            observation_time,
        })
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
struct OwCurrentResponse {
    name: String,
    dt: i64,
    main: OwMain,
    weather: Vec<OwWeather>,
    wind: OwWind,
}

#[derive(Debug, Deserialize)]
struct OwCity {
    name: String,
    country: String,
}

#[derive(Debug, Deserialize)]
struct OwForecastEntry {
    dt: i64,
    main: OwMain,
    weather: Vec<OwWeather>,
    wind: OwWind,
}

#[derive(Debug, Deserialize)]
struct OwForecastResponse {
    city: OwCity,
    list: Vec<OwForecastEntry>,
}

#[async_trait]
impl WeatherProvider for OpenWeatherProvider {
    async fn get_weather(&self, request: &WeatherRequest) -> Result<WeatherResponse> {
        let now = Utc::now();
        let date_req = classify_date(now, request.when);

        match date_req {
            DateRequest::Current => {
                self.fetch_current(&request.address).await
            }
            DateRequest::Past(dt) => {
                Err(anyhow!(
                    "Historical weather ({}) is not supported by free OpenWeather API.\n\
                     Only current weather and up to 5 days forecast are available.",
                    dt
                ))
            }
            DateRequest::Future(dt) => {
                let max_forecast = now + chrono::Duration::days(5);
                if dt > max_forecast {
                    Err(anyhow!(
                        "Requested date {} exceeds the 5-day forecast limit of free OpenWeather API.\n\
                         Allowed range: now .. {}.",
                        dt,
                        max_forecast
                    ))
                } else {
                    self.fetch_forecast(&request.address, dt).await
                }
            }
        }
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