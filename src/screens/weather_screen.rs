use crate::config_manager::ConfigManager;
use crate::screens::BasicScreen;
use crate::screens::Screen;
use crate::screens::Screenable;
use crate::weather::weather::get_weather;
use crate::weather::*;
use crate::ESP32;
use crate::LAST_BME_INFO;
use crate::TEENSY;
use ab_glyph::FontArc;
use ab_glyph::PxScale;
use chrono::Datelike;
use crossbeam_channel::bounded;
use crossbeam_channel::{Receiver, Sender};
use image::{ImageBuffer, Rgb, RgbImage};
use imageproc::drawing::draw_text_mut;

use std::sync::{atomic::AtomicBool, atomic::Ordering, Arc, RwLock};
use std::thread;
use std::time::Duration;
use std::time::Instant;

pub struct WeatherScreen {
    screen: Screen,
    symbols: FontArc,
    receiver: Receiver<Arc<WeatherInfo>>,
}

#[derive(Default, Clone)]
struct WeatherInfo {
    weather_icon: u8,
    is_day: u8,
    city: String,
    temperature: f64,
    wind: f64,
    wind_direction: String,
    weather_forecast: Vec<WeatherForecast>,
}

#[derive(Default, Clone)]
struct WeatherForecast {
    day: String,
    min: f64,
    max: f64,
    weather_icon: u8,
}

impl Screenable for WeatherScreen {
    fn get_screen(&mut self) -> &mut Screen {
        &mut self.screen
    }
}

impl BasicScreen for WeatherScreen {
    fn update(&mut self) {
        if let Ok(weather_info_arc) = self.receiver.try_recv() {
            let weather_info: &WeatherInfo = &weather_info_arc;
            self.draw_screen(weather_info);
            self.draw_companion_screen(weather_info);
        }
    }
}

impl WeatherScreen {
    fn draw_companion_screen(&mut self, weather_info: &WeatherInfo) {
        // draw initial image
        let mut image = RgbImage::new(320, 170);

        let mut x: i32 = 24;
        for forecast in &weather_info.weather_forecast {
            // day
            draw_text_mut(
                &mut image,
                Rgb([255u8, 255u8, 255u8]),
                x,
                6,
                PxScale { x: 38.0, y: 38.0 },
                &self.screen.font,
                forecast.day.as_str(),
            );

            // icon
            draw_text_mut(
                &mut image,
                Rgb([255u8, 255u8, 255u8]),
                x - 8,
                40,
                PxScale { x: 32.0, y: 32.0 },
                &self.symbols,
                format!(
                    "{: >3}",
                    WeatherScreen::get_weather_icon(forecast.weather_icon, 1)
                )
                .as_str(),
            );

            // min
            draw_text_mut(
                &mut image,
                Rgb([255u8, 255u8, 255u8]),
                x,
                80,
                PxScale { x: 22.0, y: 22.0 },
                &self.screen.font,
                format!("{: >2} \u{00B0}C", forecast.min.round() as i64).as_str(),
            );

            // max
            draw_text_mut(
                &mut image,
                Rgb([255u8, 255u8, 255u8]),
                x,
                100,
                PxScale { x: 22.0, y: 22.0 },
                &self.screen.font,
                format!("{: >2} \u{00B0}C", forecast.max.round() as i64).as_str(),
            );

            x += 103;
        }

        *self.screen.device_screen_bytes.get_mut(ESP32).unwrap() = image.into_vec();
    }

