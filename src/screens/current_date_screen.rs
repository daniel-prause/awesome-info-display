use crate::config_manager::ConfigManager;
use crate::screens::BasicScreen;
use crate::screens::Screen;
use crate::screens::ScreenControl;
use chrono::{DateTime, Local};
use crossbeam_channel::bounded;
use crossbeam_channel::{Receiver, Sender};
use image::{ImageBuffer, Rgb, RgbImage};
use imageproc::drawing::draw_text_mut;
use rusttype::Font;
use rusttype::Scale;
use std::rc::Rc;
use std::sync::{atomic::AtomicBool, atomic::Ordering, Arc, RwLock};
use std::thread;
use std::time::Duration;

pub struct CurrentDateScreen {
    screen: Screen,
    receiver: Receiver<ClockInfo>,
}

struct ClockInfo {
    local: DateTime<Local>,
}

impl Default for ClockInfo {
    fn default() -> ClockInfo {
        ClockInfo {
            local: Local::now(),
        }
    }
}

impl BasicScreen for CurrentDateScreen {
    fn description(&self) -> &String {
        &self.screen.description
    }

    fn current_image(&self) -> &Vec<u8> {
        self.screen.current_image()
    }

    fn update(&mut self) {
        CurrentDateScreen::update(self)
    }

    fn start(&self) {
        self.screen.start_worker();
    }

    fn stop(&self) {
        self.screen.stop_worker();
    }

    fn key(&self) -> &String {
        &self.screen.key()
    }

    fn initial_update_called(&mut self) -> bool {
        self.screen.initial_update_called()
    }

    fn enabled(&self) -> bool {
        self.screen
            .config_manager
            .read()
            .unwrap()
            .config
            .current_date_screen_active
    }

    fn set_status(&self, status: bool) {
        self.screen
            .config_manager
            .write()
            .unwrap()
            .config
            .current_date_screen_active = status;
    }
}

impl CurrentDateScreen {
    fn draw_screen(&mut self, local: DateTime<Local>) {
        // draw initial image
        let mut image = RgbImage::new(256, 64);
        let scale = Scale { x: 16.0, y: 16.0 };

        self.draw_clock(&mut image, local, scale);
        self.screen.bytes = image.into_vec();
    }

    pub fn draw_clock(
        &mut self,
        image: &mut ImageBuffer<Rgb<u8>, Vec<u8>>,
        local: DateTime<Local>,
        scale: Scale,
    ) {
        let date_time = local.format("%Y-%m-%d %H:%M:%S").to_string();
        draw_text_mut(
            image,
            Rgb([255u8, 255u8, 255u8]),
            46,
            24,
            scale,
            &self.screen.font,
            &date_time,
        );
    }

    pub fn new(
        description: String,
        key: String,
        font: Rc<Font<'static>>,
        config_manager: Arc<RwLock<ConfigManager>>,
    ) -> CurrentDateScreen {
        let (tx, rx): (Sender<ClockInfo>, Receiver<ClockInfo>) = bounded(1);
        let active = Arc::new(AtomicBool::new(false));
        let mut this = CurrentDateScreen {
            screen: Screen {
                description,
                key,
                font,
                active: active.clone(),
                handle: Some(thread::spawn(move || {
                    let sender = tx.to_owned();
                    let active = active.clone();
                    loop {
                        while !active.load(Ordering::Acquire) {
                            thread::park();
                        }
                        let clock_info: ClockInfo = ClockInfo::default();
                        sender.try_send(clock_info).unwrap_or_default();
                        thread::sleep(Duration::from_millis(1000));
                    }
                })),
                config_manager,
                ..Default::default()
            },
            receiver: rx,
        };

        let initial_clock_info = ClockInfo::default();
        this.draw_screen(initial_clock_info.local);
        this
    }

    pub fn update(&mut self) {
        let clock_info = self.receiver.try_recv();
        match clock_info {
            Ok(clock_info_state) => {
                self.draw_screen(clock_info_state.local);
            }
            Err(_) => {}
        }
    }
}
