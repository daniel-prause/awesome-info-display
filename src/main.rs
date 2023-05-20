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

use device::*;
use helpers::keyboard::{self, start_global_key_grabber};
use helpers::power::window_proc;
use helpers::{convert::convert_brightness, convert_image::*, power::register_power_broadcast};
use iced::widget::Text;
use iced::{
    executor, time, window, Application, Command, Element, Font, Length, Settings, Subscription,
};

use image::ImageFormat;
use lazy_static::lazy_static;
use once_cell::sync::Lazy;

use rusttype::Font as ft;
use std::{
    collections::HashMap,
    error::Error,
    ffi::CString,
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

const FONT_BYTES: &[u8] = include_bytes!("Liberation.ttf");
const SYMBOL_BYTES: &[u8] = include_bytes!("symbols.otf");
const ICONS: Font = Font::External {
    name: "Icons",
    bytes: SYMBOL_BYTES,
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

    unsafe {
        let app_image = ::image::load_from_memory(include_bytes!("../icon.ico") as &[u8]);

        let lp_text = CString::new("AwesomeInfoDisplay").unwrap_or_default();
        winapi::um::synchapi::CreateMutexA(std::ptr::null_mut(), 1, lp_text.as_ptr());
        if winapi::um::errhandlingapi::GetLastError()
            == winapi::shared::winerror::ERROR_ALREADY_EXISTS
        {
            Err(iced::Error::WindowCreationFailed(Box::new(
                get_super_error(),
            )))
        } else {
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
            AwesomeDisplay::run(settings)
        }
    }
}

struct AwesomeDisplay {
    screens: Mutex<screen_manager::ScreenManager>,
    config_manager: Arc<RwLock<config_manager::ConfigManager>>,
}

#[derive(Debug, Clone)]
enum Message {
    NextScreen,
    PreviousScreen,
    UpdateCurrentScreen,
    SaveConfig,
    SliderChanged(f32),
    ScreenStatusChanged(bool, String),
    KeyboardEventOccurred(iced::keyboard::KeyCode, u32),
    WindowEventOccurred(iced_native::Event),
    BitpandaApiKeyChanged(String),
    WeatherLocationChanged(String),
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
        screens.push(Box::new(screens::bitpanda_screen::BitpandaScreen::new(
            String::from("Bitpanda Info"),
            String::from("bitpanda_screen"),
            font.clone(),
            config_manager.clone(),
        )));
        screens.push(Box::new(screens::weather_screen::WeatherScreen::new(
            String::from("Weather Info"),
            String::from("weather_screen"),
            font.clone(),
            symbols.clone(),
            config_manager.clone(),
        )));
        screens.push(Box::new(
            screens::current_date_screen::CurrentDateScreen::new(
                String::from("Time Info"),
                String::from("current_date_screen"),
                font.clone(),
                config_manager.clone(),
            ),
        ));
        screens.push(Box::new(screens::ice_screen::IceScreen::new(
            String::from("Ice Sorts"),
            String::from("ice_screen"),
            font.clone(),
        )));

        let this = AwesomeDisplay {
            screens: Mutex::new(screen_manager::ScreenManager::new(screens)),
            config_manager,
        };

        // global key press listener
        start_global_key_grabber(keyboard::callback);

        // init device objects
        for device in DEVICES.values() {
            device.start_background_workers()
        }
        (this, Command::none())
    }
    fn title(&self) -> String {
        String::from("AwesomeInfoDisplay")
    }

    fn subscription(&self) -> Subscription<Message> {
        iced_futures::subscription::Subscription::batch(
            vec![
                iced_native::subscription::events_with(|event, status| {
                    if let iced_native::event::Status::Captured = status {
                        return None;
                    }

                    match event {
                        iced_native::Event::Keyboard(iced::keyboard::Event::KeyReleased {
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
                iced_native::subscription::events().map(Message::WindowEventOccurred),
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
                *LAST_KEY.lock().unwrap() = true;
                *LAST_KEY_VALUE.lock().unwrap() = key_code;
                screen_manager.update_current_screen();
            }
            Message::WindowEventOccurred(event) => {
                if let iced_native::Event::Window(iced_native::window::Event::CloseRequested) =
                    event
                {
                    CLOSE_REQUESTED.store(true, std::sync::atomic::Ordering::Release);
                }
            }
            Message::SliderChanged(slider_value) => {
                self.config_manager.write().unwrap().config.brightness = slider_value as u16;
            }
            Message::ScreenStatusChanged(status, screen) => {
                if screen_manager.screen_deactivatable(&screen) {
                    screen_manager.set_status_for_screen(&screen, status);
                }
            }
            Message::BitpandaApiKeyChanged(message) => {
                self.config_manager.write().unwrap().config.bitpanda_api_key = message;
            }
            Message::WeatherLocationChanged(message) => {
                self.config_manager.write().unwrap().config.weather_location = message;
            }
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

        for (device_name, buffer) in vec![
            (TEENSY, main_screen_bytes),
            (ESP32, companion_screen_bytes),
        ] {
            if !buffer.is_empty() {
                DEVICES
                    .get(device_name)
                    .unwrap()
                    .sender
                    .try_send(buffer)
                    .unwrap_or_default();
            }
        }

        let mut column_parts: Vec<iced_native::Element<Message, iced::Renderer>> = vec![
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
                "Brightness: {:.2}",
                convert_brightness(self.config_manager.read().unwrap().config.brightness) as u16
            ))
            .into(),
            iced::widget::Slider::new(
                20.0..=100.0,
                self.config_manager.read().unwrap().config.brightness as f32,
                Message::SliderChanged,
            )
            .width(Length::Fixed(190f32))
            .step(0.1)
            .into(),
        ];

        // insert screens into left column menu
        for screen in screen_manager.descriptions_and_keys_and_state().into_iter() {
            column_parts.push(special_checkbox(screen.2, screen.1, screen.0));
        }

        let mut left_column_after_screens: Vec<iced_native::Element<Message, iced::Renderer>> = vec![
            iced::widget::text_input(
                "Bitpanda Api Key",
                &self.config_manager.read().unwrap().config.bitpanda_api_key,
            )
            .on_input(Message::BitpandaApiKeyChanged)
            .password()
            .width(Length::Fixed(200f32))
            .style(iced::theme::TextInput::Custom(Box::new(
                style::TextInput {},
            )))
            .into(),
            iced::widget::TextInput::new(
                "weather Location",
                &self.config_manager.read().unwrap().config.weather_location,
            )
            .on_input(Message::WeatherLocationChanged)
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
            iced::widget::Text::new(if DEVICES.get(TEENSY).unwrap().is_connected() {
                String::from("\u{f26c} \u{f058}")
            } else {
                String::from("\u{f26c} \u{f057}")
            })
            .font(ICONS)
            .into(),
        ];

        column_parts.append(&mut left_column_after_screens);

        let col1 = iced_native::widget::Column::with_children(column_parts)
            .padding(20)
            .align_items(iced::Alignment::Center)
            .spacing(10);

        let col2: iced::widget::Column<Message> = iced::widget::Column::new()
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
            );

        iced_native::widget::Row::new().push(col1).push(col2).into()
    }
}

fn special_checkbox<'a>(
    checked: bool,
    key: String,
    description: String,
) -> iced_native::Element<'a, Message, iced::Renderer> {
    iced::widget::checkbox(description, checked, move |value: bool| {
        Message::ScreenStatusChanged(value, key.clone())
    })
    .style(iced::theme::Checkbox::Custom(Box::new(style::Checkbox {})))
    .width(Length::Fixed(200f32))
    .into()
}
