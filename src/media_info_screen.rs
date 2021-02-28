extern crate winapi;
use crate::screen::Screen;
use crate::screen::SpecificScreen;
use image::{DynamicImage, ImageBuffer, Rgb, RgbImage};
use imageproc::drawing::{draw_filled_rect_mut, draw_hollow_rect_mut, draw_text_mut};
use imageproc::rect::Rect;
use regex;
use rusttype::Font;
use rusttype::Scale;
use std::ffi::OsStr;
use std::fmt::Debug;
use std::iter::once;
use std::os::windows::ffi::OsStrExt;
use std::ptr::null_mut;
use std::sync::{atomic::Ordering, Arc, Mutex};
use std::thread;
use std::time::Duration;
use unicode_segmentation::UnicodeSegmentation;
use winapi::shared::minwindef::LPARAM;
use winapi::um::winuser::FindWindowW;
use winapi::um::winuser::*;
#[derive(Debug, Clone)]
pub struct MediaInfoScreen {
    screen: Screen,
    symbols: Option<Font<'static>>,
    playback_status: Arc<Mutex<isize>>,
    track_current_position: Arc<Mutex<isize>>,
    track_length: Arc<Mutex<isize>>,
    title: Arc<Mutex<String>>,
    artist: Arc<Mutex<String>>,
    editor_active: Arc<Mutex<bool>>,
    regex_first: Arc<Mutex<regex::Regex>>,
    regex_second: Arc<Mutex<regex::Regex>>,
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

#[cfg(windows)]
impl MediaInfoScreen {
    pub fn draw_intro(&mut self, image: &mut ImageBuffer<Rgb<u8>, Vec<u8>>, scale: Scale) {
        draw_text_mut(
            image,
            Rgb([255u8, 255u8, 255u8]),
            77,
            4,
            scale,
            self.screen.font.as_ref().unwrap(),
            "Media Screen",
        );
        draw_text_mut(
            image,
            Rgb([255u8, 255u8, 255u8]),
            65,
            32,
            scale,
            self.screen.font.as_ref().unwrap(),
            "Winamp inactive",
        );
    }
    pub fn draw_artist(&mut self, image: &mut ImageBuffer<Rgb<u8>, Vec<u8>>, scale: Scale) {
        let artist = self.artist.lock().unwrap();
        let mut position_artist = 0;
        let artist_len = artist.graphemes(true).count();
        if artist_len * 17 < 480 {
            position_artist = (1 + ((256 - (artist_len as u32 * 17) / 2) / 2)) - 2;
        }

        draw_text_mut(
            image,
            Rgb([255u8, 255u8, 255u8]),
            position_artist as u32,
            0,
            scale,
            self.screen.font.as_ref().unwrap(),
            &artist,
        );
    }

    pub fn draw_title(&mut self, image: &mut ImageBuffer<Rgb<u8>, Vec<u8>>, scale: Scale) {
        let title = self.title.lock().unwrap();
        let title_len = title.graphemes(true).count();
        let mut position_title = 0;
        if title_len * 17 < 480 {
            position_title = (1 + ((256 - (title_len as u32 * 17) / 2) / 2)) - 2;
        }

        draw_text_mut(
            image,
            Rgb([255u8, 255u8, 255u8]),
            position_title as u32,
            16,
            scale,
            self.screen.font.as_ref().unwrap(),
            &title,
        );
    }

    pub fn draw_play_button(&mut self, image: &mut ImageBuffer<Rgb<u8>, Vec<u8>>, _scale: Scale) {
        let play_button = &String::from("\u{f04B}");
        let pause_button = &String::from("\u{f04C}");
        let stop_button = &String::from("\u{f04D}");

        let mut button = stop_button;
        if *self.playback_status.lock().unwrap() == 1 {
            button = play_button;
        }

        if *self.playback_status.lock().unwrap() == 3 {
            button = pause_button;
        }
        draw_text_mut(
            image,
            Rgb([255u8, 255u8, 255u8]),
            4,
            37,
            Scale { x: 10.0, y: 10.0 },
            self.symbols.as_ref().unwrap(),
            button,
        );
    }

    pub fn draw_elapsed(&mut self, image: &mut ImageBuffer<Rgb<u8>, Vec<u8>>, _scale: Scale) {
        let original = *self.track_current_position.lock().unwrap() / 1000;
        let seconds = original % 60;
        let minutes = (original / 60) % 60;
        let hours = (original / 60) / 60;
        let elapsed = format!("{:0>2}:{:0>2}:{:0>2}", hours, minutes, seconds).to_string();
        let elapsed = format!("{: <12}", elapsed.to_string()).to_string();
        draw_text_mut(
            image,
            Rgb([255u8, 255u8, 255u8]),
            16,
            36,
            Scale { x: 14.0, y: 14.0 },
            self.screen.font.as_ref().unwrap(),
            &elapsed,
        );
    }

    pub fn draw_total(&mut self, image: &mut ImageBuffer<Rgb<u8>, Vec<u8>>, _scale: Scale) {
        let original = *self.track_length.lock().unwrap();
        let seconds = original % 60;
        let minutes = (original / 60) % 60;
        let hours = (original / 60) / 60;
        let total = format!("{:0>2}:{:0>2}:{:0>2}", hours, minutes, seconds).to_string();
        let total = format!("{: >12}", total.to_string()).to_string();
        draw_text_mut(
            image,
            Rgb([255u8, 255u8, 255u8]),
            166,
            36,
            Scale { x: 14.0, y: 14.0 },
            self.screen.font.as_ref().unwrap(),
            &total,
        );
    }

