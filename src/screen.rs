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
    pub initial_update_called: Arc<AtomicBool>,
    pub handle: Arc<Mutex<Option<JoinHandle<()>>>>,
}

impl Default for Screen {
    fn default() -> Screen {
        Screen {
            description: String::from(""),
            bytes: Vec::new(),
            font: Font::try_from_vec(Vec::from(include_bytes!("Liberation.ttf") as &[u8])),
            active: Arc::new(AtomicBool::new(false)),
            initial_update_called: Arc::new(AtomicBool::new(false)),
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
    fn initial_update_called(&mut self) -> bool;
    fn start(&self) -> ();
    fn stop(&self) -> ();
    fn convert_to_gray_scale(&self, bytes: &Vec<u8>) -> Vec<String> {
        let mut buffer = Vec::<String>::new();
        for chunk in bytes.chunks(6) {
            let gray = 0.299 * chunk[0] as f32 + 0.587 * chunk[1] as f32 + 0.114 * chunk[2] as f32;
            let gray2 = 0.299 * chunk[3] as f32 + 0.587 * chunk[4] as f32 + 0.114 * chunk[5] as f32;
            buffer.push(format!(
                "0X{:02X},",
                (((gray / 16.0) as u8) << 4 | ((gray2 / 16.0) as u8))
            ));
        }
        buffer
    }
}
