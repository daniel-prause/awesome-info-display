use std::time::Instant;
use std::time::{Duration, SystemTime};

#[derive(Debug, Default)]
pub struct ScreenManager {
    screens: Vec<Box<dyn super::screen::SpecificScreen>>,
    current: usize,
    timeout: Option<std::time::Instant>,
    last_screen: usize,
}

impl ScreenManager {
    pub fn new(screens: Vec<Box<dyn super::screen::SpecificScreen>>) -> Self {
        let this = ScreenManager {
            screens: screens,
            current: 0,
            timeout: Some(Instant::now()),
            last_screen: 0,
        };
        this.screens[this.current].start();
        this
    }

    pub fn current_screen(&mut self) -> &mut Box<dyn super::screen::SpecificScreen> {
        if self.screens.get(self.current).is_none() {
            None.unwrap()
        } else {
            let five = Duration::from_secs(5);
            if self.timeout.unwrap().elapsed() >= five {
                self.current = self.last_screen;
            }
            self.screens[self.current].start();
            &mut self.screens[self.current]
        }
    }

    pub fn next_screen(&mut self) {
        self.current_screen().stop();
        if self.current == self.screens.len() - 1 {
            self.current = 0;
        } else {
            self.current += 1
        }
        self.current_screen().start();
    }

    pub fn previous_screen(&mut self) {
        self.current_screen().stop();
        if self.current == 0 {
            self.current = self.screens.len() - 1;
        } else {
            self.current -= 1
        }
        self.current_screen().start();
    }

    pub fn update_current_screen(&mut self) {
        self.current_screen().update();
    }

    pub fn set_screen_for_short(&mut self, screen: usize) {
        self.timeout = Some(Instant::now());
        self.current = screen;
    }
}
