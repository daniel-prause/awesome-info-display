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
use winapi::um::mmdeviceapi::*;
use winapi::um::winuser::FindWindowW;
use winapi::um::winuser::*;
use winapi::Interface;
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
    title_x: Arc<Mutex<u32>>,
    artist_x: Arc<Mutex<u32>>,
    system_volume: Arc<Mutex<f32>>,
    mute: Arc<Mutex<i32>>,
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

    fn set_mode(&mut self, mode: u32) {
        MediaInfoScreen::set_mode(self, mode);
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

#[cfg(windows)]
impl MediaInfoScreen {
    fn draw_intro(&mut self, image: &mut ImageBuffer<Rgb<u8>, Vec<u8>>, scale: Scale) {
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
    fn draw_artist(&mut self, image: &mut ImageBuffer<Rgb<u8>, Vec<u8>>, scale: Scale) {
        let artist = self.artist.lock().unwrap();
        let mut position_artist = 0;
        let artist_len = artist.graphemes(true).count();
        let mut start = 0;
        if artist_len * 17 < 480 {
            position_artist = (1 + ((256 - (artist_len as u32 * 17) / 2) / 2)) - 2;
        } else {
            start = *self.artist_x.lock().unwrap() as usize;

            if *self.artist_x.lock().unwrap() == artist_len as u32 + 2 as u32 {
                *self.artist_x.lock().unwrap() = 0;
            } else {
                *self.artist_x.lock().unwrap() += 1;
            }
        }

        draw_text_mut(
            image,
            Rgb([255u8, 255u8, 255u8]),
            position_artist as u32,
            0,
            scale,
            self.screen.font.as_ref().unwrap(),
            &rotate(
                &[&artist.clone(), "   "].join("").to_string(),
                Direction::Left,
                start,
            ),
        );
    }

    fn draw_title(&mut self, image: &mut ImageBuffer<Rgb<u8>, Vec<u8>>, scale: Scale) {
        let title = self.title.lock().unwrap();
        let title_len = title.graphemes(true).count();
        let mut position_title = 0;
        let mut start = 0;
        if title_len * 17 < 480 {
            position_title = (1 + ((256 - (title_len as u32 * 17) / 2) / 2)) - 2;
        } else {
            start = *self.title_x.lock().unwrap() as usize;

            if *self.title_x.lock().unwrap() == title_len as u32 + 2 as u32 {
                *self.title_x.lock().unwrap() = 0;
            } else {
                *self.title_x.lock().unwrap() += 1;
            }
        }

        draw_text_mut(
            image,
            Rgb([255u8, 255u8, 255u8]),
            position_title as u32,
            16,
            scale,
            self.screen.font.as_ref().unwrap(),
            &rotate(
                &[&title.clone(), "   "].join("").to_string(),
                Direction::Left,
                start,
            ),
        );
    }

    fn draw_play_button(&mut self, image: &mut ImageBuffer<Rgb<u8>, Vec<u8>>, _scale: Scale) {
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

    fn draw_elapsed(&mut self, image: &mut ImageBuffer<Rgb<u8>, Vec<u8>>, _scale: Scale) {
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

    fn draw_total(&mut self, image: &mut ImageBuffer<Rgb<u8>, Vec<u8>>, _scale: Scale) {
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

    fn draw_elapsed_bar(&mut self, image: &mut ImageBuffer<Rgb<u8>, Vec<u8>>, _scale: Scale) {
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

    fn draw_volume_bar(&mut self, image: &mut ImageBuffer<Rgb<u8>, Vec<u8>>, _scale: Scale) {
        let progress = (1.0 + (238.0 * *self.system_volume.lock().unwrap())) as u32;

        draw_hollow_rect_mut(
            image,
            Rect::at(16, 50).of_size(238, 6),
            Rgb([255u8, 255u8, 255u8]),
        );
        let small_speaker = &String::from("\u{f027}");
        let big_speaker = &String::from("\u{f028}");
        let mute_speaker = &String::from("\u{f6a9}");

        draw_text_mut(
            image,
            Rgb([255u8, 255u8, 255u8]),
            16,
            38,
            Scale { x: 10.0, y: 10.0 },
            self.symbols.as_ref().unwrap(),
            small_speaker,
        );
        if *self.mute.lock().unwrap() == 1 {
            draw_text_mut(
                image,
                Rgb([255u8, 255u8, 255u8]),
                118,
                38,
                Scale { x: 10.0, y: 10.0 },
                self.symbols.as_ref().unwrap(),
                mute_speaker,
            );
        }
        draw_text_mut(
            image,
            Rgb([255u8, 255u8, 255u8]),
            240,
            37,
            Scale { x: 10.0, y: 10.0 },
            self.symbols.as_ref().unwrap(),
            big_speaker,
        );
        draw_filled_rect_mut(
            image,
            Rect::at(16, 50).of_size(progress, 6),
            Rgb([255u8, 255u8, 255u8]),
        );
    }
    fn update(&mut self) {
        let mut image = RgbImage::new(256, 64);
        let scale = Scale { x: 16.0, y: 16.0 };

        if *self.editor_active.lock().unwrap() {
            self.draw_artist(&mut image, scale);
            self.draw_title(&mut image, scale);
            if *self.screen.mode.lock().unwrap() == 0 {
                self.draw_play_button(&mut image, scale);
                self.draw_elapsed(&mut image, scale);
                self.draw_total(&mut image, scale);
                self.draw_elapsed_bar(&mut image, scale);
            } else {
                // DRAW VOLUME BAR
                self.draw_volume_bar(&mut image, scale);
            }
        } else {
            self.draw_intro(&mut image, scale);
        }
        self.screen.bytes.clear();
        let _ = DynamicImage::ImageRgb8(image)
            .write_to(&mut self.screen.bytes, image::ImageOutputFormat::Bmp);
        // let converted = self.convert_to_gray_scale(&self.screen.bytes);
    }

    fn set_mode(&mut self, mode: u32) {
        *self.screen.mode.lock().unwrap() = mode;
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
            title_x: Arc::new(Mutex::new(0)),
            artist: Arc::new(Mutex::new(String::new())),
            artist_x: Arc::new(Mutex::new(0)),
            system_volume: Arc::new(Mutex::new(get_master_volume().0)),
            mute: Arc::new(Mutex::new(get_master_volume().1)),
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

                                // WINAMP VOLUME. NOT USED RIGHT NOW.
                                // let mut volume = SendMessageW(hwnd, WM_USER, -666i32 as usize, 122);
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
                                let artist = artist_and_title_caps
                                    .get(1)
                                    .map_or("", |m| m.as_str())
                                    .trim();

                                let title = artist_and_title_caps
                                    .get(2)
                                    .map_or("", |m| m.as_str())
                                    .trim();

                                if (*this.artist.lock().unwrap() != artist)
                                    || (*this.title.lock().unwrap() != title)
                                {
                                    *this.title_x.lock().unwrap() = 0;
                                }

                                if *this.artist.lock().unwrap() != artist {
                                    *this.artist_x.lock().unwrap() = 0;
                                }
                                *this.artist.lock().unwrap() = artist.to_string();
                                *this.title.lock().unwrap() = title.to_string();
                            }
                        } else {
                            *this.editor_active.lock().unwrap() = false;
                        }

                        // TODO: only if audio mode is active!
                        let volume_data = get_master_volume();
                        *this.system_volume.lock().unwrap() = volume_data.0;
                        *this.mute.lock().unwrap() = volume_data.1;
                        thread::sleep(Duration::from_millis(500));
                    }
                })
                .expect("Cannot create JOB_EXECUTOR thread"),
        );
        this
    }
}
// MOVE EVERYTHING BELOW SOMEWHERE ELSE, TO SOME MODULE ETC.
pub enum Direction {
    Left,
    //Right,
}