    fn draw_screen(&mut self, weather_info: &WeatherInfo) {
        // draw initial image
        let mut image = RgbImage::new(256, 64);
        self.draw_weather_info(weather_info, &mut image);
        *self.screen.device_screen_bytes.get_mut(TEENSY).unwrap() = image.into_vec();
    }
    fn draw_weather_info(
        &mut self,
        weather_info: &WeatherInfo,
        image: &mut ImageBuffer<Rgb<u8>, Vec<u8>>,
    ) {
        // icon
        draw_text_mut(
            image,
            Rgb([255u8, 255u8, 255u8]),
            6,
            6,
            PxScale { x: 40.0, y: 40.0 },
            &self.symbols,
            WeatherScreen::get_weather_icon(weather_info.weather_icon, weather_info.is_day),
        );

        // temperature
        draw_text_mut(
            image,
            Rgb([255u8, 255u8, 255u8]),
            72,
            6,
            PxScale { x: 32.0, y: 32.0 },
            &self.screen.font,
            format!("{}\u{00B0}C", (weather_info.temperature.round() as i64)).as_str(),
        );

        // city
        draw_text_mut(
            image,
            Rgb([255u8, 255u8, 255u8]),
            4,
            50,
            PxScale { x: 14.0, y: 14.0 },
            &self.screen.font,
            weather_info.city.as_str(),
        );

        // wind symbol
        draw_text_mut(
            image,
            Rgb([255u8, 255u8, 255u8]),
            160,
            10,
            PxScale { x: 14.0, y: 14.0 },
            &self.symbols,
            "\u{f72e}",
        );
        // wind speed
        draw_text_mut(
            image,
            Rgb([255u8, 255u8, 255u8]),
            178,
            10,
            PxScale { x: 14.0, y: 14.0 },
            &self.screen.font,
            format!("{} km/h", ((weather_info.wind) * 3.6).round()).as_str(),
        );

        // wind direction
        draw_text_mut(
            image,
            Rgb([255u8, 255u8, 255u8]),
            178,
            24,
            PxScale { x: 14.0, y: 14.0 },
            &self.screen.font,
            weather_info.wind_direction.as_str(),
        );

        // indoor temperature / indoor humidity
        let (temperature, humidity) = LAST_BME_INFO.lock().unwrap().clone();
        draw_text_mut(
            image,
            Rgb([255u8, 255u8, 255u8]),
            72,
            38,
            PxScale { x: 14.0, y: 14.0 },
            &self.screen.font,
            format!("{}°C / {}%", temperature, humidity).as_str(),
        );
    }

