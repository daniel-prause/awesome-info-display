use crate::config::Config;

use serde::{Deserialize, Serialize};

use std::fs;

#[derive(Serialize, Deserialize, Clone)]
pub struct ConfigManager {
    pub config: Config,
    config_hash: String,
    config_path: String,
}

impl ConfigManager {
    pub fn new(filepath: Option<&str>) -> Self {
        let config = Config {
            bitpanda_api_key: String::new(),
            bitpanda_screen_active: true,
            openweather_api_key: String::new(),
            openweather_location: String::new(),
            media_screen_active: true,
            weather_screen_active: true,
            ice_screen_active: true,
            system_info_screen_active: true,
            current_date_screen_active: true,
            brightness: 100,
        };
        let mut this = ConfigManager {
            config: config,
            config_hash: String::new(),
            config_path: String::from("./settings.json"),
        };
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
        return this;
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
        let screen = &*screen;
        match screen {
            "bitpanda_screen" => {
                return self.config.bitpanda_screen_active;
            }
            "weather_screen" => {
                return self.config.weather_screen_active;
            }
            "media_info_screen" => {
                return self.config.media_screen_active;
            }
            "system_info_screen" => {
                return self.config.system_info_screen_active;
            }
            "current_date_screen" => {
                return self.config.current_date_screen_active;
            }
            "ice_screen" => {
                return self.config.ice_screen_active;
            }
            _ => false,
        }
    }

    pub fn set_screen_status(&mut self, screen: String, enabled: bool) {
        let screen = &*screen;
        match screen {
            "bitpanda_screen" => {
                self.config.bitpanda_screen_active = enabled;
            }
            "weather_screen" => {
                self.config.weather_screen_active = enabled;
            }
            "media_info_screen" => {
                self.config.media_screen_active = enabled;
            }
            "system_info_screen" => {
                self.config.system_info_screen_active = enabled;
            }
            "current_date_screen" => {
                self.config.current_date_screen_active = enabled;
            }
            "ice_screen" => {
                self.config.ice_screen_active = enabled;
            }
            _ => {}
        }
    }
}
