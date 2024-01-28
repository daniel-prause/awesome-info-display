#![windows_subsystem = "windows"]
extern crate winapi;

mod config;
mod config_manager;
mod dada_packet;
mod device;
mod helpers;
mod screen_manager;
mod screens;
mod style;
mod weather;

use debounce::EventDebouncer;
use device::*;
use exchange_format::ConfigParam;
use glob::glob;
use helpers::keyboard::{self, set_last_key, start_global_key_grabber};
use helpers::power::window_proc;
use helpers::text_manipulation::determine_field_value;
use helpers::{
    convert_image::*, power::register_power_broadcast, text_manipulation::humanize_string,
};
use iced::widget::Text;
use iced::{executor, time, window, Application, Command, Element, Length, Settings};
use image::ImageFormat;
use lazy_static::lazy_static;
use named_lock::NamedLock;
use named_lock::Result;
use once_cell::sync::Lazy;
use rusttype::Font as ft;
use std::{
    collections::HashMap,
    error::Error,
    fmt,
    rc::Rc,
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
const FONT_BYTES: &[u8] = include_bytes!("Liberation.ttf");
const SYMBOL_BYTES: &[u8] = include_bytes!("symbols.otf");
const ICONS: iced::Font = iced::Font {
    family: iced::font::Family::Name("Font Awesome 5 Free Solid"),
    weight: iced::font::Weight::Black,
    stretch: iced::font::Stretch::Normal,
    monospaced: false,
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

static DEVICES: Lazy<HashMap<String, Device>> = Lazy::new(|| {
    let mut m: HashMap<String, Device> = HashMap::new();
    m.insert(
        TEENSY.into(),
        Device::new("16c00483".into(), 4608000, false, ImageFormat::Bmp, true),
    );
    m.insert(
        ESP32.into(),
        Device::new("303a1001".into(), 921600, true, ImageFormat::WebP, false),
    );
    m
});

pub fn main() -> iced::Result {
    match signal_hook::flag::register(signal_hook::consts::SIGINT, CLOSE_REQUESTED.clone()) {
        Ok(_) => {}
        Err(_) => {}
    }

    let app_image: Result<image::DynamicImage, image::ImageError> =
        ::image::load_from_memory(include_bytes!("../icon.ico") as &[u8]);
    let lock: Result<NamedLock> = NamedLock::create("AwesomeInfoDisplay");
    match lock {
        Ok(l) => match l.try_lock() {
            Ok(_) => {
                // register power callback
                register_power_broadcast(window_proc);

                let settings = Settings {
                    exit_on_close_request: false,
                    window: window::Settings {
                        resizable: false,
                        decorations: true,
                        icon: Some(
                            iced::window::icon::from_rgba(
                                app_image.unwrap().to_rgba8().to_vec(),
                                256,
                                256,
                            )
                            .unwrap(),
                        ),
                        ..Default::default()
                    },
                    ..Default::default()
                };
                return AwesomeDisplay::run(settings);
            }
            Err(_) => {}
        },
        Err(_) => {}
    }

    return Err(iced::Error::WindowCreationFailed(Box::new(
        get_super_error(),
    )));
}

struct AwesomeDisplay {
    screens: Mutex<screen_manager::ScreenManager>,
    config_manager: Arc<RwLock<config_manager::ConfigManager>>,
    companion_brightness_debouncer: Mutex<EventDebouncer<BrightnessEvent>>,
}

#[derive(Debug, Clone)]
enum Message {
    NextScreen,
    PreviousScreen,
    UpdateCurrentScreen,
    SaveConfig,
    FontLoaded(Result<(), iced::font::Error>),
    MainScreenBrightnessChanged(f32),
    CompanionScreenBrightnessChanged(f32),
    ScreenStatusChanged(bool, String),
    KeyboardEventOccurred(iced::keyboard::KeyCode, u32),
    WindowEventOccurred(iced::Event),
    ConfigValueChanged(String, String, ConfigParam),
}

impl Application for AwesomeDisplay {
    type Executor = executor::Default;
    type Message = Message;
    type Flags = ();
    type Theme = iced::Theme;
    fn new(_flags: ()) -> (AwesomeDisplay, Command<Message>) {
        let font = Rc::new(ft::try_from_vec(Vec::from(FONT_BYTES as &[u8])).unwrap());
        let symbols = Rc::new(ft::try_from_vec(Vec::from(SYMBOL_BYTES as &[u8])).unwrap());
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

        let this = AwesomeDisplay {
            screens: Mutex::new(screen_manager::ScreenManager::new(screens)),
            config_manager,
            companion_brightness_debouncer: Mutex::new(EventDebouncer::new(
                std::time::Duration::from_millis(500),
                move |event: BrightnessEvent| {
                    if DEVICES.get(ESP32).unwrap().is_connected() {
                        DEVICES
                            .get(ESP32)
                            .unwrap()
                            .set_brightness((event.brightness_value * 2.55 as f32) as u8 - 1);
                    }
                },
            )),
        };

        // global key press listener
        start_global_key_grabber(keyboard::callback);

        // init device objects
        for device in DEVICES.values() {
            device.set_brightness(
                (this
                    .config_manager
                    .read()
                    .unwrap()
                    .config
                    .companion_brightness as f32
                    * 2.55f32) as u8
                    - 1,
            );
            device.start_background_workers()
        }
        (
            this,
            iced::font::load(SYMBOL_BYTES).map(Message::FontLoaded),
        )
    }
    fn title(&self) -> String {
        String::from("AwesomeInfoDisplay")
    }

    fn subscription(&self) -> iced::Subscription<Message> {
        iced::Subscription::batch(
            vec![
                iced::subscription::events_with(|event, status| {
                    if let iced::event::Status::Captured = status {
                        return None;
                    }

                    match event {
                        iced::Event::Keyboard(iced::keyboard::Event::KeyReleased {
                            modifiers: _,
                            key_code,
                        }) => match key_code {
                            iced::keyboard::KeyCode::PlayPause => {
                                Some(Message::KeyboardEventOccurred(key_code, 179))
                            }
                            iced::keyboard::KeyCode::MediaStop => {
                                Some(Message::KeyboardEventOccurred(key_code, 178))
                            }
                            iced::keyboard::KeyCode::PrevTrack => {
                                Some(Message::KeyboardEventOccurred(key_code, 177))
                            }
                            iced::keyboard::KeyCode::NextTrack => {
                                Some(Message::KeyboardEventOccurred(key_code, 176))
                            }
                            iced::keyboard::KeyCode::VolumeDown => {
                                Some(Message::KeyboardEventOccurred(key_code, 174))
                            }
                            iced::keyboard::KeyCode::VolumeUp => {
                                Some(Message::KeyboardEventOccurred(key_code, 175))
                            }
                            iced::keyboard::KeyCode::Mute => {
                                Some(Message::KeyboardEventOccurred(key_code, 173))
                            }
                            iced::keyboard::KeyCode::Pause => {
                                Some(Message::KeyboardEventOccurred(key_code, 180))
                            }
                            _ => None,
                        },
                        _ => None,
                    }
                }),
                time::every(std::time::Duration::from_millis(250))
                    .map(|_| Message::UpdateCurrentScreen),
                iced::subscription::events().map(Message::WindowEventOccurred),
            ]
            .into_iter(),
        )
    }
    fn update(&mut self, message: Message) -> Command<Message> {
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
                    } else if (176..180).contains(&val) {
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
                if let iced::Event::Window(iced::window::Event::CloseRequested) = event {
                    CLOSE_REQUESTED.store(true, std::sync::atomic::Ordering::Release);
                }
            }
            Message::MainScreenBrightnessChanged(slider_value) => {
                self.config_manager.write().unwrap().config.brightness = slider_value as u16;
            }
            Message::CompanionScreenBrightnessChanged(slider_value) => {
                self.config_manager
                    .write()
                    .unwrap()
                    .config
                    .companion_brightness = slider_value as u16;
                self.companion_brightness_debouncer
                    .lock()
                    .unwrap()
                    .put(BrightnessEvent {
                        brightness_value: slider_value,
                        ..Default::default()
                    });
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
                    .set_value(screen, key, value);
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
            return window::close();
        }
        Command::none()
    }

    fn theme(&self) -> iced::Theme {
        iced::Theme::custom(iced::theme::Palette {
            background: iced::Color::from_rgb(0.21, 0.22, 0.247),
            text: iced::Color::WHITE,
            primary: iced::Color::from_rgb(114.0 / 255.0, 137.0 / 255.0, 218.0 / 255.0),
            success: iced::Color::from_rgb(0.0, 1.0, 0.0),
            danger: iced::Color::from_rgb(1.0, 0.0, 0.0),
        })
    }

    fn view(&self) -> Element<Message> {
        let mut screen_manager = self.screens.lock().unwrap();
        let main_screen_bytes = screen_manager.current_screen().current_image().clone();
        let companion_screen_bytes = screen_manager
            .current_screen()
            .current_image_for_companion()
            .clone();

        // preview image
        let main_screen_image =
            rgb_bytes_to_rgba_image(&swap_rgb(&main_screen_bytes, 256, 64), 256, 64);
        let companion_screen_image =
            rgb_bytes_to_rgba_image(&swap_rgb(&companion_screen_bytes, 320, 170), 320, 170);

        // convert to gray scale for display
        let main_screen_bytes = convert_to_gray_scale(&adjust_brightness_rgb(
            &main_screen_bytes,
            self.config_manager.read().unwrap().config.brightness as f32,
        ));

        for (device_name, buffer) in
            vec![(TEENSY, main_screen_bytes), (ESP32, companion_screen_bytes)]
        {
            if !buffer.is_empty() {
                DEVICES
                    .get(device_name)
                    .unwrap()
                    .sender
                    .try_send(buffer)
                    .unwrap_or_default();
            }
        }

        let mut column_parts = vec![
            iced::widget::button(
                Text::new("Next screen").horizontal_alignment(iced::alignment::Horizontal::Center),
            )
            .on_press(Message::NextScreen)
            .width(Length::Fixed(200f32))
            .into(),
            iced::widget::button(
                Text::new("Previous screen")
                    .horizontal_alignment(iced::alignment::Horizontal::Center),
            )
            .on_press(Message::PreviousScreen)
            .width(Length::Fixed(200f32))
            .into(),
            iced::widget::text(format!(
                "Main Brightness: {:.2}",
                self.config_manager.read().unwrap().config.brightness
            ))
            .into(),
            iced::widget::Slider::new(
                20.0..=100.0,
                self.config_manager.read().unwrap().config.brightness as f32,
                Message::MainScreenBrightnessChanged,
            )
            .width(Length::Fixed(190f32))
            .step(1.0)
            .into(),
            iced::widget::text(format!(
                "Companion Brightness: {:.2}",
                self.config_manager
                    .read()
                    .unwrap()
                    .config
                    .companion_brightness
            ))
            .horizontal_alignment(iced::alignment::Horizontal::Center)
            .width(Length::Fixed(210f32))
            .into(),
            iced::widget::Slider::new(
                20.0..=100.0,
                self.config_manager
                    .read()
                    .unwrap()
                    .config
                    .companion_brightness as f32,
                Message::CompanionScreenBrightnessChanged,
            )
            .width(Length::Fixed(190f32))
            .step(1.0)
            .into(),
        ];

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

        let mut left_column_after_screens = vec![
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
            .style(iced::theme::TextInput::Custom(Box::new(
                style::TextInput {},
            )))
            .width(Length::Fixed(200f32))
            .into(),
            iced::widget::button(
                Text::new("Save config").horizontal_alignment(iced::alignment::Horizontal::Center),
            )
            .width(Length::Fixed(200f32))
            .on_press(Message::SaveConfig)
            .into(),
            iced::widget::Row::with_children(vec![iced::widget::vertical_space(10).into()]).into(),
            iced::widget::Row::with_children(vec![iced::widget::text("Devices").into()]).into(),
            iced::widget::Row::with_children(device_status(TEENSY)).into(),
            iced::widget::Row::with_children(device_status(ESP32)).into(),
        ];

        column_parts.append(&mut left_column_after_screens);

        let col1 = iced::widget::Column::with_children(column_parts)
            .padding(20)
            .align_items(iced::Alignment::Center)
            .spacing(10);

        let mut col2: iced::widget::Column<Message> = iced::widget::Column::new()
            .padding(20)
            .align_items(iced::Alignment::Center)
            .width(Length::Fill)
            .push(iced::widget::text("Current screen").size(50))
            .push(iced::widget::text(screen_manager.current_screen().description()).size(25))
            .push(
                main_screen_image
                    .width(Length::Fixed(256f32))
                    .height(Length::Fixed(64f32)),
            )
            .spacing(10)
            .push(
                // companion image
                companion_screen_image
                    .width(Length::Fixed(320f32))
                    .height(Length::Fixed(170f32)),
            )
            .push(iced::widget::Row::new().height(50));
        // push config fields:
        for (key, item) in screen_manager.current_screen().config_layout().params {
            let screen_key: String = screen_manager.current_screen().key().clone();
            let config_param_option = &self
                .config_manager
                .read()
                .unwrap()
                .get_value(&screen_key, key.as_str());

            match item {
                ConfigParam::String(value) => {
                    let field_value: String;
                    match config_param_option {
                        Some(config_param) => match config_param {
                            ConfigParam::String(saved_value) => {
                                field_value = saved_value.clone();
                            }
                            _ => {
                                field_value = value;
                            }
                        },
                        _ => {
                            field_value = value;
                        }
                    }
                    col2 = col2.push(
                        iced::widget::TextInput::new(
                            humanize_string(&key).as_str(),
                            field_value.as_str(),
                        )
                        .on_input(move |value: String| {
                            Message::ConfigValueChanged(
                                screen_key.clone(),
                                key.clone(),
                                exchange_format::ConfigParam::String(value),
                            )
                        })
                        .style(iced::theme::TextInput::Custom(Box::new(
                            style::TextInput {},
                        )))
                        .width(Length::Fixed(200f32)),
                    );
                }
                ConfigParam::Integer(value) => {
                    let field_value: u32;
                    match config_param_option {
                        Some(config_param) => match config_param {
                            ConfigParam::Integer(saved_value) => {
                                field_value = *saved_value;
                            }
                            _ => {
                                field_value = value;
                            }
                        },
                        _ => {
                            field_value = value;
                        }
                    }
                    col2 = col2.push(
                        iced::widget::TextInput::new(
                            humanize_string(&key).as_str(),
                            determine_field_value(&field_value.to_string().as_str()).as_str(),
                        )
                        .on_input(move |value: String| {
                            Message::ConfigValueChanged(
                                screen_key.clone(),
                                key.clone(),
                                exchange_format::ConfigParam::Integer({
                                    let val: String = value
                                        .to_string()
                                        .chars()
                                        .filter(|c| c.is_numeric())
                                        .collect();

                                    val.parse().unwrap_or(0)
                                }),
                            )
                        })
                        .style(iced::theme::TextInput::Custom(Box::new(
                            style::TextInput {},
                        )))
                        .width(Length::Fixed(200f32)),
                    );
                }
                ConfigParam::Password(value) => {
                    let field_value: String;
                    match config_param_option {
                        Some(config_param) => match config_param {
                            ConfigParam::Password(saved_value) => {
                                field_value = saved_value.clone();
                            }
                            _ => {
                                field_value = value;
                            }
                        },
                        _ => {
                            field_value = value;
                        }
                    }
                    col2 = col2.push(
                        iced::widget::TextInput::new(
                            humanize_string(&key).as_str(),
                            field_value.as_str(),
                        )
                        .on_input(move |value: String| {
                            Message::ConfigValueChanged(
                                screen_key.clone(),
                                key.clone(),
                                exchange_format::ConfigParam::Password(value),
                            )
                        })
                        .password()
                        .style(iced::theme::TextInput::Custom(Box::new(
                            style::TextInput {},
                        )))
                        .width(Length::Fixed(200f32)),
                    );
                }
                _ => {} /*
                        TODO: built float value!
                        ConfigParam::Float(value) => {}
                         */
            }
        }
        // TODO: check, which screen is the current screen and render only the elements of this screen.
        iced::widget::Row::new().push(col1).push(col2).into()
    }
}

fn special_checkbox<'a>(
    checked: bool,
    key: String,
    description: String,
) -> iced::Element<'a, Message, iced::Renderer> {
    iced::widget::checkbox(description, checked, move |value: bool| {
        Message::ScreenStatusChanged(value, key.clone())
    })
    .style(iced::theme::Checkbox::Custom(Box::new(style::Checkbox {})))
    .width(Length::Fixed(200f32))
    .into()
}

fn device_connected_icon<'a>(is_connected: bool) -> iced::Element<'a, Message, iced::Renderer> {
    iced::widget::text(if is_connected {
        String::from("\u{f26c} \u{f058}")
    } else {
        String::from("\u{f26c} \u{f057}")
    })
    .font(ICONS)
    .shaping(iced::widget::text::Shaping::Advanced)
    .into()
}

fn device_status<'a>(device: &str) -> Vec<iced::Element<'a, Message, iced::Renderer>> {
    vec![
        iced::widget::Text::new(device.to_uppercase())
            .width(Length::Fixed(146f32))
            .font(iced::Font::MONOSPACE)
            .into(),
        device_connected_icon(DEVICES.get(device).unwrap().is_connected()),
    ]
}