    fn get_weather_icon(code: u8, is_day: u8) -> &'static str {
        match (is_day, code) {
            // 🌙 Night
            (0, 0) => "\u{f186}",
            (0, 1 | 2 | 3) => "\u{f6c3}",
            (0, 45 | 48) => "\u{f75f}",
            (0, 51 | 53 | 55 | 56 | 57 | 61 | 63 | 65 | 66 | 67 | 80 | 81 | 82) => "\u{f73c}",
            (0, 71 | 73 | 75 | 77 | 85 | 86) => "\u{f2dc}",
            (0, 95 | 96 | 99) => "\u{f0e7}",

            // ☀️ Day
            (_, 0) => "\u{f185}",
            (_, 1 | 2 | 3) => "\u{f6c4}",
            (_, 45 | 48) => "\u{f75f}",
            (_, 51 | 53 | 55 | 56 | 57 | 61 | 63 | 65 | 66 | 67 | 80 | 81 | 82) => "\u{f73d}",
            (_, 71 | 73 | 75 | 77 | 85 | 86) => "\u{f2dc}",
            (_, 95 | 96 | 99) => "\u{f0e7}",

            _ => "",
        }
    }
    pub fn new(
        description: String,
        key: String,
        font: FontArc,
        symbols: FontArc,
        config_manager: Arc<RwLock<ConfigManager>>,
    ) -> WeatherScreen {
        let (tx, rx): (Sender<Arc<WeatherInfo>>, Receiver<Arc<WeatherInfo>>) = bounded(1);
        let active = Arc::new(AtomicBool::new(false));
        let mut this = WeatherScreen {
            screen: Screen {
                description,
                key: key.clone(),
                font,
                config_manager: config_manager.clone(),
                active: active.clone(),
                handle: Some(thread::spawn(move || {
                    let runtime = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .expect("Failed to create Tokio runtime");

                    let sender = tx;
                    let client = open_meteo_rs::Client::new();

                    let deg_to_dir = |deg: f64| {
                        const DIRS: [&str; 16] = [
                            "N", "NNE", "NE", "ENE", "E", "ESE", "SE", "SSE", "S", "SSW", "SW",
                            "WSW", "W", "WNW", "NW", "NNW",
                        ];
                        let val = ((deg / 22.5) + 0.5).floor() as usize;
                        DIRS[val % 16]
                    };

                    let mut last_weather_info = Arc::new(WeatherInfo::default());
                    let mut last_update = Instant::now() - Duration::from_secs(61);

                    loop {
                        while !active.load(Ordering::Acquire) {
                            thread::park();
                        }

                        if last_update.elapsed().as_secs() > 60 {
                            last_update = Instant::now();

                            let location = {
                                let guard = config_manager.read().unwrap();
                                guard
                                    .get_value(&key, "weather_location")
                                    .and_then(|v| match v {
                                        exchange_format::ConfigParam::String(s) => Some(s.clone()),
                                        _ => None,
                                    })
                                    .unwrap_or_default()
                            };

                            if let Ok(locations) = location::get_location(location) {
                                if let Some(closest_location) = locations.results.first() {
                                    let mut opts = open_meteo_rs::forecast::Options::default();
                                    weather::set_opts(&mut opts, &locations);

                                    let res = runtime.block_on(get_weather(&client, opts));

                                    if let Some(current) = res.current {
                                        let mut weather_info = WeatherInfo::default();

                                        weather_info.is_day = current
                                            .values
                                            .get("is_day")
                                            .and_then(|v| v.value.as_u64())
                                            .unwrap_or_default()
                                            as u8;

                                        weather_info.weather_icon = current
                                            .values
                                            .get("weather_code")
                                            .and_then(|v| v.value.as_u64())
                                            .unwrap_or_default()
                                            as u8;

                                        weather_info.temperature = current
                                            .values
                                            .get("temperature_2m")
                                            .and_then(|v| v.value.as_f64())
                                            .unwrap_or_default();

                                        weather_info.wind = current
                                            .values
                                            .get("wind_speed_10m")
                                            .and_then(|v| v.value.as_f64())
                                            .unwrap_or_default();

                                        weather_info.wind_direction = current
                                            .values
                                            .get("wind_direction_10m")
                                            .and_then(|v| v.value.as_f64())
                                            .map(deg_to_dir)
                                            .unwrap_or("N")
                                            .to_string();

                                        weather_info.city = format!(
                                            "{},{}",
                                            closest_location.name, closest_location.country_code
                                        );

                                        if let Some(daily) = res.daily {
                                            for weather in daily.iter() {
                                                weather_info.weather_forecast.push(
                                                    WeatherForecast {
                                                        day: weather.date.weekday().to_string(),
                                                        min: weather
                                                            .values
                                                            .get("temperature_2m_min")
                                                            .and_then(|v| v.value.as_f64())
                                                            .unwrap_or_default(),
                                                        max: weather
                                                            .values
                                                            .get("temperature_2m_max")
                                                            .and_then(|v| v.value.as_f64())
                                                            .unwrap_or_default(),
                                                        weather_icon: weather
                                                            .values
                                                            .get("weathercode")
                                                            .and_then(|v| v.value.as_u64())
                                                            .unwrap_or_default()
                                                            as u8,
                                                    },
                                                );
                                            }
                                        }

                                        last_weather_info = Arc::new(weather_info);
                                    }
                                }
                            }
                        }

                        let _ = sender.try_send(last_weather_info.clone());
                        thread::park_timeout(Duration::from_secs(1));
                    }
                })),
                ..Default::default()
            },
            symbols: FontArc::clone(&symbols),
            receiver: rx,
        };

        this.draw_screen(&Default::default());
        this
    }
}
