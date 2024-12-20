#![windows_subsystem = "windows"]
extern crate winapi;

mod config;
mod config_manager;
mod converters;
mod dada_packet;
mod device;
mod helpers;
mod screen_manager;
mod screens;
mod style;
mod weather;

use ab_glyph::FontArc;
use converters::image::{GrayscaleConverter, ImageProcessor, WebPConverter};
use debounce::EventDebouncer;
use device::*;
use exchange_format::ConfigParam;
use glob::glob;
use helpers::keyboard::{self, set_last_key, start_global_key_grabber};
use helpers::power::window_proc;
use helpers::{
    convert_image::*, gui_helpers::*, power::register_power_broadcast,
    text_manipulation::humanize_string,
};
use iced::widget::{Space, Text};
use iced::{time, window, Element, Length, Subscription, Task, Theme};

use indexmap::IndexMap;
use lazy_static::lazy_static;
use named_lock::NamedLock;
use named_lock::Result;
use once_cell::sync::Lazy;

use std::{
    error::Error,
    fmt,
    sync::{atomic::AtomicBool, Arc, Mutex, RwLock},
};

#[derive(Debug)]
struct SuperError {
    side: SuperErrorSideKick,
}

impl fmt::Display for SuperError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SuperError is here!")
    }
}

impl Error for SuperError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.side)
    }
}

#[derive(Debug)]
struct SuperErrorSideKick;

impl fmt::Display for SuperErrorSideKick {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "App already opened!")
    }
}

impl Error for SuperErrorSideKick {}

fn get_super_error() -> SuperError {
    SuperError {
        side: SuperErrorSideKick,
    }
}

struct BrightnessEvent {
    brightness_value: f32,
    event_type: String,
}

impl Default for BrightnessEvent {
    fn default() -> BrightnessEvent {
        BrightnessEvent {
            brightness_value: 0f32,
            event_type: "brightness_event".into(),
        }
    }
}
impl PartialEq for BrightnessEvent {
    fn eq(&self, other: &Self) -> bool {
        self.event_type == other.event_type
    }
}
const FONT_BYTES: &[u8] = include_bytes!("fonts/Liberation.ttf");
const SYMBOL_BYTES: &[u8] = include_bytes!("fonts/symbols.otf");
const ICONS: iced::Font = iced::Font {
    family: iced::font::Family::Name("Font Awesome 5 Free Solid"),
    weight: iced::font::Weight::Black,
    stretch: iced::font::Stretch::Normal,
    style: iced::font::Style::Normal,
};

lazy_static! {
    static ref LAST_KEY: Mutex<bool> = Mutex::new(false);
    static ref LAST_KEY_VALUE: Mutex<u32> = Mutex::new(0);
    static ref CLOSE_REQUESTED: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
    static ref HIBERNATING: Mutex<bool> = Mutex::new(false);
    static ref LAST_BME_INFO: Mutex<(String, String)> = Mutex::new((String::new(), String::new()));
}
const TEENSY: &str = "teensy";
const ESP32: &str = "esp32";

static DEVICES: Lazy<IndexMap<String, Device>> = Lazy::new(|| {
    let mut m: IndexMap<String, Device> = IndexMap::new();
    m.insert(
        TEENSY.into(),
        Device::new(
            "16c00483".into(),
            4608000,
            false,
            ImageProcessor::new(Box::new(GrayscaleConverter), 256, 64),
            true,
            false,
            Arc::new(|value| value),
        ),
    );
    m.insert(
        ESP32.into(),
        Device::new(
            "303a1001".into(),
            921600,
            true,
            ImageProcessor::new(Box::new(WebPConverter), 320, 170),
            false,
            true,
            Arc::new(|value| (value as f32 * 2.55f32) as u8 - 1),
        ),
    );
    m
});

