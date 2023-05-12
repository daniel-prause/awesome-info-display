use crate::config_manager::ConfigManager;
use crate::screens::BasicScreen;
use crate::screens::Screen;
use crate::screens::Screenable;
use crate::weather::*;
use crate::LAST_BME_INFO;
use chrono::Datelike;
use crossbeam_channel::bounded;
use crossbeam_channel::{Receiver, Sender};
use image::{ImageBuffer, Rgb, RgbImage};
use imageproc::drawing::draw_text_mut;
use rusttype::Font;
use rusttype::Scale;
use std::rc::Rc;
use std::sync::{atomic::AtomicBool, atomic::Ordering, Arc, RwLock};
use std::thread;
use std::time::Duration;
use std::time::Instant;

pub struct WeatherScreen {
    screen: Screen,
    symbols: Rc<Font<'static>>,
    receiver: Receiver<WeatherInfo>,
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
}

impl Screenable for WeatherScreen {
    fn get_screen(&mut self) -> &mut Screen {
        &mut self.screen
    }
}

impl BasicScreen for WeatherScreen {
    fn update(&mut self) {
        let weather_info = self.receiver.try_recv();
        match weather_info {
            Ok(weather_info) => {
                self.draw_screen(&weather_info);
                self.draw_companion_screen(&weather_info);
            }
            Err(_) => {}
        }
    }
}

impl WeatherScreen {
    fn draw_companion_screen(&mut self, weather_info: &WeatherInfo) {
        // draw initial image
        let mut image = RgbImage::new(320, 170);

        // days
        let mut x: i32 = 24;
        for forecast in &weather_info.weather_forecast {
            draw_text_mut(
                &mut image,
                Rgb([255u8, 255u8, 255u8]),
                x,
                6,
                Scale { x: 38.0, y: 38.0 },
                &self.screen.font,
                forecast.day.as_str(),
            );
            x += 103;
        }

        // temperatures
        let mut x: i32 = 24;
        for forecast in &weather_info.weather_forecast {
            // min
            draw_text_mut(
                &mut image,
                Rgb([255u8, 255u8, 255u8]),
                x,
                40,
                Scale { x: 22.0, y: 22.0 },
                &self.screen.font,
                format!("{: >2} \u{00B0}C", forecast.min.round() as i64,).as_str(),
            );
            // max
            draw_text_mut(
                &mut image,
                Rgb([255u8, 255u8, 255u8]),
                x,
                60,
                Scale { x: 22.0, y: 22.0 },
                &self.screen.font,
                format!("{: >2} \u{00B0}C", forecast.max.round() as i64,).as_str(),
            );
            x += 103;
        }

        self.screen.companion_screen_bytes = image.into_vec();
    }

    fn draw_screen(&mut self, weather_info: &WeatherInfo) {
        // draw initial image
        let mut image = RgbImage::new(256, 64);
        self.draw_weather_info(weather_info, &mut image);
        self.screen.main_screen_bytes = image.into_vec();
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
            Scale { x: 40.0, y: 40.0 },
            self.symbols.as_ref(),
            WeatherScreen::get_weather_icon(weather_info.weather_icon, weather_info.is_day)
                .as_str(),
        );

        // temperature
        draw_text_mut(
            image,
            Rgb([255u8, 255u8, 255u8]),
            72,
            6,
            Scale { x: 32.0, y: 32.0 },
            &self.screen.font,
            format!("{}\u{00B0}C", (weather_info.temperature.round() as i64)).as_str(),
        );

        // city
        draw_text_mut(
            image,
            Rgb([255u8, 255u8, 255u8]),
            4,
            50,
            Scale { x: 14.0, y: 14.0 },
            &self.screen.font,
            weather_info.city.as_str(),
        );

        // wind symbol
        draw_text_mut(
            image,
            Rgb([255u8, 255u8, 255u8]),
            160,
            10,
            Scale { x: 14.0, y: 14.0 },
            &self.symbols.as_ref(),
            format!("\u{f72e}").as_str(),
        );
        // wind speed
        draw_text_mut(
            image,
            Rgb([255u8, 255u8, 255u8]),
            178,
            10,
            Scale { x: 14.0, y: 14.0 },
            &self.screen.font,
            format!("{} km/h", ((weather_info.wind) * 3.6).round()).as_str(),
        );

        // wind direction
        draw_text_mut(
            image,
            Rgb([255u8, 255u8, 255u8]),
            178,
            24,
            Scale { x: 14.0, y: 14.0 },
            &self.screen.font,
            format!("{}", weather_info.wind_direction).as_str(),
        );

