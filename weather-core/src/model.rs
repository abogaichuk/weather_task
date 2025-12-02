use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct WeatherRequest {
    pub address: String,
    pub when: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherResponse {
    pub provider: String,
    pub location_name: String,
    pub temperature_c: f64,
    pub feels_like_c: f64,
    pub condition: String,
    pub humidity_pct: u8,
    pub wind_speed_mps: f64,
    pub observation_time: DateTime<Utc>,
}
