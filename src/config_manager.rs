use crate::config::{Config, ScreenConfig};

use exchange_format::ConfigParam;
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
        let config = Config {
            brightness: 100,
            companion_brightness: 100,
            screens: HashMap::new(),
        };
        let mut this = ConfigManager {
            config,
            config_hash: String::new(),
            config_path: String::from("./settings.json"),
        };

        // check, if file exists; if not -> create it
        let file_exists = std::path::Path::new(filepath.unwrap_or("./settings.json")).exists();
        if !file_exists {
            match File::create(filepath.unwrap_or("./settings.json")) {
                Ok(_) => {
                    println!("Settings file created!")
                }
                Err(err) => {
                    eprintln!("Error creating file: {}", err);
                }
            }
        }
        let contents = fs::read_to_string(filepath.unwrap_or("./settings.json"));
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
                    config_attributes: HashMap::new(),
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

    pub fn set_value(&mut self, screen: String, key: String, value: ConfigParam) {
        self.config.set_screen_value(screen, key, value);
    }
}