    pub fn draw_elapsed_bar(&mut self, image: &mut ImageBuffer<Rgb<u8>, Vec<u8>>, _scale: Scale) {
        let indicator_position_x_min = 16.0;
        let indicator_position_x_max = 232.0;

        let progress = (*self.track_current_position.lock().unwrap() as f64 / 1000.0)
            / (*self.track_length.lock().unwrap() as f64);
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
    }

    fn update(&mut self) {
        let mut image = RgbImage::new(256, 64);
        let scale = Scale { x: 16.0, y: 16.0 };

        if *self.editor_active.lock().unwrap() {
            self.draw_artist(&mut image, scale);
            self.draw_title(&mut image, scale);
            self.draw_play_button(&mut image, scale);
            self.draw_elapsed(&mut image, scale);
            self.draw_total(&mut image, scale);
            self.draw_elapsed_bar(&mut image, scale);
        } else {
            self.draw_intro(&mut image, scale);
        }
        self.screen.bytes.clear();
        let _ = DynamicImage::ImageRgb8(image)
            .write_to(&mut self.screen.bytes, image::ImageOutputFormat::Bmp);
    }

    #[cfg(windows)]
    pub fn new(description: String, font: Option<Font<'static>>) -> Self {
        let this = MediaInfoScreen {
            screen: Screen {
                description,
                font,
                ..Default::default()
            },
            symbols: Font::try_from_vec(Vec::from(include_bytes!("symbols.otf") as &[u8])),
            playback_status: Arc::new(Mutex::new(0)),
            track_current_position: Arc::new(Mutex::new(0)),
            track_length: Arc::new(Mutex::new(0)),
            title: Arc::new(Mutex::new(String::new())),
            artist: Arc::new(Mutex::new(String::new())),
            editor_active: Arc::new(Mutex::new(false)),
            regex_first: Arc::new(Mutex::new(regex::Regex::new(r"\s(.*)-").unwrap())),
            regex_second: Arc::new(Mutex::new(regex::Regex::new(r"(.*) - (.*)").unwrap())),
        };

        let builder = thread::Builder::new().name("JOB_EXECUTOR".into());

        *this.screen.handle.lock().unwrap() = Some(
            builder
                .spawn({
                    let this = this.clone();
                    move || loop {
                        while !this.screen.active.load(Ordering::Acquire) {
                            thread::park();
                        }
                        let window: Vec<u16> = OsStr::new("Winamp v1.x")
                            .encode_wide()
                            .chain(once(0))
                            .collect();

                        let hwnd = unsafe { FindWindowW(window.as_ptr(), null_mut()) };

                        if hwnd != null_mut() {
                            unsafe {
                                // 1 == playing, 3 == paused, anything else == stopped
                                let playback_status = SendMessageW(hwnd, WM_USER, 0, 104);
                                *this.playback_status.lock().unwrap() = playback_status;
                                // current position in msecs
                                let mut track_current_position =
                                    SendMessageW(hwnd, WM_USER, 0, 105);
                                if playback_status != 1 && playback_status != 3 {
                                    track_current_position = 0;
                                }

                                *this.track_current_position.lock().unwrap() =
                                    track_current_position;

                                // track length in seconds (multiply by thousand)
                                let track_length = SendMessageW(hwnd, WM_USER, 1, 105);
                                *this.track_length.lock().unwrap() = track_length;
                                // get title
                                let current_index = SendMessageW(hwnd, WM_USER, 0, 125);
                                let mut title_length = SendMessageA(
                                    hwnd,
                                    WM_GETTEXTLENGTH,
                                    current_index as usize,
                                    3034,
                                );
                                title_length += 1;

                                let mut buffer = Vec::<u16>::with_capacity(title_length as usize);
                                buffer.set_len(title_length as usize);
                                SendMessageW(hwnd, WM_GETTEXT, 3034, buffer.as_mut_ptr() as LPARAM);
                                let data = String::from_utf16_lossy(&buffer);

                                if title_length == 1
                                    || !this.regex_first.lock().unwrap().is_match(&data)
                                {
                                    *this.editor_active.lock().unwrap() = false;
                                    thread::sleep(Duration::from_secs(1));
                                    continue;
                                } else {
                                    *this.editor_active.lock().unwrap() = true;
                                }

                                let caps =
                                    this.regex_first.lock().unwrap().captures(&data).unwrap();
                                let artist_and_title = caps.get(1).map_or("", |m| m.as_str());

                                let artist_and_title_caps = this
                                    .regex_second
                                    .lock()
                                    .unwrap()
                                    .captures(&artist_and_title)
                                    .unwrap();
                                let artist =
                                    artist_and_title_caps.get(1).map_or("", |m| m.as_str());

                                let title = artist_and_title_caps
                                    .get(2)
                                    .map_or("", |m| m.as_str())
                                    .trim();

                                *this.artist.lock().unwrap() = artist.to_string();
                                *this.title.lock().unwrap() = title.to_string();
                            }
                        } else {
                            *this.editor_active.lock().unwrap() = false;
                        }

                        thread::sleep(Duration::from_secs(1));
                    }
                })
                .expect("Cannot create JOB_EXECUTOR thread"),
        );
        this
    }
}
