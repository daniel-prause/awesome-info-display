extern crate openweathermap;
use crate::config_manager::ConfigManager;
use crate::screen::BasicScreen;
use crate::screen::Screen;
use crate::screen::ScreenControl;
use image::{ImageBuffer, Rgb, RgbImage};
use imageproc::drawing::{draw_filled_rect_mut, draw_hollow_rect_mut, draw_text_mut};
use imageproc::rect::Rect;
use openweathermap::blocking::weather;
use rusttype::Font;
use rusttype::Scale;
use std::fmt::Debug;
use std::sync::{atomic::Ordering, Arc, Mutex, RwLock};
use std::thread;
use std::time::Duration;
use systemstat::{saturating_sub_bytes, Platform, System};

#[derive(Debug, Clone)]
pub struct WeatherScreen {
    screen: Screen,
    weather_icon: Arc<Mutex<String>>,
    city: Arc<Mutex<String>>,
    temperature: Arc<Mutex<f64>>,
    symbols: Option<Font<'static>>,
}

impl BasicScreen for std::sync::Arc<RwLock<WeatherScreen>> {
    fn description(&self) -> String {
        self.read().unwrap().screen.description.clone()
    }

    fn current_image(&self) -> Vec<u8> {
        self.read().unwrap().screen.current_image()
    }

    fn update(&mut self) {
        WeatherScreen::update(self.clone());
    }

    fn start(&self) {
        self.read().unwrap().screen.start_worker();
    }

    fn stop(&self) {
        self.read().unwrap().screen.stop_worker();
    }

    fn initial_update_called(&mut self) -> bool {
        self.write().unwrap().screen.initial_update_called()
    }

    fn enabled(&self) -> bool {
        self.read()
            .unwrap()
            .screen
            .config_manager
            .read()
            .unwrap()
            .config
            .weather_screen_active
    }

    fn set_status(&self, status: bool) {
        self.read()
            .unwrap()
            .screen
            .config_manager
            .write()
            .unwrap()
            .config
            .weather_screen_active = status;
    }
}

impl WeatherScreen {
    pub fn draw_weather_info(&mut self, image: &mut ImageBuffer<Rgb<u8>, Vec<u8>>, scale: Scale) {
        // icon
        draw_text_mut(
            image,
            Rgb([255u8, 255u8, 255u8]),
            4,
            0,
            Scale { x: 48.0, y: 48.0 },
            self.symbols.as_ref().unwrap(),
            self.weather_icon.lock().unwrap().as_str(),
        );

        // temperature
        draw_text_mut(
            image,
            Rgb([255u8, 255u8, 255u8]),
            72,
            16,
            Scale { x: 32.0, y: 32.0 },
            *&self.screen.font.lock().unwrap().as_ref().unwrap(),
            format!(
                "{}\u{00B0}C",
                (self.temperature.lock().unwrap().round() as i64)
                    .to_string()
                    .as_str()
                    .to_string()
                    .as_str()
            )
            .to_string()
            .as_str(),
        );

        // city
        draw_text_mut(
            image,
            Rgb([255u8, 255u8, 255u8]),
            4,
            50,
            Scale { x: 14.0, y: 14.0 },
            *&self.screen.font.lock().unwrap().as_ref().unwrap(),
            self.city.lock().unwrap().as_str(),
        );
    }

    fn update(instance: Arc<RwLock<WeatherScreen>>) {
        let mut image = RgbImage::new(256, 64);
        let scale = Scale { x: 16.0, y: 16.0 };

        instance
            .write()
            .unwrap()
            .draw_weather_info(&mut image, scale);
        *instance.write().unwrap().screen.bytes.lock().unwrap() = image.into_vec();
    }

    fn get_weather_icon(code: String) -> String {
        match code.as_ref() {
            "01d" => String::from("\u{f185}"),
            "01n" => String::from("\u{f186}"),
            "02d" => String::from("\u{f6c4}"),
            "02n" => String::from("\u{f6c3}"),
            "03d" | "03n" | "04d" | "04n" => String::from("\u{f0c2}"),
            "09d" | "09n" => String::from("\u{f740}"),
            "10d" => String::from("\u{f73d}"),
            "10n" => String::from("\u{f73c}"),
            "11d" | "11n" => String::from("\u{f0e7}"),
            "13d" | "13n" => String::from("\u{f2dc}"),
            "50d" | "50n" => String::from("\u{f75f}"),
            _ => String::from(""),
        }
    }
    pub fn new(
        description: String,
        font: Arc<Mutex<Option<Font<'static>>>>,
        config_manager: Arc<RwLock<ConfigManager>>,
    ) -> Arc<RwLock<WeatherScreen>> {
        let this = Arc::new(RwLock::new(WeatherScreen {
            screen: Screen {
                description,
                font,
                config_manager,
                ..Default::default()
            },
            symbols: Font::try_from_vec(Vec::from(include_bytes!("symbols.otf") as &[u8])),
            weather_icon: Arc::new(Mutex::new(String::from(""))),
            city: Arc::new(Mutex::new(String::from(""))),
            temperature: Arc::new(Mutex::new(0.0)),
        }));

        let builder = thread::Builder::new().name("JOB_EXECUTOR".into());
        *this.read().unwrap().screen.handle.lock().unwrap() = Some(
            builder
                .spawn({
                    let this = this.clone();
                    move || loop {
                        while !this.read().unwrap().screen.active.load(Ordering::Acquire) {
                            thread::park();
                        }
                        let api_key = this
                            .read()
                            .unwrap()
                            .screen
                            .config_manager
                            .read()
                            .unwrap()
                            .config
                            .openweather_api_key
                            .clone();
                        let location = this
                            .read()
                            .unwrap()
                            .screen
                            .config_manager
                            .read()
                            .unwrap()
                            .config
                            .openweather_location
                            .clone();
                        // TODO: make this configurable for language and metric/non-metric units
                        // get current weather for location
                        match &weather(location.as_str(), "metric", "en", api_key.as_str()) {
                            Ok(current) => {
                                *this.write().unwrap().weather_icon.lock().unwrap() =
                                    WeatherScreen::get_weather_icon(
                                        current.weather[0].icon.clone(),
                                    );

                                *this.write().unwrap().temperature.lock().unwrap() =
                                    current.main.temp;
                                *this.write().unwrap().city.lock().unwrap() =
                                    format!("{},{}", current.name.clone(), current.sys.country);
                            }
                            Err(e) => println!("Could not fetch weather because: {}", e),
                        }
                        thread::sleep(Duration::from_millis(60000));
                    }
                })
                .expect("Cannot create JOB_EXECUTOR thread"),
        );
        this.clone()
    }
}
