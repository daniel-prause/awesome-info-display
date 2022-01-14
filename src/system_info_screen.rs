use crate::config_manager::ConfigManager;
use crate::screen::BasicScreen;
use crate::screen::Screen;
use crate::screen::ScreenControl;

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

#[derive(Debug, Clone)]
pub struct SystemInfoScreen {
    screen: Screen,
    cpu_usage: Arc<Mutex<f64>>,
    ram_usage: Arc<Mutex<f64>>,
}

impl BasicScreen for std::sync::Arc<RwLock<SystemInfoScreen>> {
    fn description(&self) -> String {
        self.read().unwrap().screen.description.clone()
    }

    fn current_image(&self) -> Vec<u8> {
        self.read().unwrap().screen.bytes.lock().unwrap().clone()
    }

    fn update(&mut self) {
        SystemInfoScreen::update(self.clone());
    }

    fn start(&self) {
        self.read().unwrap().screen.start_worker();
    }

    fn stop(&self) {
        self.read().unwrap().screen.stop_worker();
    }

    fn initial_update_called(&mut self) -> bool {
        return self.write().unwrap().screen.initial_update_called();
    }

    fn enabled(&self) -> bool {
        return self
            .read()
            .unwrap()
            .screen
            .config_manager
            .read()
            .unwrap()
            .config
            .system_info_screen_active;
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
    pub fn draw_cpu(&mut self, image: &mut ImageBuffer<Rgb<u8>, Vec<u8>>, scale: Scale) {
        let cpu_text = format!("{: >3}%", self.cpu_usage.lock().unwrap(),).to_string();
        draw_text_mut(
            image,
            Rgb([255u8, 255u8, 255u8]),
            0,
            0,
            scale,
            *&self.screen.font.lock().unwrap().as_ref().unwrap(),
            "CPU",
        );
        draw_text_mut(
            image,
            Rgb([255u8, 255u8, 255u8]),
            222,
            0,
            scale,
            *&self.screen.font.lock().unwrap().as_ref().unwrap(),
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
            *&self.screen.font.lock().unwrap().as_ref().unwrap(),
            "RAM",
        );
        draw_text_mut(
            image,
            Rgb([255u8, 255u8, 255u8]),
            222,
            30,
            scale,
            *&self.screen.font.lock().unwrap().as_ref().unwrap(),
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

    fn update(instance: Arc<RwLock<SystemInfoScreen>>) {
        let mut image = RgbImage::new(256, 64);
        let scale = Scale { x: 16.0, y: 16.0 };

        instance.write().unwrap().draw_cpu(&mut image, scale);
        instance.write().unwrap().draw_memory(&mut image, scale);
        *instance.write().unwrap().screen.bytes.lock().unwrap() = image.into_vec();
    }

    pub fn new(
        description: String,
        font: Arc<Mutex<Option<Font<'static>>>>,
        config_manager: Arc<RwLock<ConfigManager>>,
    ) -> Arc<RwLock<SystemInfoScreen>> {
        let this = Arc::new(RwLock::new(SystemInfoScreen {
            screen: Screen {
                description,
                font,
                config_manager,
                ..Default::default()
            },
            cpu_usage: Arc::new(Mutex::new(0.0)),
            ram_usage: Arc::new(Mutex::new(0.0)),
        }));

        let builder = thread::Builder::new().name("JOB_EXECUTOR".into());
        let sys = System::new();

        *this.read().unwrap().screen.handle.lock().unwrap() = Some(
            builder
                .spawn({
                    let this = this.clone();
                    move || loop {
                        while !this.read().unwrap().screen.active.load(Ordering::Acquire) {
                            thread::park();
                        }
                        match sys.cpu_load_aggregate() {
                            Ok(cpu) => {
                                thread::sleep(Duration::from_millis(1000));
                                let cpu = cpu.done().unwrap();
                                *this.read().unwrap().cpu_usage.lock().unwrap() =
                                    ((cpu.user + cpu.nice + cpu.system) * 100.0).floor().into();
                            }
                            Err(x) => println!("\nCPU load: error: {}", x),
                        }
                        match sys.memory() {
                            Ok(mem) => {
                                let memory_used =
                                    saturating_sub_bytes(mem.total, mem.free).as_u64() as f64;
                                *this.read().unwrap().ram_usage.lock().unwrap() =
                                    ((memory_used / mem.total.as_u64() as f64) * 100.0).floor();
                            }
                            Err(x) => println!("\nMemory: error: {}", x),
                        }
                    }
                })
                .expect("Cannot create JOB_EXECUTOR thread"),
        );
        this.clone()
    }
}
