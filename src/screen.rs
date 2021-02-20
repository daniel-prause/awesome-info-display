use image::RgbImage;

#[derive(Debug, Clone, Default)]
pub struct Screen {
    description: String,
    current_image: RgbImage,
}

#[derive(Debug, Clone, Copy)]
pub enum ScreenMessage {
    UpdateScreen,
}

impl Screen {
    pub fn new(description: String) -> Self {
        Screen {
            description,
            current_image: RgbImage::new(256, 64),
        }
    }
    pub fn description(&self) -> &String {
        &self.description
    }

    pub fn update(&mut self) {}
}