        // indoor temperature / indoor humidity
        let (temperature, humidity) = LAST_BME_INFO.lock().unwrap().clone();
        draw_text_mut(
            image,
            Rgb([255u8, 255u8, 255u8]),
            72,
            38,
            Scale { x: 14.0, y: 14.0 },
            &self.screen.font,
            format!("{}°C / {}%", temperature, humidity).as_str(),
        );
    }

    fn get_weather_icon(code: u8, is_day: u8) -> String {
        if is_day == 0 {
            // night
            match code {
                0 => String::from("\u{f186}"),
                1 | 2 | 3 => String::from("\u{f6c3}"),
                45 | 48 => String::from("\u{f75f}"),
                51 | 53 | 55 | 56 | 57 | 61 | 63 | 65 | 66 | 67 | 80 | 81 | 82 => {
                    String::from("\u{f73c}")
                }
                71 | 73 | 75 | 77 | 85 | 86 => String::from("\u{f2dc}"),
                95 | 96 | 99 => String::from("\u{f0e7}"),
                _ => String::from(""),
            }
        } else {
            // day
            match code {
                0 => String::from("\u{f185}"),
                1 | 2 | 3 => String::from("\u{f6c4}"),
                45 | 48 => String::from("\u{f75f}"),
                51 | 53 | 55 | 56 | 57 | 61 | 63 | 65 | 66 | 67 | 80 | 81 | 82 => {
                    String::from("\u{f73d}")
                }
                71 | 73 | 75 | 77 | 85 | 86 => String::from("\u{f2dc}"),
                95 | 96 | 99 => String::from("\u{f0e7}"),
                _ => String::from(""),
            }
        }
    }
    pub fn new(
        description: String,
        key: String,
        font: Rc<Font<'static>>,
        symbols: Rc<Font<'static>>,
        config_manager: Arc<RwLock<ConfigManager>>,
    ) -> WeatherScreen {
        let (tx, rx): (Sender<WeatherInfo>, Receiver<WeatherInfo>) = bounded(1);
        let active = Arc::new(AtomicBool::new(false));
        let mut this = WeatherScreen {
            screen: Screen {
                description,
                key,
                font,
                config_manager: config_manager.clone(),
                active: active.clone(),
                handle: Some(thread::spawn(move || {
                    let sender = tx.to_owned();
                    let deg_to_dir = |deg: f64| {
                        let val = ((deg / 22.5) + 0.5).floor();
                        let arr = [
                            "N", "NNE", "NE", "ENE", "E", "ESE", "SE", "SSE", "S", "SSW", "SW",
                            "WSW", "W", "WNW", "NW", "NNW",
                        ];
                        return arr[(val as usize) % 16];
                    };
                    let mut last_weather_info: WeatherInfo = Default::default();
                    let mut last_update = Instant::now() - Duration::from_secs(61);
                    let client = open_meteo_rs::Client::new();
                    loop {
                        while !active.load(Ordering::Acquire) {
                            thread::park();
                        }
                        let location = config_manager
                            .read()
                            .unwrap()
                            .config
                            .weather_location
                            .clone();
                        // TODO: make this configurable for language and metric/non-metric units
                        // get current weather for location
                        if last_update.elapsed().as_secs() > 60 {
                            last_update = Instant::now();
                            // get locations first
                            //let locations = weather::location::get_location(location.into());
                            let locations = location::get_location(location.into());
                            match locations {
                                Ok(locations) => {
                                    let mut opts = open_meteo_rs::forecast::Options::default();
                                    weather::set_opts(&mut opts, &locations);
                                    let closest_location = locations.results[0].clone();
                                    let result = weather::weather_and_forecast(&client, opts);

                                    match result.current_weather {
                                        Some(current) => {
                                            let mut weather_info: WeatherInfo = Default::default();

                                            weather_info.weather_icon =
                                                current.weathercode.unwrap_or_default() as u8;

                                            weather_info.temperature =
                                                current.temperature.unwrap_or_default();
                                            weather_info.wind =
                                                current.windspeed.unwrap_or_default();
                                            weather_info.wind_direction =
                                                deg_to_dir(current.winddirection.unwrap())
                                                    .to_string();
                                            weather_info.city = format!(
                                                "{},{}",
                                                closest_location.name,
                                                closest_location.country_code
                                            );
                                            // forecast
                                            for weather in result.daily.unwrap().iter() {
                                                weather_info.weather_forecast.push(
                                                    WeatherForecast {
                                                        day: weather.date.weekday().to_string(),
                                                        min: weather
                                                            .values
                                                            .get("temperature_2m_min")
                                                            .unwrap()
                                                            .value
                                                            .as_f64()
                                                            .unwrap_or_default(),
                                                        max: weather
                                                            .values
                                                            .get("temperature_2m_max")
                                                            .unwrap()
                                                            .value
                                                            .as_f64()
                                                            .unwrap_or_default(),
                                                    },
                                                );
                                            }
                                            last_weather_info = weather_info;
                                        }
                                        None => eprintln!("Could not fetch weather"),
                                    }
                                }
                                Err(e) => {
                                    eprintln!("Could not fetch weather! Reason: {:?}", e)
                                }
                            }
                        }
                        sender
                            .try_send(last_weather_info.clone())
                            .unwrap_or_default();
                        // TODO: think about whether we want to solve this like in bitpanda screen with last_update...
                        thread::sleep(Duration::from_millis(1000));
                    }
                })),
                ..Default::default()
            },
            symbols: Rc::clone(&symbols),
            receiver: rx,
        };

        this.draw_screen(&Default::default());
        this
    }
}
