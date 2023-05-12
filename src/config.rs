use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    pub bitpanda_api_key: String,
    pub weather_location: String,
    pub bitpanda_screen_active: bool,
    pub media_screen_active: bool,
    pub system_info_screen_active: bool,
    pub weather_screen_active: bool,
    pub ice_screen_active: bool,
    pub current_date_screen_active: bool,
    pub brightness: u16,
}
