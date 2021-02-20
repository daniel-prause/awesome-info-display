#![windows_subsystem = "windows"]
use iced::{
    button, executor, Align, Application, Button, Column, Command, Container, Element,
    HorizontalAlignment, Length, Row, Settings, Text,
};
mod screen;
mod screen_manager;

mod style;
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

#[derive(Debug, Clone, Copy)]
enum Message {
    IncrementPressed,
    DecrementPressed,
}

impl Application for AwesomeDisplay {
    type Executor = executor::Default;
    type Message = Message;
    type Flags = ();
    fn new(_flags: ()) -> (AwesomeDisplay, Command<Message>) {
        //let mut vec = Vec::new();
        //vec.push(screen::Screen::new(String::from("Screen 1")));

        (
            AwesomeDisplay {
                increment_button: button::State::new(),
                decrement_button: button::State::new(),
                theme: style::Theme::Dark,
                screens: screen_manager::ScreenManager::new(vec![
                    screen::Screen::new(String::from("Screen 1")),
                    screen::Screen::new(String::from("Screen 2")),
                    screen::Screen::new(String::from("Screen 3")),
                    screen::Screen::new(String::from("Screen 4")),
                ]),
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("AwesomeInfoDisplay")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::IncrementPressed => {
                self.screens.next_screen();
                // self.current_screen
                //     .update(screen::ScreenMessage::UpdateScreen);
            }
            Message::DecrementPressed => {
                self.screens.previous_screen();
                //self.current_screen = getCurrentScreen();
                // self.current_screen
                //     .update(screen::ScreenMessage::UpdateScreen);
            }
        }

        Command::none()
    }

    fn view(&mut self) -> Element<Message> {
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
                .on_press(Message::IncrementPressed),
            )
            .push(
                Button::new(
                    &mut self.decrement_button,
                    Text::new("Previous screen").horizontal_alignment(HorizontalAlignment::Center),
                )
                .style(self.theme)
                .width(Length::Units(200))
                .on_press(Message::DecrementPressed),
            );

        let col2 = Column::new()
            .padding(20)
            .align_items(Align::Center)
            .width(Length::Fill)
            .push(Text::new("Current screen").size(50))
            .push(Text::new(self.screens.current_screen().description()).size(50));

        Container::new(Row::new().push(col1).push(col2))
            .width(Length::Fill)
            .height(Length::Fill)
            .style(self.theme)
            .into()
    }
}
