use std::time::Duration;
use std::time::Instant;

use exchange_format::ExchangeableConfig;

pub struct ScreenManager {
    screens: Vec<Box<dyn super::screens::BasicScreen>>,
    current: usize,
    timeout: Option<std::time::Instant>,
    last_screen: usize,
    switch_in_progress: bool,
}

impl ScreenManager {
    pub fn new(screens: Vec<Box<dyn super::screens::BasicScreen>>) -> Self {
        let mut this = ScreenManager {
            screens,
            current: 0,
            timeout: Some(Instant::now()),
            last_screen: 0,
            switch_in_progress: false,
        };

        if !this.screens[this.current].enabled() {
            match this.screens.iter_mut().position(|r| r.enabled()) {
                Some(idx) => {
                    this.current = idx;
                    this.last_screen = idx;
                }
                None => {}
            };
        }
        this
    }

    pub fn current_screen(&mut self) -> &mut Box<dyn super::screens::BasicScreen> {
        if self.screens.get(self.current).is_some() {
            let seconds = Duration::from_secs(3);
            if self.switch_in_progress
                && self.timeout.unwrap_or(Instant::now()).elapsed() >= seconds
            {
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

    pub fn set_screen_for_short(&mut self, key: String, mode: u32) {
        let index: usize = match self.screens.iter_mut().position(|r| *r.key() == key) {
            Some(idx) => idx,
            None => return,
        };

        if !self.screens[index].enabled() {
            return;
        }
        self.timeout = Some(Instant::now());
        if !self.switch_in_progress {
            self.current_screen().stop();
            self.last_screen = self.current;
        }
        self.current = index;
        self.current_screen().set_mode(mode); // right now, volume mode for 3 seconds for media screen
        self.current_screen().start();
        self.switch_in_progress = true;
    }

    fn find_previous_enabled_screen(&mut self) {
        loop {
            self.current = if self.current == 0 {
                self.screens.len() - 1
            } else {
                self.current - 1
            };
            if self.screens[self.current].enabled() {
                break;
            }
        }
    }

    fn find_next_enabled_screen(&mut self) {
        loop {
            self.current = (self.current + 1) % self.screens.len();
            if self.screens[self.current].enabled() {
                break;
            }
        }
    }

    pub fn set_status_for_screen(&mut self, key: &String, status: bool) {
        self.switch_in_progress = false;

        for screen in self.screens.iter_mut() {
            if *screen.key() == *key {
                screen.set_status(status)
            }
        }
        if (*key == *self.screens[self.current].key()) && !status {
            self.next_screen();
        }
    }

    pub fn screen_deactivatable(&mut self, key: &String) -> bool {
        let mut count = 0;

        for screen in self.screens.iter_mut() {
            if screen.enabled() && *key != *screen.key() {
                count += 1
            }
            if count >= 1 {
                return true;
            }
        }
        count >= 1
    }

    pub fn update_screen_config(&mut self, key: &String, config: ExchangeableConfig) {
        for screen in self.screens.iter_mut() {
            if *key == *screen.key() {
                screen.set_current_config(config.clone())
            }
        }
    }

    pub fn descriptions_and_keys_and_state(&mut self) -> Vec<(String, String, bool)> {
        let mut result = Vec::<(String, String, bool)>::new();
        for screen in self.screens.iter_mut() {
            result.push((screen.description(), screen.key(), screen.enabled()))
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use crate::screens::{BasicScreen, Screen};

    use super::*;
    use exchange_format::ExchangeableConfig;

    struct MockScreen {
        key: String,
        enabled: bool,
        screen: Screen,
    }

    impl super::super::screens::BasicScreen for MockScreen {
        fn key(&mut self) -> String {
            self.key.clone()
        }

        fn enabled(&mut self) -> bool {
            self.enabled
        }

        fn start(&mut self) {}

        fn stop(&mut self) {}

        fn update(&mut self) {}

        fn set_mode(&mut self, _mode: u32) {}

        fn set_status(&mut self, _status: bool) {}

        fn description(&mut self) -> String {
            String::from("Mock Screen")
        }

        fn set_current_config(&mut self, _config: ExchangeableConfig) {}
    }
    impl super::super::screens::Screenable for MockScreen {
        fn get_screen(&mut self) -> &mut Screen {
            &mut self.screen
        }
    }

    #[test]
    fn test_create_screen_manager_with_enabled_screen() {
        let screens: Vec<Box<dyn BasicScreen>> = vec![
            Box::new(MockScreen {
                key: String::from("screen1"),
                enabled: false,
                screen: Screen::default(),
            }),
            Box::new(MockScreen {
                key: String::from("screen2"),
                enabled: true,
                screen: Screen::default(),
            }),
        ];

        let screen_manager = ScreenManager::new(screens);

        assert_eq!(screen_manager.current, 1);
    }

    #[test]
    fn test_next_screen() {
        let screens: Vec<Box<dyn BasicScreen>> = vec![
            Box::new(MockScreen {
                key: String::from("screen1"),
                enabled: true,
                screen: Screen::default(),
            }),
            Box::new(MockScreen {
                key: String::from("screen2"),
                enabled: true,
                screen: Screen::default(),
            }),
        ];

        let mut screen_manager = ScreenManager::new(screens);

        assert_eq!(screen_manager.current, 0);
        screen_manager.next_screen();
        assert_eq!(screen_manager.current, 1);
    }

    #[test]
    fn test_previous_screen() {
        let screens: Vec<Box<dyn BasicScreen>> = vec![
            Box::new(MockScreen {
                key: String::from("screen1"),
                enabled: true,
                screen: Screen::default(),
            }),
            Box::new(MockScreen {
                key: String::from("screen2"),
                enabled: true,
                screen: Screen::default(),
            }),
        ];

        let mut screen_manager = ScreenManager::new(screens);

        assert_eq!(screen_manager.current, 0);
        screen_manager.previous_screen();
        assert_eq!(screen_manager.current, 1);
    }

    #[test]
    fn test_no_next_screen() {
        let screens: Vec<Box<dyn BasicScreen>> = vec![
            Box::new(MockScreen {
                key: String::from("screen1"),
                enabled: true,
                screen: Screen::default(),
            }),
            Box::new(MockScreen {
                key: String::from("screen2"),
                enabled: false,
                screen: Screen::default(),
            }),
        ];

        let mut screen_manager = ScreenManager::new(screens);

        assert_eq!(screen_manager.current, 0);
        screen_manager.next_screen();
        assert_eq!(screen_manager.current, 0);
    }

    #[test]
    fn test_no_previous_screen() {
        let screens: Vec<Box<dyn BasicScreen>> = vec![
            Box::new(MockScreen {
                key: String::from("screen1"),
                enabled: true,
                screen: Screen::default(),
            }),
            Box::new(MockScreen {
                key: String::from("screen2"),
                enabled: false,
                screen: Screen::default(),
            }),
        ];

        let mut screen_manager = ScreenManager::new(screens);

        assert_eq!(screen_manager.current, 0);
        screen_manager.previous_screen();
        assert_eq!(screen_manager.current, 0);
    }
}
