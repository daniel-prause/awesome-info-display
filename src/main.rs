#![windows_subsystem = "windows"]
extern crate winapi;

use iced::widget::Image;
use iced::widget::Text;
use iced::{
    executor, time, window, Application, Command, Element, Font, Length, Settings, Subscription,
};

use std::time::Duration;
mod config;
mod config_manager;
mod display_serial_com;
mod helpers;
mod screen_manager;
mod screens;
mod style;

use crate::display_serial_com::{
    convert_to_gray_scale, init_serial, reset_display, write_screen_buffer,
};
use crate::helpers::power::register_power_broadcast;
use crossbeam_channel::bounded;
use crossbeam_channel::{Receiver, Sender};
use lazy_static::lazy_static;
use rdev::{grab, Event, EventType, Key};
use rusttype::Font as ft;
use std::error::Error;
use std::ffi::CString;
use std::fmt;
use std::rc::Rc;
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use winapi::shared::minwindef::*;
use winapi::shared::windef::*;
use winapi::um::winuser::*;

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
    static ref SERIAL_PORT: Mutex<Option<Box<dyn serialport::SerialPort>>> = Mutex::new(None);
    static ref TEENSY_CONNECTED: Mutex<bool> = Mutex::new(false);
    static ref CLOSE_REQUESTED: std::sync::atomic::AtomicBool =
        std::sync::atomic::AtomicBool::new(false);
    static ref HIBERNATING: Mutex<bool> = Mutex::new(false);
}
pub fn main() -> iced::Result {
    unsafe {
        let app_image = ::image::load_from_memory(include_bytes!("../icon.ico") as &[u8]);

        let lp_text = CString::new("AwesomeInfoDisplay").unwrap();
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
                        iced::window::icon::Icon::from_rgba(
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
    screens: screen_manager::ScreenManager,
    config_manager: Arc<RwLock<config_manager::ConfigManager>>,
    should_exit: bool,
    sender: Sender<Vec<u8>>,
    current_screen: crate::screens::Screen,
    screen_descriptions: Vec<(String, String, bool)>,
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
    OpenWeatherApiKeyChanged(String),
    OpenWeatherLocationChanged(String),
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
                Rc::clone(&font),
                Arc::clone(&config_manager),
            ),
        ));
        screens.push(Box::new(screens::media_info_screen::MediaInfoScreen::new(
            String::from("Media Info"),
            String::from("media_info_screen"),
            Rc::clone(&font),
            Rc::clone(&symbols),
            Arc::clone(&config_manager),
        )));
        screens.push(Box::new(screens::bitpanda_screen::BitpandaScreen::new(
            String::from("Bitpanda Info"),
            String::from("bitpanda_screen"),
            Rc::clone(&font),
            Arc::clone(&config_manager),
        )));
        screens.push(Box::new(screens::weather_screen::WeatherScreen::new(
            String::from("Weather Info"),
            String::from("weather_screen"),
            Rc::clone(&font),
            Rc::clone(&symbols),
            Arc::clone(&config_manager),
        )));
        screens.push(Box::new(
            screens::current_date_screen::CurrentDateScreen::new(
                String::from("Time Info"),
                String::from("current_date_screen"),
                Rc::clone(&font),
                Arc::clone(&config_manager),
            ),
        ));
        screens.push(Box::new(screens::ice_screen::IceScreen::new(
            String::from("Ice Sorts"),
            String::from("ice_screen"),
            Rc::clone(&font),
            Arc::clone(&config_manager),
        )));
        let (tx, rx): (Sender<Vec<u8>>, Receiver<Vec<u8>>) = bounded(1);
        let mut screen_manager = screen_manager::ScreenManager::new(screens);
        let screen_descriptions = screen_manager.descriptions_and_keys_and_state();
        let this = AwesomeDisplay {
            screens: screen_manager,
            config_manager: config_manager.clone(),
            should_exit: false,
            sender: tx,
            current_screen: crate::screens::Screen::default(),
            screen_descriptions: screen_descriptions,
        };

        // global key press listener
        thread::spawn({
            move || loop {
                if let Err(error) = grab(callback) {
                    eprintln!("Error: {:?}", error)
                }
            }
        });

        // write to serial port ... since it is blocking, we'll just do this in a different thread
        thread::spawn(move || loop {
            let buf = rx.recv();
            if SERIAL_PORT.lock().unwrap().is_none() {
                *TEENSY_CONNECTED.lock().unwrap() = false;
                *SERIAL_PORT.lock().unwrap() = init_serial();
                if SERIAL_PORT.lock().unwrap().is_some() {
                    *TEENSY_CONNECTED.lock().unwrap() = true;
                    reset_display(
                        &mut *SERIAL_PORT.lock().unwrap(),
                        Duration::from_millis(500),
                    );
                }
            } else {
                match buf {
                    Ok(b) => {
                        let mut screen_buffer = b;
                        if CLOSE_REQUESTED.load(std::sync::atomic::Ordering::Acquire) {
                            return;
                        }
                        if *HIBERNATING.lock().unwrap() {
                            screen_buffer = vec![0; screen_buffer.len() as usize];
                        }
                        if !write_screen_buffer(&mut *SERIAL_PORT.lock().unwrap(), &screen_buffer) {
                            *SERIAL_PORT.lock().unwrap() = None;
                        }
                    }
                    Err(_) => {}
                }
            }
        });
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
        match message {
            Message::SaveConfig => {
                self.config_manager.write().unwrap().save();
            }
            Message::NextScreen => {
                self.screens.update_current_screen();
                self.screens.next_screen();
                self.screens.update_current_screen();
            }
            Message::PreviousScreen => {
                self.screens.update_current_screen();
                self.screens.previous_screen();
                self.screens.update_current_screen();
            }
            Message::UpdateCurrentScreen => {
                if *LAST_KEY.lock().unwrap() {
                    *LAST_KEY.lock().unwrap() = false;
                    let val = *LAST_KEY_VALUE.lock().unwrap();
                    if val == 174 || val == 175 {
                        // 1 is "volume mode"
                        self.screens
                            .set_screen_for_short("media_info_screen".into(), 1);
                    } else if val >= 176 && val < 180 {
                        // 0 is "normal mode"
                        self.screens
                            .set_screen_for_short("media_info_screen".into(), 0);
                    } else if val == 180 {
                        self.screens.next_screen()
                    }
                    *LAST_KEY_VALUE.lock().unwrap() = 0;
                }
                self.screens.update_current_screen();
            }
            Message::KeyboardEventOccurred(_event, key_code) => {
                // switch to media screen for a few seconds
                *LAST_KEY.lock().unwrap() = true;
                *LAST_KEY_VALUE.lock().unwrap() = key_code;
                self.screens.update_current_screen();
            }
            Message::WindowEventOccurred(event) => {
                if let iced_native::Event::Window(iced_native::window::Event::CloseRequested) =
                    event
                {
                    CLOSE_REQUESTED.store(true, std::sync::atomic::Ordering::Release);
                    if SERIAL_PORT.lock().unwrap().is_some() {
                        reset_display(&mut *SERIAL_PORT.lock().unwrap(), Duration::from_millis(0));
                    }
                    self.config_manager.write().unwrap().save();
                    self.should_exit = true;
                }
            }
            Message::SliderChanged(slider_value) => {
                self.config_manager.write().unwrap().config.brightness = slider_value as u16;
            }
            Message::ScreenStatusChanged(status, screen) => {
                if self.screens.screen_deactivatable(&screen) {
                    self.screens.set_status_for_screen(&screen, status);
                }
            }
            Message::BitpandaApiKeyChanged(message) => {
                self.config_manager.write().unwrap().config.bitpanda_api_key = message;
            }
            Message::OpenWeatherApiKeyChanged(message) => {
                self.config_manager
                    .write()
                    .unwrap()
                    .config
                    .openweather_api_key = message;
            }
            Message::OpenWeatherLocationChanged(message) => {
                self.config_manager
                    .write()
                    .unwrap()
                    .config
                    .openweather_location = message;
            }
        }

        self.current_screen.description = self.screens.current_screen().description().clone();
        self.current_screen.bytes = self.screens.current_screen().current_image().clone();
        self.screen_descriptions = self.screens.descriptions_and_keys_and_state().clone();
        if self.should_exit {
            return window::close();
        }
        Command::none()
    }

    fn theme(&self) -> iced::Theme {
        //return iced::Theme::Dark;

        return iced::Theme::custom(iced::theme::Palette {
            background: iced::Color::from_rgb(0.21, 0.22, 0.247),
            text: iced::Color::WHITE,
            primary: iced::Color::from_rgb(114.0 / 255.0, 137.0 / 255.0, 218.0 / 255.0),
            success: iced::Color::from_rgb(0.0, 1.0, 0.0),
            danger: iced::Color::from_rgb(1.0, 0.0, 0.0),
        });
    }

    fn view(&self) -> Element<Message> {
        // RENDER IN APP
        let screen_buffer = self.current_screen.bytes.clone();
        let mut converted_sb_rgba = Vec::with_capacity(65536);
        let mut converted_sb_rgb = Vec::with_capacity(49152);
        let set_brightness = |chunk_param: u8| {
            (chunk_param as f32 * self.config_manager.read().unwrap().config.brightness as f32
                / 100.0) as u8
        };

        for chunk in screen_buffer.chunks(3) {
            let chunk_two = set_brightness(chunk[2]);
            let chunk_one = set_brightness(chunk[1]);
            let chunk_zero = set_brightness(chunk[0]);
            // for preview
            converted_sb_rgba.push(chunk[2]);
            converted_sb_rgba.push(chunk[1]);
            converted_sb_rgba.push(chunk[0]);
            converted_sb_rgba.push(255);
            // for display
            converted_sb_rgb.push(chunk_two);
            converted_sb_rgb.push(chunk_one);
            converted_sb_rgb.push(chunk_zero);
        }
        let image = Image::new(iced::widget::image::Handle::from_pixels(
            256,
            64,
            converted_sb_rgba,
        ));

        // SEND TO DISPLAY
        let bytes = convert_to_gray_scale(&converted_sb_rgb);
        match self.sender.try_send(bytes.clone()) {
            Ok(_) => {}
            Err(_) => {}
        }

        let convert_brightness = |value: u16| {
            let old_range = 100f32 - 20f32;
            let new_range = 100f32;
            let new_value = ((value as f32 - 20f32) * new_range) / old_range;
            return new_value;
        };

        let mut column_parts: Vec<iced_native::Element<Message, iced::Renderer>> = vec![
            iced::widget::button(
                Text::new("Next screen").horizontal_alignment(iced::alignment::Horizontal::Center),
            )
            .on_press(Message::NextScreen)
            .width(Length::Units(200))
            .into(),
            iced::widget::button(
                Text::new("Previous screen")
                    .horizontal_alignment(iced::alignment::Horizontal::Center),
            )
            .on_press(Message::PreviousScreen)
            .width(Length::Units(200))
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
            .width(Length::Units(190))
            .step(0.1)
            .into(),
        ];

        // insert screens into left column menu
        for screen in self.screen_descriptions.clone().into_iter() {
            column_parts.push(special_checkbox(screen.2, screen.1.into(), screen.0.into()).into());
        }

        let mut left_column_after_screens: Vec<iced_native::Element<Message, iced::Renderer>> = vec![
            iced::widget::text_input(
                "Bitpanda Api Key",
                &self.config_manager.read().unwrap().config.bitpanda_api_key,
                Message::BitpandaApiKeyChanged,
            )
            .password()
            .width(Length::Units(190))
            .style(iced::theme::TextInput::Custom(Box::new(
                style::TextInput {},
            )))
            .into(),
            iced::widget::TextInput::new(
                "Openweather Api Key",
                &self
                    .config_manager
                    .read()
                    .unwrap()
                    .config
                    .openweather_api_key,
                Message::OpenWeatherApiKeyChanged,
            )
            .style(iced::theme::TextInput::Custom(Box::new(
                style::TextInput {},
            )))
            .width(Length::Units(190))
            .password()
            .into(),
            iced::widget::TextInput::new(
                "Openweather Location",
                &self
                    .config_manager
                    .read()
                    .unwrap()
                    .config
                    .openweather_location,
                Message::OpenWeatherLocationChanged,
            )
            .style(iced::theme::TextInput::Custom(Box::new(
                style::TextInput {},
            )))
            .width(Length::Units(190))
            .into(),
            iced::widget::button(
                Text::new("Save config").horizontal_alignment(iced::alignment::Horizontal::Center),
            )
            .width(Length::Units(200))
            .on_press(Message::SaveConfig)
            .into(),
            iced::widget::Text::new(if *TEENSY_CONNECTED.lock().unwrap() {
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
            .push(iced::widget::text(self.current_screen.description.clone()).size(25))
            .push(image.width(Length::Units(256)).height(Length::Units(64)));

        iced_native::widget::Row::new().push(col1).push(col2).into()
    }
}

fn callback(event: Event) -> Option<Event> {
    match event.event_type {
        EventType::KeyPress(Key::Unknown(178)) => {
            *LAST_KEY.lock().unwrap() = true;
            *LAST_KEY_VALUE.lock().unwrap() = 178;
            Some(event)
        }
        EventType::KeyPress(Key::Unknown(177)) => {
            *LAST_KEY.lock().unwrap() = true;
            *LAST_KEY_VALUE.lock().unwrap() = 177;
            Some(event)
        }
        EventType::KeyPress(Key::Unknown(176)) => {
            *LAST_KEY.lock().unwrap() = true;
            *LAST_KEY_VALUE.lock().unwrap() = 176;
            Some(event)
        }
        EventType::KeyPress(Key::Unknown(175)) => {
            *LAST_KEY.lock().unwrap() = true;
            *LAST_KEY_VALUE.lock().unwrap() = 175;
            Some(event)
        }
        EventType::KeyPress(Key::Unknown(174)) => {
            *LAST_KEY.lock().unwrap() = true;
            *LAST_KEY_VALUE.lock().unwrap() = 174;
            Some(event)
        }
        EventType::KeyPress(Key::Unknown(173)) => {
            *LAST_KEY.lock().unwrap() = true;
            *LAST_KEY_VALUE.lock().unwrap() = 173;
            Some(event)
        }
        EventType::KeyPress(Key::Unknown(179)) => {
            *LAST_KEY.lock().unwrap() = true;
            *LAST_KEY_VALUE.lock().unwrap() = 179;
            Some(event)
        }
        EventType::KeyPress(Key::Pause) => {
            *LAST_KEY.lock().unwrap() = true;
            *LAST_KEY_VALUE.lock().unwrap() = 180;
            Some(event)
        }
        _ => Some(event),
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
    .width(Length::Units(200))
    .into()
}

pub unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: UINT,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg == WM_POWERBROADCAST {
        *HIBERNATING.lock().unwrap() = wparam == PBT_APMSUSPEND;
    }

    if msg == WM_DESTROY {
        PostQuitMessage(0);
        return 0;
    }

    return DefWindowProcW(hwnd, msg, wparam, lparam);
}
