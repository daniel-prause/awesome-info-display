use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AwesomeDisplayConfig {
    pub bitpanda_api_key: String,
    // TODO: enable activation/deactivation of screens
}

impl AwesomeDisplayConfig {
    pub fn new(filepath: &str) -> Self {
        let mut this = AwesomeDisplayConfig {
            bitpanda_api_key: String::new(),
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

    // TODO: write me
    /*
    fn save(&mut self) {
        // do nothing at all right now
    }*/
}
