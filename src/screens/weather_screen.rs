extern crate openweathermap;
use crate::config_manager::ConfigManager;
use crate::screens::BasicScreen;
use crate::screens::Screen;
use crate::screens::ScreenControl;
use crossbeam_channel::bounded;
use crossbeam_channel::{Receiver, Sender};
use image::{ImageBuffer, Rgb, RgbImage};
use imageproc::drawing::draw_text_mut;
use openweathermap::blocking::weather;
use rusttype::Font;
use rusttype::Scale;
use std::rc::Rc;
use std::sync::{atomic::AtomicBool, atomic::Ordering, Arc, RwLock};
use std::thread;
use std::time::Duration;

pub struct WeatherScreen {
    screen: Screen,
    symbols: Option<Font<'static>>,
    receiver: Receiver<WeatherInfo>,
}

#[derive(Default, Clone)]
struct WeatherInfo {
    weather_icon: String,
    city: String,
    temperature: f64,
}

impl BasicScreen for WeatherScreen {
    fn description(&self) -> &String {
        &self.screen.description
    }

    fn current_image(&self) -> Vec<u8> {
        self.screen.current_image()
    }

    fn update(&mut self) {
        WeatherScreen::update(self);
    }

    fn start(&self) {
        self.screen.start_worker();
    }

    fn stop(&self) {
        self.screen.stop_worker();
    }

    fn key(&self) -> &String {
        &self.screen.key()
    }

    fn initial_update_called(&mut self) -> bool {
        self.screen.initial_update_called()
    }

    fn enabled(&self) -> bool {
        self.screen
            .config_manager
            .read()
            .unwrap()
            .config
            .weather_screen_active
    }

    fn set_status(&self, status: bool) {
        self.screen
            .config_manager
            .write()
            .unwrap()
            .config
            .weather_screen_active = status;
    }
}

impl WeatherScreen {
    fn draw_screen(&mut self, weather_info: WeatherInfo) {
        // draw initial image
        let mut image = RgbImage::new(256, 64);
        self.draw_weather_info(weather_info, &mut image);
        self.screen.bytes = image.into_vec();
    }
    fn draw_weather_info(
        &mut self,
        weather_info: WeatherInfo,
        image: &mut ImageBuffer<Rgb<u8>, Vec<u8>>,
    ) {
        // icon
        draw_text_mut(
            image,
            Rgb([255u8, 255u8, 255u8]),
            6,
            6,
            Scale { x: 40.0, y: 40.0 },
            self.symbols.as_ref().unwrap(),
            WeatherScreen::get_weather_icon(weather_info.weather_icon).as_str(),
        );

        // temperature
        draw_text_mut(
            image,
            Rgb([255u8, 255u8, 255u8]),
            72,
            16,
            Scale { x: 32.0, y: 32.0 },
            &self.screen.font,
            format!(
                "{}\u{00B0}C",
                (weather_info.temperature.round() as i64)
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
            &self.screen.font,
            weather_info.city.as_str(),
        );
    }

    fn update(&mut self) {
        let weather_info = self.receiver.try_recv();
        match weather_info {
            Ok(weather_info) => {
                self.draw_screen(weather_info);
            }
            Err(_) => {}
        }
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
        key: String,
        font: Rc<Font<'static>>,
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
                    loop {
                        while !active.load(Ordering::Acquire) {
                            thread::park();
                        }
                        let api_key = config_manager
                            .read()
                            .unwrap()
                            .config
                            .openweather_api_key
                            .clone();
                        let location = config_manager
                            .read()
                            .unwrap()
                            .config
                            .openweather_location
                            .clone();
                        // TODO: make this configurable for language and metric/non-metric units
                        // get current weather for location
                        match &weather(location.as_str(), "metric", "en", api_key.as_str()) {
                            Ok(current) => {
                                let mut weather_info: WeatherInfo = Default::default();

                                weather_info.weather_icon = current.weather[0].icon.clone();

                                weather_info.temperature = current.main.temp;
                                weather_info.city =
                                    format!("{},{}", current.name.clone(), current.sys.country);
                                sender.try_send(weather_info).unwrap_or_default();
                            }
                            Err(e) => println!("Could not fetch weather because: {}", e),
                        }
                        // TODO: think about whether we want to solve this like in bitpanda screen with last_update...
                        thread::sleep(Duration::from_millis(60000));
                    }
                })),
                ..Default::default()
            },
            symbols: Font::try_from_vec(Vec::from(include_bytes!("../symbols.otf") as &[u8])),
            receiver: rx,
        };

        this.draw_screen(Default::default());
        this
    }
}
