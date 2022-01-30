use crate::config_manager::ConfigManager;
use rusttype::Font;
use std::rc::Rc;
use std::sync::{atomic::AtomicBool, atomic::Ordering, Arc, RwLock};
use std::thread::JoinHandle;
use std::time::Instant;
pub mod bitpanda_screen;
pub mod media_info_screen;
pub mod system_info_screen;
pub mod weather_screen;

pub struct Screen {
    pub description: String,
    pub key: String,
    pub bytes: Vec<u8>,
    pub font: Rc<Font<'static>>,
    pub active: Arc<AtomicBool>,
    pub initial_update_called: bool,
    pub handle: Option<JoinHandle<()>>,
    pub mode: u32,
    pub mode_timeout: Option<Instant>,
    pub config_manager: Arc<RwLock<ConfigManager>>,
}

impl Default for Screen {
    fn default() -> Screen {
        Screen {
            description: String::from(""),
            key: String::from(""),
            bytes: Vec::new(),
            font: Rc::new(
                Font::try_from_vec(Vec::from(include_bytes!("../Liberation.ttf") as &[u8]))
                    .unwrap(),
            ),
            active: Arc::new(AtomicBool::new(false)),
            initial_update_called: false,
            handle: None,
            mode: 0,
            mode_timeout: Some(Instant::now()),
            config_manager: Arc::new(RwLock::new(ConfigManager::new(None))),
        }
    }
}

pub trait BasicScreen {
    fn update(&mut self) -> ();
    fn description(&self) -> String;
    fn key(&self) -> String;
    fn current_image(&self) -> Vec<u8>;
    fn initial_update_called(&mut self) -> bool;
    fn start(&self) -> ();
    fn stop(&self) -> ();
    fn set_mode_for_short(&mut self, _mode: u32) {}
    fn enabled(&self) -> bool;
    fn set_status(&self, status: bool) -> ();
}

pub trait ScreenControl {
    fn start_worker(&self);
    fn stop_worker(&self);
    fn initial_update_called(&mut self) -> bool;
    fn current_image(&self) -> Vec<u8>;
    fn key(&self) -> String;
}

impl ScreenControl for Screen {
    fn key(&self) -> String {
        self.key.clone()
    }

    fn start_worker(&self) {
        self.active.store(true, Ordering::Release);
        match self.handle.as_ref() {
            Some(handle) => {
                handle.thread().unpark();
            }
            None => {}
        }
    }

    fn stop_worker(&self) {
        self.active.store(false, Ordering::Release);
    }

    fn initial_update_called(&mut self) -> bool {
        if !self.initial_update_called {
            self.initial_update_called = true;
            return false;
        }
        true
    }

    fn current_image(&self) -> Vec<u8> {
        self.bytes.clone()
    }
}
