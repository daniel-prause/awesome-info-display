use rusttype::Font;
use std::fmt::Debug;
use std::sync::{atomic::AtomicBool, Arc, Mutex};
use std::thread::JoinHandle;

#[derive(Debug, Clone)]
pub struct Screen {
    pub description: String,
    pub bytes: Vec<u8>,
    pub font: Option<Font<'static>>,
    pub active: Arc<AtomicBool>,
    pub handle: Arc<Mutex<Option<JoinHandle<()>>>>,
}

impl Default for Screen {
    fn default() -> Screen {
        Screen {
            description: String::from(""),
            bytes: Vec::new(),
            font: Font::try_from_vec(Vec::from(include_bytes!("Liberation.ttf") as &[u8])),
            active: Arc::new(AtomicBool::new(false)),
            handle: Arc::new(Mutex::new(None)),
        }
    }
}

impl std::fmt::Debug for dyn SpecificScreen {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.description())
    }
}

pub trait SpecificScreen {
    fn update(&mut self) -> ();
    fn description(&self) -> &String;
    fn current_image(&self) -> Vec<u8>;
    fn start(&self) -> ();
    fn stop(&self) -> ();
}
