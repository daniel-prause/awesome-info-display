extern crate winapi;
use crate::config_manager::ConfigManager;
use crate::screen::BasicScreen;
use crate::screen::Screen;

use image::{ImageBuffer, Rgb, RgbImage};
use imageproc::drawing::{
    draw_filled_rect_mut, draw_hollow_rect_mut, draw_line_segment_mut, draw_text_mut,
};
use imageproc::rect::Rect;
use regex;
use rusttype::Font;
use rusttype::Scale;
use std::ffi::OsStr;
use std::fmt::Debug;
use std::iter::once;
use std::os::windows::ffi::OsStrExt;
use std::ptr::null_mut;
use std::sync::{atomic::Ordering, Arc, Mutex, RwLock};
use std::thread;
use std::time::Duration;
use std::time::Instant;
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

impl BasicScreen for std::sync::Arc<RwLock<MediaInfoScreen>> {
    fn description(&self) -> String {
        self.read().unwrap().screen.description.clone()
    }

    fn current_image(&self) -> Vec<u8> {
        self.read().unwrap().screen.bytes.clone()
    }

    fn update(&mut self) {
        MediaInfoScreen::update(self.clone());
    }

    fn set_mode_for_short(&mut self, mode: u32) {
        MediaInfoScreen::set_mode(self.clone(), mode);
    }

    fn start(&self) {
        self.read()
            .unwrap()
            .screen
            .active
            .store(true, Ordering::Release);
        match self.read().unwrap().screen.handle.lock() {
            Ok(lock) => match lock.as_ref() {
                Some(handle) => {
                    handle.thread().unpark();
                }
                None => {}
            },
            Err(_) => {}
        }
    }

    fn stop(&self) {
        self.read()
            .unwrap()
            .screen
            .active
            .store(false, Ordering::Release);
    }

    fn initial_update_called(&mut self) -> bool {
        if !self
            .read()
            .unwrap()
            .screen
            .initial_update_called
            .load(Ordering::Acquire)
        {
            self.read()
                .unwrap()
                .screen
                .initial_update_called
                .store(true, Ordering::Release);
            return false;
        }
        true
    }

    fn enabled(&self) -> bool {
        return self
            .read()
            .unwrap()
            .screen
            .config
            .read()
            .unwrap()
            .config
            .media_screen_active;
    }

    fn set_status(&self, status: bool) {
        self.read()
            .unwrap()
            .screen
            .config
            .write()
            .unwrap()
            .config
            .media_screen_active = status;
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
            Rect::at(16, 50).of_size(238, 7),
            Rgb([255u8, 255u8, 255u8]),
        );

        draw_filled_rect_mut(
            image,
            Rect::at(position as i32, 50).of_size(6, 7),
            Rgb([255u8, 255u8, 255u8]),
        );

        let start = 16;
        let end = position as i32;
        let segment_length = 6;
        let line_length = (end - start) + segment_length;
        let segments = line_length / segment_length;

