use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AwesomeDisplayConfig {
    pub bitpanda_api_key: String,
    pub bitpanda_screen_active: bool,
    pub media_screen_active: bool,
    pub system_info_screen_active: bool,
    pub brightness: u16,
    // TODO: enable activation/deactivation of screens
}

impl AwesomeDisplayConfig {
    pub fn new(filepath: &str) -> Self {
        let mut this = AwesomeDisplayConfig {
            bitpanda_api_key: String::new(),
            bitpanda_screen_active: true,
            media_screen_active: true,
            system_info_screen_active: true,
            brightness: 100,
        };
        let contents = fs::read_to_string(filepath);
        let path = env::current_dir().unwrap();
        println!("The current directory is {}", path.display());
        match contents {
            Ok(config) => {
                this = serde_json::from_str(&config).unwrap();
            }
            Err(e) => {
                println!("Error: {:?}", e);
            }
        }
        return this;
    }

    pub fn save(&mut self, filepath: &str) {
        // do nothing at all right now
        let config = serde_json::to_string(self);
        match config {
            Ok(conf) => {
                let result = fs::write(filepath, conf);
                match result {
                    Ok(_) => {}
                    Err(e) => {
                        println!("Error: {:?}", e);
                    }
                }
            }
            Err(e) => {
                println!("Error: {:?}", e);
            }
        }
    }
}
