use image::io::Reader as ImageReader;
use image::DynamicImage;
use image::ImageBuffer;
use image::Rgb;
use image::RgbImage;
use image::Rgba;
use imageproc::drawing;
use imageproc::drawing::draw_text_mut;
use rusttype::{Font, Scale};
use std::env;
use std::io::Cursor;
use std::path::Path;

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
        let mut image = RgbImage::new(256, 64);
        let mut bytes = Vec::new();

        let font = Vec::from(include_bytes!("DejaVuSans.ttf") as &[u8]);
        let font = Font::try_from_vec(font).unwrap();
        let height = 16.0;
        let scale = Scale {
            x: height,
            y: height,
        };
        let text = "CPU: 33% / Hallo Chris =)";
        draw_text_mut(
            &mut image,
            Rgb([255u8, 255u8, 255u8]),
            0,
            0,
            scale,
            &font,
            text,
        );
        let _ = DynamicImage::ImageRgb8(image).write_to(&mut bytes, image::ImageOutputFormat::Bmp);
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
