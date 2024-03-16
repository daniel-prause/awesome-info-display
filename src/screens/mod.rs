use crate::config_manager::ConfigManager;
use crate::{ESP32, TEENSY};
use ab_glyph::{FontArc};
use exchange_format::ExchangeableConfig;

use std::sync::{atomic::AtomicBool, atomic::Ordering, Arc, RwLock};
use std::thread::JoinHandle;
use std::time::Instant;
pub mod media_info_screen;
pub mod plugin_screen;
pub mod system_info_screen;
pub mod weather_screen;

pub struct Screen {
    pub description: String,
    pub key: String,
    pub main_screen_bytes: Vec<u8>,
    pub companion_screen_bytes: Vec<u8>,
    pub font: FontArc,
    pub symbols: FontArc,
    pub active: Arc<AtomicBool>,
    pub handle: Option<JoinHandle<()>>,
    pub mode: u32,
    pub mode_timeout: Option<Instant>,
    pub config_manager: Arc<RwLock<ConfigManager>>,
    pub config_layout: ExchangeableConfig,
}

impl Default for Screen {
    fn default() -> Screen {
        Screen {
            description: String::from(""),
            key: String::from(""),
            main_screen_bytes: Vec::new(), // Oled display byte image
            companion_screen_bytes: vec![0; 320 * 170 * 3], // companion display byte image
            font: FontArc::try_from_slice(include_bytes!("../fonts/Liberation.ttf") as &[u8])
                .unwrap(),

            symbols: FontArc::try_from_slice(include_bytes!("../fonts/symbols.otf") as &[u8])
                .unwrap(),

            active: Arc::new(AtomicBool::new(false)),
            handle: None,
            mode: 0,
            mode_timeout: Some(Instant::now()),
            config_manager: Arc::new(RwLock::new(ConfigManager::new(None))),
            config_layout: ExchangeableConfig::default(),
        }
    }
}

pub trait Screenable {
    fn get_screen(&mut self) -> &mut Screen;
}

pub trait BasicScreen: Screenable {
    fn update(&mut self);

    fn description(&mut self) -> String {
        let screen = self.get_screen();
        screen.description.clone()
    }

    fn config_layout(&mut self) -> exchange_format::ExchangeableConfig {
        let screen = self.get_screen();
        screen.config_layout.clone()
    }

    fn key(&mut self) -> String {
        let screen = self.get_screen();
        screen.key.clone()
    }

    fn current_image(&mut self, device: &str) -> Option<Vec<u8>> {
        let screen = self.get_screen();
        match device {
            TEENSY => Some(screen.main_screen_bytes.clone()),
            ESP32 => Some(screen.companion_screen_bytes.clone()),
            _ => None,
        }
    }

    fn start(&mut self) {
        let screen = self.get_screen();
        screen.active.store(true, Ordering::Release);
        match screen.handle.as_ref() {
            Some(handle) => {
                handle.thread().unpark();
            }
            None => {}
        }
    }

    fn stop(&mut self) {
        self.get_screen().active.store(false, Ordering::Release)
    }

    fn set_mode(&mut self, mode: u32) {
        let screen = self.get_screen();
        screen.mode_timeout = Some(Instant::now());
        screen.mode = mode;
    }

    fn enabled(&mut self) -> bool {
        let screen = self.get_screen();

        return screen
            .config_manager
            .write()
            .unwrap()
            .screen_enabled(screen.key.clone());
    }

    fn set_status(&mut self, status: bool) {
        let screen = self.get_screen();
        screen
            .config_manager
            .write()
            .unwrap()
            .set_screen_status(screen.key.clone(), status)
    }

    fn set_current_config(&mut self, _config: ExchangeableConfig) {
        // implement, if needed
    }
}
