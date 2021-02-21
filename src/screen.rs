use image::io::Reader as ImageReader;
use image::ImageBuffer;
use image::Rgb;
use image::RgbImage;

#[derive(Debug)]
struct Error1;
#[derive(Debug)]
struct Error2;

#[derive(Debug, Clone, Default)]
pub struct Screen {
    description: String,
    current_image: RgbImage,
    bytes: Vec<u8>,
}

impl Screen {
    pub fn new(description: String, path: String) -> Self {
        //let img = image::open(path).unwrap();
        let img = ImageReader::open(path).unwrap().decode().unwrap();
        let mut bytes = Vec::new();
        let _ = img.write_to(&mut bytes, image::ImageOutputFormat::Bmp);
        Screen {
            description,
            current_image: RgbImage::new(256, 64),
            bytes: bytes,
        }
    }
    pub fn description(&self) -> &String {
        &self.description
    }

    pub fn current_image(&self) -> Vec<u8> {
        self.bytes.clone()
    }
}

trait Update {
    fn update(&mut self) {}
}

impl Update for Screen {
    fn update(&mut self) {}
}
