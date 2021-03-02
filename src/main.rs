//#![windows_subsystem = "windows"]
use iced::{
    button, executor, time, Align, Application, Button, Column, Command, Container, Element,
    HorizontalAlignment, Image, Length, Row, Settings, Subscription, Text,
};
use std::io::{self, Write};
mod media_info_screen;
mod screen;
mod screen_manager;
mod style;
mod system_info_screen;
use lazy_static::lazy_static;
use rdev::{grab, Event, EventType, Key};
use rusttype::Font;
use std::sync::Mutex;
use std::thread;
lazy_static! {
    static ref LAST_KEY: Mutex<bool> = Mutex::new(false);
}
pub fn main() -> iced::Result {
    AwesomeDisplay::run(Settings::default())
}

#[derive(Default)]
struct AwesomeDisplay {
    theme: style::Theme,
    increment_button: button::State,
    decrement_button: button::State,
    screens: screen_manager::ScreenManager,
}

#[derive(Debug, Clone)]
enum Message {
    NextScreen,
    PreviousScreen,
    UpdateCurrentScreen,
    EventOccurred(iced::keyboard::KeyCode),
}

impl Application for AwesomeDisplay {
    type Executor = executor::Default;
    type Message = Message;
    type Flags = ();
    fn new(_flags: ()) -> (AwesomeDisplay, Command<Message>) {
        let font = Font::try_from_vec(Vec::from(include_bytes!("Liberation.ttf") as &[u8]));
        let builder = thread::Builder::new().name("JOB_EXECUTOR".into());

        let this = AwesomeDisplay {
            increment_button: button::State::new(),
            decrement_button: button::State::new(),
            theme: style::Theme::Dark,
            screens: screen_manager::ScreenManager::new(vec![
                Box::new(system_info_screen::SystemInfoScreen::new(
                    String::from("System Stats"),
                    font.clone(),
                )),
                Box::new(media_info_screen::MediaInfoScreen::new(
                    String::from("Media Stats"),
                    font.clone(),
                )),
            ]),
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
                                Some(Message::EventOccurred(key_code))
                            }
                            iced::keyboard::KeyCode::MediaStop => {
                                Some(Message::EventOccurred(key_code))
                            }
                            iced::keyboard::KeyCode::PrevTrack => {
                                Some(Message::EventOccurred(key_code))
                            }
                            iced::keyboard::KeyCode::NextTrack => {
                                Some(Message::EventOccurred(key_code))
                            }
                            iced::keyboard::KeyCode::VolumeDown => {
                                Some(Message::EventOccurred(key_code))
                            }
                            iced::keyboard::KeyCode::VolumeUp => {
                                Some(Message::EventOccurred(key_code))
                            }
                            iced::keyboard::KeyCode::Mute => Some(Message::EventOccurred(key_code)),
                            _ => None,
                        },
                        _ => None,
                    }
                }),
                time::every(std::time::Duration::from_millis(500))
                    .map(|_| Message::UpdateCurrentScreen),
            ]
            .into_iter(),
        )
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::NextScreen => {
                self.screens.update_current_screen();
                self.screens.next_screen();
            }
            Message::PreviousScreen => {
                self.screens.update_current_screen();
                self.screens.previous_screen();
            }
            Message::UpdateCurrentScreen => {
                if *LAST_KEY.lock().unwrap() {
                    *LAST_KEY.lock().unwrap() = false;
                    self.screens.set_screen_for_short(1); // 1 is media screen right now
                }
                self.screens.update_current_screen();
            }
            Message::EventOccurred(_event) => {
                // switch to media screen for a few seconds
                *LAST_KEY.lock().unwrap() = true;
            }
        }
        Command::none()
    }

    fn view(&mut self) -> Element<Message> {
        io::stdout().flush().unwrap();
        if !self.screens.current_screen().initial_update_called() {
            self.screens.update_current_screen();
        }
        let image = Image::new(iced::image::Handle::from_memory(
            self.screens.current_screen().current_image(),
        ));

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
    //*this.lock().unwrap() = true;
    match event.event_type {
        EventType::KeyPress(Key::Unknown(178)) => {
            *LAST_KEY.lock().unwrap() = true;
            Some(event)
        }
        EventType::KeyPress(Key::Unknown(177)) => {
            *LAST_KEY.lock().unwrap() = true;
            Some(event)
        }
        EventType::KeyPress(Key::Unknown(176)) => {
            *LAST_KEY.lock().unwrap() = true;
            Some(event)
        }
        EventType::KeyPress(Key::Unknown(175)) => {
            *LAST_KEY.lock().unwrap() = true;
            Some(event)
        }
        EventType::KeyPress(Key::Unknown(174)) => {
            *LAST_KEY.lock().unwrap() = true;
            Some(event)
        }
        EventType::KeyPress(Key::Unknown(173)) => {
            *LAST_KEY.lock().unwrap() = true;
            Some(event)
        }
        EventType::KeyPress(Key::Unknown(179)) => {
            *LAST_KEY.lock().unwrap() = true;
            Some(event)
        }
        _ => Some(event),
    }
}
