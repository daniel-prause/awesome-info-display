use exchange_format::ConfigParam;
use indexmap::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    pub brightness: u16,
    pub companion_brightness: u16,
    pub screens: HashMap<String, ScreenConfig>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ScreenConfig {
    pub active: bool,
    pub config_attributes: IndexMap<String, ConfigParam>,
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
                    config_attributes: IndexMap::new(),
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
                    config_attributes: IndexMap::new(),
                };
                new_config.config_attributes.insert(key, value);
                self.screens.insert(screen, new_config);
            }
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use exchange_format::ConfigParam;

    #[test]
    fn test_set_screen_active_existing_screen() {
        let mut config = Config {
            brightness: 100,
            companion_brightness: 100,
            screens: HashMap::new(),
        };

        let screen_name = "screen1".to_string();
        config.set_screen_active(screen_name.clone(), true);

        assert_eq!(config.screens.get(&screen_name).unwrap().active, true);
    }

    #[test]
    fn test_set_screen_active_new_screen() {
        let mut config = Config {
            brightness: 100,
            companion_brightness: 100,
            screens: HashMap::new(),
        };

        let screen_name = "screen1".to_string();
        config.set_screen_active(screen_name.clone(), true);

        assert_eq!(config.screens.len(), 1);
        assert_eq!(config.screens.get(&screen_name).unwrap().active, true);
    }

    #[test]
    fn test_set_screen_value_existing_screen() {
        let mut config = Config {
            brightness: 100,
            companion_brightness: 100,
            screens: HashMap::new(),
        };

        let screen_name = "screen1".to_string();
        let key = "key1".to_string();
        let value = ConfigParam::Integer(42);

        config.set_screen_active(screen_name.clone(), true);
        config.set_screen_value(screen_name.clone(), key.clone(), value.clone());

        assert!(matches!(
            config
                .screens
                .get(&screen_name)
                .unwrap()
                .config_attributes
                .get(&key),
            Some(ConfigParam::Integer(42))
        ));
    }

    #[test]
    fn test_set_screen_value_new_screen() {
        let mut config = Config {
            brightness: 100,
            companion_brightness: 100,
            screens: HashMap::new(),
        };

        let screen_name = "screen1".to_string();
        let key = "key1".to_string();
        let value = ConfigParam::Integer(42);

        config.set_screen_value(screen_name.clone(), key.clone(), value.clone());

        assert_eq!(config.screens.len(), 1);
        assert!(matches!(
            config
                .screens
                .get(&screen_name)
                .unwrap()
                .config_attributes
                .get(&key),
            Some(ConfigParam::Integer(42))
        ));
    }
}
