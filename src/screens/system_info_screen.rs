use crate::{
    config_manager::ConfigManager,
    screens::{BasicScreen, Screen, Screenable},
    TEENSY,
};
use ab_glyph::{FontArc, PxScale};
use cpu_monitor::CpuInstant;
use crossbeam_channel::{bounded, Receiver, Sender};
use image::{ImageBuffer, Rgb, RgbImage};
use imageproc::{
    drawing::{draw_filled_rect_mut, draw_hollow_rect_mut, draw_text_mut},
    rect::Rect,
};
use std::{
    sync::{atomic::AtomicBool, atomic::Ordering, Arc, RwLock},
    thread,
    time::Duration,
};
use systemstat::{saturating_sub_bytes, Platform, System};

pub struct SystemInfoScreen {
    screen: Screen,
    receiver: Receiver<Arc<SystemInfoState>>,
}

#[derive(Default)]
struct SystemInfoState {
    cpu_usage: f64,
    ram_usage: f64,
}

impl Screenable for SystemInfoScreen {
    fn get_screen(&mut self) -> &mut Screen {
        &mut self.screen
    }
}

impl BasicScreen for SystemInfoScreen {
    fn update(&mut self) {
        if let Ok(system_info_arc) = self.receiver.try_recv() {
            let system_info: &SystemInfoState = &system_info_arc;
            self.draw_screen(system_info.cpu_usage, system_info.ram_usage);
        }
    }
}

impl SystemInfoScreen {
    fn draw_cpu(&self, image: &mut ImageBuffer<Rgb<u8>, Vec<u8>>, cpu_usage: f64) {
        let scale = PxScale { x: 16.0, y: 16.0 };
        draw_text_mut(
            image,
            Rgb([255, 255, 255]),
            0,
            0,
            scale,
            &self.screen.font,
            "CPU",
        );
        draw_text_mut(
            image,
            Rgb([255, 255, 255]),
            222,
            0,
            scale,
            &self.screen.font,
            &format!("{:3.0}%", cpu_usage),
        );

        draw_hollow_rect_mut(
            image,
            Rect::at(0, 16).of_size(256, 10),
            Rgb([255, 255, 255]),
        );
        let cpu_filled = ((cpu_usage * 2.56) + 1.0).floor() as u32;
        draw_filled_rect_mut(
            image,
            Rect::at(0, 16).of_size(cpu_filled, 10),
            Rgb([255, 255, 255]),
        );
    }

    fn draw_memory(&self, image: &mut ImageBuffer<Rgb<u8>, Vec<u8>>, ram_usage: f64) {
        let scale = PxScale { x: 16.0, y: 16.0 };
        draw_text_mut(
            image,
            Rgb([255, 255, 255]),
            0,
            30,
            scale,
            &self.screen.font,
            "RAM",
        );
        draw_text_mut(
            image,
            Rgb([255, 255, 255]),
            222,
            30,
            scale,
            &self.screen.font,
            &format!("{:3.0}%", ram_usage),
        );

        draw_hollow_rect_mut(
            image,
            Rect::at(0, 48).of_size(256, 10),
            Rgb([255, 255, 255]),
        );
        let memory_filled = ((ram_usage * 2.56) + 1.0).floor() as u32;
        draw_filled_rect_mut(
            image,
            Rect::at(0, 48).of_size(memory_filled, 10),
            Rgb([255, 255, 255]),
        );
    }

    fn draw_screen(&mut self, cpu_usage: f64, ram_usage: f64) {
        let mut image = RgbImage::new(256, 64);
        self.draw_cpu(&mut image, cpu_usage);
        self.draw_memory(&mut image, ram_usage);
        *self.screen.device_screen_bytes.get_mut(TEENSY).unwrap() = image.into_vec();
    }

    pub fn new(
        description: String,
        key: String,
        font: FontArc,
        config_manager: Arc<RwLock<ConfigManager>>,
    ) -> SystemInfoScreen {
        let (tx, rx): (Sender<Arc<SystemInfoState>>, Receiver<Arc<SystemInfoState>>) = bounded(1);
        let active = Arc::new(AtomicBool::new(false));

        let screen = Screen {
            description,
            key,
            font,
            config_manager,
            active: active.clone(),
            handle: Some(thread::spawn({
                let active = active.clone();
                move || {
                    let sys = System::new();
                    let sender = tx;
                    loop {
                        while !active.load(Ordering::Acquire) {
                            thread::park();
                        }

                        let start = CpuInstant::now().unwrap();
                        thread::sleep(Duration::from_secs(1));
                        let end = CpuInstant::now().unwrap();
                        let duration = end - start;

                        let cpu_usage = (duration.non_idle() * 100.0).floor();
                        let ram_usage = match sys.memory() {
                            Ok(mem) => {
                                let used =
                                    saturating_sub_bytes(mem.total, mem.free).as_u64() as f64;
                                ((used / mem.total.as_u64() as f64) * 100.0).floor()
                            }
                            Err(_) => 0.0,
                        };

                        let system_info = Arc::new(SystemInfoState {
                            cpu_usage,
                            ram_usage,
                        });
                        let _ = sender.try_send(system_info);
                    }
                }
            })),
            ..Default::default()
        };

        let mut this = SystemInfoScreen {
            screen,
            receiver: rx,
        };

        this.draw_screen(0.0, 0.0); // initial draw
        this
    }
}
