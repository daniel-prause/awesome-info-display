use crate::screen::Screen;
use crate::screen::SpecificScreen;

use image::{DynamicImage, ImageBuffer, Rgb, RgbImage};
use imageproc::drawing::{draw_filled_rect_mut, draw_hollow_rect_mut, draw_text_mut};
use imageproc::rect::Rect;
use rusttype::Font;
use rusttype::Scale;
use std::fmt::Debug;
use std::sync::{atomic::Ordering, Arc, Mutex};
use std::thread;
use std::time::Duration;
use systemstat::{saturating_sub_bytes, Platform, System};

#[derive(Debug, Clone)]
pub struct MediaInfoScreen {
    screen: Screen,
    symbols: Option<Font<'static>>,
    cpu_usage: Arc<Mutex<f64>>,
    ram_usage: Arc<Mutex<f64>>,
}

impl SpecificScreen for MediaInfoScreen {
    fn description(&self) -> &String {
        &self.screen.description
    }

    fn current_image(&self) -> Vec<u8> {
        self.screen.bytes.clone()
    }

    fn update(&mut self) {
        MediaInfoScreen::update(self);
    }

    fn start(&self) {
        self.screen.active.store(true, Ordering::Release);
        if !self.screen.handle.lock().unwrap().is_none() {
            self.screen
                .handle
                .lock()
                .as_ref()
                .unwrap()
                .as_ref()
                .unwrap()
                .thread()
                .unpark();
        }
    }
    fn stop(&self) {
        self.screen.active.store(false, Ordering::Release);
    }
}

impl MediaInfoScreen {
    pub fn draw_artist(&mut self, image: &mut ImageBuffer<Rgb<u8>, Vec<u8>>, scale: Scale) {
        let artist = "Dire Straits";
        let position_artist = (256 - (artist.len() * 16) / 2) / 2;
        draw_text_mut(
            image,
            Rgb([255u8, 255u8, 255u8]),
            position_artist as u32,
            0,
            scale,
            self.screen.font.as_ref().unwrap(),
            artist,
        );
    }

    pub fn draw_title(&mut self, image: &mut ImageBuffer<Rgb<u8>, Vec<u8>>, scale: Scale) {
        let title = "Money for nothing";
        let position_title = (256 - (title.len() * 16) / 2) / 2;
        draw_text_mut(
            image,
            Rgb([255u8, 255u8, 255u8]),
            position_title as u32,
            16,
            scale,
            self.screen.font.as_ref().unwrap(),
            title,
        );
    }

    pub fn draw_play_button(&mut self, image: &mut ImageBuffer<Rgb<u8>, Vec<u8>>, scale: Scale) {
        let play_button = &String::from("\u{f04B}");
        let pause_button = &String::from("\u{f04C}");
        let stop_button = &String::from("\u{f04D}");
        draw_text_mut(
            image,
            Rgb([255u8, 255u8, 255u8]),
            4,
            37,
            Scale { x: 10.0, y: 10.0 },
            self.symbols.as_ref().unwrap(),
            pause_button,
        );
    }

    pub fn draw_elapsed(&mut self, image: &mut ImageBuffer<Rgb<u8>, Vec<u8>>, scale: Scale) {
        let elapsed = "00:00:00";
        draw_text_mut(
            image,
            Rgb([255u8, 255u8, 255u8]),
            16,
            36,
            Scale { x: 14.0, y: 14.0 },
            self.screen.font.as_ref().unwrap(),
            elapsed,
        );
    }

    pub fn draw_total(&mut self, image: &mut ImageBuffer<Rgb<u8>, Vec<u8>>, scale: Scale) {
        let elapsed = format!("{: >12}", "00:00:00").to_string();
        draw_text_mut(
            image,
            Rgb([255u8, 255u8, 255u8]),
            166,
            36,
            Scale { x: 14.0, y: 14.0 },
            self.screen.font.as_ref().unwrap(),
            &elapsed,
        );
    }

    pub fn draw_elapsed_bar(&mut self, image: &mut ImageBuffer<Rgb<u8>, Vec<u8>>, scale: Scale) {
        let indicator_position_x_min = 16.0;
        let indicator_position_x_max = 232.0;
        let progress = 0.0;

        let position = indicator_position_x_min + (progress * indicator_position_x_max);
        draw_hollow_rect_mut(
            image,
            Rect::at(16, 50).of_size(238, 6),
            Rgb([255u8, 255u8, 255u8]),
        );

        draw_filled_rect_mut(
            image,
            Rect::at(position as i32, 50).of_size(6, 6),
            Rgb([255u8, 255u8, 255u8]),
        );
        /*
        let cpu_filled = ((*self.cpu_usage.lock().unwrap() * 2.56) + 1.0).floor() as u32;
        draw_filled_rect_mut(
            image,
            Rect::at(0, 16).of_size(cpu_filled, 10),
            Rgb([255u8, 255u8, 255u8]),
        );*/
    }

    fn update(&mut self) {
        let mut image = RgbImage::new(256, 64);
        let scale = Scale { x: 16.0, y: 16.0 };

        self.draw_artist(&mut image, scale);
        self.draw_title(&mut image, scale);
        self.draw_play_button(&mut image, scale);
        self.draw_elapsed(&mut image, scale);
        self.draw_total(&mut image, scale);
        self.draw_elapsed_bar(&mut image, scale);
        self.screen.bytes.clear();
        let _ = DynamicImage::ImageRgb8(image)
            .write_to(&mut self.screen.bytes, image::ImageOutputFormat::Bmp);
    }

    pub fn new(description: String, font: Option<Font<'static>>) -> Self {
        let this = MediaInfoScreen {
            screen: Screen {
                description,
                font,
                ..Default::default()
            },
            symbols: Font::try_from_vec(Vec::from(include_bytes!("symbols.otf") as &[u8])),
            cpu_usage: Arc::new(Mutex::new(0.0)),
            ram_usage: Arc::new(Mutex::new(0.0)),
        };

        let builder = thread::Builder::new().name("JOB_EXECUTOR".into());
        let sys = System::new();

        *this.screen.handle.lock().unwrap() = Some(
            builder
                .spawn({
                    let this = this.clone();
                    move || loop {
                        while !this.screen.active.load(Ordering::Acquire) {
                            thread::park();
                        }
                        match sys.cpu_load_aggregate() {
                            Ok(cpu) => {
                                thread::sleep(Duration::from_millis(1000));
                                let mut value = this.cpu_usage.lock().unwrap();
                                let cpu = cpu.done().unwrap();
                                *value =
                                    ((cpu.user + cpu.nice + cpu.system) * 100.0).floor().into();
                            }
                            Err(x) => println!("\nCPU load: error: {}", x),
                        }
                        match sys.memory() {
                            Ok(mem) => {
                                let mut value = this.ram_usage.lock().unwrap();
                                let memory_used =
                                    saturating_sub_bytes(mem.total, mem.free).as_u64() as f64;
                                *value =
                                    ((memory_used / mem.total.as_u64() as f64) * 100.0).floor();
                            }
                            Err(x) => println!("\nMemory: error: {}", x),
                        }
                    }
                })
                .expect("Cannot create JOB_EXECUTOR thread"),
        );
        this
    }
}
