#![windows_subsystem = "windows"]
use iced::{
    button, executor, slider, time, window, Align, Application, Button, Checkbox, Column, Command,
    Container, Element, HorizontalAlignment, Image, Length, Row, Settings, Slider, Subscription,
    Text,
};
use std::io::{self, Write};
mod awesome_display_config;
mod bitpanda_screen;
mod display_serial_com;
mod media_info_screen;
mod screen;
mod screen_manager;
mod style;
mod system_info_screen;
use crate::display_serial_com::{convert_to_gray_scale, init_serial, write_screen_buffer};
use lazy_static::lazy_static;
use rdev::{grab, Event, EventType, Key};
use rusttype::Font;
use std::ffi::CString;
use std::sync::Mutex;
use std::thread;

use std::error::Error;
use std::fmt;

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

lazy_static! {
    static ref LAST_KEY: Mutex<bool> = Mutex::new(false);
    static ref LAST_KEY_VALUE: Mutex<u32> = Mutex::new(0);
    static ref SERIAL_PORT: Mutex<Option<Box<dyn serialport::SerialPort>>> = Mutex::new(None);
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
    theme: style::Theme,
    increment_button: button::State,
    decrement_button: button::State,
    save_config_button: button::State,
    screens: screen_manager::ScreenManager,
    config: awesome_display_config::AwesomeDisplayConfig,
    should_exit: bool,
    slider: slider::State,
}

#[derive(Debug, Clone)]
enum Message {
    NextScreen,
    PreviousScreen,
    UpdateCurrentScreen,
    SaveConfig,
    SliderChanged(f32),
    SystemInfoScreenStatusChanged(bool),
    MediaScreenStatusChanged(bool),
    BitpandaInfoStatusChanged(bool),
    KeyboardEventOccurred(iced::keyboard::KeyCode, u32),
    WindowEventOccurred(iced_native::Event),
}

impl Application for AwesomeDisplay {
    type Executor = executor::Default;
    type Message = Message;
    type Flags = ();
    fn new(_flags: ()) -> (AwesomeDisplay, Command<Message>) {
        let font = Font::try_from_vec(Vec::from(include_bytes!("Liberation.ttf") as &[u8]));
        let builder = thread::Builder::new().name("JOB_EXECUTOR".into());
        let config = awesome_display_config::AwesomeDisplayConfig::new("./settings.json");
        let mut screens: Vec<Box<dyn screen::SpecificScreen>> = Vec::new();

        screens.push(Box::new(system_info_screen::SystemInfoScreen::new(
            String::from("System Stats"),
            font.clone(),
            std::sync::Arc::new(std::sync::atomic::AtomicBool::new(
                config.system_info_screen_active,
            )),
        )));
        screens.push(Box::new(media_info_screen::MediaInfoScreen::new(
            String::from("Media Stats"),
            font.clone(),
            std::sync::Arc::new(std::sync::atomic::AtomicBool::new(
                config.media_screen_active,
            )),
        )));
        screens.push(Box::new(bitpanda_screen::BitpandaScreen::new(
            String::from("Bitpanda Info"),
            font.clone(),
            config.bitpanda_api_key.to_string(),
            std::sync::Arc::new(std::sync::atomic::AtomicBool::new(
                config.bitpanda_screen_active,
            )),
        )));

        let this = AwesomeDisplay {
            increment_button: button::State::new(),
            decrement_button: button::State::new(),
            save_config_button: button::State::new(),
            theme: style::Theme::Dark,
            screens: screen_manager::ScreenManager::new(screens),
            config: config,
            should_exit: false,
            slider: slider::State::new(),
        };
        builder
            .spawn({
                move || loop {
                    if let Err(error) = grab(callback) {
                        println!("Error: {:?}", error)
                    }
                }
            })
            .expect("Cannot create JOB_EXECUTOR thread");

        thread::spawn(move || loop {
            let initialized = SERIAL_PORT.lock().unwrap().is_some();
            if !initialized {
                let port = init_serial();
                if port.is_some() {
                    *SERIAL_PORT.lock().unwrap() = port;
                }
            } else {
                thread::sleep(std::time::Duration::from_millis(1000));
            }
        });
        (this, Command::none())
    }
    fn title(&self) -> String {
        String::from("AwesomeInfoDisplay")
    }

