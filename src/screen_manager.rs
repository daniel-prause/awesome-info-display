use std::time::Duration;
use std::time::Instant;

pub struct ScreenManager {
    screens: Vec<Box<dyn super::screen::BasicScreen>>,
    current: usize,
    timeout: Option<std::time::Instant>,
    last_screen: usize,
    switch_in_progress: bool,
}

impl ScreenManager {
    pub fn new(screens: Vec<Box<dyn super::screen::BasicScreen>>) -> Self {
        let mut this = ScreenManager {
            screens: screens,
            current: 0,
            timeout: Some(Instant::now()),
            last_screen: 0,
            switch_in_progress: false,
        };
        // find first enabled screen
        if !this.screens[this.current].enabled() {
            for (i, screen) in this.screens.iter().enumerate() {
                if screen.enabled() {
                    this.current = i;
                    this.last_screen = i;
                    break;
                }
            }
        }
        this
    }

    pub fn current_screen(&mut self) -> &mut Box<dyn super::screen::BasicScreen> {
        if self.screens.get(self.current).is_some() {
            let seconds = Duration::from_secs(3);
            if self.switch_in_progress && self.timeout.unwrap().elapsed() >= seconds {
                self.screens[self.current].update();
                self.current = self.last_screen;
                self.switch_in_progress = false;
            } else {
                self.screens[self.current].start();
            }
            return &mut self.screens[self.current];
        }
        // this should never happen...
        panic!("No current screen!");
    }

    pub fn next_screen(&mut self) {
        self.current_screen().stop();
        self.switch_in_progress = false;
        self.find_next_enabled_screen();
        self.current_screen().start();
    }

    pub fn previous_screen(&mut self) {
        self.current_screen().stop();
        self.switch_in_progress = false;
        self.find_previous_enabled_screen();
        self.current_screen().start();
    }

    pub fn update_current_screen(&mut self) {
        self.current_screen().update();
    }

    pub fn set_screen_for_short(&mut self, screen: usize, mode: u32) {
        if !self.screens[screen].enabled() {
            return;
        }
        self.timeout = Some(Instant::now());
        if !self.switch_in_progress {
            self.current_screen().stop();
            self.last_screen = self.current;
        }
        self.current = screen;
        self.current_screen().set_mode_for_short(mode); // right now, volume mode for 3 seconds for media screen
        self.current_screen().start();
        self.switch_in_progress = true;
    }

    fn find_previous_enabled_screen(&mut self) {
        loop {
            if self.current == 0 {
                self.current = self.screens.len() - 1;
            } else {
                self.current -= 1
            }
            if self.screens[self.current].enabled() {
                break;
            }
        }
    }

    fn find_next_enabled_screen(&mut self) {
        loop {
            if self.current == self.screens.len() - 1 {
                self.current = 0;
            } else {
                self.current += 1
            }
            if self.screens[self.current].enabled() {
                break;
            }
        }
    }

    pub fn set_status_for_screen(&mut self, screen: usize, status: bool) {
        self.switch_in_progress = false;
        self.screens[screen].set_status(status);
        if screen == self.current && !status {
            self.next_screen();
        }
    }

    pub fn screen_deactivatable(&self, key: String) -> bool {
        let mut count = 0;

        for screen in self.screens.iter() {
            if screen.enabled() && key != screen.key() {
                count += 1
            }
            if count >= 1 {
                return true;
            }
        }
        count >= 1
    }
}
