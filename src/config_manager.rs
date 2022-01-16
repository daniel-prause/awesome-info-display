use crate::config::Config;

use serde::{Deserialize, Serialize};

use std::fs;

#[derive(Serialize, Deserialize, Debug, Clone)]
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
            system_info_screen_active: true,
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
                println!("Error: {:?}", e);
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
                            println!("Error: {:?}", e);
                        }
                    }
                }
            }
            Err(e) => {
                println!("Error: {:?}", e);
            }
        }
    }
}