pub fn rotate(str: &str, direction: Direction, count: usize) -> String {
    let mut str_vec: Vec<char> = str.chars().collect();
    match direction {
        Direction::Left => str_vec.rotate_left(count),
        //Direction::Right => str_vec.rotate_right(count),
    }
    str_vec.iter().collect()
}

// TODO: disable for non-windows OS and find another way.
pub fn get_master_volume() -> (f32, i32) {
    let mut current_volume = 0.0 as f32;
    let mut mute = 0;
    unsafe {
        winapi::um::objbase::CoInitialize(std::ptr::null_mut());
        let mut device_enumerator: *mut winapi::um::mmdeviceapi::IMMDeviceEnumerator =
            std::ptr::null_mut();
        // let mut hresult = winapi::um::combaseapi::CoCreateInstance(
        winapi::um::combaseapi::CoCreateInstance(
            &winapi::um::mmdeviceapi::CLSID_MMDeviceEnumerator,
            std::ptr::null_mut(),
            winapi::um::combaseapi::CLSCTX_ALL,
            &IMMDeviceEnumerator::uuidof(),
            &mut device_enumerator as *mut *mut winapi::um::mmdeviceapi::IMMDeviceEnumerator
                as *mut _,
        );
        let mut default_device: *mut winapi::um::mmdeviceapi::IMMDevice = std::mem::zeroed();
        (*device_enumerator).GetDefaultAudioEndpoint(
            winapi::um::mmdeviceapi::eRender,
            winapi::um::mmdeviceapi::eConsole,
            &mut default_device,
        );
        if default_device == std::ptr::null_mut() {
            return (0.0, 0);
        }

        (*device_enumerator).Release();
        //device_enumerator = std::mem::zeroed();
        let mut endpoint_volume: *mut winapi::um::endpointvolume::IAudioEndpointVolume =
            std::mem::zeroed();

        (*default_device).Activate(
            &winapi::um::endpointvolume::IAudioEndpointVolume::uuidof(),
            winapi::shared::wtypesbase::CLSCTX_INPROC_SERVER,
            std::ptr::null_mut(),
            &mut endpoint_volume as *mut *mut winapi::um::endpointvolume::IAudioEndpointVolume
                as *mut _,
        );

        (*default_device).Release();
        (*endpoint_volume).GetMasterVolumeLevelScalar(&mut current_volume as *mut f32);
        (*endpoint_volume).GetMute(&mut mute as *mut i32);
    }
    (current_volume, mute)
}
