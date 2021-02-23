




use rusttype::{Font};






use std::fmt::Debug;



use systemstat::{saturating_sub_bytes, Platform, System};

#[derive(Debug, Clone)]
pub struct Screen {
    pub description: String,
    pub bytes: Vec<u8>,
    pub font: Option<Font<'static>>,
}

impl Default for Screen {
    fn default() -> Screen {
        Screen {
            description: String::from(""),
            bytes: Vec::new(),
            font: Font::try_from_vec(Vec::from(include_bytes!("Liberation.ttf") as &[u8])),
        }
    }
}

impl std::fmt::Debug for dyn SpecificScreen {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Hi")
    }
}

pub trait SpecificScreen {
    fn update(&mut self) -> ();
    //fn new(&self, description: String) -> Self;
    fn description(&self) -> &String;
    fn current_image(&self) -> Vec<u8>;
}
