extern crate cpu_monitor;
use crate::config_manager::ConfigManager;
use crate::screens::BasicScreen;
use crate::screens::Screen;
use crate::screens::ScreenControl;

use cpu_monitor::CpuInstant;
use image::{ImageBuffer, Rgb, RgbImage};
use imageproc::drawing::{draw_filled_rect_mut, draw_hollow_rect_mut, draw_text_mut};
use imageproc::rect::Rect;
use rusttype::Font;
use rusttype::Scale;
use std::fmt::Debug;
use std::sync::{atomic::Ordering, Arc, Mutex, RwLock};
use std::thread;
use std::time::Duration;
use systemstat::{saturating_sub_bytes, Platform, System};

#[derive(Debug)]
pub struct SystemInfoScreen {
    screen: Screen,
}

impl BasicScreen for std::sync::Arc<RwLock<SystemInfoScreen>> {
    fn description(&self) -> String {
        self.read().unwrap().screen.description.clone()
    }

    fn current_image(&self) -> Vec<u8> {
        self.read().unwrap().screen.current_image()
    }

    fn update(&mut self) {}

    fn start(&self) {
        self.read().unwrap().screen.start_worker();
    }

    fn stop(&self) {
        self.read().unwrap().screen.stop_worker();
    }

    fn key(&self) -> String {
        self.read().unwrap().screen.key()
    }

    fn initial_update_called(&mut self) -> bool {
        self.write().unwrap().screen.initial_update_called()
    }

    fn enabled(&self) -> bool {
        self.read()
            .unwrap()
            .screen
            .config_manager
            .read()
            .unwrap()
            .config
            .system_info_screen_active
    }

    fn set_status(&self, status: bool) {
        self.read()
            .unwrap()
            .screen
            .config_manager
            .write()
            .unwrap()
            .config
            .system_info_screen_active = status;
    }
}

impl SystemInfoScreen {
    pub fn draw_cpu(
        &mut self,
        image: &mut ImageBuffer<Rgb<u8>, Vec<u8>>,
        cpu_usage: f64,
        scale: Scale,
    ) {
        let cpu_text = format!("{: >3}%", cpu_usage.to_string());
        draw_text_mut(
            image,
            Rgb([255u8, 255u8, 255u8]),
            0,
            0,
            scale,
            self.screen.font.lock().unwrap().as_ref().unwrap(),
            "CPU",
        );
        draw_text_mut(
            image,
            Rgb([255u8, 255u8, 255u8]),
            222,
            0,
            scale,
            self.screen.font.lock().unwrap().as_ref().unwrap(),
            &cpu_text,
        );
        draw_hollow_rect_mut(
            image,
            Rect::at(0, 16).of_size(256, 10),
            Rgb([255u8, 255u8, 255u8]),
        );

        let cpu_filled = ((cpu_usage * 2.56) + 1.0).floor() as u32;
        draw_filled_rect_mut(
            image,
            Rect::at(0, 16).of_size(cpu_filled, 10),
            Rgb([255u8, 255u8, 255u8]),
        );
    }
    pub fn draw_memory(
        &mut self,
        image: &mut ImageBuffer<Rgb<u8>, Vec<u8>>,
        ram_usage: f64,
        scale: Scale,
    ) {
        let memory_text = format!("{: >3}%", ram_usage.to_string());
        draw_text_mut(
            image,
            Rgb([255u8, 255u8, 255u8]),
            0,
            30,
            scale,
            self.screen.font.lock().unwrap().as_ref().unwrap(),
            "RAM",
        );
        draw_text_mut(
            image,
            Rgb([255u8, 255u8, 255u8]),
            222,
            30,
            scale,
            self.screen.font.lock().unwrap().as_ref().unwrap(),
            &memory_text,
        );
        draw_hollow_rect_mut(
            image,
            Rect::at(0, 48).of_size(256, 10),
            Rgb([255u8, 255u8, 255u8]),
        );

        let memory_filled = ((ram_usage * 2.56) + 1.0).floor() as u32;
        draw_filled_rect_mut(
            image,
            Rect::at(0, 48).of_size(memory_filled, 10),
            Rgb([255u8, 255u8, 255u8]),
        );
    }

    fn draw_screen(&mut self, cpu_usage: f64, ram_usage: f64) {
        // draw initial image
        let mut image = RgbImage::new(256, 64);
        let scale = Scale { x: 16.0, y: 16.0 };

        self.draw_cpu(&mut image, cpu_usage, scale);
        self.draw_memory(&mut image, ram_usage, scale);
        *self.screen.bytes.lock().unwrap() = image.into_vec();
    }

    pub fn new(
        description: String,
        key: String,
        font: Arc<Mutex<Option<Font<'static>>>>,
        config_manager: Arc<RwLock<ConfigManager>>,
    ) -> Arc<RwLock<SystemInfoScreen>> {
        let this = Arc::new(RwLock::new(SystemInfoScreen {
            screen: Screen {
                description,
                key,
                font,
                config_manager,
                ..Default::default()
            },
        }));

        // start thread
        let builder = thread::Builder::new().name("JOB_EXECUTOR".into());
        let sys = System::new();
        this.write().unwrap().draw_screen(0f64, 0f64);
        *this.read().unwrap().screen.handle.lock().unwrap() = Some(
            builder
                .spawn({
                    let this = this.clone();
                    move || loop {
                        while !this.read().unwrap().screen.active.load(Ordering::Acquire) {
                            thread::park();
                        }

                        // cpu load from different crate since systemstats is not specific enough
                        let start = cpu_monitor::CpuInstant::now().unwrap();
                        std::thread::sleep(Duration::from_millis(1000));
                        let end = CpuInstant::now().unwrap();
                        let duration = end - start;
                        let cpu_usage: f64 = (duration.non_idle() * 100.0).floor().into();
                        let mut memory_used = 0f64;
                        match sys.memory() {
                            Ok(mem) => {
                                memory_used =
                                    saturating_sub_bytes(mem.total, mem.free).as_u64() as f64;
                                memory_used =
                                    ((memory_used / mem.total.as_u64() as f64) * 100.0).floor();
                            }
                            Err(x) => println!("\nMemory: error: {}", x),
                        }
                        // draw image
                        this.write().unwrap().draw_screen(cpu_usage, memory_used);
                    }
                })
                .expect("Cannot create JOB_EXECUTOR thread"),
        );
        this.clone()
    }
}
