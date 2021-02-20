#[derive(Debug, Clone, Default)]
pub struct ScreenManager {
    screens: Vec<super::screen::Screen>,
    current: usize,
}

impl ScreenManager {
    pub fn new(screens: Vec<super::screen::Screen>) -> Self {
        ScreenManager {
            screens: screens,
            current: 0,
        }
    }

    pub fn current_screen(&mut self) -> &super::screen::Screen {
        if self.screens.get(self.current).is_none() {
            None.unwrap()
        } else {
            return self.screens.get(self.current).unwrap();
        }
    }

    pub fn next_screen(&mut self) {
        if self.current == self.screens.len() - 1 {
            self.current = 0;
        } else {
            self.current += 1
        }
    }

    pub fn previous_screen(&mut self) {
        if self.current == 0 {
            self.current = self.screens.len() - 1;
        } else {
            self.current -= 1
        }
    }
}
