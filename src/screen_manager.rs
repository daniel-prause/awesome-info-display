#[derive(Debug, Default)]
pub struct ScreenManager {
    screens: Vec<Box<dyn super::screen::SpecificScreen>>,
    current: usize,
}

impl ScreenManager {
    pub fn new(screens: Vec<Box<dyn super::screen::SpecificScreen>>) -> Self {
        ScreenManager {
            screens: screens,
            current: 0,
        }
    }

    pub fn current_screen(&mut self) -> &mut Box<dyn super::screen::SpecificScreen> {
        if self.screens.get(self.current).is_none() {
            None.unwrap()
        } else {
            &mut self.screens[self.current]
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

    pub fn update_current_screen(&mut self) {
        self.current_screen().update();
    }
}
