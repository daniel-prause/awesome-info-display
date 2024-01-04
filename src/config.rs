use std::collections::HashMap;

use exchange_format::ConfigParam;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    pub brightness: u16,
    pub companion_brightness: u16,
    pub screens: HashMap<String, ScreenConfig>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ScreenConfig {
    pub active: bool,
    pub config_attributes: HashMap<String, ConfigParam>,
}

impl Config {
    pub fn set_screen_active(&mut self, screen: String, status: bool) {
        match self.screens.get_mut(&screen) {
            Some(config) => {
                config.active = status;
            }
            None => {
                let new_config = ScreenConfig {
                    active: true,
                    config_attributes: HashMap::new(),
                };
                self.screens.insert(screen, new_config);
            }
        }
    }

    pub fn set_screen_value(&mut self, screen: String, key: String, value: ConfigParam) {
        match self.screens.get_mut(&screen) {
            Some(config) => {
                config.config_attributes.insert(key, value);
            }
            None => {
                let mut new_config = ScreenConfig {
                    active: true,
                    config_attributes: HashMap::new(),
                };
                new_config.config_attributes.insert(key, value);
                self.screens.insert(screen, new_config);
            }
        };
    }
}