pub fn main() -> iced::Result {
    // register signal hook
    if let Err(e) =
        signal_hook::flag::register(signal_hook::consts::SIGINT, CLOSE_REQUESTED.clone())
    {
        eprintln!("Error registering SIGINT: {:?}", e);
    }

    // load app icon
    let app_image =
        image::load_from_memory(include_bytes!("../icon.ico") as &[u8]).map_err(|e| {
            eprintln!("Could not load app icon {:?}", e);
            iced::Error::WindowCreationFailed(Box::new(e))
        })?;

    // prevent opening app multiple times
    let _lock = NamedLock::create("AwesomeInfoDisplay")
        .and_then(|l| {
            l.try_lock().map_err(|e| {
                eprintln!("App probably already open: {:?}", e);
                e
            })
        })
        .map_err(|_| iced::Error::WindowCreationFailed(Box::new(get_super_error())))?;

    // register power broadcast
    register_power_broadcast(window_proc);

    // set windows settings
    let settings = window::Settings {
        exit_on_close_request: false,
        resizable: false,
        decorations: true,
        icon: Some(
            iced::window::icon::from_rgba(app_image.to_rgba8().to_vec(), 256, 256)
                .expect("Could not convert app icon"),
        ),
        ..Default::default()
    };

    // start application
    iced::application(
        "AwesomeInfoDisplay",
        AwesomeDisplay::update,
        AwesomeDisplay::view,
    )
    .window(settings)
    .subscription(AwesomeDisplay::subscription)
    .default_font(iced::Font::DEFAULT)
    .theme(AwesomeDisplay::theme)
    .run_with(AwesomeDisplay::new)
}

struct AwesomeDisplay {
    render_preview_image: bool,
    screens: Arc<Mutex<screen_manager::ScreenManager>>,
    config_manager: Arc<RwLock<config_manager::ConfigManager>>,
    companion_brightness_debouncers: IndexMap<String, Mutex<EventDebouncer<BrightnessEvent>>>,
}

#[derive(Debug, Clone)]
enum Message {
    NextScreen,
    PreviousScreen,
    UpdateCurrentScreen,
    SaveConfig,
    FontLoaded(Result<(), iced::font::Error>),
    BrightnessChanged(f32, String),
    ScreenStatusChanged(bool, String),
    KeyboardEventOccurred(iced::keyboard::Key, u32),
    WindowEventOccurred(iced::window::Event),
    ConfigValueChanged(String, String, ConfigParam),
}

impl AwesomeDisplay {
    fn new() -> (AwesomeDisplay, Task<Message>) {
        let font = FontArc::try_from_slice(FONT_BYTES).unwrap();
        let symbols = FontArc::try_from_slice(SYMBOL_BYTES).unwrap();
        let config_manager =
            std::sync::Arc::new(RwLock::new(config_manager::ConfigManager::new(None)));
        let mut screens: Vec<Box<dyn screens::BasicScreen>> = Vec::new();

        screens.push(Box::new(
            screens::system_info_screen::SystemInfoScreen::new(
                String::from("System Info"),
                String::from("system_info_screen"),
                font.clone(),
                config_manager.clone(),
            ),
        ));
        screens.push(Box::new(screens::media_info_screen::MediaInfoScreen::new(
            String::from("Media Info"),
            String::from("media_info_screen"),
            font.clone(),
            symbols.clone(),
            config_manager.clone(),
        )));
        screens.push(Box::new(screens::weather_screen::WeatherScreen::new(
            String::from("Weather Info"),
            String::from("weather_screen"),
            font.clone(),
            symbols.clone(),
            config_manager.clone(),
        )));

        // look for plugins - windows only right now
        for entry in glob("./*.dll").expect("Failed to read glob pattern") {
            match entry {
                Ok(path) => {
                    // load email screen
                    screens.push(Box::new(screens::plugin_screen::PluginScreen::new(
                        font.clone(),
                        symbols.clone(),
                        config_manager.clone(),
                        path,
                    )));
                }
                Err(e) => println!("Failed to load plugins: {:?}", e),
            }
        }

        let mut debouncers = IndexMap::new();

        for (key, device) in DEVICES.iter() {
            if device.adjust_brightness_on_device() {
                debouncers.insert(
                    key.clone(),
                    Mutex::new(EventDebouncer::new(
                        std::time::Duration::from_millis(500),
                        move |event: BrightnessEvent| {
                            if DEVICES.get(key).unwrap().is_connected() {
                                DEVICES
                                    .get(key)
                                    .unwrap()
                                    .set_brightness(event.brightness_value as u8);
                            }
                        },
                    )),
                );
            }
        }

        let this = AwesomeDisplay {
            render_preview_image: true,
            screens: Arc::new(Mutex::new(screen_manager::ScreenManager::new(screens))),
            config_manager,
            companion_brightness_debouncers: debouncers,
        };

        // global key press listener
        start_global_key_grabber(keyboard::callback);

        // init device objects
        for (key, device) in DEVICES.iter() {
            device.set_brightness(this.config_manager.read().unwrap().get_brightness(key));

            device.start_background_workers()
        }
        (
            this,
            iced::font::load(SYMBOL_BYTES).map(Message::FontLoaded),
        )
    }

