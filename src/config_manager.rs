use crate::config::{Config, ScreenConfig};

use exchange_format::ConfigParam;
use indexmap::*;
use serde::{Deserialize, Serialize};

use std::{
    collections::HashMap,
    fs::{self, File},
};

#[derive(Serialize, Deserialize, Clone)]
pub struct ConfigManager {
    pub config: Config,
    config_hash: String,
    config_path: String,
}

impl ConfigManager {
    pub fn new(filepath: Option<&str>) -> Self {
        let filepath = filepath.unwrap_or("./settings.json").to_string();
        let config = Config {
            brightness: 100,
            companion_brightness: 100,
            screens: HashMap::new(),
        };
        let mut this = ConfigManager {
            config,
            config_hash: String::new(),
            config_path: filepath.clone(),
        };

        // check, if file exists; if not -> create it
        let file_exists = std::path::Path::new(&filepath.clone()).exists();
        if !file_exists {
            match File::create(&filepath) {
                Ok(_) => {
                    println!("Settings file created!")
                }
                Err(err) => {
                    eprintln!("Error creating file: {}", err);
                }
            }
        }
        let contents = fs::read_to_string(&filepath);
        match contents {
            Ok(config) => {
                this.config_hash = config.clone();
                this.config = serde_json::from_str(&config).unwrap_or(this.config);
            }
            Err(e) => {
                eprintln!("Error: {:?}", e);
            }
        }
        this
    }

    pub fn save(&mut self) {
        let config = serde_json::to_string(&self.config);
        match config {
            Ok(conf) => {
                if conf != self.config_hash {
                    let result = fs::write(&self.config_path, conf);
                    match result {
                        Ok(_) => {}
                        Err(e) => {
                            eprintln!("Error: {:?}", e);
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("Error: {:?}", e);
            }
        }
    }

    pub fn screen_enabled(&mut self, screen: String) -> bool {
        match self.config.screens.get(&screen) {
            Some(val) => val.active,
            None => match self.config.screens.insert(
                screen,
                ScreenConfig {
                    active: true,
                    config_attributes: IndexMap::new(),
                },
            ) {
                Some(screen_config) => screen_config.active,
                None => false,
            },
        }
    }

    pub fn set_screen_status(&mut self, screen: String, enabled: bool) {
        self.config.set_screen_active(screen, enabled);
    }

    pub fn get_value(&self, screen: &str, key: &str) -> Option<ConfigParam> {
        match self.config.screens.get(screen) {
            Some(screen_config) => match screen_config.config_attributes.get(key) {
                Some(value) => Some(value.clone()),
                None => None,
            },
            None => None,
        }
    }

    pub fn get_screen_config(&self, screen: &str) -> Option<exchange_format::ExchangeableConfig> {
        match self.config.screens.get(screen) {
            Some(screen_config) => Some(exchange_format::ExchangeableConfig {
                params: screen_config.config_attributes.clone(),
            }),
            None => None,
        }
    }

    pub fn set_value(&mut self, screen: String, key: String, value: ConfigParam) {
        self.config.set_screen_value(screen, key, value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use exchange_format::ConfigParam;
    const PATH: Option<&str> = Some("./settings_test.json");
    #[test]
    fn test_create_config_manager_with_default_filepath() {
        let mut config_manager = ConfigManager::new(PATH);
        config_manager.save();
        assert_eq!(config_manager.config.brightness, 100);
        assert_eq!(config_manager.config.companion_brightness, 100);
        assert_eq!(config_manager.config.screens.is_empty(), true);
    }

    #[test]
    fn test_save_config() {
        let mut config_manager = ConfigManager::new(PATH);
        config_manager.save();
    }

    #[test]
    fn test_screen_enabled_existing_screen() {
        let mut config_manager = ConfigManager::new(PATH);
        let screen_name = "screen1".to_string();
        config_manager.set_screen_status(screen_name.clone(), true);
        assert_eq!(config_manager.screen_enabled(screen_name.clone()), true);

        config_manager.set_screen_status(screen_name.clone(), false);
        assert_eq!(config_manager.screen_enabled(screen_name), false);
    }

    #[test]
    fn test_get_set_value() {
        let mut config_manager = ConfigManager::new(PATH);
        let screen_name = "screen1".to_string();
        let key = "key1".to_string();
        let value = ConfigParam::Integer(42);

        config_manager.set_value(screen_name.clone(), key.clone(), value.clone());
        let retrieved_value = config_manager.get_value(&screen_name, &key);

        assert!(matches!(retrieved_value.unwrap(), ConfigParam::Integer(42)));
    }

    #[test]
    fn test_get_screen_config() {
        let mut config_manager = ConfigManager::new(PATH);
        let screen_name = "screen1".to_string();
        let key = "key1".to_string();
        let value = ConfigParam::Integer(42);

        config_manager.set_value(screen_name.clone(), key.clone(), value.clone());
        let exchangeable_config = config_manager.get_screen_config(&screen_name);

        assert!(matches!(
            exchangeable_config.unwrap().params.get(&key),
            Some(ConfigParam::Integer(42))
        ));
    }
}