        for n in 0..segments {
            let formula = start as f32 + (n as f32 * segment_length as f32);
            draw_line_segment_mut(
                image,
                (formula, 53.0),
                (formula + (segment_length / 2) as f32, 53.0),
                Rgb([255u8, 255u8, 255u8]),
            );
        }
    }

    fn draw_mute_speaker(&mut self, image: &mut ImageBuffer<Rgb<u8>, Vec<u8>>, _scale: Scale) {
        let mute_speaker = &String::from("\u{f6a9}");
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

        draw_text_mut(
            image,
            Rgb([255u8, 255u8, 255u8]),
            16,
            38,
            Scale { x: 10.0, y: 10.0 },
            self.symbols.as_ref().unwrap(),
            small_speaker,
        );

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

        self.draw_play_button(image, Scale { x: 16.0, y: 16.0 });
    }
    fn update(instance: Arc<RwLock<MediaInfoScreen>>) {
        let mut image = RgbImage::new(256, 64);
        let scale = Scale { x: 16.0, y: 16.0 };
        let seconds = Duration::from_secs(3);
        if instance
            .read()
            .unwrap()
            .screen
            .mode_timeout
            .lock()
            .unwrap()
            .unwrap()
            .elapsed()
            >= seconds
        {
            *instance.read().unwrap().screen.mode.lock().unwrap() = 0;
        }
        if *instance.read().unwrap().editor_active.lock().unwrap() {
            instance.write().unwrap().draw_artist(&mut image, scale);
            instance.write().unwrap().draw_title(&mut image, scale);
            instance
                .write()
                .unwrap()
                .draw_mute_speaker(&mut image, scale);

            if *instance.read().unwrap().screen.mode.lock().unwrap() == 0 {
                instance
                    .write()
                    .unwrap()
                    .draw_play_button(&mut image, scale);
                instance.write().unwrap().draw_elapsed(&mut image, scale);
                instance.write().unwrap().draw_total(&mut image, scale);
                instance
                    .write()
                    .unwrap()
                    .draw_elapsed_bar(&mut image, scale);
            } else {
                // DRAW VOLUME BAR
                instance.write().unwrap().draw_volume_bar(&mut image, scale);
            }
        } else {
            instance.write().unwrap().draw_intro(&mut image, scale);
        }
        instance.write().unwrap().screen.bytes = image.into_vec();
    }

    fn set_mode(instance: Arc<RwLock<MediaInfoScreen>>, mode: u32) {
        *instance.read().unwrap().screen.mode_timeout.lock().unwrap() = Some(Instant::now());
        *instance.read().unwrap().screen.mode.lock().unwrap() = mode;
    }

    #[cfg(windows)]
    pub fn new(
        description: String,
        font: Option<Font<'static>>,
        config: Arc<RwLock<ConfigManager>>,
    ) -> Arc<RwLock<MediaInfoScreen>> {
        let this = Arc::new(RwLock::new(MediaInfoScreen {
            screen: Screen {
                description,
                font,
                config,
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
            system_volume: Arc::new(Mutex::new(get_master_volume(true).0)),
            mute: Arc::new(Mutex::new(get_master_volume(false).1)),
            editor_active: Arc::new(Mutex::new(false)),
            regex_first: Arc::new(Mutex::new(regex::Regex::new(r"\s(.*)-").unwrap())),
            regex_second: Arc::new(Mutex::new(regex::Regex::new(r"(.*) - (.*)").unwrap())),
        }));

        let builder = thread::Builder::new().name("JOB_EXECUTOR".into());
        *this.read().unwrap().screen.handle.lock().unwrap() = Some(
            builder
                .spawn({
                    let this = this.clone();
                    move || {
                        let window: Vec<u16> = OsStr::new("Winamp v1.x")
                            .encode_wide()
                            .chain(once(0))
                            .collect();

                        loop {
                            while !this.read().unwrap().screen.active.load(Ordering::Acquire) {
                                thread::park();
                            }

                            let hwnd = unsafe { FindWindowW(window.as_ptr(), null_mut()) };

                            if hwnd != null_mut() {
                                unsafe {
                                    // 1 == playing, 3 == paused, anything else == stopped
                                    let playback_status = SendMessageW(hwnd, WM_USER, 0, 104);
                                    *this.read().unwrap().playback_status.lock().unwrap() =
                                        playback_status;
                                    // current position in msecs
                                    let mut track_current_position =
                                        SendMessageW(hwnd, WM_USER, 0, 105);
                                    if playback_status != 1 && playback_status != 3 {
                                        track_current_position = 0;
                                    }

                                    *this.read().unwrap().track_current_position.lock().unwrap() =
                                        track_current_position;

                                    // track length in seconds (multiply by thousand)
                                    let track_length = SendMessageW(hwnd, WM_USER, 1, 105);
                                    *this.read().unwrap().track_length.lock().unwrap() =
                                        track_length;
                                    // get title
                                    let current_index = SendMessageW(hwnd, WM_USER, 0, 125);
                                    let title_length = SendMessageW(
                                        hwnd,
                                        WM_GETTEXTLENGTH,
                                        current_index as usize,
                                        0,
                                    );

                                    // WINAMP VOLUME. NOT USED RIGHT NOW.
                                    // let mut volume = SendMessageW(hwnd, WM_USER, -666i32 as usize, 122);

                                    let buffer_length = title_length + 1;
                                    let mut buffer =
                                        Vec::<u16>::with_capacity(buffer_length as usize);
                                    buffer.resize(buffer_length as usize, 0);
                                    SendMessageW(
                                        hwnd,
                                        WM_GETTEXT,
                                        buffer_length as usize,
                                        buffer.as_mut_ptr() as LPARAM,
                                    );
                                    let data = String::from_utf16_lossy(&buffer);

                                    if title_length == 0
                                        || !this
                                            .read()
                                            .unwrap()
                                            .regex_first
                                            .lock()
                                            .unwrap()
                                            .is_match(&data)
                                    {
                                        *this.read().unwrap().editor_active.lock().unwrap() = false;
                                        thread::sleep(Duration::from_millis(200));
                                        continue;
                                    } else {
                                        *this.read().unwrap().editor_active.lock().unwrap() = true;
                                    }

                                    let caps = this
                                        .read()
                                        .unwrap()
                                        .regex_first
                                        .lock()
                                        .unwrap()
                                        .captures(&data)
                                        .unwrap();
                                    let artist_and_title = caps.get(1).map_or("", |m| m.as_str());

                                    let artist_and_title_caps = this
                                        .read()
                                        .unwrap()
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

                                    if (*this.read().unwrap().artist.lock().unwrap() != artist)
                                        || (*this.read().unwrap().title.lock().unwrap() != title)
                                    {
                                        *this.read().unwrap().title_x.lock().unwrap() = 0;
                                    }

                                    if *this.read().unwrap().artist.lock().unwrap() != artist {
                                        *this.read().unwrap().artist_x.lock().unwrap() = 0;
                                    }
                                    *this.read().unwrap().artist.lock().unwrap() =
                                        artist.to_string();
                                    *this.read().unwrap().title.lock().unwrap() = title.to_string();
                                }
                            } else {
                                *this.read().unwrap().editor_active.lock().unwrap() = false;
                            }

                            // TODO: only if audio mode is active!
                            let volume_data = get_master_volume(false);
                            *this.read().unwrap().system_volume.lock().unwrap() = volume_data.0;
                            *this.read().unwrap().mute.lock().unwrap() = volume_data.1;
                            thread::sleep(Duration::from_millis(200));
                        }
                    }
                })
                .expect("Cannot create JOB_EXECUTOR thread"),
        );
        this.clone()
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
pub fn get_master_volume(init: bool) -> (f32, i32) {
    let mut current_volume = 0.0 as f32;
    let mut mute = 0;
    unsafe {
        if init {
            winapi::um::objbase::CoInitialize(std::ptr::null_mut());
        }
        let mut device_enumerator: *mut winapi::um::mmdeviceapi::IMMDeviceEnumerator =
            std::ptr::null_mut();
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

impl Drop for MediaInfoScreen {
    fn drop(&mut self) {
        unsafe {
            winapi::um::combaseapi::CoUninitialize();
        }
    }
}
