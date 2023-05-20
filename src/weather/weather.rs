use open_meteo_rs::forecast::{ForecastResult, Options};

use super::location::Locations;

pub async fn get_weather(
    client: &open_meteo_rs::Client,
    opts: open_meteo_rs::forecast::Options,
) -> ForecastResult {
    client.forecast(opts).await.unwrap_or_default()
}

pub fn set_opts(opts: &mut Options, locations: &Locations) {
    // Location
    opts.location = open_meteo_rs::Location {
        lat: locations.results[0].latitude,
        lng: locations.results[0].longitude,
    };

    // Current weather
    opts.current_weather = Some(true);

    // Temperature unit
    opts.temperature_unit = Some(open_meteo_rs::forecast::TemperatureUnit::Celsius); // or

    // Wind speed unit
    opts.wind_speed_unit = Some(open_meteo_rs::forecast::WindSpeedUnit::Kmh); // or

    // Precipitation unit
    opts.precipitation_unit = Some(open_meteo_rs::forecast::PrecipitationUnit::Millimeters); // or

    // Time zone (default to UTC)
    opts.time_zone = Some(chrono_tz::Europe::Paris.name().into());

    // Forecast days (0-16)
    opts.forecast_days = Some(3); // !! mutually exclusive with dates

    // Cell selection
    opts.cell_selection = Some(open_meteo_rs::forecast::CellSelection::Nearest); // or

    // Daily parameters
    opts.daily.push("temperature_2m_max".into());
    opts.daily.push("temperature_2m_min".into());
    opts.daily.push("weathercode".into());
}

pub fn weather_and_forecast(client: &open_meteo_rs::Client, opts: Options) -> ForecastResult {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(get_weather(client, opts))
}