    fn subscription(&self) -> iced::Subscription<Message> {
        let tick = time::every(std::time::Duration::from_millis(250))
            .map(|_| Message::UpdateCurrentScreen);

        fn handle_hotkey(
            key: iced::keyboard::Key,
            _modifiers: iced::keyboard::Modifiers,
        ) -> Option<Message> {
            match key.as_ref() {
                iced::keyboard::Key::Named(iced::keyboard::key::Named::MediaPlayPause) => {
                    Some(Message::KeyboardEventOccurred(key, 179))
                }
                iced::keyboard::Key::Named(iced::keyboard::key::Named::MediaStop) => {
                    Some(Message::KeyboardEventOccurred(key, 178))
                }
                iced::keyboard::Key::Named(iced::keyboard::key::Named::MediaTrackPrevious) => {
                    Some(Message::KeyboardEventOccurred(key, 177))
                }
                iced::keyboard::Key::Named(iced::keyboard::key::Named::MediaTrackNext) => {
                    Some(Message::KeyboardEventOccurred(key, 176))
                }
                iced::keyboard::Key::Named(iced::keyboard::key::Named::AudioVolumeDown) => {
                    Some(Message::KeyboardEventOccurred(key, 174))
                }
                iced::keyboard::Key::Named(iced::keyboard::key::Named::AudioVolumeUp) => {
                    Some(Message::KeyboardEventOccurred(key, 175))
                }
                iced::keyboard::Key::Named(iced::keyboard::key::Named::AudioVolumeMute) => {
                    Some(Message::KeyboardEventOccurred(key, 173))
                }
                iced::keyboard::Key::Named(iced::keyboard::key::Named::Pause) => {
                    Some(Message::KeyboardEventOccurred(key, 180))
                }
                _ => None,
            }
        }
        fn handle_window_event(
            event: iced::Event,
            _status: iced::event::Status,
            _id: iced::window::Id,
        ) -> Option<Message> {
            match event {
                iced::event::Event::Window(event) => Some(Message::WindowEventOccurred(event)),
                _ => None,
            }
        }

        Subscription::batch(vec![
            tick,
            iced::keyboard::on_key_press(handle_hotkey),
            iced::event::listen_with(handle_window_event),
        ])
    }
    fn update(&mut self, message: Message) -> Task<Message> {
        let mut screen_manager = self.screens.lock().unwrap();
        match message {
            Message::SaveConfig => {
                self.config_manager.write().unwrap().save();
            }
            Message::NextScreen => {
                screen_manager.update_current_screen();
                screen_manager.next_screen();
                screen_manager.update_current_screen();
            }
            Message::PreviousScreen => {
                screen_manager.update_current_screen();
                screen_manager.previous_screen();
                screen_manager.update_current_screen();
            }
            Message::UpdateCurrentScreen => {
                if *LAST_KEY.lock().unwrap() {
                    *LAST_KEY.lock().unwrap() = false;
                    let val = *LAST_KEY_VALUE.lock().unwrap();
                    if val == 174 || val == 175 {
                        // 1 is "volume mode"
                        screen_manager.set_screen_for_short("media_info_screen".into(), 1);
                    } else if [173, 176, 177, 178, 179].contains(&val) {
                        // 0 is "normal mode"
                        screen_manager.set_screen_for_short("media_info_screen".into(), 0);
                    } else if val == 180 {
                        screen_manager.next_screen()
                    }
                    *LAST_KEY_VALUE.lock().unwrap() = 0;
                }
                screen_manager.update_current_screen();
            }
            Message::KeyboardEventOccurred(_event, key_code) => {
                // switch to media screen for a few seconds
                set_last_key(key_code);
                screen_manager.update_current_screen();
            }
            Message::WindowEventOccurred(event) => {
                match event {
                    iced::window::Event::Moved(position) => {
                        if position.x < 0.0 && position.y < 0.0 {
                            self.render_preview_image = false
                        } else {
                            self.render_preview_image = true
                        }
                    }
                    _ => {}
                }

                if iced::window::Event::CloseRequested == event {
                    CLOSE_REQUESTED.store(true, std::sync::atomic::Ordering::Release);
                }
            }

            Message::BrightnessChanged(slider_value, key) => {
                let device = DEVICES.get(&key);
                match device {
                    Some(d) => {
                        self.config_manager
                            .write()
                            .unwrap()
                            .set_brightness(key.as_str(), slider_value as u8);
                        if d.adjust_brightness_on_device() {
                            self.companion_brightness_debouncers
                                .get(&key)
                                .unwrap()
                                .lock()
                                .unwrap()
                                .put(BrightnessEvent {
                                    brightness_value: slider_value,
                                    ..Default::default()
                                });
                        } else {
                            d.set_brightness(slider_value as u8);
                        }
                    }
                    _ => {}
                }
            }

            Message::ScreenStatusChanged(status, screen) => {
                if screen_manager.screen_deactivatable(&screen) {
                    screen_manager.set_status_for_screen(&screen, status);
                }
            }
            Message::ConfigValueChanged(screen, key, value) => {
                self.config_manager
                    .write()
                    .unwrap()
                    .set_value(screen, key.clone(), value);
                let serialized_screen_config = self
                    .config_manager
                    .read()
                    .unwrap()
                    .get_screen_config(&key)
                    .unwrap_or_default();

                screen_manager.update_screen_config(&key, serialized_screen_config.clone())
            }
            _ => (),
        }

        // disconnect all devices, if application will be closed
        if CLOSE_REQUESTED.load(std::sync::atomic::Ordering::Acquire) {
            for (_, device) in DEVICES.iter() {
                if device.is_connected() {
                    device.reset_display();
                    device.disconnect();
                }
            }
            self.config_manager.write().unwrap().save();
            return iced::window::get_latest().and_then(iced::window::close);
        }

        Task::none()
    }

