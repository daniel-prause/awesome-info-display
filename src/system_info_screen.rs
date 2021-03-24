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
pub struct SystemInfoScreen {
    screen: Screen,
    cpu_usage: Arc<Mutex<f64>>,
    ram_usage: Arc<Mutex<f64>>,
}

impl SpecificScreen for SystemInfoScreen {
    fn description(&self) -> &String {
        &self.screen.description
    }

    fn current_image(&self) -> Vec<u8> {
        self.screen.bytes.clone()
    }

    fn update(&mut self) {
        SystemInfoScreen::update(self);
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

    fn initial_update_called(&mut self) -> bool {
        if !self.screen.initial_update_called.load(Ordering::Acquire) {
            self.screen
                .initial_update_called
                .store(true, Ordering::Release);
            return false;
        }
        true
    }
}

impl SystemInfoScreen {
    pub fn draw_cpu(&mut self, image: &mut ImageBuffer<Rgb<u8>, Vec<u8>>, scale: Scale) {
        let cpu_text = format!("{: >3}%", self.cpu_usage.lock().unwrap(),).to_string();
        draw_text_mut(
            image,
            Rgb([255u8, 255u8, 255u8]),
            0,
            0,
            scale,
            self.screen.font.as_ref().unwrap(),
            "CPU",
        );
        draw_text_mut(
            image,
            Rgb([255u8, 255u8, 255u8]),
            222,
            0,
            scale,
            self.screen.font.as_ref().unwrap(),
            &cpu_text,
        );
        draw_hollow_rect_mut(
            image,
            Rect::at(0, 16).of_size(256, 10),
            Rgb([255u8, 255u8, 255u8]),
        );

        let cpu_filled = ((*self.cpu_usage.lock().unwrap() * 2.56) + 1.0).floor() as u32;
        draw_filled_rect_mut(
            image,
            Rect::at(0, 16).of_size(cpu_filled, 10),
            Rgb([255u8, 255u8, 255u8]),
        );
    }
    pub fn draw_memory(&mut self, image: &mut ImageBuffer<Rgb<u8>, Vec<u8>>, scale: Scale) {
        let memory_text = format!("{: >3}%", self.ram_usage.lock().unwrap()).to_string();
        draw_text_mut(
            image,
            Rgb([255u8, 255u8, 255u8]),
            0,
            30,
            scale,
            self.screen.font.as_ref().unwrap(),
            "RAM",
        );
        draw_text_mut(
            image,
            Rgb([255u8, 255u8, 255u8]),
            222,
            30,
            scale,
            self.screen.font.as_ref().unwrap(),
            &memory_text,
        );
        draw_hollow_rect_mut(
            image,
            Rect::at(0, 48).of_size(256, 10),
            Rgb([255u8, 255u8, 255u8]),
        );

        let memory_filled = ((*self.ram_usage.lock().unwrap() * 2.56) + 1.0).floor() as u32;
        draw_filled_rect_mut(
            image,
            Rect::at(0, 48).of_size(memory_filled, 10),
            Rgb([255u8, 255u8, 255u8]),
        );
    }

    fn update(&mut self) {
        let mut image = RgbImage::new(256, 64);
        let scale = Scale { x: 16.0, y: 16.0 };

        self.draw_cpu(&mut image, scale);
        self.draw_memory(&mut image, scale);
        self.screen.bytes = image.into_vec();
    }

    pub fn new(description: String, font: Option<Font<'static>>) -> Self {
        let this = SystemInfoScreen {
            screen: Screen {
                description,
                font,
                ..Default::default()
            },
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