    fn should_exit(&self) -> bool {
        self.should_exit
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

    fn update(&mut self, message: Message, _clipboard: &mut iced::Clipboard) -> Command<Message> {
        match message {
            Message::SaveConfig => {
                self.config.save("./settings.json");
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
                    if val > 173 && val < 176 {
                        self.screens.set_screen_for_short(1, 1); // 1 is media screen right now, 1 is "volume mode"
                    } else {
                        self.screens.set_screen_for_short(1, 0); // 1 is media screen right now, 0 is "normal mode"
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
                    self.config.save("./settings.json");
                    self.should_exit = true;
                }
            }
            Message::SliderChanged(slider_value) => {
                self.config.brightness = slider_value as u16;
            }
            Message::SystemInfoScreenStatusChanged(status) => {
                if self.config.media_screen_active || self.config.bitpanda_screen_active {
                    self.config.system_info_screen_active = status;
                    self.screens.set_status_for_screen(0, status);
                }
            }
            Message::MediaScreenStatusChanged(status) => {
                if self.config.system_info_screen_active || self.config.bitpanda_screen_active {
                    self.config.media_screen_active = status;
                    self.screens.set_status_for_screen(1, status);
                }
            }
            Message::BitpandaInfoStatusChanged(status) => {
                if self.config.system_info_screen_active || self.config.media_screen_active {
                    self.config.bitpanda_screen_active = status;
                    self.screens.set_status_for_screen(2, status);
                }
            }
        }
        Command::none()
    }

    fn view(&mut self) -> Element<Message> {
        //io::stdout().flush().unwrap();
        if !self.screens.current_screen().initial_update_called() {
            self.screens.update_current_screen();
        }
        // RENDER IN APP
        let screen_buffer = self.screens.current_screen().current_image();
        let mut converted_sb = Vec::new();
        for chunk in screen_buffer.chunks(3) {
            converted_sb.push((chunk[2] as f32 * self.config.brightness as f32 / 100.0) as u8);
            converted_sb.push((chunk[1] as f32 * self.config.brightness as f32 / 100.0) as u8);
            converted_sb.push((chunk[0] as f32 * self.config.brightness as f32 / 100.0) as u8);
            converted_sb.push(255);
        }
        let image = Image::new(iced::image::Handle::from_pixels(256, 64, converted_sb));

        // SEND TO DISPLAY
        let bytes = &self.screens.current_screen().current_image();
        let bytes = convert_to_gray_scale(bytes);
        if !write_screen_buffer(&mut *SERIAL_PORT.lock().unwrap(), &bytes) {
            *SERIAL_PORT.lock().unwrap() = None;
        }

        let col1 = Column::new()
            .padding(20)
            .align_items(Align::Center)
            .spacing(10)
            .push(
                Button::new(
                    &mut self.increment_button,
                    Text::new("Next screen").horizontal_alignment(HorizontalAlignment::Center),
                )
                .style(self.theme)
                .width(Length::Units(200))
                .on_press(Message::NextScreen),
            )
            .push(
                Button::new(
                    &mut self.decrement_button,
                    Text::new("Previous screen").horizontal_alignment(HorizontalAlignment::Center),
                )
                .style(self.theme)
                .width(Length::Units(200))
                .on_press(Message::PreviousScreen),
            )
            .push(Text::new(format!(
                "Brightness: {:.2}",
                self.config.brightness
            )))
            .push(
                Slider::new(
                    &mut self.slider,
                    0.0..=100.0,
                    self.config.brightness as f32,
                    Message::SliderChanged,
                )
                .width(Length::Units(200))
                .step(0.1),
            )
            .push(
                Checkbox::new(
                    self.config.system_info_screen_active,
                    "System Stats",
                    Message::SystemInfoScreenStatusChanged,
                )
                .width(Length::Units(200)),
            )
            .push(
                Checkbox::new(
                    self.config.media_screen_active,
                    "Media Stats",
                    Message::MediaScreenStatusChanged,
                )
                .width(Length::Units(200)),
            )
            .push(
                Checkbox::new(
                    self.config.bitpanda_screen_active,
                    "Bitpanda Info",
                    Message::BitpandaInfoStatusChanged,
                )
                .width(Length::Units(200)),
            )
            .push(
                Button::new(
                    &mut self.save_config_button,
                    Text::new("Save config").horizontal_alignment(HorizontalAlignment::Center),
                )
                .style(self.theme)
                .width(Length::Units(200))
                .on_press(Message::SaveConfig),
            );

        let col2 = Column::new()
            .padding(20)
            .align_items(Align::Center)
            .width(Length::Fill)
            .push(Text::new("Current screen").size(50))
            .push(Text::new(self.screens.current_screen().description()).size(25))
            .push(image.width(Length::Units(256)).height(Length::Units(64)));

        Container::new(Row::new().push(col1).push(col2))
            .width(Length::Fill)
            .height(Length::Fill)
            .style(self.theme)
            .into()
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
        _ => Some(event),
    }
}