    fn theme(&self) -> iced::Theme {
        iced::Theme::custom(
            "Default".into(),
            iced::theme::Palette {
                background: iced::Color::from_rgb(0.21, 0.22, 0.247),
                text: iced::Color::WHITE,
                primary: iced::Color::from_rgb(114.0 / 255.0, 137.0 / 255.0, 218.0 / 255.0),
                success: iced::Color::from_rgb(0.0, 1.0, 0.0),
                danger: iced::Color::from_rgb(1.0, 0.0, 0.0),
            },
        )
    }

    fn view(&self) -> Element<Message> {
        let mut screen_manager = self.screens.lock().unwrap();
        let mut screen_bytes: IndexMap<&str, Vec<u8>> = IndexMap::new();

        for key in DEVICES.keys() {
            let bytes = screen_manager.current_screen().current_image(key);
            match bytes {
                Some(b) => {
                    screen_bytes.insert(key, b);
                }
                _ => {}
            }
        }

        for (device_name, buffer) in screen_bytes.iter() {
            if !buffer.is_empty() {
                DEVICES
                    .get(*device_name)
                    .unwrap()
                    .sender
                    .try_send(buffer.clone())
                    .unwrap_or_default();
            }
        }

        let mut column_parts: Vec<iced::Element<Message, Theme, iced::Renderer>> = vec![
            iced::widget::button(Text::new("Next screen").center())
                .on_press(Message::NextScreen)
                .width(Length::Fixed(200f32))
                .into(),
            iced::widget::button(Text::new("Previous screen").center())
                .on_press(Message::PreviousScreen)
                .width(Length::Fixed(200f32))
                .into(),
        ];

        for key in DEVICES.keys() {
            let text: iced::Element<Message, Theme, iced::Renderer> = iced::widget::text(format!(
                "{} brightness: {}%",
                key.to_uppercase(),
                self.config_manager.read().unwrap().get_brightness(key)
            ))
            .center()
            .width(Length::Fixed(220f32))
            .into();
            let slider: iced::Element<Message, Theme, iced::Renderer> = iced::widget::Slider::new(
                20.0..=100.0,
                self.config_manager.read().unwrap().get_brightness(key) as f32,
                |slider_value| -> Message {
                    Message::BrightnessChanged(slider_value, key.to_string())
                },
            )
            .width(Length::Fixed(200f32))
            .step(1.0)
            .into();

            column_parts.push(text);
            column_parts.push(slider);
        }

        // insert screens into left column menu
        for screen in screen_manager.descriptions_and_keys_and_state().into_iter() {
            column_parts.push(special_checkbox(screen.2, screen.1, screen.0));
        }
        // TODO: find a way to generate these fields dynamically
        let weather_location_option = &self
            .config_manager
            .read()
            .unwrap()
            .get_value("weather_screen", "weather_location");

        let weather_location;
        match weather_location_option {
            Some(config_param) => match config_param {
                exchange_format::ConfigParam::String(value) => {
                    weather_location = value.clone();
                }
                _ => {
                    weather_location = String::new();
                }
            },
            None => {
                weather_location = String::new();
            }
        }

        // TODO: move to its own plugin
        let mut left_column_after_screens: Vec<iced::Element<Message, Theme, iced::Renderer>> = vec![
            iced::widget::TextInput::new(
                humanize_string("weather_location").as_str(),
                weather_location.as_str(),
            )
            .on_input(move |value: String| {
                Message::ConfigValueChanged(
                    "weather_screen".into(),
                    "weather_location".into(),
                    exchange_format::ConfigParam::String(value),
                )
            })
            .style(|_theme, _status| crate::style::text_field())
            .width(Length::Fixed(200f32))
            .into(),
            iced::widget::button(Text::new("Save config").center())
                .width(Length::Fixed(200f32))
                .on_press(Message::SaveConfig)
                .into(),
            iced::widget::Row::with_children(vec![Space::with_height(10).into()]).into(),
            iced::widget::Row::with_children(vec![iced::widget::text("Devices").into()]).into(),
        ];

        for key in DEVICES.keys() {
            let row: iced::Element<Message, Theme, iced::Renderer> =
                iced::widget::Row::with_children(device_status(key)).into();
            left_column_after_screens.push(row);
        }

        column_parts.append(&mut left_column_after_screens);

        let col1 = iced::widget::Column::with_children(column_parts)
            .padding(20)
            .align_x(iced::Alignment::Center)
            .spacing(10);

        let mut col2 = iced::widget::Column::new()
            .padding(20)
            .align_x(iced::Alignment::Center)
            .width(Length::Fill)
            .spacing(10)
            .push(iced::widget::text("Current screen").size(50))
            .push(iced::widget::text(screen_manager.current_screen().description()).size(25));

        if self.render_preview_image {
            col2 = col2.extend(preview_images(screen_bytes));
        }
        col2 = col2.push(iced::widget::Row::new().height(50));
        // push config fields:
        col2 = col2.extend(push_config_fields(
            self.config_manager.clone(),
            screen_manager.current_screen().config_layout().params,
            screen_manager.current_screen().key(),
        ));
        // TODO: check, which screen is the current screen and render only the elements of this screen.
        iced::widget::Row::new().push(col1).push(col2).into()
    }
}
