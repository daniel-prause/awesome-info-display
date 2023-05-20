
use serde::Deserialize;
use serde::Serialize;
use serde_json;
use std::fmt::Formatter;
use std::fmt::{Display, Error as FmtError};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Locations {
    pub results: Vec<Result>,
    #[serde(rename = "generationtime_ms")]
    pub generationtime_ms: f64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Result {
    pub id: i64,
    pub name: String,
    pub latitude: f64,
    pub longitude: f64,
    pub elevation: f64,
    #[serde(rename = "feature_code")]
    pub feature_code: String,
    #[serde(rename = "country_code")]
    pub country_code: String,
    #[serde(rename = "admin1_id")]
    pub admin1_id: i64,
    #[serde(rename = "admin2_id")]
    pub admin2_id: i64,
    #[serde(rename = "admin3_id")]
    pub admin3_id: i64,
    #[serde(rename = "admin4_id")]
    pub admin4_id: i64,
    pub timezone: String,
    pub population: i64,
    #[serde(rename = "country_id")]
    pub country_id: i64,
    pub country: String,
    pub admin1: String,
    pub admin2: String,
    pub admin3: String,
    pub admin4: String,
}
#[derive(Debug, Clone, Deserialize)]
pub enum ExtractCodeError {
    RequestFailed(String),
    JSONerror(String),
}

impl From<reqwest::Error> for ExtractCodeError {
    fn from(e: reqwest::Error) -> Self {
        Self::RequestFailed(e.to_string())
    }
}
impl From<serde_json::Error> for ExtractCodeError {
    fn from(e: serde_json::Error) -> Self {
        Self::JSONerror(e.to_string())
    }
}

impl Display for ExtractCodeError {
    fn fmt(&self, f: &mut Formatter) -> core::result::Result<(), FmtError> {
        match self {
            Self::RequestFailed(e) => write!(f, "request failed: {}", e),
            Self::JSONerror(e) => write!(f, "could not decode json {}", e),
        }
    }
}
pub fn get_location(city: String) -> core::result::Result<Locations, ExtractCodeError> {
    let locations: Locations = serde_json::from_str(
        reqwest::blocking::get(format!(
        "https://geocoding-api.open-meteo.com/v1/search?name={}&count=1&language=en&format=json",
        city
    ))?
        .text()?
        .as_str(),
    )?;
    Ok(locations)
}
