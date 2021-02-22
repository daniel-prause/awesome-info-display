use image::DynamicImage;
use image::ImageBuffer;
use image::Rgb;
use image::RgbImage;
use imageproc::drawing::draw_text_mut;
use rusttype::{Font, Scale};
use std::thread;
//use sysinfo::RefreshKind;
//use sysinfo::{System, SystemExt};
use cpu_monitor::CpuInstant;
use imageproc::drawing::draw_filled_rect_mut;
use imageproc::drawing::draw_hollow_rect_mut;
use imageproc::rect::Rect;
use std::io;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::sync::Mutex;
use std::time;
use std::time::Duration;
use systemstat::{saturating_sub_bytes, Platform, System};
#[derive(Debug)]
struct Error1;
#[derive(Debug)]
struct Error2;

#[derive(Debug, Clone, Default)]
pub struct Screen {
    description: String,
    bytes: Vec<u8>,
    font: Option<Font<'static>>,
    cpu_usage: Arc<Mutex<f32>>,
    ram_usage: Arc<Mutex<f64>>,
}

impl Screen {
    pub fn new(description: String) -> Self {
        let bytes = Vec::new();
        let font = Vec::from(include_bytes!("Liberation.ttf") as &[u8]);
        let font = Font::try_from_vec(font);
        let this = Screen {
            description,
            bytes: bytes,
            font: font,
            cpu_usage: Arc::new(Mutex::new(0.0)),
            ram_usage: Arc::new(Mutex::new(0.0)),
        };

        let builder = thread::Builder::new().name("JOB_EXECUTOR".into());
        let sys = System::new();

        builder
            .spawn({
                let this = this.clone();
                move || loop {
                    match sys.cpu_load_aggregate() {
                        Ok(cpu) => {
                            thread::sleep(Duration::from_secs(1));
                            let mut value = this.cpu_usage.lock().unwrap();
                            let cpu = cpu.done().unwrap();
                            *value = (cpu.user * 100.0).floor();
                        }
                        Err(x) => println!("\nCPU load: error: {}", x),
                    }
                    match sys.memory() {
                        Ok(mem) => {
                            let mut value = this.ram_usage.lock().unwrap();
                            let memory_used =
                                saturating_sub_bytes(mem.total, mem.free).as_u64() as f64;
                            *value = ((memory_used / mem.total.as_u64() as f64) * 100.0).floor();
                        }
                        Err(x) => println!("\nMemory: error: {}", x),
                    }
                }
            })
            .expect("Cannot create JOB_EXECUTOR thread");
        this
    }

    pub fn description(&self) -> &String {
        &self.description
    }

    pub fn current_image(&self) -> Vec<u8> {
        self.bytes.clone()
    }
    pub fn update(&mut self) {
        //let mut image = RgbImage::new(256, 64);
        let mut image = RgbImage::new(256, 64);
        let height = 16.0;
        let scale = Scale {
            x: height,
            y: height,
        };

        let cpu_text = format!("{: >3}%", cpu = self.cpu_usage.lock().unwrap(),).to_string();
        let memory_text = format!("{: >3}%", memory = self.ram_usage.lock().unwrap()).to_string();

        let font = self.font.as_ref().unwrap();

        // DRAW CPU
        draw_text_mut(
            &mut image,
            Rgb([255u8, 255u8, 255u8]),
            0,
            0,
            scale,
            &font,
            "CPU",
        );
        draw_text_mut(
            &mut image,
            Rgb([255u8, 255u8, 255u8]),
            222,
            0,
            scale,
            &font,
            &cpu_text,
        );
        draw_hollow_rect_mut(
            &mut image,
            Rect::at(0, 16).of_size(256, 10),
            Rgb([255u8, 255u8, 255u8]),
        );

        let cpu_filled = ((*self.cpu_usage.lock().unwrap() * 2.56) + 1.0).floor() as u32;
        draw_filled_rect_mut(
            &mut image,
            Rect::at(0, 16).of_size(cpu_filled, 10),
            Rgb([255u8, 255u8, 255u8]),
        );

        // DRAW MEMORY
        draw_text_mut(
            &mut image,
            Rgb([255u8, 255u8, 255u8]),
            0,
            30,
            scale,
            &font,
            "RAM",
        );
        draw_text_mut(
            &mut image,
            Rgb([255u8, 255u8, 255u8]),
            222,
            30,
            scale,
            &font,
            &memory_text,
        );

        draw_hollow_rect_mut(
            &mut image,
            Rect::at(0, 48).of_size(256, 10),
            Rgb([255u8, 255u8, 255u8]),
        );

        let memory_filled = ((*self.ram_usage.lock().unwrap() * 2.56) + 1.0).floor() as u32;
        draw_filled_rect_mut(
            &mut image,
            Rect::at(0, 48).of_size(memory_filled, 10),
            Rgb([255u8, 255u8, 255u8]),
        );

        self.bytes.clear();
        let _ =
            DynamicImage::ImageRgb8(image).write_to(&mut self.bytes, image::ImageOutputFormat::Bmp);
    }
}
